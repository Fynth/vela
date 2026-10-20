//! Private, non-secret offline profile: which canonical origin and account UUID
//! this device holds saved copies for.
//!
//! AUD-05: without a server, a cold start could not name the private offline
//! namespace, because the namespace is keyed by the canonical origin *and* the
//! account UUID (AUD-06) and neither is derivable from the device alone. This
//! document therefore records exactly those two identifiers and nothing else:
//!
//! ```json
//! {"version":1,"origin":"https://music.example.org/","account_id":"<uuid>"}
//! ```
//!
//! It never stores a session token, password, CSRF token, audio URL, media path,
//! username, queue, listening event or server response. Both stored values are
//! identifiers the saved copies are already scoped by on disk, so the file grants
//! no access by itself: the offline screen it enables is read/play-only and every
//! server route, save, favorite and search still requires a real sign-in.
//!
//! The document lives next to the copies it describes, at
//! `$XDG_DATA_HOME/vela/offline/profile.json` (or `$HOME/.local/share/...`). It is
//! written atomically (private temp file, fsync, rename) with mode 0600 inside the
//! 0700 offline tree. A corrupt, oversized, foreign-version, non-canonical or
//! foreign-shaped document is *ignored*: [`load`] reports `None`, this module never
//! rewrites or deletes such a file, and no saved offline copy is ever derived from,
//! rewritten by or deleted because of a profile lookup.
use crate::client::{canonical_origin, canonical_uuid};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{
        Mutex, MutexGuard,
        atomic::{AtomicU64, Ordering},
    },
};

/// Only version 1 exists. A document that declares anything else is ignored.
const VERSION: u8 = 1;
/// The whole document is two identifiers, so a larger file is not one of ours and
/// is never parsed (nor read into memory).
const MAX_BYTES: u64 = 4 * 1024;
const WRITE: &str = "The offline profile could not be stored.";
const READ: &str = "The offline profile could not be read.";

/// One validated offline profile. `origin` is the canonical origin exactly as the
/// native HTTP client spells it, and `account_id` is a canonical lower-case UUID.
/// Neither value is a credential.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OfflineProfile {
    pub origin: String,
    pub account_id: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope {
    version: u8,
    origin: String,
    account_id: String,
}

/// Monotonic sign-in generation. An explicit sign-out bumps it, so a profile write
/// that a sign-in scheduled earlier can never recreate the hint afterwards. This is
/// the same attempt rule the saved session uses (AUD-01).
static GENERATION: AtomicU64 = AtomicU64::new(0);
/// Serializes writes and clears, so "store the hint" and "withdraw the hint" can
/// never interleave and leave the opposite of what the listener asked for.
static STORE_LOCK: Mutex<()> = Mutex::new(());

/// The result of a guarded profile write.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StoreOutcome {
    Stored,
    /// A newer sign-in or an explicit sign-out owns the profile now.
    Superseded,
}

/// One sign-in's claim on the profile document.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Attempt(u64);

/// Claim the profile for one sign-in attempt. Every later [`clear`] supersedes it.
pub fn begin_attempt() -> Attempt {
    Attempt(GENERATION.fetch_add(1, Ordering::SeqCst).wrapping_add(1))
}

fn is_current(attempt: Attempt) -> bool {
    GENERATION.load(Ordering::SeqCst) == attempt.0
}

/// Withdrawal wins over every write that was claimed before it.
fn supersede() {
    GENERATION.fetch_add(1, Ordering::SeqCst);
}

fn lock_store() -> MutexGuard<'static, ()> {
    STORE_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// `$XDG_DATA_HOME/vela/offline/profile.json`, beside the per-origin namespaces it
/// describes. A missing private data root means no offline profile, never a public
/// temporary directory.
pub fn store_path() -> Result<PathBuf, String> {
    Ok(crate::offline::offline_base()?.join("profile.json"))
}

pub fn load() -> Result<Option<OfflineProfile>, String> {
    load_from(&store_path()?)
}

