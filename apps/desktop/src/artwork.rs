//! Framework-free cover presentation state for the native UI.
//!
//! One bounded, deduplicated request per artwork id, scoped to the current
//! origin/account/session generation. The UI renders `Ready` bytes as a `data:` URL and
//! keeps the existing neutral Vela cover for every other state, so an absent, denied or
//! failed cover never adds a broken-image glyph, never turns into a request storm and
//! never displaces the audio source. All transitions here are pure and unit-tested;
//! the actual native window remains separate evidence.
use crate::client::ArtworkError;
use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
};

/// Concurrent cover requests. Deliberately small: artwork is user-visible presentation,
/// not audio, so it must not be able to crowd the single authorized source path.
pub const MAX_INFLIGHT: usize = 4;
/// Waiting requests are bounded too; beyond this a row keeps its fallback and may ask
/// again only when it is mounted later.
pub const MAX_QUEUED: usize = 32;
/// Per-session entries (`Loading`/`Ready`/`Denied`/`Failed`) stay bounded during long
/// scrolling instead of growing with every artwork id ever seen.
pub const MAX_ENTRIES: usize = 512;
/// Session budget of encoded `data:` URLs handed to the renderer. A cover larger than
/// this is never retained; the session cache evicts its oldest ready covers first.
pub const MAX_READY_BYTES: usize = 32 * 1024 * 1024;

/// One artwork of one app session on one origin and account. The id must be a canonical
/// UUID: a renderer or list row can never supply an arbitrary path or URL.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct ArtworkKey {
    generation: u64,
    origin: Arc<str>,
    account_id: Arc<str>,
    artwork_id: Arc<str>,
}
impl ArtworkKey {
    pub fn new(
        origin: &str,
        account_id: &str,
        generation: u64,
        artwork_id: Option<&str>,
    ) -> Option<Self> {
        let artwork_id = artwork_id?;
        if origin.is_empty()
            || origin.len() > 2048
            || !crate::client::canonical_uuid(account_id)
            || !crate::client::canonical_uuid(artwork_id)
        {
            return None;
        }
        Some(Self {
            generation,
            origin: Arc::from(origin),
            account_id: Arc::from(account_id),
            artwork_id: Arc::from(artwork_id),
        })
    }
    pub fn generation(&self) -> u64 {
        self.generation
    }
    pub fn artwork_id(&self) -> &str {
        &self.artwork_id
    }
    /// True only for the exact origin, account and session generation this key was
    /// created for. A stale key can never publish into another account's presentation.
    pub fn belongs_to(&self, origin: &str, account_id: &str, generation: u64) -> bool {
        self.generation == generation
            && self.origin.as_ref() == origin
            && self.account_id.as_ref() == account_id
    }
}

/// What a cover slot renders. `Ready` is the only state that reaches the renderer as an
/// image; everything else keeps the neutral cover.
#[derive(Clone, PartialEq, Eq)]
pub enum ArtworkStatus {
    Loading,
    Ready(Arc<str>),
    Denied,
    Failed,
}
/// Never prints the `data:` payload: it is private image content, not diagnostics.
impl std::fmt::Debug for ArtworkStatus {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.label())
    }
}
impl ArtworkStatus {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Loading => "loading",
            Self::Ready(_) => "ready",
            Self::Denied => "denied",
            Self::Failed => "failed",
        }
    }
    /// The already-authorized `data:` URL, if any. Never a cookie, token or origin.
    pub fn data_url(&self) -> Option<&str> {
        match self {
            Self::Ready(url) => Some(url),
            Self::Loading | Self::Denied | Self::Failed => None,
        }
    }
}

/// What the UI should do with one [`ArtworkStore::request`] answer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArtworkDecision {
    /// Start exactly one fetch for this key now; it is admitted to the in-flight set.
    Fetch,
    /// Already known, waiting, or deliberately not retried automatically.
    Waiting,
    /// Not applicable: another generation, an invalid id, or no entry room left.
    Rejected,
}

struct Entry {
    status: ArtworkStatus,
    bytes: usize,
}

#[derive(Clone, Copy)]
struct Limits {
    inflight: usize,
    queued: usize,
    entries: usize,
    ready_bytes: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            inflight: MAX_INFLIGHT,
            queued: MAX_QUEUED,
            entries: MAX_ENTRIES,
            ready_bytes: MAX_READY_BYTES,
        }
    }
}

