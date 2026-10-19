//! Private bounded disk cache of already-authorized cover images.
//!
//! Layout: `$XDG_CACHE_HOME/vela/artwork/<origin-hash>/<account-id>/<artwork-id>.<ext>`.
//! Every directory is `0700`, every file `0600`, a new file is written to a unique
//! staging name and published with one rename, and eviction only ever deletes this
//! cache's own published `<canonical-uuid>.<ext>` names inside one account namespace.
//! Offline copies (`$XDG_DATA_HOME/vela/offline`), the media cache and arbitrary user
//! files are structurally unreachable from here.
//!
//! A cached file is never a grant. The client revalidates the current session with the
//! origin before it reads one, and a denial never reaches this module; cached bytes can
//! therefore only reappear while the origin itself still authorizes this account and
//! artwork id.
use crate::client::{ArtworkBytes, ArtworkMime, MAX_ARTWORK_BYTES};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

/// Published cover files per origin+account namespace.
pub(crate) const MAX_FILES: usize = 200;
/// Published cover bytes per origin+account namespace.
pub(crate) const MAX_BYTES: u64 = 64 * 1024 * 1024;

const BLOCK_BYTES: usize = 32 * 1024;
const MAX_DIRECTORY_ENTRIES: usize = 4096;
const CANCELED: &str = "Artwork request canceled.";
const UNAVAILABLE: &str = "Cached cover art is unavailable.";
const WRITE: &str = "Cached cover art could not be written.";

/// The budget of one origin+account namespace. Production uses [`Budget::default`];
/// tests use smaller values to exercise refusal and eviction.
#[derive(Clone, Copy)]
pub(crate) struct Budget {
    pub(crate) files: usize,
    pub(crate) bytes: u64,
}
impl Default for Budget {
    fn default() -> Self {
        Self {
            files: MAX_FILES,
            bytes: MAX_BYTES,
        }
    }
}

/// One origin/account/artwork identity. Only canonical UUIDs and a non-empty origin
/// are accepted, so a name can never escape the namespace directory.
#[derive(Clone)]
pub(crate) struct Key {
    origin: String,
    account_id: String,
    artwork_id: String,
}
impl Key {
    pub(crate) fn new(origin: &str, account_id: &str, artwork_id: &str) -> Option<Self> {
        if origin.is_empty()
            || origin.len() > 2048
            || origin.chars().any(char::is_control)
            || !crate::client::canonical_uuid(account_id)
            || !crate::client::canonical_uuid(artwork_id)
        {
            return None;
        }
        Some(Self {
            origin: origin.to_owned(),
            account_id: account_id.to_owned(),
            artwork_id: artwork_id.to_owned(),
        })
    }
    pub(crate) fn origin(&self) -> &str {
        &self.origin
    }
    pub(crate) fn account_id(&self) -> &str {
        &self.account_id
    }
    pub(crate) fn artwork_id(&self) -> &str {
        &self.artwork_id
    }
}

/// `$XDG_CACHE_HOME/vela/artwork`, or `$HOME/.cache/vela/artwork`. A missing cache root
/// means no cache (the authorized GET stays the only path), never a public temp dir.
pub(crate) fn default_root() -> Result<PathBuf, String> {
    let base = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| {
            std::env::var_os("HOME")
                .map(PathBuf::from)
                .map(|home| home.join(".cache"))
        })
        .ok_or_else(|| "No local cache directory is available for cover art.".to_owned())?;
    Ok(base.join("vela").join("artwork"))
}

/// `<root>/<origin-hash>`: a cross-origin UUID collision can never share a directory.
pub(crate) fn origin_root(root: &Path, key: &Key) -> PathBuf {
    root.join(blake3::hash(key.origin().as_bytes()).to_hex())
}

/// `<root>/<origin-hash>/<account-id>`: the namespace the budget and files belong to.
pub(crate) fn account_root(root: &Path, key: &Key) -> PathBuf {
    origin_root(root, key).join(key.account_id())
}

/// `<root>/<origin-hash>/<account-id>/<artwork-id>.<ext>`.
pub(crate) fn path(root: &Path, key: &Key, mime: ArtworkMime) -> PathBuf {
    account_root(root, key).join(format!("{}.{}", key.artwork_id(), mime.extension()))
}

