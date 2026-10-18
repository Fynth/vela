//! Explicit, private, account-scoped offline copies of owner-authorized originals.
//!
//! Only completed originals already validated by the native HTTP client are copied.
//! The cache stores public catalog metadata and audio bytes, never credentials,
//! session tokens or listening events. A manifest and its audio are published atomically
//! under a private per-account directory; corrupt/incomplete entries are ignored.
//!
//! Every operation that touches audio bytes or manifests takes a cooperative
//! cancellation probe and is meant to run in a blocking worker, never on the native
//! UI thread (AUD-03). The probe is only consulted between bounded blocks, so a
//! canceled operation stops within one block and publishes nothing.
use crate::PreparedAudio;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use vela_domain::Track;

const MAX_TRACKS: usize = 100;
const MAX_CACHE_BYTES: u64 = 2 * 1024 * 1024 * 1024;
/// A manifest is public catalog metadata, so anything larger is neither read nor
/// written. The cap is also the byte reservation kept for one manifest (AUD-12).
const MAX_MANIFEST_BYTES: usize = 64 * 1024;
/// Bounded streaming block. Cancellation is checked between blocks (AUD-03) and
/// every buffer is this size, never a length claimed by a file.
const BLOCK_BYTES: usize = 64 * 1024;
/// Directory entries inspected per pass: one account directory only holds a
/// manifest, an audio file and this crate's own staging/backup names per copy.
const MAX_DIRECTORY_ENTRIES: usize = 4096;
/// Manifests parsed per load. Deliberately larger than `MAX_TRACKS` so corrupt or
/// unreferenced manifests cannot hide healthy copies behind a fixed cut (AUD-12).
const MAX_MANIFEST_SCAN: usize = MAX_TRACKS * 4;

/// Returned when the cancellation probe stopped the operation. Callers must not
/// report it as a file-system failure.
pub const CANCELED: &str = "The offline operation was canceled.";
const UNAVAILABLE: &str = "This track cannot be saved offline.";
const FULL: &str = "The offline cache is full. Remove space or use the web app.";
const READ_ORIGINAL: &str = "The prepared original could not be read.";
const WRITE_COPY: &str = "The offline copy could not be written.";
const WRITE_MANIFEST: &str = "The offline manifest could not be written.";
const READ_COPY: &str = "The offline copy could not be read.";
const MISSING_COPY: &str = "The offline copy is missing. Save it again from the library.";
const UNREADABLE_CACHE: &str = "Saved offline music could not be read.";
const UNMEASURABLE_CACHE: &str = "The offline cache could not be measured.";

#[derive(Clone)]
pub struct OfflineTrack {
    pub track: Track,
    pub saved_at: u64,
    pub length_bytes: u64,
    /// BLAKE3 of the copied device bytes; None only for legacy v1 manifests.
    pub digest: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct VerifyReport {
    pub checked: usize,
    pub healthy: usize,
    pub unverified: usize,
    pub corrupt: Vec<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
    version: u8,
    account_id: String,
    saved_at: u64,
    length_bytes: u64,
    track: Track,
    #[serde(default)]
    digest: String,
}

fn data_root(account_id: &str) -> Result<PathBuf, String> {
    if account_id.parse::<uuid::Uuid>().is_err() {
        return Err("Offline copies require a valid account.".into());
    }
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| {
            std::env::var_os("HOME")
                .map(PathBuf::from)
                .map(|home| home.join(".local").join("share"))
        })
        .ok_or_else(|| "No local data directory is available for offline copies.".to_owned())?;
    Ok(base.join("vela").join("offline").join(account_id))
}

pub fn load(account_id: &str) -> Result<Vec<OfflineTrack>, String> {
    load_from(&data_root(account_id)?, account_id)
}

pub fn audio_path(account_id: &str, track_id: &str) -> Result<PathBuf, String> {
    let root = data_root(account_id)?;
    let path = audio_path_from(&root, track_id)?;
    if !path.is_file() {
        return Err(MISSING_COPY.into());
    }
    Ok(path)
}

pub fn remove(account_id: &str, track_id: &str) -> Result<(), String> {
    remove_from(&data_root(account_id)?, track_id)
}

fn remove_from(root: &Path, track_id: &str) -> Result<(), String> {
    let audio = audio_path_from(root, track_id)?;
    let manifest = root.join(format!("{track_id}.json"));
    for path in [audio, manifest] {
        match fs::remove_file(&path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return Err("The offline copy could not be removed.".into()),
        }
    }
    Ok(())
}

