//! Reusable complete-file playback cache of one authenticated account on one origin.
//!
//! Layout: `$XDG_CACHE_HOME/vela/media/<origin_hash>/<account_id>/<track_id>.audio`
//! plus a versioned JSON sidecar `<track_id>.json`. `origin_hash` is the BLAKE3 hex of
//! the canonical origin, so the same account UUID and the same track UUID on two
//! different servers never share a path, a sidecar or a byte.
//!
//! This cache is an optimization, not a grant and not an offline copy:
//!
//! - a hit is only attempted after the origin itself re-authorized the current
//!   session and returned the same strong validator (`client::prepare`); cached bytes
//!   never answer a revoked, denied or removed request;
//! - a hit still hashes the cached file from the same opened descriptor it hands to
//!   the decoder, so decoded bytes are the verified bytes;
//! - the sidecar digest must equal the authoritative origin ETag, so rewriting both a
//!   cached file and its sidecar cannot forge a hit of tampered bytes;
//! - publication is atomic (audio first, sidecar last), a failed publication restores
//!   the previous healthy entry, and eviction only ever deletes this cache's own
//!   `<uuid>.audio` + `<uuid>.json` pairs. Pinned/offline copies live under
//!   `$XDG_DATA_HOME/vela/offline` and are structurally unreachable from this root.
//!
//! Every entry point is meant to run in a blocking worker and takes a cooperative
//! cancellation probe, consulted between bounded 64 KiB blocks.
use crate::PreparedAudio;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

/// Complete originals one account+origin namespace may keep.
pub(crate) const MAX_ENTRIES: usize = 200;
/// Bytes one account+origin namespace may keep, staging/orphans/metadata included.
pub(crate) const MAX_BYTES: u64 = 2 * 1024 * 1024 * 1024;
/// A sidecar is a few hundred bytes; anything larger is neither read nor written.
const MAX_SIDECAR_BYTES: usize = 4 * 1024;
/// Directory entries inspected per pass; a namespace only holds two files per entry.
const MAX_DIRECTORY_ENTRIES: usize = 8192;
/// Bounded streaming block for hashing/copying and cancellation latency.
pub(crate) const BLOCK_BYTES: usize = 64 * 1024;
/// Sidecar schema version. A sidecar of another version is never reinterpreted.
const VERSION: u8 = 1;

/// The wall clock of the recency label: whole seconds since the Unix epoch.
///
/// The label of a confirmed hit is the clock of the moment of the mark, so two
/// tracks are always compared against the same clock. A test may pin the clock
/// to make the recorded value deterministic.
type Clock<'a> = &'a dyn Fn() -> u64;

pub(crate) const CANCELED: &str = "The media cache operation was canceled.";
const UNAVAILABLE: &str = "The prepared original could not be cached.";
const INTEGRITY: &str = "The cached audio copy failed its BLAKE3 fingerprint check.";
const WRITE: &str = "The playback cache could not be written.";
const MEASURE: &str = "The playback cache could not be measured.";
const FULL: &str = "The playback cache is full.";

/// Identity of one cached representation. The origin is the canonical origin string
/// (with its trailing `/`), never a user-supplied spelling of the same server.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct CacheKey {
    origin: String,
    account_id: String,
    track_id: String,
}

impl CacheKey {
    pub(crate) fn new(origin: &str, account_id: &str, track_id: &str) -> Result<Self, String> {
        if origin.is_empty()
            || origin.len() > 2048
            || origin.chars().any(|c| c.is_control() || c.is_whitespace())
            || !crate::client::canonical_uuid(account_id)
            || !crate::client::canonical_uuid(track_id)
        {
            return Err(UNAVAILABLE.into());
        }
        Ok(Self {
            origin: origin.to_owned(),
            account_id: account_id.to_owned(),
            track_id: track_id.to_owned(),
        })
    }
    pub(crate) fn track_id(&self) -> &str {
        &self.track_id
    }
}

/// A verified complete original. `file` is the descriptor that was hashed and rewound,
/// so the decoder reads exactly the bytes this cache verified.
pub(crate) struct Cached {
    /// Only the descriptor-lease platform reads this field; elsewhere a cached original
    /// cannot become a decoder lease and the verified GET is used instead.
    #[cfg_attr(not(target_os = "linux"), allow(dead_code))]
    pub(crate) file: fs::File,
    /// The verified length of the cached original. It is read where the cached
    /// bytes become a decoder lease; elsewhere the verified GET is used instead.
    #[cfg_attr(not(target_os = "linux"), allow(dead_code))]
    pub(crate) length_bytes: u64,
}

/// Sidecar written next to the audio. `digest` is the BLAKE3 of the cached audio and
/// must equal `etag`, the origin's strong validator for the same representation.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Sidecar {
    version: u8,
    origin: String,
    account_id: String,
    track_id: String,
    size_bytes: u64,
    etag: String,
    digest: String,
    saved_at: u64,
}

/// The budget of one account+origin namespace. The production values are the module
/// constants; tests use smaller ones to exercise refusal and eviction.
#[derive(Clone, Copy)]
pub(crate) struct Budget {
    pub(crate) entries: usize,
    pub(crate) bytes: u64,
}
impl Default for Budget {
    fn default() -> Self {
        Self {
            entries: MAX_ENTRIES,
            bytes: MAX_BYTES,
        }
    }
}

#[derive(Clone)]
struct Entry {
    track_id: String,
    saved_at: u64,
    bytes: u64,
}

/// `$XDG_CACHE_HOME/vela/media`, or `$HOME/.cache/vela/media`. No cache root means no
/// cache (the verified full transfer stays the only path), never a public temp dir.
pub(crate) fn default_root() -> Result<PathBuf, String> {
    let base = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| {
            std::env::var_os("HOME")
                .map(PathBuf::from)
                .map(|home| home.join(".cache"))
        })
        .ok_or_else(|| "No local cache directory is available for native playback.".to_owned())?;
    Ok(base.join("vela").join("media"))
}

fn origin_hash(origin: &str) -> String {
    blake3::hash(origin.as_bytes()).to_hex().to_string()
}

/// `<root>/<origin_hash>/<account_id>`: the namespace the budget and the files are
/// both scoped to.
pub(crate) fn account_root(root: &Path, key: &CacheKey) -> PathBuf {
    root.join(origin_hash(&key.origin)).join(&key.account_id)
}

fn namespace_root(origin: &str, account_id: &str) -> Result<PathBuf, String> {
    if origin.is_empty()
        || origin.len() > 2048
        || origin.chars().any(|c| c.is_control() || c.is_whitespace())
        || !crate::client::canonical_uuid(account_id)
    {
        return Err(UNAVAILABLE.into());
    }
    Ok(default_root()?.join(origin_hash(origin)).join(account_id))
}

/// Real bytes and published entry count of this account+origin playback cache.
pub(crate) fn usage(origin: &str, account_id: &str) -> Result<(usize, u64), String> {
    usage_at(&namespace_root(origin, account_id)?)
}

fn usage_at(root: &Path) -> Result<(usize, u64), String> {
    let entries = published(root)?;
    let bytes = entries.iter().map(|entry| entry.bytes).sum();
    Ok((entries.len(), bytes))
}

/// Delete only this account+origin published cache pairs. Staging, orphans and
/// pinned offline copies are deliberately left untouched; an open decoder lease
/// keeps playing because Unix unlink does not close the descriptor.
pub(crate) fn clear(origin: &str, account_id: &str) -> Result<(), String> {
    let root = namespace_root(origin, account_id)?;
    clear_at(&root, origin, account_id)
}

fn clear_at(account: &Path, origin: &str, account_id: &str) -> Result<(), String> {
    for entry in published(account)? {
        // `entry.track_id` was canonicalised by `published`.
        let _ = CacheKey::new(origin, account_id, &entry.track_id)?;
        let _ = fs::remove_file(account.join(format!("{}.audio", entry.track_id)));
        let _ = fs::remove_file(account.join(format!("{}.json", entry.track_id)));
    }
    Ok(())
}

pub(crate) fn audio_path(root: &Path, key: &CacheKey) -> PathBuf {
    account_root(root, key).join(format!("{}.audio", key.track_id))
}

pub(crate) fn sidecar_path(root: &Path, key: &CacheKey) -> PathBuf {
    account_root(root, key).join(format!("{}.json", key.track_id))
}

/// Reuse a cached complete original only when it still is the representation the
/// origin just re-authorized. `Ok(None)` is an honest miss (missing, mismatched,
/// tampered, or a sidecar of a different version); `Err` is cancellation only.
pub(crate) fn open_verified(
    root: &Path,
    key: &CacheKey,
    expected_size: u64,
    expected_etag: &str,
    canceled: &dyn Fn() -> bool,
) -> Result<Option<Cached>, String> {
    if canceled() {
        return Err(CANCELED.into());
    }
    if expected_size == 0
        || expected_size > crate::client::MAX_MEDIA_BYTES
        || !crate::client::lower_hex64(expected_etag)
    {
        return Ok(None);
    }
    let Some(sidecar) = read_sidecar(&sidecar_path(root, key)) else {
        return Ok(None);
    };
    if sidecar.version != VERSION
        || sidecar.origin != key.origin
        || sidecar.account_id != key.account_id
        || sidecar.track_id != key.track_id
        || sidecar.size_bytes != expected_size
        || sidecar.etag != expected_etag
        // A local edit of both the file and its sidecar still cannot forge a hit:
        // only the origin's validator is authoritative for immutable bytes.
        || sidecar.digest != expected_etag
        || !crate::client::lower_hex64(&sidecar.digest)
        || sidecar.saved_at == 0
    {
        return Ok(None);
    }
    let path = audio_path(root, key);
    let Ok(link) = fs::symlink_metadata(&path) else {
        return Ok(None);
    };
    // A symlink is never a cached original, even one that points at a real file.
    if link.file_type().is_symlink() || !link.is_file() {
        return Ok(None);
    }
    let Ok(mut file) = fs::File::open(&path) else {
        return Ok(None);
    };
    let Ok(metadata) = file.metadata() else {
        return Ok(None);
    };
    if !metadata.is_file() || metadata.len() != sidecar.size_bytes {
        return Ok(None);
    }
    let mut hasher = blake3::Hasher::new();
    let mut buffer = [0_u8; BLOCK_BYTES];
    loop {
        if canceled() {
            return Err(CANCELED.into());
        }
        match file.read(&mut buffer) {
            Ok(0) => break,
            Ok(read) => {
                hasher.update(&buffer[..read]);
            }
            Err(_) => return Ok(None),
        }
    }
    if hasher.finalize().to_hex().as_str() != sidecar.digest {
        return Ok(None);
    }
    if file.seek(SeekFrom::Start(0)).is_err() {
        return Ok(None);
    }
    Ok(Some(Cached {
        file,
        length_bytes: metadata.len(),
    }))
}

