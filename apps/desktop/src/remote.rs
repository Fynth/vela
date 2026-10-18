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
use std::{
    io::{Seek, Write},
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
            && url.scheme() == "http"
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
    Client::builder()
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(Duration::from_secs(5))
        .read_timeout(Duration::from_secs(20))
        .timeout(Duration::from_secs(60))
        .build()
        .map_err(|_| NETWORK_ERROR.to_owned())
}

pub async fn preview(input: &str, cancel: CancellationToken) -> Result<RemotePreview, String> {
    catalog(input, None, 20, cancel).await
}

async fn catalog(
    input: &str,
    query: Option<&str>,
    limit: u32,
    cancel: CancellationToken,
) -> Result<RemotePreview, String> {
    if !(1..=20).contains(&limit)
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
    Ok(RemotePreview {
        origin: peer.canonical.as_str().to_owned(),
        server_id: catalog.server_id,
        items: catalog.items,
    })
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
                catalog(&origin, Some(&query), 20, cancel),
            )
            .await;
            match result {
                Ok(Ok(preview)) => PeerSearch {
                    origin,
                    server_id: Some(preview.server_id),
                    items: preview.items,
                    error: None,
                },
                Ok(Err(error)) => PeerSearch {
                    origin,
                    server_id: None,
                    items: Vec::new(),
                    error: Some(error),
                },
                Err(_) => PeerSearch {
                    origin,
                    server_id: None,
                    items: Vec::new(),
                    error: Some("The public server did not answer within five seconds.".into()),
                },
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
