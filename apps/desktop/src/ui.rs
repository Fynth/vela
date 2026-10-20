//! Native presentation. HTTP credentials and the one audio output stay in Rust.
use crate::{
    AudioPhase, AudioSnapshot, DownloadProgress, PreparedAudio,
    artwork::{ArtworkDecision, ArtworkKey, ArtworkStatus},
    audio::AudioEngine,
    client::VelaClient,
    mpris::{self, Mpris, RepeatKind as SystemRepeat},
    playback::{Command, MAX_QUEUE, PlaybackOrder, Repeat, Transition},
    presentation::{PresentationAttributes, Repair, SaveStatus, Theme},
};
use dioxus::prelude::*;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;
use vela_domain::{AlbumSummary, ArtistSummary, PublicTrack, Track, TrackPage, User};

#[path = "../../web/src/icons.rs"]
mod icons;
use icons::Icon;

// Preferences belong to the owned online queue. Other source adapters expose
// only modes they actually execute, without silently discarding preferences.
fn playback_modes(offline: bool, remote: bool, shuffle: bool, repeat: Repeat) -> (bool, Repeat) {
    if remote {
        (false, Repeat::Off)
    } else if offline {
        (
            false,
            if repeat == Repeat::One {
                Repeat::One
            } else {
                Repeat::Off
            },
        )
    } else {
        (shuffle, repeat)
    }
}

#[component]
fn WindowControls() -> Element {
    if !cfg!(target_os = "linux") {
        return rsx! {};
    }
    rsx! { div {class:"native-window-controls", role:"group", "aria-label":"Window controls",
        button {class:"icon-button", "aria-label":"Minimize window", title:"Minimize", onclick:move |_|dioxus::desktop::window().set_minimized(true), Icon {name:"minimize"}}
        button {class:"icon-button", "aria-label":"Maximize or restore window", title:"Maximize or restore", onclick:move |_|dioxus::desktop::window().toggle_maximized(), Icon {name:"maximize"}}
        button {class:"icon-button window-close", "aria-label":"Close window", title:"Close", onclick:move |_|dioxus::desktop::window().close(), Icon {name:"close"}}
    } }
}

#[derive(Clone)]
struct Lifetime(Arc<AtomicBool>);
impl Lifetime {
    fn get(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
    fn set(&self, value: bool) {
        self.0.store(value, Ordering::Release);
    }
}

#[derive(Clone, Default)]
struct Gate {
    generation: Rc<Cell<u64>>,
    cancel: Rc<std::cell::RefCell<Option<CancellationToken>>>,
}
impl Gate {
    fn cancel(&self) {
        self.generation.set(self.generation.get().wrapping_add(1));
        if let Some(cancel) = self.cancel.borrow_mut().take() {
            cancel.cancel();
        }
    }
    fn begin(&self, epoch: u64) -> Ticket {
        self.cancel();
        let cancel = CancellationToken::new();
        *self.cancel.borrow_mut() = Some(cancel.clone());
        Ticket {
            epoch,
            generation: self.generation.get(),
            gate: self.clone(),
            cancel,
        }
    }
    /// A token that ends with the current session/operation. Cover requests use it so a
    /// sign-out cancels every in-flight fetch without touching the audio source gate.
    fn child(&self) -> Option<CancellationToken> {
        self.cancel
            .borrow()
            .as_ref()
            .map(CancellationToken::child_token)
    }
}
#[derive(Clone)]
struct Ticket {
    epoch: u64,
    generation: u64,
    gate: Gate,
    cancel: CancellationToken,
}
impl Ticket {
    fn current(&self, ui: &Ui) -> bool {
        ui.alive.get()
            && !self.cancel.is_cancelled()
            && *ui.epoch.peek() == self.epoch
            && self.gate.generation.get() == self.generation
    }
}

#[derive(Clone)]
struct Account {
    epoch: u64,
    client: VelaClient,
    user: User,
    origin: String,
}

/// AUD-05: the local-only offline identity — the canonical origin plus the account
/// UUID that a private device copy is scoped by (AUD-06). It is deliberately *not* an
/// [`Account`]: it carries no HTTP client, no session token, no CSRF token and no
/// username, so it can never authorize a request. It exists only to name the private
/// copy namespace and to render a read/play-only screen when no server answers.
#[derive(Clone, Debug, PartialEq, Eq)]
struct OfflineSession {
    origin: String,
    account_id: String,
}
#[derive(Clone)]
struct Catalog {
    epoch: u64,
    query: String,
    cursor: Option<String>,
    favorites: bool,
    page: Arc<TrackPage>,
}

/// Serialised device-document writer. Only the newest pending value is written, so
/// dragging a slider or clicking a stepper cannot queue an unbounded number of
/// blocking writes or publish an older value over a newer one.
#[derive(Default)]
struct DeviceWrite {
    busy: bool,
    dirty: Option<crate::presentation::Preferences>,
}

#[derive(Clone)]
struct Ui {
    alive: Lifetime,
    audio: Signal<Option<AudioEngine>>,
    audio_error: Signal<String>,
    engine_cleanup: Rc<RefCell<Option<AudioEngine>>>,
    login_gate: Gate,
    catalog_gate: Gate,
    feedback_gate: Gate,
    playlist_gate: Gate,
    playlist_write_gate: Gate,
    folder_gate: Gate,
    browse_gate: Gate,
    remote_gate: Gate,
    /// One save-to-device operation.
    offline_gate: Gate,
    /// Saved-copy list operations: sign-in load/verify, removal and its reload.
    offline_list_gate: Gate,
    /// One offline playback verification. Exists before the blocking worker
    /// starts so Cancel always wins (AUD-03).
    offline_play_gate: Gate,
    source_gate: Gate,
    logout_gate: Gate,
    /// One app session of cover requests. Sign-out and account/origin changes cancel it,
    /// so old-account art is never still loading into the new session.
    artwork_gate: Gate,
    epoch: Signal<u64>,
    selection: Signal<u64>,
    generation_clock: Rc<Cell<u64>>,
    account: Signal<Option<Account>>,
    /// Bounded, deduplicated native cover presentation state (no credentials).
    artwork: Signal<crate::artwork::ArtworkStore>,
    origin: Signal<String>,
    username: Signal<String>,
    password: Signal<String>,
    connecting: Signal<bool>,
    resuming: Signal<bool>,
    remember: Signal<bool>,
    saved_session: Signal<Option<crate::session_store::SavedSession>>,
    login_error: Signal<String>,
    view: Signal<&'static str>,
    /// Device-only appearance preferences for this installation. They survive
    /// sign-in/sign-out, carry no account data and never trigger a request.
    device: Signal<crate::presentation::Preferences>,
    device_save: Signal<SaveStatus>,
    /// Set when the stored document had to be repaired; explained once in Settings.
    device_repair: Signal<Option<Repair>>,
    device_write: Rc<RefCell<DeviceWrite>>,
    /// Abbreviated config path of the device document, or empty when unavailable.
    device_store: Rc<String>,
    query_draft: Signal<String>,
    query: Signal<String>,
    cursor: Signal<Option<String>>,
    catalog: Signal<Option<Catalog>>,
    catalog_loading: Signal<bool>,
    catalog_error: Signal<String>,
    favorite_busy: Signal<String>,
    playlists: Signal<Vec<vela_domain::PlaylistSummary>>,
    playlist: Signal<Option<vela_domain::PlaylistDetail>>,
    playlist_id: Signal<Option<String>>,
    playlist_loading: Signal<bool>,
    playlist_error: Signal<String>,
    playlist_name_draft: Signal<String>,
    folders: Signal<Vec<vela_domain::PlaylistFolder>>,
    folder_filter: Signal<Option<String>>,
    folder_name_draft: Signal<String>,
    folder_rename_draft: Signal<String>,
    folder_busy: Signal<bool>,
    folder_confirm: Signal<String>,
    playlist_folder_target: Signal<String>,
    playlist_candidates: Signal<Vec<Track>>,
    playlist_candidate: Signal<String>,
    playlist_busy: Signal<bool>,
    playlist_delete_confirm: Signal<bool>,
    browse_kind: Signal<&'static str>,
    albums: Signal<Vec<AlbumSummary>>,
    artists: Signal<Vec<ArtistSummary>>,
    browse_tracks: Signal<Vec<Track>>,
    browse_id: Signal<Option<String>>,
    browse_title: Signal<String>,
    browse_loading: Signal<bool>,
    browse_error: Signal<String>,
    remote_origin: Signal<String>,
    remote_preview: Signal<Option<crate::remote::RemotePreview>>,
    remote_loading: Signal<bool>,
    remote_error: Signal<String>,
    remote_now: Signal<Option<crate::remote::RemoteNow>>,
    remote_queue: Signal<Vec<crate::remote::RemoteItem>>,
    remote_index: Signal<usize>,
    remote_query: Signal<String>,
    remote_results: Signal<Vec<crate::remote::PeerSearch>>,
    remote_searching: Signal<bool>,
    remote_search_error: Signal<String>,
    remote_more_busy: Signal<bool>,
    remote_more_error: Signal<String>,
    remote_peer_more_busy: Signal<String>,
    remote_peer_more_error: Signal<String>,
    remote_retry_busy: Signal<String>,
    remote_retry_error: Signal<String>,
    offline: Signal<Vec<crate::offline::OfflineTrack>>,
    offline_busy: Signal<String>,
    offline_error: Signal<String>,
    offline_corrupt: Signal<Vec<String>>,
    offline_active: Signal<bool>,
    /// AUD-05: the private offline profile this device holds and whose namespace has
    /// real saved copies. `Some` makes the offline screen reachable and nothing else:
    /// it never enables a server route, an account or a credential.
    offline_local: Signal<Option<OfflineSession>>,
    /// AUD-05: the local-only offline screen that is on screen now. While this is
    /// `Some` no account is adopted; a real sign-in replaces it.
    offline_session: Signal<Option<OfflineSession>>,
    offline_confirm: Signal<String>,
    cache_usage: Signal<Option<(usize, u64)>>,
    cache_busy: Signal<bool>,
    cache_confirm: Signal<bool>,
    restored_position: Signal<u64>,
    /// The saved copy currently being verified off the UI thread (AUD-03).
    offline_checking: Signal<Option<crate::offline::OfflineTrack>>,
    /// The copy whose removal is in flight; removals are single-flight so an
    /// older operation can never overwrite a newer list.
    offline_removing: Signal<String>,
    saved_peers: Signal<Vec<String>>,
    queue: Signal<Vec<Track>>,
    order: Signal<PlaybackOrder>,
    shuffle: Signal<bool>,
    repeat: Signal<Repeat>,
    context: Signal<String>,
    snapshot: Signal<AudioSnapshot>,
    /// Last audible level remembered for the player's Mute/Unmute pair. It is the
    /// engine's own volume that is muted to zero, so there is still exactly one
    /// output authority; this is only the value to restore.
    volume_restore: Signal<f32>,
    preparing: Signal<bool>,
    source_canceled: Signal<bool>,
    /// Requested transport state; independent of delayed decoder snapshots.
    wants_play: Signal<bool>,
    progress: Signal<DownloadProgress>,
    prepared: Signal<Option<Arc<PreparedAudio>>>,
    source_error: Signal<String>,
    notice: Signal<String>,
    ended: Rc<Cell<Option<u64>>>,
    mpris: Mpris,
    mpris_note: Signal<&'static str>,
    mpris_seek_serial: Signal<u64>,
}
impl Ui {
    fn signed_in(&self) -> bool {
        self.alive.get()
            && self
                .account
                .peek()
                .as_ref()
                .is_some_and(|account| account.epoch == *self.epoch.peek())
    }
    /// AUD-05: the local-only offline library is on screen. This is not a signed-in
    /// state: there is no account, no client and no credential.
    fn offline_mode(&self) -> bool {
        self.alive.get() && self.offline_session.peek().is_some()
    }
    /// Transport and the private saved copies are usable for a real signed-in
    /// account or for the local offline library. Every write, search, save, removal
    /// and server route still requires [`Self::signed_in`].
    fn can_play(&self) -> bool {
        self.signed_in() || self.offline_mode()
    }
    fn audio_alive(&self) -> bool {
        self.audio.read().as_ref().is_some_and(AudioEngine::alive)
    }
    fn audio_ready(&self) -> bool {
        self.audio_alive()
    }
    fn current_track(&self) -> Option<Track> {
        if !self.can_play() {
            return None;
        }
        self.order
            .peek()
            .current()
            .and_then(|at| self.queue.peek().get(at).cloned())
    }
    fn source_current(&self, ticket: &Ticket, selection: u64, id: &str) -> bool {
        ticket.current(self)
            && self.signed_in()
            && *self.selection.peek() == selection
            && self.current_track().is_some_and(|track| track.id == id)
    }
    /// The cover key of the current account/origin/session generation, or `None` when
    /// the id is absent/invalid or no account is current. Lists only pass the server's
    /// artwork id; the renderer never derives a URL, path or credential.
    fn artwork_key(&self, artwork_id: Option<&str>) -> Option<ArtworkKey> {
        if !self.signed_in() {
            return None;
        }
        let account = self.account.peek();
        let account = account
            .as_ref()
            .filter(|account| account.epoch == *self.epoch.peek())?;
        ArtworkKey::new(
            &account.origin,
            &account.user.id,
            *self.epoch.peek(),
            artwork_id,
        )
    }
    fn artwork_status(&self, key: &ArtworkKey) -> Option<ArtworkStatus> {
        // `read` subscribes the calling component to cover updates; `peek` cannot.
        self.artwork.read().status(key).cloned()
    }
    fn cancel_all(&self) {
        self.login_gate.cancel();
        self.catalog_gate.cancel();
        self.feedback_gate.cancel();
        self.playlist_gate.cancel();
        self.playlist_write_gate.cancel();
        self.browse_gate.cancel();
        self.remote_gate.cancel();
        self.folder_gate.cancel();
        self.offline_gate.cancel();
        self.offline_list_gate.cancel();
        self.offline_play_gate.cancel();
        self.source_gate.cancel();
        self.logout_gate.cancel();
        self.artwork_gate.cancel();
    }
    fn new_selection(&mut self) -> u64 {
        self.source_gate.cancel();
        // AUD-03: a pending offline verification belongs to the previous
        // selection, so its blocking worker is canceled here as well.
        self.offline_play_gate.cancel();
        self.offline_checking.set(None);
        let selection = self.generation_clock.get().wrapping_add(1);
        self.generation_clock.set(selection);
        self.selection.set(selection);
        self.ended.set(None);
        self.preparing.set(false);
        self.source_canceled.set(false);
        self.wants_play.set(false);
        self.progress.set(DownloadProgress::default());
        self.source_error.set(String::new());
        let volume = self.snapshot.peek().volume;
        self.snapshot.set(AudioSnapshot {
            generation: selection,
            volume,
            ..AudioSnapshot::default()
        });
        if let Some(audio) = self.audio.peek().clone()
            && let Err(error) = audio.stop(selection)
        {
            self.source_error.set(error);
        }
        selection
    }
    fn clear_account(&mut self) {
        self.cancel_all();
        let epoch = self.epoch.peek().wrapping_add(1);
        self.epoch.set(epoch);
        // Every cover of the previous account is dropped, not merely hidden: no data
        // URL of a signed-out account survives in this session's presentation state.
        self.artwork.write().reset(epoch);
        self.new_selection();
        self.account.set(None);
        // The in-memory copy of the saved session ends with the account. The
        // keyring item is deleted separately and never on this thread (AUD-02).
        self.saved_session.set(None);
        self.password.set(String::new());
        self.connecting.set(false);
        self.catalog.set(None);
        self.query.set(String::new());
        self.query_draft.set(String::new());
        self.cursor.set(None);
        self.catalog_loading.set(false);
        self.catalog_error.set(String::new());
        self.favorite_busy.set(String::new());
        self.playlists.set(Vec::new());
        self.playlist.set(None);
        self.playlist_id.set(None);
        self.playlist_loading.set(false);
        self.playlist_error.set(String::new());
        self.playlist_name_draft.set(String::new());
        self.folders.set(Vec::new());
        self.folder_filter.set(None);
        self.folder_name_draft.set(String::new());
        self.folder_rename_draft.set(String::new());
        self.folder_busy.set(false);
        self.folder_confirm.set(String::new());
        self.playlist_folder_target.set(String::new());
        self.playlist_candidates.set(Vec::new());
        self.playlist_candidate.set(String::new());
        self.playlist_busy.set(false);
        self.playlist_delete_confirm.set(false);
        self.albums.set(Vec::new());
        self.artists.set(Vec::new());
        self.browse_tracks.set(Vec::new());
        self.browse_id.set(None);
        self.browse_title.set(String::new());
        self.browse_loading.set(false);
        self.browse_error.set(String::new());
        self.remote_preview.set(None);
        self.remote_loading.set(false);
        self.remote_error.set(String::new());
        self.remote_now.set(None);
        self.remote_queue.set(Vec::new());
        self.remote_index.set(0);
        self.remote_query.set(String::new());
        self.remote_results.set(Vec::new());
        self.remote_searching.set(false);
        self.remote_search_error.set(String::new());
        self.remote_more_busy.set(false);
        self.remote_more_error.set(String::new());
        self.remote_peer_more_busy.set(String::new());
        self.remote_peer_more_error.set(String::new());
        self.remote_retry_busy.set(String::new());
        self.remote_retry_error.set(String::new());
        self.offline.set(Vec::new());
        self.offline_busy.set(String::new());
        self.offline_error.set(String::new());
        self.offline_corrupt.set(Vec::new());
        // AUD-05: an explicit sign-out also ends the local offline screen and
        // withdraws the profile hint (the saved copies stay on this device).
        self.offline_local.set(None);
        self.offline_session.set(None);
        self.cache_usage.set(None);
        self.cache_busy.set(false);
        self.cache_confirm.set(false);
        self.restored_position.set(0);
        self.offline_active.set(false);
        self.offline_confirm.set(String::new());
        self.offline_checking.set(None);
        self.offline_removing.set(String::new());
        self.queue.set(Vec::new());
        self.order.set(PlaybackOrder::default());
        self.shuffle.set(false);
        self.repeat.set(Repeat::Off);
        self.context.set(String::new());
        self.prepared.set(None);
        self.source_error.set(String::new());
        self.notice.set(String::new());
        self.view.set("library");
    }
    /// Publish a bounded, credential-free snapshot to the OS media session.
    /// Ranking, HTTP and audio stay owned by this UI; MPRIS only observes.
    fn publish_system_controls(&self) {
        if self.remote_now.peek().is_some() {
            self.mpris.publish(mpris::Update {
                status: mpris::Status::Stopped,
                track: None,
                position_ms: 0,
                volume: self.snapshot.peek().volume as f64,
                shuffle: false,
                repeat: SystemRepeat::Off,
                can_play: false,
                can_pause: false,
                can_seek: false,
                can_go_next: false,
                can_go_previous: false,
                seek_serial: *self.mpris_seek_serial.peek(),
            });
            return;
        }
        let track = self.current_track();
        let snapshot = self.snapshot.peek().clone();
        let preparing = *self.preparing.peek();
        let (shuffle, effective_repeat) = playback_modes(
            *self.offline_active.peek(),
            false,
            *self.shuffle.peek(),
            *self.repeat.peek(),
        );
        let repeat = match effective_repeat {
            Repeat::Off => SystemRepeat::Off,
            Repeat::All => SystemRepeat::All,
            Repeat::One => SystemRepeat::One,
        };
        let status = if track.is_none() {
            mpris::Status::Stopped
        } else if snapshot.phase == AudioPhase::Playing {
            mpris::Status::Playing
        } else if preparing || matches!(snapshot.phase, AudioPhase::Paused | AudioPhase::Loading) {
            mpris::Status::Paused
        } else {
            mpris::Status::Stopped
        };
        let can_play = track.is_some() && self.audio_alive();
        let can_seek = can_play
            && self.prepared.peek().is_some()
            && snapshot.duration_ms > 0
            && matches!(
                snapshot.phase,
                AudioPhase::Playing | AudioPhase::Paused | AudioPhase::Ended
            );
        let queue_len = self.queue.peek().len();
        self.mpris.publish(mpris::Update {
            status,
            track: track.map(|track| mpris::Track {
                id: track.id,
                title: track.title,
                artist: track.artist,
                album: track.album,
                duration_ms: track.duration_ms.max(0) as u64,
            }),
            position_ms: if can_play { snapshot.position_ms } else { 0 },
            volume: snapshot.volume as f64,
            shuffle,
            repeat,
            can_play,
            can_pause: can_play
                && matches!(snapshot.phase, AudioPhase::Playing | AudioPhase::Paused),
            can_seek,
            can_go_next: queue_len > 1 || repeat != SystemRepeat::Off,
            can_go_previous: queue_len > 1 || repeat == SystemRepeat::All,
            seek_serial: *self.mpris_seek_serial.peek(),
        });
    }
    fn catalog_current(&self, catalog: &Catalog) -> bool {
        self.signed_in()
            && catalog.epoch == *self.epoch.peek()
            && catalog.query == *self.query.peek()
            && catalog.cursor == *self.cursor.peek()
            && catalog.favorites == (*self.view.peek() == "favorites")
            && !*self.catalog_loading.peek()
            && self.catalog_error.peek().is_empty()
    }
    /// True only for a key of the exact account, origin and session generation that is
    /// current now. A stale row can never start a fetch for a previous account.
    fn artwork_key_is_current(&self, key: &ArtworkKey) -> bool {
        if !self.signed_in() {
            return false;
        }
        let account = self.account.peek();
        account.as_ref().is_some_and(|account| {
            key.belongs_to(&account.origin, &account.user.id, *self.epoch.peek())
        })
    }
    /// Root wrapper attributes for the validated device preference.
    ///
    /// `read` (not `peek`) subscribes this render to size/theme changes; the change
    /// rewrites one `style` and one `data-theme` attribute on the existing wrapper,
    /// so the audio engine, prepared source, queue, selection and position are never
    /// remounted and no request is made.
    fn device_attributes(&self) -> PresentationAttributes {
        crate::presentation::attributes(*self.device.read())
    }
}
/// Component props only need identity: every native surface shares this one live state
/// by reference. The cover's own `artwork_id` prop still compares by value, so a
/// recycled row with different artwork is never memoized away.
impl PartialEq for Ui {
    fn eq(&self, other: &Self) -> bool {
        std::rc::Rc::ptr_eq(&self.generation_clock, &other.generation_clock)
    }
}

/// One native cover slot. Only already-authorized image bytes reach the renderer, as a
/// self-contained `data:` URL with no cookie, token or origin; every other state keeps
/// the existing neutral Vela cover, so there is no broken-image glyph and no retry
/// storm. The request key is the artwork's own id, never a row index.
#[component]
fn ArtworkCover(
    ui: Ui,
    artwork_id: Option<String>,
    class: &'static str,
    icon: &'static str,
) -> Element {
    let key = ui.artwork_key(artwork_id.as_deref());
    let status = key.as_ref().and_then(|key| ui.artwork_status(key));
    let request_ui = ui.clone();
    let dependency = key.clone();
    use_effect(use_reactive((&dependency,), move |(key,)| {
        if let Some(key) = key {
            request_artwork(request_ui.clone(), key);
        }
    }));
    let state = cover_state(key.is_some(), status.as_ref());
    if let Some(ArtworkStatus::Ready(url)) = status {
        return rsx! { img {class: class, src: "{url}", alt: "", "data-native-artwork": "ready", loading: "lazy", decoding: "async"} };
    }
    rsx! { div {class: class, "aria-hidden": "true", "data-native-artwork": "{state}", Icon {name: icon}} }
}

/// The renderer state of one cover slot: an absent/invalid artwork id is `missing`, a
/// known presentation state keeps its own name, and a key that has not answered yet is
/// `pending`. Every non-`ready` state renders the existing neutral Vela cover.
fn cover_state(has_key: bool, status: Option<&ArtworkStatus>) -> &'static str {
    match (has_key, status) {
        (false, _) => "missing",
        (true, Some(status)) => status.label(),
        (true, None) => "pending",
    }
}

/// Ask the bounded presentation store for one cover. Repeated renders, duplicate rows
/// and already-answered ids never spawn another request.
fn request_artwork(mut ui: Ui, key: ArtworkKey) {
    if !ui.artwork_key_is_current(&key) {
        return;
    }
    if ui.artwork.peek().status(&key).is_some() {
        return;
    }
    if ui.artwork.write().request(key.clone()) == ArtworkDecision::Fetch {
        spawn_artwork(ui, key);
    }
}

/// The one network task of one admitted cover request. The fetch shares the artwork
/// session token, so sign-out cancels it; a late answer is rejected by the store's
/// generation, and a cover failure never touches the audio source.
fn spawn_artwork(mut ui: Ui, key: ArtworkKey) {
    let Some(account) = ui.account.peek().clone() else {
        return;
    };
    let Some(cancel) = ui.artwork_gate.child() else {
        return;
    };
    spawn(async move {
        let result = account.client.artwork(key.artwork_id(), cancel).await;
        let status = result.map(|art| std::sync::Arc::<str>::from(art.data_url()));
        let mut store = ui.artwork.write();
        let next = store.complete(&key, status);
        drop(store);
        if let Some(next) = next {
            spawn_artwork(ui.clone(), next);
        }
    });
}

fn default_origin() -> String {
    let fallback = "http://127.0.0.1:3210";
    let value = std::env::var("VELA_DESKTOP_SERVER_URL").unwrap_or_else(|_| fallback.into());
    // Use the same canonical-origin policy as the HTTP client so a malformed
    // environment value can never prefill a form that the client would reject.
    if value.len() <= 2048 && crate::client::canonical_origin(&value).is_ok() {
        value
    } else {
        fallback.into()
    }
}

fn login(mut ui: Ui) {
    if !ui.alive.get() || ui.signed_in() || *ui.connecting.peek() {
        return;
    }
    if ui.audio.peek().is_none() && ui.audio_error.peek().is_empty() {
        return;
    }
    // A previous server sign-out cannot replace feedback for this new attempt.
    ui.logout_gate.cancel();
    let origin = ui.origin.peek().trim().to_string();
    let username = ui.username.peek().trim().to_string();
    let password = ui.password.peek().clone();
    let remember = *ui.remember.peek();
    if origin.is_empty() || username.is_empty() || password.is_empty() {
        ui.login_error
            .set("Enter your server origin, username and password.".into());
        return;
    }
    let ticket = ui.login_gate.begin(*ui.epoch.peek());
    // AUD-01: this attempt claims the saved item. An older attempt may no longer
    // write it, even while its helper process is still running.
    let attempt = crate::session_store::begin_attempt();
    ui.password.set(String::new());
    ui.connecting.set(true);
    ui.login_error.set(String::new());
    ui.notice.set(String::new());
    spawn(async move {
        if !ticket.current(&ui) {
            return;
        }
        let result =
            VelaClient::connect(&origin, username.clone(), password, ticket.cancel.clone()).await;
        if !login_still_current(&ui, &ticket, &origin, &username) {
            return;
        }
        ui.connecting.set(false);
        match result {
            Ok((client, auth)) => {
                let token = client.session_token();
                if remember {
                    let save_origin = origin.clone();
                    let stored = tokio::task::spawn_blocking(move || {
                        crate::session_store::save(attempt, &save_origin, &token)
                    })
                    .await;
                    // AUD-01: the keyring may outlive this wait. A canceled or
                    // replaced login publishes no feedback and adopts nothing.
                    if !login_still_current(&ui, &ticket, &origin, &username) {
                        return;
                    }
                    match stored {
                        Ok(Ok(crate::session_store::StoreStatus::Stored)) => {
                            ui.notice.set("Session saved on this device.".into());
                        }
                        // A newer attempt owns the item; it reports for itself.
                        Ok(Ok(crate::session_store::StoreStatus::Superseded)) => {}
                        Ok(Err(error)) => ui.notice.set(error.message().to_owned()),
                        Err(_) => ui
                            .notice
                            .set("The session could not be saved on this device.".into()),
                    }
                }
                // AUD-01: adoption and every later UI mutation is guarded again.
                if !login_still_current(&ui, &ticket, &origin, &username) {
                    return;
                }
                adopt_account(ui, client, auth, origin, ticket.epoch);
            }
            Err(error) => ui.login_error.set(error),
        }
    });
}

/// AUD-01: the account may only be adopted while this exact login attempt still
/// owns the UI: same ticket/epoch and the same origin and username the attempt
/// was started with.
fn login_still_current(ui: &Ui, ticket: &Ticket, origin: &str, username: &str) -> bool {
    ticket.current(ui) && ui.origin.peek().trim() == origin && ui.username.peek().trim() == username
}

fn save_queue_snapshot(ui: Ui) {
    let Some(account) = ui.account.peek().clone() else {
        return;
    };
    let queue = ui.queue.peek().clone();
    if queue.is_empty() {
        return;
    }
    let Some(current) = ui.order.read().current() else {
        return;
    };
    let repeat = match *ui.repeat.peek() {
        Repeat::Off => "off",
        Repeat::All => "all",
        Repeat::One => "one",
    };
    if let Ok(snapshot) = crate::queue_store::QueueSnapshot::new(
        &account.origin,
        &account.user.id,
        queue,
        current,
        ui.snapshot.peek().position_ms,
        *ui.shuffle.peek(),
        repeat,
        &ui.context.peek(),
    ) {
        spawn(async move {
            let _ = tokio::task::spawn_blocking(move || crate::queue_store::save(&snapshot)).await;
        });
    }
}

