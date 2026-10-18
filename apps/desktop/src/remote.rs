//! Read-only public catalog access and authorized public audio preparation.
//!
//! This module never sends the Vela session cookie to a peer. It accepts only a
//! canonical origin, validates the peer's advertised identity, and plays only a
//! track whose explicit public stream grant is present. A debug-only transport
//! override lets the isolated two-instance QA reach a loopback peer while still
//! validating the canonical advertised origin; release builds ignore it.
use crate::{PreparedAudio, client::canonical_origin};
use futures_util::{StreamExt, future::join_all};
use reqwest::{Client, StatusCode, header};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{Seek, Write},
    path::PathBuf,
    sync::Arc,
    time::Duration,
};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
#[cfg(debug_assertions)]
use url::Host;
use url::Url;
use vela_domain::{PublicCatalog, PublicTrack};

const MAX_CATALOG_BYTES: usize = 128 * 1024;
const MAX_MEDIA_BYTES: u64 = 250 * 1024 * 1024;
const NETWORK_ERROR: &str = "Unable to reach the public server. Check its origin and availability.";
const INVALID_RESPONSE: &str = "The public server returned an unsupported catalog.";

#[derive(Clone, Debug)]
pub struct RemoteNow {
    pub id: String,
    pub title: String,
    pub artist: String,
    pub origin: String,
}

#[derive(Clone)]
pub struct PeerSearch {
    pub origin: String,
    pub server_id: Option<String>,
    pub items: Vec<PublicTrack>,
    pub error: Option<String>,
    pub cached: bool,
    pub cached_at: Option<u64>,
    pub next_cursor: Option<String>,
}

#[derive(Clone)]
pub struct RemoteItem {
    pub origin: String,
    pub track: PublicTrack,
}

#[derive(Clone)]
pub struct RemotePreview {
    pub origin: String,
    pub server_id: String,
    pub items: Vec<PublicTrack>,
    pub next_cursor: Option<String>,
}

struct Peer {
    canonical: Url,
    transport: Url,
}

/// Validate the requested origin and decide which transport URL to dial.
fn peer(input: &str) -> Result<Peer, String> {
    let canonical = canonical_origin(input)?;
    #[cfg(debug_assertions)]
    let mut transport = canonical.clone();
    #[cfg(not(debug_assertions))]
    let transport = canonical.clone();
    #[cfg(debug_assertions)]
    {
        let mut overrides = std::collections::HashMap::<String, String>::new();
        if let Ok(raw) = std::env::var("VELA_NATIVE_PEER_TRANSPORTS")
            && let Ok(map) = serde_json::from_str::<std::collections::HashMap<String, String>>(&raw)
        {
            overrides.extend(map);
        }
        if let (Ok(expected), Ok(value)) = (
            std::env::var("VELA_NATIVE_PEER_CANONICAL"),
            std::env::var("VELA_NATIVE_PEER_TRANSPORT"),
        ) {
            overrides.insert(expected, value);
        }
        if let Some(value) = overrides.get(canonical.as_str())
            && let Ok(url) = Url::parse(value)
            && canonical_origin(value).is_ok()
            && (url.scheme() == "http" || url.scheme() == "https")
            && matches!(url.host(), Some(Host::Ipv4(ip)) if ip.is_loopback())
            && url.port_or_known_default().is_some_and(|port| port != 0)
        {
            transport = url;
        }
    }
    Ok(Peer {
        canonical,
        transport,
    })
}

fn endpoint(origin: &Url, path: &str) -> Url {
    let mut url = origin.clone();
    url.set_path(path);
    url
}

fn client() -> Result<Client, String> {
    let builder = Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(Duration::from_secs(5))
        .read_timeout(Duration::from_secs(20))
        .timeout(Duration::from_secs(60));
    #[cfg(debug_assertions)]
    let builder = if let Ok(path) = std::env::var("VELA_NATIVE_PEER_CA") {
        match fs::read(path)
            .map_err(|_| ())
            .and_then(|bytes| reqwest::Certificate::from_pem(&bytes).map_err(|_| ()))
        {
            Ok(certificate) => builder.add_root_certificate(certificate),
            Err(()) => builder,
        }
    } else {
        builder
    };
    builder.build().map_err(|_| NETWORK_ERROR.to_owned())
}