/// Mark one published entry as recently played.
///
/// The recency of an entry is the time of its last *confirmed hit*, not the
/// time it was inserted: a track that is played every day must not leave the
/// cache before a track that was downloaded once and never played again.
///
/// A confirmed hit is the origin re-authorizing the representation and the
/// cached original verifying against it. It is not a listening event: no media
/// time is counted, nothing is reported to the origin and nothing leaves the
/// device.
///
/// The mark is an optimization and never a grant:
///
/// - it reads and writes no audio byte — the cached original is not opened,
///   not read and not written, and only the sidecar's `saved_at` is replaced;
/// - it weakens no check of [`open_verified`]: the origin's validator still
///   decides whether the entry may be used at all, and the cached original is
///   still re-hashed from the very descriptor the decoder receives;
/// - it makes no network request and consults no session.
///
/// `Ok(true)` — the sidecar now records this confirmed hit at the current
/// clock.
/// `Ok(false)` — the sidecar is absent, is of another version or describes
/// another representation: nothing was written at all.
/// `Err` — cancellation only.
pub(crate) fn touch(
    root: &Path,
    key: &CacheKey,
    expected_size: u64,
    expected_etag: &str,
    canceled: &dyn Fn() -> bool,
) -> Result<bool, String> {
    touch_with_clock(
        root,
        key,
        expected_size,
        expected_etag,
        &saved_at_now,
        canceled,
    )
}

/// The mark of a confirmed hit, with the clock that records its label.
///
/// The label of a confirmed hit is the **wall clock of the moment of the
/// mark**, in whole seconds. It is never a value from the future: a label that
/// ran ahead of the clock would make one track look older than a track that
/// was really played later, and the eviction would drop exactly the entry
/// that was played most recently. Two confirmed hits inside the same second
/// therefore record the same label, and the order between them is resolved by
/// the tie-break on `track_id` that [`published`] already applies.
fn touch_with_clock(
    root: &Path,
    key: &CacheKey,
    expected_size: u64,
    expected_etag: &str,
    clock: Clock<'_>,
    canceled: &dyn Fn() -> bool,
) -> Result<bool, String> {
    if canceled() {
        return Err(CANCELED.into());
    }
    if expected_size == 0
        || expected_size > crate::client::MAX_MEDIA_BYTES
        || !crate::client::lower_hex64(expected_etag)
    {
        return Ok(false);
    }
    let record = sidecar_path(root, key);
    let Some(bytes) = read_sidecar_bytes(&record) else {
        return Ok(false);
    };
    let Some(sidecar) = serde_json::from_slice::<Sidecar>(&bytes).ok() else {
        return Ok(false);
    };
    // The record must still describe the very representation the origin
    // confirmed. Another version, another identity, another size, another
    // validator or a digest that is not the origin's validator is another
    // entry, and it is not marked.
    if sidecar.version != VERSION
        || sidecar.origin != key.origin
        || sidecar.account_id != key.account_id
        || sidecar.track_id != key.track_id
        || sidecar.size_bytes != expected_size
        || sidecar.etag != expected_etag
        || sidecar.digest != expected_etag
        || !crate::client::lower_hex64(&sidecar.digest)
        || sidecar.saved_at == 0
    {
        return Ok(false);
    }
    // The cached original must still be the file the record describes: a
    // record without its original describes nothing playable, and a playback
    // that cannot happen must not keep an entry alive.
    if !intact(root, key, sidecar.size_bytes) {
        return Ok(false);
    }
    if canceled() {
        return Err(CANCELED.into());
    }
    let account = account_root(root, key);
    // Unique per operation and per process: a second mark never writes over
    // the staging file of another one.
    let suffix = unique_suffix();
    let staged = account.join(format!(".{}.{suffix}.json.tmp", key.track_id));
    let mut updated = sidecar.clone();
    // The label of a confirmed hit is the clock of the moment of the mark, and
    // never a value from the future: the eviction compares the labels of two
    // tracks against the same wall clock.
    updated.saved_at = clock().min(saved_at_now());
    if let Err(error) = stage_sidecar(&staged, &updated) {
        let _ = fs::remove_file(&staged);
        return Err(error);
    }
    if canceled() {
        let _ = fs::remove_file(&staged);
        return Err(CANCELED.into());
    }
    // The record is replaced only while it still is the record that was read:
    // a publication that replaced it in the meantime wins, and the mark is
    // dropped instead of describing bytes that are no longer there.
    if read_sidecar_bytes(&record).as_deref() != Some(bytes.as_slice())
        || !intact(root, key, sidecar.size_bytes)
    {
        let _ = fs::remove_file(&staged);
        return Ok(false);
    }
    // The commit point: the record is replaced by a rename, so a reader never
    // sees a half-written sidecar, and a failed rename leaves the previous
    // record exactly as it was.
    if fs::rename(&staged, &record).is_err() {
        let _ = fs::remove_file(&staged);
        return Err(WRITE.into());
    }
    Ok(true)
}

/// Whether the cached original of this entry is still the file its record
/// describes. Only metadata is read: no byte of the original is touched.
fn intact(root: &Path, key: &CacheKey, size_bytes: u64) -> bool {
    let Ok(link) = fs::symlink_metadata(audio_path(root, key)) else {
        return false;
    };
    !link.file_type().is_symlink() && link.is_file() && link.len() == size_bytes
}

/// Copy one already verified prepared original into this namespace. Best effort by
/// contract: the caller keeps its own verified lease, so a full disk, a full cache or
/// a cancellation here can never fail or invalidate playback.
pub(crate) fn store(
    root: &Path,
    key: &CacheKey,
    original: &PreparedAudio,
    etag: &str,
    canceled: &dyn Fn() -> bool,
) -> Result<(), String> {
    let source = original.path().to_owned();
    store_from(root, key, &source, original.length_bytes, etag, canceled)
}

fn store_from(
    root: &Path,
    key: &CacheKey,
    source: &Path,
    length_bytes: u64,
    etag: &str,
    canceled: &dyn Fn() -> bool,
) -> Result<(), String> {
    store_at(
        root,
        key,
        source,
        length_bytes,
        etag,
        saved_at_now(),
        Budget::default(),
        canceled,
    )
}

#[allow(clippy::too_many_arguments)]
fn store_at(
    root: &Path,
    key: &CacheKey,
    source: &Path,
    length_bytes: u64,
    etag: &str,
    saved_at: u64,
    budget: Budget,
    canceled: &dyn Fn() -> bool,
) -> Result<(), String> {
    if canceled() {
        return Err(CANCELED.into());
    }
    if length_bytes == 0
        || length_bytes > crate::client::MAX_MEDIA_BYTES
        || !crate::client::lower_hex64(etag)
    {
        return Err(UNAVAILABLE.into());
    }
    let account = account_root(root, key);
    private_directory(&account)?;
    // Unique per operation and per process: a second attempt (or a second process)
    // never writes, publishes or removes another operation's staging file.
    let suffix = unique_suffix();
    let staged_audio = account.join(format!(".{}.{suffix}.audio.tmp", key.track_id));
    let staged_sidecar = account.join(format!(".{}.{suffix}.json.tmp", key.track_id));
    let digest = match copy_verified(source, &staged_audio, length_bytes, canceled) {
        Ok(digest) => digest,
        Err(error) => {
            let _ = fs::remove_file(&staged_audio);
            return Err(error);
        }
    };
    if digest != etag {
        let _ = fs::remove_file(&staged_audio);
        return Err(INTEGRITY.into());
    }
    let sidecar = Sidecar {
        version: VERSION,
        origin: key.origin.clone(),
        account_id: key.account_id.clone(),
        track_id: key.track_id.clone(),
        size_bytes: length_bytes,
        etag: etag.to_owned(),
        digest,
        saved_at,
    };
    let staged = (|| -> Result<(), String> {
        make_room(&account, key, MAX_SIDECAR_BYTES as u64, budget, canceled)?;
        stage_sidecar(&staged_sidecar, &sidecar)?;
        // The commit point: cancellation observed before it removes only this
        // operation's own staging and never an earlier healthy entry.
        if canceled() {
            return Err(CANCELED.into());
        }
        publish(&account, key, &staged_audio, &staged_sidecar, &suffix)
    })();
    if staged.is_err() {
        // Removing a name that a failed publication already moved is a no-op.
        let _ = fs::remove_file(&staged_audio);
        let _ = fs::remove_file(&staged_sidecar);
    }
    staged
}