fn restore_queue_snapshot(mut ui: Ui) {
    let Some(account) = ui.account.peek().clone() else {
        return;
    };
    let epoch = account.epoch;
    spawn(async move {
        let loaded = tokio::task::spawn_blocking({
            let origin = account.origin.clone();
            let user = account.user.id.clone();
            move || crate::queue_store::load(&origin, &user)
        })
        .await;
        if !ui.alive.get()
            || *ui.epoch.peek() != epoch
            || !ui.signed_in()
            || !ui.queue.peek().is_empty()
        {
            return;
        }
        if let Ok(Ok(Some(snapshot))) = loaded {
            let len = snapshot.queue.len();
            ui.queue.set(snapshot.queue);
            ui.order.write().reset(len, snapshot.current);
            ui.shuffle.set(snapshot.shuffle);
            ui.repeat.set(match snapshot.repeat.as_str() {
                "all" => Repeat::All,
                "one" => Repeat::One,
                _ => Repeat::Off,
            });
            ui.context.set(snapshot.context);
            ui.restored_position.set(snapshot.position_ms);
        }
    });
}

fn adopt_account(
    mut ui: Ui,
    client: crate::client::VelaClient,
    auth: vela_domain::AuthResponse,
    origin: String,
    epoch: u64,
) {
    // AUD-05: a real sign-in replaces the local offline screen. Its transient state
    // (offline queue, decoder source, projected list) is dropped here; the private
    // saved copies and the profile file are not touched.
    leave_offline_mode(&mut ui);
    ui.login_error.set(String::new());
    ui.account.set(Some(Account {
        epoch,
        client,
        user: auth.user,
        origin,
    }));
    ui.offline.set(Vec::new());
    // A fresh account/origin/session starts with no cover state and one cancel token
    // for every cover request that belongs to it.
    ui.artwork.write().reset(epoch);
    ui.artwork_gate.begin(epoch);
    // AUD-08: the saved-copy list is read and verified off this thread, and only
    // the account that started the read may publish its result.
    let ticket = ui.offline_list_gate.begin(epoch);
    let Some(owner) = ui
        .account
        .peek()
        .as_ref()
        .map(|account| OfflineOwner::new(&ui.offline_list_gate, &ticket, account))
    else {
        return;
    };
    let catalog_ui = ui.clone();
    spawn(async move {
        let scope = OfflineScope::new(&owner);
        let loaded = tokio::task::spawn_blocking({
            let scope = scope.clone();
            move || crate::offline::load(&scope.origin, &scope.account_id)
        })
        .await;
        if !owner.current(&ui, &ticket) {
            return;
        }
        match loaded {
            Ok(Ok(tracks)) => ui.offline.set(tracks),
            Ok(Err(error)) => ui.offline_error.set(error),
            Err(_) => ui
                .offline_error
                .set("Saved offline music could not be read.".into()),
        }
        let verified = tokio::task::spawn_blocking({
            let cancel = ticket.cancel.clone();
            move || {
                crate::offline::verify(&scope.origin, &scope.account_id, &|| cancel.is_cancelled())
            }
        })
        .await;
        // AUD-08: the integrity sweep awaited the reload; an old account may not
        // publish its notice or its corrupt list any more.
        if !owner.current(&ui, &ticket) {
            return;
        }
        if let Ok(Ok(report)) = verified {
            if !report.corrupt.is_empty() {
                ui.notice.set(format!(
                    "{} offline copy(ies) failed their integrity check. Save them again.",
                    report.corrupt.len()
                ));
            }
            ui.offline_corrupt.set(report.corrupt);
        }
    });
    restore_queue_snapshot(catalog_ui.clone());
    // AUD-05: the private, non-secret offline profile is written after a real
    // sign-in/resume so a later cold start can name this account's saved copies
    // without any server or stored credential. Local file work only.
    remember_offline_profile(catalog_ui.clone());
    load_catalog(catalog_ui, String::new(), None, false);
}

/// AUD-05: fixed honest message for a private offline namespace that exists but
/// cannot be read. It never claims the copies are gone.
const OFFLINE_UNREADABLE: &str = "Saved offline music could not be read.";

/// AUD-05: record the private offline profile for the currently signed-in account.
/// Best effort and local-only: the document holds a canonical origin and an account
/// UUID, never a token, password, CSRF value, media path or username. A failure is
/// reported honestly, because it means cold offline playback needs one more sign-in.
fn remember_offline_profile(mut ui: Ui) {
    let Some(account) = ui.account.peek().clone().filter(|_| ui.signed_in()) else {
        return;
    };
    let epoch = *ui.epoch.peek();
    let (origin, account_id) = (account.origin.clone(), account.user.id.clone());
    // Claimed before the blocking write, so an explicit sign-out that happens while
    // the write is queued supersedes it and the withdrawn hint cannot reappear
    // (AUD-01/AUD-05).
    let attempt = crate::offline_profile::begin_attempt();
    spawn(async move {
        let stored = tokio::task::spawn_blocking(move || {
            crate::offline_profile::save(attempt, &origin, &account_id)
        })
        .await;
        if !ui.alive.get() || *ui.epoch.peek() != epoch || !ui.signed_in() {
            return;
        }
        match stored {
            Ok(Ok(crate::offline_profile::StoreOutcome::Stored))
            | Ok(Ok(crate::offline_profile::StoreOutcome::Superseded)) => {}
            _ => ui.notice.set(
                "Saved offline copies will need a sign-in after a restart: the private offline profile could not be stored on this device."
                    .into(),
            ),
        }
    });
}

/// What the local-only offline check found before any server contact (AUD-05).
enum OfflineEvidence {
    /// No usable profile, or a valid profile whose namespace has no real copies.
    None,
    /// A usable profile plus the real saved copies of exactly its namespace.
    Ready(OfflineSession, Vec<crate::offline::OfflineTrack>),
    /// The profile or its namespace could not be read. The caller shows the ordinary
    /// sign-in screen with this honest reason instead of guessing.
    Unreadable(String),
}

/// Read the private offline profile and its real saved copies. Purely local: no
/// request, no credential and no path outside the private data tree. A corrupt or
/// foreign profile is already reported as `None` by the store and never deletes or
/// rewrites a copy.
fn read_offline_evidence() -> OfflineEvidence {
    let profile = match crate::offline_profile::load() {
        Ok(Some(profile)) => profile,
        Ok(None) => return OfflineEvidence::None,
        Err(error) => return OfflineEvidence::Unreadable(error),
    };
    match crate::offline::load(&profile.origin, &profile.account_id) {
        Ok(tracks) if tracks.is_empty() => OfflineEvidence::None,
        Ok(tracks) => OfflineEvidence::Ready(
            OfflineSession {
                origin: profile.origin,
                account_id: profile.account_id,
            },
            tracks,
        ),
        Err(error) => OfflineEvidence::Unreadable(error),
    }
}

/// AUD-05 cold-start sequence, in this order and only with bounded local work:
///
/// 1. read the private offline profile and its real saved copies (no network);
/// 2. read the opt-in saved session exactly as before (bounded keyring helper);
/// 3. if a saved session exists for the profile's canonical origin, resume it once
///    through the existing guarded path. A real sign-in wins; an unreachable,
///    expired, unusable or refused server falls through to the offline library;
/// 4. with no saved session, or no saved session for this origin, open the offline
///    library directly.
///
/// Nothing is played automatically, no request is retried and no account is invented.
async fn startup(mut ui: Ui) {
    if !ui.alive.get() {
        return;
    }
    let generation = ui.login_gate.generation.get();
    let evidence = tokio::task::spawn_blocking(read_offline_evidence)
        .await
        .unwrap_or(OfflineEvidence::None);
    if !ui.alive.get() {
        return;
    }
    // Claim the item for this read: a cleanup may only delete the record that was
    // actually read, never one a newer login already saved (AUD-01).
    let attempt = crate::session_store::begin_attempt();
    let loaded = tokio::task::spawn_blocking(crate::session_store::load).await;
    if !ui.alive.get() || ui.signed_in() || ui.login_gate.generation.get() != generation {
        return;
    }
    let mut cleanup = None;
    match loaded {
        Ok(Ok(Some(saved))) => ui.saved_session.set(Some(saved)),
        Ok(Ok(None)) => {}
        Ok(Err(error)) => {
            ui.login_error.set(error.message().to_owned());
            // AUD-04/AUD-11: only a malformed record is cleaned up, and only after
            // the user-visible error is published.
            if error.is_corrupt_record() {
                cleanup = Some(attempt);
            }
        }
        Err(_) => ui.login_error.set(
            crate::session_store::KeyringError::Unavailable
                .message()
                .to_owned(),
        ),
    }
    if let Some(attempt) = cleanup {
        let _ = delete_saved_session(move || crate::session_store::clear_attempt(attempt)).await;
        if !ui.alive.get() || ui.login_gate.generation.get() != generation {
            return;
        }
    }
    let (session, tracks) = match evidence {
        OfflineEvidence::None => return,
        OfflineEvidence::Unreadable(error) => {
            // Requirement of AUD-05: a failed profile/copy lookup never guesses; the
            // ordinary sign-in screen stays and says what happened.
            ui.login_error.set(error);
            return;
        }
        OfflineEvidence::Ready(session, tracks) => (session, tracks),
    };
    ui.offline_local.set(Some(session.clone()));
    let resume = ui
        .saved_session
        .peek()
        .clone()
        .filter(|saved| saved.origin == session.origin);
    match resume {
        Some(_) => resume_attempt(ui, Some((session, tracks))),
        None => enter_offline_mode(ui, session, tracks),
    }
}

/// AUD-05: open the local-only offline screen. It is a read/play surface: no account
/// is adopted, no request is made, no queue is uploaded and no listening event
/// exists. The integrity sweep runs on a blocking worker and nothing is played until
/// the listener presses Play.
fn enter_offline_mode(
    mut ui: Ui,
    session: OfflineSession,
    tracks: Vec<crate::offline::OfflineTrack>,
) {
    if !ui.alive.get() || ui.signed_in() {
        return;
    }
    ui.offline_local.set(Some(session.clone()));
    ui.offline_session.set(Some(session.clone()));
    ui.offline.set(tracks);
    ui.offline_error.set(String::new());
    ui.offline_checking.set(None);
    ui.offline_active.set(false);
    ui.wants_play.set(false);
    sweep_offline_copies(ui, session);
}

/// AUD-05: reopen an offline library the profile already vouched for (for example
/// after visiting the sign-in screen). Local reload only.
fn open_offline_library(mut ui: Ui) {
    if !ui.alive.get() || ui.signed_in() {
        return;
    }
    let Some(session) = ui.offline_local.peek().clone() else {
        return;
    };
    if ui.offline_session.peek().is_some() {
        return;
    }
    ui.offline_session.set(Some(session.clone()));
    ui.offline_checking.set(None);
    ui.offline_active.set(false);
    ui.wants_play.set(false);
    refresh_offline_copies(ui);
}

/// AUD-05: leave the local-only offline screen and drop its transient state. The
/// private profile file, the saved copies and the device preferences stay exactly
/// as they are; this only withdraws the current presentation and its decoder.
fn leave_offline_mode(ui: &mut Ui) {
    if ui.offline_session.peek().is_none() {
        return;
    }
    ui.offline_session.set(None);
    ui.offline_list_gate.cancel();
    ui.offline_play_gate.cancel();
    ui.offline_checking.set(None);
    ui.new_selection();
    ui.prepared.set(None);
    ui.offline_active.set(false);
    ui.offline_error.set(String::new());
    ui.queue.set(Vec::new());
    ui.order.write().reset(0, 0);
    ui.context.set(String::new());
    ui.source_error.set(String::new());
}

/// Reload the saved-copy list of the offline screen from this device, then sweep its
/// integrity. Local file work only; a late result publishes only while the same
/// profile is still the live offline identity (AUD-03/AUD-08).
fn refresh_offline_copies(mut ui: Ui) {
    let Some(session) = ui.offline_session.peek().clone() else {
        return;
    };
    let epoch = *ui.epoch.peek();
    let ticket = ui.offline_list_gate.begin(epoch);
    let owner = OfflineOwner::profile(&ui.offline_list_gate, &ticket, epoch, &session);
    spawn(async move {
        let scope = OfflineScope::new(&owner);
        let loaded = tokio::task::spawn_blocking({
            let scope = scope.clone();
            move || crate::offline::load(&scope.origin, &scope.account_id)
        })
        .await;
        if !owner.current(&ui, &ticket) {
            return;
        }
        match loaded {
            Ok(Ok(tracks)) => {
                ui.offline.set(tracks);
                ui.offline_error.set(String::new());
            }
            Ok(Err(error)) => ui.offline_error.set(error),
            Err(_) => ui.offline_error.set(OFFLINE_UNREADABLE.into()),
        }
        sweep_verified(&mut ui, owner, ticket, scope).await;
    });
}

/// Start one guarded integrity sweep of the offline screen's copies.
fn sweep_offline_copies(ui: Ui, session: OfflineSession) {
    let epoch = *ui.epoch.peek();
    let ticket = ui.offline_list_gate.begin(epoch);
    let owner = OfflineOwner::profile(&ui.offline_list_gate, &ticket, epoch, &session);
    let mut sweeping = ui.clone();
    spawn(async move {
        let scope = OfflineScope::new(&owner);
        sweep_verified(&mut sweeping, owner, ticket, scope).await;
    });
}

/// Hash the saved copies on a blocking worker and publish the corrupt set only for a
/// live identity. Cancellation is observed between blocks inside `offline::verify`.
async fn sweep_verified(ui: &mut Ui, owner: OfflineOwner, ticket: Ticket, scope: OfflineScope) {
    let verified = tokio::task::spawn_blocking({
        let cancel = ticket.cancel.clone();
        move || crate::offline::verify(&scope.origin, &scope.account_id, &|| cancel.is_cancelled())
    })
    .await;
    if !owner.current(ui, &ticket) {
        return;
    }
    if let Ok(Ok(report)) = verified {
        ui.offline_corrupt.set(report.corrupt);
    }
}
/// Resume the saved session after an explicit **Continue saved session** action.
fn resume_saved(ui: Ui) {
    resume_attempt(ui, None);
}

/// The resume attempt. `fallback` is present only for the AUD-05 cold start: it
/// carries the private offline evidence so a server that does not answer falls
/// through to the offline library on this device instead of a dead end.
fn resume_attempt(
    mut ui: Ui,
    fallback: Option<(OfflineSession, Vec<crate::offline::OfflineTrack>)>,
) {
    if !ui.alive.get()
        || ui.signed_in()
        || *ui.connecting.peek()
        || *ui.resuming.peek()
        // The button waits for the native output; an automatic startup attempt must
        // still be able to learn that the saved session is unusable while the
        // output thread is being prepared.
        || (fallback.is_none()
            && ui.audio.peek().is_none()
            && ui.audio_error.peek().is_empty())
    {
        return;
    }
    let Some(saved) = ui.saved_session.peek().clone() else {
        return;
    };
    let ticket = ui.login_gate.begin(*ui.epoch.peek());
    // AUD-01: this resume claims the saved item; a stale attempt may not delete
    // an item that a newer login already wrote.
    let attempt = crate::session_store::begin_attempt();
    ui.resuming.set(true);
    ui.login_error.set(String::new());
    ui.notice.set(String::new());
    spawn(async move {
        let result =
            crate::client::VelaClient::resume(&saved.origin, &saved.token, ticket.cancel.clone())
                .await;
        // AUD-01: a canceled or superseded resume neither reports nor adopts.
        if !ticket.current(&ui) || ui.signed_in() {
            return;
        }
        ui.resuming.set(false);
        match result {
            Ok((client, auth)) => {
                // AUD-01: re-checked once more before the account is adopted.
                if !ticket.current(&ui) || ui.signed_in() {
                    return;
                }
                adopt_account(ui, client, auth, saved.origin, ticket.epoch);
            }
            Err(error) => {
                let failure = saved_session_failure(&error);
                if !failure.deletes_saved_item() {
                    // AUD-04: a timeout, a network/DNS failure, a 5xx or a
                    // malformed record must keep the saved item so the listener
                    // can retry instead of losing a valid login.
                    if matches!(failure, SavedSessionFailure::Canceled) {
                        return;
                    }
                    ui.login_error.set(failure.message(&error));
                } else {
                    // Genuine rejection (HTTP 401 / expired session): the stored
                    // token can never authenticate again. Drop the in-memory copy
                    // first so the button cannot start another doomed attempt, then
                    // delete the item off the UI thread with a bounded deadline.
                    let message = failure.message(&error);
                    ui.saved_session.set(None);
                    let deletion =
                        delete_saved_session(move || crate::session_store::clear_attempt(attempt))
                            .await;
                    // AUD-01: the deletion may outlive the UI state that ordered it.
                    if !ticket.current(&ui) {
                        return;
                    }
                    ui.login_error.set(match deletion {
                        Ok(()) => message,
                        Err(problem) => format!("{message} {problem}"),
                    });
                }
                // AUD-05: an unreachable, timed-out, refused or expired server does
                // not strand the listener. The private saved copies on this device
                // open in a read/play-only offline screen, with the reason kept
                // visible; nothing is played and nothing is requested again.
                if let Some((session, tracks)) = fallback {
                    enter_offline_mode(ui, session, tracks);
                }
            }
        }
    });
}

/// AUD-04: why a saved-session resume failed. Only a genuine authentication
/// rejection may delete the stored item.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SavedSessionFailure {
    /// HTTP 401 / expired session: the stored token can never work again.
    Rejected,
    /// The saved record itself is unusable, but no server rejected the session.
    Unusable,
    /// Network, DNS, timeout or 5xx: keep the item and retry.
    Transient,
    /// The server refused, redirected or answered malformed data: keep the item.
    Refused,
    /// The attempt was canceled or superseded.
    Canceled,
}

impl SavedSessionFailure {
    fn deletes_saved_item(self) -> bool {
        matches!(self, Self::Rejected)
    }

    fn message(self, error: &str) -> String {
        match self {
            Self::Rejected | Self::Canceled => error.to_owned(),
            Self::Transient => format!(
                "{error} The saved session was kept on this device; retry when the server is reachable."
            ),
            Self::Unusable => format!(
                "{error} The saved session was kept on this device; sign in again to replace it."
            ),
            Self::Refused => format!("{error} The saved session was kept on this device."),
        }
    }
}

/// These are the native client's fixed user-facing error strings
/// (`client.rs`), matched exactly on purpose: a substring match on "Sign in
/// again" would classify the 403 message as a rejected session and delete a
/// valid saved login. An unrecognized message falls back to `Refused`, which
/// preserves the item.
const SAVED_SESSION_EXPIRED: &str = "The saved session expired. Sign in again.";
const SESSION_SIGNED_OUT: &str = "Sign in again to continue.";
const SAVED_SESSION_INVALID: &str = "The saved session is invalid. Sign in again.";
const REQUEST_CANCELED: &str = "Request canceled.";
const NETWORK_UNREACHABLE: &str =
    "Unable to reach the Vela server. Check the connection and retry.";
const REQUEST_TIMED_OUT: &str = "The Vela request timed out. Retry when the server is available.";
const REQUEST_FAILED: &str = "The Vela request failed. Check the server and retry.";
const SERVER_BUSY: &str = "The server is busy. Wait before retrying.";

fn saved_session_failure(error: &str) -> SavedSessionFailure {
    match error {
        REQUEST_CANCELED => SavedSessionFailure::Canceled,
        SAVED_SESSION_EXPIRED | SESSION_SIGNED_OUT => SavedSessionFailure::Rejected,
        SAVED_SESSION_INVALID => SavedSessionFailure::Unusable,
        NETWORK_UNREACHABLE | REQUEST_TIMED_OUT | REQUEST_FAILED | SERVER_BUSY => {
            SavedSessionFailure::Transient
        }
        _ => SavedSessionFailure::Refused,
    }
}

/// Run one keyring deletion off the UI thread with a bounded wait (AUD-02). The
/// helper process itself is deadline-bounded and killed inside `session_store`;
/// this outer bound only covers a worker that never returns from the runtime.
/// Neither the local logout nor the audio engine ever waits for it.
async fn delete_saved_session(
    delete: impl FnOnce() -> Result<(), crate::session_store::KeyringError> + Send + 'static,
) -> Result<(), String> {
    let task = tokio::task::spawn_blocking(delete);
    match tokio::time::timeout(crate::session_store::worst_case_deadline(), task).await {
        Ok(Ok(Ok(()))) => Ok(()),
        Ok(Ok(Err(error))) => Err(error.message().to_owned()),
        Ok(Err(_)) => Err(crate::session_store::KeyringError::Unavailable
            .message()
            .to_owned()),
        Err(_) => Err(
            "The system keyring did not answer in time, so the saved session may still exist."
                .to_owned(),
        ),
    }
}

fn sign_out(mut ui: Ui) {
    let old = ui.account.peek().clone();
    save_queue_snapshot(ui.clone());
    // AUD-02: local access, the queue and audio end immediately. The keyring
    // deletion is started after that and is never awaited on the UI thread, so
    // a hung Secret Service cannot delay stopping playback or the local logout.
    ui.clear_account();
    ui.login_error.set(String::new());
    ui.notice
        .set("Signed out on this device. Your queue and temporary audio have been cleared.".into());
    let epoch = *ui.epoch.peek();
    let login_generation = ui.login_gate.generation.get();
    let mut keyring_ui = ui.clone();
    spawn(async move {
        let deletion = delete_saved_session(crate::session_store::clear).await;
        // AUD-01/AUD-02: only the still-current sign-out may publish this error.
        if !keyring_ui.alive.get()
            || *keyring_ui.epoch.peek() != epoch
            || keyring_ui.signed_in()
            || keyring_ui.login_gate.generation.get() != login_generation
        {
            return;
        }
        if let Err(problem) = deletion {
            keyring_ui.login_error.set(format!(
                "Local sign-out is complete, but the saved session was not confirmed as removed: {problem}"
            ));
        }
    });
    // AUD-05: an explicit sign-out withdraws the private offline profile, so a cold
    // start asks for a sign-in again. Only this non-secret hint is removed: saved
    // copies, their manifests and their audio stay on this device. Local file work
    // only, on a blocking worker.
    let mut profile_ui = ui.clone();
    spawn(async move {
        let cleared = tokio::task::spawn_blocking(crate::offline_profile::clear).await;
        if !profile_ui.alive.get() || *profile_ui.epoch.peek() != epoch || profile_ui.signed_in() {
            return;
        }
        if !matches!(cleared, Ok(Ok(()))) {
            profile_ui.login_error.set(
                "Signed out. The private offline profile could not be removed, so saved copies may still open offline after a restart."
                    .into(),
            );
        }
    });
    let Some(account) = old else {
        return;
    };
    let ticket = ui.logout_gate.begin(epoch);
    spawn(async move {
        let result = account.client.logout(ticket.cancel.clone()).await;
        if ticket.current(&ui)
            && !ui.signed_in()
            && let Err(error) = result
        {
            ui.login_error.set(format!(
                "Local sign-out is complete. Server sign-out was not confirmed: {error}"
            ));
        }
    });
}

fn expired(mut ui: Ui, error: &str) -> bool {
    if error != "Sign in again to continue." {
        return false;
    }
    ui.clear_account();
    ui.login_error
        .set("Your session ended. Sign in again to continue.".into());
    true
}

fn load_catalog(mut ui: Ui, query: String, cursor: Option<String>, favorites: bool) {
    let Some(account) = ui.account.peek().clone().filter(|_| ui.signed_in()) else {
        return;
    };
    ui.catalog_gate.cancel();
    ui.catalog.set(None);
    ui.catalog_loading.set(false);
    ui.catalog_error.set(String::new());
    if query.len() > 256 || query.chars().any(char::is_control) {
        ui.catalog_error
            .set("Use a search of at most 256 bytes without control characters.".into());
        return;
    }
    ui.query.set(query.clone());
    ui.cursor.set(cursor.clone());
    ui.view.set(if favorites { "favorites" } else { "library" });
    let ticket = ui.catalog_gate.begin(account.epoch);
    ui.catalog_loading.set(true);
    spawn(async move {
        if !ticket.current(&ui)
            || *ui.query.peek() != query
            || *ui.cursor.peek() != cursor
            || (*ui.view.peek() == "favorites") != favorites
        {
            return;
        }
        let result = account
            .client
            .tracks(
                (!query.is_empty()).then_some(query.as_str()),
                cursor.as_deref(),
                favorites,
                ticket.cancel.clone(),
            )
            .await;
        if !ticket.current(&ui)
            || *ui.query.peek() != query
            || *ui.cursor.peek() != cursor
            || (*ui.view.peek() == "favorites") != favorites
        {
            return;
        }
        ui.catalog_loading.set(false);
        match result {
            Ok(page) => {
                ui.catalog_error.set(String::new());
                ui.catalog.set(Some(Catalog {
                    epoch: ticket.epoch,
                    query,
                    cursor,
                    favorites,
                    page: Arc::new(page),
                }));
            }
            Err(error) => {
                if !expired(ui.clone(), &error) {
                    ui.catalog_error.set(error);
                }
            }
        }
    });
}

fn start_source(mut ui: Ui, reuse: bool, autoplay: bool, position_ms: u64) {
    ui.offline_active.set(false);
    ui.remote_now.set(None);
    ui.remote_queue.set(Vec::new());
    ui.remote_index.set(0);
    let (shuffle, repeat) = (*ui.shuffle.peek(), *ui.repeat.peek());
    ui.order.write().configure(shuffle, repeat);
    let Some(account) = ui.account.peek().clone().filter(|_| ui.signed_in()) else {
        return;
    };
    let Some(track) = ui.current_track() else {
        return;
    };
    let Some(audio) = ui.audio.peek().clone() else {
        return;
    };
    let prepared = if reuse {
        ui.prepared.peek().clone()
    } else {
        None
    };
    let selection = ui.new_selection();
    ui.wants_play.set(autoplay);
    ui.prepared.set(None);
    if !ui.source_error.peek().is_empty() {
        return;
    }
    if let Some(prepared) = prepared {
        match audio.load(selection, prepared.clone(), autoplay, position_ms) {
            Ok(()) => ui.prepared.set(Some(prepared)),
            Err(error) => {
                ui.wants_play.set(false);
                ui.source_error.set(error);
            }
        }
        return;
    }
    let ticket = ui.source_gate.begin(account.epoch);
    let (progress, mut updates) = watch::channel(DownloadProgress::default());
    ui.preparing.set(true);
    let mut progress_ui = ui.clone();
    let progress_ticket = ticket.clone();
    let progress_id = track.id.clone();
    spawn(async move {
        loop {
            tokio::select! {
                _ = progress_ticket.cancel.cancelled() => return,
                result = updates.changed() => {
                    if result.is_err() || !progress_ui.source_current(&progress_ticket, selection, &progress_id) { return; }
                    progress_ui.progress.set(*updates.borrow_and_update());
                }
            }
        }
    });
    spawn(async move {
        if !ui.source_current(&ticket, selection, &track.id) {
            return;
        }
        let result = account
            .client
            .prepare(&track, ticket.cancel.clone(), progress)
            .await;
        if !ui.source_current(&ticket, selection, &track.id) {
            return;
        }
        ui.preparing.set(false);
        let requested = *ui.wants_play.peek();
        match result {
            Ok(prepared) => match audio.load(selection, prepared.clone(), requested, position_ms) {
                Ok(()) => ui.prepared.set(Some(prepared)),
                Err(error) => {
                    ui.wants_play.set(false);
                    ui.source_error.set(error);
                }
            },
            Err(error) => {
                if !expired(ui.clone(), &error) {
                    ui.wants_play.set(false);
                    ui.source_error.set(error);
                }
            }
        }
    });
}

fn load_playlists(mut ui: Ui) {
    let Some(account) = ui.account.peek().clone().filter(|_| ui.signed_in()) else {
        return;
    };
    ui.playlist_gate.cancel();
    ui.playlist.set(None);
    ui.playlist_id.set(None);
    ui.playlist_loading.set(true);
    ui.playlist_error.set(String::new());
    ui.view.set("playlists");
    let ticket = ui.playlist_gate.begin(account.epoch);
    spawn(async move {
        let result = account.client.playlists(ticket.cancel.clone()).await;
        if !ticket.current(&ui) || *ui.view.peek() != "playlists" {
            return;
        }
        ui.playlist_loading.set(false);
        match result {
            Ok(playlists) => {
                ui.playlist_error.set(String::new());
                ui.playlists.set(playlists);
                if let Ok(folders) = account.client.playlist_folders(ticket.cancel.clone()).await
                    && ticket.current(&ui)
                    && *ui.view.peek() == "playlists"
                {
                    ui.folders.set(folders);
                }
            }
            Err(error) => {
                if !expired(ui.clone(), &error) {
                    ui.playlist_error.set(error);
                }
            }
        }
    });
}

fn open_playlist(mut ui: Ui, id: String) {
    let Some(account) = ui.account.peek().clone().filter(|_| ui.signed_in()) else {
        return;
    };
    if !uuid::Uuid::parse_str(&id).is_ok_and(|parsed| parsed.to_string() == id) {
        ui.playlist_error.set("Select a valid playlist.".into());
        return;
    }
    ui.playlist_gate.cancel();
    ui.playlist_id.set(Some(id.clone()));
    ui.playlist.set(None);
    ui.playlist_loading.set(true);
    ui.playlist_error.set(String::new());
    let ticket = ui.playlist_gate.begin(account.epoch);
    spawn(async move {
        let result = account.client.playlist(&id, ticket.cancel.clone()).await;
        if !ticket.current(&ui)
            || *ui.view.peek() != "playlists"
            || ui.playlist_id.peek().as_deref() != Some(id.as_str())
        {
            return;
        }
        ui.playlist_loading.set(false);
        match result {
            Ok(detail) => {
                ui.playlist_error.set(String::new());
                ui.playlist_name_draft.set(detail.playlist.name.clone());
                ui.playlist_delete_confirm.set(false);
                ui.playlist.set(Some(detail));
                let candidates = account
                    .client
                    .tracks(None, None, false, ticket.cancel.clone())
                    .await;
                if ticket.current(&ui)
                    && *ui.view.peek() == "playlists"
                    && ui.playlist_id.peek().as_deref() == Some(id.as_str())
                {
                    match candidates {
                        Ok(page) => ui.playlist_candidates.set(page.items),
                        Err(_) => ui.playlist_candidates.set(Vec::new()),
                    }
                }
            }
            Err(error) => {
                if !expired(ui.clone(), &error) {
                    ui.playlist_error.set(error);
                }
            }
        }
    });
}

