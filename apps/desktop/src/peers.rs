//! Non-secret saved public peer origins.
//!
//! Only canonical origins are stored; no credentials, tokens, listening state or
//! server metadata. Writes are atomic and private, and a malformed/oversized file
//! is rejected rather than silently discarded.
use crate::client::canonical_origin;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

const MAX_PEERS: usize = 32;
const MAX_FILE_BYTES: u64 = 64 * 1024;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PeerFile {
    version: u8,
    peers: Vec<String>,
}

fn validated(peers: &[String]) -> Result<Vec<String>, String> {
    if peers.len() > MAX_PEERS {
        return Err("At most 32 public servers can be saved.".into());
    }
    let mut out: Vec<String> = Vec::new();
    for peer in peers {
        let canonical = canonical_origin(peer)?;
        let value = canonical.as_str().to_owned();
        if !out.contains(&value) {
            out.push(value);
        }
    }
    if out.len() > MAX_PEERS {
        return Err("At most 32 public servers can be saved.".into());
    }
    Ok(out)
}

pub fn store_path() -> Result<PathBuf, String> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| {
            std::env::var_os("HOME")
                .map(PathBuf::from)
                .map(|home| home.join(".config"))
        })
        .ok_or_else(|| "No configuration directory is available.".to_owned())?;
    Ok(base.join("vela").join("peers.json"))
}

pub fn load() -> Result<Vec<String>, String> {
    load_from(&store_path()?)
}

pub fn save(peers: &[String]) -> Result<(), String> {
    save_to(&store_path()?, peers)
}

fn load_from(path: &Path) -> Result<Vec<String>, String> {
    let metadata = match fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(_) => return Err("Saved public servers could not be read.".into()),
    };
    if !metadata.is_file() || metadata.len() > MAX_FILE_BYTES {
        return Err("Saved public servers are invalid.".into());
    }
    let bytes = fs::read(path).map_err(|_| "Saved public servers could not be read.".to_owned())?;
    let file: PeerFile = serde_json::from_slice(&bytes)
        .map_err(|_| "Saved public servers are invalid.".to_owned())?;
    if file.version != 1 {
        return Err("Saved public servers use an unsupported version.".into());
    }
    validated(&file.peers)
}

fn save_to(path: &Path, peers: &[String]) -> Result<(), String> {
    let peers = validated(peers)?;
    let parent = path
        .parent()
        .ok_or_else(|| "Saved public servers path is invalid.".to_owned())?;
    fs::create_dir_all(parent)
        .map_err(|_| "Saved public servers could not be written.".to_owned())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(parent, fs::Permissions::from_mode(0o700))
            .map_err(|_| "Saved public servers could not be written.".to_owned())?;
    }
    let temporary = parent.join(format!(".peers-{}.tmp", uuid::Uuid::new_v4()));
    let payload = serde_json::to_vec(&PeerFile { version: 1, peers })
        .map_err(|_| "Saved public servers could not be written.".to_owned())?;
    #[cfg(unix)]
    let file = {
        use std::os::unix::fs::OpenOptionsExt;
        fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&temporary)
    };
    #[cfg(not(unix))]
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary);
    let mut file = file.map_err(|_| "Saved public servers could not be written.".to_owned())?;
    let result = (|| {
        file.write_all(&payload)?;
        file.sync_all()?;
        fs::rename(&temporary, path)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
        }
        Ok::<_, std::io::Error>(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
        return Err("Saved public servers could not be written.".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_private_store_round_trips_dedupes_and_rejects_bad_input() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("vela/peers.json");
        assert!(load_from(&path).unwrap().is_empty());
        let peers = vec![
            "https://peer.example.org".to_owned(),
            "https://peer.example.org/".to_owned(),
            "http://127.0.0.1:8080".to_owned(),
        ];
        save_to(&path, &peers).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(&path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
        assert_eq!(
            load_from(&path).unwrap(),
            vec![
                "https://peer.example.org/".to_owned(),
                "http://127.0.0.1:8080/".to_owned()
            ]
        );
        assert!(save_to(&path, &["https://user:secret@peer.example.org".into()]).is_err());
        assert!(save_to(&path, &["http://peer.example.org".into()]).is_err());
        assert!(save_to(&path, &vec!["https://peer.example.org".into(); 33]).is_err());
        fs::write(&path, b"{\"version\":2,\"peers\":[]}").unwrap();
        assert!(load_from(&path).is_err());
        fs::write(&path, b"not json").unwrap();
        assert!(load_from(&path).is_err());
    }
}