/// Read one cached cover only after the caller revalidated the current session. Any
/// missing, foreign, oversized, tampered or symlinked entry is an honest miss; a
/// magic/content-type mismatch removes the file so it cannot be trusted later.
pub(crate) fn read(
    root: &Path,
    key: &Key,
    mime: ArtworkMime,
    canceled: &dyn Fn() -> bool,
) -> Result<Option<ArtworkBytes>, String> {
    if canceled() {
        return Err(CANCELED.into());
    }
    let path = path(root, key, mime);
    let Ok(link) = fs::symlink_metadata(&path) else {
        return Ok(None);
    };
    // A symlink is never cache content, even one that points at a regular file.
    if link.file_type().is_symlink() || !link.is_file() {
        return Ok(None);
    }
    if link.len() == 0 || link.len() > MAX_ARTWORK_BYTES {
        let _ = fs::remove_file(&path);
        return Ok(None);
    }
    let Ok(mut file) = File::open(&path) else {
        return Ok(None);
    };
    let Ok(metadata) = file.metadata() else {
        return Ok(None);
    };
    if !metadata.is_file() || metadata.len() != link.len() {
        return Ok(None);
    }
    let mut bytes = Vec::with_capacity(link.len() as usize);
    let mut buffer = [0_u8; BLOCK_BYTES];
    loop {
        if canceled() {
            return Err(CANCELED.into());
        }
        match file.read(&mut buffer) {
            Ok(0) => break,
            Ok(read) => {
                if bytes.len().saturating_add(read) as u64 > MAX_ARTWORK_BYTES {
                    return Ok(None);
                }
                bytes.extend_from_slice(&buffer[..read]);
            }
            Err(_) => return Ok(None),
        }
    }
    if bytes.len() as u64 != link.len() {
        return Ok(None);
    }
    match ArtworkBytes::new(mime, bytes) {
        Some(bytes) => Ok(Some(bytes)),
        None => {
            // A file that is not the declared raster type is not cover art any more.
            let _ = fs::remove_file(&path);
            Ok(None)
        }
    }
}

/// Publish one already-authorized cover into this namespace. Best effort by contract:
/// the caller keeps the bytes it just verified, so a full disk, a full cache or a
/// cancellation here can never fail the cover itself.
pub(crate) fn store(
    root: &Path,
    key: &Key,
    art: &ArtworkBytes,
    canceled: &dyn Fn() -> bool,
) -> Result<(), String> {
    store_at(root, key, art, Budget::default(), canceled)
}

fn store_at(
    root: &Path,
    key: &Key,
    art: &ArtworkBytes,
    budget: Budget,
    canceled: &dyn Fn() -> bool,
) -> Result<(), String> {
    if canceled() {
        return Err(CANCELED.into());
    }
    if art.is_empty() || art.len() as u64 > MAX_ARTWORK_BYTES {
        return Err(UNAVAILABLE.into());
    }
    private_directory(root)?;
    private_directory(&origin_root(root, key))?;
    let account = account_root(root, key);
    private_directory(&account)?;
    // Unique per operation and per process: a second attempt never writes, publishes
    // or removes another operation's staging file.
    let staged = account.join(format!(".{}.{}.tmp", key.artwork_id(), unique_suffix()));
    let staged_result = (|| -> Result<(), String> {
        let mut file = private_file(&staged).map_err(|_| WRITE.to_owned())?;
        file.write_all(art.bytes()).map_err(|_| WRITE.to_owned())?;
        file.sync_all().map_err(|_| WRITE.to_owned())?;
        drop(file);
        make_room(&account, key, art.len() as u64, budget, canceled)?;
        // The commit point: cancellation observed before it removes only this
        // operation's own staging and never an earlier healthy entry.
        if canceled() {
            return Err(CANCELED.into());
        }
        publish(&staged, &path(root, key, art.mime()))
    })();
    if staged_result.is_err() {
        let _ = fs::remove_file(&staged);
    }
    staged_result
}