/// Bounded cover presentation state for one app session.
pub struct ArtworkStore {
    generation: u64,
    limits: Limits,
    entries: HashMap<ArtworkKey, Entry>,
    inflight: Vec<ArtworkKey>,
    queued: VecDeque<ArtworkKey>,
    ready_order: VecDeque<ArtworkKey>,
    ready_bytes: usize,
}
impl ArtworkStore {
    pub fn new(generation: u64) -> Self {
        Self::with_limits(generation, Limits::default())
    }
    fn with_limits(generation: u64, limits: Limits) -> Self {
        Self {
            generation,
            limits,
            entries: HashMap::new(),
            inflight: Vec::new(),
            queued: VecDeque::new(),
            ready_order: VecDeque::new(),
            ready_bytes: 0,
        }
    }
    pub fn generation(&self) -> u64 {
        self.generation
    }
    /// Drop every cover and every pending request, and adopt the new session generation.
    /// Sign-out and origin/account switches call this, so old-account art can never stay
    /// visible or be published by a late fetch.
    pub fn reset(&mut self, generation: u64) {
        self.generation = generation;
        self.entries.clear();
        self.inflight.clear();
        self.queued.clear();
        self.ready_order.clear();
        self.ready_bytes = 0;
    }
    /// The current presentation state of one key, if it is known.
    pub fn status(&self, key: &ArtworkKey) -> Option<&ArtworkStatus> {
        if key.generation != self.generation {
            return None;
        }
        self.entries.get(key).map(|entry| &entry.status)
    }
    /// Ask for one cover. Duplicate rows and repeated renders share the same entry and
    /// therefore the same single request; denied/failed entries are not retried
    /// automatically, while a canceled fetch leaves no entry so a later mount may retry.
    pub fn request(&mut self, key: ArtworkKey) -> ArtworkDecision {
        if key.generation != self.generation {
            return ArtworkDecision::Rejected;
        }
        if self.entries.contains_key(&key) {
            return ArtworkDecision::Waiting;
        }
        self.evict_ready();
        if self.entries.len() >= self.limits.entries {
            return ArtworkDecision::Rejected;
        }
        self.entries.insert(
            key.clone(),
            Entry {
                status: ArtworkStatus::Loading,
                bytes: 0,
            },
        );
        if self.inflight.len() < self.limits.inflight {
            self.inflight.push(key);
            ArtworkDecision::Fetch
        } else {
            if self.queued.len() < self.limits.queued {
                self.queued.push_back(key);
            }
            ArtworkDecision::Waiting
        }
    }
    /// Publish one completed fetch. A stale generation, or a key that was dropped by
    /// [`ArtworkStore::reset`], is ignored. Returns the next queued key to fetch, if any.
    pub fn complete(
        &mut self,
        key: &ArtworkKey,
        result: Result<Arc<str>, ArtworkError>,
    ) -> Option<ArtworkKey> {
        if key.generation != self.generation {
            return None;
        }
        self.inflight.retain(|pending| pending != key);
        if self.entries.contains_key(key) {
            match result {
                Ok(url) => self.ready(key, url),
                Err(ArtworkError::Denied) => self.set(key, ArtworkStatus::Denied),
                Err(ArtworkError::Failed) => self.set(key, ArtworkStatus::Failed),
                // A canceled fetch is not an answer about the artwork: forget it so a
                // later mount may ask again.
                Err(ArtworkError::Canceled) => {
                    self.entries.remove(key);
                }
            }
        }
        self.promote()
    }
    fn set(&mut self, key: &ArtworkKey, status: ArtworkStatus) {
        if let Some(entry) = self.entries.get_mut(key) {
            entry.status = status;
            entry.bytes = 0;
        }
    }
    fn ready(&mut self, key: &ArtworkKey, url: Arc<str>) {
        let bytes = url.len();
        if bytes > self.limits.ready_bytes {
            // Never retain a cover that alone exceeds the renderer budget.
            self.set(key, ArtworkStatus::Failed);
            return;
        }
        if let Some(entry) = self.entries.get_mut(key) {
            entry.status = ArtworkStatus::Ready(url);
            entry.bytes = bytes;
        } else {
            return;
        }
        self.ready_bytes = self.ready_bytes.saturating_add(bytes);
        self.ready_order.retain(|ready| ready != key);
        self.ready_order.push_back(key.clone());
        self.evict_ready();
    }
    /// Evict the oldest ready covers until the count and byte budgets fit. Loading,
    /// denied and failed entries cost no image memory and are kept bounded by
    /// [`MAX_ENTRIES`] alone.
    fn evict_ready(&mut self) {
        while self.ready_bytes > self.limits.ready_bytes || self.entries.len() > self.limits.entries
        {
            let Some(oldest) = self.ready_order.pop_front() else {
                break;
            };
            if let Some(entry) = self.entries.remove(&oldest) {
                self.ready_bytes = self.ready_bytes.saturating_sub(entry.bytes);
            }
        }
    }
    fn promote(&mut self) -> Option<ArtworkKey> {
        while self.inflight.len() < self.limits.inflight {
            let next = self.queued.pop_front()?;
            if self
                .entries
                .get(&next)
                .is_some_and(|entry| matches!(entry.status, ArtworkStatus::Loading))
            {
                self.inflight.push(next.clone());
                return Some(next);
            }
        }
        None
    }
    #[cfg(test)]
    fn inflight_len(&self) -> usize {
        self.inflight.len()
    }
    #[cfg(test)]
    fn entries_len(&self) -> usize {
        self.entries.len()
    }
    #[cfg(test)]
    fn ready_bytes(&self) -> usize {
        self.ready_bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ORIGIN: &str = "https://music.example.org";
    const OTHER_ORIGIN: &str = "https://other.example.org";
    const ACCOUNT: &str = "10000000-0000-4000-8000-000000000001";
    const OTHER_ACCOUNT: &str = "10000000-0000-4000-8000-000000000002";
    const ART: &str = "20000000-0000-4000-8000-000000000001";
    const OTHER_ART: &str = "20000000-0000-4000-8000-000000000002";

    fn key(generation: u64, artwork: &str) -> ArtworkKey {
        ArtworkKey::new(ORIGIN, ACCOUNT, generation, Some(artwork)).unwrap()
    }

    fn url(bytes: usize) -> Arc<str> {
        Arc::from(format!("data:image/jpeg;base64,{}", "A".repeat(bytes)))
    }

    fn small() -> Limits {
        Limits {
            inflight: 2,
            queued: 2,
            entries: 4,
            ready_bytes: 64,
        }
    }

    #[test]
    fn keys_require_a_canonical_artwork_id_and_account() {
        assert!(ArtworkKey::new(ORIGIN, ACCOUNT, 1, None).is_none());
        assert!(ArtworkKey::new(ORIGIN, ACCOUNT, 1, Some("../private")).is_none());
        assert!(ArtworkKey::new(ORIGIN, ACCOUNT, 1, Some("not-a-uuid")).is_none());
        assert!(ArtworkKey::new(ORIGIN, ACCOUNT, 1, Some("ABC")).is_none());
        assert!(ArtworkKey::new("", ACCOUNT, 1, Some(ART)).is_none());
        let key = ArtworkKey::new(ORIGIN, ACCOUNT, 1, Some(ART)).unwrap();
        assert!(key.belongs_to(ORIGIN, ACCOUNT, 1));
        assert!(!key.belongs_to(OTHER_ORIGIN, ACCOUNT, 1));
        assert!(!key.belongs_to(ORIGIN, OTHER_ACCOUNT, 1));
        assert!(!key.belongs_to(ORIGIN, ACCOUNT, 2));
    }

    #[test]
    fn a_duplicate_reference_shares_one_request() {
        let mut store = ArtworkStore::new(1);
        assert_eq!(store.request(key(1, ART)), ArtworkDecision::Fetch);
        assert_eq!(store.request(key(1, ART)), ArtworkDecision::Waiting);
        assert_eq!(store.request(key(1, ART)), ArtworkDecision::Waiting);
        assert_eq!(store.inflight_len(), 1);
        assert_eq!(store.entries_len(), 1);
    }

    #[test]
    fn requests_are_bounded_and_the_queue_pumps_one_at_a_time() {
        let mut store = ArtworkStore::with_limits(1, small());
        assert_eq!(store.request(key(1, ART)), ArtworkDecision::Fetch);
        assert_eq!(store.request(key(1, OTHER_ART)), ArtworkDecision::Fetch);
        let third = "20000000-0000-4000-8000-000000000003";
        let fourth = "20000000-0000-4000-8000-000000000004";
        let fifth = "20000000-0000-4000-8000-000000000005";
        assert_eq!(store.request(key(1, third)), ArtworkDecision::Waiting);
        assert_eq!(store.request(key(1, fourth)), ArtworkDecision::Waiting);
        // The session entry bound is reached: the fifth row keeps its fallback.
        assert_eq!(store.request(key(1, fifth)), ArtworkDecision::Rejected);
        assert_eq!(store.inflight_len(), 2);
        let next = store
            .complete(&key(1, ART), Ok(url(4)))
            .expect("one queued request takes the freed slot");
        assert_eq!(next, key(1, third));
        assert_eq!(store.inflight_len(), 2);
        // The promoted request is already in flight, so re-requesting it never doubles it.
        assert_eq!(store.request(key(1, third)), ArtworkDecision::Waiting);
    }

    #[test]
    fn late_results_of_another_generation_origin_or_account_are_ignored() {
        let mut store = ArtworkStore::new(1);
        let old = key(1, ART);
        assert_eq!(store.request(old.clone()), ArtworkDecision::Fetch);
        store.reset(2);
        assert_eq!(
            store.complete(&old, Ok(url(4))),
            None,
            "a late cover of the previous session may never render"
        );
        assert!(store.status(&old).is_none());
        assert_eq!(store.request(old), ArtworkDecision::Rejected);
        // A key of the current generation but another account is never the same entry.
        let own = key(2, ART);
        let foreign = ArtworkKey::new(OTHER_ORIGIN, OTHER_ACCOUNT, 2, Some(ART)).unwrap();
        assert_eq!(store.request(own), ArtworkDecision::Fetch);
        store.complete(&foreign, Ok(url(4)));
        assert!(store.status(&foreign).is_none());
        assert_eq!(
            store.status(&key(2, ART).clone()),
            Some(&ArtworkStatus::Loading)
        );
    }

    #[test]
    fn denied_and_failed_covers_are_not_retried_automatically() {
        let mut store = ArtworkStore::new(1);
        assert_eq!(store.request(key(1, ART)), ArtworkDecision::Fetch);
        store.complete(&key(1, ART), Err(ArtworkError::Denied));
        assert_eq!(store.status(&key(1, ART)), Some(&ArtworkStatus::Denied));
        assert!(store.status(&key(1, ART)).unwrap().data_url().is_none());
        assert_eq!(store.request(key(1, ART)), ArtworkDecision::Waiting);

        assert_eq!(store.request(key(1, OTHER_ART)), ArtworkDecision::Fetch);
        store.complete(&key(1, OTHER_ART), Err(ArtworkError::Failed));
        assert_eq!(
            store.status(&key(1, OTHER_ART)),
            Some(&ArtworkStatus::Failed)
        );
        assert_eq!(store.request(key(1, OTHER_ART)), ArtworkDecision::Waiting);
    }

    #[test]
    fn a_canceled_fetch_may_be_requested_again() {
        let mut store = ArtworkStore::new(1);
        assert_eq!(store.request(key(1, ART)), ArtworkDecision::Fetch);
        store.complete(&key(1, ART), Err(ArtworkError::Canceled));
        assert!(store.status(&key(1, ART)).is_none());
        assert_eq!(store.request(key(1, ART)), ArtworkDecision::Fetch);
    }

    #[test]
    fn ready_covers_are_bounded_by_the_data_url_budget() {
        let mut store = ArtworkStore::with_limits(1, small());
        assert_eq!(store.request(key(1, ART)), ArtworkDecision::Fetch);
        assert_eq!(store.request(key(1, OTHER_ART)), ArtworkDecision::Fetch);
        store.complete(&key(1, ART), Ok(url(20)));
        store.complete(&key(1, OTHER_ART), Ok(url(20)));
        assert_eq!(
            store.status(&key(1, ART)),
            None,
            "the oldest ready cover is evicted first"
        );
        assert!(matches!(
            store.status(&key(1, OTHER_ART)),
            Some(ArtworkStatus::Ready(_))
        ));
        assert_eq!(
            store.ready_bytes(),
            "data:image/jpeg;base64,".len() + 20,
            "the budget counts the encoded data URL, not the raw payload"
        );
        // A single cover above the whole budget is never retained.
        let third = "20000000-0000-4000-8000-000000000003";
        assert_eq!(store.request(key(1, third)), ArtworkDecision::Fetch);
        store.complete(&key(1, third), Ok(url(64)));
        assert_eq!(store.status(&key(1, third)), Some(&ArtworkStatus::Failed));
    }

    #[test]
    fn entry_count_stays_bounded_during_long_scrolling() {
        let mut store = ArtworkStore::with_limits(1, small());
        for index in 0..6_u64 {
            let artwork = format!("20000000-0000-4000-8000-{index:012}");
            let decision = store.request(key(1, &artwork));
            if index < 4 {
                assert_ne!(decision, ArtworkDecision::Rejected);
            } else {
                assert_eq!(
                    decision,
                    ArtworkDecision::Rejected,
                    "no entry may be added beyond the session bound"
                );
            }
        }
        assert_eq!(store.entries_len(), 4);
    }

    #[test]
    fn status_labels_and_data_urls_are_the_only_renderer_contract() {
        assert_eq!(ArtworkStatus::Loading.label(), "loading");
        assert_eq!(ArtworkStatus::Denied.label(), "denied");
        assert_eq!(ArtworkStatus::Failed.label(), "failed");
        assert_eq!(ArtworkStatus::Loading.data_url(), None);
        assert_eq!(ArtworkStatus::Denied.data_url(), None);
        assert_eq!(ArtworkStatus::Failed.data_url(), None);
        let ready = ArtworkStatus::Ready(url(3));
        assert_eq!(ready.label(), "ready");
        assert_eq!(ready.data_url(), Some("data:image/jpeg;base64,AAA"));
    }
}