/// Stream the prepared original into a private staging file, hashing exactly the bytes
/// written. The expected length comes from the caller, never from a cached claim.
fn copy_verified(
    source: &Path,
    staged: &Path,
    expected: u64,
    canceled: &dyn Fn() -> bool,
) -> Result<String, String> {
    let Ok(mut input) = fs::File::open(source) else {
        return Err(UNAVAILABLE.into());
    };
    let Ok(metadata) = input.metadata() else {
        return Err(UNAVAILABLE.into());
    };
    if !metadata.is_file() || metadata.len() != expected {
        return Err(INTEGRITY.into());
    }
    let mut output = private_file(staged).map_err(|_| WRITE.to_owned())?;
    let mut hasher = blake3::Hasher::new();
    let mut buffer = [0_u8; BLOCK_BYTES];
    let mut copied = 0_u64;
    loop {
        if canceled() {
            return Err(CANCELED.into());
        }
        let read = input
            .read(&mut buffer)
            .map_err(|_| UNAVAILABLE.to_owned())?;
        if read == 0 {
            break;
        }
        copied = copied.saturating_add(read as u64);
        if copied > expected {
            return Err(INTEGRITY.into());
        }
        hasher.update(&buffer[..read]);
        output
            .write_all(&buffer[..read])
            .map_err(|_| WRITE.to_owned())?;
        // Test-only: the first copied block is on disk, so a test can cancel a
        // copy that has really started instead of one that never began.
        #[cfg(test)]
        if copied == BLOCK_BYTES as u64 {
            copy_gate::hold_after_first_block(staged);
        }
    }
    if copied != expected {
        return Err(INTEGRITY.into());
    }
    output.sync_all().map_err(|_| WRITE.to_owned())?;
    Ok(hasher.finalize().to_hex().to_string())
}

/// Test-only gate inside a running cache copy.
///
/// A copy that is canceled before it begins proves nothing about cancelling a
/// copy that is already writing: the client test that asserts "a sign-out leaves
/// no staging file" is vacuous while the copy never starts. This gate holds a
/// copy right after its first copied block is on disk, so the test can cancel a
/// transfer that has really begun.
#[cfg(test)]
pub(crate) mod copy_gate {
    use std::{
        path::{Path, PathBuf},
        sync::{
            Arc, Condvar, LazyLock, Mutex, MutexGuard,
            atomic::{AtomicUsize, Ordering},
        },
    };

    /// One installed gate. It is global because `copy_verified` runs on a
    /// blocking worker without a client handle; one test at a time installs it,
    /// which the section (`exclusive`) enforces. The gate only holds copies
    /// below `root`, so tests that run in parallel with their own private cache
    /// root are never held.
    pub(crate) struct CopyGate {
        root: PathBuf,
        entered: AtomicUsize,
        released: Mutex<bool>,
        signal: Condvar,
    }

    impl CopyGate {
        pub(crate) fn new(root: &Path) -> Arc<Self> {
            Arc::new(Self {
                root: root.to_owned(),
                entered: AtomicUsize::new(0),
                released: Mutex::new(false),
                signal: Condvar::new(),
            })
        }
        /// How many copies below this gate's root reached the gate.
        pub(crate) fn entered(&self) -> usize {
            self.entered.load(Ordering::SeqCst)
        }
        /// Let a held copy continue.
        pub(crate) fn release(&self) {
            if let Ok(mut released) = self.released.lock() {
                *released = true;
                self.signal.notify_all();
            }
        }
    }

    static INSTALLED: LazyLock<Mutex<Option<Arc<CopyGate>>>> = LazyLock::new(|| Mutex::new(None));

    /// The section lock. The gate slot is global because `copy_verified` runs
    /// on a blocking worker without a client handle, so the tests that install
    /// a gate must never hold it at the same time.
    static SECTION: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

    /// The exclusive right to install the global gate.
    ///
    /// Entering the section takes the process-wide section lock, saves the
    /// current slot value and empties the slot, so the test inside the section
    /// starts from a clean state. Dropping the guard restores the saved value
    /// and releases the lock — also when the test panics, so a red test never
    /// leaves the slot or the lock behind.
    pub(crate) struct Exclusive {
        saved: Option<Arc<CopyGate>>,
        _section: MutexGuard<'static, ()>,
    }

    impl Exclusive {
        /// The slot value observed when this section was entered.
        ///
        /// A section that ends restores this value, so the baseline of the next
        /// section is `None` unless a section leaked a gate.
        pub(crate) fn baseline(&self) -> Option<Arc<CopyGate>> {
            self.saved.clone()
        }
    }

    impl Drop for Exclusive {
        fn drop(&mut self) {
            // The slot is restored *before* the section lock is released: the
            // next section observes the restored slot, not a foreign one.
            if let Ok(mut slot) = INSTALLED.lock() {
                *slot = self.saved.take();
            }
        }
    }

    /// Take the exclusive right to install the global gate.
    ///
    /// A poisoned section lock is recovered instead of propagated: the lock
    /// guards no state of its own, and the slot it protects is restored by
    /// `Drop` even when the test panics. The section is not re-entrant: a
    /// second call in the same thread blocks forever, so one guard per test.
    pub(crate) fn exclusive() -> Exclusive {
        let section = SECTION
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let saved = INSTALLED.lock().ok().and_then(|mut slot| slot.take());
        Exclusive {
            saved,
            _section: section,
        }
    }

    /// Whether a section is held right now.
    ///
    /// A test uses it to prove that the section it found was really taken
    /// instead of merely free.
    pub(crate) fn section_is_held() -> bool {
        match SECTION.try_lock() {
            Ok(_) => false,
            Err(std::sync::TryLockError::WouldBlock) => true,
            Err(std::sync::TryLockError::Poisoned(_)) => true,
        }
    }

    /// Install a gate for the current test and return the previous one.
    ///
    /// Only this module and its sibling tests install gates directly; a test
    /// outside them uses `install_scoped`, and all of them are meant to run
    /// inside a section.
    pub(super) fn install(gate: Arc<CopyGate>) -> Option<Arc<CopyGate>> {
        INSTALLED
            .lock()
            .ok()
            .and_then(|mut slot| slot.replace(gate))
    }

    /// Remove *this* gate only if the slot still holds it, restoring the
    /// previous one. A guard whose gate was already replaced by another test
    /// (the suite runs with more than one thread) must not steal the slot of a
    /// gate it does not own.
    fn restore_owned(gate: &Arc<CopyGate>, previous: Option<Arc<CopyGate>>) {
        if let Ok(mut slot) = INSTALLED.lock() {
            let owned = slot
                .as_ref()
                .is_some_and(|current| Arc::ptr_eq(current, gate));
            if owned {
                *slot = previous;
            }
        }
    }

    /// The installed gate, removed again when this value is dropped — including
    /// when the test panics.
    ///
    /// Dropping also *releases* the gate: a test that panics while a copy is
    /// held would otherwise leave the blocking worker parked in `Condvar::wait`
    /// and hang the whole test binary instead of reporting one red test.
    pub(crate) struct Installed {
        gate: Option<Arc<CopyGate>>,
        previous: Option<Arc<CopyGate>>,
    }

    impl Drop for Installed {
        fn drop(&mut self) {
            let Some(gate) = self.gate.take() else {
                return;
            };
            gate.release();
            restore_owned(&gate, self.previous.take());
        }
    }

    /// Install a gate for the duration of the returned guard.
    pub(crate) fn install_scoped(gate: Arc<CopyGate>) -> Installed {
        let previous = install(gate.clone());
        Installed {
            gate: Some(gate),
            previous,
        }
    }

    /// The installed gate, for a test that asserts which gate owns the slot.
    pub(crate) fn installed() -> Option<Arc<CopyGate>> {
        INSTALLED.lock().ok().and_then(|slot| slot.clone())
    }

    /// Hold the running copy until the gate is released. Called with the first
    /// copied block already written to the staging file, so the staging file
    /// really exists while the test cancels the copy.
    pub(crate) fn hold_after_first_block(staged: &Path) {
        let Some(gate) = INSTALLED
            .lock()
            .ok()
            .and_then(|slot| slot.as_ref().cloned())
        else {
            return;
        };
        if !staged.starts_with(&gate.root) {
            return;
        }
        gate.entered.fetch_add(1, Ordering::SeqCst);
        let Ok(mut released) = gate.released.lock() else {
            return;
        };
        while !*released {
            let Ok(next) = gate.signal.wait(released) else {
                return;
            };
            released = next;
        }
    }
}

/// Evict this cache's own oldest published pairs until the namespace fits. Staging and
/// orphan bytes are counted but never deleted here, so no in-flight operation of this
/// or another process loses its file; a namespace that cannot fit refuses cleanly and
/// deletes nothing.
fn make_room(
    account: &Path,
    key: &CacheKey,
    sidecar_reservation: u64,
    budget: Budget,
    canceled: &dyn Fn() -> bool,
) -> Result<(), String> {
    if canceled() {
        return Err(CANCELED.into());
    }
    // Our own staging file is already on disk and deliberately counted here.
    let mut bytes = measure(account)?.saturating_add(sidecar_reservation);
    let mut candidates = published(account)?;
    // The entry being replaced is neither counted twice nor evicted by this operation.
    candidates.retain(|entry| entry.track_id != key.track_id);
    let mut entries = candidates.len().saturating_add(1);
    let mut doomed = Vec::new();
    while entries > budget.entries || bytes > budget.bytes {
        if canceled() {
            return Err(CANCELED.into());
        }
        let Some(oldest) = candidates.first().cloned() else {
            return Err(FULL.into());
        };
        bytes = bytes.saturating_sub(oldest.bytes);
        entries = entries.saturating_sub(1);
        candidates.remove(0);
        doomed.push(oldest);
    }
    if doomed.is_empty() {
        return Ok(());
    }
    if canceled() {
        return Err(CANCELED.into());
    }
    for entry in doomed {
        remove_pair(account, &entry.track_id);
    }
    Ok(())
}