/// One explicitly requested copy. The probe is consulted between blocks so the
/// caller can stop the copy without losing an already healthy previous copy.
pub fn save(
    account_id: &str,
    track: &Track,
    prepared: &PreparedAudio,
    canceled: &dyn Fn() -> bool,
) -> Result<(), String> {
    save_to(
        &data_root(account_id)?,
        account_id,
        track,
        prepared,
        canceled,
    )
}

fn load_from(root: &Path, account_id: &str) -> Result<Vec<OfflineTrack>, String> {
    let entries = match fs::read_dir(root) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(_) => return Err(UNREADABLE_CACHE.into()),
    };
    // Only `.json` names are candidates; audio files, this crate's staging names
    // and unrelated orphan entries are skipped before the bounded cut, so a
    // directory full of audio files cannot hide the manifests (AUD-12).
    let mut manifests = entries
        .filter_map(Result::ok)
        .take(MAX_DIRECTORY_ENTRIES)
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|value| value.to_str()) == Some("json"))
        .collect::<Vec<_>>();
    // Sorting makes which manifests are parsed deterministic instead of
    // dependent on the order the file system happens to return them in.
    manifests.sort();
    manifests.truncate(MAX_MANIFEST_SCAN);
    let mut tracks = Vec::new();
    for path in manifests {
        let Some(stem) = path.file_stem().and_then(|value| value.to_str()) else {
            continue;
        };
        let Ok(bytes) = read_manifest(&path) else {
            continue;
        };
        let Ok(record) = serde_json::from_slice::<Record>(&bytes) else {
            continue;
        };
        let digest_valid = match record.version {
            1 => true,
            2 => {
                record.digest.len() == 64
                    && record.digest.bytes().all(|byte| byte.is_ascii_hexdigit())
            }
            _ => false,
        };
        if !digest_valid
            || record.account_id != account_id
            || record.track.id != stem
            || record.length_bytes == 0
            || record.length_bytes != record.track.size_bytes as u64
            || record.length_bytes > crate::client::MAX_MEDIA_BYTES
            || record.track.duration_ms <= 0
            || fs::metadata(root.join(format!("{stem}.audio")))
                .is_ok_and(|metadata| !metadata.is_file() || metadata.len() != record.length_bytes)
        {
            continue;
        }
        tracks.push(OfflineTrack {
            track: record.track,
            saved_at: record.saved_at,
            length_bytes: record.length_bytes,
            digest: (record.version == 2).then_some(record.digest),
        });
    }
    tracks.sort_by(|left, right| {
        right
            .saved_at
            .cmp(&left.saved_at)
            .then_with(|| right.track.id.cmp(&left.track.id))
    });
    tracks.truncate(MAX_TRACKS);
    Ok(tracks)
}