fn create_playlist_ui(mut ui: Ui) {
    let name = ui.playlist_name_draft.peek().trim().to_string();
    if name.is_empty() || *ui.playlist_busy.peek() {
        return;
    }
    let Some(account) = ui.account.peek().clone().filter(|_| ui.signed_in()) else {
        return;
    };
    let ticket = ui.playlist_write_gate.begin(account.epoch);
    ui.playlist_busy.set(true);
    spawn(async move {
        let result = account
            .client
            .create_playlist(&name, ticket.cancel.clone())
            .await;
        if !ticket.current(&ui) || *ui.view.peek() != "playlists" {
            return;
        }
        ui.playlist_busy.set(false);
        match result {
            Ok(created) => {
                ui.playlist_error.set(String::new());
                ui.playlist_name_draft.set(String::new());
                ui.playlists.write().insert(0, created);
                ui.notice.set("Playlist created.".into());
            }
            Err(error) => {
                if !expired(ui.clone(), &error) {
                    ui.playlist_error.set(error);
                }
            }
        }
    });
}

fn rename_selected_playlist(mut ui: Ui) {
    let Some(detail) = ui.playlist.peek().clone().filter(|_| ui.signed_in()) else {
        return;
    };
    let name = ui.playlist_name_draft.peek().trim().to_string();
    if name.is_empty() || *ui.playlist_busy.peek() {
        return;
    }
    let Some(account) = ui.account.peek().clone() else {
        return;
    };
    let id = detail.playlist.id;
    let revision = detail.playlist.revision;
    let ticket = ui.playlist_write_gate.begin(account.epoch);
    ui.playlist_busy.set(true);
    spawn(async move {
        let result = account
            .client
            .rename_playlist(&id, revision, &name, ticket.cancel.clone())
            .await;
        if !ticket.current(&ui)
            || *ui.view.peek() != "playlists"
            || ui.playlist_id.peek().as_deref() != Some(id.as_str())
        {
            return;
        }
        ui.playlist_busy.set(false);
        match result {
            Ok(updated) => {
                ui.playlist_error.set(String::new());
                ui.playlist_name_draft.set(updated.name.clone());
                if let Some(detail) = ui.playlist.write().as_mut() {
                    detail.playlist = updated.clone();
                }
                if let Some(entry) = ui.playlists.write().iter_mut().find(|list| list.id == id) {
                    *entry = updated;
                }
                ui.notice.set("Playlist renamed.".into());
            }
            Err(error) => {
                if !expired(ui.clone(), &error) {
                    ui.playlist_error.set(error);
                }
            }
        }
    });
}

fn delete_selected_playlist(mut ui: Ui) {
    let Some(detail) = ui.playlist.peek().clone().filter(|_| ui.signed_in()) else {
        return;
    };
    if *ui.playlist_busy.peek() {
        return;
    }
    let Some(account) = ui.account.peek().clone() else {
        return;
    };
    let id = detail.playlist.id;
    let revision = detail.playlist.revision;
    let ticket = ui.playlist_write_gate.begin(account.epoch);
    ui.playlist_busy.set(true);
    ui.playlist_delete_confirm.set(false);
    spawn(async move {
        let result = account
            .client
            .delete_playlist(&id, revision, ticket.cancel.clone())
            .await;
        if !ticket.current(&ui)
            || *ui.view.peek() != "playlists"
            || ui.playlist_id.peek().as_deref() != Some(id.as_str())
        {
            return;
        }
        ui.playlist_busy.set(false);
        match result {
            Ok(()) => {
                ui.playlist_error.set(String::new());
                ui.playlist.set(None);
                ui.playlist_id.set(None);
                ui.playlist_name_draft.set(String::new());
                ui.playlists.write().retain(|list| list.id != id);
                ui.notice
                    .set("Playlist deleted. Your tracks and audio are unchanged.".into());
            }
            Err(error) => {
                if !expired(ui.clone(), &error) {
                    ui.playlist_error.set(error);
                }
            }
        }
    });
}

fn move_playlist_entry(mut ui: Ui, at: usize, direction: i32) {
    let Some(detail) = ui.playlist.peek().clone().filter(|_| ui.signed_in()) else {
        return;
    };
    if *ui.playlist_busy.peek() {
        return;
    }
    let target = at as i32 + direction;
    if target < 0 || target as usize >= detail.items.len() {
        return;
    }
    let Some(account) = ui.account.peek().clone() else {
        return;
    };
    let mut ids = detail
        .items
        .iter()
        .map(|item| item.id.clone())
        .collect::<Vec<_>>();
    ids.swap(at, target as usize);
    let id = detail.playlist.id;
    let revision = detail.playlist.revision;
    let ticket = ui.playlist_write_gate.begin(account.epoch);
    ui.playlist_busy.set(true);
    spawn(async move {
        let result = account
            .client
            .reorder_playlist(&id, revision, &ids, ticket.cancel.clone())
            .await;
        if !ticket.current(&ui)
            || *ui.view.peek() != "playlists"
            || ui.playlist_id.peek().as_deref() != Some(id.as_str())
        {
            return;
        }
        ui.playlist_busy.set(false);
        match result {
            Ok(updated) => {
                ui.playlist_error.set(String::new());
                if let Some(entry) = ui.playlists.write().iter_mut().find(|list| list.id == id) {
                    entry.revision = updated.playlist.revision;
                    entry.item_count = updated.items.len() as i64;
                }
                ui.playlist.set(Some(updated));
                ui.notice.set("Playlist order updated.".into());
            }
            Err(error) => {
                if !expired(ui.clone(), &error) {
                    ui.playlist_error.set(error);
                }
            }
        }
    });
}

fn remove_playlist_entry(mut ui: Ui, item_id: String) {
    let Some(detail) = ui.playlist.peek().clone().filter(|_| ui.signed_in()) else {
        return;
    };
    if *ui.playlist_busy.peek() {
        return;
    }
    let Some(account) = ui.account.peek().clone() else {
        return;
    };
    let id = detail.playlist.id;
    let revision = detail.playlist.revision;
    let ticket = ui.playlist_write_gate.begin(account.epoch);
    ui.playlist_busy.set(true);
    spawn(async move {
        let result = account
            .client
            .remove_playlist_item(&id, &item_id, revision, ticket.cancel.clone())
            .await;
        if !ticket.current(&ui)
            || *ui.view.peek() != "playlists"
            || ui.playlist_id.peek().as_deref() != Some(id.as_str())
        {
            return;
        }
        ui.playlist_busy.set(false);
        match result {
            Ok(updated) => {
                ui.playlist_error.set(String::new());
                if let Some(entry) = ui.playlists.write().iter_mut().find(|list| list.id == id) {
                    entry.revision = updated.playlist.revision;
                    entry.item_count = updated.items.len() as i64;
                }
                ui.playlist.set(Some(updated));
                ui.notice.set("Playlist entry removed.".into());
            }
            Err(error) => {
                if !expired(ui.clone(), &error) {
                    ui.playlist_error.set(error);
                }
            }
        }
    });
}

fn create_folder(mut ui: Ui) {
    let name = ui.folder_name_draft.peek().trim().to_string();
    if name.is_empty() || *ui.folder_busy.peek() || !ui.signed_in() {
        return;
    }
    let Some(account) = ui.account.peek().clone() else {
        return;
    };
    let ticket = ui.folder_gate.begin(account.epoch);
    ui.folder_busy.set(true);
    ui.playlist_error.set(String::new());
    spawn(async move {
        let result = account
            .client
            .create_playlist_folder(&name, ticket.cancel.clone())
            .await;
        if !ticket.current(&ui) || *ui.view.peek() != "playlists" {
            return;
        }
        ui.folder_busy.set(false);
        match result {
            Ok(folder) => {
                ui.folder_name_draft.set(String::new());
                ui.folders.write().push(folder);
                ui.folders.write().sort_by(|left, right| {
                    left.name
                        .to_lowercase()
                        .cmp(&right.name.to_lowercase())
                        .then_with(|| left.id.cmp(&right.id))
                });
                ui.notice.set("Playlist folder created.".into());
            }
            Err(error) => {
                if !expired(ui.clone(), &error) {
                    ui.playlist_error.set(error);
                }
            }
        }
    });
}

fn select_folder(mut ui: Ui, id: Option<String>) {
    ui.folder_confirm.set(String::new());
    ui.folder_rename_draft.set(
        id.as_deref()
            .and_then(|id| {
                ui.folders
                    .peek()
                    .iter()
                    .find(|folder| folder.id == id)
                    .map(|folder| folder.name.clone())
            })
            .unwrap_or_default(),
    );
    ui.folder_filter.set(id);
}

fn rename_folder(mut ui: Ui) {
    let Some(id) = ui.folder_filter.peek().clone() else {
        return;
    };
    let name = ui.folder_rename_draft.peek().trim().to_string();
    if name.is_empty() || *ui.folder_busy.peek() {
        return;
    }
    let Some(folder) = ui
        .folders
        .peek()
        .iter()
        .find(|folder| folder.id == id)
        .cloned()
    else {
        return;
    };
    let Some(account) = ui.account.peek().clone() else {
        return;
    };
    let ticket = ui.folder_gate.begin(account.epoch);
    ui.folder_busy.set(true);
    spawn(async move {
        let result = account
            .client
            .rename_playlist_folder(&id, folder.revision, &name, ticket.cancel.clone())
            .await;
        if !ticket.current(&ui) || *ui.view.peek() != "playlists" {
            return;
        }
        ui.folder_busy.set(false);
        match result {
            Ok(updated) => {
                ui.folder_rename_draft.set(updated.name.clone());
                if let Some(entry) = ui.folders.write().iter_mut().find(|folder| folder.id == id) {
                    *entry = updated;
                }
                ui.notice.set("Folder renamed.".into());
            }
            Err(error) => {
                if !expired(ui.clone(), &error) {
                    ui.playlist_error.set(error);
                }
            }
        }
    });
}

fn delete_folder(mut ui: Ui, id: String) {
    let Some(folder) = ui
        .folders
        .peek()
        .iter()
        .find(|folder| folder.id == id)
        .cloned()
    else {
        return;
    };
    if *ui.folder_busy.peek() || !ui.signed_in() {
        return;
    }
    let Some(account) = ui.account.peek().clone() else {
        return;
    };
    let ticket = ui.folder_gate.begin(account.epoch);
    ui.folder_busy.set(true);
    spawn(async move {
        let result = account
            .client
            .delete_playlist_folder(&id, folder.revision, ticket.cancel.clone())
            .await;
        if !ticket.current(&ui) || *ui.view.peek() != "playlists" {
            return;
        }
        ui.folder_busy.set(false);
        match result {
            Ok(()) => {
                ui.folders.write().retain(|folder| folder.id != id);
                if ui.folder_filter.peek().as_deref() == Some(id.as_str()) {
                    ui.folder_filter.set(None);
                    ui.folder_rename_draft.set(String::new());
                }
                ui.folder_confirm.set(String::new());
                if let Ok(playlists) = account.client.playlists(ticket.cancel.clone()).await
                    && ticket.current(&ui)
                    && *ui.view.peek() == "playlists"
                {
                    ui.playlists.set(playlists);
                }
                ui.notice
                    .set("Folder deleted. Its playlists are now unfiled.".into());
            }
            Err(error) => {
                if !expired(ui.clone(), &error) {
                    ui.playlist_error.set(error);
                }
            }
        }
    });
}

fn move_playlist_folder(mut ui: Ui) {
    let Some(detail) = ui.playlist.peek().clone() else {
        return;
    };
    let target = ui.playlist_folder_target.peek().clone();
    let folder_id = (!target.is_empty()).then_some(target);
    let Some(account) = ui.account.peek().clone() else {
        return;
    };
    let id = detail.playlist.id;
    let ticket = ui.folder_gate.begin(account.epoch);
    ui.folder_busy.set(true);
    spawn(async move {
        let result = account
            .client
            .move_playlist(
                &id,
                detail.playlist.revision,
                folder_id.as_deref(),
                ticket.cancel.clone(),
            )
            .await;
        if !ticket.current(&ui) || *ui.view.peek() != "playlists" {
            return;
        }
        ui.folder_busy.set(false);
        match result {
            Ok(updated) => {
                if let Some(entry) = ui.playlists.write().iter_mut().find(|list| list.id == id) {
                    *entry = updated.clone();
                }
                if let Ok(detail) = account.client.playlist(&id, ticket.cancel.clone()).await
                    && ticket.current(&ui)
                    && *ui.view.peek() == "playlists"
                {
                    ui.playlist.set(Some(detail));
                }
                if let Ok(folders) = account.client.playlist_folders(ticket.cancel.clone()).await
                    && ticket.current(&ui)
                    && *ui.view.peek() == "playlists"
                {
                    ui.folders.set(folders);
                }
                ui.notice.set("Playlist folder updated.".into());
            }
            Err(error) => {
                if !expired(ui.clone(), &error) {
                    ui.playlist_error.set(error);
                }
            }
        }
    });
}

fn add_selected_playlist_track(mut ui: Ui) {
    let Some(detail) = ui.playlist.peek().clone().filter(|_| ui.signed_in()) else {
        return;
    };
    let track_id = ui.playlist_candidate.peek().clone();
    if track_id.is_empty() || *ui.playlist_busy.peek() {
        return;
    }
    let Some(account) = ui.account.peek().clone() else {
        return;
    };
    let id = detail.playlist.id;
    let revision = detail.playlist.revision;
    let ticket = ui.playlist_write_gate.begin(account.epoch);
    ui.playlist_busy.set(true);
    spawn(async move {
        let result = account
            .client
            .add_playlist_item(&id, revision, &track_id, ticket.cancel.clone())
            .await;
        if !ticket.current(&ui)
            || *ui.view.peek() != "playlists"
            || ui.playlist_id.peek().as_deref() != Some(id.as_str())
        {
            return;
        }
        ui.playlist_busy.set(false);
        match result {
            Ok(updated) => {
                ui.playlist_error.set(String::new());
                ui.playlist_candidate.set(String::new());
                if let Some(entry) = ui.playlists.write().iter_mut().find(|list| list.id == id) {
                    entry.revision = updated.playlist.revision;
                    entry.item_count = updated.items.len() as i64;
                }
                ui.playlist.set(Some(updated));
                ui.notice.set("Track added to playlist.".into());
            }
            Err(error) => {
                if !expired(ui.clone(), &error) {
                    ui.playlist_error.set(error);
                }
            }
        }
    });
}

fn play_playlist(mut ui: Ui) {
    let Some(detail) = ui.playlist.peek().clone().filter(|_| ui.signed_in()) else {
        return;
    };
    if *ui.view.peek() != "playlists" || ui.audio.peek().is_none() {
        return;
    }
    let tracks = detail
        .items
        .iter()
        .filter(|item| item.can_stream)
        .map(|item| item.track.clone())
        .collect::<Vec<_>>();
    if tracks.is_empty() {
        ui.playlist_error
            .set("This playlist has no tracks your account may stream.".into());
        return;
    }
    queue_owned_context(
        ui,
        tracks,
        0,
        format!("Playlist · {}", detail.playlist.name),
    );
}

fn toggle_favorite(mut ui: Ui, catalog: Catalog, track: Track) {
    if !ui.catalog_current(&catalog) || !ui.favorite_busy.peek().is_empty() {
        return;
    }
    let Some(account) = ui
        .account
        .peek()
        .clone()
        .filter(|account| ui.signed_in() && account.epoch == catalog.epoch)
    else {
        return;
    };
    let desired = !track.favorite;
    let ticket = ui.feedback_gate.begin(catalog.epoch);
    ui.favorite_busy.set(track.id.clone());
    spawn(async move {
        let result = account
            .client
            .favorite(&track.id, desired, ticket.cancel.clone())
            .await;
        if !ticket.current(&ui) {
            return;
        }
        if *ui.favorite_busy.peek() == track.id {
            ui.favorite_busy.set(String::new());
        }
        match result {
            Ok(()) => {
                if catalog.favorites && !desired {
                    // Re-read the filtered page instead of leaving a stale row.
                    load_catalog(ui.clone(), catalog.query.clone(), None, true);
                } else {
                    let mut page = (*catalog.page).clone();
                    for item in &mut page.items {
                        if item.id == track.id {
                            item.favorite = desired;
                        }
                    }
                    ui.catalog.set(Some(Catalog {
                        page: Arc::new(page),
                        ..catalog.clone()
                    }));
                }
                for item in ui.queue.write().iter_mut() {
                    if item.id == track.id {
                        item.favorite = desired;
                    }
                }
                ui.notice.set(
                    if desired {
                        "Added to favorites."
                    } else {
                        "Removed from favorites."
                    }
                    .into(),
                );
            }
            Err(error) => {
                if !expired(ui.clone(), &error) {
                    ui.catalog_error.set(error);
                }
            }
        }
    });
}

fn toggle_library_favorite(mut ui: Ui, track: Track) {
    if !ui.signed_in() || !ui.favorite_busy.peek().is_empty() {
        return;
    }
    let Some(account) = ui.account.peek().clone() else {
        return;
    };
    let desired = !track.favorite;
    let ticket = ui.feedback_gate.begin(account.epoch);
    ui.favorite_busy.set(track.id.clone());
    spawn(async move {
        let result = account
            .client
            .favorite(&track.id, desired, ticket.cancel.clone())
            .await;
        if !ticket.current(&ui) {
            return;
        }
        if *ui.favorite_busy.peek() == track.id {
            ui.favorite_busy.set(String::new());
        }
        match result {
            Ok(()) => {
                for item in ui.queue.write().iter_mut() {
                    if item.id == track.id {
                        item.favorite = desired;
                    }
                }
                let current_catalog = ui.catalog.peek().clone();
                if let Some(catalog) = current_catalog {
                    let mut page = (*catalog.page).clone();
                    let mut changed = false;
                    for item in &mut page.items {
                        if item.id == track.id {
                            item.favorite = desired;
                            changed = true;
                        }
                    }
                    if changed {
                        ui.catalog.set(Some(Catalog {
                            page: Arc::new(page),
                            ..catalog
                        }));
                    }
                }
                ui.notice.set(
                    if desired {
                        "Added to favorites."
                    } else {
                        "Removed from favorites."
                    }
                    .into(),
                );
            }
            Err(error) => {
                if !expired(ui.clone(), &error) {
                    ui.source_error.set(error);
                }
            }
        }
    });
}

fn play_catalog(mut ui: Ui, catalog: Catalog, at: usize) {
    if !ui.catalog_current(&catalog) {
        ui.catalog_error
            .set("This page changed. Refresh the library before playing.".into());
        return;
    }
    if at >= catalog.page.items.len() || ui.audio.peek().is_none() {
        return;
    }
    if ui
        .current_track()
        .is_some_and(|track| track.id == catalog.page.items[at].id)
    {
        toggle(ui);
        return;
    }
    let mut tracks = catalog.page.items.clone();
    tracks.truncate(MAX_QUEUE);
    ui.order.write().reset(tracks.len(), at);
    ui.queue.set(tracks);
    ui.context.set(if catalog.query.is_empty() {
        "Your library".into()
    } else {
        "Search results".into()
    });
    start_source(ui, false, true, 0);
}

fn owned_target_current(ui: &Ui, tracks: &[Track], at: usize, context: &str) -> bool {
    let Some(target) = tracks.get(at) else {
        return false;
    };
    let active = ui
        .current_track()
        .is_some_and(|track| track.id == target.id);
    let duplicate = tracks.iter().filter(|track| track.id == target.id).count() > 1;
    let same_occurrence = ui.order.peek().current() == Some(at)
        && ui.context.peek().as_str() == context
        && ui
            .queue
            .peek()
            .iter()
            .map(|t| &t.id)
            .eq(tracks.iter().map(|t| &t.id));
    active && (!duplicate || same_occurrence)
}

fn play_owned_context(ui: Ui, mut tracks: Vec<Track>, at: usize, context: String) {
    tracks.truncate(MAX_QUEUE);
    if at >= tracks.len() {
        return;
    }
    if !ui.signed_in() || !ui.audio_ready() {
        return;
    }
    if owned_target_current(&ui, &tracks, at, &context) {
        toggle(ui);
        return;
    }
    queue_owned_context(ui, tracks, at, context);
}

fn queue_owned_context(mut ui: Ui, tracks: Vec<Track>, at: usize, context: String) {
    ui.order.write().reset(tracks.len(), at);
    ui.queue.set(tracks);
    ui.context.set(context);
    start_source(ui, false, true, 0);
}

fn enqueue(mut ui: Ui, track: Track, next: bool) {
    if !ui.signed_in() {
        return;
    }
    if ui.queue.peek().len() >= MAX_QUEUE {
        ui.notice
            .set("Your queue is full (1,000 tracks). Remove a queued track first.".into());
        return;
    }
    let at = if next {
        ui.order
            .peek()
            .current()
            .map_or(0, |at| at + 1)
            .min(ui.queue.peek().len())
    } else {
        ui.queue.peek().len()
    };
    if !ui.order.write().insert(at, next) {
        return;
    }
    ui.queue.write().insert(at, track);
    if ui.context.peek().is_empty() {
        ui.context.set("Your queue".into());
    }
    ui.notice.set(
        if next {
            "Added next in your queue."
        } else {
            "Added to your queue."
        }
        .into(),
    );
}

fn select_queue(mut ui: Ui, at: usize) {
    if !ui.signed_in() || at >= ui.queue.peek().len() || ui.audio.peek().is_none() {
        return;
    }
    if ui.order.peek().current() == Some(at) {
        toggle(ui);
        return;
    }
    if *ui.offline_active.peek() {
        let id = ui.queue.peek()[at].id.clone();
        let saved = ui
            .offline
            .peek()
            .iter()
            .position(|item| item.track.id == id);
        if let Some(saved) = saved {
            start_offline_at(ui, saved);
        }
        return;
    }
    ui.order.write().reset(ui.queue.peek().len(), at);
    start_source(ui, false, true, 0);
}

fn advance(mut ui: Ui, command: Command) {
    if !ui.can_play() || ui.current_track().is_none() {
        return;
    }
    // Natural EOF during offline playback must stay on the device queue; the
    // generic transition would route to server prepare and fail with the server
    // stopped. Repeat-one keeps the existing replay path.
    if command == Command::Ended && *ui.offline_active.peek() {
        if *ui.repeat.peek() == Repeat::One {
            // Replay the already verified descriptor and retain the offline
            // source domain; server preparation must never enter this path.
            set_playing(ui, true);
        } else {
            offline_step(ui, 1);
        }
        return;
    }
    let elapsed = ui.snapshot.peek().position_ms as f64 / 1000.0;
    let random = uuid::Uuid::new_v4();
    let bytes = random.as_bytes();
    let entropy = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
    let transition = ui.order.write().transition(command, elapsed, entropy);
    match transition {
        Transition::Play(_) => start_source(ui.clone(), false, true, 0),
        Transition::SeekStart { resume: true } => start_source(ui.clone(), true, true, 0),
        Transition::SeekStart { resume: false } => {
            if *ui.preparing.peek() {
                return;
            }
            if let Some(audio) = ui.audio.peek().clone()
                && let Err(error) = audio.seek(*ui.selection.peek(), 0)
            {
                ui.source_error.set(error);
            }
        }
        Transition::Stop => {
            ui.wants_play.set(false);
            if *ui.preparing.peek() {
                ui.new_selection();
                ui.prepared.set(None);
            } else if command != Command::Ended
                && let Some(audio) = ui.audio.peek().clone()
                && let Err(error) = audio.pause(*ui.selection.peek())
            {
                ui.source_error.set(error);
            }
        }
        Transition::None => {}
    }
    save_queue_snapshot(ui);
}

fn reconnect_audio(mut ui: Ui) {
    if ui.audio.peek().as_ref().is_some_and(AudioEngine::alive) {
        return;
    }
    let Some(account) = ui
        .account
        .peek()
        .clone()
        .filter(|account| ui.signed_in() && account.epoch == *ui.epoch.peek())
    else {
        return;
    };
    ui.audio.set(None);
    ui.audio_error.set(String::new());
    let mut installing = ui.clone();
    spawn(async move {
        let result = tokio::task::spawn_blocking(AudioEngine::new).await;
        if !installing.alive.get() || *installing.epoch.peek() != account.epoch {
            return;
        }
        match result {
            Ok(Ok(engine)) => {
                *installing.engine_cleanup.borrow_mut() = Some(engine.clone());
                installing.audio.set(Some(engine));
            }
            Ok(Err(error)) => installing.audio_error.set(error),
            Err(_) => installing.audio_error.set(
                "Native audio output could not be reconnected. Restart Vela to try again.".into(),
            ),
        }
    });
}
fn toggle(ui: Ui) {
    let desired = !*ui.wants_play.peek();
    set_playing(ui, desired);
}

/// Cancel the actual source operation, then invalidate every deferred completion.
/// Keep the selected occurrence so Play retries it through the same authorized path.
fn cancel_source_preparation(mut ui: Ui) {
    if !ui.signed_in() || !*ui.preparing.peek() {
        return;
    }
    ui.new_selection();
    ui.prepared.set(None);
    ui.source_canceled.set(true);
}

fn source_preparation_message(progress: DownloadProgress, public: bool) -> String {
    if public || progress.total_bytes == 0 {
        "Preparing audio…".into()
    } else if progress.received_bytes >= progress.total_bytes {
        "Checking audio…".into()
    } else {
        format!(
            "Preparing audio · {} of {} KiB",
            progress.received_bytes / 1024,
            progress.total_bytes.div_ceil(1024)
        )
    }
}

fn set_playing(mut ui: Ui, desired: bool) {
    if !ui.can_play() || ui.offline_checking.peek().is_some() {
        return;
    }
    let Some(audio) = ui.audio.peek().clone() else {
        return;
    };
    if ui.current_track().is_none() && ui.remote_now.peek().is_none() {
        return;
    }
    if *ui.preparing.peek() {
        // A second click pauses the pending selection. Its bounded download may
        // finish, but its late result must not start audio against this intent.
        ui.wants_play.set(desired);
        return;
    }
    let phase = ui.snapshot.peek().phase;
    let prepared = ui.prepared.peek().clone();
    let result = if desired
        && matches!(phase, AudioPhase::Ended | AudioPhase::Error)
        && let Some(prepared) = prepared.as_ref()
    {
        let selection = ui.new_selection();
        let prepared = prepared.clone();
        ui.prepared.set(Some(prepared.clone()));
        audio.load(selection, prepared, true, 0)
    } else if desired && prepared.is_none() {
        if ui.remote_now.peek().is_some() {
            start_remote(ui);
        } else {
            let position = *ui.restored_position.peek();
            start_source(ui, false, true, position);
        }
        return;
    } else if desired {
        audio.resume(*ui.selection.peek())
    } else {
        audio.pause(*ui.selection.peek())
    };
    match result {
        Ok(()) => ui.wants_play.set(desired),
        Err(error) => ui.source_error.set(error),
    }
}

fn seek_to(mut ui: Ui, position: u64) {
    if !ui.can_play() || *ui.preparing.peek() || ui.prepared.peek().is_none() {
        return;
    }
    let maximum = ui.snapshot.peek().duration_ms;
    let target = position.min(maximum);
    if let Some(audio) = ui.audio.peek().clone() {
        match audio.seek(*ui.selection.peek(), target) {
            Ok(()) => {
                let serial = ui.mpris_seek_serial.peek().wrapping_add(1);
                ui.mpris_seek_serial.set(serial);
            }
            Err(error) => ui.source_error.set(error),
        }
    }
}

fn seek(ui: Ui, position: String) {
    let Ok(position) = position.parse::<u64>() else {
        return;
    };
    seek_to(ui, position);
}

fn system_controls_note(state: mpris::State) -> &'static str {
    match state {
        mpris::State::Starting => "System media controls starting…",
        mpris::State::Ready => "System media controls active",
        mpris::State::Failed => "System media controls unavailable in this session",
        mpris::State::Unsupported => "System media controls are not integrated here yet",
    }
}

fn play_system(ui: Ui) {
    set_playing(ui, true);
}

fn pause_system(ui: Ui) {
    set_playing(ui, false);
}

fn stop_system(mut ui: Ui) {
    if !ui.can_play() {
        return;
    }
    ui.new_selection();
    ui.prepared.set(None);
}