pub async fn preview(input: &str, cancel: CancellationToken) -> Result<RemotePreview, String> {
    catalog(input, None, 20, None, cancel).await
}

pub async fn more(
    input: &str,
    cursor: &str,
    cancel: CancellationToken,
) -> Result<RemotePreview, String> {
    if cursor.parse::<uuid::Uuid>().is_err() {
        return Err("The public catalog cursor is invalid.".into());
    }
    catalog(input, None, 20, Some(cursor), cancel).await
}

async fn catalog(
    input: &str,
    query: Option<&str>,
    limit: u32,
    cursor: Option<&str>,
    cancel: CancellationToken,
) -> Result<RemotePreview, String> {
    if !(1..=20).contains(&limit)
        || cursor.is_some_and(|cursor| cursor.parse::<uuid::Uuid>().is_err())
        || query.is_some_and(|query| {
            query.trim().is_empty() || query.len() > 128 || query.chars().any(char::is_control)
        })
    {
        return Err("Use a public catalog query of 1–128 characters without controls.".into());
    }
    let peer = peer(input)?;
    let mut url = endpoint(&peer.transport, "/api/public/catalog");
    url.query_pairs_mut()
        .append_pair("limit", &limit.to_string());
    if let Some(query) = query {
        url.query_pairs_mut().append_pair("q", query.trim());
    }
    if let Some(cursor) = cursor {
        url.query_pairs_mut().append_pair("cursor", cursor);
    }
    let request = client()?
        .get(url)
        .header(header::ACCEPT, "application/json")
        .send();
    let response = tokio::select! {
        _ = cancel.cancelled() => return Err("Request canceled.".into()),
        result = request => result.map_err(|_| NETWORK_ERROR.to_owned())?,
    };
    if response.status() != StatusCode::OK {
        return Err(format!(
            "The public server did not allow this catalog ({}).",
            response.status().as_u16()
        ));
    }
    if response
        .content_length()
        .is_some_and(|size| size > MAX_CATALOG_BYTES as u64)
    {
        return Err(INVALID_RESPONSE.into());
    }
    let bytes = tokio::select! {
        _ = cancel.cancelled() => return Err("Request canceled.".into()),
        result = response.bytes() => result.map_err(|_| NETWORK_ERROR.to_owned())?,
    };
    if bytes.len() > MAX_CATALOG_BYTES {
        return Err(INVALID_RESPONSE.into());
    }
    let catalog: PublicCatalog =
        serde_json::from_slice(&bytes).map_err(|_| INVALID_RESPONSE.to_owned())?;
    let advertised = canonical_origin(&catalog.origin).map_err(|_| INVALID_RESPONSE.to_owned())?;
    if advertised.as_str() != peer.canonical.as_str()
        || catalog.server_id.parse::<uuid::Uuid>().is_err()
        || catalog.items.len() > 20
        || catalog.next_cursor.as_ref().is_some_and(|cursor| {
            cursor.parse::<uuid::Uuid>().is_err()
                || catalog.items.last().is_none_or(|track| track.id != *cursor)
        })
        || catalog.items.iter().any(|track| {
            track.id.parse::<uuid::Uuid>().is_err()
                || track.duration_ms <= 0
                || track.duration_ms > 7 * 24 * 60 * 60 * 1000
                || track.title.is_empty()
                || track.title.len() > 512
                || track.artist.len() > 512
                || track.album.len() > 512
                || track.attribution.len() > 2000
                || track
                    .title
                    .chars()
                    .chain(track.artist.chars())
                    .chain(track.album.chars())
                    .any(char::is_control)
        })
    {
        return Err(INVALID_RESPONSE.into());
    }
    if cursor.is_none() {
        save_cached(peer.canonical.as_str(), &catalog.items);
    }
    Ok(RemotePreview {
        origin: peer.canonical.as_str().to_owned(),
        server_id: catalog.server_id,
        items: catalog.items,
        next_cursor: catalog.next_cursor,
    })
}