/// AUD-12: the manifest length is checked before any allocation and the read is
/// clamped again, so a manifest that grows between the check and the read still
/// cannot exhaust memory.
fn read_manifest(path: &Path) -> Result<Vec<u8>, ()> {
    let metadata = fs::metadata(path).map_err(|_| ())?;
    if !metadata.is_file() || metadata.len() > MAX_MANIFEST_BYTES as u64 {
        return Err(());
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    fs::File::open(path)
        .map_err(|_| ())?
        .take(MAX_MANIFEST_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| ())?;
    if bytes.len() > MAX_MANIFEST_BYTES {
        return Err(());
    }
    Ok(bytes)
}

fn audio_path_from(root: &Path, track_id: &str) -> Result<PathBuf, String> {
    if track_id.parse::<uuid::Uuid>().is_err() {
        return Err("Select a valid track.".into());
    }
    Ok(root.join(format!("{track_id}.audio")))
}

/// Every byte this account directory already holds: published audio, manifests,
/// this crate's own staging/backup leftovers and orphans no manifest references.
/// Accounting only measures; it never deletes a file (AUD-12).
fn directory_bytes(root: &Path) -> Result<u64, String> {
    let entries = fs::read_dir(root).map_err(|_| UNMEASURABLE_CACHE.to_owned())?;
    let mut total = 0_u64;
    for entry in entries.filter_map(Result::ok).take(MAX_DIRECTORY_ENTRIES) {
        // `DirEntry::metadata` does not follow symlinks, so a link can never
        // make the cache look larger than the files it really owns.
        if let Ok(metadata) = entry.metadata()
            && metadata.is_file()
        {
            total = total.saturating_add(metadata.len());
        }
    }
    Ok(total)
}

/// Bytes of the published pair that a successful replacement releases.
fn published_pair_bytes(root: &Path, track_id: &str) -> u64 {
    let mut total = 0_u64;
    for name in [format!("{track_id}.audio"), format!("{track_id}.json")] {
        if let Ok(metadata) = fs::metadata(root.join(name))
            && metadata.is_file()
        {
            total = total.saturating_add(metadata.len());
        }
    }
    total
}

/// AUD-12: what the account directory may hold once the staged copy publishes.
/// A replacement only releases its own previous pair, so this charges the delta
/// instead of pretending the whole cache is empty.
fn projected_usage(used: u64, released: u64, reservation: u64) -> u64 {
    used.saturating_sub(released).saturating_add(reservation)
}

/// Unique per operation and process, so two saves of the same track (two
/// processes, or a second attempt in one process) can never write the same
/// staging file, and a leftover staging file is never overwritten (AUD-12).
fn unique_suffix() -> String {
    format!("{}-{}", std::process::id(), uuid::Uuid::new_v4().simple())
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

fn save_to(
    root: &Path,
    account_id: &str,
    track: &Track,
    prepared: &PreparedAudio,
    canceled: &dyn Fn() -> bool,
) -> Result<(), String> {
    if canceled() {
        return Err(CANCELED.into());
    }
    if account_id.parse::<uuid::Uuid>().is_err()
        || track.id.parse::<uuid::Uuid>().is_err()
        || prepared.length_bytes == 0
        || prepared.length_bytes > crate::client::MAX_MEDIA_BYTES
        || track.size_bytes as u64 != prepared.length_bytes
    {
        return Err(UNAVAILABLE.into());
    }
    fs::create_dir_all(root).map_err(|_| WRITE_COPY.to_owned())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(root, fs::Permissions::from_mode(0o700))
            .map_err(|_| WRITE_COPY.to_owned())?;
    }
    let existing = load_from(root, account_id)?;
    let replacing = existing.iter().any(|entry| entry.track.id == track.id);
    if !replacing && existing.len() >= MAX_TRACKS {
        return Err(FULL.into());
    }
    // AUD-12: the budget covers everything already on disk, including staging
    // and orphan bytes, and a replacement only has to fit its own delta.
    let used = directory_bytes(root)?;
    let released = if replacing {
        published_pair_bytes(root, &track.id)
    } else {
        0
    };
    let reservation = prepared
        .length_bytes
        .saturating_add(MAX_MANIFEST_BYTES as u64);
    if projected_usage(used, released, reservation) > MAX_CACHE_BYTES {
        return Err(FULL.into());
    }
    let suffix = unique_suffix();
    let staged_audio = root.join(format!(".{}.{suffix}.audio.tmp", track.id));
    let staged_manifest = root.join(format!(".{}.{suffix}.json.tmp", track.id));
    let mut source = fs::File::open(prepared.path()).map_err(|_| READ_ORIGINAL.to_owned())?;
    let digest = match copy_audio(&mut source, &staged_audio, prepared.length_bytes, canceled) {
        Ok(digest) => digest,
        Err(error) => {
            let _ = fs::remove_file(&staged_audio);
            return Err(error);
        }
    };
    let record = Record {
        version: 2,
        account_id: account_id.to_owned(),
        saved_at: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_secs())
            .unwrap_or(1),
        length_bytes: prepared.length_bytes,
        track: track.clone(),
        digest,
    };
    if let Err(error) = stage_manifest(&staged_manifest, &record) {
        let _ = fs::remove_file(&staged_audio);
        let _ = fs::remove_file(&staged_manifest);
        return Err(error);
    }
    publish(root, &track.id, &staged_audio, &staged_manifest, &suffix)
}

/// Stream the prepared original into a private staging file, hashing exactly the
/// bytes written. Cancellation is checked between bounded blocks (AUD-03).
fn copy_audio(
    source: &mut fs::File,
    staged: &Path,
    expected: u64,
    canceled: &dyn Fn() -> bool,
) -> Result<String, String> {
    let mut output = private_file(staged).map_err(|_| WRITE_COPY.to_owned())?;
    let mut hasher = blake3::Hasher::new();
    let mut buffer = [0_u8; BLOCK_BYTES];
    let mut copied = 0_u64;
    loop {
        if canceled() {
            return Err(CANCELED.into());
        }
        let read = source
            .read(&mut buffer)
            .map_err(|_| READ_ORIGINAL.to_owned())?;
        if read == 0 {
            break;
        }
        copied = copied.saturating_add(read as u64);
        if copied > expected {
            return Err(WRITE_COPY.into());
        }
        hasher.update(&buffer[..read]);
        output
            .write_all(&buffer[..read])
            .map_err(|_| WRITE_COPY.to_owned())?;
    }
    if copied != expected {
        return Err(WRITE_COPY.into());
    }
    output.sync_all().map_err(|_| WRITE_COPY.to_owned())?;
    Ok(hasher.finalize().to_hex().to_string())
}