/// Apply one external media command on the UI thread. Returns true when the
/// command closed this window. Anything that needs account state is ignored
/// while signed out; only Quit remains valid at the sign-in screen.
fn apply_system_control(mut ui: Ui, command: mpris::Command) -> bool {
    match command {
        mpris::Command::Quit => {
            dioxus::desktop::window().close();
            return true;
        }
        mpris::Command::Play => play_system(ui),
        mpris::Command::Pause => pause_system(ui),
        mpris::Command::PlayPause => toggle(ui),
        mpris::Command::Stop => stop_system(ui),
        mpris::Command::Next => player_next(ui),
        mpris::Command::Previous => player_previous(ui),
        mpris::Command::SeekRelative(offset) => {
            if ui.can_play() && ui.prepared.peek().is_some() {
                let position = ui.snapshot.peek().position_ms;
                let duration = ui.snapshot.peek().duration_ms;
                let target = (i128::from(position) + i128::from(offset) / 1000)
                    .clamp(0, i128::from(duration)) as u64;
                seek_to(ui, target);
            }
        }
        mpris::Command::SetPosition {
            track_id,
            position_ms,
        } => {
            if ui
                .current_track()
                .is_some_and(|track| mpris::track_path(&track.id) == track_id)
            {
                seek_to(ui, position_ms);
            }
        }
        mpris::Command::SetVolume(value) => {
            if value.is_finite()
                && ui.can_play()
                && let Some(audio) = ui.audio.peek().clone()
            {
                let volume = (value.clamp(0.0, 1.0)) as f32;
                if let Err(error) = audio.set_volume(*ui.selection.peek(), volume) {
                    ui.source_error.set(error);
                }
            }
        }
        mpris::Command::SetShuffle(shuffle) => {
            set_shuffle(ui, shuffle);
        }
        mpris::Command::SetRepeat(repeat) => {
            if !ui.can_play() {
                return false;
            }
            let repeat = match repeat {
                SystemRepeat::Off => Repeat::Off,
                SystemRepeat::All => Repeat::All,
                SystemRepeat::One => Repeat::One,
            };
            set_repeat(ui, repeat);
        }
    }
    false
}

fn set_shuffle(mut ui: Ui, shuffle: bool) {
    if !ui.can_play()
        || ui.offline_mode()
        || *ui.offline_active.peek()
        || ui.remote_now.peek().is_some()
    {
        return;
    }
    if *ui.shuffle.peek() != shuffle {
        ui.shuffle.set(shuffle);
        let repeat = *ui.repeat.peek();
        ui.order.write().configure(shuffle, repeat);
    }
}

fn set_repeat(mut ui: Ui, repeat: Repeat) {
    let offline = *ui.offline_active.peek() || ui.offline_mode();
    if !ui.can_play() || ui.remote_now.peek().is_some() || (offline && repeat == Repeat::All) {
        return;
    }
    if *ui.repeat.peek() != repeat {
        ui.repeat.set(repeat);
        let (shuffle, repeat) = playback_modes(offline, false, *ui.shuffle.peek(), repeat);
        ui.order.write().configure(shuffle, repeat);
    }
}

fn volume(mut ui: Ui, value: String) {
    if !ui.can_play() {
        return;
    }
    let Ok(value) = value.parse::<f32>() else {
        return;
    };
    let Some(audio) = ui.audio.peek().clone() else {
        return;
    };
    let level = (value / 100.0).clamp(0.0, 1.0);
    // Remember the last audible level so Mute/Unmute restores what the listener
    // actually set instead of a hard-coded percentage.
    if level > 0.0 {
        ui.volume_restore.set(level);
    }
    if let Err(error) = audio.set_volume(*ui.selection.peek(), level) {
        ui.source_error.set(error);
    }
}

/// Player Mute/Unmute. The engine's own volume is set to zero (or back to the
/// remembered audible level); no second volume owner, element or output exists and
/// the current source, queue and position are untouched.
fn toggle_mute(mut ui: Ui) {
    if !ui.can_play() {
        return;
    }
    let Some(audio) = ui.audio.peek().clone() else {
        return;
    };
    let current = ui.snapshot.peek().volume.clamp(0.0, 1.0);
    let level = if current > 0.001 {
        ui.volume_restore.set(current);
        0.0
    } else {
        let remembered = ui.volume_restore.peek().clamp(0.0, 1.0);
        if remembered > 0.001 { remembered } else { 0.7 }
    };
    if let Err(error) = audio.set_volume(*ui.selection.peek(), level) {
        ui.source_error.set(error);
    }
}

fn queue_edit(mut ui: Ui, at: usize, remove: bool) {
    if !ui.signed_in() {
        return;
    }
    if remove {
        if !ui.order.write().remove(at) {
            return;
        }
        ui.queue.write().remove(at);
    } else {
        if at == 0 || !ui.order.write().swap(at, at - 1) {
            return;
        }
        ui.queue.write().swap(at, at - 1);
    }
}

fn queue_move_down(mut ui: Ui, at: usize) {
    if !ui.signed_in() || at + 1 >= ui.queue.peek().len() {
        return;
    }
    if ui.order.write().swap(at, at + 1) {
        ui.queue.write().swap(at, at + 1);
        ui.notice.set("Queue order updated.".into());
    }
}

fn queue_clear_upcoming(mut ui: Ui) {
    if !ui.signed_in() {
        return;
    }
    let Some(current) = ui.order.peek().current() else {
        return;
    };
    let keep = current + 1;
    while ui.queue.peek().len() > keep {
        let last = ui.queue.peek().len() - 1;
        if !ui.order.write().remove(last) {
            break;
        }
        ui.queue.write().remove(last);
    }
}

fn clear_queue(mut ui: Ui) {
    if !ui.signed_in() {
        return;
    }
    ui.new_selection();
    ui.prepared.set(None);
    ui.remote_now.set(None);
    ui.remote_queue.set(Vec::new());
    ui.remote_index.set(0);
    ui.offline_active.set(false);
    ui.queue.set(Vec::new());
    ui.order.write().reset(0, 0);
    ui.context.set(String::new());
    ui.notice.set("Queue cleared.".into());
}

fn cached_age_label(fetched_at: u64) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(fetched_at);
    let age = now.saturating_sub(fetched_at);
    if age < 60 {
        "just now".into()
    } else if age < 3_600 {
        format!("{}m old", age / 60)
    } else if age < 86_400 {
        format!("{}h old", age / 3_600)
    } else {
        format!("{}d old", age / 86_400)
    }
}

fn time_label(milliseconds: u64) -> String {
    let seconds = milliseconds / 1000;
    format!("{}:{:02}", seconds / 60, seconds % 60)
}

/// Apply a validated device size live without writing (slider drag).
///
/// Persistence is coalesced: the caller saves on pointer release/blur.
fn preview_app_scale(mut ui: Ui, percent: u16) {
    if !crate::presentation::scale_is_valid(percent) {
        return;
    }
    let mut next = *ui.device.peek();
    if next.app_scale_percent == percent {
        return;
    }
    next.app_scale_percent = percent;
    ui.device.set(next);
}

/// Apply a validated device size live and persist it on this device only.
///
/// An explicit choice is also the recovery path for an unreadable document: the
/// value is applied even when it equals the effective default, so Reset to 100%
/// always rewrites a usable document.
fn set_app_scale(mut ui: Ui, percent: u16) {
    if !crate::presentation::scale_is_valid(percent) {
        return;
    }
    let mut next = *ui.device.peek();
    let unchanged = next.app_scale_percent == percent;
    if unchanged && ui.device_repair.peek().is_none() {
        return;
    }
    next.app_scale_percent = percent;
    ui.device.set(next);
    persist_device(ui);
}

/// One stepper step, clamped to the documented range.
fn step_app_scale(ui: Ui, delta: i32) {
    let percent = crate::presentation::stepped(ui.device.peek().app_scale_percent, delta);
    set_app_scale(ui, percent);
}

/// Reset this device's app size to 100%, even when the stored file was unreadable.
fn reset_app_scale(ui: Ui) {
    set_app_scale(ui, crate::presentation::SCALE_DEFAULT);
}

/// Choose System/Dark/Light and persist it for this device.
fn set_device_theme(mut ui: Ui, theme: Theme) {
    let mut next = *ui.device.peek();
    if next.theme == theme && ui.device_repair.peek().is_none() {
        return;
    }
    next.theme = theme;
    ui.device.set(next);
    persist_device(ui);
}

/// One-click topbar toggle between the two explicit themes.
fn toggle_device_theme(ui: Ui) {
    let theme = match ui.device.peek().theme {
        Theme::Light => Theme::Dark,
        _ => Theme::Light,
    };
    set_device_theme(ui, theme);
}

/// Persist the current device preference document off the UI thread.
///
/// One writer at a time and only the newest pending value: repeated clicks coalesce
/// instead of racing, and status is published only for the final value. A failure
/// keeps the live value and says it is temporary; it never resets playback.
fn persist_device(mut ui: Ui) {
    let preferences = *ui.device.peek();
    ui.device_save.set(SaveStatus::Pending);
    let start = {
        let mut write = ui.device_write.borrow_mut();
        write.dirty = Some(preferences);
        if write.busy {
            false
        } else {
            write.busy = true;
            true
        }
    };
    if !start {
        return;
    }
    spawn(async move {
        loop {
            let pending = ui.device_write.borrow_mut().dirty.take();
            let Some(preferences) = pending else { break };
            let result =
                tokio::task::spawn_blocking(move || crate::presentation::save(preferences)).await;
            if !ui.alive.get() {
                ui.device_write.borrow_mut().busy = false;
                return;
            }
            if ui.device_write.borrow().dirty.is_none() {
                match result {
                    Ok(Ok(())) => {
                        ui.device_save.set(SaveStatus::Saved);
                        // The device copy is now authoritative and readable.
                        ui.device_repair.set(None);
                    }
                    _ => ui.device_save.set(SaveStatus::SessionOnly),
                }
            }
        }
        ui.device_write.borrow_mut().busy = false;
    });
}

/// AUD-05: the local-only offline library, rendered when no account is or can be
/// adopted. It lists real saved copies of exactly the profile's canonical
/// origin/account namespace, plays them through the existing `offline::prepare` path
/// and the one native `AudioEngine`, and offers nothing that needs a server: no
/// account, save, removal, favorite, search, playlist or library route, no queue
/// upload and no listening event. Every deferred result is guarded by the same
/// private offline gate as the signed-in list (AUD-03/AUD-08).
fn offline_library(ui: Ui) -> Element {
    let Some(session) = ui.offline_session.read().clone() else {
        return rsx! {};
    };
    let tracks = ui.offline.read().clone();
    let error = ui.offline_error.read().clone();
    let problem = ui.login_error.read().clone();
    let notice = ui.notice.read().clone();
    let checking = ui.offline_checking.read().clone();
    let corrupt = ui
        .offline_corrupt
        .read()
        .iter()
        .cloned()
        .collect::<std::collections::HashSet<_>>();
    let active = *ui.offline_active.read();
    let wants_play = *ui.wants_play.read();
    let current = ui.current_track().map(|track| track.id);
    let output_error = ui.audio_error.read().clone();
    let origin = session.origin.clone();
    let count = tracks.len();
    let copies = if count == 1 {
        "1 private device copy".to_owned()
    } else {
        format!("{count} private device copies")
    };
    let refresh = ui.clone();
    let sign_in = ui.clone();
    let mut notice_signal = ui.notice;
    rsx! {
        aside { class: "native-sidebar", "aria-label": "Offline library",
            div { class: "native-brand", Icon {name:"music"} span {"vela"} small {"OFFLINE"} }
            div { class: "native-private-note",
                Icon {name:"check"}
                strong {"Read and play only"}
                p {"{copies} saved for {origin}."}
                p {"Saving, removing, favorites, search, playlists and your server library need a sign-in. Nothing here is uploaded."}
            }
            button { class: "button secondary small", "data-testid": "native-offline-refresh", onclick: move |_| refresh_offline_copies(refresh.clone()), "Check saved copies" }
        }
        main { class: "native-main", "data-testid": "native-offline-library", "data-offline-origin": "{origin}", "data-offline-count": "{count}",
            header { class: "native-topbar",
                div { class: "native-window-drag", onmousedown: move |event| { if event.trigger_button() == Some(dioxus::html::input_data::MouseButton::Primary) { dioxus::desktop::window().drag(); } }, ondoubleclick: move |_| dioxus::desktop::window().toggle_maximized(),
                    span { class:"eyebrow", "OFFLINE LIBRARY" }
                    strong { "Saved on this device" }
                    small { "{origin}" }
                }
                div { class: "topbar-actions",
                    button { class: "button secondary small", "data-testid": "native-offline-signin", onclick: move |_| { let mut ui = sign_in.clone(); leave_offline_mode(&mut ui); }, "Sign in to Vela" }
                    WindowControls {}
                }
            }
            div { class: "native-scroll", "data-testid": "native-scroll",
                div { class: "native-scroll-inner",
                    if !problem.is_empty() { div { class:"alert error", role:"alert", "{problem}" } }
                    if !notice.is_empty() {
                        div { class:"alert notice", role:"status",
                            span { "{notice}" }
                            button { class:"icon-button", "aria-label":"Dismiss notification", onclick: move |_| notice_signal.set(String::new()), Icon {name:"close"} }
                        }
                    }
                    if !output_error.is_empty() { div { class:"alert error", role:"alert", "{output_error}" } }
                    div { class:"library-intro",
                        div {
                            span { class:"eyebrow", "NO SERVER CONNECTION" }
                            h1 { "Offline library" }
                            p { "Your saved private copies, played by this device. No account is signed in and no request is made." }
                        }
                    }
                    if !error.is_empty() { div { class:"alert error", role:"alert", "{error}" } }
                    if tracks.is_empty() {
                        div { class:"native-empty", Icon {name:"down"} h2 {"No saved offline copies"}
                            p {"This device has no readable private copy for {origin}. Sign in to Vela to browse your library and save one for offline listening."}
                        }
                    } else {
                        div { class:"catalog-caption", p {"Saved offline · {copies}"} span {"READ AND PLAY ONLY"} }
                        div { class:"native-queue-list", for saved in tracks.iter().cloned() {
                            {
                                let id = saved.track.id.clone();
                                let title = saved.track.title.clone();
                                let artist = saved.track.artist.trim().to_owned();
                                let device_note = format!("{} KiB on this device", saved.length_bytes / 1024);
                                let damaged = corrupt.contains(&id);
                                let checking_this = checking.as_ref().is_some_and(|entry| entry.track.id == id);
                                let playing = active && current.as_deref() == Some(id.as_str()) && wants_play;
                                let verb = if playing { "Pause" } else { "Play" };
                                let play = ui.clone();
                                let play_saved = saved.clone();
                                rsx! { article { class: if playing { "native-queue-row current" } else { "native-queue-row" }, key: "{id}", "data-offline-track-id": "{id}",
                                    ArtworkCover { ui: ui.clone(), artwork_id: saved.track.artwork_id.clone(), class: "neutral-cover", icon: "down" }
                                    div { class: "native-queue-select",
                                        div { strong { "{saved.track.title}" }
                                            small { if artist.is_empty() { "{device_note}" } else { "{artist} · {device_note}" } }
                                            small { if checking_this { "Checking this device copy…" } else if damaged { "Integrity check failed · save it again from the library" } else { "Private offline copy" } }
                                        }
                                    }
                                    button { class: "button primary small", disabled: damaged || checking_this, "aria-label": "{verb} offline {title}", onclick: move |_| play_offline(play.clone(), play_saved.clone()),
                                        if playing { Icon {name:"pause"} } else { Icon {name:"play"} } "{verb} offline"
                                    }
                                } }
                            }
                        } }
                    }
                    p { class:"native-preparation-note", "This screen is local-only. Your queue, favorites, playlists, search and history stay unavailable until you sign in to Vela; the saved copies and their private manifests are kept on this device either way." }
                }
            }
        }
        {player(ui.clone())}
    }
}