/// Bounded remote-queue stepping. No wrap, no index past either end.
pub(crate) fn queue_target(current: usize, length: usize, delta: i32) -> Option<usize> {
    if length == 0 {
        return None;
    }
    let next = current as i32 + delta;
    if next < 0 || next as usize >= length {
        None
    } else {
        Some(next as usize)
    }
}

fn cached_or_error(origin: String, error: String) -> PeerSearch {
    match load_cached(&origin) {
        Some(entry) if !entry.items.is_empty() => PeerSearch {
            origin,
            server_id: None,
            items: entry.items,
            error: None,
            cached: true,
            cached_at: Some(entry.fetched_at),
            next_cursor: None,
        },
        _ => PeerSearch {
            origin,
            server_id: None,
            items: Vec::new(),
            error: Some(error),
            cached: false,
            cached_at: None,
            next_cursor: None,
        },
    }
}

fn cache_key(origin: &str) -> String {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in origin.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

fn cache_root() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| {
            std::env::var_os("HOME")
                .map(PathBuf::from)
                .map(|home| home.join(".cache"))
        })?;
    Some(base.join("vela").join("public-catalog"))
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CachedCatalog {
    version: u8,
    origin: String,
    fetched_at: u64,
    items: Vec<PublicTrack>,
}

struct CachedEntry {
    fetched_at: u64,
    items: Vec<PublicTrack>,
}

fn load_cached(origin: &str) -> Option<CachedEntry> {
    load_cached_from(&cache_root()?, origin)
}

fn load_cached_from(root: &std::path::Path, origin: &str) -> Option<CachedEntry> {
    let path = root.join(format!("{}.json", cache_key(origin)));
    let metadata = fs::metadata(&path).ok()?;
    if !metadata.is_file() || metadata.len() > MAX_CATALOG_BYTES as u64 {
        return None;
    }
    let bytes = fs::read(&path).ok()?;
    let cached: CachedCatalog = serde_json::from_slice(&bytes).ok()?;
    if cached.version != 1
        || cached.origin != origin
        || cached.fetched_at == 0
        || cached.items.is_empty()
        || cached.items.len() > 50
        || cached
            .items
            .iter()
            .any(|track| track.id.parse::<uuid::Uuid>().is_err() || track.duration_ms <= 0)
    {
        return None;
    }
    Some(CachedEntry {
        fetched_at: cached.fetched_at,
        items: cached.items,
    })
}

fn save_cached(origin: &str, items: &[PublicTrack]) {
    if let Some(root) = cache_root() {
        save_cached_to(&root, origin, items);
    }
}

fn save_cached_to(root: &std::path::Path, origin: &str, items: &[PublicTrack]) {
    if items.is_empty() || items.len() > 50 {
        return;
    }
    if fs::read_dir(root)
        .map(|entries| entries.filter_map(Result::ok).take(65).count() > 64)
        .unwrap_or(false)
    {
        return;
    }
    let Ok(payload) = serde_json::to_vec(&CachedCatalog {
        version: 1,
        origin: origin.to_owned(),
        fetched_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_secs())
            .unwrap_or(0),
        items: items.to_vec(),
    }) else {
        return;
    };
    if payload.len() > MAX_CATALOG_BYTES {
        return;
    }
    let _ = fs::create_dir_all(root);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(root, fs::Permissions::from_mode(0o700));
    }
    let temporary = root.join(format!(".{}.tmp", cache_key(origin)));
    let write = || -> std::io::Result<()> {
        #[cfg(unix)]
        let mut file = {
            use std::os::unix::fs::OpenOptionsExt;
            fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .mode(0o600)
                .open(&temporary)?
        };
        #[cfg(not(unix))]
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&temporary)?;
        file.write_all(&payload)?;
        file.sync_all()?;
        fs::rename(&temporary, root.join(format!("{}.json", cache_key(origin))))?;
        Ok(())
    };
    if write().is_err() {
        let _ = fs::remove_file(&temporary);
    }
}