/// Evict this cache's own oldest published covers until the namespace fits. Staging
/// names never match the published pattern, so an in-flight operation of this or
/// another process can never lose its file. A namespace that cannot fit refuses
/// cleanly and deletes nothing.
fn make_room(
    account: &Path,
    key: &Key,
    new_bytes: u64,
    budget: Budget,
    canceled: &dyn Fn() -> bool,
) -> Result<(), String> {
    if canceled() {
        return Err(CANCELED.into());
    }
    let mut published = published(account)?;
    // The entry being replaced is neither counted twice nor evicted by this operation.
    published.retain(|entry| entry.artwork_id != key.artwork_id());
    let mut files = published.len().saturating_add(1);
    let mut bytes = published
        .iter()
        .fold(new_bytes, |total, entry| total.saturating_add(entry.bytes));
    let mut doomed = Vec::new();
    while files > budget.files || bytes > budget.bytes {
        if canceled() {
            return Err(CANCELED.into());
        }
        let Some(oldest) = published.first().cloned() else {
            return Err(UNAVAILABLE.into());
        };
        bytes = bytes.saturating_sub(oldest.bytes);
        files = files.saturating_sub(1);
        published.remove(0);
        doomed.push(oldest.path);
    }
    for path in doomed {
        let _ = fs::remove_file(path);
    }
    Ok(())
}

#[derive(Clone)]
struct Entry {
    artwork_id: String,
    bytes: u64,
    modified: u128,
    path: PathBuf,
}

/// Every published cover in this namespace, oldest first. Foreign or malformed names
/// are ignored, and an unreadable file is treated as the oldest possible entry so it
/// can never protect itself from eviction.
fn published(account: &Path) -> Result<Vec<Entry>, String> {
    let entries = match fs::read_dir(account) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(_) => return Err(UNAVAILABLE.into()),
    };
    let mut published = Vec::new();
    for entry in entries.filter_map(Result::ok).take(MAX_DIRECTORY_ENTRIES) {
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        let Some(stem) = name
            .strip_suffix(".jpg")
            .or_else(|| name.strip_suffix(".png"))
            .or_else(|| name.strip_suffix(".webp"))
        else {
            continue;
        };
        if !crate::client::canonical_uuid(stem) {
            continue;
        }
        let Ok(link) = fs::symlink_metadata(entry.path()) else {
            continue;
        };
        if link.file_type().is_symlink() || !link.is_file() {
            continue;
        }
        let modified = link
            .modified()
            .ok()
            .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
            .map_or(0, |age| age.as_nanos());
        published.push(Entry {
            artwork_id: stem.to_owned(),
            bytes: link.len(),
            modified,
            path: entry.path(),
        });
    }
    published.sort_by(|left, right| {
        left.modified
            .cmp(&right.modified)
            .then_with(|| left.artwork_id.cmp(&right.artwork_id))
    });
    Ok(published)
}

/// One atomic rename publishes the complete file. The previous entry of the same name
/// is replaced on unix; elsewhere a stale target is removed first.
fn publish(staged: &Path, target: &Path) -> Result<(), String> {
    #[cfg(not(unix))]
    let _ = fs::remove_file(target);
    fs::rename(staged, target).map_err(|_| WRITE.to_owned())
}

fn private_directory(path: &Path) -> Result<(), String> {
    fs::create_dir_all(path).map_err(|_| WRITE.to_owned())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))
            .map_err(|_| WRITE.to_owned())?;
    }
    Ok(())
}

fn private_file(path: &Path) -> std::io::Result<File> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options.open(path)
}