/// Record the identity of the account whose copies this device may open offline.
/// Called only after a real sign-in/resume, never from a stored credential.
pub fn save(attempt: Attempt, origin: &str, account_id: &str) -> Result<StoreOutcome, String> {
    let path = store_path()?;
    let _guard = lock_store();
    if !is_current(attempt) {
        return Ok(StoreOutcome::Superseded);
    }
    save_to(&path, origin, account_id)?;
    Ok(StoreOutcome::Stored)
}

/// Explicit sign-out removes the profile and supersedes every pending write. Saved
/// copies and their manifests are not touched: only the cold-start hint is withdrawn.
pub fn clear() -> Result<(), String> {
    let path = store_path()?;
    let _guard = lock_store();
    supersede();
    clear_at(&path)
}

/// `Ok(None)` means "no usable profile": missing, unreadable-as-a-document,
/// oversized, corrupt, a foreign version, or not naming a canonical origin plus
/// account UUID. A real file-system failure to read an existing document is
/// reported as an error so the caller can show the ordinary sign-in screen with an
/// honest reason instead of silently pretending there is no profile.
fn load_from(path: &Path) -> Result<Option<OfflineProfile>, String> {
    let metadata = match fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(READ.into()),
    };
    if !metadata.is_file() || metadata.len() > MAX_BYTES {
        return Ok(None);
    }
    let mut bytes = Vec::with_capacity(metadata.len().min(MAX_BYTES) as usize);
    fs::File::open(path)
        .and_then(|file| file.take(MAX_BYTES + 1).read_to_end(&mut bytes))
        .map_err(|_| READ.to_owned())?;
    if bytes.len() as u64 > MAX_BYTES {
        return Ok(None);
    }
    Ok(parse(&bytes))
}

/// Pure document parser. Every rejection is `None`, so a foreign or corrupt
/// document can never name a namespace.
fn parse(bytes: &[u8]) -> Option<OfflineProfile> {
    let envelope = serde_json::from_slice::<Envelope>(bytes).ok()?;
    if envelope.version != VERSION {
        return None;
    }
    let origin = canonical_origin(&envelope.origin).ok()?.as_str().to_owned();
    if !canonical_uuid(&envelope.account_id) {
        return None;
    }
    Some(OfflineProfile {
        origin,
        account_id: envelope.account_id,
    })
}

fn save_to(path: &Path, origin: &str, account_id: &str) -> Result<(), String> {
    let origin = canonical_origin(origin).map_err(|_| WRITE.to_owned())?;
    if !canonical_uuid(account_id) {
        return Err(WRITE.into());
    }
    let payload = serde_json::to_vec(&Envelope {
        version: VERSION,
        origin: origin.as_str().to_owned(),
        account_id: account_id.to_owned(),
    })
    .map_err(|_| WRITE.to_owned())?;
    if payload.len() as u64 > MAX_BYTES {
        return Err(WRITE.into());
    }
    let parent = path.parent().ok_or_else(|| WRITE.to_owned())?;
    private_directory(parent)?;
    // Unique per write, so two processes can never stage the same name and a
    // leftover staging file is never overwritten.
    let temporary = parent.join(format!(
        ".profile.{}-{}.tmp",
        std::process::id(),
        uuid::Uuid::new_v4().simple()
    ));
    let result = (|| {
        let mut file = private_file(&temporary)?;
        file.write_all(&payload)?;
        file.sync_all()?;
        drop(file);
        fs::rename(&temporary, path)?;
        private_mode(path)?;
        Ok::<_, std::io::Error>(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
        return Err(WRITE.into());
    }
    Ok(())
}

/// Removing the hint is idempotent and never touches anything else in the tree.
fn clear_at(path: &Path) -> Result<(), String> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(_) => Err("The offline profile could not be removed.".into()),
    }
}

/// Create (or repair) the private directory the profile lives in as 0700.
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