pub async fn search_peers(
    origins: &[String],
    query: &str,
    cancel: CancellationToken,
) -> Result<Vec<PeerSearch>, String> {
    if query.trim().is_empty() || query.len() > 128 || query.chars().any(char::is_control) {
        return Err("Use a public search of 1–128 characters without controls.".into());
    }
    let mut peers: Vec<String> = Vec::new();
    for origin in origins {
        let canonical = canonical_origin(origin)?;
        let value = canonical.as_str().to_owned();
        if !peers.contains(&value) {
            peers.push(value);
        }
    }
    peers.truncate(8);
    if peers.is_empty() {
        return Err("Save at least one public server first.".into());
    }
    let query = query.trim().to_owned();
    let futures = peers.into_iter().map(|origin| {
        let cancel = cancel.clone();
        let query = query.clone();
        async move {
            let result = tokio::time::timeout(
                Duration::from_secs(5),
                catalog(&origin, Some(&query), 20, None, cancel),
            )
            .await;
            match result {
                Ok(Ok(preview)) => PeerSearch {
                    origin,
                    server_id: Some(preview.server_id),
                    items: preview.items,
                    error: None,
                    cached: false,
                    cached_at: None,
                    next_cursor: preview.next_cursor,
                },
                Ok(Err(error)) => cached_or_error(origin, error),
                Err(_) => cached_or_error(
                    origin,
                    "The public server did not answer within five seconds.".into(),
                ),
            }
        }
    });
    let mut results = join_all(futures).await;
    results.sort_by(|left, right| left.origin.cmp(&right.origin));
    Ok(results)
}

pub async fn prepare(
    origin: &str,
    track: &PublicTrack,
    cancel: CancellationToken,
) -> Result<Arc<PreparedAudio>, String> {
    if !track.can_stream || track.id.parse::<uuid::Uuid>().is_err() {
        return Err("This public track has no streaming grant.".into());
    }
    let peer = peer(origin)?;
    let url = endpoint(
        &peer.transport,
        &format!("/api/public/media/{}/stream", track.id),
    );
    let request = client()?.get(url).send();
    let response = tokio::select! {
        _ = cancel.cancelled() => return Err("Request canceled.".into()),
        result = request => result.map_err(|_| NETWORK_ERROR.to_owned())?,
    };
    if response.status() != StatusCode::OK {
        return Err("The public server no longer grants this stream.".into());
    }
    let expected = response
        .content_length()
        .filter(|size| *size > 0 && *size <= MAX_MEDIA_BYTES)
        .ok_or_else(|| "The public audio response has no bounded length.".to_owned())?;
    if response.headers().contains_key(header::CONTENT_RANGE) {
        return Err(INVALID_RESPONSE.into());
    }
    let hash = response
        .headers()
        .get(header::ETAG)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix('"'))
        .and_then(|value| value.strip_suffix('"'))
        .filter(|value| {
            value.len() == 64
                && value
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        })
        .ok_or_else(|| INVALID_RESPONSE.to_owned())?
        .to_owned();
    let (sender, receiver) = mpsc::channel(2);
    let writer =
        tokio::task::spawn_blocking(move || write_public(receiver, expected, hash, cancel));
    let mut received = 0_u64;
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|_| NETWORK_ERROR.to_owned())?;
        received = received
            .checked_add(chunk.len() as u64)
            .filter(|size| *size <= expected && *size <= MAX_MEDIA_BYTES)
            .ok_or_else(|| "The public audio response exceeded its declared size.".to_owned())?;
        for bytes in chunk.chunks(64 * 1024) {
            sender
                .send(bytes.to_vec())
                .await
                .map_err(|_| "The public audio preparation was canceled.".to_owned())?;
        }
    }
    if received != expected {
        return Err("The public audio transfer ended early.".into());
    }
    drop(sender);
    writer
        .await
        .map_err(|_| "The public audio preparation failed.".to_owned())?
}