fn stage_manifest(path: &Path, record: &Record) -> Result<(), String> {
    let payload = serde_json::to_vec(record).map_err(|_| WRITE_MANIFEST.to_owned())?;
    if payload.len() > MAX_MANIFEST_BYTES {
        return Err(WRITE_MANIFEST.into());
    }
    let mut file = private_file(path).map_err(|_| WRITE_MANIFEST.to_owned())?;
    file.write_all(&payload)
        .and_then(|()| file.sync_all())
        .map_err(|_| WRITE_MANIFEST.to_owned())
}

/// Publish a staged pair. The previous pair is renamed aside first, so a failed
/// publication restores the last healthy copy instead of deleting it (AUD-12).
/// Manifests are the visibility record and are published last: an interruption
/// between the two renames leaves bytes that `load_from` rejects, never a
/// manifest that describes different audio.
fn publish(
    root: &Path,
    track_id: &str,
    staged_audio: &Path,
    staged_manifest: &Path,
    suffix: &str,
) -> Result<(), String> {
    let audio = root.join(format!("{track_id}.audio"));
    let manifest = root.join(format!("{track_id}.json"));
    let previous_audio = root.join(format!(".{track_id}.{suffix}.audio.bak"));
    let previous_manifest = root.join(format!(".{track_id}.{suffix}.json.bak"));
    let moved_audio = match fs::rename(&audio, &previous_audio) {
        Ok(()) => true,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
        Err(_) => return Err(WRITE_COPY.into()),
    };
    let moved_manifest = match fs::rename(&manifest, &previous_manifest) {
        Ok(()) => true,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
        Err(_) => {
            if moved_audio {
                let _ = fs::rename(&previous_audio, &audio);
            }
            return Err(WRITE_COPY.into());
        }
    };
    let restore = |published_audio: bool| {
        if published_audio {
            let _ = fs::remove_file(&audio);
        }
        if moved_audio {
            let _ = fs::rename(&previous_audio, &audio);
        }
        if moved_manifest {
            let _ = fs::rename(&previous_manifest, &manifest);
        }
    };
    if fs::rename(staged_audio, &audio).is_err() {
        restore(false);
        let _ = fs::remove_file(staged_manifest);
        return Err(WRITE_COPY.into());
    }
    if fs::rename(staged_manifest, &manifest).is_err() {
        restore(true);
        let _ = fs::remove_file(staged_manifest);
        return Err(WRITE_MANIFEST.into());
    }
    // Only this operation's own backups are removed: the published replacement
    // is complete, and no file this operation did not create is touched.
    if moved_audio {
        let _ = fs::remove_file(&previous_audio);
    }
    if moved_manifest {
        let _ = fs::remove_file(&previous_manifest);
    }
    Ok(())
}

pub fn verify(account_id: &str, canceled: &dyn Fn() -> bool) -> Result<VerifyReport, String> {
    verify_from(&data_root(account_id)?, account_id, canceled)
}

fn verify_from(
    root: &Path,
    account_id: &str,
    canceled: &dyn Fn() -> bool,
) -> Result<VerifyReport, String> {
    let entries = load_from(root, account_id)?;
    let mut report = VerifyReport::default();
    for entry in entries {
        if canceled() {
            return Err(CANCELED.into());
        }
        report.checked += 1;
        let Some(expected) = entry.digest.as_deref() else {
            report.unverified += 1;
            continue;
        };
        let path = audio_path_from(root, &entry.track.id)?;
        let Ok(mut file) = fs::File::open(&path) else {
            report.corrupt.push(entry.track.id);
            continue;
        };
        let mut hasher = blake3::Hasher::new();
        let mut buffer = [0_u8; BLOCK_BYTES];
        let mut failed = false;
        loop {
            if canceled() {
                return Err(CANCELED.into());
            }
            match file.read(&mut buffer) {
                Ok(0) => break,
                Ok(read) => {
                    hasher.update(&buffer[..read]);
                }
                Err(_) => {
                    failed = true;
                    break;
                }
            }
        }
        if failed || hasher.finalize().to_hex().as_str() != expected {
            report.corrupt.push(entry.track.id);
        } else {
            report.healthy += 1;
        }
    }
    Ok(report)
}