fn private_mode(path: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const ORIGIN: &str = "https://origin-a.example.org";
    const OTHER_ORIGIN: &str = "https://origin-b.example.org";

    fn account() -> String {
        uuid::Uuid::new_v4().to_string()
    }

    fn mode(path: &Path) -> u32 {
        #[cfg(unix)]
        let mode = {
            use std::os::unix::fs::PermissionsExt;
            fs::metadata(path).unwrap().permissions().mode() & 0o777
        };
        #[cfg(not(unix))]
        let mode = {
            let _ = path;
            0o600
        };
        mode
    }

    #[test]
    fn profile_round_trips_private_and_carries_no_secret() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("vela/offline/profile.json");
        assert_eq!(load_from(&path).unwrap(), None, "a missing profile is None");
        let account = account();
        save_to(&path, ORIGIN, &account).unwrap();
        assert_eq!(
            load_from(&path).unwrap(),
            Some(OfflineProfile {
                origin: format!("{ORIGIN}/"),
                account_id: account.clone(),
            }),
            "the stored origin is the canonical spelling, not the typed one"
        );
        assert_eq!(mode(&path), 0o600, "the profile is owner-only");
        assert_eq!(
            mode(path.parent().unwrap()),
            0o700,
            "the offline tree stays owner-only"
        );
        // The document holds exactly the three documented fields.
        let stored: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        let mut keys = stored
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        keys.sort();
        assert_eq!(keys, vec!["account_id", "origin", "version"]);
        assert_eq!(stored["version"], serde_json::Value::from(1));
        let text = String::from_utf8(fs::read(&path).unwrap()).unwrap();
        for forbidden in [
            "token", "password", "csrf", "audio", "username", "secret", ".flac", ".mp3",
        ] {
            assert!(
                !text.to_ascii_lowercase().contains(forbidden),
                "the profile must not carry {forbidden}: {text}"
            );
        }
        // A repeated write replaces the same document instead of accumulating.
        save_to(&path, ORIGIN, &account).unwrap();
        let entries = fs::read_dir(path.parent().unwrap())
            .unwrap()
            .filter_map(Result::ok)
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        assert_eq!(entries, vec!["profile.json".to_owned()]);
    }

    /// AUD-05: an explicit sign-out supersedes a profile write that a sign-in had
    /// already claimed, so the withdrawn hint cannot reappear afterwards. The rule is
    /// the same attempt/generation rule the saved session uses (AUD-01); it is driven
    /// here without touching a real data directory.
    #[test]
    fn a_sign_out_supersedes_an_earlier_sign_in_write() {
        let first = begin_attempt();
        assert!(is_current(first));
        let second = begin_attempt();
        assert!(!is_current(first), "a newer sign-in owns the document");
        assert!(is_current(second));
        supersede();
        assert!(
            !is_current(second),
            "an explicit sign-out wins over a pending write"
        );
    }

    #[test]
    fn unrepresentable_identities_are_never_stored() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("vela/offline/profile.json");
        let account_id = account();
        for origin in [
            "",
            "http://origin-a.example.org",
            "https://origin-a.example.org/library",
            "https://user:secret@origin-a.example.org",
            " https://origin-a.example.org",
        ] {
            assert!(
                save_to(&path, origin, &account_id).is_err(),
                "{origin:?} must not be stored"
            );
        }
        for invalid in ["", "not-a-uuid", &account_id.to_uppercase()] {
            assert!(
                save_to(&path, ORIGIN, invalid).is_err(),
                "{invalid:?} must not be stored"
            );
        }
        assert!(!path.exists(), "a refused write creates no document");
        // Numeric loopback HTTP is a canonical local origin.
        save_to(&path, "http://127.0.0.1:8080", &account_id).unwrap();
        assert_eq!(
            load_from(&path).unwrap().unwrap().origin,
            "http://127.0.0.1:8080/".to_owned()
        );
    }

    #[test]
    fn corrupt_foreign_and_oversized_profiles_are_ignored_and_kept() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("vela/offline/profile.json");
        let account = account();
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let documents = [
            // A foreign field: a future build, a hand edit or an older writer must
            // not smuggle a credential of any kind into this document.
            format!(
                "{{\"version\":1,\"origin\":\"{ORIGIN}/\",\"account_id\":\"{account}\",\"token\":\"{}\"}}",
                "a".repeat(64)
            ),
            "not json at all".to_owned(),
            "{}".to_owned(),
            format!("{{\"version\":2,\"origin\":\"{ORIGIN}/\",\"account_id\":\"{account}\"}}"),
            format!(
                "{{\"version\":1,\"origin\":\"http://origin-a.example.org/\",\"account_id\":\"{account}\"}}"
            ),
            format!("{{\"version\":1,\"origin\":\"{ORIGIN}/\",\"account_id\":\"not-a-uuid\"}}"),
            format!("{{\"version\":1,\"origin\":\"{ORIGIN}/\"}}"),
        ];
        for document in documents {
            fs::write(&path, document.as_bytes()).unwrap();
            let before = fs::read(&path).unwrap();
            assert_eq!(
                load_from(&path).unwrap(),
                None,
                "an unusable profile is ignored: {document}"
            );
            assert_eq!(
                fs::read(&path).unwrap(),
                before,
                "loading never rewrites the document it ignored"
            );
        }
        // An oversized file is rejected before it is parsed into memory.
        let oversized = vec![b' '; MAX_BYTES as usize + 1];
        fs::write(&path, &oversized).unwrap();
        assert_eq!(load_from(&path).unwrap(), None);
        assert_eq!(fs::read(&path).unwrap(), oversized);
        // A directory where the document should be is not a profile either.
        fs::remove_file(&path).unwrap();
        fs::create_dir(&path).unwrap();
        assert_eq!(load_from(&path).unwrap(), None);
    }

    /// AUD-05/AUD-06: the document only ever names the namespace it was written
    /// for, and neither loading nor clearing a profile touches anything else in the
    /// private tree. The synthetic entries below stand in for a saved copy pair, so
    /// this proof never writes to the real user data directory.
    #[test]
    fn a_foreign_origin_profile_names_no_other_namespace_and_keeps_the_tree() {
        let directory = tempfile::tempdir().unwrap();
        let base = directory.path().join("vela/offline");
        let path = base.join("profile.json");
        // A namespace-shaped tree for two identities, exactly as `offline.rs`
        // derives it: <blake3(canonical origin)>/<account-uuid>/<file>.
        let origin_a = canonical_origin(ORIGIN).unwrap();
        let origin_b = canonical_origin(OTHER_ORIGIN).unwrap();
        let account_a = account();
        let account_b = account();
        let mut fixtures = Vec::new();
        for (origin, account_id) in [(&origin_a, &account_a), (&origin_b, &account_b)] {
            let root = base
                .join(blake3::hash(origin.as_str().as_bytes()).to_hex())
                .join(account_id);
            fs::create_dir_all(&root).unwrap();
            let audio = root.join("00000000-0000-4000-8000-000000000001.audio");
            fs::write(&audio, b"private-device-copy").unwrap();
            fixtures.push(audio);
        }
        // A profile for origin B is stored and read back as origin B; it can never be
        // reinterpreted as origin A's namespace.
        save_to(&path, OTHER_ORIGIN, &account_b).unwrap();
        let profile = load_from(&path).unwrap().unwrap();
        assert_eq!(profile.origin, origin_b.as_str());
        assert_eq!(profile.account_id, account_b);
        assert_ne!(profile.origin, origin_a.as_str());
        // Nothing in the tree but the document itself was created or rewritten.
        for fixture in &fixtures {
            assert_eq!(fs::read(fixture).unwrap(), b"private-device-copy");
        }
        // Clearing the profile withdraws the hint only, idempotently.
        clear_at(&path).unwrap();
        assert_eq!(load_from(&path).unwrap(), None);
        clear_at(&path).unwrap();
        for fixture in &fixtures {
            assert_eq!(fs::read(fixture).unwrap(), b"private-device-copy");
        }
        // A document naming a non-canonical origin is ignored and left in place: the
        // saved copies of whatever origin it claimed to name stay untouched.
        let foreign = format!(
            "{{\"version\":1,\"origin\":\"http://origin-a.example.org/\",\"account_id\":\"{account_a}\"}}"
        );
        fs::write(&path, foreign.as_bytes()).unwrap();
        assert_eq!(load_from(&path).unwrap(), None);
        assert_eq!(fs::read(&path).unwrap(), foreign.as_bytes());
        for fixture in &fixtures {
            assert_eq!(fs::read(fixture).unwrap(), b"private-device-copy");
        }
    }
}