fn sign_in(ui: Ui) -> Element {
    let origin = ui.origin.read().clone();
    let username = ui.username.read().clone();
    let password = ui.password.read().clone();
    let connecting = *ui.connecting.read();
    let output_error = ui.audio_error.read().clone();
    let output_pending = !ui.audio_ready() && output_error.is_empty();
    let error = ui.login_error.read().clone();
    let notice = ui.notice.read().clone();
    let saved_session = ui.saved_session.read().clone();
    let resuming = *ui.resuming.read();
    // AUD-05: a private offline profile with real saved copies is an explicit
    // alternative to signing in. It is offered, never entered from here.
    let offline_local = ui.offline_local.read().clone();
    let mut remember = ui.remember;
    let mut appearance_signal = ui.view;
    let resume = ui.clone();
    let open_offline = ui.clone();
    let mut origin_signal = ui.origin;
    let mut username_signal = ui.username;
    let mut password_signal = ui.password;
    let submit = ui.clone();
    let cancel = ui.clone();
    rsx! { main { class: "native-signin", "aria-label": "Sign in to Vela",
        section { class: "signin-story", div { class: "native-brand", Icon {name:"music"} span {"vela"} small {"NATIVE"} }
            div {class:"signin-copy", span {class:"eyebrow","YOUR MUSIC, A LITTLE CLOSER"} h1 {"A space for" br {} "your own sound."} p {"Your private Vela library, played through this device’s native audio output."} }
            div {class:"signin-record","aria-hidden":"true",span {"v"}}
            small {class:"signin-footnote","Your account stays in memory. Sign out to clear the queue and temporary audio."}
        }
        section {class:"signin-form",span {class:"eyebrow","WELCOME TO YOUR LISTENING SPACE"} h2 {"Connect to your Vela"} p {"Use an existing account. Your administrator creates accounts on your Vela server."}
            if !error.is_empty() {div {class:"alert error",role:"alert","{error}"}}
            if !notice.is_empty() {p {class:"signin-notice",role:"status","{notice}"}}
            if let Some(saved) = saved_session {
                div { class: "signin-notice", role: "group", "aria-label": "Saved session",
                    p { "Saved session for {saved.origin}" }
                    button { class: "button secondary", disabled: connecting || resuming || output_pending, onclick: move |_| resume_saved(resume.clone()), if resuming { "Resuming…" } else { "Continue saved session" } }
                    small { "The keyring stores only this session token, not your password or CSRF token." }
                }
            }
            if let Some(local) = offline_local {
                div { class: "signin-notice", role: "group", "aria-label": "Offline library",
                    p { "Saved offline copies for {local.origin} are on this device." }
                    button { class: "button secondary", "data-testid": "native-offline-open", disabled: connecting || resuming, onclick: move |_| open_offline_library(open_offline.clone()), "Open offline library" }
                    small { "Read and play only: saving, removing, favorites, search and your server library need a sign-in." }
                }
            }
            if output_pending {p {class:"signin-notice",role:"status","data-testid":"native-output-status","Preparing native audio output…"}}
            if !output_error.is_empty() {div {class:"alert error",role:"alert","{output_error} You can browse your library; playback needs an available audio output."}}
            form {onsubmit:move |event|{event.prevent_default();login(submit.clone());},
                label {r#for:"native-origin","Server origin"}
                input {id:"native-origin",r#type:"url",required:true,maxlength:2048,value:"{origin}",disabled:connecting,autocomplete:"url",placeholder:"https://music.example.org",oninput:move |event|origin_signal.set(event.value())}
                small {"HTTPS for remote servers; numeric loopback HTTP is available for local development."}
                label {r#for:"native-username","Username"}
                input {id:"native-username",required:true,maxlength:32,value:"{username}",disabled:connecting,autocomplete:"username",oninput:move |event|username_signal.set(event.value())}
                label {r#for:"native-password","Password"}
                input {id:"native-password",r#type:"password",required:true,value:"{password}",disabled:connecting,autocomplete:"off",oninput:move |event|password_signal.set(event.value())}
                label { class: "checkbox-label", r#for: "native-remember", input { id: "native-remember", r#type: "checkbox", checked: remember(), disabled: connecting, onchange: move |event| remember.set(event.checked()) } span { "Remember this session on this device" } }
                button {class:"button primary",r#type:"submit",disabled:connecting||output_pending,if connecting {"Connecting…"} else if output_pending {"Preparing native output…"} else {"Sign in"}}
                if connecting {button {class:"button secondary",r#type:"button",onclick:move |_|{let mut ui=cancel.clone();ui.login_gate.cancel();ui.connecting.set(false);ui.login_error.set("Connection canceled. Enter your password to try again.".into());},"Cancel connection"}}
            }
            small {class:"signin-limit","Opt-in session saving uses the system Secret Service and stores only the opaque session token; the password and CSRF token are never stored. Signing out deletes the saved session. No listening history is sent."}
            button {class:"button secondary small signin-appearance",onclick:move |_|appearance_signal.set("settings"),Icon {name:"settings"}"Appearance and this device"}
        }
    } }
}

/// Which local identity an offline operation belongs to. Both name a canonical
/// origin plus account UUID, so the namespace and freshness rules are identical; only
/// the live source differs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OfflineIdentity {
    /// A real signed-in account.
    Account,
    /// The private offline profile (AUD-05): no account exists.
    Profile,
}

/// AUD-03/AUD-08: the identity plus operation generation a deferred offline result
/// belongs to. Every signal write after an `await` is re-checked against it, so a
/// delayed delete, reload or verification from a replaced account *or* a replaced
/// offline profile can never overwrite the current list or playback state.
#[derive(Clone)]
struct OfflineOwner {
    gate: Gate,
    generation: u64,
    epoch: u64,
    user_id: String,
    origin: String,
    identity: OfflineIdentity,
}

impl OfflineOwner {
    fn new(gate: &Gate, ticket: &Ticket, account: &Account) -> Self {
        Self {
            gate: gate.clone(),
            generation: ticket.generation,
            epoch: account.epoch,
            user_id: account.user.id.clone(),
            origin: account.origin.clone(),
            identity: OfflineIdentity::Account,
        }
    }
    /// The same guard for the local-only offline screen, whose live identity is the
    /// private profile rather than an account.
    fn profile(gate: &Gate, ticket: &Ticket, epoch: u64, session: &OfflineSession) -> Self {
        Self {
            gate: gate.clone(),
            generation: ticket.generation,
            epoch,
            user_id: session.account_id.clone(),
            origin: session.origin.clone(),
            identity: OfflineIdentity::Profile,
        }
    }
    /// The whole freshness rule as one pure predicate, so it is testable without
    /// a native window: the operation generation, the account epoch and the
    /// exact identity (user id *and* origin) must all still be the live ones, the
    /// ticket must be uncancelled and the window still open.
    fn matches(
        &self,
        alive: bool,
        canceled: bool,
        generation: u64,
        epoch: u64,
        account: Option<(&str, &str)>,
    ) -> bool {
        alive
            && !canceled
            && generation == self.generation
            && epoch == self.epoch
            && account
                .is_some_and(|(user_id, origin)| user_id == self.user_id && origin == self.origin)
    }
    fn current(&self, ui: &Ui, ticket: &Ticket) -> bool {
        let live = match self.identity {
            OfflineIdentity::Account => ui
                .account
                .peek()
                .as_ref()
                .map(|account| (account.user.id.clone(), account.origin.clone())),
            OfflineIdentity::Profile => ui
                .offline_session
                .peek()
                .as_ref()
                .map(|session| (session.account_id.clone(), session.origin.clone())),
        };
        self.matches(
            ui.alive.get(),
            ticket.cancel.is_cancelled(),
            self.gate.generation.get(),
            *ui.epoch.peek(),
            live.as_ref()
                .map(|(user_id, origin)| (user_id.as_str(), origin.as_str())),
        )
    }
}

/// AUD-06: the local namespace one offline operation may touch — the canonical
/// origin plus the account UUID. It is derived from the same owner the freshness
/// rule checks, so a superseded operation cannot even name a newer account's (or
/// another origin's) directory, and the blocking worker carries only these two
/// strings: never a media path, token or credential.
#[derive(Clone)]
struct OfflineScope {
    origin: String,
    account_id: String,
}

impl OfflineScope {
    fn new(owner: &OfflineOwner) -> Self {
        Self {
            origin: owner.origin.clone(),
            account_id: owner.user_id.clone(),
        }
    }
}

/// AUD-07: what removing one saved copy does to the live offline playback.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OfflineRemoval {
    /// The decoder owns the removed occurrence: stop and clear the queue.
    StopPlayback,
    /// Drop this occurrence from the queue and remap the order model; the active
    /// source, its position and the other occurrences stay untouched.
    Remap(usize),
    /// Nothing in the live playback refers to the removed copy.
    Keep,
}

fn offline_removal(
    offline_active: bool,
    current: Option<usize>,
    queue: &[Track],
    track_id: &str,
) -> OfflineRemoval {
    if !offline_active {
        return OfflineRemoval::Keep;
    }
    if current
        .and_then(|at| queue.get(at))
        .is_some_and(|track| track.id == track_id)
    {
        return OfflineRemoval::StopPlayback;
    }
    match queue.iter().position(|track| track.id == track_id) {
        Some(at) => OfflineRemoval::Remap(at),
        None => OfflineRemoval::Keep,
    }
}

/// Stop the offline decoder and queue without touching any saved copy.
fn stop_offline_playback(mut ui: Ui) {
    // `new_selection` also cancels a pending offline verification.
    ui.new_selection();
    ui.prepared.set(None);
    ui.offline_active.set(false);
    ui.queue.set(Vec::new());
    ui.order.write().reset(0, 0);
    // The removed occurrence is gone, so the player must not keep claiming this
    // source context is still playing.
    ui.context.set(String::new());
}

/// AUD-07: the live offline queue is a projection of the saved-copy list. When
/// the two disagree, rebuild the projection from the list and keep the decoder
/// that is already playing untouched.
fn sync_offline_queue(mut ui: Ui) {
    if !*ui.offline_active.peek() {
        return;
    }
    let playing = ui.current_track().map(|track| track.id);
    let queue = ui
        .offline
        .peek()
        .iter()
        .map(|entry| entry.track.clone())
        .collect::<Vec<_>>();
    let Some(at) = playing
        .as_deref()
        .and_then(|id| queue.iter().position(|track| track.id == id))
    else {
        // Nothing the decoder owns is still on this device: stop instead of
        // claiming another occurrence is playing.
        stop_offline_playback(ui);
        return;
    };
    let length = queue.len();
    ui.queue.set(queue);
    ui.order.write().reset(length, at);
}

/// AUD-07: add a newly saved copy to the live offline queue at the position the
/// saved-copy list gives it, without resetting the shuffle deck/history.
fn offline_queue_added(mut ui: Ui, track_id: &str) {
    if !*ui.offline_active.peek() {
        return;
    }
    let position = {
        let saved = ui.offline.peek();
        saved
            .iter()
            .position(|entry| entry.track.id == track_id)
            .map(|at| (at, saved[at].track.clone()))
    };
    let Some((at, track)) = position else {
        return;
    };
    let at = at.min(ui.queue.peek().len());
    if ui.order.write().insert(at, false) {
        ui.queue.write().insert(at, track);
    } else {
        sync_offline_queue(ui);
    }
}

fn download_offline(mut ui: Ui, track: Track) {
    if !ui.offline_busy.peek().is_empty()
        || ui
            .offline
            .peek()
            .iter()
            .any(|saved| saved.track.id == track.id)
    {
        return;
    }
    let Some(account) = ui.account.peek().clone().filter(|_| ui.signed_in()) else {
        return;
    };
    let track_id = track.id.clone();
    let ticket = ui.offline_gate.begin(account.epoch);
    let owner = OfflineOwner::new(&ui.offline_gate, &ticket, &account);
    ui.offline_busy.set(track_id.clone());
    ui.offline_error.set(String::new());
    spawn(async move {
        let (progress, _updates) = tokio::sync::watch::channel(DownloadProgress::default());
        let result = account
            .client
            .prepare(&track, ticket.cancel.clone(), progress)
            .await;
        if !owner.current(&ui, &ticket) {
            return;
        }
        let prepared = match result {
            Ok(prepared) => prepared,
            Err(error) => {
                ui.offline_busy.set(String::new());
                if !expired(ui.clone(), &error) {
                    ui.offline_error.set(error);
                }
                return;
            }
        };
        let scope = OfflineScope::new(&owner);
        let saved = tokio::task::spawn_blocking({
            let scope = scope.clone();
            let track = track.clone();
            let cancel = ticket.cancel.clone();
            move || {
                crate::offline::save(&scope.origin, &scope.account_id, &track, &prepared, &|| {
                    cancel.is_cancelled()
                })
            }
        })
        .await;
        // AUD-08: the copy finished, but the account may have been replaced.
        if !owner.current(&ui, &ticket) {
            return;
        }
        ui.offline_busy.set(String::new());
        match saved {
            Ok(Ok(())) => {
                let loaded = tokio::task::spawn_blocking({
                    let scope = scope.clone();
                    move || crate::offline::load(&scope.origin, &scope.account_id)
                })
                .await;
                // AUD-08: this reload awaited as well; only this operation may
                // publish it.
                if !owner.current(&ui, &ticket) {
                    return;
                }
                match loaded {
                    Ok(Ok(tracks)) => {
                        ui.offline.set(tracks);
                        offline_queue_added(ui.clone(), &track_id);
                    }
                    Ok(Err(error)) => ui.offline_error.set(error),
                    Err(_) => ui
                        .offline_error
                        .set("Saved offline music could not be read.".into()),
                }
                ui.offline_corrupt
                    .write()
                    .retain(|track| track != &track_id);
                ui.notice.set("Saved for offline listening.".into());
            }
            Ok(Err(error)) => {
                if error != crate::offline::CANCELED {
                    ui.offline_error.set(error);
                }
            }
            Err(_) => ui
                .offline_error
                .set("The offline copy could not be written.".into()),
        }
    });
}

fn play_offline(ui: Ui, saved: crate::offline::OfflineTrack) {
    let Some(index) = ui
        .offline
        .peek()
        .iter()
        .position(|entry| entry.track.id == saved.track.id)
    else {
        return;
    };
    if *ui.offline_active.peek() && ui.current_track().is_some_and(|t| t.id == saved.track.id) {
        toggle(ui);
    } else {
        start_offline_at(ui, index);
    }
}

fn start_offline_at(mut ui: Ui, index: usize) {
    if !ui.can_play() {
        return;
    }
    let Some(saved) = ui.offline.peek().get(index).cloned() else {
        return;
    };
    let Some(audio) = ui.audio.peek().clone() else {
        return;
    };
    // AUD-05: the private namespace a copy lives in is named either by the live
    // account or by the local offline profile. The offline screen has no account at
    // all, so the identity and its freshness guard are chosen here.
    let epoch = *ui.epoch.peek();
    let (session, owner, ticket) =
        if let Some(account) = ui.account.peek().clone().filter(|_| ui.signed_in()) {
            let ticket = ui.offline_play_gate.begin(account.epoch);
            let owner = OfflineOwner::new(&ui.offline_play_gate, &ticket, &account);
            (
                OfflineSession {
                    origin: account.origin,
                    account_id: account.user.id,
                },
                owner,
                ticket,
            )
        } else if let Some(session) = ui.offline_session.peek().clone() {
            let ticket = ui.offline_play_gate.begin(epoch);
            let owner = OfflineOwner::profile(&ui.offline_play_gate, &ticket, epoch, &session);
            (session, owner, ticket)
        } else {
            return;
        };
    // AUD-03: the manifest scan and the BLAKE3 hash of up to 250 MiB belong to a
    // blocking worker, never to this thread. The ticket exists before that
    // worker starts, so Cancel/Next/sign-out cannot be racing it, and nothing is
    // stopped or replaced until the copy has been verified.
    ui.offline_checking.set(Some(saved.clone()));
    ui.offline_error.set(String::new());
    ui.source_error.set(String::new());
    let track_id = saved.track.id.clone();
    let title = saved.track.title.clone();
    spawn(async move {
        let prepared = tokio::task::spawn_blocking({
            let track_id = track_id.clone();
            let cancel = ticket.cancel.clone();
            let session = session.clone();
            move || {
                crate::offline::prepare(&session.origin, &session.account_id, &track_id, &|| {
                    cancel.is_cancelled()
                })
            }
        })
        .await;
        // AUD-03/AUD-08: a late, canceled or superseded verification writes
        // nothing and drops its descriptor lease with `prepared`.
        if !owner.current(&ui, &ticket) {
            return;
        }
        ui.offline_checking.set(None);
        let prepared = match prepared {
            Ok(Ok(prepared)) => prepared,
            Ok(Err(error)) => {
                if error != crate::offline::CANCELED {
                    ui.offline_error.set(error);
                }
                return;
            }
            Err(_) => {
                ui.offline_error
                    .set("The offline copy could not be checked.".into());
                return;
            }
        };
        // The saved-copy list may have changed while the copy was verified; the
        // queue is rebuilt from it, so resolve the occurrence by track id.
        let queue = ui
            .offline
            .peek()
            .iter()
            .map(|entry| entry.track.clone())
            .collect::<Vec<_>>();
        let Some(at) = queue.iter().position(|track| track.id == track_id) else {
            ui.offline_error
                .set("The offline copy is missing. Save it again from the library.".into());
            return;
        };
        let length = queue.len();
        let selection = ui.new_selection();
        ui.offline_active.set(true);
        ui.wants_play.set(true);
        ui.remote_now.set(None);
        ui.remote_queue.set(Vec::new());
        ui.remote_index.set(0);
        ui.queue.set(queue);
        ui.order.write().reset(length, at);
        let (shuffle, repeat) = playback_modes(true, false, *ui.shuffle.peek(), *ui.repeat.peek());
        ui.order.write().configure(shuffle, repeat);
        ui.context.set(format!("Offline · {title}"));
        ui.prepared.set(None);
        match audio.load(selection, prepared.clone(), *ui.wants_play.peek(), 0) {
            Ok(()) => ui.prepared.set(Some(prepared)),
            Err(error) => {
                ui.offline_active.set(false);
                ui.source_error.set(error);
            }
        }
    });
}

fn offline_step(ui: Ui, delta: i32) {
    if !ui.can_play() {
        return;
    }
    let current = ui.order.read().current();
    let Some(current) = current else {
        return;
    };
    let Some(next) = crate::remote::queue_target(current, ui.queue.peek().len(), delta) else {
        return;
    };
    // AUD-07: the occurrence index belongs to the queue, so resolve the target
    // track first and only then look up its position in the saved-copy list.
    let Some(track_id) = ui.queue.peek().get(next).map(|track| track.id.clone()) else {
        return;
    };
    let Some(index) = ui
        .offline
        .peek()
        .iter()
        .position(|entry| entry.track.id == track_id)
    else {
        // The copy left the list between the two projections: resynchronize
        // instead of starting a track that is no longer on this device.
        sync_offline_queue(ui);
        return;
    };
    start_offline_at(ui, index);
}

fn remove_offline(mut ui: Ui, track_id: String) {
    // Removals are single-flight: a second removal may not interleave with the
    // delete/reload of the first one.
    if !ui.offline_removing.peek().is_empty() || !ui.signed_in() {
        return;
    }
    let Some(account) = ui.account.peek().clone() else {
        return;
    };
    // A copy under verification is going away: cancel the blocking hash first so
    // that worker stops at its next block rather than after the whole file.
    if ui
        .offline_checking
        .peek()
        .as_ref()
        .is_some_and(|entry| entry.track.id == track_id)
    {
        ui.offline_play_gate.cancel();
        ui.offline_checking.set(None);
    }
    let ticket = ui.offline_list_gate.begin(account.epoch);
    let owner = OfflineOwner::new(&ui.offline_list_gate, &ticket, &account);
    ui.offline_removing.set(track_id.clone());
    ui.offline_error.set(String::new());
    spawn(async move {
        let scope = OfflineScope::new(&owner);
        let removed = tokio::task::spawn_blocking({
            let scope = scope.clone();
            let track_id = track_id.clone();
            move || crate::offline::remove(&scope.origin, &scope.account_id, &track_id)
        })
        .await;
        // AUD-08: the delete ran while this operation could have been replaced
        // (sign-out, another account, a newer removal).
        if !owner.current(&ui, &ticket) {
            return;
        }
        match removed {
            Ok(Ok(())) => {}
            Ok(Err(error)) => {
                ui.offline_removing.set(String::new());
                ui.offline_error.set(error);
                return;
            }
            Err(_) => {
                ui.offline_removing.set(String::new());
                ui.offline_error
                    .set("The offline copy could not be removed.".into());
                return;
            }
        }
        ui.offline_confirm.set(String::new());
        // AUD-07: only the occurrence the decoder owns stops playback. Any other
        // saved copy is dropped from the queue and the order model while the
        // active source and its position keep playing.
        let effect = offline_removal(
            *ui.offline_active.peek(),
            ui.order.peek().current(),
            &ui.queue.peek(),
            &track_id,
        );
        match effect {
            OfflineRemoval::StopPlayback => stop_offline_playback(ui.clone()),
            OfflineRemoval::Remap(at) => {
                // Every structural queue edit updates the order model in the
                // same step, so Next/Previous indices stay valid.
                if !ui.order.write().remove(at) {
                    sync_offline_queue(ui.clone());
                } else {
                    ui.queue.write().remove(at);
                }
            }
            OfflineRemoval::Keep => {}
        }
        let reloaded = tokio::task::spawn_blocking({
            move || crate::offline::load(&scope.origin, &scope.account_id)
        })
        .await;
        // AUD-08: the reload awaited again; a stale operation may not replace
        // the list a newer login/removal already published.
        if !owner.current(&ui, &ticket) {
            return;
        }
        ui.offline_removing.set(String::new());
        match reloaded {
            Ok(Ok(tracks)) => ui.offline.set(tracks),
            Ok(Err(error)) => ui.offline_error.set(error),
            Err(_) => ui
                .offline_error
                .set("Saved offline music could not be read.".into()),
        }
        ui.offline_corrupt
            .write()
            .retain(|track| track != &track_id);
        ui.notice.set("Offline copy removed.".into());
    });
}

/// IN01: the pending private-library page is one bounded list of placeholder rows.
/// The count is fixed and small: the request itself carries no page size, so the
/// skeleton never claims how many rows are coming.
const LIBRARY_SKELETON_ROWS: usize = 8;

fn library(ui: Ui) -> Element {
    let loading = *ui.catalog_loading.read();
    let error = ui.catalog_error.read().clone();
    let draft = ui.query_draft.read().clone();
    let query = ui.query.read().clone();
    let favorites = *ui.view.read() == "favorites";
    let catalog = ui
        .catalog
        .read()
        .clone()
        .filter(|catalog| ui.catalog_current(catalog));
    let catalog_items = catalog
        .as_ref()
        .map_or(0, |catalog| catalog.page.items.len());
    let mut search = ui.query_draft;
    let submit = ui.clone();
    let refresh = ui.clone();
    let first = ui.clone();
    let next = ui.clone();
    let offline = ui.offline.read().clone();
    let offline_busy = ui.offline_busy.read().clone();
    let offline_error = ui.offline_error.read().clone();
    let offline_confirm = ui.offline_confirm.read().clone();
    let offline_removing = ui.offline_removing.read().clone();
    let offline_corrupt = ui
        .offline_corrupt
        .read()
        .iter()
        .cloned()
        .collect::<std::collections::HashSet<_>>();
    let offline_ids = offline
        .iter()
        .map(|saved| saved.track.id.clone())
        .collect::<std::collections::HashSet<_>>();
    rsx! {section {class:"native-library","aria-label":if favorites {"Favorite tracks"} else {"Private library"},"data-view":if favorites {"favorites"} else {"library"},"data-query":"{query}","data-loading":"{loading}","aria-busy":"{loading}","data-error":"{error}","data-catalog-items":"{catalog_items}",
        div {class:"library-intro",div {h1 {if favorites {"Your favorites"} else {"Your library"}} p {if favorites {"The tracks you love, all in one place."} else {"Find your next listen."}}}
            button {class:"button secondary",disabled:loading,onclick:move |_|{let query=refresh.query.peek().clone();let cursor=refresh.cursor.peek().clone();let favorites=*refresh.view.peek()=="favorites";load_catalog(refresh.clone(),query,cursor,favorites);},Icon {name:"history"} if favorites {"Refresh favorites"} else {"Refresh library"}}
        }
        form {class:"native-search",onsubmit:move |event|{event.prevent_default();let query=submit.query_draft.peek().trim().to_string();let favorites=*submit.view.peek()=="favorites";load_catalog(submit.clone(),query,None,favorites);},
            Icon {name:"search"} label {class:"sr-only",r#for:"native-search",if favorites {"Search your favorites"} else {"Search your library"}}
            input {id:"native-search",value:"{draft}",maxlength:256,placeholder:"Search titles, artists or albums",oninput:move |event|search.set(event.value())}
            button {class:"button primary",r#type:"submit","Search"}
        }
        if !offline.is_empty() {
            div {class:"catalog-caption",p {"Saved offline · {offline.len()} private device copies"} span {"NO SERVER NEEDED"}}
            div {class:"native-queue-list",for saved in offline.iter().cloned() {
                {
                    let play_handler = ui.clone();
                    let confirm_ui = ui.clone();
                    let cancel_ui = ui.clone();
                    let set_confirm_ui = ui.clone();
                    let confirm_id = saved.track.id.clone();
                    let offline_id = saved.track.id.clone();
                    let offline_damaged = offline_corrupt.contains(&offline_id);
                    let offline_removing_this = offline_removing == offline_id;
                    let set_confirm_id = offline_id.clone();
                    let play_saved = saved.clone();
                    let title = saved.track.title.clone();
                    let active=*ui.offline_active.read() && ui.current_track().is_some_and(|t|t.id==saved.track.id) && *ui.wants_play.read();
                    let verb=if active {"Pause"}else{"Play"};
                    rsx! { article {class:"native-queue-row",key:"offline-{offline_id.clone()}","data-offline-track-id":"{offline_id.clone()}",
                        ArtworkCover {ui:ui.clone(),artwork_id:saved.track.artwork_id.clone(),class:"neutral-cover",icon:"down"}
                        div {class:"native-queue-select",div {strong {"{saved.track.title}"} small {"{saved.track.artist} · {saved.length_bytes / 1024} KiB on this device"} small {if offline_removing_this {"Removing this device copy…"} else if offline_damaged {"Integrity check failed · save it again"} else {"Private offline copy"}}}}
                        button {class:"button primary small",disabled:offline_damaged||offline_removing_this,"aria-label":"{verb} offline {title}",onclick:move |_|play_offline(play_handler.clone(),play_saved.clone()),if active {Icon {name:"pause"}}else{Icon {name:"play"}} "{verb} offline"}
                        if offline_confirm == offline_id {
                            button {class:"button danger small",disabled:offline_removing_this,"aria-label":"Confirm remove offline {title}",onclick:move |_|remove_offline(confirm_ui.clone(),confirm_id.clone()),if offline_removing_this {"Removing…"} else {"Confirm remove"}}
                            button {class:"button secondary small",disabled:offline_removing_this,"aria-label":"Cancel remove offline {title}",onclick:move |_|{let mut ui=cancel_ui.clone();ui.offline_confirm.set(String::new());},"Cancel"}
                        } else {
                            button {class:"icon-button","aria-label":"Remove offline copy {title}",title:"Remove only this device copy; the original and server library stay.",onclick:move |_|{let mut ui=set_confirm_ui.clone();ui.offline_confirm.set(set_confirm_id.clone());},Icon {name:"close"}}
                        }
                    } }
                }
            } }
        }
        if !offline_error.is_empty() {div {class:"alert error",role:"alert","{offline_error}"}}
        div {class:"catalog-caption",p {if favorites {if query.is_empty() {"Your favorites · Up to 50 per page"} else {"Favorite results for “{query}” · Up to 50 per page"}} else if query.is_empty() {"Your private tracks · Up to 50 per page"} else {"Results for “{query}” · Up to 50 per page"}} span {"ORIGINAL AUDIO"}}
        if loading {
            // IN01: the pending request replaces the old text-only indicator with
            // geometry-matched skeleton rows. Every row is a real `.native-track`
            // (same grid, height and padding) whose artwork block is a `.neutral-cover`,
            // so the settled page replaces it in place. The rows carry no title,
            // artist, album, duration or percentage, render no controls and request no
            // artwork; audio, queue and the selected source are untouched.
            div {class:"native-skeleton-status",role:"status","aria-live":"polite",
                span {class:"sr-only",if favorites {"Loading your favorites…"} else {"Loading your library…"}}
            }
            div {class:"native-track-table native-skeleton-table","aria-busy":"true","data-native-skeleton":"{LIBRARY_SKELETON_ROWS}",
                div {class:"native-track-header","aria-hidden":"true",span {""} span {"TITLE / ARTIST"} span {"ALBUM"} span {"TIME"} span {"QUEUE"}}
                for at in 0..LIBRARY_SKELETON_ROWS {
                    article {class:"native-track native-skeleton-row",key:"library-skeleton-{at}","aria-hidden":"true","data-native-skeleton-row":"{at}",
                        span {class:"native-skeleton-bar native-skeleton-start"}
                        div {class:"native-track-name",
                            div {class:"neutral-cover native-skeleton-art","aria-hidden":"true"}
                            div {class:"native-skeleton-lines",span {class:"native-skeleton-bar native-skeleton-title"} span {class:"native-skeleton-bar native-skeleton-artist"}}
                        }
                        div {class:"native-track-album",span {class:"native-skeleton-bar native-skeleton-album"}}
                        div {class:"native-track-time",span {class:"native-skeleton-bar native-skeleton-time"}}
                        div {class:"native-track-actions",span {class:"native-skeleton-bar native-skeleton-actions"}}
                    }
                }
            }
        }
        if !error.is_empty() {div {class:"alert error",role:"alert","{error} Refresh the library to try again."}}
        if let Some(catalog) = catalog {
            if catalog.page.items.is_empty() {div {class:"native-empty",Icon {name:"heart"} h2 {if favorites {if query.is_empty() {"No favorites yet"} else {"No matching favorites"}} else if query.is_empty() {"Make room for your first listen"} else {"No matching tracks"}} p {if favorites {if query.is_empty() {"Favorite a track from your library to find it here. Favorites are private to your account."} else {"Try a different title, artist or album. Only your favorite tracks are searched."}} else if query.is_empty() {"Add music in Vela’s web app, then refresh this library. Your tracks stay private."} else {"Try a different title, artist or album. Search uses only your own library."}}}}
            else {div {class:"native-track-table",div {class:"native-track-header","aria-hidden":"true",span {""} span {"TITLE / ARTIST"} span {"ALBUM"} span {"TIME"} span {"QUEUE"}}
                for (at,track) in catalog.page.items.iter().cloned().enumerate() { {
                    let play_ui=ui.clone();let played_catalog=catalog.clone();let next_ui=ui.clone();let next_catalog=catalog.clone();let last_ui=ui.clone();let last_catalog=catalog.clone();let next_track=track.clone();let last_track=track.clone();let favorite_ui=ui.clone();let favorite_catalog=catalog.clone();let favorite_track=track.clone();let offline_ui=ui.clone();let offline_track=track.clone();let offline_saved=offline_ids.contains(&track.id);let offline_working=offline_busy==track.id;let offline_title=track.title.clone();
                    let current=ui.current_track().is_some_and(|current|current.id==track.id);
                    let favorite_disabled=ui.favorite_busy.read().as_str()==track.id;
                    let row_playing=current && *ui.wants_play.read();
                    let row_action=if row_playing {"Pause"}else{"Play"};
                    rsx! {article {class:if current {"native-track current"} else {"native-track"},key:"{track.id}","data-native-track-id":"{track.id}",
                        button {class:"icon-button track-start",disabled:!ui.audio_ready(),title:if !ui.audio_ready() {"Native audio output is unavailable"} else {"Play original"},"aria-label":"{row_action} {track.title}","aria-pressed":row_playing,onclick:move |_|play_catalog(play_ui.clone(),played_catalog.clone(),at),if row_playing {Icon {name:"pause"}}else{Icon {name:"play"}}}
                        div {class:"native-track-name",ArtworkCover {ui:ui.clone(),artwork_id:track.artwork_id.clone(),class:"neutral-cover",icon:"music"} div {strong {"{track.title}"} small {"{track.artist}"}}}
                        span {class:"native-track-album","{track.album}"}
                        span {class:"native-track-time","{time_label(track.duration_ms.max(0) as u64)}"}
                        div {class:"native-track-actions",
                            button {class:if track.favorite {"icon-button selected"} else {"icon-button"},"aria-pressed":track.favorite,disabled:favorite_disabled,"aria-label":if track.favorite {format!("Remove {} from favorites",track.title)} else {format!("Favorite {}",track.title)},title:if favorite_disabled {"Updating this favorite…"} else if track.favorite {"Remove from favorites"} else {"Add to favorites"},onclick:move |_|toggle_favorite(favorite_ui.clone(),favorite_catalog.clone(),favorite_track.clone()),Icon {name:"heart"}}
                            button {class:"icon-button","aria-label":"Play {track.title} next",title:"Play next",onclick:move |_|{if next_ui.catalog_current(&next_catalog) {enqueue(next_ui.clone(),next_track.clone(),true);}},Icon {name:"next-add"}}
                            button {class:"icon-button","aria-label":"Add {track.title} to queue",title:"Add to queue",onclick:move |_|{if last_ui.catalog_current(&last_catalog){enqueue(last_ui.clone(),last_track.clone(),false);}},Icon {name:"plus"}}
                            button {class:"icon-button",disabled:offline_saved||offline_working,"aria-label":if offline_saved {format!("Already saved offline: {offline_title}")} else if offline_working {format!("Saving offline: {offline_title}")} else {format!("Save {offline_title} offline")},title:if offline_saved {"Saved on this device"} else if offline_working {"Downloading a private copy…"} else {"Save a private copy for offline listening"},onclick:move |_|download_offline(offline_ui.clone(),offline_track.clone()),Icon {name:"down"}}
                        }
                    } }
                } }
            } }
            div {class:"native-pagination",button {class:"button secondary",disabled:ui.cursor.peek().is_none(),onclick:move |_|{let query=first.query.peek().clone();let favorites=first.catalog.peek().as_ref().is_some_and(|c|c.favorites);load_catalog(first.clone(),query,None,favorites);},"First page"}
                if let Some(cursor)=catalog.page.next_cursor.clone() {button {class:"button secondary",onclick:move |_|{let query=next.query.peek().clone();let favorites=next.catalog.peek().as_ref().is_some_and(|c|c.favorites);load_catalog(next.clone(),query,Some(cursor.clone()),favorites);},"Next page"}}
                small {"Only this page is queued when you play a result."}
            }
        }
    } }
}

fn load_groups(mut ui: Ui, kind: &'static str, cursor: Option<String>) {
    if !matches!(kind, "albums" | "artists") {
        return;
    }
    let Some(account) = ui.account.peek().clone().filter(|_| ui.signed_in()) else {
        return;
    };
    ui.browse_gate.cancel();
    ui.view.set(kind);
    ui.browse_kind.set(kind);
    ui.browse_id.set(None);
    ui.browse_title.set(String::new());
    ui.browse_tracks.set(Vec::new());
    ui.browse_loading.set(true);
    ui.browse_error.set(String::new());
    let ticket = ui.browse_gate.begin(account.epoch);
    spawn(async move {
        if kind == "albums" {
            let result = account
                .client
                .albums(cursor.as_deref(), ticket.cancel.clone())
                .await;
            if !ticket.current(&ui) || *ui.view.peek() != "albums" {
                return;
            }
            ui.browse_loading.set(false);
            match result {
                Ok(page) => {
                    ui.browse_error.set(String::new());
                    ui.albums.set(page.items);
                }
                Err(error) => {
                    if !expired(ui.clone(), &error) {
                        ui.browse_error.set(error);
                    }
                }
            }
        } else {
            let result = account
                .client
                .artists(cursor.as_deref(), ticket.cancel.clone())
                .await;
            if !ticket.current(&ui) || *ui.view.peek() != "artists" {
                return;
            }
            ui.browse_loading.set(false);
            match result {
                Ok(page) => {
                    ui.browse_error.set(String::new());
                    ui.artists.set(page.items);
                }
                Err(error) => {
                    if !expired(ui.clone(), &error) {
                        ui.browse_error.set(error);
                    }
                }
            }
        }
    });
}

fn open_group(mut ui: Ui, kind: &'static str, id: String) {
    let Some(account) = ui.account.peek().clone().filter(|_| ui.signed_in()) else {
        return;
    };
    if !uuid::Uuid::parse_str(&id).is_ok_and(|parsed| parsed.to_string() == id) {
        ui.browse_error.set("Select a valid group.".into());
        return;
    }
    ui.browse_gate.cancel();
    ui.browse_kind.set(kind);
    ui.browse_id.set(Some(id.clone()));
    ui.browse_title.set(String::new());
    ui.browse_tracks.set(Vec::new());
    ui.browse_loading.set(true);
    ui.browse_error.set(String::new());
    let ticket = ui.browse_gate.begin(account.epoch);
    spawn(async move {
        let result = if kind == "albums" {
            match account.client.album(&id, ticket.cancel.clone()).await {
                Ok(album) => account
                    .client
                    .album_tracks(&id, None, ticket.cancel.clone())
                    .await
                    .map(|page| (format!("{} · {}", album.title, album.artist), page.items)),
                Err(error) => Err(error),
            }
        } else {
            match account.client.artist(&id, ticket.cancel.clone()).await {
                Ok(artist) => account
                    .client
                    .artist_tracks(&id, None, ticket.cancel.clone())
                    .await
                    .map(|page| (artist.name, page.items)),
                Err(error) => Err(error),
            }
        };
        if !ticket.current(&ui)
            || *ui.view.peek() != kind
            || ui.browse_id.peek().as_deref() != Some(id.as_str())
        {
            return;
        }
        ui.browse_loading.set(false);
        match result {
            Ok((title, tracks)) => {
                ui.browse_error.set(String::new());
                ui.browse_title.set(title);
                ui.browse_tracks.set(tracks);
            }
            Err(error) => {
                if !expired(ui.clone(), &error) {
                    ui.browse_error.set(error);
                }
            }
        }
    });
}

fn play_group(ui: Ui) {
    if !ui.signed_in() || ui.audio.peek().is_none() || ui.browse_id.peek().is_none() {
        return;
    }
    let tracks = ui.browse_tracks.peek().clone();
    if tracks.is_empty() {
        return;
    }
    let title = ui.browse_title.peek().clone();
    queue_owned_context(ui, tracks, 0, title);
}

fn playlists_view(ui: Ui) -> Element {
    let loading = *ui.playlist_loading.read();
    let error = ui.playlist_error.read().clone();
    let lists = ui.playlists.read().clone();
    let folders = ui.folders.read().clone();
    let folder_filter_value = ui.folder_filter.read().clone();
    let filtered_lists = lists
        .iter()
        .filter(|list| {
            folder_filter_value
                .as_deref()
                .is_none_or(|id| list.folder_id.as_deref() == Some(id))
        })
        .cloned()
        .collect::<Vec<_>>();
    let folder_busy = *ui.folder_busy.read();
    let folder_confirm = ui.folder_confirm.read().clone();
    let playlist_folder_target_value = ui.playlist_folder_target.read().clone();
    let mut playlist_folder_target = ui.playlist_folder_target;
    let folder_create = ui.clone();
    let folder_rename_ui = ui.clone();
    let mut folder_name = ui.folder_name_draft;
    let mut folder_rename = ui.folder_rename_draft;
    let detail = ui.playlist.read().clone();
    let playable_tracks = detail
        .as_ref()
        .map(|d| {
            d.items
                .iter()
                .filter(|i| i.can_stream)
                .map(|i| i.track.clone())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let refresh = ui.clone();
    let back = ui.clone();
    let play = ui.clone();
    let create = ui.clone();
    let rename = ui.clone();
    let delete = ui.clone();
    let busy = ui.playlist_busy;
    let candidates = ui.playlist_candidates.read().clone();
    let mut candidate = ui.playlist_candidate;
    let add = ui.clone();
    let move_folder = ui.clone();
    let select_all = ui.clone();
    let mut draft = ui.playlist_name_draft;
    let mut confirm = ui.playlist_delete_confirm;
    let is_owner = detail.as_ref().is_some_and(|detail| {
        ui.account
            .peek()
            .as_ref()
            .is_some_and(|account| account.user.id == detail.playlist.owner_id)
    });
    rsx! {section {class:"native-library","aria-label":"Playlists",
        div {class:"library-intro",div {if detail.is_none() {h1 {"Playlists"} p {"A place for every listening mood."}}}
            button {class:"button secondary",disabled:loading,onclick:move |_|{load_playlists(refresh.clone());},Icon {name:"history"} "Refresh playlists"}
        }
        if loading {div {class:"native-loading",role:"status",span {class:"loading-mark","aria-hidden":"true"} "Loading playlists…"}}
        if !error.is_empty() {div {class:"alert error",role:"alert","{error}"}}
        if let Some(detail) = detail {
            div { class: "library-intro playlist-detail-intro",
                ArtworkCover { ui:ui.clone(), artwork_id:detail.playlist.artwork_id.clone(), class:"neutral-cover playlist-detail-cover", icon:"queue" }
                div {
                    h1 { "data-testid": "native-playlist-name", "{detail.playlist.name}" }
                    p { "{detail.items.len()} tracks" if detail.playlist.can_edit { " · You can edit" } else { " · Shared with you" } }
                }
            }
            div { class: "native-track-actions",
                button { class: "button secondary small", onclick: move |_| { let mut ui = back.clone(); ui.playlist.set(None); ui.playlist_id.set(None); }, "Back to playlists" }
                button { class: "button primary small", disabled: !ui.audio_ready() || detail.items.iter().all(|item| !item.can_stream), onclick: move |_| play_playlist(play.clone()), Icon { name: "play" } "Play playlist" }
            }
            if is_owner {
                div { class: "inline-form",
                    label { class: "sr-only", r#for: "native-playlist-rename", "Playlist name" }
                    input { id: "native-playlist-rename", maxlength: 120, value: "{draft}", disabled: *busy.read(), oninput: move |event| draft.set(event.value()) }
                    button { class: "button secondary small", disabled: *busy.read() || draft.read().trim().is_empty(), onclick: move |_| rename_selected_playlist(rename.clone()), if *busy.read() { "Saving…" } else { "Save name" } }
                    if *confirm.read() {
                        button { class: "button danger small", disabled: *busy.read(), onclick: move |_| delete_selected_playlist(delete.clone()), "Confirm delete" }
                        button { class: "button secondary small", onclick: move |_| confirm.set(false), "Cancel" }
                    } else {
                        button { class: "button danger small", disabled: *busy.read(), onclick: move |_| confirm.set(true), "Delete playlist" }
                    }
                }
                if *confirm.read() { p { class: "muted", role: "status", "Deleting removes this playlist and its share links. Your original tracks and audio stay." } }
                div { class: "inline-form",
                    label { class: "sr-only", r#for: "native-playlist-track", "Library track" }
                    select { id: "native-playlist-track", value: "{candidate}", disabled: *busy.read(), onchange: move |event| candidate.set(event.value()),
                        option { value: "", "Choose a library track" }
                        for track in candidates.iter().cloned() {
                            option { value: "{track.id}", "{track.title} · {track.artist}" }
                        }
                    }
                    button { class: "button secondary small", disabled: *busy.read() || candidate.read().is_empty(), onclick: move |_| add_selected_playlist_track(add.clone()), "Add selected track" }
                }
                div { class: "inline-form",
                    label { class: "sr-only", r#for: "native-playlist-folder", "Playlist folder" }
                    select { id: "native-playlist-folder", value: "{playlist_folder_target_value}", disabled: *busy.read() || folder_busy, onchange: move |event| playlist_folder_target.set(event.value()),
                        option { value: "", "Unfiled" }
                        for folder in folders.iter().cloned() { option { value: "{folder.id}", "{folder.name}" } }
                    }
                    button { class: "button secondary small", disabled: *busy.read() || folder_busy, onclick: move |_| move_playlist_folder(move_folder.clone()), "Move to folder" }
                }
            }
            if detail.items.is_empty() {
                div { class: "native-empty", Icon { name: "queue" } h2 { "This playlist is empty" } p { "Add an owned track above, or manage it from the web app." } }
            } else {
                div { class: "native-queue-list",
                    for (at, entry) in detail.items.iter().cloned().enumerate() {
                        {
                            let play_entry = ui.clone();
                            let played_id=entry.id.clone();
                            let playable_at=detail.items.iter().filter(|i|i.can_stream).position(|i|i.id==entry.id);
                            let active=*ui.wants_play.read() && playable_at.is_some_and(|index|owned_target_current(&ui,&playable_tracks,index,&format!("Playlist · {}",detail.playlist.name)));
                            let verb=if active {"Pause"}else{"Play"};
                            let up = ui.clone();
                            let down = ui.clone();
                            let remove = ui.clone();
                            let item_id = entry.id.clone();
                            const LAST_HINT: &str = "Already the last playlist entry.";
                            rsx! { article { class: "native-queue-row", key: "{entry.id}",
                                if entry.can_stream {button {class:"icon-button", "aria-label":"{verb} playlist entry {at + 1}: {entry.track.title}", onclick:move |_|{
                                    if let Some(detail)=play_entry.playlist.peek().clone() {
                                        let items=detail.items.iter().filter(|i|i.can_stream).collect::<Vec<_>>();
                                        if let Some(at)=items.iter().position(|i|i.id==played_id) {
                                            let tracks=items.into_iter().map(|i|i.track.clone()).collect();
                                            play_owned_context(play_entry.clone(),tracks,at,format!("Playlist · {}",detail.playlist.name));
                                        }
                                    }
                                },if active {Icon {name:"pause"}}else{Icon {name:"play"}}}}
                                div { class: "native-queue-select", ArtworkCover {ui:ui.clone(),artwork_id:entry.track.artwork_id.clone(),class:"neutral-cover",icon:"music"}, div { strong { "{entry.track.title}" } small { "{entry.track.artist}" } } }
                                if entry.can_stream { span { class: "queue-current", "Playable" } } else { small { "Metadata only" } }
                                if detail.playlist.can_edit {
                                    button { class: "icon-button", disabled: at == 0 || *busy.read(), "aria-label": "Move playlist entry {at + 1} up", title: if at == 0 { "Already the first playlist entry." } else { "Move this entry one place earlier." }, onclick: move |_| move_playlist_entry(up.clone(), at, -1), Icon { name: "up" } }
                                    button { class: "icon-button", disabled: at + 1 >= detail.items.len() || *busy.read(), "aria-label": "Move playlist entry {at + 1} down", title: if at + 1 >= detail.items.len() { LAST_HINT } else { "Move this entry one place later." }, onclick: move |_| move_playlist_entry(down.clone(), at, 1), Icon { name: "down" } }
                                    button { class: "icon-button", disabled: *busy.read(), "aria-label": "Remove playlist entry {at + 1}", title: "Remove this entry from the playlist; the original track is unchanged.", onclick: move |_| remove_playlist_entry(remove.clone(), item_id.clone()), Icon { name: "close" } }
                                }
                            } }
                        }
                    }
                }
            }
        } else {
            div { class: "catalog-caption", p { "Playlist folders" } span { "PRIVATE GROUPING" } }
            details { class: "native-create", id: "native-create-folder",
              summary { "New folder" }
              form { class: "inline-form", onsubmit: move |event| { event.prevent_default(); create_folder(folder_create.clone()); },
                label { class: "sr-only", r#for: "native-folder-name", "New folder name" }
                input { id: "native-folder-name", maxlength: 120, placeholder: "New folder name", value: "{folder_name}", disabled: folder_busy, oninput: move |event| folder_name.set(event.value()) }
                button { class: "button primary small", r#type: "submit", disabled: folder_busy || folder_name.read().trim().is_empty(), if folder_busy { "Creating…" } else { "Create folder" } }
              }
            }
            div { class: "inline-form", role: "group", "aria-label": "Folder filters",
                button { class: if folder_filter_value.is_none() { "button secondary small" } else { "button small" }, "aria-pressed": folder_filter_value.is_none(), onclick: move |_| select_folder(select_all.clone(), None), "All playlists" }
                for folder in folders.iter().cloned() {
                    {
                        let filter = folder.id.clone();
                        let select_ui = ui.clone();
                        let delete_ui = ui.clone();
                        let set_confirm_ui = ui.clone();
                        let folder_id = folder.id.clone();
                        let confirm_id = folder.id.clone();
                        let name = folder.name.clone();
                        let selected = folder_filter_value.as_deref() == Some(folder.id.as_str());
                        rsx! { div { class: "inline-form", "data-native-folder": "{folder.id}",
                            button { class: if selected { "button secondary small" } else { "button small" }, "aria-pressed": selected, onclick: move |_| select_folder(select_ui.clone(), Some(filter.clone())), "{name} ({folder.playlist_count})" }
                            if folder_confirm == folder_id {
                                button { class: "button danger small", "aria-label": "Confirm delete folder {folder.name}", onclick: move |_| delete_folder(delete_ui.clone(), confirm_id.clone()), "Confirm folder delete" }
                                button { class: "button secondary small", onclick: move |_| set_confirm_ui.clone().folder_confirm.set(String::new()), "Keep folder" }
                            } else {
                                button { class: "icon-button", "aria-label": "Delete folder {folder.name}", onclick: move |_| set_confirm_ui.clone().folder_confirm.set(folder_id.clone()), Icon { name: "close" } }
                            }
                        } }
                    }
                }
            }
            if let Some(folder) = folders.iter().find(|folder| folder_filter_value.as_deref() == Some(folder.id.as_str())).cloned() {
                form { class: "inline-form", onsubmit: move |event| { event.prevent_default(); rename_folder(folder_rename_ui.clone()); },
                    label { class: "sr-only", r#for: "native-folder-rename", "Folder name" }
                    input { id: "native-folder-rename", maxlength: 120, value: "{folder_rename}", disabled: folder_busy, oninput: move |event| folder_rename.set(event.value()) }
                    button { class: "button secondary small", r#type: "submit", disabled: folder_busy || folder_rename.read().trim().is_empty(), "Save folder name" }
                }
                small { "Deleting this folder unfiles its playlists; their tracks and shares stay." }
                if folder_confirm == folder.id { "" }
            }
            details { class: "native-create", id: "native-create-playlist",
              summary { "New playlist" }
              form { class: "inline-form", onsubmit: move |event| { event.prevent_default(); create_playlist_ui(create.clone()); },
                label { class: "sr-only", r#for: "native-playlist-name", "New playlist name" }
                input { id: "native-playlist-name", maxlength: 120, placeholder: "New playlist name", value: "{draft}", disabled: *busy.read(), oninput: move |event| draft.set(event.value()) }
                button { class: "button primary small", r#type: "submit", disabled: *busy.read() || draft.read().trim().is_empty(), if *busy.read() { "Creating…" } else { "Create playlist" } }
              }
            }
            if !loading && lists.is_empty() && error.is_empty() {
                div { class: "native-empty", Icon { name: "queue" } h2 { "Your next mix starts here" } p { "Choose New playlist to bring your tracks together." } }
            }
            div { class: "native-queue-list",
                for list in filtered_lists {
                    {
                        let open = ui.clone();
                        let list_id = list.id.clone();
                        rsx! { article { class: "native-queue-row", key: "{list.id}",
                            button { class: "native-queue-select", "aria-label": "Open playlist {list.name}", onclick: move |_| open_playlist(open.clone(), list_id.clone()),
                                ArtworkCover { ui:ui.clone(), artwork_id:list.artwork_id.clone(), class:"neutral-cover", icon:"queue" }
                                div { strong { "{list.name}" } small { "{list.item_count} tracks" if !list.can_edit { " · shared" } } }
                            }
                        } }
                    }
                }
            }
        }
    } }
}

fn explore_public(mut ui: Ui) {
    let origin = ui.remote_origin.peek().trim().to_string();
    if origin.is_empty() {
        ui.remote_error.set("Enter a public server origin.".into());
        return;
    }
    let Some(account) = ui.account.peek().clone().filter(|_| ui.signed_in()) else {
        return;
    };
    ui.remote_gate.cancel();
    ui.view.set("public");
    ui.remote_loading.set(true);
    ui.remote_error.set(String::new());
    ui.remote_preview.set(None);
    let ticket = ui.remote_gate.begin(account.epoch);
    spawn(async move {
        let result = crate::remote::preview(&origin, ticket.cancel.clone()).await;
        if !ticket.current(&ui) || *ui.view.peek() != "public" {
            return;
        }
        ui.remote_loading.set(false);
        match result {
            Ok(preview) => {
                ui.remote_error.set(String::new());
                ui.remote_preview.set(Some(preview));
            }
            Err(error) => {
                if !expired(ui.clone(), &error) {
                    ui.remote_error.set(error);
                }
            }
        }
    });
}

fn play_public(mut ui: Ui, track: PublicTrack, origin: String) {
    if !track.can_stream || !ui.signed_in() {
        return;
    }
    if ui
        .remote_now
        .peek()
        .as_ref()
        .is_some_and(|now| now.origin == origin && now.id == track.id)
    {
        toggle(ui);
        return;
    }
    ui.remote_queue
        .set(vec![crate::remote::RemoteItem { origin, track }]);
    ui.remote_index.set(0);
    start_remote(ui);
}

fn play_public_result(mut ui: Ui, origin: String, track_id: String) {
    if ui
        .remote_now
        .peek()
        .as_ref()
        .is_some_and(|now| now.origin == origin && now.id == track_id)
    {
        toggle(ui);
        return;
    }
    let mut items = Vec::new();
    for peer in ui.remote_results.peek().clone() {
        for track in peer.items {
            if track.can_stream {
                items.push(crate::remote::RemoteItem {
                    origin: peer.origin.clone(),
                    track,
                });
            }
        }
    }
    let Some(index) = items
        .iter()
        .position(|item| item.origin == origin && item.track.id == track_id)
    else {
        return;
    };
    ui.remote_queue.set(items);
    ui.remote_index.set(index);
    start_remote(ui);
}

fn start_remote(mut ui: Ui) {
    if !ui.signed_in() {
        return;
    }
    ui.offline_active.set(false);
    let Some(item) = ui.remote_queue.peek().get(*ui.remote_index.peek()).cloned() else {
        return;
    };
    if !item.track.can_stream {
        return;
    }
    let Some(account) = ui.account.peek().clone() else {
        return;
    };
    let Some(audio) = ui.audio.peek().clone() else {
        return;
    };
    let origin = item.origin.clone();
    let track = item.track.clone();
    let selection = ui.new_selection();
    // Switching to a public item must not leave a local occurrence current:
    // an in-flight local EOF/advance must not replace the public selection.
    ui.order.write().reset(0, 0);
    let _ = audio.stop(selection);
    ui.prepared.set(None);
    ui.remote_now.set(Some(crate::remote::RemoteNow {
        id: track.id.clone(),
        title: track.title.clone(),
        artist: track.artist.clone(),
        origin: origin.clone(),
    }));
    ui.context.set(format!("Public · {origin}"));
    ui.wants_play.set(true);
    ui.preparing.set(true);
    let ticket = ui.source_gate.begin(account.epoch);
    spawn(async move {
        let result = crate::remote::prepare(&origin, &track, ticket.cancel.clone()).await;
        if !ticket.current(&ui)
            || *ui.selection.peek() != selection
            || ui
                .remote_now
                .peek()
                .as_ref()
                .map(|remote| remote.id.as_str())
                != Some(track.id.as_str())
            || ui
                .remote_queue
                .peek()
                .get(*ui.remote_index.peek())
                .map(|item| item.track.id.as_str())
                != Some(track.id.as_str())
        {
            return;
        }
        ui.preparing.set(false);
        let requested = *ui.wants_play.peek();
        match result {
            Ok(prepared) => match audio.load(selection, prepared.clone(), requested, 0) {
                Ok(()) => ui.prepared.set(Some(prepared)),
                Err(error) => {
                    ui.wants_play.set(false);
                    ui.source_error.set(error);
                }
            },
            Err(error) => {
                if !expired(ui.clone(), &error) {
                    ui.wants_play.set(false);
                    ui.source_error.set(error);
                }
            }
        }
    });
}

fn remote_step(mut ui: Ui, delta: i32) {
    if ui.remote_now.peek().is_none() {
        return;
    }
    let Some(next) =
        crate::remote::queue_target(*ui.remote_index.peek(), ui.remote_queue.peek().len(), delta)
    else {
        return;
    };
    ui.remote_index.set(next);
    start_remote(ui);
}

fn player_previous(ui: Ui) {
    if *ui.offline_active.peek() {
        offline_step(ui, -1);
        return;
    }
    if ui.remote_now.peek().is_some() {
        remote_step(ui, -1);
    } else {
        advance(ui, Command::Previous);
    }
}

fn player_next(ui: Ui) {
    if *ui.offline_active.peek() {
        offline_step(ui, 1);
        return;
    }
    if ui.remote_now.peek().is_some() {
        remote_step(ui, 1);
    } else {
        advance(ui, Command::Next);
    }
}

fn load_more_public(mut ui: Ui) {
    let Some(preview) = ui.remote_preview.peek().clone() else {
        return;
    };
    let Some(cursor) = preview.next_cursor.clone() else {
        return;
    };
    if *ui.remote_more_busy.peek() || !ui.signed_in() || *ui.view.peek() != "public" {
        return;
    }
    let Some(account) = ui.account.peek().clone() else {
        return;
    };
    let origin = preview.origin;
    let ticket = ui.remote_gate.begin(account.epoch);
    ui.remote_more_busy.set(true);
    ui.remote_more_error.set(String::new());
    spawn(async move {
        let result = crate::remote::more(&origin, &cursor, ticket.cancel.clone()).await;
        if !ticket.current(&ui) || *ui.view.peek() != "public" {
            return;
        }
        let current = ui.remote_preview.peek().clone();
        if current.as_ref().is_none_or(|preview| {
            preview.origin != origin || preview.next_cursor.as_deref() != Some(cursor.as_str())
        }) {
            return;
        }
        ui.remote_more_busy.set(false);
        match result {
            Ok(page) if page.origin == origin => {
                let mut current = current.expect("preview checked above");
                let mut seen = current
                    .items
                    .iter()
                    .map(|track| track.id.clone())
                    .collect::<std::collections::HashSet<_>>();
                for track in page.items {
                    if seen.insert(track.id.clone()) {
                        current.items.push(track);
                    }
                }
                current.next_cursor = page.next_cursor;
                ui.remote_preview.set(Some(current));
            }
            Ok(_) => ui
                .remote_more_error
                .set("The public server changed origin; the page was ignored.".into()),
            Err(error) => {
                if !expired(ui.clone(), &error) {
                    ui.remote_more_error.set(error);
                }
            }
        }
    });
}

fn search_saved_peers(mut ui: Ui) {
    let query = ui.remote_query.peek().trim().to_string();
    if query.is_empty() {
        ui.remote_search_error.set("Enter a public search.".into());
        return;
    }
    let Some(account) = ui.account.peek().clone().filter(|_| ui.signed_in()) else {
        return;
    };
    let origins = ui.saved_peers.peek().clone();
    if origins.is_empty() {
        ui.remote_search_error
            .set("Save at least one public server first.".into());
        return;
    }
    ui.remote_gate.cancel();
    ui.remote_searching.set(true);
    ui.remote_search_error.set(String::new());
    ui.remote_results.set(Vec::new());
    let ticket = ui.remote_gate.begin(account.epoch);
    spawn(async move {
        let result = crate::remote::search_peers(&origins, &query, ticket.cancel.clone()).await;
        if !ticket.current(&ui) || *ui.view.peek() != "public" {
            return;
        }
        ui.remote_searching.set(false);
        match result {
            Ok(results) => ui.remote_results.set(results),
            Err(error) => {
                if !expired(ui.clone(), &error) {
                    ui.remote_search_error.set(error);
                }
            }
        }
    });
}

fn load_more_peer(mut ui: Ui, origin: String) {
    if !ui.remote_peer_more_busy.peek().is_empty() || *ui.view.peek() != "public" {
        return;
    }
    let Some(peer) = ui
        .remote_results
        .peek()
        .iter()
        .find(|peer| peer.origin == origin)
        .cloned()
    else {
        return;
    };
    let Some(cursor) = peer.next_cursor.clone() else {
        return;
    };
    let Some(account) = ui.account.peek().clone().filter(|_| ui.signed_in()) else {
        return;
    };
    let ticket = ui.remote_gate.begin(account.epoch);
    ui.remote_peer_more_busy.set(origin.clone());
    ui.remote_peer_more_error.set(String::new());
    spawn(async move {
        let result = crate::remote::more(&origin, &cursor, ticket.cancel.clone()).await;
        if !ticket.current(&ui) || *ui.view.peek() != "public" {
            return;
        }
        let mut results = ui.remote_results.peek().clone();
        let Some(index) = results.iter().position(|peer| {
            peer.origin == origin && peer.next_cursor.as_deref() == Some(cursor.as_str())
        }) else {
            return;
        };
        ui.remote_peer_more_busy.set(String::new());
        match result {
            Ok(page) if page.origin == origin => {
                let mut seen = results[index]
                    .items
                    .iter()
                    .map(|track| track.id.clone())
                    .collect::<std::collections::HashSet<_>>();
                for track in page.items {
                    if seen.insert(track.id.clone()) {
                        results[index].items.push(track);
                    }
                }
                results[index].next_cursor = page.next_cursor;
                ui.remote_results.set(results);
            }
            Ok(_) => ui
                .remote_peer_more_error
                .set("The public server changed origin; the page was ignored.".into()),
            Err(error) => {
                if !expired(ui.clone(), &error) {
                    ui.remote_peer_more_error.set(error);
                }
            }
        }
    });
}

fn retry_peer(mut ui: Ui, origin: String) {
    if !ui.remote_retry_busy.peek().is_empty() || *ui.view.peek() != "public" {
        return;
    }
    let query = ui.remote_query.peek().trim().to_string();
    if query.is_empty() {
        return;
    }
    let Some(account) = ui.account.peek().clone().filter(|_| ui.signed_in()) else {
        return;
    };
    let ticket = ui.remote_gate.begin(account.epoch);
    ui.remote_retry_busy.set(origin.clone());
    ui.remote_retry_error.set(String::new());
    spawn(async move {
        let result = crate::remote::search_peers(
            std::slice::from_ref(&origin),
            &query,
            ticket.cancel.clone(),
        )
        .await;
        if !ticket.current(&ui)
            || *ui.view.peek() != "public"
            || ui.remote_query.peek().trim() != query
        {
            return;
        }
        let mut results = ui.remote_results.peek().clone();
        let Some(index) = results.iter().position(|peer| peer.origin == origin) else {
            return;
        };
        ui.remote_retry_busy.set(String::new());
        match result {
            Ok(peers) if peers.len() == 1 && peers[0].origin == origin => {
                results[index] = peers[0].clone();
                ui.remote_results.set(results);
            }
            Ok(_) => ui
                .remote_retry_error
                .set("The public server returned an unexpected result.".into()),
            Err(error) => {
                if !expired(ui.clone(), &error) {
                    ui.remote_retry_error.set(error);
                }
            }
        }
    });
}

fn save_public_peer(mut ui: Ui) {
    let Some(preview) = ui.remote_preview.peek().clone() else {
        return;
    };
    let mut peers = ui.saved_peers.peek().clone();
    if !peers.contains(&preview.origin) {
        peers.push(preview.origin);
        peers.sort();
    }
    match crate::peers::save(&peers) {
        Ok(()) => {
            ui.saved_peers.set(peers);
            ui.remote_error.set(String::new());
            ui.notice.set("Public server saved.".into());
        }
        Err(error) => ui.remote_error.set(error),
    }
}

fn remove_public_peer(mut ui: Ui, origin: String) {
    let mut peers = ui.saved_peers.peek().clone();
    peers.retain(|peer| peer != &origin);
    match crate::peers::save(&peers) {
        Ok(()) => {
            ui.saved_peers.set(peers);
            ui.notice.set("Public server removed.".into());
        }
        Err(error) => ui.remote_error.set(error),
    }
}

fn open_saved_peer(mut ui: Ui, origin: String) {
    ui.remote_origin.set(origin);
    explore_public(ui);
}

fn public_view(ui: Ui) -> Element {
    let loading = *ui.remote_loading.read();
    let error = ui.remote_error.read().clone();
    let preview = ui.remote_preview.read().clone();
    let saved = ui.saved_peers.read().clone();
    let searching = *ui.remote_searching.read();
    let search_error = ui.remote_search_error.read().clone();
    let results = ui.remote_results.read().clone();
    let explore = ui.clone();
    let save_current = ui.clone();
    let search = ui.clone();
    let more = ui.clone();
    let more_busy = *ui.remote_more_busy.read();
    let more_error = ui.remote_more_error.read().clone();
    let peer_more_busy = ui.remote_peer_more_busy.read().clone();
    let peer_more_error = ui.remote_peer_more_error.read().clone();
    let retry_busy = ui.remote_retry_busy.read().clone();
    let retry_error = ui.remote_retry_error.read().clone();
    let mut query = ui.remote_query;
    let mut origin = ui.remote_origin;
    rsx! {section {class:"native-library","aria-label":"Public catalogs",
        div {class:"library-intro",div {span {class:"eyebrow","OPT-IN PUBLIC NETWORK"} h1 {"Public catalogs"} p {"Fetch one explicitly published catalog over its advertised origin. Private libraries, history and credentials are never sent."}}}
        form {class:"native-search",onsubmit:move |event|{event.prevent_default();explore_public(explore.clone());},
            Icon {name:"network"} label {class:"sr-only",r#for:"native-peer-origin","Public server origin"}
            input {id:"native-peer-origin",r#type:"url",maxlength:2048,value:"{origin}",placeholder:"https://music.example.org",oninput:move |event|origin.set(event.value())}
            button {class:"button primary",r#type:"submit",disabled:loading,if loading {"Exploring…"} else {"Explore public catalog"}}
        }
        if !saved.is_empty() {
            form { class: "native-search", onsubmit: move |event| { event.prevent_default(); search_saved_peers(search.clone()); },
                Icon { name: "search" } label { class: "sr-only", r#for: "native-peer-query", "Search saved public servers" }
                input { id: "native-peer-query", maxlength: 128, value: "{query}", placeholder: "Search all saved public servers", oninput: move |event| query.set(event.value()) }
                button { class: "button primary", r#type: "submit", disabled: searching, if searching { "Searching…" } else { "Search saved servers" } }
            }
            if !search_error.is_empty() { div { class: "alert error", role: "alert", "{search_error}" } }
            if !results.is_empty() {
                div { class: "catalog-caption", p { "Unified saved-server results" } span { "{results.len()} SERVERS" } }
                for peer in results.iter().cloned() {
                    {
                        let peer_origin = peer.origin.clone();
                        let peer_label = peer.origin.clone();
                        let peer_error = peer.error.clone();
                        let peer_error_text = peer.error.clone().unwrap_or_default();
                        let peer_cached = peer.cached;
                        let peer_has_more = peer.next_cursor.is_some();
                        let peer_more_origin = peer.origin.clone();
                        let peer_cached_age = peer.cached_at.map(cached_age_label).unwrap_or_default();
                        let retry_origin = peer.origin.clone();
                        let retry_ui = ui.clone();
                        let more_ui = ui.clone();
                        rsx! {
                            article { class: "native-queue-row", key: "search-{peer.origin}", "data-public-origin": "{peer.origin}", "data-public-peer-summary": "true", "data-public-peer-error": "{peer_error_text}", "data-public-peer-cached": "{peer.cached}",
                                div { class: "neutral-cover", "aria-hidden": "true", Icon { name: "network" } }
                                div { class: "native-queue-select",
                                    div { strong { "Public origin" } small { "{peer_label}" } }
                                }
                                if peer_cached { small { "Cached metadata · {peer_cached_age} · stream unverified" } button { class: "button secondary small", "aria-label": "Retry live for {peer.origin}", disabled: retry_busy == peer.origin, onclick: move |_| retry_peer(retry_ui.clone(), retry_origin.clone()), if retry_busy == retry_origin { "Retrying…" } else { "Retry live" } } } else if peer_error.is_some() { small { "Unavailable" } } else { span { class: "queue-current", "{peer.items.len()} results" } }
                            }
                            if let Some(error) = peer_error { p { class: "muted", "{error}" } }
                            div { class: "native-queue-list",
                                for track in peer.items.iter().cloned() {
                                    {
                                        let play_ui = ui.clone();
                                        let play_track = track.clone();
                                        let play_origin = peer_origin.clone();
                                        let active=ui.remote_now.read().as_ref().is_some_and(|n|n.id==track.id && n.origin==play_origin) && *ui.wants_play.read();
                                        let verb=if active {"Pause"}else{"Play"};
                                        let track_title = track.title.clone();
                                        rsx! { article { class: "native-queue-row", key: "{peer.origin}-{track.id}", "data-public-track-id": "{track.id}", "data-public-origin": "{peer.origin}", "data-public-cached": "{peer_cached}",
                                            div { class: "neutral-cover", "aria-hidden": "true", Icon { name: "music" } }
                                            div { class: "native-queue-select", div { strong { "{track.title}" } small { "{track.artist} · {track.album}" } small { "{peer.origin}" } } }
                                            span { class: "native-track-time", "{time_label(track.duration_ms.max(0) as u64)}" }
                                            if track.can_stream && !peer_cached {
                                                button { class: "button primary small", disabled: !ui.audio_ready(), "aria-label": "{verb} public track {track_title}", onclick: move |_| play_public_result(play_ui.clone(), play_origin.clone(), play_track.id.clone()), if active {Icon {name:"pause"}}else{Icon {name:"play"}} "{verb} public track" }
                                            } else if peer_cached { small { "Cached metadata only" } } else { small { "Metadata only" } }
                                        } }
                                    }
                                }
                            }
                            if peer_has_more {
                                button { class: "button secondary small", "aria-label": "Load more from {peer_more_origin}", disabled: peer_more_busy == peer_more_origin, onclick: move |_| load_more_peer(more_ui.clone(), peer_more_origin.clone()), if peer_more_busy == peer_more_origin { "Loading more…" } else { "Load more from this server" } }
                            }
                        }
                    }
                }
            }
            if !peer_more_error.is_empty() { p { class: "alert error", role: "alert", "{peer_more_error}" } }
            if !retry_error.is_empty() { p { class: "alert error", role: "alert", "{retry_error}" } }
            div { class: "catalog-caption", p { "Saved public servers" } span { "NO SECRETS" } }
            div { class: "native-queue-list",
                for peer in saved.iter().cloned() {
                    {
                        let open = ui.clone();
                        let remove = ui.clone();
                        let peer_label = peer.clone();
                        let peer_open = peer_label.clone();
                        rsx! { article { class: "native-queue-row", key: "{peer}",
                            button { class: "native-queue-select", "aria-label": "Open saved server {peer_label}", onclick: move |_| open_saved_peer(open.clone(), peer_open.clone()),
                                div { class: "neutral-cover", "aria-hidden": "true", Icon { name: "network" } }
                                div { strong { "Saved public server" } small { "{peer_label}" } }
                            }
                            button { class: "icon-button", "aria-label": "Remove saved server {peer_label}", title: "Remove this saved origin; no server data is deleted.", onclick: move |_| remove_public_peer(remove.clone(), peer_label.clone()), Icon { name: "close" } }
                        } }
                    }
                }
            }
        }
        if loading {div {class:"native-loading",role:"status",span {class:"loading-mark","aria-hidden":"true"} "Reading the public catalog…"}}
        if !error.is_empty() {div {class:"alert error",role:"alert","{error}"}}
        if let Some(preview) = preview {
            div {class:"catalog-caption",p {"{preview.items.len()} explicit public records · origin {preview.origin}"} if !saved.contains(&preview.origin) { button { class: "button secondary small", onclick: move |_| save_public_peer(save_current.clone()), "Save this server" } } else { span { "SAVED" } }}
            if preview.items.is_empty() {
                div { class: "native-empty", Icon { name: "network" } h2 { "This public catalog is empty" } p { "The server exposes metadata only when its owner explicitly publishes a licensed track." } }
            } else {
                div { class: "native-queue-list",
                    for track in preview.items.iter().cloned() {
                        {
                            let play_ui = ui.clone();
                            let play_track = track.clone();
                            let track_title = track.title.clone();
                            let origin_label = preview.origin.clone();
                            let play_origin = preview.origin.clone();
                            let active=ui.remote_now.read().as_ref().is_some_and(|n|n.id==track.id && n.origin==play_origin) && *ui.wants_play.read();
                            let verb=if active {"Pause"}else{"Play"};
                            rsx! { article { class: "native-queue-row", key: "{track.id}", "data-public-track-id": "{track.id}",
                                div { class: "neutral-cover", "aria-hidden": "true", Icon { name: "music" } }
                                div { class: "native-queue-select",
                                    div {
                                        strong { "{track.title}" }
                                        small { "{track.artist} · {track.album}" }
                                        small { "Public origin · {origin_label}" }
                                    }
                                }
                                span { class: "native-track-time", "{time_label(track.duration_ms.max(0) as u64)}" }
                                if track.can_stream {
                                    button { class: "button primary small", disabled: !ui.audio_ready(), "aria-label": "{verb} public track {track_title}", onclick: move |_| play_public(play_ui.clone(), play_track.clone(), play_origin.clone()), if active {Icon {name:"pause"}}else{Icon {name:"play"}} "{verb} public track" }
                                } else {
                                    small { "Metadata only" }
                                }
                            } }
                        }
                    }
                }
            }
            if preview.next_cursor.is_some() {
                div { class: "inline-form",
                    button { class: "button secondary", disabled: more_busy, onclick: move |_| load_more_public(more.clone()), if more_busy { "Loading more…" } else { "Load more public records" } }
                }
                if !more_error.is_empty() { p { class: "alert error", role: "alert", "{more_error}" } }
            }
        }
    } }
}

fn browse_view(ui: Ui) -> Element {
    let kind = *ui.browse_kind.read();
    let albums = ui.albums.read().clone();
    let artists = ui.artists.read().clone();
    let tracks = ui.browse_tracks.read().clone();
    let title = ui.browse_title.read().clone();
    let selected = ui.browse_id.read().is_some();
    let loading = *ui.browse_loading.read();
    let error = ui.browse_error.read().clone();
    let refresh = ui.clone();
    let back = ui.clone();
    let play = ui.clone();
    let group_label = if kind == "albums" { "album" } else { "artist" };
    let heading = if kind == "albums" {
        "Albums in your library"
    } else {
        "Artists in your library"
    };
    let refresh_label = format!("Refresh {group_label}s");
    rsx! {section {class:"native-library","aria-label":if kind=="albums" {"Albums"} else {"Artists"},
        div {class:"library-intro",div {span {class:"eyebrow","BROWSE YOUR OWN CATALOG"} h1 { "data-testid": "native-group-title", if selected {"{title}"} else {"{heading}"} } p {"Grouping is derived from the private catalog; playback still requires the owner-authorized original."}}
            button {class:"button secondary",disabled:loading,onclick:move |_|{let kind=*refresh.browse_kind.peek();load_groups(refresh.clone(),kind,None);},Icon {name:"history"} "{refresh_label}"}
        }
        if loading {div {class:"native-loading",role:"status",span {class:"loading-mark","aria-hidden":"true"} "Loading {group_label}s…"}}
        if !error.is_empty() {div {class:"alert error",role:"alert","{error}"}}
        if selected {
            div {class:"native-track-actions",button {class:"button secondary small",onclick:move |_|{let mut ui=back.clone();ui.browse_id.set(None);ui.browse_title.set(String::new());ui.browse_tracks.set(Vec::new());},"Back to {group_label}s"} button {class:"button primary small",disabled:!ui.audio_ready()||tracks.is_empty(),onclick:move |_|play_group(play.clone()),Icon {name:"play"} "Play {group_label}"}}
            if !tracks.is_empty() {div {class:"native-track-table","data-native-detail-table":"1",
                div {class:"native-track-header","aria-hidden":"true",span {""} span {"TITLE / ARTIST"} span {"ALBUM"} span {"TIME"} span {""}}
                for (at, track) in tracks.iter().cloned().enumerate() {{let play=ui.clone();let active=ui.current_track().is_some_and(|t|t.id==track.id) && *ui.wants_play.read();let current=ui.current_track().is_some_and(|t|t.id==track.id);let verb=if active {"Pause"}else{"Play"}; rsx!{article {class:if current {"native-track current"} else {"native-track"},key:"{at}-{track.id}","data-native-detail-track-id":"{track.id}",
                button {class:"icon-button track-start", "aria-label":"{verb} {track.title} from {group_label}",onclick:move |_|{let tracks=play.browse_tracks.peek().clone();let title=play.browse_title.peek().clone();play_owned_context(play.clone(),tracks,at,title);},if active{Icon {name:"pause"}}else{Icon {name:"play"}}}
                div {class:"native-track-name",ArtworkCover {ui:ui.clone(),artwork_id:track.artwork_id.clone(),class:"neutral-cover",icon:"music"} div {strong {"{track.title}"} small {"{track.artist}"}}}
                div {class:"native-track-album","{track.album}"}
                div {class:"native-track-time","{time_label(track.duration_ms.max(0) as u64)}"}
                div {class:"native-track-actions",small {"{at+1}"}}
            } } }} } }
        } else {
            if kind == "albums" {
                if albums.is_empty() && !loading && error.is_empty() {div {class:"native-empty",Icon {name:"albums"} h2 {"No album groups yet"} p {"Uploaded or scanned tracks with album metadata will appear here."}}}
                div {class:"native-queue-list",for album in albums { { let open=ui.clone(); let id=album.id.clone(); rsx! { article {class:"native-queue-row",key:"{album.id}",
                    button {class:"native-queue-select","aria-label":"Open album {album.title} by {album.artist}",onclick:move |_|open_group(open.clone(),"albums",id.clone()),ArtworkCover {ui:ui.clone(),artwork_id:album.artwork_id.clone(),class:"neutral-cover",icon:"albums"} div {strong {"{album.title}"} small {"{album.artist} · {album.track_count} tracks"} }}
                } } } } }
            } else {
                if artists.is_empty() && !loading && error.is_empty() {div {class:"native-empty",Icon {name:"artists"} h2 {"No artist groups yet"} p {"Uploaded or scanned tracks with artist metadata will appear here."}}}
                div {class:"native-queue-list",for artist in artists { { let open=ui.clone(); let id=artist.id.clone(); rsx! { article {class:"native-queue-row",key:"{artist.id}",
                    button {class:"native-queue-select","aria-label":"Open artist {artist.name}",onclick:move |_|open_group(open.clone(),"artists",id.clone()),ArtworkCover {ui:ui.clone(),artwork_id:artist.artwork_id.clone(),class:"neutral-cover",icon:"artists"} div {strong {"{artist.name}"} small {"{artist.track_count} tracks · {artist.album_count} albums"} }}
                } } } } }
            }
        }
    } }
}

fn queue_view(ui: Ui) -> Element {
    let queue = ui.queue.read().clone();
    let queue_empty = queue.is_empty();
    let queue_len = queue.len();
    let current = ui.order.read().current();
    let projection = ui.order.read().projection();
    let explicit_next = projection.explicit_next.first().copied();
    let next_at = projection.scheduled_next;
    let next_track = next_at.and_then(|at| queue.get(at).cloned());
    let shuffled = *ui.shuffle.read() && !*ui.offline_active.read();
    let next_label = if next_at.is_some() && next_at == explicit_next {
        "Play next request"
    } else if shuffled && next_at.is_some_and(|at| !projection.pending.contains(&at)) {
        "Forward shuffle history"
    } else if shuffled {
        "Remaining shuffle track"
    } else {
        "Saved queue order"
    };
    let clear = ui.clone();
    let clear_upcoming = ui.clone();
    let has_upcoming = current.is_some_and(|current| current + 1 < queue_len);
    rsx! {section {class:"native-queue","aria-label":"Playback queue",
        div {class:"library-intro",div {span {class:"eyebrow","ROOM FOR YOUR NEXT LISTEN"} h1 {"Your listening queue"} p {"{queue_len} of 1,000 places · Duplicate tracks keep their own place."}}
            div {class:"inline-form",button {class:"button secondary",disabled:!has_upcoming,title:if has_upcoming {"Remove only the queued occurrences after the current one."} else {"Nothing is queued after the current occurrence."},onclick:move |_|queue_clear_upcoming(clear_upcoming.clone()),"Clear upcoming"} button {class:"button secondary",disabled:queue_empty,onclick:move |_|clear_queue(clear.clone()),"Clear queue"}}
        }
        if let Some(track) = next_track {div {class:"native-next-up","data-testid":"native-next-up",div {small {"MANUAL NEXT · {next_label}"} strong {"{track.title}"} small {"{track.artist}"}} p {"Repeat one can replay the current track at its end."}}}
        else if shuffled && current.is_some() && !projection.pending.is_empty() {p {class:"queue-explanation","data-testid":"native-next-unknown","Shuffle chooses on advance."}}
        if queue_empty {
            div {class:"native-empty",Icon {name:"queue"} h2 {"A little room for what comes next"} p {"Play a library result, use Play next, or add tracks to your queue."}}
        } else {
            div {class:"native-queue-list",for (at,track) in queue.into_iter().enumerate() { {
                let select=ui.clone();let up=ui.clone();let down=ui.clone();let remove=ui.clone();
                rsx! {article {class:if current==Some(at){"native-queue-row current"}else{"native-queue-row"},key:"{at}-{track.id}","data-native-queue-index":"{at}",
                    button {class:"native-queue-select",disabled:!ui.audio_ready(),"aria-label":format!("{} queue occurrence {at}: {}",if current==Some(at) && *ui.wants_play.read(){"Pause"}else{"Play"},track.title),onclick:move |_|select_queue(select.clone(),at),span {class:"queue-number","{at+1}"} ArtworkCover {ui:ui.clone(),artwork_id:track.artwork_id.clone(),class:"neutral-cover",icon:"music"} div {strong {"{track.title}"} small {"{track.artist}"}} if current==Some(at){span {class:"queue-current","Current"}}}
                    button {class:"icon-button",disabled:at==0,"aria-label":"Move queue occurrence {at} up",title:"Move up",onclick:move |_|queue_edit(up.clone(),at,false),Icon {name:"up"}}
                    button {class:"icon-button",disabled:at+1>=queue_len,"aria-label":"Move queue occurrence {at} down",title:if at+1>=queue_len {"Already the last queued occurrence."} else {"Move down"},onclick:move |_|queue_move_down(down.clone(),at),Icon {name:"down"}}
                    button {class:"icon-button",disabled:current==Some(at),"aria-label":"Remove queue occurrence {at}",title:if current==Some(at){"Current occurrence cannot be removed; clear the queue to stop it"}else{"Remove from queue"},onclick:move |_|queue_edit(remove.clone(),at,true),Icon {name:"close"}}
                } }
            } } }
        }
        p {class:"queue-explanation",if *ui.offline_active.read() {"Saved tracks play in order. You can repeat one track."} else {"Shuffle follows a temporary fair deck and playback history. The visible list is saved queue order, not a prediction of shuffled playback."}}
    } }
}

/// Settings → Appearance for this device only.
///
/// Only settings this build actually implements are exposed: app size (applied
/// exactly once on the root `.native-app` wrapper) and theme. There is no server or
/// account setting in this slice and no disabled switch standing in for an absent
/// engine feature. Every control is a native focusable button/range/radio with an
/// accessible name, and the percentage is announced through `aria-valuetext` rather
/// than a live region that would announce on every drag step.
fn cache_size_label(bytes: u64) -> String {
    if bytes >= 1024 * 1024 {
        format!("{:.1} MiB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{} KiB", bytes / 1024)
    }
}

fn refresh_cache_usage(mut ui: Ui) {
    let Some(account) = ui.account.peek().clone() else {
        return;
    };
    let epoch = account.epoch;
    ui.cache_busy.set(true);
    spawn(async move {
        let result = tokio::task::spawn_blocking({
            let origin = account.origin.clone();
            let user = account.user.id.clone();
            move || crate::media_cache::usage(&origin, &user)
        })
        .await;
        if !ui.alive.get() || *ui.epoch.peek() != epoch || !ui.signed_in() {
            return;
        }
        ui.cache_busy.set(false);
        match result {
            Ok(Ok((count, bytes))) => ui.cache_usage.set(Some((count, bytes))),
            Ok(Err(error)) => {
                ui.cache_usage.set(None);
                ui.notice.set(error);
            }
            Err(_) => ui
                .notice
                .set("The playback cache could not be measured.".into()),
        }
    });
}

fn clear_media_cache(mut ui: Ui) {
    let Some(account) = ui.account.peek().clone() else {
        return;
    };
    let epoch = account.epoch;
    if *ui.cache_busy.peek() {
        return;
    }
    ui.cache_busy.set(true);
    ui.cache_confirm.set(false);
    spawn(async move {
        let result = tokio::task::spawn_blocking({
            let origin = account.origin.clone();
            let user = account.user.id.clone();
            move || crate::media_cache::clear(&origin, &user)
        })
        .await;
        if !ui.alive.get() || *ui.epoch.peek() != epoch || !ui.signed_in() {
            return;
        }
        ui.cache_busy.set(false);
        match result {
            Ok(Ok(())) => {
                ui.cache_usage.set(Some((0, 0)));
                ui.notice
                    .set("Playback cache cleared. Saved offline copies were not touched.".into());
            }
            Ok(Err(error)) => ui.notice.set(error),
            Err(_) => ui
                .notice
                .set("The playback cache could not be cleared.".into()),
        }
    });
}

fn settings_view(ui: Ui) -> Element {
    use crate::presentation::{SCALE_DEFAULT, SCALE_MAX, SCALE_MIN, SCALE_STEP};
    let preferences = *ui.device.read();
    let percent = preferences.app_scale_percent;
    let theme = preferences.theme;
    let save = *ui.device_save.read();
    let repair = *ui.device_repair.read();
    let store = ui.device_store.as_str().to_owned();
    let at_min = percent <= SCALE_MIN;
    let at_max = percent >= SCALE_MAX;
    let at_default = percent == SCALE_DEFAULT;
    let usage = *ui.cache_usage.read();
    let cache_busy = *ui.cache_busy.read();
    let cache_confirm = *ui.cache_confirm.read();
    let effect_ui = ui.clone();
    use_effect(move || refresh_cache_usage(effect_ui.clone()));
    let refresh_cache = ui.clone();
    let clear_cache = ui.clone();
    let confirm_cache = ui.clone();
    let cancel_cache = ui.clone();
    let down = ui.clone();
    let up = ui.clone();
    let preview = ui.clone();
    let persist = ui.clone();
    let reset = ui.clone();
    let save_role = if save == SaveStatus::SessionOnly {
        "alert"
    } else {
        "status"
    };
    let save_message = save.message().unwrap_or_default();
    rsx! {section {class:"native-library native-settings","aria-label":"Settings",
        div {class:"library-intro",div {span {class:"eyebrow","THIS DEVICE"} h1 {"Settings"} p {"Appearance belongs to this computer. It is stored in one private file here, never sent to your server, and it is applied before you sign in."}}}
        section {class:"settings-panel","aria-labelledby":"appearance-title","data-testid":"native-settings-appearance",
            h2 {id:"appearance-title","Appearance"}
            if let Some(reason)=repair {
                p {class:"settings-note warning",role:"status","data-testid":"native-appearance-recovery","{reason.message()}"}
            }
            div {class:"setting-row",
                div {class:"setting-text",
                    h3 {id:"app-size-title","App size"}
                    p {id:"app-size-help","Changes this device only. Vela enlarges its own layout without reloading playback, and your desktop's monitor scaling is left alone."}
                }
                div {class:"setting-value",
                    button {class:"icon-button app-scale-step","aria-label":"Decrease app size",title:"Decrease app size",disabled:at_min,onclick:move |_|step_app_scale(down.clone(),-i32::from(SCALE_STEP)),"−"}
                    span {class:"app-scale-value","data-testid":"native-app-scale-value","{percent}%"}
                    button {class:"icon-button app-scale-step","aria-label":"Increase app size",title:"Increase app size",disabled:at_max,onclick:move |_|step_app_scale(up.clone(),i32::from(SCALE_STEP)),"+"}
                }
            }
            div {class:"app-scale-control",
                label {r#for:"native-app-scale","App size slider"}
                input {id:"native-app-scale",r#type:"range",min:"{SCALE_MIN}",max:"{SCALE_MAX}",step:"{SCALE_STEP}",value:"{percent}",
                    "aria-label":"App size","aria-valuemin":"{SCALE_MIN}","aria-valuemax":"{SCALE_MAX}","aria-valuenow":"{percent}","aria-valuetext":"{percent} percent","aria-describedby":"app-size-help",
                    oninput:move |event|{if let Ok(value)=event.value().parse::<u16>(){preview_app_scale(preview.clone(),value);}},
                    onchange:move |_|persist_device(persist.clone())}
                div {class:"app-scale-legend",span {"{SCALE_MIN}%"} span {"{SCALE_DEFAULT}%"} span {"{SCALE_MAX}%"}}
            }
            div {class:"appearance-actions",
                button {class:"button secondary small",disabled:at_default&&repair.is_none(),onclick:move |_|reset_app_scale(reset.clone()),"Reset to 100%"}
                if !save_message.is_empty(){
                    p {class:"settings-note",role:"{save_role}","data-testid":"native-appearance-save-state","{save_message}"}
                }
            }
            div {class:"setting-divider"}
            div {class:"setting-row",
                div {class:"setting-text",
                    h3 {id:"theme-title","Theme"}
                    p {id:"theme-help","System follows this desktop's own light or dark appearance signal. Dark and Light apply immediately and keep readable contrast and focus."}
                }
                div {class:"theme-choices",role:"radiogroup","aria-labelledby":"theme-title",
                    for (value,label) in [(Theme::System,"System"),(Theme::Dark,"Dark"),(Theme::Light,"Light")] {
                        {
                            let choose = ui.clone();
                            rsx! {label {class:"theme-choice",key:"{label}",
                                input {r#type:"radio",name:"native-theme",value:"{value.as_str()}",checked:theme==value,onchange:move |_|set_device_theme(choose.clone(),value)}
                                span {"{label}"}
                            }}
                        }
                    }
                }
            }
            if !store.is_empty(){
                p {class:"settings-note","data-testid":"native-appearance-store","Stored for this computer only at {store} (owner-only file permissions). Removing that file restores the defaults."}
            }
            p {class:"settings-note","Account, playback, download and network preferences are not available in this version."}
        }
        section {class:"settings-panel","aria-labelledby":"storage-title","data-testid":"native-settings-storage",
            h2 {id:"storage-title","Storage"}
            div {class:"setting-row",
                div {class:"setting-text",
                    h3 {"Playback cache"}
                    p {"Reusable complete originals for this account on this server. Clearing it never deletes your library, saved offline copies, queue or current playback."}
                }
                div {class:"setting-value",
                    span {class:"app-scale-value","data-testid":"native-cache-usage",if cache_busy {"Measuring…"} else if let Some((count,bytes))=usage {{format!("{count} · {}",cache_size_label(bytes))}} else {"Unavailable"}}
                }
            }
            div {class:"appearance-actions",
                button {class:"button secondary small",disabled:cache_busy,onclick:move |_|refresh_cache_usage(refresh_cache.clone()),"Refresh cache size"}
                if cache_confirm {
                    button {class:"button danger small",disabled:cache_busy,onclick:move |_|clear_media_cache(clear_cache.clone()),"Confirm clear playback cache"}
                    button {class:"button secondary small",onclick:move |_|{let mut ui=cancel_cache.clone();ui.cache_confirm.set(false);},"Keep cache"}
                } else {
                    button {class:"button secondary small",disabled:cache_busy||usage.is_none(),onclick:move |_|{let mut ui=confirm_cache.clone();ui.cache_confirm.set(true);},"Clear playback cache"}
                }
            }
            p {class:"settings-note","Offline downloads are separate and are removed only from Library/Offline. Clearing the cache keeps an already playing decoder lease alive."}
        }
    }}
}

fn player(ui: Ui) -> Element {
    let track = ui.current_track();
    let remote = ui.remote_now.read().clone();
    // AUD-05: the local-only offline screen owns no server surface, so the player
    // omits the favorite mutation and the server queue view there.
    let local_only = ui.offline_mode();
    let offline_active = *ui.offline_active.read();
    // The offline screen is an offline source before the first Play as well, so the
    // transport never offers shuffle/repeat-all it would not execute.
    let offline_source = offline_active || local_only;
    let offline_current = ui.order.read().current();
    let offline_len = ui.queue.read().len();
    let snapshot = ui.snapshot.read().clone();
    let preparing = *ui.preparing.read();
    let source_canceled = *ui.source_canceled.read();
    let progress = *ui.progress.read();
    let preparation_message = source_preparation_message(progress, remote.is_some());
    let checking = ui.offline_checking.read().clone();
    let error = ui.source_error.read().clone();
    let (shuffle, repeat) = playback_modes(
        offline_source,
        remote.is_some(),
        *ui.shuffle.read(),
        *ui.repeat.read(),
    );
    let context = ui.context.read().clone();
    let toggle_ui = ui.clone();
    let previous = ui.clone();
    let next = ui.clone();
    let reconnect = ui.clone();
    let audio_alive = ui.audio_alive();
    let seek_ui = ui.clone();
    let volume_ui = ui.clone();
    let shuffle_ui = ui.clone();
    let repeat_ui = ui.clone();
    // A copy under verification is a pending transport action: the decoder keeps
    // what it has and every control that would replace or seek it waits.
    let blocked = (track.is_none() && remote.is_none()) || !ui.audio_ready() || checking.is_some();
    let playing = *ui.wants_play.read();
    let can_seek = !blocked
        && !preparing
        && ui.prepared.peek().is_some()
        && snapshot.duration_ms > 0
        && matches!(
            snapshot.phase,
            AudioPhase::Playing | AudioPhase::Paused | AudioPhase::Ended
        );
    let repeat_label = match repeat {
        Repeat::Off => "Repeat off",
        Repeat::All => "Repeat all",
        Repeat::One => "Repeat one",
    };
    let shuffle_help = if remote.is_some() {
        "Shuffle is unavailable for public sources"
    } else if offline_source {
        "Shuffle is unavailable for offline sources"
    } else {
        "Shuffle"
    };
    let repeat_help = if remote.is_some() {
        "Repeat is unavailable for public sources"
    } else if offline_source {
        "Offline repeat cycles between off and one track"
    } else {
        "Cycle repeat off, all and one"
    };
    let favorite_disabled = track
        .as_ref()
        .is_some_and(|track| ui.favorite_busy.read().as_str() == track.id);
    // The player only states facts the current DTO carries. Album comes from the
    // owned track or from the exact public queue occurrence; an absent album is
    // omitted instead of being replaced with invented metadata.
    let album = if remote.is_some() {
        ui.remote_queue
            .read()
            .get(*ui.remote_index.read())
            .map(|item| item.track.album.trim().to_owned())
            .filter(|album| !album.is_empty())
    } else {
        track
            .as_ref()
            .map(|track| track.album.trim().to_owned())
            .filter(|album| !album.is_empty())
    };
    let (title, artist) = match (&remote, &track) {
        (Some(remote), _) => (remote.title.clone(), remote.artist.clone()),
        (None, Some(track)) => (track.title.clone(), track.artist.clone()),
        (None, None) => (
            "Ready when you are".to_owned(),
            "Choose something you love".to_owned(),
        ),
    };
    // An untagged recording has no artist or album: the band then shows its title
    // only instead of inventing a performer or a release.
    let has_sub = !artist.is_empty() || album.is_some() || remote.is_some();
    let mut announcement = match (&remote, &track) {
        (Some(remote), _) if remote.artist.is_empty() => {
            format!("{}, public source", remote.title)
        }
        (Some(remote), _) => format!("{} by {}, public source", remote.title, remote.artist),
        (None, Some(track)) if track.artist.is_empty() => track.title.clone(),
        (None, Some(track)) => format!("{} by {}", track.title, track.artist),
        (None, None) => String::new(),
    };
    // Byte updates stay visual; announce the state once rather than every chunk.
    if preparing {
        announcement.push_str(". Preparing audio. Cancel is available.");
    } else if source_canceled {
        announcement.push_str(". Preparation canceled. Press Play to try again.");
    }
    // Mute is the engine's own zero level, so the button can never disagree with
    // what is actually coming out. The remembered audible level is restored on
    // Unmute rather than a hard-coded default.
    let muted = snapshot.volume <= 0.001;
    let volume_percent = (snapshot.volume * 100.0).round().clamp(0.0, 100.0);
    let volume_style = format!("--vela-progress:{volume_percent:.2}%;");
    let volume_text = format!("{volume_percent} percent");
    let seek_percent = if snapshot.duration_ms > 0 {
        (snapshot.position_ms as f64 * 100.0 / snapshot.duration_ms as f64).clamp(0.0, 100.0)
    } else {
        0.0
    };
    let seek_style = format!("--vela-progress:{seek_percent:.2}%;");
    let seek_text = format!(
        "{} of {}",
        time_label(snapshot.position_ms),
        time_label(snapshot.duration_ms)
    );
    let mut open_queue = ui.clone();
    let mute_ui = ui.clone();
    let cancel_preparation = ui.clone();
    rsx! {footer {class:"native-player","aria-label":"Native music player","data-audio-phase":"{snapshot.phase:?}","data-audio-generation":"{snapshot.generation}","data-preparing":"{preparing}",
        div {class:"player-track","data-testid":"native-player-track",
            ArtworkCover {ui:ui.clone(),artwork_id:track.as_ref().and_then(|track|track.artwork_id.clone()),class:"neutral-cover player-cover",icon:"music"}
            div {class:"player-meta",strong {"{title}"}
                if preparing {
                    p {class:"native-source-preparation","data-testid":"native-source-preparation","data-received-bytes":"{progress.received_bytes}","data-total-bytes":"{progress.total_bytes}",title:"The complete original is downloaded and checked before playback.","{preparation_message}"}
                } else if source_canceled {
                    p {class:"native-source-preparation","data-testid":"native-source-canceled",title:"Preparation canceled. Press Play to try again.","Preparation canceled. Press Play to try again."}
                } else if has_sub {
                    p {class:"player-sub",
                        if !artist.is_empty(){span {class:"player-artist","{artist}"}}
                        if let Some(album)=album.clone(){span {class:"player-album",title:"{album}","{album}"}}
                        if remote.is_some(){span {class:"player-kind","Public"}}
                    }
                }
            }
            if !local_only && remote.is_none() && let Some(favorite_track)=track.clone() {
                {
                    let favorite_ui=ui.clone();
                    rsx! { button {class:if favorite_track.favorite {"icon-button selected"}else{"icon-button"},"data-testid":"native-player-favorite","aria-pressed":favorite_track.favorite,disabled:favorite_disabled,"aria-label":if favorite_track.favorite {format!("Remove {} from favorites",favorite_track.title)} else {format!("Favorite {}",favorite_track.title)},title:if favorite_disabled {"Updating this favorite…"} else if favorite_track.favorite {"Remove from favorites"} else {"Add to favorites"},onclick:move |_|toggle_library_favorite(favorite_ui.clone(),favorite_track.clone()),Icon {name:"heart"}} }
                }
            }
        }
        div {class:"player-controls",
            div {class:"transport",role:"group","aria-label":"Playback transport",
                button {class:if shuffle{"icon-button selected"}else{"icon-button"},"aria-label":"Shuffle",title:shuffle_help,"aria-description":shuffle_help,"aria-pressed":shuffle,disabled:blocked||offline_source||remote.is_some(),onclick:move |_|{let ui=shuffle_ui.clone();let next=!*ui.shuffle.peek();set_shuffle(ui,next);},Icon {name:"shuffle"}}
                button {class:"icon-button","aria-label":"Previous",disabled:blocked||(offline_source&&offline_current.is_none_or(|current|current==0))||(remote.is_some()&&*ui.remote_index.read()==0),onclick:move |_|player_previous(previous.clone()),Icon {name:"previous"}}
                button {class:"native-play-toggle","data-testid":"native-player-play","aria-label":if playing{"Pause"}else{"Play"},disabled:blocked,onclick:move |_|toggle(toggle_ui.clone()),
                    span {class:"play-circle","aria-hidden":"true",if playing{Icon {name:"pause"}}else{Icon {name:"play"}}}
                }
                button {class:"icon-button","aria-label":"Next",disabled:blocked||(offline_source&&offline_current.is_none_or(|current|current+1>=offline_len))||(remote.is_some()&&*ui.remote_index.read()+1>=ui.remote_queue.read().len()),onclick:move |_|player_next(next.clone()),Icon {name:"next"}}
                button {class:if repeat!=Repeat::Off{"icon-button selected"}else{"icon-button"},"aria-label":repeat_label,title:repeat_help,"aria-description":repeat_help,"aria-pressed":repeat!=Repeat::Off,disabled:blocked||remote.is_some(),onclick:move |_|{let next=match repeat{Repeat::Off if offline_source=>Repeat::One,Repeat::Off=>Repeat::All,Repeat::All=>Repeat::One,Repeat::One=>Repeat::Off};set_repeat(repeat_ui.clone(),next);},if repeat==Repeat::One{Icon {name:"repeat-one"}}else{Icon {name:"repeat"}}}
            }
            div {class:"native-seek",
                span {class:"player-time","{time_label(snapshot.position_ms)}"}
                label {class:"sr-only",r#for:"native-seek","Playback position"}
                input {id:"native-seek",r#type:"range",min:0,max:snapshot.duration_ms.max(1),step:100,value:snapshot.position_ms,disabled:!can_seek,"aria-valuetext":"{seek_text}",style:"{seek_style}",onchange:move |event|seek(seek_ui.clone(),event.value())}
                span {class:"player-time","{time_label(snapshot.duration_ms)}"}
            }
        }
        div {class:"player-extras","data-testid":"native-player-extras",
            if !context.is_empty(){
                div {class:"player-source","data-testid":"native-player-source","aria-label":"Playback source",
                    span {class:"player-source-label","Source"}
                    span {class:"player-source-value",title:"{context}","{context}"}
                }
            }
            div {class:"player-actions",
                if preparing { button {class:"button secondary small native-cancel-preparation","data-testid":"native-cancel-preparation","aria-label":"Cancel audio preparation",onclick:move |_|cancel_source_preparation(cancel_preparation.clone()),"Cancel"} }
                if !local_only && !preparing { button {class:"icon-button player-queue","data-testid":"native-player-queue","aria-label":"Open queue",title:"Queue",onclick:move |_|open_queue.view.set("queue"),Icon {name:"queue"}} }
                button {class:if muted{"icon-button player-mute selected"}else{"icon-button player-mute"},"data-testid":"native-player-mute","aria-label":if muted{"Unmute"}else{"Mute"},"aria-pressed":muted,title:if muted{"Restore the last audible level"}else{"Mute the native output"},disabled:!ui.audio_ready(),onclick:move |_|toggle_mute(mute_ui.clone()),Icon {name:"volume"}}
                div {class:"player-volume",
                    label {class:"sr-only",r#for:"native-volume","Volume"}
                    input {id:"native-volume",r#type:"range",min:0,max:100,step:1,value:volume_percent,disabled:!ui.audio_ready(),"aria-valuetext":"{volume_text}",style:"{volume_style}",oninput:move |event|volume(volume_ui.clone(),event.value())}
                }
            }
        }
        div { class: "native-player-status", hidden: true, "data-testid": "native-playback-status", small { "{context}" } }
        div {class:"sr-only",role:"status","aria-live":"polite","aria-atomic":"true","data-testid":"native-player-announcement","{announcement}"}
        if !ui.audio_error.read().is_empty(){div {class:"player-error",role:"alert","{ui.audio_error}"}}
        if !error.is_empty(){div {class:"player-error",role:"alert","{error} Press Play to retry."}}
        if let Some(error)=snapshot.error{div {class:"player-error",role:"alert","{error}"}}
        if ui.audio.peek().is_some() && !audio_alive {div {class:"player-error",role:"alert","Native audio output stopped. " button {class:"text-button",onclick:move |_|reconnect_audio(reconnect.clone()),"Reconnect audio"}}}
    } }
}

#[allow(non_snake_case)]
pub fn App() -> Element {
    // Read the private device document on the first render, before any content is
    // painted, so a saved app size and theme are in place for the first frame. The
    // read is a bounded local file with no server contact; a bad document resolves
    // to the defaults and is explained in Settings.
    let device_load = use_hook(crate::presentation::load);
    let ui = Ui {
        alive: use_hook(|| Lifetime(Arc::new(AtomicBool::new(true)))),
        audio: use_signal(|| None),
        audio_error: use_signal(String::new),
        engine_cleanup: use_hook(|| Rc::new(RefCell::new(None))),
        login_gate: use_hook(Gate::default),
        catalog_gate: use_hook(Gate::default),
        feedback_gate: use_hook(Gate::default),
        playlist_gate: use_hook(Gate::default),
        playlist_write_gate: use_hook(Gate::default),
        folder_gate: use_hook(Gate::default),
        browse_gate: use_hook(Gate::default),
        remote_gate: use_hook(Gate::default),
        offline_gate: use_hook(Gate::default),
        offline_list_gate: use_hook(Gate::default),
        offline_play_gate: use_hook(Gate::default),
        source_gate: use_hook(Gate::default),
        logout_gate: use_hook(Gate::default),
        artwork_gate: use_hook(Gate::default),
        epoch: use_signal(|| 1),
        selection: use_signal(|| 0),
        generation_clock: use_hook(|| Rc::new(Cell::new(0))),
        account: use_signal(|| None),
        artwork: use_signal(|| crate::artwork::ArtworkStore::new(0)),
        origin: use_signal(default_origin),
        username: use_signal(String::new),
        password: use_signal(String::new),
        connecting: use_signal(|| false),
        resuming: use_signal(|| false),
        remember: use_signal(|| false),
        saved_session: use_signal(|| None),
        login_error: use_signal(String::new),
        view: use_signal(|| "library"),
        device: use_signal(|| device_load.preferences),
        device_save: use_signal(|| SaveStatus::Unchanged),
        device_repair: use_signal(|| device_load.repair),
        device_write: use_hook(|| Rc::new(RefCell::new(DeviceWrite::default()))),
        device_store: use_hook(|| Rc::new(crate::presentation::store_note())),
        query_draft: use_signal(String::new),
        query: use_signal(String::new),
        cursor: use_signal(|| None),
        catalog: use_signal(|| None),
        catalog_loading: use_signal(|| false),
        catalog_error: use_signal(String::new),
        favorite_busy: use_signal(String::new),
        playlists: use_signal(Vec::new),
        playlist: use_signal(|| None),
        playlist_id: use_signal(|| None),
        playlist_loading: use_signal(|| false),
        playlist_error: use_signal(String::new),
        playlist_name_draft: use_signal(String::new),
        folders: use_signal(Vec::new),
        folder_filter: use_signal(|| None),
        folder_name_draft: use_signal(String::new),
        folder_rename_draft: use_signal(String::new),
        folder_busy: use_signal(|| false),
        folder_confirm: use_signal(String::new),
        playlist_folder_target: use_signal(String::new),
        playlist_candidates: use_signal(Vec::new),
        playlist_candidate: use_signal(String::new),
        playlist_busy: use_signal(|| false),
        playlist_delete_confirm: use_signal(|| false),
        browse_kind: use_signal(|| "albums"),
        albums: use_signal(Vec::new),
        artists: use_signal(Vec::new),
        browse_tracks: use_signal(Vec::new),
        browse_id: use_signal(|| None),
        browse_title: use_signal(String::new),
        browse_loading: use_signal(|| false),
        browse_error: use_signal(String::new),
        remote_origin: use_signal(|| {
            std::env::var("VELA_NATIVE_QA_PEER_ORIGIN").unwrap_or_default()
        }),
        remote_preview: use_signal(|| None),
        remote_loading: use_signal(|| false),
        remote_error: use_signal(String::new),
        remote_now: use_signal(|| None),
        remote_queue: use_signal(Vec::new),
        remote_index: use_signal(|| 0),
        remote_query: use_signal(String::new),
        remote_results: use_signal(Vec::new),
        remote_searching: use_signal(|| false),
        remote_search_error: use_signal(String::new),
        remote_more_busy: use_signal(|| false),
        remote_more_error: use_signal(String::new),
        remote_peer_more_busy: use_signal(String::new),
        remote_peer_more_error: use_signal(String::new),
        remote_retry_busy: use_signal(String::new),
        remote_retry_error: use_signal(String::new),
        offline: use_signal(Vec::new),
        offline_busy: use_signal(String::new),
        offline_error: use_signal(String::new),
        offline_corrupt: use_signal(Vec::new),
        offline_active: use_signal(|| false),
        offline_local: use_signal(|| None),
        offline_session: use_signal(|| None),
        offline_confirm: use_signal(String::new),
        cache_usage: use_signal(|| None),
        cache_busy: use_signal(|| false),
        cache_confirm: use_signal(|| false),
        restored_position: use_signal(|| 0),
        offline_checking: use_signal(|| None),
        offline_removing: use_signal(String::new),
        saved_peers: use_signal(|| crate::peers::load().unwrap_or_default()),
        queue: use_signal(Vec::new),
        order: use_signal(PlaybackOrder::default),
        shuffle: use_signal(|| false),
        repeat: use_signal(Repeat::default),
        context: use_signal(String::new),
        snapshot: use_signal(AudioSnapshot::default),
        volume_restore: use_signal(|| 0.7),
        preparing: use_signal(|| false),
        source_canceled: use_signal(|| false),
        wants_play: use_signal(|| false),
        progress: use_signal(DownloadProgress::default),
        prepared: use_signal(|| None),
        source_error: use_signal(String::new),
        notice: use_signal(|| {
            device_load
                .repair
                .map(Repair::message)
                .unwrap_or_default()
                .to_owned()
        }),
        ended: use_hook(|| Rc::new(Cell::new(None))),
        mpris: use_hook(mpris::spawn),
        mpris_note: use_signal(|| "System media controls starting…"),
        mpris_seek_serial: use_signal(|| 0),
    };
    let cleanup = ui.clone();
    use_drop(move || {
        cleanup.alive.set(false);
        cleanup.cancel_all();
        if let Some(audio) = cleanup.engine_cleanup.borrow_mut().take() {
            let _ = audio.stop(cleanup.generation_clock.get().wrapping_add(1));
        }
    });
    let initialize_ui = ui.clone();
    use_future(move || {
        let mut initializing = initialize_ui.clone();
        async move {
            let lifetime = initializing.alive.clone();
            let result = tokio::task::spawn_blocking(move || {
                let result = AudioEngine::new();
                // A detached blocking constructor still drops its late output on close.
                if !lifetime.get() {
                    drop(result);
                    None
                } else {
                    Some(result)
                }
            })
            .await;
            if !initializing.alive.get() {
                return;
            }
            match result {
                Ok(Some(Ok(engine))) => {
                    *initializing.engine_cleanup.borrow_mut() = Some(engine.clone());
                    initializing.audio.set(Some(engine));
                }
                Ok(Some(Err(error))) => initializing.audio_error.set(error),
                Err(_) => initializing.audio_error.set(
                    "Native audio output could not be initialized. Restart Vela to try again."
                        .into(),
                ),
                Ok(None) => {}
            }
        }
    });
    let observe_ui = ui.clone();
    use_future(move || {
        let mut observer = observe_ui.clone();
        async move {
            loop {
                tokio::time::sleep(Duration::from_millis(200)).await;
                if !observer.alive.get() {
                    return;
                }
                // External OS controls are applied on this UI thread so every
                // account/session/lifetime check remains authoritative.
                while let Some(command) = observer.mpris.try_command() {
                    if apply_system_control(observer.clone(), command) {
                        return;
                    }
                }
                let note = system_controls_note(observer.mpris.state());
                if *observer.mpris_note.peek() != note {
                    observer.mpris_note.set(note);
                }
                if !observer.can_play() {
                    observer.publish_system_controls();
                    continue;
                }
                let Some(audio) = observer.audio.peek().clone() else {
                    observer.publish_system_controls();
                    continue;
                };
                let snapshot = audio.snapshot();
                let generation = *observer.selection.peek();
                if snapshot.generation != generation {
                    observer.publish_system_controls();
                    continue;
                }
                let ended = snapshot.phase == AudioPhase::Ended;
                if ended || snapshot.phase == AudioPhase::Error {
                    observer.wants_play.set(false);
                }
                observer.snapshot.set(snapshot);
                observer.publish_system_controls();
                if ended && observer.ended.get() != Some(generation) {
                    observer.ended.set(Some(generation));
                    if observer.remote_now.peek().is_some() {
                        // Keep the public item/queue visible after natural EOF so
                        // the listener can replay or move through the remote queue.
                        observer.prepared.set(None);
                    } else {
                        advance(observer.clone(), Command::Ended);
                    }
                } else if !ended {
                    observer.ended.set(None);
                }
            }
        }
    });
    let _account = ui.account.read();
    // One wrapper attribute pair for the validated device size/theme. Reapplying it
    // rewrites the inline zoom and `data-theme` only: the audio engine, prepared
    // source, queue, cursor and position are untouched and no request is made.
    let device = ui.device_attributes();
    let view = *ui.view.read();
    let notice = ui.notice.read().clone();
    let mut notice_signal = ui.notice;
    let mut view_signal = ui.view;
    let logout = ui.clone();
    let nav_library = ui.clone();
    let nav_favorites = ui.clone();
    let nav_albums = ui.clone();
    let nav_artists = ui.clone();
    let nav_playlists = ui.clone();
    let nav_public = ui.clone();
    let compact_library = ui.clone();
    let compact_favorites = ui.clone();
    let compact_albums = ui.clone();
    let compact_artists = ui.clone();
    let compact_playlists = ui.clone();
    let compact_public = ui.clone();
    let theme_toggle = ui.clone();
    let css = include_str!("../style.css");
    // AUD-05: one coordinated cold-start sequence. The private offline profile and
    // its real saved copies are read locally first, then the opt-in saved session;
    // a usable saved session is resumed once through the existing guarded path and
    // anything other than a real sign-in falls through to the offline library.
    let starting = ui.clone();
    use_future(move || {
        let starting = starting.clone();
        async move { startup(starting).await }
    });
    rsx! {
        document::Style {"{css}"}
        div {class:"native-app","data-theme":"{device.theme}","data-app-scale":"{device.app_scale}",style:"{device.style}",
            if ui.signed_in(){
                aside {class:"native-sidebar",div {class:"native-brand",Icon {name:"music"} span {"vela"} small {"NATIVE"}}
                    nav {"aria-label":"Main navigation",button {class:if view=="library"{"native-nav active"}else{"native-nav"},"aria-label":"Library",onclick:move |_|{view_signal.set("library");load_catalog(nav_library.clone(),String::new(),None,false);},Icon {name:"library"}"Library"} button {class:if view=="albums"{"native-nav active"}else{"native-nav"},"aria-label":"Albums",onclick:move |_|{load_groups(nav_albums.clone(),"albums",None);},Icon {name:"albums"}"Albums"} button {class:if view=="artists"{"native-nav active"}else{"native-nav"},"aria-label":"Artists",onclick:move |_|{load_groups(nav_artists.clone(),"artists",None);},Icon {name:"artists"}"Artists"} button {class:if view=="favorites"{"native-nav active"}else{"native-nav"},"aria-label":"Favorites",onclick:move |_|{view_signal.set("favorites");load_catalog(nav_favorites.clone(),String::new(),None,true);},Icon {name:"heart"}"Favorites"} button {class:if view=="playlists"{"native-nav active"}else{"native-nav"},"aria-label":"Playlists",onclick:move |_|{load_playlists(nav_playlists.clone());},Icon {name:"playlists"}"Playlists"} button {class:if view=="public"{"native-nav active"}else{"native-nav"},"aria-label":"Public",onclick:move |_|{view_signal.set("public");explore_public(nav_public.clone());},Icon {name:"network"}"Public"} button {class:if view=="queue"{"native-nav active"}else{"native-nav"},"aria-label":"Queue",onclick:move |_|view_signal.set("queue"),Icon {name:"queue"}"Queue"} button {class:if view=="settings"{"native-nav active"}else{"native-nav"},"aria-label":"Settings",onclick:move |_|view_signal.set("settings"),Icon {name:"settings"}"Settings"}}
                    div {class:"native-private-note",Icon {name:"check"} strong {"Private by default"} p {"Share only what you choose."} p {class:"native-mpris-note",role:"status","data-testid":"native-mpris-status","{ui.mpris_note}"}}
                }
                main {class:"native-main",
                    header {class:"native-topbar",
                        div {class:"native-window-drag",onmousedown:move |event|{if event.trigger_button()==Some(dioxus::html::input_data::MouseButton::Primary){dioxus::desktop::window().drag();}},ondoubleclick:move |_|dioxus::desktop::window().toggle_maximized(),span {class:"eyebrow","YOUR PERSONAL MUSIC SPACE"} if let Some(account)=ui.account.read().as_ref(){strong {"{account.user.username}"} small {"{account.origin}"}}}
                        div {class:"topbar-actions",button {class:"icon-button","aria-label":if device.theme!="light"{"Use the light theme"}else{"Use the dark theme"},title:if device.theme!="light"{"Switch to the light theme"}else{"Switch to the dark theme"},onclick:move |_|toggle_device_theme(theme_toggle.clone()),if device.theme!="light"{Icon {name:"sun"}}else{Icon {name:"moon"}}} button {class:"button secondary small",onclick:move |_|sign_out(logout.clone()),"Sign out"} WindowControls {}}
                    }
                    div {class:"native-scroll","data-testid":"native-scroll",
                        div {class:"native-scroll-inner",
                            nav {class:"native-compact-nav","aria-label":"Compact navigation",button {class:if view=="library"{"active"}else{""},onclick:move |_|{view_signal.set("library");load_catalog(compact_library.clone(),String::new(),None,false);},"Library"} button {class:if view=="albums"{"active"}else{""},onclick:move |_|{load_groups(compact_albums.clone(),"albums",None);},"Albums"} button {class:if view=="artists"{"active"}else{""},onclick:move |_|{load_groups(compact_artists.clone(),"artists",None);},"Artists"} button {class:if view=="favorites"{"active"}else{""},onclick:move |_|{view_signal.set("favorites");load_catalog(compact_favorites.clone(),String::new(),None,true);},"Favorites"} button {class:if view=="playlists"{"active"}else{""},onclick:move |_|{load_playlists(compact_playlists.clone());},"Playlists"} button {class:if view=="public"{"active"}else{""},onclick:move |_|{view_signal.set("public");explore_public(compact_public.clone());},"Public"} button {class:if view=="queue"{"active"}else{""},onclick:move |_|view_signal.set("queue"),"Queue"} button {class:if view=="settings"{"active"}else{""},"aria-label":"Settings",onclick:move |_|view_signal.set("settings"),"Settings"}}
                            if !notice.is_empty(){div {class:"alert notice",role:"status",span {"{notice}"} button {class:"icon-button","aria-label":"Dismiss notification",onclick:move |_|notice_signal.set(String::new()),Icon {name:"close"}}}}
                            if view=="queue"{{queue_view(ui.clone())}}else if view=="playlists"{{playlists_view(ui.clone())}}else if view=="albums"||view=="artists"{{browse_view(ui.clone())}}else if view=="public"{{public_view(ui.clone())}}else if view=="settings"{{settings_view(ui.clone())}}else{{library(ui.clone())}}
                        }
                    }
                }
                {player(ui.clone())}
            }else if ui.offline_session.read().is_some(){
                // AUD-05: a truthful local library, rendered before any `/api/me`
                // success and without an account, token or request.
                {offline_library(ui.clone())}
            }else{
                div {class:"signin-window-controls",WindowControls {}}
                if view=="settings"{
                    main {class:"native-signed-out-settings",
                        div {class:"signed-out-back",button {class:"button secondary small",onclick:move |_|view_signal.set("library"),Icon {name:"back"}"Back to sign in"}}
                        {settings_view(ui.clone())}
                    }
                }else{{sign_in(ui.clone())}}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cover_slots_fall_back_for_missing_pending_denied_and_failed_states() {
        assert_eq!(cover_state(false, None), "missing");
        assert_eq!(
            cover_state(
                false,
                Some(&ArtworkStatus::Ready("data:image/jpeg;base64,AA".into()))
            ),
            "missing",
            "an absent artwork id never renders bytes, even if a stale state existed"
        );
        assert_eq!(cover_state(true, None), "pending");
        assert_eq!(cover_state(true, Some(&ArtworkStatus::Loading)), "loading");
        assert_eq!(cover_state(true, Some(&ArtworkStatus::Denied)), "denied");
        assert_eq!(cover_state(true, Some(&ArtworkStatus::Failed)), "failed");
        assert_eq!(
            cover_state(
                true,
                Some(&ArtworkStatus::Ready("data:image/jpeg;base64,AA".into()))
            ),
            "ready"
        );
        // Only `Ready` exposes a renderer source; nothing else may reach an `<img>`.
        assert!(ArtworkStatus::Denied.data_url().is_none());
        assert!(ArtworkStatus::Failed.data_url().is_none());
        assert!(ArtworkStatus::Loading.data_url().is_none());
    }

    #[test]
    fn source_modes_do_not_apply_owned_shuffle_or_repeat_all_to_other_adapters() {
        let preferences = (true, Repeat::All);
        assert_eq!(
            playback_modes(true, false, preferences.0, preferences.1),
            (false, Repeat::Off)
        );
        assert_eq!(
            playback_modes(false, true, preferences.0, preferences.1),
            (false, Repeat::Off)
        );
        let (shuffle, repeat) = playback_modes(false, false, preferences.0, preferences.1);
        let mut order = PlaybackOrder::default();
        order.reset(2, 1);
        order.configure(shuffle, repeat);
        assert_eq!(
            order.transition(Command::Ended, 30.0, 0),
            Transition::Play(0),
            "returning to owned playback retains the original queue preferences"
        );
    }

    #[test]
    fn offline_repeat_one_is_an_available_mode_without_claiming_shuffle() {
        let (shuffle, repeat) = playback_modes(true, false, true, Repeat::One);
        assert!(!shuffle);
        let mut order = PlaybackOrder::default();
        order.reset(2, 0);
        order.configure(shuffle, repeat);
        assert_eq!(
            order.transition(Command::Ended, 30.0, 0),
            Transition::SeekStart { resume: true }
        );
        assert_eq!(
            order.transition(Command::Next, 0.0, 0),
            Transition::Play(1),
            "repeat one does not capture manual Next"
        );
    }

    #[test]
    fn saved_session_failures_are_classified_by_exact_client_message() {
        // HTTP 401 from `/api/me` and the generic expired-session error are the
        // only messages that may delete the stored item.
        assert_eq!(
            saved_session_failure(SAVED_SESSION_EXPIRED),
            SavedSessionFailure::Rejected
        );
        assert_eq!(
            saved_session_failure(SESSION_SIGNED_OUT),
            SavedSessionFailure::Rejected
        );
        // Transient transport and server conditions must preserve the item.
        for error in [
            NETWORK_UNREACHABLE,
            REQUEST_TIMED_OUT,
            REQUEST_FAILED,
            "The server is busy. Wait before retrying.",
        ] {
            assert_eq!(
                saved_session_failure(error),
                SavedSessionFailure::Transient,
                "{error}"
            );
        }
        // A malformed stored record is unusable but was not rejected by a server.
        assert_eq!(
            saved_session_failure(SAVED_SESSION_INVALID),
            SavedSessionFailure::Unusable
        );
        // A 403 talks about signing in but is not a session rejection.
        assert_eq!(
            saved_session_failure(
                "The server denied this action. Sign in again or check your permissions."
            ),
            SavedSessionFailure::Refused
        );
        assert_eq!(
            saved_session_failure(
                "The server redirected a Vela request. Use its direct HTTPS origin."
            ),
            SavedSessionFailure::Refused
        );
        assert_eq!(
            saved_session_failure("The server returned an invalid Vela response."),
            SavedSessionFailure::Refused
        );
        assert_eq!(
            saved_session_failure(REQUEST_CANCELED),
            SavedSessionFailure::Canceled
        );
        assert_eq!(
            saved_session_failure("something new"),
            SavedSessionFailure::Refused
        );
    }

    #[test]
    fn only_a_real_session_rejection_deletes_the_saved_item() {
        for failure in [
            SavedSessionFailure::Unusable,
            SavedSessionFailure::Transient,
            SavedSessionFailure::Refused,
            SavedSessionFailure::Canceled,
        ] {
            assert!(!failure.deletes_saved_item(), "{failure:?}");
        }
        assert!(SavedSessionFailure::Rejected.deletes_saved_item());
    }

    #[test]
    fn non_rejection_messages_say_the_item_was_kept() {
        let error = NETWORK_UNREACHABLE;
        assert!(
            SavedSessionFailure::Transient
                .message(error)
                .contains("kept")
        );
        assert!(
            SavedSessionFailure::Unusable
                .message(error)
                .contains("kept")
        );
        assert!(SavedSessionFailure::Refused.message(error).contains("kept"));
        assert_eq!(SavedSessionFailure::Rejected.message(error), error);
        // The rejected message never claims the item survived.
        assert!(
            !SavedSessionFailure::Rejected
                .message(error)
                .contains("kept")
        );
    }

    fn sample_track(id: &str) -> Track {
        Track {
            id: id.into(),
            title: format!("Track {id}"),
            artist: "Vela Test".into(),
            album: "Offline".into(),
            album_artist: "Vela Test".into(),
            genre: String::new(),
            year: None,
            track_number: Some(1),
            disc_number: Some(1),
            duration_ms: 30_000,
            codec: "flac".into(),
            bitrate: None,
            sample_rate: Some(44_100),
            bit_depth: Some(16),
            channels: Some(2),
            size_bytes: 1_024,
            artwork_id: None,
            added_at: 1,
            favorite: false,
        }
    }

    /// REG-AUD-07: removing a saved copy that is not the active occurrence must
    /// leave the decoder and its occurrence alone while Next/Previous keep
    /// working on the shortened queue.
    #[test]
    fn removing_a_different_saved_copy_keeps_playback_and_remaps_the_queue() {
        let mut queue = vec![
            sample_track("a"),
            sample_track("b"),
            sample_track("c"),
            sample_track("d"),
        ];
        let mut order = PlaybackOrder::default();
        order.reset(queue.len(), 1);
        let playing = queue[1].clone();

        let effect = offline_removal(true, order.current(), &queue, "d");
        assert_eq!(effect, OfflineRemoval::Remap(3));
        let OfflineRemoval::Remap(at) = effect else {
            unreachable!("checked above");
        };
        assert!(order.remove(at));
        queue.remove(at);
        // The active source and its occurrence survive untouched.
        assert_eq!(order.current(), Some(1));
        assert_eq!(queue[order.current().expect("still current")], playing);
        // Next and Previous still walk the remapped queue.
        assert_eq!(order.transition(Command::Next, 0.0, 0), Transition::Play(2));
        assert_eq!(queue[2].id, "c");
        assert_eq!(
            order.transition(Command::Previous, 0.0, 0),
            Transition::Play(1)
        );
        assert_eq!(queue[1].id, "b");
    }

    /// Removing an occurrence before the active one shifts the current index
    /// instead of changing the track that is playing.
    #[test]
    fn removing_an_earlier_saved_copy_shifts_the_active_occurrence() {
        let mut queue = vec![
            sample_track("a"),
            sample_track("b"),
            sample_track("c"),
            sample_track("d"),
        ];
        let mut order = PlaybackOrder::default();
        order.reset(queue.len(), 2);
        assert_eq!(
            offline_removal(true, order.current(), &queue, "a"),
            OfflineRemoval::Remap(0)
        );
        assert!(order.remove(0));
        queue.remove(0);
        assert_eq!(order.current(), Some(1));
        assert_eq!(queue[1].id, "c");
        assert_eq!(order.transition(Command::Next, 0.0, 0), Transition::Play(2));
        assert_eq!(queue[2].id, "d");
    }

    /// The stop/remap/keep decision is made before any signal is written.
    #[test]
    fn removal_effect_depends_on_the_active_occurrence_only() {
        let queue = vec![sample_track("a"), sample_track("b")];
        assert_eq!(
            offline_removal(true, Some(1), &queue, "b"),
            OfflineRemoval::StopPlayback
        );
        // Removing a device copy while a library source plays stops nothing.
        assert_eq!(
            offline_removal(false, Some(1), &queue, "b"),
            OfflineRemoval::Keep
        );
        // A copy saved after playback started is not part of the live queue.
        assert_eq!(
            offline_removal(true, Some(1), &queue, "z"),
            OfflineRemoval::Keep
        );
        // A cleared offline queue never stops a stopped decoder.
        assert_eq!(
            offline_removal(true, None, &queue, "b"),
            OfflineRemoval::Remap(1)
        );
    }

    /// REG-AUD-08: a delayed delete/reload/verify may only publish while the
    /// account epoch, the account identity (user id and origin) and the
    /// operation generation are still the live ones.
    #[test]
    fn a_delayed_offline_operation_cannot_publish_for_a_replaced_account() {
        let owner = OfflineOwner {
            gate: Gate::default(),
            generation: 7,
            epoch: 4,
            user_id: "account-a".into(),
            origin: "https://a.example".into(),
            identity: OfflineIdentity::Account,
        };
        let same_account = Some(("account-a", "https://a.example"));
        assert!(owner.matches(true, false, 7, 4, same_account));
        assert!(
            !owner.matches(true, false, 7, 5, same_account),
            "sign-out/sign-in bumps the account epoch"
        );
        assert!(
            !owner.matches(true, false, 7, 4, Some(("account-a", "https://b.example"))),
            "the same user UUID on another origin is another local library"
        );
        assert!(
            !owner.matches(true, false, 7, 4, Some(("account-b", "https://a.example"))),
            "another account may not publish this result"
        );
        assert!(
            !owner.matches(true, false, 8, 4, same_account),
            "a newer offline operation superseded this one"
        );
        assert!(
            !owner.matches(true, false, 7, 4, None),
            "signed out while the delete was running"
        );
        assert!(
            !owner.matches(true, true, 7, 4, same_account),
            "canceled work publishes nothing"
        );
        assert!(
            !owner.matches(false, false, 7, 4, same_account),
            "a closed window publishes nothing"
        );
    }

    /// REG-AUD-05: the local-only offline screen runs the same freshness rule on a
    /// private-profile identity. A delayed verification, reload or play may only
    /// publish for the profile that is still on screen, and the account and profile
    /// identities can never stand in for each other.
    #[test]
    fn a_delayed_offline_result_cannot_publish_for_a_replaced_offline_profile() {
        let owner = OfflineOwner {
            gate: Gate::default(),
            generation: 3,
            epoch: 2,
            user_id: "account-a".into(),
            origin: "https://a.example".into(),
            identity: OfflineIdentity::Profile,
        };
        assert_eq!(owner.identity, OfflineIdentity::Profile);
        let same = Some(("account-a", "https://a.example"));
        assert!(owner.matches(true, false, 3, 2, same));
        assert!(
            !owner.matches(true, false, 3, 3, same),
            "leaving or replacing the offline screen bumps the epoch"
        );
        assert!(
            !owner.matches(true, false, 3, 2, Some(("account-a", "https://b.example"))),
            "the same account UUID on another origin is another local library"
        );
        assert!(
            !owner.matches(true, false, 3, 2, Some(("account-b", "https://a.example"))),
            "a replaced profile may not publish this result"
        );
        assert!(
            !owner.matches(true, false, 3, 2, None),
            "leaving the offline screen publishes nothing"
        );
        assert!(!owner.matches(true, true, 3, 2, same), "canceled work");
        assert!(!owner.matches(false, false, 3, 2, same), "closed window");
        // A profile identity never equals an account identity with the same UUID.
        let account = OfflineOwner {
            identity: OfflineIdentity::Account,
            ..owner
        };
        assert_ne!(account.identity, OfflineIdentity::Profile);
    }
}