/// Every `<uuid>.json` sidecar in this namespace with its audio size, oldest first.
///
/// The order is the recency label of the entry, then its track id. The
/// tie-break matters because the label of a confirmed hit is a one-second
/// clock: two entries confirmed inside the same second carry the same label,
/// and the track id keeps their order stable and reproducible.
fn published(account: &Path) -> Result<Vec<Entry>, String> {
    let entries = match fs::read_dir(account) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(_) => return Err(MEASURE.into()),
    };
    let mut published = Vec::new();
    for entry in entries.filter_map(Result::ok).take(MAX_DIRECTORY_ENTRIES) {
        let name = entry.file_name();
        let Some(stem) = name.to_str().and_then(|name| name.strip_suffix(".json")) else {
            continue;
        };
        if !crate::client::canonical_uuid(stem) {
            continue;
        }
        // An unreadable sidecar is the oldest possible entry, so a corrupt or foreign
        // sidecar can never protect itself from eviction.
        let saved_at = read_sidecar(&entry.path()).map_or(0, |sidecar| sidecar.saved_at);
        let audio = account.join(format!("{stem}.audio"));
        let mut bytes = file_bytes(&entry.path());
        if let Ok(metadata) = fs::metadata(&audio)
            && metadata.is_file()
        {
            bytes = bytes.saturating_add(metadata.len());
        }
        published.push(Entry {
            track_id: stem.to_owned(),
            saved_at,
            bytes,
        });
    }
    published.sort_by(|left, right| {
        left.saved_at
            .cmp(&right.saved_at)
            .then_with(|| left.track_id.cmp(&right.track_id))
    });
    Ok(published)
}

/// Delete one published pair by its own two names only. Offline/pinned copies live
/// under a different root, and staging/orphan names never match these two.
fn remove_pair(account: &Path, track_id: &str) {
    let _ = fs::remove_file(account.join(format!("{track_id}.audio")));
    let _ = fs::remove_file(account.join(format!("{track_id}.json")));
}

/// Everything this namespace already holds: published pairs, this crate's staging
/// leftovers and orphans nothing references. Accounting only measures.
fn measure(account: &Path) -> Result<u64, String> {
    let entries = match fs::read_dir(account) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(0),
        Err(_) => return Err(MEASURE.into()),
    };
    let mut total = 0_u64;
    for entry in entries.filter_map(Result::ok).take(MAX_DIRECTORY_ENTRIES) {
        total = total.saturating_add(file_bytes(&entry.path()));
    }
    Ok(total)
}

/// A regular file's size. Symlinks and directories are deliberately zero, so a link
/// cannot make the cache look larger than the files it owns.
fn file_bytes(path: &Path) -> u64 {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_file() => metadata.len(),
        _ => 0,
    }
}

/// The raw bytes of one sidecar, read under the same guards as the record
/// itself: a symlink, a directory, an oversized or an unreadable file is no
/// sidecar at all.
fn read_sidecar_bytes(path: &Path) -> Option<Vec<u8>> {
    let metadata = fs::symlink_metadata(path).ok()?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return None;
    }
    if metadata.len() > MAX_SIDECAR_BYTES as u64 {
        return None;
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    fs::File::open(path)
        .ok()?
        .take(MAX_SIDECAR_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .ok()?;
    if bytes.len() > MAX_SIDECAR_BYTES {
        return None;
    }
    Some(bytes)
}

fn read_sidecar(path: &Path) -> Option<Sidecar> {
    serde_json::from_slice(&read_sidecar_bytes(path)?).ok()
}

fn stage_sidecar(path: &Path, sidecar: &Sidecar) -> Result<(), String> {
    let payload = serde_json::to_vec(sidecar).map_err(|_| WRITE.to_owned())?;
    if payload.len() > MAX_SIDECAR_BYTES {
        return Err(WRITE.into());
    }
    let mut file = private_file(path).map_err(|_| WRITE.to_owned())?;
    file.write_all(&payload)
        .and_then(|()| file.sync_all())
        .map_err(|_| WRITE.to_owned())
}

/// Publish a staged pair. The previous pair is renamed aside first, so a failed
/// publication restores the last healthy entry instead of destroying it. The sidecar
/// is published last: an interruption between the two renames leaves audio whose
/// digest no longer matches the old sidecar, which `open_verified` rejects.
fn publish(
    account: &Path,
    key: &CacheKey,
    staged_audio: &Path,
    staged_sidecar: &Path,
    suffix: &str,
) -> Result<(), String> {
    let track_id = key.track_id();
    let audio = account.join(format!("{track_id}.audio"));
    let sidecar = account.join(format!("{track_id}.json"));
    let previous_audio = account.join(format!(".{track_id}.{suffix}.audio.bak"));
    let previous_sidecar = account.join(format!(".{track_id}.{suffix}.json.bak"));
    let moved_audio = match fs::rename(&audio, &previous_audio) {
        Ok(()) => true,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
        Err(_) => return Err(WRITE.into()),
    };
    let moved_sidecar = match fs::rename(&sidecar, &previous_sidecar) {
        Ok(()) => true,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
        Err(_) => {
            if moved_audio {
                let _ = fs::rename(&previous_audio, &audio);
            }
            return Err(WRITE.into());
        }
    };
    let restore = |published_audio: bool| {
        if published_audio {
            let _ = fs::remove_file(&audio);
        }
        if moved_audio {
            let _ = fs::rename(&previous_audio, &audio);
        }
        if moved_sidecar {
            let _ = fs::rename(&previous_sidecar, &sidecar);
        }
    };
    if fs::rename(staged_audio, &audio).is_err() {
        restore(false);
        let _ = fs::remove_file(staged_sidecar);
        return Err(WRITE.into());
    }
    if fs::rename(staged_sidecar, &sidecar).is_err() {
        restore(true);
        let _ = fs::remove_file(staged_sidecar);
        return Err(WRITE.into());
    }
    // Only this operation's own backups are removed: the published replacement is
    // complete, and no file this operation did not create is touched.
    if moved_audio {
        let _ = fs::remove_file(&previous_audio);
    }
    if moved_sidecar {
        let _ = fs::remove_file(&previous_sidecar);
    }
    Ok(())
}

fn private_directory(path: &Path) -> Result<(), String> {
    let mut builder = fs::DirBuilder::new();
    builder.recursive(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder.create(path).map_err(|_| WRITE.to_owned())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))
            .map_err(|_| WRITE.to_owned())?;
    }
    Ok(())
}

fn private_file(path: &Path) -> std::io::Result<fs::File> {
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options.open(path)
}

fn unique_suffix() -> String {
    format!("{}-{}", std::process::id(), uuid::Uuid::new_v4().simple())
}

