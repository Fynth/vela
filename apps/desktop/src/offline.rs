//! Explicit, private, account-scoped offline copies of owner-authorized originals.
//!
//! Only completed originals already validated by the native HTTP client are copied.
//! The cache stores public catalog metadata and audio bytes, never credentials,
//! session tokens or listening events. A manifest and its audio are written atomically
//! under a private per-account directory; corrupt/incomplete entries are ignored.
use crate::PreparedAudio;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use vela_domain::Track;

const MAX_TRACKS: usize = 100;
const MAX_CACHE_BYTES: u64 = 2 * 1024 * 1024 * 1024;

#[derive(Clone)]
pub struct OfflineTrack {
    pub track: Track,
    pub saved_at: u64,
    pub length_bytes: u64,
    /// BLAKE3 of the copied device bytes; None only for legacy v1 manifests.
    pub digest: Option<String>,
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
        return Err("The offline copy is missing. Save it again from the library.".into());
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

pub fn save(account_id: &str, track: &Track, prepared: &PreparedAudio) -> Result<(), String> {
    save_to(&data_root(account_id)?, account_id, track, prepared)
}

fn load_from(root: &Path, account_id: &str) -> Result<Vec<OfflineTrack>, String> {
    let entries = match fs::read_dir(root) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(_) => return Err("Saved offline music could not be read.".into()),
    };
    let mut tracks = Vec::new();
    for entry in entries.filter_map(Result::ok).take(MAX_TRACKS * 2) {
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some("json") {
            continue;
        }
        let Some(stem) = path.file_stem().and_then(|value| value.to_str()) else {
            continue;
        };
        let Ok(bytes) = fs::read(&path) else {
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

fn audio_path_from(root: &Path, track_id: &str) -> Result<PathBuf, String> {
    if track_id.parse::<uuid::Uuid>().is_err() {
        return Err("Select a valid track.".into());
    }
    Ok(root.join(format!("{track_id}.audio")))
}

fn save_to(
    root: &Path,
    account_id: &str,
    track: &Track,
    prepared: &PreparedAudio,
) -> Result<(), String> {
    if account_id.parse::<uuid::Uuid>().is_err()
        || track.id.parse::<uuid::Uuid>().is_err()
        || prepared.length_bytes == 0
        || prepared.length_bytes > crate::client::MAX_MEDIA_BYTES
        || track.size_bytes as u64 != prepared.length_bytes
    {
        return Err("This track cannot be saved offline.".into());
    }
    let existing = load_from(root, account_id)?;
    let existing_bytes = existing.iter().map(|track| track.length_bytes).sum::<u64>();
    if !existing.iter().any(|entry| entry.track.id == track.id)
        && (existing.len() >= MAX_TRACKS
            || existing_bytes.saturating_add(prepared.length_bytes) > MAX_CACHE_BYTES)
    {
        return Err("The offline cache is full. Remove space or use the web app.".into());
    }
    fs::create_dir_all(root).map_err(|_| "The offline copy could not be written.".to_owned())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(root, fs::Permissions::from_mode(0o700))
            .map_err(|_| "The offline copy could not be written.".to_owned())?;
    }
    let audio = root.join(format!("{}.audio", track.id));
    let temporary_audio = root.join(format!(".{}.audio.tmp", track.id));
    let mut source = fs::File::open(prepared.path())
        .map_err(|_| "The prepared original could not be read.".to_owned())?;
    #[cfg(unix)]
    let output = {
        use std::os::unix::fs::OpenOptionsExt;
        fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(&temporary_audio)
    };
    #[cfg(not(unix))]
    let mut output = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&temporary_audio);
    let copy = (|| -> std::io::Result<String> {
        let mut output = output?;
        let mut hasher = blake3::Hasher::new();
        let mut buffer = [0_u8; 64 * 1024];
        let mut copied = 0_u64;
        loop {
            let read = std::io::Read::read(&mut source, &mut buffer)?;
            if read == 0 {
                break;
            }
            copied = copied.saturating_add(read as u64);
            if copied > prepared.length_bytes {
                return Err(std::io::Error::other("offline copy size mismatch"));
            }
            hasher.update(&buffer[..read]);
            std::io::Write::write_all(&mut output, &buffer[..read])?;
        }
        if copied != prepared.length_bytes {
            return Err(std::io::Error::other("offline copy size mismatch"));
        }
        output.sync_all()?;
        fs::rename(&temporary_audio, &audio)?;
        Ok(hasher.finalize().to_hex().to_string())
    })();
    let digest = match copy {
        Ok(digest) => digest,
        Err(_) => {
            let _ = fs::remove_file(&temporary_audio);
            return Err("The offline copy could not be written.".into());
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
    let payload = serde_json::to_vec(&record)
        .map_err(|_| "The offline manifest could not be written.".to_owned())?;
    let manifest = root.join(format!("{}.json", track.id));
    let temporary_manifest = root.join(format!(".{}.json.tmp", track.id));
    let write_manifest = || -> std::io::Result<()> {
        #[cfg(unix)]
        let mut file = {
            use std::os::unix::fs::OpenOptionsExt;
            fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .mode(0o600)
                .open(&temporary_manifest)?
        };
        #[cfg(not(unix))]
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&temporary_manifest)?;
        file.write_all(&payload)?;
        file.sync_all()?;
        fs::rename(&temporary_manifest, &manifest)?;
        Ok(())
    };
    if write_manifest().is_err() {
        let _ = fs::remove_file(&temporary_manifest);
        let _ = fs::remove_file(&audio);
        return Err("The offline manifest could not be written.".into());
    }
    Ok(())
}

pub fn prepare(account_id: &str, track_id: &str) -> Result<std::sync::Arc<PreparedAudio>, String> {
    prepare_from(&data_root(account_id)?, account_id, track_id)
}

fn prepare_from(
    root: &Path,
    account_id: &str,
    track_id: &str,
) -> Result<std::sync::Arc<PreparedAudio>, String> {
    let entry = load_from(root, account_id)?
        .into_iter()
        .find(|entry| entry.track.id == track_id)
        .ok_or_else(|| "The offline copy is missing. Save it again from the library.".to_owned())?;
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
    let metadata = file
        .metadata()
        .map_err(|_| "The offline copy could not be read.".to_owned())?;
    if !metadata.is_file() || metadata.len() != entry.length_bytes {
        return Err("The offline copy does not match its manifest. Save it again.".into());
    }
    if let Some(expected) = entry.digest.as_deref() {
        let mut hasher = blake3::Hasher::new();
        let mut buffer = [0_u8; 64 * 1024];
        loop {
            let read = std::io::Read::read(&mut file, &mut buffer)
                .map_err(|_| "The offline copy could not be read.".to_owned())?;
            if read == 0 {
                break;
            }
            hasher.update(&buffer[..read]);
        }
        if hasher.finalize().to_hex().as_str() != expected {
            return Err("The offline copy failed its integrity check. Save it again.".into());
        }
        use std::io::Seek;
        file.seek(std::io::SeekFrom::Start(0))
            .map_err(|_| "The offline copy could not be read.".to_owned())?;
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
    use std::io::Write;

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
        save_to(&root, &account, &track(id, bytes.len() as u64), &prepared).unwrap();
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
            assert_eq!(
                fs::metadata(root.join(format!("{id}.audio")))
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o777,
                0o600
            );
            assert_eq!(
                fs::metadata(root.join(format!("{id}.json")))
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o777,
                0o600
            );
        }
        assert!(
            load_from(&root, &uuid::Uuid::new_v4().to_string())
                .unwrap()
                .is_empty()
        );
        fs::write(root.join(format!("{id}.audio")), b"tampered").unwrap();
        assert!(load_from(&root, &account).unwrap().is_empty());
    }
}