/// Verify one saved copy and hand back the very descriptor that was hashed, so
/// the decoded bytes can never differ from the verified bytes (no TOCTOU). The
/// caller owns the returned lease: dropping the result closes the descriptor.
pub fn prepare(
    account_id: &str,
    track_id: &str,
    canceled: &dyn Fn() -> bool,
) -> Result<std::sync::Arc<PreparedAudio>, String> {
    prepare_from(&data_root(account_id)?, account_id, track_id, canceled)
}

fn prepare_from(
    root: &Path,
    account_id: &str,
    track_id: &str,
    canceled: &dyn Fn() -> bool,
) -> Result<std::sync::Arc<PreparedAudio>, String> {
    if canceled() {
        return Err(CANCELED.into());
    }
    let entry = load_from(root, account_id)?
        .into_iter()
        .find(|entry| entry.track.id == track_id)
        .ok_or_else(|| MISSING_COPY.to_owned())?;
    let path = audio_path_from(root, track_id)?;
    let symlink = fs::symlink_metadata(&path)
        .map_err(|_| "The offline copy is missing. Save it again.".to_owned())?;
    if symlink.file_type().is_symlink() {
        return Err("The offline copy is not a regular file. Save it again.".into());
    }
    // Open once and hash that same descriptor, then rewind it for decoding. A
    // path replacement between verification and open cannot substitute bytes.
    let mut file = fs::File::open(&path)
        .map_err(|_| "The offline copy could not be opened. Save it again.".to_owned())?;
    let metadata = file.metadata().map_err(|_| READ_COPY.to_owned())?;
    if !metadata.is_file() || metadata.len() != entry.length_bytes {
        return Err("The offline copy does not match its manifest. Save it again.".into());
    }
    if let Some(expected) = entry.digest.as_deref() {
        let mut hasher = blake3::Hasher::new();
        let mut buffer = [0_u8; BLOCK_BYTES];
        loop {
            // AUD-03: a 250 MiB copy spans many blocks, so Cancel must win
            // between blocks instead of after the whole file.
            if canceled() {
                return Err(CANCELED.into());
            }
            let read = file.read(&mut buffer).map_err(|_| READ_COPY.to_owned())?;
            if read == 0 {
                break;
            }
            hasher.update(&buffer[..read]);
        }
        if hasher.finalize().to_hex().as_str() != expected {
            return Err("The offline copy failed its integrity check. Save it again.".into());
        }
        file.seek(SeekFrom::Start(0))
            .map_err(|_| READ_COPY.to_owned())?;
    }
    #[cfg(target_os = "linux")]
    {
        Ok(std::sync::Arc::new(PreparedAudio::from_file(
            file,
            metadata.len(),
        )))
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = file;
        Err("Offline playback is currently available on Linux only.".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    fn track(id: uuid::Uuid, length: u64) -> Track {
        Track {
            id: id.to_string(),
            title: "Offline Signal".into(),
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
            size_bytes: length as i64,
            artwork_id: None,
            added_at: 1,
            favorite: false,
        }
    }

    fn never() -> impl Fn() -> bool {
        || false
    }

    /// A prepared original of `length` bytes with recognizable content. The
    /// lease has to stay alive for the whole save.
    fn prepared(length: usize) -> PreparedAudio {
        let mut original = tempfile::NamedTempFile::new().unwrap();
        let bytes = (0..length).map(|at| (at % 251) as u8).collect::<Vec<_>>();
        original.write_all(&bytes).unwrap();
        original.flush().unwrap();
        PreparedAudio::new(original, length as u64)
    }

    fn saved(root: &Path, account: &str, id: uuid::Uuid, bytes: usize) -> PreparedAudio {
        let prepared = prepared(bytes);
        save_to(root, account, &track(id, bytes as u64), &prepared, &never()).unwrap();
        prepared
    }

    /// Comparable projection of a loaded list (the public struct is not `Eq`).
    fn summary(tracks: &[OfflineTrack]) -> Vec<(String, u64, Option<String>)> {
        tracks
            .iter()
            .map(|entry| {
                (
                    entry.track.id.clone(),
                    entry.length_bytes,
                    entry.digest.clone(),
                )
            })
            .collect()
    }

    fn names(root: &Path) -> Vec<String> {
        let mut names = fs::read_dir(root)
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
    fn offline_copy_round_trips_private_exact_bytes_and_ignores_tampering() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().join("offline");
        let account = uuid::Uuid::new_v4().to_string();
        let id = uuid::Uuid::new_v4();
        let bytes = b"offline-audio-fixture".to_vec();
        let mut original = tempfile::NamedTempFile::new().unwrap();
        original.write_all(&bytes).unwrap();
        original.flush().unwrap();
        let prepared = PreparedAudio::new(original, bytes.len() as u64);
        save_to(
            &root,
            &account,
            &track(id, bytes.len() as u64),
            &prepared,
            &never(),
        )
        .unwrap();
        let loaded = load_from(&root, &account).unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].track.id, id.to_string());
        assert!(
            loaded[0]
                .digest
                .as_deref()
                .is_some_and(|digest| digest.len() == 64)
        );
        assert_eq!(fs::read(root.join(format!("{id}.audio"))).unwrap(), bytes);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = |path: PathBuf| fs::metadata(path).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode(root.join(format!("{id}.audio"))), 0o600);
            assert_eq!(mode(root.join(format!("{id}.json"))), 0o600);
            assert_eq!(mode(root.clone()), 0o700);
        }
        assert!(
            load_from(&root, &uuid::Uuid::new_v4().to_string())
                .unwrap()
                .is_empty()
        );
        let mut tampered = bytes.clone();
        tampered[0] ^= 0x7f;
        fs::write(root.join(format!("{id}.audio")), &tampered).unwrap();
        let report = verify_from(&root, &account, &never()).unwrap();
        assert_eq!(report.checked, 1);
        assert_eq!(report.corrupt, vec![id.to_string()]);
        assert!(prepare_from(&root, &account, &id.to_string(), &never()).is_err());
    }

    #[test]
    fn prepare_is_canceled_between_blocks_and_returns_no_lease() {
        const BLOCKS: usize = 8;
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().join("offline");
        let account = uuid::Uuid::new_v4().to_string();
        let id = uuid::Uuid::new_v4();
        saved(&root, &account, id, BLOCKS * BLOCK_BYTES);
        let track_id = id.to_string();

        // An already-canceled probe stops before the copy is opened.
        assert_eq!(
            prepare_from(&root, &account, &track_id, &|| true).unwrap_err(),
            CANCELED
        );

        // Cancelling mid-file stops at a block boundary, not after the hash.
        let calls = Cell::new(0_usize);
        assert_eq!(
            prepare_from(&root, &account, &track_id, &|| {
                calls.set(calls.get() + 1);
                calls.get() > 3
            })
            .unwrap_err(),
            CANCELED
        );
        assert_eq!(calls.get(), 4, "the probe is consulted once per block");

        // The same copy still verifies: a canceled preparation left no state.
        let calls = Cell::new(0_usize);
        let prepared = prepare_from(&root, &account, &track_id, &|| {
            calls.set(calls.get() + 1);
            false
        })
        .unwrap();
        assert_eq!(prepared.length_bytes, (BLOCKS * BLOCK_BYTES) as u64);
        assert!(
            calls.get() >= BLOCKS,
            "every hashed block is cancellable, saw {} checks",
            calls.get()
        );
    }

    #[test]
    fn verify_is_canceled_without_reporting_corruption() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().join("offline");
        let account = uuid::Uuid::new_v4().to_string();
        let id = uuid::Uuid::new_v4();
        saved(&root, &account, id, 3 * BLOCK_BYTES);
        assert_eq!(
            verify_from(&root, &account, &|| true).unwrap_err(),
            CANCELED
        );
        let report = verify_from(&root, &account, &never()).unwrap();
        assert_eq!(report.checked, 1);
        assert_eq!(report.healthy, 1);
        assert!(report.corrupt.is_empty());
        assert_eq!(report.unverified, 0);
    }

    /// Measurement, not part of the default gate: how much work AUD-03 moved off
    /// the native UI thread, and how quickly a cancellation is observed. Run it
    /// with `cargo test -p vela-desktop -- --ignored --nocapture offline_cost`.
    #[test]
    #[ignore = "measurement: run explicitly with --ignored --nocapture"]
    fn offline_copy_cost_and_cancel_latency() {
        use std::time::Instant;
        // `VELA_OFFLINE_MEASURE_MIB` lets a run use the 250 MiB client cap.
        let size_mib = std::env::var("VELA_OFFLINE_MEASURE_MIB")
            .ok()
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(96)
            .clamp(1, 250);
        let size = size_mib * 1024 * 1024;
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().join("offline");
        let account = uuid::Uuid::new_v4().to_string();
        let id = uuid::Uuid::new_v4();
        let started = Instant::now();
        saved(&root, &account, id, size);
        let copy = started.elapsed();
        let track_id = id.to_string();
        let started = Instant::now();
        let prepared = prepare_from(&root, &account, &track_id, &never()).unwrap();
        let verify = started.elapsed();
        assert_eq!(prepared.length_bytes, size as u64);
        // A cancellation is observed at the fourth block: 0.25 MiB of the file.
        let cancels_at = Cell::new(None);
        let calls = Cell::new(0_usize);
        let started = Instant::now();
        assert_eq!(
            prepare_from(&root, &account, &track_id, &|| {
                calls.set(calls.get() + 1);
                if calls.get() == 4 {
                    cancels_at.set(Some(Instant::now()));
                }
                calls.get() > 3
            })
            .unwrap_err(),
            CANCELED
        );
        let latency = cancels_at
            .take()
            .map(|at| at.saturating_duration_since(started))
            .unwrap_or_default();
        println!(
            "offline measurement: {size_mib} MiB copy={copy:?} full-verify={verify:?} cancel-latency={latency:?} (4 blocks)"
        );
        assert!(
            latency * 8 < verify,
            "cancellation must be far cheaper than the whole verification ({latency:?} vs {verify:?})"
        );
    }

    #[test]
    fn oversized_manifest_is_ignored_before_it_is_read() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().join("offline");
        let account = uuid::Uuid::new_v4().to_string();
        let id = uuid::Uuid::new_v4();
        saved(&root, &account, id, 1_024);
        let manifest = root.join(format!("{id}.json"));
        let mut payload = fs::read(&manifest).unwrap();
        assert!(payload.len() < MAX_MANIFEST_BYTES);
        // Trailing whitespace keeps the JSON valid, so only the byte cap can
        // stop this manifest from being accepted.
        payload.resize(MAX_MANIFEST_BYTES + 1, b' ');
        fs::write(&manifest, &payload).unwrap();
        assert!(fs::metadata(&manifest).unwrap().len() > MAX_MANIFEST_BYTES as u64);
        assert!(
            load_from(&root, &account).unwrap().is_empty(),
            "a manifest above the cap is never parsed"
        );
    }

    #[test]
    fn quota_counts_staging_and_orphan_bytes_without_deleting_them() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().join("offline");
        let account = uuid::Uuid::new_v4().to_string();
        let id = uuid::Uuid::new_v4();
        let original = prepared(4_096);
        save_to(&root, &account, &track(id, 4_096), &original, &never()).unwrap();
        let healthy = directory_bytes(&root).unwrap();
        assert!(healthy > 4_096);

        // Leftovers no manifest references: an interrupted save and an orphan.
        // Accounting must count both (AUD-12).
        let staging = root.join(".leftover.audio.tmp");
        fs::write(&staging, vec![7_u8; 1_024]).unwrap();
        let orphan = root.join("orphan.bin");
        let sparse = fs::File::create(&orphan).unwrap();
        // A sparse file costs no test disk but reports its honest logical size.
        sparse.set_len(MAX_CACHE_BYTES).unwrap();
        drop(sparse);
        assert_eq!(
            directory_bytes(&root).unwrap(),
            healthy + 1_024 + MAX_CACHE_BYTES
        );

        // Only the manifest reservation is left, so the next save is refused
        // instead of silently exceeding the cache budget.
        let second = prepared(4_096);
        let other = uuid::Uuid::new_v4();
        assert_eq!(
            save_to(&root, &account, &track(other, 4_096), &second, &never()).unwrap_err(),
            FULL
        );
        // Nothing was deleted to make room: the healthy copy, the leftover
        // staging file and the orphan are all still there.
        assert_eq!(load_from(&root, &account).unwrap().len(), 1);
        assert!(staging.is_file());
        assert!(orphan.is_file());
        assert!(!root.join(format!("{other}.audio")).exists());
    }

    #[test]
    fn quota_charges_only_the_replacement_delta() {
        let reserve = MAX_MANIFEST_BYTES as u64 + 4_096;
        // Near the budget, replacing a copy of the same size still fits because
        // a successful publish releases the pair it replaced.
        assert_eq!(
            projected_usage(MAX_CACHE_BYTES - reserve, reserve, reserve),
            MAX_CACHE_BYTES - reserve
        );
        assert!(projected_usage(MAX_CACHE_BYTES - reserve + 1, 0, reserve) > MAX_CACHE_BYTES);
        assert!(projected_usage(MAX_CACHE_BYTES, 0, 1) > MAX_CACHE_BYTES);
        // A released amount never makes the projection negative.
        assert_eq!(projected_usage(0, 4_096, 0), 0);
    }

    #[test]
    fn replacement_keeps_the_previous_healthy_copy_when_publication_fails() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().join("offline");
        let account = uuid::Uuid::new_v4().to_string();
        let id = uuid::Uuid::new_v4();
        saved(&root, &account, id, 2_048);
        let before = load_from(&root, &account).unwrap();
        let audio = fs::read(root.join(format!("{id}.audio"))).unwrap();

        // Publication fails after the previous pair was moved aside: the staged
        // manifest never appears, so the published copy must be restored.
        let staged_audio = root.join(".staged.audio.tmp");
        fs::write(&staged_audio, vec![9_u8; 2_048]).unwrap();
        let missing_manifest = root.join(".staged.json.tmp");
        assert_eq!(
            publish(
                &root,
                &id.to_string(),
                &staged_audio,
                &missing_manifest,
                "test"
            )
            .unwrap_err(),
            WRITE_MANIFEST
        );
        assert_eq!(fs::read(root.join(format!("{id}.audio"))).unwrap(), audio);
        assert_eq!(
            summary(&load_from(&root, &account).unwrap()),
            summary(&before)
        );
        assert_eq!(verify_from(&root, &account, &never()).unwrap().healthy, 1);
        // Only our own staging file was cleaned up, and no backup lingers.
        assert!(!staged_audio.exists());
        assert!(
            names(&root).iter().all(|name| !name.ends_with(".bak")),
            "the restored pair leaves no backup behind: {:?}",
            names(&root)
        );
    }

    #[test]
    fn replacement_publishes_new_bytes_and_cleans_its_own_staging() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().join("offline");
        let account = uuid::Uuid::new_v4().to_string();
        let id = uuid::Uuid::new_v4();
        saved(&root, &account, id, 1_024);
        let first = load_from(&root, &account).unwrap()[0].digest.clone();
        let replacement = prepared(3_072);
        save_to(&root, &account, &track(id, 3_072), &replacement, &never()).unwrap();
        let second = load_from(&root, &account).unwrap();
        assert_eq!(second.len(), 1, "a replacement never duplicates the copy");
        assert_eq!(second[0].length_bytes, 3_072);
        assert_ne!(second[0].digest, first);
        assert_eq!(verify_from(&root, &account, &never()).unwrap().healthy, 1);
        assert_eq!(
            names(&root),
            vec![format!("{id}.audio"), format!("{id}.json")],
            "no staging or backup file survives a successful publish"
        );
    }

    #[test]
    fn unique_staging_names_never_clobber_foreign_leftovers() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().join("offline");
        let account = uuid::Uuid::new_v4().to_string();
        let id = uuid::Uuid::new_v4();
        fs::create_dir_all(&root).unwrap();
        // The fixed staging names an older build used, left behind by a crash.
        let old_staging = root.join(format!(".{id}.audio.tmp"));
        fs::write(&old_staging, b"interrupted-save").unwrap();
        let old_manifest_staging = root.join(format!(".{id}.json.tmp"));
        fs::write(&old_manifest_staging, b"{}").unwrap();
        saved(&root, &account, id, 512);
        assert_eq!(fs::read(&old_staging).unwrap(), b"interrupted-save");
        assert_eq!(fs::read(&old_manifest_staging).unwrap(), b"{}");
        assert_eq!(load_from(&root, &account).unwrap().len(), 1);
    }

    #[test]
    fn canceled_save_publishes_nothing() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().join("offline");
        let account = uuid::Uuid::new_v4().to_string();
        let id = uuid::Uuid::new_v4();
        let prepared = prepared(4 * BLOCK_BYTES);
        // A size that disagrees with the manifest is rejected before any work.
        assert_eq!(
            save_to(&root, &account, &track(id, 0), &prepared, &never()).unwrap_err(),
            UNAVAILABLE
        );
        let calls = Cell::new(0_usize);
        let result = save_to(
            &root,
            &account,
            &track(id, (4 * BLOCK_BYTES) as u64),
            &prepared,
            &|| {
                calls.set(calls.get() + 1);
                calls.get() > 2
            },
        );
        assert_eq!(result.unwrap_err(), CANCELED);
        assert!(calls.get() > 2, "the copy is cancelled between blocks");
        assert!(load_from(&root, &account).unwrap().is_empty());
        assert_eq!(
            names(&root)
                .iter()
                .filter(|name| name.contains("tmp"))
                .count(),
            0,
            "a canceled save removes its own staging file: {:?}",
            names(&root)
        );
        assert!(!root.join(format!("{id}.audio")).exists());
        // The same prepared original can still be saved afterwards.
        save_to(
            &root,
            &account,
            &track(id, (4 * BLOCK_BYTES) as u64),
            &prepared,
            &never(),
        )
        .unwrap();
        assert_eq!(load_from(&root, &account).unwrap().len(), 1);
    }
}