/// Never reused by a second operation of this or another process.
fn unique_suffix() -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let counter = COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |age| age.as_nanos());
    format!("{}-{counter}-{nanos}", std::process::id())
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

    const JPEG: &[u8] = &[
        0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, b'J', b'F', b'I', b'F', 0x00, 0x01, 0x01, 0x00, 0x00,
        0x01, 0x00, 0x01, 0x00, 0x00, 0xFF, 0xD9,
    ];

    fn key(origin: &str, account: &str, artwork: &str) -> Key {
        Key::new(origin, account, artwork).unwrap()
    }

    fn art() -> ArtworkBytes {
        ArtworkBytes::new(ArtworkMime::Jpeg, JPEG.to_vec()).unwrap()
    }

    fn never() -> bool {
        false
    }

    fn names(directory: &Path) -> Vec<String> {
        let mut names = fs::read_dir(directory)
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

    #[test]
    fn namespace_paths_isolate_origin_account_and_artwork() {
        let root = Path::new("/private/cache/vela/artwork");
        let base = key(ORIGIN, ACCOUNT, ART);
        let origin_dir = origin_root(root, &base);
        assert_eq!(origin_dir.parent().unwrap(), root);
        assert_eq!(
            origin_dir.file_name().unwrap().to_str().unwrap().len(),
            64,
            "the origin is identified by a full BLAKE3 hash"
        );
        assert_eq!(
            path(root, &base, ArtworkMime::Jpeg),
            origin_dir.join(ACCOUNT).join(format!("{ART}.jpg"))
        );
        assert_eq!(
            path(root, &base, ArtworkMime::Png),
            origin_dir.join(ACCOUNT).join(format!("{ART}.png"))
        );
        assert_ne!(
            path(root, &base, ArtworkMime::Jpeg),
            path(root, &key(OTHER_ORIGIN, ACCOUNT, ART), ArtworkMime::Jpeg),
            "a colliding UUID on another origin never shares a directory"
        );
        assert_ne!(
            path(root, &base, ArtworkMime::Jpeg),
            path(root, &key(ORIGIN, OTHER_ACCOUNT, ART), ArtworkMime::Jpeg)
        );
        assert_ne!(
            path(root, &base, ArtworkMime::Jpeg),
            path(root, &key(ORIGIN, ACCOUNT, OTHER_ART), ArtworkMime::Jpeg)
        );
    }

    #[test]
    fn keys_reject_non_canonical_names() {
        assert!(Key::new(ORIGIN, ACCOUNT, "../private").is_none());
        assert!(Key::new(ORIGIN, ACCOUNT, "not-a-uuid").is_none());
        let lettered = "20000000-0000-4000-8000-00000000000a";
        assert!(Key::new(ORIGIN, ACCOUNT, lettered).is_some());
        assert!(Key::new(ORIGIN, ACCOUNT, &lettered.to_ascii_uppercase()).is_none());
        assert!(Key::new("", ACCOUNT, ART).is_none());
        assert!(Key::new(ORIGIN, "../account", ART).is_none());
        assert!(Key::new(ORIGIN, ACCOUNT, ART).is_some());
    }

    #[test]
    fn store_read_round_trip_is_private_and_has_no_staging_left() {
        let cache = tempfile::tempdir().unwrap();
        let base = key(ORIGIN, ACCOUNT, ART);
        store(cache.path(), &base, &art(), &never).unwrap();
        let file = path(cache.path(), &base, ArtworkMime::Jpeg);
        assert_eq!(fs::read(&file).unwrap(), JPEG);
        assert_eq!(
            names(file.parent().unwrap()),
            vec![format!("{ART}.jpg")],
            "no staging name survives a successful publish"
        );
        assert_eq!(
            read(cache.path(), &base, ArtworkMime::Jpeg, &never)
                .unwrap()
                .unwrap()
                .bytes(),
            JPEG
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = |path: &Path| fs::metadata(path).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode(cache.path()), 0o700);
            assert_eq!(mode(&origin_root(cache.path(), &base)), 0o700);
            assert_eq!(mode(&account_root(cache.path(), &base)), 0o700);
            assert_eq!(mode(&file), 0o600);
        }
    }

    #[test]
    fn cached_bytes_are_scoped_to_their_own_account_origin_and_artwork() {
        let cache = tempfile::tempdir().unwrap();
        store(cache.path(), &key(ORIGIN, ACCOUNT, ART), &art(), &never).unwrap();
        for foreign in [
            key(OTHER_ORIGIN, ACCOUNT, ART),
            key(ORIGIN, OTHER_ACCOUNT, ART),
            key(ORIGIN, ACCOUNT, OTHER_ART),
        ] {
            assert!(
                read(cache.path(), &foreign, ArtworkMime::Jpeg, &never)
                    .unwrap()
                    .is_none()
            );
        }
        // A different declared type never reuses a file of another extension.
        assert!(
            read(
                cache.path(),
                &key(ORIGIN, ACCOUNT, ART),
                ArtworkMime::Png,
                &never
            )
            .unwrap()
            .is_none()
        );
    }

    #[test]
    fn tampered_or_oversized_cache_files_are_rejected_and_removed() {
        let cache = tempfile::tempdir().unwrap();
        let base = key(ORIGIN, ACCOUNT, ART);
        store(cache.path(), &base, &art(), &never).unwrap();
        let file = path(cache.path(), &base, ArtworkMime::Jpeg);
        fs::write(&file, b"<svg onload=\"alert(1)\"></svg>").unwrap();
        assert!(
            read(cache.path(), &base, ArtworkMime::Jpeg, &never)
                .unwrap()
                .is_none(),
            "content that is not the declared raster type is never trusted"
        );
        assert!(!file.exists(), "a mismatched cache file is removed");

        let oversized = cache.path().join("oversized.jpg");
        {
            let file = File::create(&oversized).unwrap();
            file.set_len(MAX_ARTWORK_BYTES + 1).unwrap();
        }
        let oversized_key = key(ORIGIN, ACCOUNT, OTHER_ART);
        let target = path(cache.path(), &oversized_key, ArtworkMime::Jpeg);
        private_directory(target.parent().unwrap()).unwrap();
        fs::rename(&oversized, &target).unwrap();
        assert!(
            read(cache.path(), &oversized_key, ArtworkMime::Jpeg, &never)
                .unwrap()
                .is_none()
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_symlinked_cache_entry_is_never_followed() {
        let cache = tempfile::tempdir().unwrap();
        let base = key(ORIGIN, ACCOUNT, ART);
        let real = cache.path().join("outside.jpg");
        fs::write(&real, JPEG).unwrap();
        let account = account_root(cache.path(), &base);
        private_directory(&account).unwrap();
        std::os::unix::fs::symlink(&real, account.join(format!("{ART}.jpg"))).unwrap();
        assert!(
            read(cache.path(), &base, ArtworkMime::Jpeg, &never)
                .unwrap()
                .is_none(),
            "a symlink outside the namespace must not become cover art"
        );
        assert!(real.exists());
    }

    #[test]
    fn budget_eviction_removes_only_the_oldest_published_covers() {
        let cache = tempfile::tempdir().unwrap();
        let budget = Budget {
            files: 2,
            bytes: 64,
        };
        let first = key(ORIGIN, ACCOUNT, ART);
        let second = key(ORIGIN, ACCOUNT, OTHER_ART);
        store_at(cache.path(), &first, &art(), budget, &never).unwrap();
        // Make the first entry unambiguously older than the second.
        let older = path(cache.path(), &first, ArtworkMime::Jpeg);
        let time = SystemTime::now() - std::time::Duration::from_secs(60);
        filetime(&older, time);
        store_at(cache.path(), &second, &art(), budget, &never).unwrap();
        let third = key(ORIGIN, ACCOUNT, "20000000-0000-4000-8000-000000000003");
        store_at(cache.path(), &third, &art(), budget, &never).unwrap();
        let account = account_root(cache.path(), &first);
        let published = names(&account);
        assert!(
            !published.contains(&format!("{ART}.jpg")),
            "the oldest cover is evicted first: {published:?}"
        );
        assert!(published.contains(&format!("{OTHER_ART}.jpg")));
        assert!(published.contains(&format!("{}.jpg", "20000000-0000-4000-8000-000000000003")));
        // A file this cache never published is not evictable cache data.
        let foreign = account.join("notes.txt");
        fs::write(&foreign, b"keep me").unwrap();
        store_at(
            cache.path(),
            &key(ORIGIN, ACCOUNT, "20000000-0000-4000-8000-000000000004"),
            &art(),
            budget,
            &never,
        )
        .unwrap();
        assert!(foreign.exists());
    }

    #[test]
    fn a_canceled_store_leaves_no_staging_or_damaged_entry() {
        let cache = tempfile::tempdir().unwrap();
        let base = key(ORIGIN, ACCOUNT, ART);
        store(cache.path(), &base, &art(), &never).unwrap();
        let before = fs::read(path(cache.path(), &base, ArtworkMime::Jpeg)).unwrap();
        let canceled = || true;
        assert!(
            store(cache.path(), &base, &art(), &canceled).is_err(),
            "a canceled store is an honest failure"
        );
        assert_eq!(
            fs::read(path(cache.path(), &base, ArtworkMime::Jpeg)).unwrap(),
            before,
            "the previous healthy entry survives a canceled replacement"
        );
        assert_eq!(
            names(
                path(cache.path(), &base, ArtworkMime::Jpeg)
                    .parent()
                    .unwrap()
            ),
            vec![format!("{ART}.jpg")],
            "cancellation leaves no staging file"
        );
    }

    fn filetime(path: &Path, time: SystemTime) {
        let file = OpenOptions::new().write(true).open(path).unwrap();
        file.set_modified(time).unwrap();
    }
}
