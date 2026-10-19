//! Private, versioned paused-queue snapshot for one account on one origin.
//!
//! This is not listening history and not an authorization: a restored queue is
//! paused, never auto-played, and every resulting stream is re-authorized by the
//! normal root source path. Only canonical IDs, queue order, current occurrence,
//! position/modes and the non-secret playback context are stored.
use serde::{Deserialize, Serialize};
use std::{fs, io::Write, path::PathBuf};
use vela_domain::Track;

const VERSION: u8 = 1;
const MAX_BYTES: u64 = 512 * 1024;
const MAX_TRACKS: usize = 1000;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QueueSnapshot {
    pub version: u8,
    pub origin: String,
    pub account_id: String,
    pub queue: Vec<Track>,
    pub current: usize,
    pub position_ms: u64,
    pub shuffle: bool,
    pub repeat: String,
    pub context: String,
}

impl QueueSnapshot {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        origin: &str,
        account_id: &str,
        queue: Vec<Track>,
        current: usize,
        position_ms: u64,
        shuffle: bool,
        repeat: &str,
        context: &str,
    ) -> Result<Self, String> {
        let snapshot = Self {
            version: VERSION,
            origin: origin.to_owned(),
            account_id: account_id.to_owned(),
            queue,
            current,
            position_ms,
            shuffle,
            repeat: repeat.to_owned(),
            context: context.to_owned(),
        };
        snapshot.validate()?;
        Ok(snapshot)
    }

    fn validate(&self) -> Result<(), String> {
        if self.version != VERSION
            || self.origin.is_empty()
            || self.origin.len() > 2048
            || self
                .origin
                .chars()
                .any(|c| c.is_control() || c.is_whitespace())
            || !crate::client::canonical_uuid(&self.account_id)
            || self.queue.is_empty()
            || self.queue.len() > MAX_TRACKS
            || self.current >= self.queue.len()
            || self.position_ms > 86_400_000
            || !matches!(self.repeat.as_str(), "off" | "all" | "one")
            || self.context.len() > 512
            || self.context.chars().any(char::is_control)
        {
            return Err("The saved queue is invalid.".into());
        }
        for track in &self.queue {
            if !crate::client::canonical_uuid(&track.id)
                || track.title.is_empty()
                || track.title.len() > 512
                || track.size_bytes < 0
            {
                return Err("The saved queue contains an invalid track.".into());
            }
        }
        Ok(())
    }
}

fn config_root() -> Result<PathBuf, String> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .ok_or_else(|| "No private configuration directory is available.".to_owned())?;
    Ok(base.join("vela"))
}

fn path() -> Result<PathBuf, String> {
    Ok(config_root()?.join("desktop.queue.json"))
}

pub fn load(origin: &str, account_id: &str) -> Result<Option<QueueSnapshot>, String> {
    let path = path()?;
    let Ok(metadata) = fs::metadata(&path) else {
        return Ok(None);
    };
    if !metadata.is_file() || metadata.len() > MAX_BYTES {
        return Ok(None);
    }
    let Ok(bytes) = fs::read(&path) else {
        return Ok(None);
    };
    let Ok(snapshot) = serde_json::from_slice::<QueueSnapshot>(&bytes) else {
        return Ok(None);
    };
    if snapshot.origin != origin
        || snapshot.account_id != account_id
        || snapshot.validate().is_err()
    {
        return Ok(None);
    }
    Ok(Some(snapshot))
}

pub fn save(snapshot: &QueueSnapshot) -> Result<(), String> {
    snapshot.validate()?;
    let path = path()?;
    let directory = path
        .parent()
        .ok_or_else(|| "The queue path is invalid.".to_owned())?;
    fs::create_dir_all(directory).map_err(|_| "The saved queue could not be stored.".to_owned())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(directory, fs::Permissions::from_mode(0o700));
    }
    let bytes = serde_json::to_vec(snapshot)
        .map_err(|_| "The saved queue could not be serialized.".to_owned())?;
    if bytes.len() as u64 > MAX_BYTES {
        return Err("The saved queue is too large.".into());
    }
    let staging = directory.join(format!(".desktop.queue-{}.tmp", uuid::Uuid::new_v4()));
    let write = (|| -> std::io::Result<()> {
        let mut options = fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(&staging)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        fs::rename(&staging, &path)?;
        Ok(())
    })();
    if write.is_err() {
        let _ = fs::remove_file(&staging);
        return Err("The saved queue could not be stored.".into());
    }
    Ok(())
}

pub fn clear() -> Result<(), String> {
    let _ = fs::remove_file(path()?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn track(id: &str) -> Track {
        serde_json::from_value(serde_json::json!({
            "id": id, "title": "Song", "artist": "Artist", "album": "", "album_artist": "",
            "genre": "", "year": null, "track_number": null, "disc_number": null,
            "duration_ms": 1000, "codec": "FLAC", "bitrate": null, "sample_rate": null,
            "bit_depth": null, "channels": null, "size_bytes": 10, "artwork_id": null,
            "added_at": 1, "favorite": false
        }))
        .unwrap()
    }

    #[test]
    fn snapshot_round_trips_and_rejects_foreign_namespaces() {
        let id = "20000000-0000-4000-8000-000000000001";
        let snapshot = QueueSnapshot::new(
            "https://a.example.org/",
            "10000000-0000-4000-8000-000000000001",
            vec![track(id)],
            0,
            7000,
            true,
            "all",
            "Your library",
        )
        .unwrap();
        let encoded = serde_json::to_vec(&snapshot).unwrap();
        let decoded: QueueSnapshot = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(decoded, snapshot);
        assert!(
            QueueSnapshot::new(
                "https://a.example.org/",
                "10000000-0000-4000-8000-000000000001",
                vec![],
                0,
                0,
                false,
                "off",
                ""
            )
            .is_err()
        );
        assert!(
            QueueSnapshot::new(
                "https://a.example.org/",
                "10000000-0000-4000-8000-000000000001",
                vec![track(id)],
                1,
                0,
                false,
                "off",
                ""
            )
            .is_err()
        );
        let mut bad = snapshot.clone();
        bad.repeat = "shuffle".into();
        assert!(bad.validate().is_err());
    }
}