/// The wall clock of the recency label, in whole seconds since the Unix epoch.
///
/// The label of a confirmed hit is this clock, read at the moment of the mark.
/// It is never a value from the future: a label that ran ahead of the clock
/// would make one track look older than a track that was really played later.
pub(crate) fn saved_at_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(1)
        .max(1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    const ORIGIN: &str = "https://music.example.org/";
    const OTHER_ORIGIN: &str = "https://other.example.org/";
    const ACCOUNT: &str = "10000000-0000-4000-8000-000000000001";
    const OTHER_ACCOUNT: &str = "10000000-0000-4000-8000-000000000002";
    const TRACK: &str = "20000000-0000-4000-8000-000000000001";
    const OTHER_TRACK: &str = "20000000-0000-4000-8000-000000000002";
    /// A recency label of the past: an entry that was published long ago, so a
    /// confirmed hit is observed to move the label to the current clock
    /// without a sleep.
    const AGED_RECENCY: u64 = 1_000_000;

    fn key(origin: &str, account: &str, track: &str) -> CacheKey {
        CacheKey::new(origin, account, track).unwrap()
    }

    fn never() -> impl Fn() -> bool {
        || false
    }

    fn digest(bytes: &[u8]) -> String {
        blake3::hash(bytes).to_hex().to_string()
    }

    /// A private source file of exactly `bytes`, as a verified lease would provide.
    fn original(bytes: &[u8]) -> (tempfile::TempDir, PathBuf) {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("original.audio");
        fs::write(&path, bytes).unwrap();
        (directory, path)
    }

    fn publish_bytes(
        root: &Path,
        key: &CacheKey,
        bytes: &[u8],
        saved_at: u64,
    ) -> Result<(), String> {
        let (_directory, source) = original(bytes);
        store_at(
            root,
            key,
            &source,
            bytes.len() as u64,
            &digest(bytes),
            saved_at,
            Budget::default(),
            &never(),
        )
    }

    fn names(root: &Path, key: &CacheKey) -> Vec<String> {
        let mut names = fs::read_dir(account_root(root, key))
            .map(|entries| {
                entries
                    .filter_map(Result::ok)
                    .map(|entry| entry.file_name().to_string_lossy().into_owned())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        names.sort();
        names
    }

    fn read_cached(cached: Cached) -> (Vec<u8>, u64) {
        let length = cached.length_bytes;
        let mut file = cached.file;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes).unwrap();
        (bytes, length)
    }

    fn clone_sidecar(sidecar: &Sidecar) -> Sidecar {
        Sidecar {
            version: sidecar.version,
            origin: sidecar.origin.clone(),
            account_id: sidecar.account_id.clone(),
            track_id: sidecar.track_id.clone(),
            size_bytes: sidecar.size_bytes,
            etag: sidecar.etag.clone(),
            digest: sidecar.digest.clone(),
            saved_at: sidecar.saved_at,
        }
    }

    /// The recency the sidecar of one published entry carries.
    fn sidecar_saved_at(root: &Path, key: &CacheKey) -> u64 {
        read_sidecar(&sidecar_path(root, key))
            .expect("the sidecar of a published entry")
            .saved_at
    }

    /// The identity of a file as the operating system sees it: the same bytes
    /// under the same name are the same file only when the device, the inode,
    /// the length and the modification time are the same.
    #[cfg(unix)]
    fn identity(path: &Path) -> (u64, u64, u64, std::time::SystemTime) {
        use std::os::unix::fs::MetadataExt;
        let metadata = fs::metadata(path).unwrap();
        (
            metadata.dev(),
            metadata.ino(),
            metadata.len(),
            metadata.modified().unwrap(),
        )
    }

    #[test]
    fn cache_paths_isolate_origins_accounts_and_tracks() {
        let root = Path::new("/private/cache/vela/media");
        let base = key(ORIGIN, ACCOUNT, TRACK);
        assert!(crate::client::lower_hex64(&origin_hash(ORIGIN)));
        assert_eq!(
            account_root(root, &base),
            root.join(origin_hash(ORIGIN)).join(ACCOUNT)
        );
        assert_eq!(
            audio_path(root, &base),
            root.join(origin_hash(ORIGIN))
                .join(ACCOUNT)
                .join(format!("{TRACK}.audio"))
        );
        assert_eq!(
            sidecar_path(root, &base),
            root.join(origin_hash(ORIGIN))
                .join(ACCOUNT)
                .join(format!("{TRACK}.json"))
        );
        // One account UUID and one track UUID on two origins never share a path.
        assert_ne!(origin_hash(ORIGIN), origin_hash(OTHER_ORIGIN));
        assert_ne!(
            account_root(root, &base),
            account_root(root, &key(OTHER_ORIGIN, ACCOUNT, TRACK))
        );
        assert_ne!(
            account_root(root, &base),
            account_root(root, &key(ORIGIN, OTHER_ACCOUNT, TRACK))
        );
        assert_ne!(
            audio_path(root, &base),
            audio_path(root, &key(ORIGIN, ACCOUNT, OTHER_TRACK))
        );
        // A path is a pure function of the canonical identity.
        assert_eq!(audio_path(root, &base), audio_path(root, &base.clone()));
        for invalid in [
            CacheKey::new("", ACCOUNT, TRACK),
            CacheKey::new(ORIGIN, "../escape", TRACK),
            CacheKey::new(ORIGIN, ACCOUNT, "../../private"),
            CacheKey::new(&format!("{ORIGIN}\ncontrol"), ACCOUNT, TRACK),
            CacheKey::new(ORIGIN, &"a".repeat(64), TRACK),
        ] {
            assert!(invalid.is_err(), "an unusable cache identity was accepted");
        }
    }

    #[test]
    fn cached_original_hits_only_for_the_same_origin_account_and_representation() {
        let cache = tempfile::tempdir().unwrap();
        let bytes = b"complete-origin-fixture".to_vec();
        let etag = digest(&bytes);
        let published = key(ORIGIN, ACCOUNT, TRACK);
        publish_bytes(cache.path(), &published, &bytes, 1).unwrap();
        assert_eq!(names(cache.path(), &published).len(), 2);

        let hit = open_verified(
            cache.path(),
            &published,
            bytes.len() as u64,
            &etag,
            &never(),
        )
        .unwrap()
        .expect("an intact entry must be reusable");
        assert_eq!(hit.length_bytes, bytes.len() as u64);
        // The returned descriptor is rewound, so the decoder reads from byte zero.
        let (read, length) = read_cached(hit);
        assert_eq!(length, bytes.len() as u64);
        assert_eq!(read, bytes, "the hit must return the verified bytes");

        for stranger in [
            key(OTHER_ORIGIN, ACCOUNT, TRACK),
            key(ORIGIN, OTHER_ACCOUNT, TRACK),
            key(ORIGIN, ACCOUNT, OTHER_TRACK),
        ] {
            assert!(
                open_verified(cache.path(), &stranger, bytes.len() as u64, &etag, &never())
                    .unwrap()
                    .is_none(),
                "another origin/account/track must not see this entry"
            );
        }
        // Another authoritative validator is another representation.
        assert!(
            open_verified(
                cache.path(),
                &published,
                bytes.len() as u64,
                &"0".repeat(64),
                &never()
            )
            .unwrap()
            .is_none()
        );
        // A size the origin did not confirm is never matched against local bytes.
        assert!(
            open_verified(
                cache.path(),
                &published,
                bytes.len() as u64 + 1,
                &etag,
                &never()
            )
            .unwrap()
            .is_none()
        );
    }

    #[test]
    fn sidecar_mismatch_and_version_drift_are_misses() {
        let cache = tempfile::tempdir().unwrap();
        let bytes = b"sidecar-validation-fixture".to_vec();
        let etag = digest(&bytes);
        let key = key(ORIGIN, ACCOUNT, TRACK);
        publish_bytes(cache.path(), &key, &bytes, 7).unwrap();
        let path = sidecar_path(cache.path(), &key);
        let valid: Sidecar = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        assert_eq!(valid.version, VERSION);
        assert_eq!(valid.origin, ORIGIN);
        assert_eq!(valid.account_id, ACCOUNT);
        assert_eq!(valid.track_id, TRACK);
        assert_eq!(valid.size_bytes, bytes.len() as u64);
        assert_eq!(valid.etag, etag);
        assert_eq!(valid.digest, etag);
        assert_eq!(valid.saved_at, 7);

        let cases: Vec<(&str, Sidecar)> = vec![
            (
                "version",
                Sidecar {
                    version: 2,
                    ..clone_sidecar(&valid)
                },
            ),
            (
                "origin",
                Sidecar {
                    origin: OTHER_ORIGIN.into(),
                    ..clone_sidecar(&valid)
                },
            ),
            (
                "account",
                Sidecar {
                    account_id: OTHER_ACCOUNT.into(),
                    ..clone_sidecar(&valid)
                },
            ),
            (
                "track",
                Sidecar {
                    track_id: OTHER_TRACK.into(),
                    ..clone_sidecar(&valid)
                },
            ),
            (
                "size",
                Sidecar {
                    size_bytes: valid.size_bytes + 1,
                    ..clone_sidecar(&valid)
                },
            ),
            (
                "etag",
                Sidecar {
                    etag: "1".repeat(64),
                    ..clone_sidecar(&valid)
                },
            ),
            (
                "digest",
                Sidecar {
                    digest: "2".repeat(64),
                    ..clone_sidecar(&valid)
                },
            ),
            (
                "unsaved",
                Sidecar {
                    saved_at: 0,
                    ..clone_sidecar(&valid)
                },
            ),
        ];
        for (case, record) in cases {
            fs::write(&path, serde_json::to_vec(&record).unwrap()).unwrap();
            assert!(
                open_verified(cache.path(), &key, bytes.len() as u64, &etag, &never())
                    .unwrap()
                    .is_none(),
                "a {case} mismatch must not become a hit"
            );
        }
        // Unparsable, unknown-field and oversized sidecars are misses as well.
        for payload in [
            b"not json".to_vec(),
            serde_json::to_vec(&serde_json::json!({
                "version": VERSION,
                "origin": ORIGIN,
                "account_id": ACCOUNT,
                "track_id": TRACK,
                "size_bytes": bytes.len(),
                "etag": etag,
                "digest": etag,
                "saved_at": 1,
                "unexpected": "field",
            }))
            .unwrap(),
            vec![b' '; MAX_SIDECAR_BYTES + 1],
        ] {
            fs::write(&path, payload).unwrap();
            assert!(
                open_verified(cache.path(), &key, bytes.len() as u64, &etag, &never())
                    .unwrap()
                    .is_none()
            );
        }
        fs::remove_file(&path).unwrap();
        assert!(
            open_verified(cache.path(), &key, bytes.len() as u64, &etag, &never())
                .unwrap()
                .is_none(),
            "a missing sidecar is a miss"
        );
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_sidecar_or_original_is_never_a_hit() {
        let cache = tempfile::tempdir().unwrap();
        let bytes = b"symlink-fixture".to_vec();
        let etag = digest(&bytes);
        let key = key(ORIGIN, ACCOUNT, TRACK);
        publish_bytes(cache.path(), &key, &bytes, 1).unwrap();
        let audio = audio_path(cache.path(), &key);
        let sidecar = sidecar_path(cache.path(), &key);
        let elsewhere = cache.path().join("elsewhere");
        fs::create_dir(&elsewhere).unwrap();

        let moved = elsewhere.join("real.audio");
        fs::rename(&audio, &moved).unwrap();
        std::os::unix::fs::symlink(&moved, &audio).unwrap();
        assert!(
            open_verified(cache.path(), &key, bytes.len() as u64, &etag, &never())
                .unwrap()
                .is_none(),
            "a symlinked original is not a cache hit"
        );
        fs::remove_file(&audio).unwrap();
        fs::rename(&moved, &audio).unwrap();

        let moved = elsewhere.join("real.json");
        fs::rename(&sidecar, &moved).unwrap();
        std::os::unix::fs::symlink(&moved, &sidecar).unwrap();
        assert!(
            open_verified(cache.path(), &key, bytes.len() as u64, &etag, &never())
                .unwrap()
                .is_none(),
            "a symlinked sidecar is not a cache record"
        );
    }

    #[test]
    fn same_size_tampering_is_rejected_by_the_local_digest() {
        let cache = tempfile::tempdir().unwrap();
        let bytes = vec![3_u8; 64 * 1024 + 17];
        let etag = digest(&bytes);
        let key = key(ORIGIN, ACCOUNT, TRACK);
        publish_bytes(cache.path(), &key, &bytes, 1).unwrap();
        let audio = audio_path(cache.path(), &key);
        let mut tampered = bytes.clone();
        tampered[1234] ^= 0xff;
        assert_eq!(tampered.len(), bytes.len(), "tampering keeps the byte size");
        fs::write(&audio, &tampered).unwrap();
        assert!(
            open_verified(cache.path(), &key, bytes.len() as u64, &etag, &never())
                .unwrap()
                .is_none(),
            "same-size tampering must never be served"
        );
        // Rewriting the sidecar to describe the tampered bytes still cannot forge a
        // hit, because only the origin's ETag is authoritative.
        let path = sidecar_path(cache.path(), &key);
        let mut record = clone_sidecar(&serde_json::from_slice(&fs::read(&path).unwrap()).unwrap());
        record.digest = digest(&tampered);
        fs::write(&path, serde_json::to_vec(&record).unwrap()).unwrap();
        assert!(
            open_verified(cache.path(), &key, bytes.len() as u64, &etag, &never())
                .unwrap()
                .is_none()
        );
        // A truncated file is a size mismatch, not a hit.
        fs::write(&audio, &bytes[..bytes.len() - 1]).unwrap();
        assert!(
            open_verified(cache.path(), &key, bytes.len() as u64, &etag, &never())
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn failed_publication_keeps_the_previous_healthy_entry() {
        let cache = tempfile::tempdir().unwrap();
        let healthy = b"previously-verified-original".to_vec();
        let healthy_etag = digest(&healthy);
        let key = key(ORIGIN, ACCOUNT, TRACK);
        publish_bytes(cache.path(), &key, &healthy, 1).unwrap();
        let account = account_root(cache.path(), &key);
        let missing = account.join(".missing.1-aa.audio.tmp");

        // The staged audio is gone: publication fails at the audio rename.
        let replacement = b"new-representation".to_vec();
        let staged_sidecar = account.join(".replacement.1-aa.json.tmp");
        stage_sidecar(&staged_sidecar, &valid_sidecar(&replacement)).unwrap();
        assert!(
            publish(&account, &key, &missing, &staged_sidecar, "1-aa").is_err(),
            "a publication without staged audio must fail"
        );
        assert_eq!(
            read_cached(
                open_verified(
                    cache.path(),
                    &key,
                    healthy.len() as u64,
                    &healthy_etag,
                    &never()
                )
                .unwrap()
                .expect("the previous healthy entry must survive a failed rename")
            )
            .0,
            healthy
        );

        // The audio publishes but the sidecar rename fails: the new audio is rolled
        // back so no audio is ever left described by nothing.
        let staged_audio = account.join(".replacement.1-bb.audio.tmp");
        fs::write(&staged_audio, &replacement).unwrap();
        let missing_sidecar = account.join(".missing.1-bb.json.tmp");
        assert!(
            publish(&account, &key, &staged_audio, &missing_sidecar, "1-bb").is_err(),
            "a publication without a staged sidecar must fail"
        );
        assert_eq!(
            read_cached(
                open_verified(
                    cache.path(),
                    &key,
                    healthy.len() as u64,
                    &healthy_etag,
                    &never()
                )
                .unwrap()
                .expect("a half-published replacement must not destroy the healthy entry")
            )
            .0,
            healthy
        );
        assert!(
            names(cache.path(), &key)
                .iter()
                .all(|name| !name.ends_with(".bak")),
            "a completed restoration leaves no backup behind"
        );
        assert!(
            names(cache.path(), &key)
                .iter()
                .all(|name| name == &format!("{TRACK}.audio") || name == &format!("{TRACK}.json")),
            "only the healthy pair remains"
        );
    }

    fn valid_sidecar(bytes: &[u8]) -> Sidecar {
        Sidecar {
            version: VERSION,
            origin: ORIGIN.into(),
            account_id: ACCOUNT.into(),
            track_id: TRACK.into(),
            size_bytes: bytes.len() as u64,
            etag: digest(bytes),
            digest: digest(bytes),
            saved_at: 2,
        }
    }

    #[test]
    fn publication_replaces_an_entry_and_writes_private_modes() {
        let cache = tempfile::tempdir().unwrap();
        let key = key(ORIGIN, ACCOUNT, TRACK);
        let first = b"first-representation".to_vec();
        publish_bytes(cache.path(), &key, &first, 1).unwrap();
        let second = b"second-representation-with-other-bytes".to_vec();
        publish_bytes(cache.path(), &key, &second, 2).unwrap();
        assert_eq!(names(cache.path(), &key).len(), 2);
        assert_eq!(
            read_cached(
                open_verified(
                    cache.path(),
                    &key,
                    second.len() as u64,
                    &digest(&second),
                    &never()
                )
                .unwrap()
                .expect("the replacement must be the new representation")
            )
            .0,
            second
        );
        assert!(
            open_verified(
                cache.path(),
                &key,
                first.len() as u64,
                &digest(&first),
                &never()
            )
            .unwrap()
            .is_none(),
            "the replaced representation must not be reusable"
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = |path: &Path| fs::metadata(path).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode(&account_root(cache.path(), &key)), 0o700);
            assert_eq!(mode(&audio_path(cache.path(), &key)), 0o600);
            assert_eq!(mode(&sidecar_path(cache.path(), &key)), 0o600);
        }
    }

    #[test]
    fn quota_eviction_removes_only_the_oldest_own_cache_pairs() {
        let cache = tempfile::tempdir().unwrap();
        let oldest = key(ORIGIN, ACCOUNT, "20000000-0000-4000-8000-00000000000a");
        let middle = key(ORIGIN, ACCOUNT, "20000000-0000-4000-8000-00000000000b");
        let newest = key(ORIGIN, ACCOUNT, "20000000-0000-4000-8000-00000000000c");
        for (entry, saved_at) in [(&middle, 20_u64), (&oldest, 10), (&newest, 30)] {
            publish_bytes(cache.path(), entry, b"cached-original", saved_at).unwrap();
        }
        let account = account_root(cache.path(), &oldest);
        // Names this cache must never delete: an unrelated file, a leftover staging
        // name of another operation and an orphan audio without a sidecar.
        let unrelated = account.join("notes.txt");
        let foreign_staging = account.join(".20000000-0000-4000-8000-00000000000d.1-aa.audio.tmp");
        let orphan = account.join("20000000-0000-4000-8000-00000000000e.audio");
        for path in [&unrelated, &foreign_staging, &orphan] {
            fs::write(path, b"not-a-cache-entry").unwrap();
        }

        let target = key(ORIGIN, ACCOUNT, "20000000-0000-4000-8000-00000000000f");
        let (_directory, source) = original(b"a-new-cached-original");
        store_at(
            cache.path(),
            &target,
            &source,
            b"a-new-cached-original".len() as u64,
            &digest(b"a-new-cached-original"),
            40,
            // Three entries fit: publishing the fourth must evict exactly the oldest.
            Budget {
                entries: 3,
                bytes: MAX_BYTES,
            },
            &never(),
        )
        .unwrap();

        assert!(
            open_verified(
                cache.path(),
                &oldest,
                15,
                &digest(b"cached-original"),
                &never()
            )
            .unwrap()
            .is_none(),
            "the oldest sidecar's pair must be evicted first"
        );
        assert!(!audio_path(cache.path(), &oldest).exists());
        assert!(!sidecar_path(cache.path(), &oldest).exists());
        for survivor in [&middle, &newest] {
            assert!(
                open_verified(
                    cache.path(),
                    survivor,
                    15,
                    &digest(b"cached-original"),
                    &never()
                )
                .unwrap()
                .is_some(),
                "a newer entry must survive quota eviction"
            );
        }
        assert_eq!(
            read_cached(
                open_verified(
                    cache.path(),
                    &target,
                    b"a-new-cached-original".len() as u64,
                    &digest(b"a-new-cached-original"),
                    &never()
                )
                .unwrap()
                .expect("the freshly published entry must be reusable")
            )
            .0,
            b"a-new-cached-original".to_vec()
        );
        assert!(unrelated.exists(), "unrelated files are never evicted");
        assert!(
            foreign_staging.exists(),
            "another operation's staging file is counted but never deleted"
        );
        assert!(
            orphan.exists(),
            "an orphan without a sidecar is not assumed to be evictable"
        );
        assert_eq!(published(&account).unwrap().len(), 3);
    }

    /// A confirmed hit refreshes the recency of the entry and touches nothing
    /// else: the cached original keeps its bytes, its size, its inode and its
    /// modification time.
    #[cfg(unix)]
    #[test]
    fn a_confirmed_hit_refreshes_the_recency_without_touching_the_audio() {
        let cache = tempfile::tempdir().unwrap();
        let bytes = b"recency-refresh-fixture".to_vec();
        let etag = digest(&bytes);
        let key = key(ORIGIN, ACCOUNT, TRACK);
        // The entry was published with an older clock: a confirmed hit moves
        // the recency to the current clock, so the value grows by
        // construction and the growth is observed without a sleep.
        publish_bytes(cache.path(), &key, &bytes, AGED_RECENCY).unwrap();
        let audio = audio_path(cache.path(), &key);
        let before = identity(&audio);
        let recency = sidecar_saved_at(cache.path(), &key);

        assert!(
            touch(cache.path(), &key, bytes.len() as u64, &etag, &never()).unwrap(),
            "a confirmed hit must refresh the recency of the entry"
        );

        assert_eq!(
            before,
            identity(&audio),
            "the cached original is not touched by a recency mark"
        );
        assert!(
            sidecar_saved_at(cache.path(), &key) > recency,
            "the recency of a confirmed hit must grow"
        );
        assert!(
            sidecar_saved_at(cache.path(), &key) <= saved_at_now(),
            "the recency of a confirmed hit is the current clock, never a value from the future"
        );
        // The record still describes the very same representation.
        let record = read_sidecar(&sidecar_path(cache.path(), &key)).unwrap();
        assert_eq!(record.version, VERSION);
        assert_eq!(record.origin, ORIGIN);
        assert_eq!(record.account_id, ACCOUNT);
        assert_eq!(record.track_id, TRACK);
        assert_eq!(record.size_bytes, bytes.len() as u64);
        assert_eq!(record.etag, etag);
        assert_eq!(record.digest, etag);
        // The namespace holds the published pair and nothing else.
        assert_eq!(
            names(cache.path(), &key),
            vec![format!("{TRACK}.audio"), format!("{TRACK}.json")]
        );
        // The private modes of the namespace and of the record are kept.
        use std::os::unix::fs::PermissionsExt;
        let mode = |path: &Path| fs::metadata(path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode(&account_root(cache.path(), &key)), 0o700);
        assert_eq!(mode(&sidecar_path(cache.path(), &key)), 0o600);
        assert_eq!(mode(&audio), 0o600);
    }

    /// A confirmed hit never writes a label from the future.
    ///
    /// The label of a confirmed hit is the wall clock of the moment of the
    /// mark. A label that ran ahead of the clock would make one track look
    /// older than a track that was really played later, and the eviction would
    /// drop exactly the entry that was played most recently.
    #[test]
    fn a_recency_mark_never_writes_a_value_from_the_future() {
        let cache = tempfile::tempdir().unwrap();
        let bytes = b"recency-clock-fixture".to_vec();
        let etag = digest(&bytes);
        let key = key(ORIGIN, ACCOUNT, TRACK);
        let published = saved_at_now();
        publish_bytes(cache.path(), &key, &bytes, published).unwrap();

        // Ten confirmed hits in a row: the label may never leave the clock.
        for _ in 0..10 {
            assert!(
                touch(cache.path(), &key, bytes.len() as u64, &etag, &never()).unwrap(),
                "a confirmed hit must refresh the recency of the entry"
            );
        }
        let recency = sidecar_saved_at(cache.path(), &key);
        assert!(
            recency <= saved_at_now(),
            "the recency of a confirmed hit is the current clock, never a value from the future"
        );
        assert!(
            recency >= published,
            "the recency of a confirmed hit never moves backwards"
        );
    }

    /// The eviction compares the labels of two entries against one clock.
    ///
    /// A burst of confirmed hits of one track must not push its label past a
    /// track that was really played later: the label of a confirmed hit is the
    /// clock of the moment of the hit, so two tracks stay comparable. The
    /// clock is pinned here, so the burst of hits is the one-second burst the
    /// review described and the result does not depend on the wall clock of
    /// the test run.
    #[test]
    fn the_eviction_order_compares_two_entries_by_one_clock() {
        let cache = tempfile::tempdir().unwrap();
        let first = key(ORIGIN, ACCOUNT, "20000000-0000-4000-8000-00000000000a");
        let second = key(ORIGIN, ACCOUNT, "20000000-0000-4000-8000-00000000000b");
        let newcomer = key(ORIGIN, ACCOUNT, "20000000-0000-4000-8000-00000000000c");
        let body = b"one-clock-original";
        let (_directory, source) = original(body);
        let budget = Budget {
            entries: 2,
            bytes: MAX_BYTES,
        };
        let clock = saved_at_now();

        // A was published at `clock`, B one second later.
        publish_bytes(cache.path(), &first, body, clock).unwrap();
        publish_bytes(cache.path(), &second, body, clock + 1).unwrap();

        // A is played again, several times inside the same second: the label
        // of A stays the clock of that second and does not run past the label
        // of B.
        for _ in 0..8 {
            assert!(
                touch_with_clock(
                    cache.path(),
                    &first,
                    body.len() as u64,
                    &digest(body),
                    &|| clock,
                    &never()
                )
                .unwrap(),
                "a confirmed hit must refresh the recency of the entry"
            );
        }
        assert_eq!(
            sidecar_saved_at(cache.path(), &first),
            clock,
            "the label of a confirmed hit is the clock of the hit, not a value that runs ahead of it"
        );

        // C is published into a namespace of two entries: the older label goes
        // first, so A is evicted and B keeps its place.
        store_at(
            cache.path(),
            &newcomer,
            &source,
            body.len() as u64,
            &digest(body),
            clock + 2,
            budget,
            &never(),
        )
        .unwrap();
        assert!(
            !audio_path(cache.path(), &first).exists(),
            "the entry with the older label is evicted first"
        );
        assert!(
            audio_path(cache.path(), &second).exists(),
            "the entry that was played later keeps its place"
        );
        assert!(audio_path(cache.path(), &newcomer).exists());
    }

    /// The eviction order follows the last confirmed playback, not the
    /// insertion: a track that is played again keeps its entry while the track
    /// that was only downloaded once is evicted.
    #[test]
    fn the_eviction_follows_the_confirmed_playback_and_not_the_insertion() {
        let oldest = key(ORIGIN, ACCOUNT, "20000000-0000-4000-8000-00000000000a");
        let newer = key(ORIGIN, ACCOUNT, "20000000-0000-4000-8000-00000000000b");
        let newcomer = key(ORIGIN, ACCOUNT, "20000000-0000-4000-8000-00000000000c");
        let body = b"eviction-order-original";
        let (_directory, source) = original(body);
        let budget = Budget {
            entries: 2,
            bytes: MAX_BYTES,
        };

        // Control: without a confirmed playback the older insertion goes
        // first.
        let control = tempfile::tempdir().unwrap();
        publish_bytes(control.path(), &oldest, body, 10).unwrap();
        publish_bytes(control.path(), &newer, body, 20).unwrap();
        store_at(
            control.path(),
            &newcomer,
            &source,
            body.len() as u64,
            &digest(body),
            30,
            budget,
            &never(),
        )
        .unwrap();
        assert!(
            !audio_path(control.path(), &oldest).exists(),
            "without a confirmed playback the older insertion is evicted first"
        );
        assert!(audio_path(control.path(), &newer).exists());

        // The same two entries with a confirmed playback of the older one: it
        // is the newer entry now, and the untouched one is evicted.
        let marked = tempfile::tempdir().unwrap();
        publish_bytes(marked.path(), &oldest, body, 10).unwrap();
        publish_bytes(marked.path(), &newer, body, 20).unwrap();
        assert!(
            touch(
                marked.path(),
                &oldest,
                body.len() as u64,
                &digest(body),
                &never()
            )
            .unwrap(),
            "a confirmed playback must refresh the recency"
        );
        store_at(
            marked.path(),
            &newcomer,
            &source,
            body.len() as u64,
            &digest(body),
            30,
            budget,
            &never(),
        )
        .unwrap();
        assert!(
            audio_path(marked.path(), &oldest).exists(),
            "an entry that was played again keeps its place in the cache"
        );
        assert!(
            !audio_path(marked.path(), &newer).exists(),
            "an entry that was never played again is evicted first"
        );
        assert!(audio_path(marked.path(), &newcomer).exists());
    }

    /// A recency mark needs the very record of the confirmed representation:
    /// another validator, another size, another identity, another version and
    /// a missing record are all refused, and a refused mark writes no file.
    #[test]
    fn a_recency_mark_needs_the_same_representation_and_writes_nothing_otherwise() {
        let cache = tempfile::tempdir().unwrap();
        let bytes = b"recency-mismatch-fixture".to_vec();
        let etag = digest(&bytes);
        let key = key(ORIGIN, ACCOUNT, TRACK);
        publish_bytes(cache.path(), &key, &bytes, 5).unwrap();
        let record = sidecar_path(cache.path(), &key);
        let valid = read_sidecar(&record).unwrap();
        let before = names(cache.path(), &key);

        // Another authoritative validator is another representation.
        assert!(
            !touch(
                cache.path(),
                &key,
                bytes.len() as u64,
                &"1".repeat(64),
                &never()
            )
            .unwrap()
        );
        // Another size is another representation.
        assert!(!touch(cache.path(), &key, bytes.len() as u64 + 1, &etag, &never()).unwrap());
        assert_eq!(
            names(cache.path(), &key),
            before,
            "a refused mark writes no file"
        );

        for foreign in [
            Sidecar {
                version: 2,
                ..clone_sidecar(&valid)
            },
            Sidecar {
                origin: OTHER_ORIGIN.into(),
                ..clone_sidecar(&valid)
            },
            Sidecar {
                account_id: OTHER_ACCOUNT.into(),
                ..clone_sidecar(&valid)
            },
            Sidecar {
                track_id: OTHER_TRACK.into(),
                ..clone_sidecar(&valid)
            },
            Sidecar {
                size_bytes: valid.size_bytes + 1,
                ..clone_sidecar(&valid)
            },
            Sidecar {
                etag: "3".repeat(64),
                ..clone_sidecar(&valid)
            },
            Sidecar {
                digest: "4".repeat(64),
                ..clone_sidecar(&valid)
            },
            Sidecar {
                saved_at: 0,
                ..clone_sidecar(&valid)
            },
        ] {
            fs::write(&record, serde_json::to_vec(&foreign).unwrap()).unwrap();
            assert!(
                !touch(cache.path(), &key, bytes.len() as u64, &etag, &never()).unwrap(),
                "a foreign record is never marked"
            );
            assert_eq!(
                names(cache.path(), &key),
                before,
                "a refused mark writes no file"
            );
        }

        // A missing record is refused as well, and nothing is created for it.
        fs::remove_file(&record).unwrap();
        assert!(!touch(cache.path(), &key, bytes.len() as u64, &etag, &never()).unwrap());
        assert_eq!(
            names(cache.path(), &key),
            vec![format!("{TRACK}.audio")],
            "a refused mark creates no file"
        );
    }

    /// A canceled recency mark leaves no staging file behind: neither before
    /// the first block nor while its own staging file is on disk.
    #[test]
    fn a_canceled_recency_mark_leaves_no_staging_file() {
        let cache = tempfile::tempdir().unwrap();
        let bytes = b"recency-cancellation-fixture".to_vec();
        let etag = digest(&bytes);
        let key = key(ORIGIN, ACCOUNT, TRACK);
        publish_bytes(cache.path(), &key, &bytes, 3).unwrap();
        let before = names(cache.path(), &key);

        // Canceled before the first block: nothing is written at all.
        assert_eq!(
            touch(cache.path(), &key, bytes.len() as u64, &etag, &|| true).unwrap_err(),
            CANCELED
        );
        assert_eq!(names(cache.path(), &key), before);

        // Canceled while this operation's own staging file is on disk: only
        // that staging file disappears.
        let canceled = || {
            names(cache.path(), &key)
                .iter()
                .any(|name| name.ends_with(".tmp"))
        };
        assert_eq!(
            touch(cache.path(), &key, bytes.len() as u64, &etag, &canceled).unwrap_err(),
            CANCELED
        );
        assert_eq!(
            names(cache.path(), &key),
            before,
            "a canceled mark leaves no staging file behind"
        );
        assert!(
            names(cache.path(), &key)
                .iter()
                .all(|name| !name.ends_with(".tmp") && !name.ends_with(".bak")),
            "a canceled mark leaves no staging and no backup behind"
        );
    }

    #[test]
    fn quota_counts_staging_bytes_and_refusal_keeps_the_healthy_entry() {
        let cache = tempfile::tempdir().unwrap();
        let healthy = key(ORIGIN, ACCOUNT, TRACK);
        publish_bytes(cache.path(), &healthy, b"healthy-original", 10).unwrap();
        let account = account_root(cache.path(), &healthy);
        // A leftover staging file of a crashed operation still consumes the budget.
        fs::write(account.join(".stale.1-bb.audio.tmp"), vec![0_u8; 4096]).unwrap();
        let (_directory, source) = original(b"another-original");
        let refused = store_at(
            cache.path(),
            &key(ORIGIN, ACCOUNT, OTHER_TRACK),
            &source,
            b"another-original".len() as u64,
            &digest(b"another-original"),
            20,
            Budget {
                entries: MAX_ENTRIES,
                bytes: 4096,
            },
            &never(),
        );
        assert_eq!(refused.unwrap_err(), FULL);
        assert!(
            open_verified(
                cache.path(),
                &healthy,
                b"healthy-original".len() as u64,
                &digest(b"healthy-original"),
                &never()
            )
            .unwrap()
            .is_some(),
            "a refused publication must keep the healthy entry"
        );
        assert_eq!(
            names(cache.path(), &healthy)
                .iter()
                .filter(|name| name.contains(".tmp"))
                .count(),
            1,
            "a refused publication removes only its own staging"
        );
        assert!(
            !audio_path(cache.path(), &key(ORIGIN, ACCOUNT, OTHER_TRACK)).exists(),
            "a refused publication publishes nothing"
        );
    }

    #[test]
    fn cancellation_removes_only_its_own_staging_and_never_a_previous_entry() {
        let cache = tempfile::tempdir().unwrap();
        let key = key(ORIGIN, ACCOUNT, TRACK);
        let previous = b"previous-healthy-original".to_vec();
        publish_bytes(cache.path(), &key, &previous, 1).unwrap();
        let before = names(cache.path(), &key);
        let bytes = vec![9_u8; BLOCK_BYTES * 3];
        let (_directory, source) = original(&bytes);

        // Canceled before the first block: nothing is published.
        assert_eq!(
            store_at(
                cache.path(),
                &key,
                &source,
                bytes.len() as u64,
                &digest(&bytes),
                2,
                Budget::default(),
                &|| true,
            )
            .unwrap_err(),
            CANCELED
        );
        assert_eq!(names(cache.path(), &key), before);

        // Canceled during the copy: only this operation's own staging disappears.
        let probes = Cell::new(0_u32);
        let canceled = || {
            probes.set(probes.get() + 1);
            probes.get() > 2
        };
        assert_eq!(
            store_at(
                cache.path(),
                &key,
                &source,
                bytes.len() as u64,
                &digest(&bytes),
                2,
                Budget::default(),
                &canceled,
            )
            .unwrap_err(),
            CANCELED
        );
        assert_eq!(
            names(cache.path(), &key),
            before,
            "a canceled copy leaves no staging, no partial entry and no foreign file"
        );
        assert_eq!(
            read_cached(
                open_verified(
                    cache.path(),
                    &key,
                    previous.len() as u64,
                    &digest(&previous),
                    &never()
                )
                .unwrap()
                .expect("the previous entry must survive a canceled replacement")
            )
            .0,
            previous
        );
    }

    #[test]
    fn a_diverging_fingerprint_is_never_published() {
        let cache = tempfile::tempdir().unwrap();
        let key = key(ORIGIN, ACCOUNT, TRACK);
        let bytes = b"fingerprint-divergence-fixture".to_vec();
        let (_directory, source) = original(&bytes);
        // The transfer claims one validator while the copied bytes hash to another.
        assert_eq!(
            store_at(
                cache.path(),
                &key,
                &source,
                bytes.len() as u64,
                &"a".repeat(64),
                1,
                Budget::default(),
                &never(),
            )
            .unwrap_err(),
            INTEGRITY
        );
        assert!(!audio_path(cache.path(), &key).exists());
        assert!(!sidecar_path(cache.path(), &key).exists());
        assert!(names(cache.path(), &key).is_empty());
    }
    #[test]
    fn usage_and_clear_touch_only_published_pairs() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let key = key(ORIGIN, ACCOUNT, TRACK);
        let namespace = account_root(root, &key);
        fs::create_dir_all(&namespace).unwrap();
        let bytes = b"cached audio bytes";
        fs::write(audio_path(root, &key), bytes).unwrap();
        let sidecar = Sidecar {
            version: VERSION,
            origin: ORIGIN.into(),
            account_id: ACCOUNT.into(),
            track_id: TRACK.into(),
            size_bytes: bytes.len() as u64,
            etag: digest(bytes),
            digest: digest(bytes),
            saved_at: 7,
        };
        fs::write(
            sidecar_path(root, &key),
            serde_json::to_vec(&sidecar).unwrap(),
        )
        .unwrap();
        fs::write(namespace.join("foreign.tmp"), b"do not delete").unwrap();
        let (entries, used) = usage_at(&namespace).unwrap();
        assert_eq!(entries, 1);
        assert!(used > bytes.len() as u64);
        clear_at(&namespace, ORIGIN, ACCOUNT).unwrap();
        assert!(!audio_path(root, &key).exists());
        assert!(!sidecar_path(root, &key).exists());
        assert!(namespace.join("foreign.tmp").exists());
        assert_eq!(usage_at(&namespace).unwrap(), (0, 0));
    }

    /// A guard removes only the gate it installed. Two tests that install a gate
    /// concurrently (the suite runs with more than one thread) must not steal each
    /// other's slot: the second test's copy would otherwise run unheld, and its
    /// assertion would fail for the wrong reason.
    #[test]
    fn a_copy_gate_guard_restores_only_the_gate_it_installed() {
        let root = tempfile::tempdir().unwrap();
        // The gate slot is global: only one test may hold it at a time.
        let section = copy_gate::exclusive();
        let first = copy_gate::CopyGate::new(root.path());
        let second = copy_gate::CopyGate::new(root.path());
        let outer = copy_gate::install_scoped(first.clone());
        {
            let inner = copy_gate::install_scoped(second.clone());
            assert!(Arc::ptr_eq(
                &copy_gate::installed().expect("the inner gate is installed"),
                &second
            ));
            drop(inner);
            assert!(
                Arc::ptr_eq(
                    &copy_gate::installed().expect("the outer gate is restored"),
                    &first
                ),
                "an inner guard must restore the gate it replaced"
            );
        }
        // The slot now belongs to a gate this guard never installed: dropping the
        // outer guard must leave it alone.
        let _ = copy_gate::install(second.clone());
        drop(outer);
        assert!(
            Arc::ptr_eq(
                &copy_gate::installed().expect("the foreign gate must survive"),
                &second
            ),
            "a guard must not remove a gate it does not own"
        );
        // The section owns the slot: it hands it back when it ends. The next
        // section observes the state the previous one left.
        drop(section);
        let next = copy_gate::exclusive();
        assert!(
            next.baseline().is_none(),
            "a section must not leave a gate behind"
        );
    }

    /// The global gate is installed by tests that must not run at the same
    /// time: `copy_verified` runs on a blocking worker without a client
    /// handle, so the slot cannot be scoped to one client.
    ///
    /// The second thread must not enter the section while the first one holds
    /// it, and it must find the slot empty when it does: the gate of the first
    /// section is never observed by a later one, and a section never leaves a
    /// gate behind.
    #[test]
    fn a_copy_gate_section_admits_one_test_at_a_time() {
        use std::sync::mpsc;
        use std::time::Duration;

        let root = tempfile::tempdir().unwrap();
        let first_gate = copy_gate::CopyGate::new(root.path());
        let second_gate = copy_gate::CopyGate::new(root.path());
        let second_entered = Arc::new(AtomicUsize::new(0));

        let (attempted_tx, attempted_rx) = mpsc::channel::<()>();
        let (held_tx, held_rx) = mpsc::channel::<bool>();
        let (outcome_tx, outcome_rx) = mpsc::channel::<Result<(), String>>();

        // The first test holds the section and installs its own gate.
        let section = copy_gate::exclusive();
        let _previous = copy_gate::install(first_gate.clone());
        assert!(Arc::ptr_eq(
            &copy_gate::installed().expect("the first gate is installed"),
            &first_gate
        ));

        let worker = {
            let second_entered = second_entered.clone();
            let second_gate = second_gate.clone();
            std::thread::spawn(move || {
                attempted_tx
                    .send(())
                    .expect("the first thread receives the attempt");
                let held = copy_gate::section_is_held();
                held_tx
                    .send(held)
                    .expect("the first thread receives the section state");
                let section = copy_gate::exclusive();
                second_entered.fetch_add(1, Ordering::SeqCst);
                let outcome = (|| {
                    if section.baseline().is_some() {
                        return Err("a section must not leave a gate behind".to_owned());
                    }
                    if copy_gate::installed().is_some() {
                        return Err("a section must start with an empty slot".to_owned());
                    }
                    let _gate = copy_gate::install_scoped(second_gate.clone());
                    let installed = copy_gate::installed().expect("the second gate is installed");
                    if !Arc::ptr_eq(&installed, &second_gate) {
                        return Err("the second section must own the slot".to_owned());
                    }
                    Ok(())
                })();
                drop(section);
                outcome_tx
                    .send(outcome)
                    .expect("the first thread receives the outcome");
            })
        };

        attempted_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("the second thread attempts the section");
        assert_eq!(
            second_entered.load(Ordering::SeqCst),
            0,
            "the second thread must not enter while the section is held"
        );
        assert!(
            held_rx
                .recv_timeout(Duration::from_secs(5))
                .expect("the second thread reports the section state"),
            "the second thread must find the section held"
        );
        assert_eq!(
            second_entered.load(Ordering::SeqCst),
            0,
            "the second thread must not enter while the section is held"
        );

        // The first section ends: the slot is handed back before the lock is
        // released, so the second thread finds an empty slot. `_previous` is the
        // slot value saved by `install` (always `None` here) and is not a guard.
        drop(section);

        let outcome = outcome_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("the second thread enters the section");
        assert_eq!(
            second_entered.load(Ordering::SeqCst),
            1,
            "the second thread enters after the first section ended"
        );
        outcome.expect("the second section observes a clean slot");
        worker.join().expect("the second thread finishes");
    }
}