fn write_public(
    mut receiver: mpsc::Receiver<Vec<u8>>,
    expected: u64,
    expected_hash: String,
    cancel: CancellationToken,
) -> Result<Arc<PreparedAudio>, String> {
    #[cfg(target_os = "linux")]
    let mut file =
        tempfile::tempfile().map_err(|_| "Unable to prepare public audio.".to_owned())?;
    #[cfg(not(target_os = "linux"))]
    let mut file =
        tempfile::NamedTempFile::new().map_err(|_| "Unable to prepare public audio.".to_owned())?;
    let mut size = 0_u64;
    let mut hash = blake3::Hasher::new();
    while let Some(bytes) = receiver.blocking_recv() {
        if cancel.is_cancelled() {
            return Err("Request canceled.".into());
        }
        size = size
            .checked_add(bytes.len() as u64)
            .filter(|size| *size <= expected)
            .ok_or_else(|| "The public audio response exceeded its declared size.".to_owned())?;
        file.write_all(&bytes)
            .map_err(|_| "Unable to prepare public audio.".to_owned())?;
        hash.update(&bytes);
    }
    if cancel.is_cancelled() {
        return Err("Request canceled.".into());
    }
    if size != expected || hash.finalize().to_hex().as_str() != expected_hash {
        return Err("The public audio failed its size or BLAKE3 fingerprint check.".into());
    }
    #[cfg(target_os = "linux")]
    file.rewind()
        .map_err(|_| "Unable to prepare public audio.".to_owned())?;
    #[cfg(not(target_os = "linux"))]
    file.as_file_mut()
        .rewind()
        .map_err(|_| "Unable to prepare public audio.".to_owned())?;
    #[cfg(target_os = "linux")]
    {
        Ok(Arc::new(PreparedAudio::from_file(file, size)))
    }
    #[cfg(not(target_os = "linux"))]
    {
        Ok(Arc::new(PreparedAudio::new(file, size)))
    }
}

#[cfg(test)]
mod queue_tests {
    use super::queue_target;

    #[test]
    fn remote_queue_stepping_is_bounded_without_wrap() {
        assert_eq!(queue_target(0, 3, 1), Some(1));
        assert_eq!(queue_target(1, 3, -1), Some(0));
        assert_eq!(queue_target(2, 3, 1), None);
        assert_eq!(queue_target(0, 3, -1), None);
        assert_eq!(queue_target(0, 0, 1), None);
        assert_eq!(queue_target(0, 1, 1), None);
        assert_eq!(queue_target(0, 2, 2), None);
        assert_eq!(queue_target(1, 2, -2), None);
    }
}

#[cfg(test)]
mod cache_tests {
    use super::*;

    fn track(id: uuid::Uuid) -> PublicTrack {
        PublicTrack {
            id: id.to_string(),
            revision: 1,
            title: "Cached Signal".into(),
            artist: "Vela Test".into(),
            album: "Cached Album".into(),
            genre: String::new(),
            year: None,
            duration_ms: 30_000,
            codec: "flac".into(),
            license: vela_domain::PublicLicense::Cc0,
            attribution: "Cached fixture".into(),
            can_stream: true,
            can_download: false,
        }
    }

    #[test]
    fn cached_public_catalog_is_private_exact_origin_and_validated() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().join("public-catalog");
        let items = vec![track(uuid::Uuid::new_v4())];
        save_cached_to(&root, "https://offline.example.org/", &items);
        let loaded = load_cached_from(&root, "https://offline.example.org/").unwrap();
        assert!(loaded.fetched_at > 0);
        assert_eq!(loaded.items.len(), 1);
        assert_eq!(loaded.items[0].id, items[0].id);
        assert!(load_cached_from(&root, "https://other.example.org/").is_none());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let path = root.join(format!(
                "{}.json",
                cache_key("https://offline.example.org/")
            ));
            assert_eq!(
                fs::metadata(path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
        let path = root.join(format!(
            "{}.json",
            cache_key("https://offline.example.org/")
        ));
        fs::write(&path, b"{\"version\":2,\"origin\":\"https://offline.example.org/\",\"fetched_at\":0,\"items\":[]}").unwrap();
        assert!(load_cached_from(&root, "https://offline.example.org/").is_none());
    }
}
