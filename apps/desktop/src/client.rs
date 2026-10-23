//! One explicitly selected Vela origin. Credentials remain in Rust memory;
//! owned originals become playable only after bounded size/hash verification.
//!
//! A complete-file cache of the authenticated account on this origin is reused when,
//! and only when, the origin itself re-authorizes the current session for the same
//! strong validator and the cached file still hashes to it from the descriptor the
//! decoder receives. Cached bytes are never a grant: a denial, a changed validator or
//! a local digest mismatch falls back to the existing authorized full GET.
use crate::{DownloadProgress, PreparedAudio, artwork_cache, media_cache};
use futures_util::StreamExt;
use reqwest::{Client, Method, Response, StatusCode, header};
use serde::de::DeserializeOwned;
#[cfg(test)]
use std::sync::atomic::{AtomicUsize, Ordering};
use std::{
    collections::HashMap,
    future::Future,
    io::{Seek, Write},
    net::ToSocketAddrs,
    path::PathBuf,
    sync::{Arc, Mutex, OnceLock, Weak},
    time::Duration,
};
use tokio::sync::{OwnedSemaphorePermit, Semaphore, mpsc, watch};
use tokio_util::sync::CancellationToken;
use url::{Host, Url};
use vela_domain::{
    AlbumPage, AlbumSummary, ArtistPage, ArtistSummary, AuthResponse, Credentials, PlaylistDetail,
    PlaylistFolder, PlaylistSummary, Track, TrackPage, UpdatePlaylist,
};

const CANCELED: &str = "Request canceled.";
const SIGNED_OUT: &str = "Sign in again to continue.";
const INVALID_RESPONSE: &str = "The server returned an invalid Vela response.";
const NETWORK_ERROR: &str = "Unable to reach the Vela server. Check the connection and retry.";
const AUDIO_ERROR: &str =
    "Unable to prepare private temporary audio. Check available disk space and retry.";
const PREPARE_BUSY: &str = "Busy: another track is preparing. Cancel it first.";
/// Bounded HEAD revalidation. A short permission/validator check may be re-sent for a
/// reusable cache hit; it never replaces the origin's own authorization.
const REVALIDATE_TIMEOUT: Duration = Duration::from_secs(20);
/// Cache publications that may run in the background at the same time. At the limit a
/// prepare keeps the previous synchronous publication instead of accumulating an
/// unbounded number of local copies.
const MAX_IN_FLIGHT_PUBLICATIONS: usize = 32;
/// How long a prepare waits for a running publication of the same key before it
/// continues with an ordinary verified transfer.
const PUBLICATION_WAIT: Duration = Duration::from_secs(60);
pub(crate) const MAX_MEDIA_BYTES: u64 = 250 * 1024 * 1024;
const MAX_JSON_BYTES: usize = 4 * 1024 * 1024;
const FILE_CHUNK: usize = 64 * 1024;
/// Cover art is user-visible presentation, never an audio source. One response is
/// capped independently of the 250 MiB original-audio path.
pub(crate) const MAX_ARTWORK_BYTES: u64 = 8 * 1024 * 1024;
/// A cover transfer is bounded end to end, including its body.
const ARTWORK_TIMEOUT: Duration = Duration::from_secs(20);
/// The closed raster allowlist. SVG/HTML is never accepted from an origin.
const ARTWORK_ACCEPT: &str = "image/jpeg,image/png,image/webp";

/// The raster formats the native renderer accepts as cover art.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArtworkMime {
    Jpeg,
    Png,
    Webp,
}
impl ArtworkMime {
    pub fn content_type(self) -> &'static str {
        match self {
            Self::Jpeg => "image/jpeg",
            Self::Png => "image/png",
            Self::Webp => "image/webp",
        }
    }
    pub(crate) fn extension(self) -> &'static str {
        match self {
            Self::Jpeg => "jpg",
            Self::Png => "png",
            Self::Webp => "webp",
        }
    }
    /// Strict allowlist parse of the media type, before any parameters.
    fn from_content_type(value: &str) -> Option<Self> {
        let media = value.split(';').next()?.trim();
        if media.eq_ignore_ascii_case("image/jpeg") {
            Some(Self::Jpeg)
        } else if media.eq_ignore_ascii_case("image/png") {
            Some(Self::Png)
        } else if media.eq_ignore_ascii_case("image/webp") {
            Some(Self::Webp)
        } else {
            None
        }
    }
    /// The raster magic prefix that must agree with the declared content type.
    fn matches(self, bytes: &[u8]) -> bool {
        match self {
            Self::Jpeg => bytes.starts_with(&[0xFF, 0xD8, 0xFF]),
            Self::Png => bytes.starts_with(b"\x89PNG\r\n\x1A\n"),
            Self::Webp => {
                bytes.len() >= 12 && bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WEBP"
            }
        }
    }
}

/// One bounded cover response: already-authorized raster bytes only.
///
/// Its `Debug` redacts the payload: these bytes are private user content, not
/// diagnostic output. The renderer only ever receives [`ArtworkBytes::data_url`],
/// which contains no cookie, token or origin and cannot become a general
/// authenticated URL.
pub struct ArtworkBytes {
    mime: ArtworkMime,
    bytes: Vec<u8>,
}
impl std::fmt::Debug for ArtworkBytes {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ArtworkBytes")
            .field("mime", &self.mime)
            .field("length", &self.bytes.len())
            .finish()
    }
}
impl ArtworkBytes {
    /// Validate a complete image body: non-empty, within the 8 MiB cap and carrying a
    /// raster magic prefix that agrees with the declared content type. Anything else
    /// is rejected before a byte can reach the WebView.
    pub(crate) fn new(mime: ArtworkMime, bytes: Vec<u8>) -> Option<Self> {
        if bytes.is_empty() || bytes.len() as u64 > MAX_ARTWORK_BYTES || !mime.matches(&bytes) {
            return None;
        }
        Some(Self { mime, bytes })
    }
    pub fn mime(&self) -> ArtworkMime {
        self.mime
    }
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn len(&self) -> usize {
        self.bytes.len()
    }
    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }
    /// The only renderer representation: a self-contained `data:` URL of the image
    /// bytes that were just authorized. It carries no credentials and never points at
    /// the origin.
    pub fn data_url(&self) -> String {
        use base64::Engine as _;
        format!(
            "data:{};base64,{}",
            self.mime.content_type(),
            base64::engine::general_purpose::STANDARD.encode(&self.bytes)
        )
    }
}

/// Why a cover was not produced. Every variant keeps the existing neutral fallback in
/// the renderer; only [`ArtworkError::Failed`] is worth a later user-initiated retry,
/// and [`ArtworkError::Denied`] never is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArtworkError {
    /// 401/403/404 or an already-closed session: the origin did not authorize this
    /// artwork for this account now. Cached bytes are not returned.
    Denied,
    /// The request was canceled or its app session/lifetime ended.
    Canceled,
    /// Transport, redirect, MIME, size, magic or cache failure.
    Failed,
}
impl ArtworkError {
    pub fn retryable(self) -> bool {
        matches!(self, Self::Failed)
    }
}

// Intentionally no Debug implementation: neither request headers nor the
// selected origin/query are diagnostic output for the native UI.
#[derive(Clone)]
pub struct VelaClient {
    inner: Arc<Inner>,
}
/// The gate one running cache publication holds for its whole duration. A second
/// prepare of the same key waits for the gate instead of starting a second full
/// transfer.
type PublicationGate = tokio::sync::Mutex<()>;
struct Inner {
    http: Client,
    origin: Url,
    /// The authenticated account this client's catalog, queue and cache belong to.
    user_id: String,
    session: Mutex<Option<Session>>,
    token: String,
    lifetime: CancellationToken,
    preparing: Arc<Semaphore>,
    /// Cache publications that are still running. A second prepare of the same key
    /// waits for the running publication instead of starting a second full
    /// transfer, so one body per track is transferred once.
    publications: Mutex<HashMap<media_cache::CacheKey, Weak<PublicationGate>>>,
    #[cfg(test)]
    temporary_directory: Option<PathBuf>,
    #[cfg(test)]
    cache_directory: Option<PathBuf>,
    #[cfg(test)]
    publication_pause: Mutex<Option<Arc<PublicationPause>>>,
}
#[derive(Clone)]
struct Session {
    cookie: header::HeaderValue,
    csrf: header::HeaderValue,
}

fn http_client() -> Result<Client, String> {
    Client::builder()
        .no_proxy()
        .no_gzip()
        .no_brotli()
        .no_deflate()
        .no_zstd()
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(Duration::from_secs(10))
        .read_timeout(Duration::from_secs(30))
        .timeout(Duration::from_secs(300))
        .dns_resolver(Arc::new(NativeResolver))
        .user_agent(concat!("VelaNative/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|_| NETWORK_ERROR.to_owned())
}

impl VelaClient {
    pub fn session_token(&self) -> String {
        self.inner.token.clone()
    }
    pub async fn resume(
        origin: &str,
        token: &str,
        cancel: CancellationToken,
    ) -> Result<(Self, AuthResponse), String> {
        let origin = canonical_origin(origin)?;
        if token.len() != 64 || !lower_hex64(token) {
            return Err("The saved session is invalid. Sign in again.".into());
        }
        let http = http_client()?;
        let lifetime = CancellationToken::new();
        let mut cookie = header::HeaderValue::from_str(&format!("vela_session={token}"))
            .map_err(|_| INVALID_RESPONSE.to_owned())?;
        cookie.set_sensitive(true);
        let (session, auth) = bounded(&cancel, &lifetime, Duration::from_secs(20), async {
            let response = http
                .get(endpoint(&origin, "/api/me"))
                .header(header::COOKIE, cookie.clone())
                .header(header::ACCEPT, "application/json")
                .send()
                .await
                .map_err(|_| NETWORK_ERROR.to_owned())?;
            if response.status() != StatusCode::OK {
                return Err(if response.status() == StatusCode::UNAUTHORIZED {
                    "The saved session expired. Sign in again.".into()
                } else {
                    status_error(response.status())
                });
            }
            let auth: AuthResponse = json(response, 16 * 1024).await?;
            if !canonical_uuid(&auth.user.id)
                || auth.user.username.is_empty()
                || auth.user.username.len() > 32
                || auth.user.username.chars().any(char::is_control)
                || !lower_hex64(&auth.csrf_token)
            {
                return Err(INVALID_RESPONSE.into());
            }
            let mut csrf = header::HeaderValue::from_str(&auth.csrf_token)
                .map_err(|_| INVALID_RESPONSE.to_owned())?;
            csrf.set_sensitive(true);
            Ok((Session { cookie, csrf }, auth))
        })
        .await?;
        Ok((
            Self {
                inner: Arc::new(Inner {
                    http,
                    origin,
                    // The cache namespace is the authenticated account, never a
                    // username or a session token.
                    user_id: auth.user.id.clone(),
                    session: Mutex::new(Some(session)),
                    token: token.to_owned(),
                    lifetime,
                    preparing: Arc::new(Semaphore::new(1)),
                    publications: Mutex::new(HashMap::new()),
                    #[cfg(test)]
                    temporary_directory: None,
                    #[cfg(test)]
                    cache_directory: None,
                    #[cfg(test)]
                    publication_pause: Mutex::new(None),
                }),
            },
            auth,
        ))
    }
    pub async fn connect(
        origin: &str,
        username: String,
        password: String,
        cancel: CancellationToken,
    ) -> Result<(Self, AuthResponse), String> {
        let origin = canonical_origin(origin)?;
        if username.is_empty() || username.len() > 32 || password.is_empty() || password.len() > 128
        {
            return Err("Enter a valid Vela username and password.".into());
        }
        let http = http_client()?;
        let lifetime = CancellationToken::new();
        let credentials = Credentials { username, password };
        let (session, auth, token) = bounded(&cancel, &lifetime, Duration::from_secs(20), async {
            let response = http
                .post(endpoint(&origin, "/api/login"))
                .header(header::ORIGIN, origin.origin().ascii_serialization())
                .header(header::ACCEPT, "application/json")
                .json(&credentials)
                .send()
                .await
                .map_err(|_| NETWORK_ERROR.to_owned())?;
            if response.status() != StatusCode::OK {
                return Err(if response.status() == StatusCode::UNAUTHORIZED {
                    "Sign-in failed. Check your username and password.".into()
                } else {
                    status_error(response.status())
                });
            }
            let cookie = session_cookie(response.headers())?;
            let token = cookie
                .to_str()
                .ok()
                .and_then(|value| value.strip_prefix("vela_session="))
                .map(str::to_owned)
                .ok_or_else(|| INVALID_RESPONSE.to_owned())?;
            let auth: AuthResponse = json(response, 16 * 1024).await?;
            if !canonical_uuid(&auth.user.id)
                || auth.user.username.is_empty()
                || auth.user.username.len() > 32
                || auth.user.username.chars().any(char::is_control)
                || !lower_hex64(&auth.csrf_token)
            {
                return Err(INVALID_RESPONSE.into());
            }
            let mut csrf = header::HeaderValue::from_str(&auth.csrf_token)
                .map_err(|_| INVALID_RESPONSE.to_owned())?;
            csrf.set_sensitive(true);
            Ok((Session { cookie, csrf }, auth, token))
        })
        .await?;
        // Password and username are not retained in the client. There is no
        // cookie jar, localStorage, configuration file or credential database.
        drop(credentials);
        Ok((
            Self {
                inner: Arc::new(Inner {
                    http,
                    origin,
                    // The cache namespace is the authenticated account, never a
                    // username or a session token.
                    user_id: auth.user.id.clone(),
                    session: Mutex::new(Some(session)),
                    token,
                    lifetime,
                    preparing: Arc::new(Semaphore::new(1)),
                    publications: Mutex::new(HashMap::new()),
                    #[cfg(test)]
                    temporary_directory: None,
                    #[cfg(test)]
                    cache_directory: None,
                    #[cfg(test)]
                    publication_pause: Mutex::new(None),
                }),
            },
            auth,
        ))
    }

    pub async fn tracks(
        &self,
        q: Option<&str>,
        cursor: Option<&str>,
        favorites: bool,
        cancel: CancellationToken,
    ) -> Result<TrackPage, String> {
        let q = q.map(str::trim).filter(|q| !q.is_empty());
        if q.is_some_and(|q| q.len() > 256 || q.chars().any(char::is_control))
            || cursor.is_some_and(|cursor| !canonical_uuid(cursor))
        {
            return Err("Use a valid library query and page cursor.".into());
        }
        let session = self.session()?;
        let mut url = endpoint(&self.inner.origin, "/api/tracks");
        {
            let mut query = url.query_pairs_mut();
            query.append_pair("limit", "50");
            if favorites {
                query.append_pair("favorites", "true");
            }
            if let Some(q) = q {
                query.append_pair("q", q);
            }
            if let Some(cursor) = cursor {
                query.append_pair("cursor", cursor);
            }
        }
        bounded(
            &cancel,
            &self.inner.lifetime,
            Duration::from_secs(20),
            async {
                let response = self
                    .request(Method::GET, url, &session)
                    .send()
                    .await
                    .map_err(|_| NETWORK_ERROR.to_owned())?;
                if response.status() != StatusCode::OK {
                    return Err(status_error(response.status()));
                }
                let page: TrackPage = json(response, MAX_JSON_BYTES).await?;
                validate_page(&page, cursor)?;
                Ok(page)
            },
        )
        .await
    }

    /// Resolve one owner-authorized original for the single native decoder.
    ///
    /// Order: (a) the current session re-authorizes the track with a bounded HEAD and
    /// returns the authoritative validators; (b) a cached complete original of this
    /// account on this origin is reused only when its strong validator matches *and*
    /// the cached file still hashes to it from the descriptor handed to the decoder;
    /// (c) otherwise the existing authorized full GET verifies size plus BLAKE3 and
    /// warms the cache. A denial (401/403/404) returns the same error and never touches
    /// the cache, and an origin that cannot answer HEAD still gets the full GET.
    pub async fn prepare(
        &self,
        track: &Track,
        cancel: CancellationToken,
        progress: watch::Sender<DownloadProgress>,
    ) -> Result<Arc<PreparedAudio>, String> {
        if !canonical_uuid(&track.id)
            || track.size_bytes <= 0
            || track.size_bytes as u64 > MAX_MEDIA_BYTES
        {
            return Err(
                "This track cannot be prepared: native originals must be at most 250 MiB.".into(),
            );
        }
        let session = self.session()?;
        let expected = track.size_bytes as u64;
        // The reusable cache is optional: without a private cache root the client keeps
        // the verified full-transfer behavior and sends no revalidation request.
        let scope = self.cache_scope(&track.id);
        bounded(
            &cancel,
            &self.inner.lifetime,
            Duration::from_secs(300),
            async {
                // A canceled prior writer retains admission through actual cleanup.
                // Give normal rapid selections a short, cancellable cleanup window.
                let permit = tokio::time::timeout(
                    Duration::from_secs(1),
                    self.inner.preparing.clone().acquire_owned(),
                )
                .await
                .map_err(|_| PREPARE_BUSY.to_owned())?
                .map_err(|_| PREPARE_BUSY.to_owned())?;
                progress.send_replace(DownloadProgress {
                    received_bytes: 0,
                    total_bytes: expected,
                });
                // (a) Re-authorize with the current session and request the
                // authoritative validators. Cached bytes are never consulted before
                // the origin itself answers for this account at this moment.
                let mut validator = None;
                if scope.is_some() {
                    match self.stream_validators(&track.id, &session).await? {
                        Some((etag, advertised))
                            if advertised.is_none_or(|size| size == expected) =>
                        {
                            validator = Some(etag);
                        }
                        // An origin that cannot answer HEAD, or a HEAD that disagrees
                        // about the size, is decided by the authorized GET below.
                        Some(_) | None => {}
                    }
                }
                if let (Some(scope), Some(etag)) = (&scope, &validator) {
                    // (b) A hit re-hashes its own bytes from the very descriptor the
                    // decoder receives, so a hit is a verified complete original.
                    if let Some(cached) = scope.open_verified(expected, etag, &cancel).await? {
                        progress.send_replace(DownloadProgress {
                            received_bytes: expected,
                            total_bytes: expected,
                        });
                        return Ok(cached);
                    }
                    // (b2) A publication of this exact key may still be running: its
                    // verified bytes are already on their way to the cache. Waiting
                    // for it is cheaper than a second full transfer, and the cache is
                    // then re-checked once. A publication that failed leaves the
                    // cache cold, and the ordinary transfer below decides.
                    if let Some(publication) = self.in_flight_publication(&scope.key)
                        && self.wait_for_publication(&publication, &cancel).await?
                        && let Some(cached) = scope.open_verified(expected, etag, &cancel).await?
                    {
                        progress.send_replace(DownloadProgress {
                            received_bytes: expected,
                            total_bytes: expected,
                        });
                        return Ok(cached);
                    }
                }
                let response = self
                    .request(
                        Method::GET,
                        endpoint(
                            &self.inner.origin,
                            &format!("/api/tracks/{}/stream", track.id),
                        ),
                        &session,
                    )
                    .header(header::ACCEPT_ENCODING, "identity")
                    .send()
                    .await
                    .map_err(|_| NETWORK_ERROR.to_owned())?;
                if response.status() != StatusCode::OK {
                    return Err(status_error(response.status()));
                }
                if response
                    .content_length()
                    .is_some_and(|size| size != expected)
                    || response.headers().contains_key(header::CONTENT_RANGE)
                    || response
                        .headers()
                        .get(header::CONTENT_ENCODING)
                        .is_some_and(|encoding| encoding != "identity")
                {
                    return Err("The audio response does not match the selected original.".into());
                }
                let hash = response
                    .headers()
                    .get(header::ETAG)
                    .and_then(|value| value.to_str().ok())
                    .and_then(|value| value.strip_prefix('"'))
                    .and_then(|value| value.strip_suffix('"'))
                    .filter(|value| lower_hex64(value))
                    .ok_or_else(|| {
                        "The audio response has no valid original fingerprint.".to_owned()
                    })?
                    .to_owned();
                let stop = CancellationToken::new();
                let _stop_on_drop = stop.clone().drop_guard();
                let (sender, receiver) = mpsc::channel(2);
                let directory: Option<PathBuf> = {
                    #[cfg(test)]
                    {
                        self.inner.temporary_directory.clone()
                    }
                    #[cfg(not(test))]
                    {
                        None
                    }
                };
                // The verified fingerprint also authorizes the cache warm-up below.
                let verified_hash = hash.clone();
                let writer = tokio::task::spawn_blocking(move || {
                    write_original(receiver, expected, hash, stop, permit, directory)
                });
                let mut received = 0_u64;
                let mut stream = response.bytes_stream();
                while let Some(chunk) = stream.next().await {
                    let chunk = chunk.map_err(|_| NETWORK_ERROR.to_owned())?;
                    received = received
                        .checked_add(chunk.len() as u64)
                        .filter(|size| *size <= expected && *size <= MAX_MEDIA_BYTES)
                        .ok_or_else(|| {
                            "The audio response exceeded the selected original size.".to_owned()
                        })?;
                    for bytes in chunk.chunks(FILE_CHUNK) {
                        sender
                            .send(bytes.to_vec())
                            .await
                            .map_err(|_| AUDIO_ERROR.to_owned())?;
                    }
                    progress.send_replace(DownloadProgress {
                        received_bytes: received,
                        total_bytes: expected,
                    });
                }
                if received != expected {
                    return Err("The audio transfer ended before the original was complete.".into());
                }
                drop(sender);
                let prepared = writer.await.map_err(|_| AUDIO_ERROR.to_owned())??;
                // Warm the reusable cache from the lease that was just verified. The
                // cache is an optimization, not a condition of playback, so the local
                // copy runs in the background: the first sound never waits for it.
                // A full cache, a full disk or a canceled copy must never fail or
                // invalidate playback that already succeeded.
                if let Some(scope) = &scope
                    && !self
                        .publish(scope, prepared.clone(), verified_hash.clone())
                        .await
                {
                    // The registry of running publications is full: keep the
                    // previous synchronous publication instead of silently
                    // skipping the warm-up.
                    scope.store(&prepared, &verified_hash, &cancel).await;
                }
                Ok(prepared)
            },
        )
        .await
    }

    /// Resolve one owner-authorized cover image for the native renderer.
    ///
    /// Order: (a) one bounded HEAD re-authorizes this artwork for the current session
    /// and reports its content type; (b) only that answer may unlock cached bytes of
    /// this exact origin/account/artwork; (c) otherwise a bounded GET verifies the
    /// content type, the 8 MiB cap and the raster magic prefix before returning bytes.
    /// A 401/403/404 is `Denied` and never reads or returns cached bytes, so a foreign
    /// or revoked artwork cannot leak through the disk cache. Nothing here touches the
    /// audio engine: a cover failure has no transport path.
    pub async fn artwork(
        &self,
        id: &str,
        cancel: CancellationToken,
    ) -> Result<Arc<ArtworkBytes>, ArtworkError> {
        if !canonical_uuid(id) {
            // Arbitrary paths, traversal attempts and non-canonical ids never reach a
            // request or a cache lookup.
            return Err(ArtworkError::Failed);
        }
        if self.inner.lifetime.is_cancelled() {
            return Err(ArtworkError::Canceled);
        }
        let session = self.session().map_err(|_| ArtworkError::Denied)?;
        // The reusable cache is optional: without a private cache root the client keeps
        // the authorized GET only, sends no revalidation request and never trusts cached
        // bytes.
        let scope = self.artwork_scope(id);
        if let Some(scope) = &scope {
            match self.artwork_revalidate(id, &session, &cancel).await {
                Ok(mime) => {
                    if let Some(cached) = scope.read(mime, &cancel).await? {
                        return Ok(Arc::new(cached));
                    }
                }
                Err(ArtworkError::Canceled) => return Err(ArtworkError::Canceled),
                Err(ArtworkError::Denied) => return Err(ArtworkError::Denied),
                // A HEAD the origin cannot answer is not a rejection: the authorized GET
                // below is the authoritative check and reports the honest result.
                Err(ArtworkError::Failed) => {}
            }
        }
        if cancel.is_cancelled() {
            return Err(ArtworkError::Canceled);
        }
        if self.inner.lifetime.is_cancelled() {
            return Err(ArtworkError::Denied);
        }
        let art = Arc::new(self.fetch_artwork(id, &session, &cancel).await?);
        if let Some(scope) = &scope {
            // Best effort: the caller already holds verified bytes, so a canceled or
            // failed private copy can never fail the cover.
            scope.store(&art, &cancel).await;
        }
        Ok(art)
    }

    /// Ask the origin, for the current session, whether this artwork may be shown.
    /// The answer's content type is the only claim a cached file may be reused for.
    async fn artwork_revalidate(
        &self,
        id: &str,
        session: &Session,
        cancel: &CancellationToken,
    ) -> Result<ArtworkMime, ArtworkError> {
        let request = self.artwork_request(
            Method::HEAD,
            endpoint(&self.inner.origin, &format!("/api/artwork/{id}")),
            session,
        );
        let sent = tokio::select! {
            biased;
            _ = cancel.cancelled() => return Err(ArtworkError::Canceled),
            _ = self.inner.lifetime.cancelled() => return Err(ArtworkError::Denied),
            result = tokio::time::timeout(REVALIDATE_TIMEOUT, request.send()) => result,
        };
        let response = match sent {
            Ok(Ok(response)) => response,
            // Transport failures and deadlines are not a rejection: the full GET below
            // is the authoritative check and reports the honest error.
            Ok(Err(_)) | Err(_) => return Err(ArtworkError::Failed),
        };
        let status = response.status();
        if matches!(
            status,
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN | StatusCode::NOT_FOUND
        ) {
            return Err(ArtworkError::Denied);
        }
        if status != StatusCode::OK {
            return Err(ArtworkError::Failed);
        }
        if response.headers().contains_key(header::CONTENT_RANGE)
            || response
                .headers()
                .get(header::CONTENT_ENCODING)
                .is_some_and(|encoding| encoding != "identity")
            || response
                .headers()
                .get(header::CONTENT_LENGTH)
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.parse::<u64>().ok())
                // A HEAD answer may legitimately declare no body; only an oversized
                // declaration is disqualifying here. The GET below caps the real body.
                .is_some_and(|length| length > MAX_ARTWORK_BYTES)
        {
            return Err(ArtworkError::Failed);
        }
        response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .and_then(ArtworkMime::from_content_type)
            .ok_or(ArtworkError::Failed)
    }

    /// One bounded authorized GET. The body is capped while streaming, so a lying or
    /// endless response cannot exhaust UI memory, and the magic prefix must match the
    /// allowlisted content type before the bytes become [`ArtworkBytes`].
    async fn fetch_artwork(
        &self,
        id: &str,
        session: &Session,
        cancel: &CancellationToken,
    ) -> Result<ArtworkBytes, ArtworkError> {
        let request = self.artwork_request(
            Method::GET,
            endpoint(&self.inner.origin, &format!("/api/artwork/{id}")),
            session,
        );
        let result = tokio::select! {
            biased;
            _ = cancel.cancelled() => return Err(ArtworkError::Canceled),
            _ = self.inner.lifetime.cancelled() => return Err(ArtworkError::Denied),
            result = tokio::time::timeout(ARTWORK_TIMEOUT, self.artwork_body(request)) => result,
        };
        match result {
            Ok(result) => result,
            Err(_) => Err(ArtworkError::Failed),
        }
    }

    async fn artwork_body(
        &self,
        request: reqwest::RequestBuilder,
    ) -> Result<ArtworkBytes, ArtworkError> {
        let response = request.send().await.map_err(|_| ArtworkError::Failed)?;
        let status = response.status();
        if matches!(
            status,
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN | StatusCode::NOT_FOUND
        ) {
            return Err(ArtworkError::Denied);
        }
        if status != StatusCode::OK {
            // Redirects are disabled for every native request; a 3xx answer is a
            // failure, not a second origin to fetch the image from.
            return Err(ArtworkError::Failed);
        }
        if response.headers().contains_key(header::CONTENT_RANGE)
            || response
                .headers()
                .get(header::CONTENT_ENCODING)
                .is_some_and(|encoding| encoding != "identity")
            || response
                .content_length()
                .is_some_and(|length| length == 0 || length > MAX_ARTWORK_BYTES)
        {
            return Err(ArtworkError::Failed);
        }
        let mime = response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .and_then(ArtworkMime::from_content_type)
            .ok_or(ArtworkError::Failed)?;
        let mut bytes = Vec::new();
        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|_| ArtworkError::Failed)?;
            if bytes.len().saturating_add(chunk.len()) as u64 > MAX_ARTWORK_BYTES {
                return Err(ArtworkError::Failed);
            }
            bytes.extend_from_slice(&chunk);
        }
        ArtworkBytes::new(mime, bytes).ok_or(ArtworkError::Failed)
    }

    /// A cover request carries only the current in-memory session cookie, a strict
    /// raster allowlist and identity encoding. `http` already disables redirects.
    fn artwork_request(
        &self,
        method: Method,
        url: Url,
        session: &Session,
    ) -> reqwest::RequestBuilder {
        self.inner
            .http
            .request(method, url)
            .header(header::COOKIE, session.cookie.clone())
            .header(header::ACCEPT, ARTWORK_ACCEPT)
            .header(header::ACCEPT_ENCODING, "identity")
    }

    /// The reusable cover namespace of this client, or `None` when no private cache
    /// root exists. `None` keeps the authorized GET as the only path.
    fn artwork_scope(&self, artwork_id: &str) -> Option<ArtworkScope> {
        let key =
            artwork_cache::Key::new(self.inner.origin.as_str(), &self.inner.user_id, artwork_id)?;
        Some(ArtworkScope {
            root: self.artwork_cache_root().ok()?,
            key,
        })
    }

    fn artwork_cache_root(&self) -> Result<PathBuf, String> {
        #[cfg(test)]
        if let Some(directory) = &self.inner.cache_directory {
            return Ok(directory.join("artwork"));
        }
        artwork_cache::default_root()
    }

    pub async fn albums(
        &self,
        cursor: Option<&str>,
        cancel: CancellationToken,
    ) -> Result<AlbumPage, String> {
        if cursor.is_some_and(|cursor| !canonical_uuid(cursor)) {
            return Err("Use a valid album page cursor.".into());
        }
        let session = self.session()?;
        let mut url = endpoint(&self.inner.origin, "/api/albums");
        url.query_pairs_mut().append_pair("limit", "50");
        if let Some(cursor) = cursor {
            url.query_pairs_mut().append_pair("cursor", cursor);
        }
        bounded(
            &cancel,
            &self.inner.lifetime,
            Duration::from_secs(20),
            async {
                let response = self
                    .request(Method::GET, url, &session)
                    .send()
                    .await
                    .map_err(|_| NETWORK_ERROR.to_owned())?;
                if response.status() != StatusCode::OK {
                    return Err(status_error(response.status()));
                }
                let page: AlbumPage = json(response, MAX_JSON_BYTES).await?;
                if page.items.len() > 50
                    || page.items.iter().any(|album| !canonical_uuid(&album.id))
                    || page
                        .next_cursor
                        .as_ref()
                        .is_some_and(|cursor| !canonical_uuid(cursor))
                {
                    return Err(INVALID_RESPONSE.into());
                }
                Ok(page)
            },
        )
        .await
    }

    pub async fn album(&self, id: &str, cancel: CancellationToken) -> Result<AlbumSummary, String> {
        if !canonical_uuid(id) {
            return Err("Select a valid album.".into());
        }
        let session = self.session()?;
        bounded(
            &cancel,
            &self.inner.lifetime,
            Duration::from_secs(20),
            async {
                let response = self
                    .request(
                        Method::GET,
                        endpoint(&self.inner.origin, &format!("/api/albums/{id}")),
                        &session,
                    )
                    .send()
                    .await
                    .map_err(|_| NETWORK_ERROR.to_owned())?;
                if response.status() != StatusCode::OK {
                    return Err(status_error(response.status()));
                }
                let album: AlbumSummary = json(response, MAX_JSON_BYTES).await?;
                if album.id != id {
                    return Err(INVALID_RESPONSE.into());
                }
                Ok(album)
            },
        )
        .await
    }

    pub async fn album_tracks(
        &self,
        id: &str,
        cursor: Option<&str>,
        cancel: CancellationToken,
    ) -> Result<TrackPage, String> {
        self.group_tracks("albums", id, cursor, cancel).await
    }

    pub async fn artists(
        &self,
        cursor: Option<&str>,
        cancel: CancellationToken,
    ) -> Result<ArtistPage, String> {
        if cursor.is_some_and(|cursor| !canonical_uuid(cursor)) {
            return Err("Use a valid artist page cursor.".into());
        }
        let session = self.session()?;
        let mut url = endpoint(&self.inner.origin, "/api/artists");
        url.query_pairs_mut().append_pair("limit", "50");
        if let Some(cursor) = cursor {
            url.query_pairs_mut().append_pair("cursor", cursor);
        }
        bounded(
            &cancel,
            &self.inner.lifetime,
            Duration::from_secs(20),
            async {
                let response = self
                    .request(Method::GET, url, &session)
                    .send()
                    .await
                    .map_err(|_| NETWORK_ERROR.to_owned())?;
                if response.status() != StatusCode::OK {
                    return Err(status_error(response.status()));
                }
                let page: ArtistPage = json(response, MAX_JSON_BYTES).await?;
                if page.items.len() > 50
                    || page.items.iter().any(|artist| !canonical_uuid(&artist.id))
                    || page
                        .next_cursor
                        .as_ref()
                        .is_some_and(|cursor| !canonical_uuid(cursor))
                {
                    return Err(INVALID_RESPONSE.into());
                }
                Ok(page)
            },
        )
        .await
    }

    pub async fn artist(
        &self,
        id: &str,
        cancel: CancellationToken,
    ) -> Result<ArtistSummary, String> {
        if !canonical_uuid(id) {
            return Err("Select a valid artist.".into());
        }
        let session = self.session()?;
        bounded(
            &cancel,
            &self.inner.lifetime,
            Duration::from_secs(20),
            async {
                let response = self
                    .request(
                        Method::GET,
                        endpoint(&self.inner.origin, &format!("/api/artists/{id}")),
                        &session,
                    )
                    .send()
                    .await
                    .map_err(|_| NETWORK_ERROR.to_owned())?;
                if response.status() != StatusCode::OK {
                    return Err(status_error(response.status()));
                }
                let artist: ArtistSummary = json(response, MAX_JSON_BYTES).await?;
                if artist.id != id {
                    return Err(INVALID_RESPONSE.into());
                }
                Ok(artist)
            },
        )
        .await
    }

    pub async fn artist_tracks(
        &self,
        id: &str,
        cursor: Option<&str>,
        cancel: CancellationToken,
    ) -> Result<TrackPage, String> {
        self.group_tracks("artists", id, cursor, cancel).await
    }

    async fn group_tracks(
        &self,
        kind: &str,
        id: &str,
        cursor: Option<&str>,
        cancel: CancellationToken,
    ) -> Result<TrackPage, String> {
        if !canonical_uuid(id) || cursor.is_some_and(|cursor| !canonical_uuid(cursor)) {
            return Err("Select a valid group and page cursor.".into());
        }
        let session = self.session()?;
        let mut url = endpoint(&self.inner.origin, &format!("/api/{kind}/{id}/tracks"));
        url.query_pairs_mut().append_pair("limit", "50");
        if let Some(cursor) = cursor {
            url.query_pairs_mut().append_pair("cursor", cursor);
        }
        bounded(
            &cancel,
            &self.inner.lifetime,
            Duration::from_secs(20),
            async {
                let response = self
                    .request(Method::GET, url, &session)
                    .send()
                    .await
                    .map_err(|_| NETWORK_ERROR.to_owned())?;
                if response.status() != StatusCode::OK {
                    return Err(status_error(response.status()));
                }
                let page: TrackPage = json(response, MAX_JSON_BYTES).await?;
                validate_page(&page, cursor)?;
                Ok(page)
            },
        )
        .await
    }

    pub async fn playlists(
        &self,
        cancel: CancellationToken,
    ) -> Result<Vec<PlaylistSummary>, String> {
        let session = self.session()?;
        bounded(
            &cancel,
            &self.inner.lifetime,
            Duration::from_secs(20),
            async {
                let response = self
                    .request(
                        Method::GET,
                        endpoint(&self.inner.origin, "/api/playlists"),
                        &session,
                    )
                    .send()
                    .await
                    .map_err(|_| NETWORK_ERROR.to_owned())?;
                if response.status() != StatusCode::OK {
                    return Err(status_error(response.status()));
                }
                let playlists: Vec<PlaylistSummary> = json(response, MAX_JSON_BYTES).await?;
                if playlists.len() > 200
                    || playlists
                        .iter()
                        .any(|playlist| !canonical_uuid(&playlist.id))
                {
                    return Err(INVALID_RESPONSE.into());
                }
                Ok(playlists)
            },
        )
        .await
    }

    pub async fn playlist_folders(
        &self,
        cancel: CancellationToken,
    ) -> Result<Vec<PlaylistFolder>, String> {
        let session = self.session()?;
        bounded(
            &cancel,
            &self.inner.lifetime,
            Duration::from_secs(20),
            async {
                let response = self
                    .request(
                        Method::GET,
                        endpoint(&self.inner.origin, "/api/playlist-folders"),
                        &session,
                    )
                    .send()
                    .await
                    .map_err(|_| NETWORK_ERROR.to_owned())?;
                if response.status() != StatusCode::OK {
                    return Err(status_error(response.status()));
                }
                let folders: Vec<PlaylistFolder> = json(response, MAX_JSON_BYTES).await?;
                if folders.len() > 100
                    || folders.iter().any(|folder| {
                        !canonical_uuid(&folder.id)
                            || folder.revision < 0
                            || folder.playlist_count < 0
                    })
                {
                    return Err(INVALID_RESPONSE.into());
                }
                Ok(folders)
            },
        )
        .await
    }

    pub async fn create_playlist_folder(
        &self,
        name: &str,
        cancel: CancellationToken,
    ) -> Result<PlaylistFolder, String> {
        let name = playlist_name(name)?;
        let session = self.session()?;
        bounded(
            &cancel,
            &self.inner.lifetime,
            Duration::from_secs(20),
            async {
                let response = self
                    .request(
                        Method::POST,
                        endpoint(&self.inner.origin, "/api/playlist-folders"),
                        &session,
                    )
                    .json(&serde_json::json!({"name": name}))
                    .send()
                    .await
                    .map_err(|_| NETWORK_ERROR.to_owned())?;
                if response.status() != StatusCode::OK {
                    return Err(status_error(response.status()));
                }
                let folder: PlaylistFolder = json(response, MAX_JSON_BYTES).await?;
                if !canonical_uuid(&folder.id) {
                    return Err(INVALID_RESPONSE.into());
                }
                Ok(folder)
            },
        )
        .await
    }

    pub async fn rename_playlist_folder(
        &self,
        id: &str,
        revision: i64,
        name: &str,
        cancel: CancellationToken,
    ) -> Result<PlaylistFolder, String> {
        if !canonical_uuid(id) || revision < 0 {
            return Err("Select a valid playlist folder revision.".into());
        }
        let name = playlist_name(name)?;
        let session = self.session()?;
        bounded(
            &cancel,
            &self.inner.lifetime,
            Duration::from_secs(20),
            async {
                let response = self
                    .request(
                        Method::PATCH,
                        endpoint(&self.inner.origin, &format!("/api/playlist-folders/{id}")),
                        &session,
                    )
                    .json(&serde_json::json!({"revision": revision, "name": name}))
                    .send()
                    .await
                    .map_err(|_| NETWORK_ERROR.to_owned())?;
                if response.status() != StatusCode::OK {
                    return Err(status_error(response.status()));
                }
                let folder: PlaylistFolder = json(response, MAX_JSON_BYTES).await?;
                if folder.id != id {
                    return Err(INVALID_RESPONSE.into());
                }
                Ok(folder)
            },
        )
        .await
    }

    pub async fn delete_playlist_folder(
        &self,
        id: &str,
        revision: i64,
        cancel: CancellationToken,
    ) -> Result<(), String> {
        if !canonical_uuid(id) || revision < 0 {
            return Err("Select a valid playlist folder revision.".into());
        }
        let session = self.session()?;
        let mut url = endpoint(&self.inner.origin, &format!("/api/playlist-folders/{id}"));
        url.query_pairs_mut()
            .append_pair("revision", &revision.to_string());
        bounded(
            &cancel,
            &self.inner.lifetime,
            Duration::from_secs(20),
            async {
                let response = self
                    .request(Method::DELETE, url, &session)
                    .send()
                    .await
                    .map_err(|_| NETWORK_ERROR.to_owned())?;
                if response.status() == StatusCode::NO_CONTENT {
                    Ok(())
                } else {
                    Err(status_error(response.status()))
                }
            },
        )
        .await
    }

    pub async fn move_playlist(
        &self,
        id: &str,
        revision: i64,
        folder_id: Option<&str>,
        cancel: CancellationToken,
    ) -> Result<PlaylistSummary, String> {
        if !canonical_uuid(id)
            || revision < 0
            || folder_id.is_some_and(|folder| !canonical_uuid(folder))
        {
            return Err("Select a valid playlist, folder and revision.".into());
        }
        let session = self.session()?;
        bounded(
            &cancel,
            &self.inner.lifetime,
            Duration::from_secs(20),
            async {
                let response = self
                    .request(
                        Method::PATCH,
                        endpoint(&self.inner.origin, &format!("/api/playlists/{id}/folder")),
                        &session,
                    )
                    .json(&serde_json::json!({"revision": revision, "folder_id": folder_id}))
                    .send()
                    .await
                    .map_err(|_| NETWORK_ERROR.to_owned())?;
                if response.status() != StatusCode::OK {
                    return Err(status_error(response.status()));
                }
                let playlist: PlaylistSummary = json(response, MAX_JSON_BYTES).await?;
                if playlist.id != id {
                    return Err(INVALID_RESPONSE.into());
                }
                Ok(playlist)
            },
        )
        .await
    }

    pub async fn playlist(
        &self,
        id: &str,
        cancel: CancellationToken,
    ) -> Result<PlaylistDetail, String> {
        if !canonical_uuid(id) {
            return Err("Select a valid playlist.".into());
        }
        let session = self.session()?;
        bounded(
            &cancel,
            &self.inner.lifetime,
            Duration::from_secs(20),
            async {
                let response = self
                    .request(
                        Method::GET,
                        endpoint(&self.inner.origin, &format!("/api/playlists/{id}")),
                        &session,
                    )
                    .send()
                    .await
                    .map_err(|_| NETWORK_ERROR.to_owned())?;
                if response.status() != StatusCode::OK {
                    return Err(status_error(response.status()));
                }
                let detail: PlaylistDetail = json(response, MAX_JSON_BYTES).await?;
                if detail.playlist.id != id
                    || detail.items.len() > 1000
                    || detail
                        .items
                        .iter()
                        .any(|item| !canonical_uuid(&item.id) || !canonical_uuid(&item.track.id))
                {
                    return Err(INVALID_RESPONSE.into());
                }
                Ok(detail)
            },
        )
        .await
    }

    pub async fn create_playlist(
        &self,
        name: &str,
        cancel: CancellationToken,
    ) -> Result<PlaylistSummary, String> {
        let name = playlist_name(name)?;
        let session = self.session()?;
        bounded(
            &cancel,
            &self.inner.lifetime,
            Duration::from_secs(20),
            async {
                let response = self
                    .request(
                        Method::POST,
                        endpoint(&self.inner.origin, "/api/playlists"),
                        &session,
                    )
                    .json(&serde_json::json!({"name": name}))
                    .send()
                    .await
                    .map_err(|_| NETWORK_ERROR.to_owned())?;
                if response.status() != StatusCode::OK {
                    return Err(status_error(response.status()));
                }
                let playlist: PlaylistSummary = json(response, MAX_JSON_BYTES).await?;
                if !canonical_uuid(&playlist.id) {
                    return Err(INVALID_RESPONSE.into());
                }
                Ok(playlist)
            },
        )
        .await
    }

    pub async fn rename_playlist(
        &self,
        id: &str,
        revision: i64,
        name: &str,
        cancel: CancellationToken,
    ) -> Result<PlaylistSummary, String> {
        if !canonical_uuid(id) || revision < 0 {
            return Err("Select a valid playlist revision.".into());
        }
        let name = playlist_name(name)?;
        let session = self.session()?;
        let update = UpdatePlaylist {
            revision,
            mutation_id: uuid::Uuid::new_v4().to_string(),
            name: name.to_owned(),
        };
        bounded(
            &cancel,
            &self.inner.lifetime,
            Duration::from_secs(20),
            async {
                let response = self
                    .request(
                        Method::PATCH,
                        endpoint(&self.inner.origin, &format!("/api/playlists/{id}")),
                        &session,
                    )
                    .json(&update)
                    .send()
                    .await
                    .map_err(|_| NETWORK_ERROR.to_owned())?;
                if response.status() != StatusCode::OK {
                    return Err(status_error(response.status()));
                }
                let playlist: PlaylistSummary = json(response, MAX_JSON_BYTES).await?;
                if playlist.id != id {
                    return Err(INVALID_RESPONSE.into());
                }
                Ok(playlist)
            },
        )
        .await
    }

    pub async fn delete_playlist(
        &self,
        id: &str,
        revision: i64,
        cancel: CancellationToken,
    ) -> Result<(), String> {
        if !canonical_uuid(id) || revision < 0 {
            return Err("Select a valid playlist revision.".into());
        }
        let session = self.session()?;
        let url = endpoint(&self.inner.origin, &format!("/api/playlists/{id}"));
        let url = {
            let mut url = url;
            url.query_pairs_mut()
                .append_pair("revision", &revision.to_string())
                .append_pair("mutation_id", &uuid::Uuid::new_v4().to_string());
            url
        };
        bounded(
            &cancel,
            &self.inner.lifetime,
            Duration::from_secs(20),
            async {
                let response = self
                    .request(Method::DELETE, url, &session)
                    .send()
                    .await
                    .map_err(|_| NETWORK_ERROR.to_owned())?;
                if response.status() == StatusCode::NO_CONTENT {
                    Ok(())
                } else {
                    Err(status_error(response.status()))
                }
            },
        )
        .await
    }

    pub async fn reorder_playlist(
        &self,
        id: &str,
        revision: i64,
        item_ids: &[String],
        cancel: CancellationToken,
    ) -> Result<PlaylistDetail, String> {
        if !canonical_uuid(id)
            || revision < 0
            || item_ids.is_empty()
            || item_ids.len() > 1000
            || item_ids.iter().any(|item| !canonical_uuid(item))
        {
            return Err("Select a valid playlist order.".into());
        }
        let session = self.session()?;
        let body = serde_json::json!({
            "revision": revision,
            "mutation_id": uuid::Uuid::new_v4().to_string(),
            "item_ids": item_ids,
        });
        bounded(
            &cancel,
            &self.inner.lifetime,
            Duration::from_secs(20),
            async {
                let response = self
                    .request(
                        Method::PUT,
                        endpoint(&self.inner.origin, &format!("/api/playlists/{id}/order")),
                        &session,
                    )
                    .json(&body)
                    .send()
                    .await
                    .map_err(|_| NETWORK_ERROR.to_owned())?;
                if response.status() != StatusCode::OK {
                    return Err(status_error(response.status()));
                }
                let detail: PlaylistDetail = json(response, MAX_JSON_BYTES).await?;
                if detail.playlist.id != id {
                    return Err(INVALID_RESPONSE.into());
                }
                Ok(detail)
            },
        )
        .await
    }

    pub async fn add_playlist_item(
        &self,
        id: &str,
        revision: i64,
        track_id: &str,
        cancel: CancellationToken,
    ) -> Result<PlaylistDetail, String> {
        if !canonical_uuid(id) || !canonical_uuid(track_id) || revision < 0 {
            return Err("Select a valid playlist and owned track.".into());
        }
        let session = self.session()?;
        let body = serde_json::json!({
            "revision": revision,
            "mutation_id": uuid::Uuid::new_v4().to_string(),
            "track_id": track_id,
            "item_ids": null,
        });
        bounded(
            &cancel,
            &self.inner.lifetime,
            Duration::from_secs(20),
            async {
                let response = self
                    .request(
                        Method::POST,
                        endpoint(&self.inner.origin, &format!("/api/playlists/{id}/items")),
                        &session,
                    )
                    .json(&body)
                    .send()
                    .await
                    .map_err(|_| NETWORK_ERROR.to_owned())?;
                if response.status() != StatusCode::OK {
                    return Err(status_error(response.status()));
                }
                let detail: PlaylistDetail = json(response, MAX_JSON_BYTES).await?;
                if detail.playlist.id != id {
                    return Err(INVALID_RESPONSE.into());
                }
                Ok(detail)
            },
        )
        .await
    }

    pub async fn remove_playlist_item(
        &self,
        id: &str,
        item_id: &str,
        revision: i64,
        cancel: CancellationToken,
    ) -> Result<PlaylistDetail, String> {
        if !canonical_uuid(id) || !canonical_uuid(item_id) || revision < 0 {
            return Err("Select a valid playlist entry.".into());
        }
        let session = self.session()?;
        let url = endpoint(
            &self.inner.origin,
            &format!("/api/playlists/{id}/items/{item_id}"),
        );
        let url = {
            let mut url = url;
            url.query_pairs_mut()
                .append_pair("revision", &revision.to_string())
                .append_pair("mutation_id", &uuid::Uuid::new_v4().to_string());
            url
        };
        bounded(
            &cancel,
            &self.inner.lifetime,
            Duration::from_secs(20),
            async {
                let response = self
                    .request(Method::DELETE, url, &session)
                    .send()
                    .await
                    .map_err(|_| NETWORK_ERROR.to_owned())?;
                if response.status() != StatusCode::OK {
                    return Err(status_error(response.status()));
                }
                let detail: PlaylistDetail = json(response, MAX_JSON_BYTES).await?;
                if detail.playlist.id != id {
                    return Err(INVALID_RESPONSE.into());
                }
                Ok(detail)
            },
        )
        .await
    }

    pub async fn favorite(
        &self,
        track_id: &str,
        desired: bool,
        cancel: CancellationToken,
    ) -> Result<(), String> {
        if !canonical_uuid(track_id) {
            return Err("Select a valid owned track.".into());
        }
        let session = self.session()?;
        bounded(
            &cancel,
            &self.inner.lifetime,
            Duration::from_secs(20),
            async {
                let response = self
                    .request(
                        Method::POST,
                        endpoint(
                            &self.inner.origin,
                            &format!("/api/tracks/{track_id}/favorite"),
                        ),
                        &session,
                    )
                    .json(&serde_json::json!({"favorite":desired}))
                    .send()
                    .await
                    .map_err(|_| NETWORK_ERROR.to_owned())?;
                if response.status() == StatusCode::NO_CONTENT {
                    Ok(())
                } else {
                    Err(status_error(response.status()))
                }
            },
        )
        .await
    }

    pub async fn logout(&self, cancel: CancellationToken) -> Result<(), String> {
        let session = self
            .inner
            .session
            .lock()
            .map_err(|_| SIGNED_OUT.to_owned())?
            .take();
        self.inner.lifetime.cancel();
        let Some(session) = session else {
            return Ok(());
        };
        // Local logout is immediate even if the remote request is canceled or
        // unreachable. Previously issued server sessions then expire normally.
        bounded(
            &cancel,
            &CancellationToken::new(),
            Duration::from_secs(20),
            async {
                let response = self
                    .request(
                        Method::POST,
                        endpoint(&self.inner.origin, "/api/logout"),
                        &session,
                    )
                    .send()
                    .await
                    .map_err(|_| NETWORK_ERROR.to_owned())?;
                if matches!(
                    response.status(),
                    StatusCode::NO_CONTENT | StatusCode::UNAUTHORIZED
                ) {
                    Ok(())
                } else {
                    Err(status_error(response.status()))
                }
            },
        )
        .await
    }

    fn session(&self) -> Result<Session, String> {
        self.inner
            .session
            .lock()
            .map_err(|_| SIGNED_OUT.to_owned())?
            .clone()
            .ok_or_else(|| SIGNED_OUT.to_owned())
    }
    fn request(&self, method: Method, url: Url, session: &Session) -> reqwest::RequestBuilder {
        let write = method != Method::GET && method != Method::HEAD;
        let request = self
            .inner
            .http
            .request(method, url)
            .header(header::COOKIE, session.cookie.clone())
            .header(header::ACCEPT, "application/json");
        if write {
            request.header("X-CSRF-Token", session.csrf.clone()).header(
                header::ORIGIN,
                self.inner.origin.origin().ascii_serialization(),
            )
        } else {
            request
        }
    }

    /// Bounded HEAD revalidation of the stream endpoint for the current session.
    ///
    /// `Ok(Some((etag, declared_length)))` means the origin itself authorized this
    /// representation now; `Ok(None)` means the origin cannot revalidate a reusable
    /// hit (HEAD unsupported, unusable headers, transport failure) and the caller must
    /// use the full authorized GET. A denial is returned as the same error the GET
    /// produces, so cached bytes are never consulted for it.
    async fn stream_validators(
        &self,
        track_id: &str,
        session: &Session,
    ) -> Result<Option<(String, Option<u64>)>, String> {
        let request = self
            .request(
                Method::HEAD,
                endpoint(
                    &self.inner.origin,
                    &format!("/api/tracks/{track_id}/stream"),
                ),
                session,
            )
            .header(header::ACCEPT_ENCODING, "identity");
        let response = match tokio::time::timeout(REVALIDATE_TIMEOUT, request.send()).await {
            Ok(Ok(response)) => response,
            // Transport failures and deadlines are not a rejection: the full GET below
            // is the authoritative check and reports the honest error.
            Ok(Err(_)) | Err(_) => return Ok(None),
        };
        let status = response.status();
        if matches!(
            status,
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN | StatusCode::NOT_FOUND
        ) {
            return Err(status_error(status));
        }
        if status != StatusCode::OK {
            // 405/501 and any other answer mean "no reusable hit"; the GET decides.
            return Ok(None);
        }
        if response.headers().contains_key(header::CONTENT_RANGE)
            || response
                .headers()
                .get(header::CONTENT_ENCODING)
                .is_some_and(|encoding| encoding != "identity")
        {
            return Ok(None);
        }
        // `Response::content_length` reports the received body size hint, which is empty
        // for HEAD, so the declared header is the only honest size of a HEAD answer.
        let declared = response
            .headers()
            .get(header::CONTENT_LENGTH)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<u64>().ok());
        let etag = response
            .headers()
            .get(header::ETAG)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.strip_prefix('"'))
            .and_then(|value| value.strip_suffix('"'))
            .filter(|value| lower_hex64(value))
            .map(str::to_owned);
        Ok(etag.map(|etag| (etag, declared)))
    }

    /// The reusable cache namespace of this client, or `None` when it is unavailable
    /// (no cache root, or a platform whose lease is not a descriptor). `None` keeps the
    /// verified full-transfer path and sends no revalidation request.
    fn cache_scope(&self, track_id: &str) -> Option<CacheScope> {
        #[cfg(not(target_os = "linux"))]
        {
            let _ = track_id;
            None
        }
        #[cfg(target_os = "linux")]
        {
            let key = media_cache::CacheKey::new(
                self.inner.origin.as_str(),
                &self.inner.user_id,
                track_id,
            )
            .ok()?;
            Some(CacheScope {
                root: self.cache_root().ok()?,
                key,
            })
        }
    }

    /// The private media cache directory. It is consulted on the platform that
    /// keeps the cache, and in tests that point it at a temporary directory.
    #[cfg(any(target_os = "linux", test))]
    fn cache_root(&self) -> Result<PathBuf, String> {
        #[cfg(test)]
        if let Some(directory) = &self.inner.cache_directory {
            return Ok(directory.clone());
        }
        media_cache::default_root()
    }

    /// The publication that is still running for this key, if any.
    ///
    /// The registry holds a weak reference to the gate, so an entry disappears as
    /// soon as its publication ended.
    fn in_flight_publication(&self, key: &media_cache::CacheKey) -> Option<Arc<PublicationGate>> {
        let registry = self.inner.publications.lock().ok()?;
        registry.get(key).and_then(Weak::upgrade)
    }

    /// Register a publication that is about to run.
    ///
    /// The gate is taken before the entry becomes visible, so a waiter can never
    /// observe an unlocked publication. The registry is bounded: at the limit the
    /// caller keeps the previous synchronous publication instead of accumulating
    /// unbounded background copies.
    async fn register_publication(
        &self,
        key: &media_cache::CacheKey,
    ) -> Option<tokio::sync::OwnedMutexGuard<()>> {
        let gate = Arc::new(tokio::sync::Mutex::new(()));
        let guard = gate.clone().lock_owned().await;
        let Ok(mut registry) = self.inner.publications.lock() else {
            return None;
        };
        // A publication that already ended is not in flight any more.
        registry.retain(|_, gate| gate.strong_count() > 0);
        if registry.len() >= MAX_IN_FLIGHT_PUBLICATIONS {
            return None;
        }
        registry.insert(key.clone(), Arc::downgrade(&gate));
        Some(guard)
    }

    /// Wait for a running publication of the same key to end.
    ///
    /// The wait is bounded and honors the caller's cancellation and the client's
    /// lifetime. `Ok(true)` means the publication ended and the cache may be
    /// re-checked; `Ok(false)` means the wait ran out and the caller continues with
    /// an ordinary verified transfer.
    async fn wait_for_publication(
        &self,
        publication: &Arc<PublicationGate>,
        cancel: &CancellationToken,
    ) -> Result<bool, String> {
        tokio::select! {
            biased;
            _ = cancel.cancelled() => Err(CANCELED.into()),
            _ = self.inner.lifetime.cancelled() => Err(SIGNED_OUT.into()),
            result = tokio::time::timeout(PUBLICATION_WAIT, publication.lock()) => {
                match result {
                    // The guard is released here: the publication is over and the
                    // next publication of this key may run.
                    Ok(_guard) => Ok(true),
                    Err(_elapsed) => Ok(false),
                }
            },
        }
    }

    /// Publish a verified original into the reusable cache in the background.
    ///
    /// The publication keeps the verified lease alive for its whole run, so the
    /// bytes it copies are exactly the bytes the decoder received. It is bound to
    /// the client's lifetime — a sign-out stops it — and it never holds the
    /// `preparing` admission, so it can neither delay nor block another transfer.
    ///
    /// The caller's per-selection cancel token is deliberately not observed: the
    /// UI cancels it as soon as another track is selected, and aborting the
    /// warm-up of the track that just played would break the A→B→A guarantee
    /// that one body per track is transferred once.
    ///
    /// Returns `false` when the registry of running publications is full; the
    /// caller then keeps the previous synchronous publication.
    async fn publish(
        &self,
        scope: &CacheScope,
        prepared: Arc<PreparedAudio>,
        etag: String,
    ) -> bool {
        let Some(guard) = self.register_publication(&scope.key).await else {
            return false;
        };
        let root = scope.root.clone();
        let key = scope.key.clone();
        let lifetime = self.inner.lifetime.clone();
        let lease = prepared.clone();
        #[cfg(test)]
        let pause = self
            .inner
            .publication_pause
            .lock()
            .ok()
            .and_then(|slot| slot.clone());
        tokio::spawn(async move {
            // The gate is held for the whole publication, so a second prepare of the
            // same key waits for it instead of starting a second transfer.
            let _guard = guard;
            #[cfg(test)]
            if let Some(pause) = &pause {
                pause.enter().await;
            }
            let _ = tokio::task::spawn_blocking(move || {
                media_cache::store(&root, &key, &prepared, &etag, &|| lifetime.is_cancelled())
            })
            .await;
            // The verified original is released as soon as its copy is done: a
            // finished publication never keeps a private original open.
            drop(lease);
            #[cfg(test)]
            if let Some(pause) = &pause {
                pause.finished();
            }
        });
        true
    }
}

/// A test-only gate that holds a running cache publication, so a test can observe
/// that `prepare` returned before the local copy of the verified original finished.
#[cfg(test)]
struct PublicationPause {
    entered: AtomicUsize,
    finished: AtomicUsize,
    released: watch::Sender<bool>,
}
#[cfg(test)]
impl PublicationPause {
    fn new() -> Arc<Self> {
        let (released, _) = watch::channel(false);
        Arc::new(Self {
            entered: AtomicUsize::new(0),
            finished: AtomicUsize::new(0),
            released,
        })
    }
    /// Hold the publication until the test releases it.
    async fn enter(&self) {
        self.entered.fetch_add(1, Ordering::SeqCst);
        let mut released = self.released.subscribe();
        while !*released.borrow_and_update() {
            if released.changed().await.is_err() {
                break;
            }
        }
    }
    /// Count a publication that ran to its end.
    fn finished(&self) {
        self.finished.fetch_add(1, Ordering::SeqCst);
    }
    /// Let the held publication continue.
    fn release(&self) {
        self.released.send_replace(true);
    }
}

/// One account's cover cache on one origin. Never shared across accounts or origins,
/// and only consulted after the origin re-authorized the current session.
struct ArtworkScope {
    root: PathBuf,
    key: artwork_cache::Key,
}

impl ArtworkScope {
    /// A revalidated hit, or an honest miss. A denial never reaches this method, and an
    /// unreadable, tampered or mismatched file is a miss rather than an error.
    async fn read(
        &self,
        mime: ArtworkMime,
        cancel: &CancellationToken,
    ) -> Result<Option<ArtworkBytes>, ArtworkError> {
        let root = self.root.clone();
        let key = self.key.clone();
        let probe = cancel.clone();
        // Blocking worker: reading a private file never runs on the UI thread.
        let read = tokio::task::spawn_blocking(move || {
            artwork_cache::read(&root, &key, mime, &|| probe.is_cancelled())
        })
        .await;
        match read {
            Ok(Ok(bytes)) => Ok(bytes),
            Ok(Err(_)) | Err(_) => {
                if cancel.is_cancelled() {
                    Err(ArtworkError::Canceled)
                } else {
                    Ok(None)
                }
            }
        }
    }

    /// Warm this namespace from bytes that were just verified by the authorized GET.
    async fn store(&self, art: &Arc<ArtworkBytes>, cancel: &CancellationToken) {
        let root = self.root.clone();
        let key = self.key.clone();
        let payload = art.clone();
        let probe = cancel.clone();
        let _ = tokio::task::spawn_blocking(move || {
            artwork_cache::store(&root, &key, &payload, &|| probe.is_cancelled())
        })
        .await;
    }
}

/// One account's reusable complete-file cache on one origin.
struct CacheScope {
    root: PathBuf,
    key: media_cache::CacheKey,
}

impl CacheScope {
    /// A verified hit, or `Ok(None)` for an honest miss. Any denial, mismatch or
    /// corruption is a miss; the caller then re-reads the origin.
    async fn open_verified(
        &self,
        expected: u64,
        etag: &str,
        cancel: &CancellationToken,
    ) -> Result<Option<Arc<PreparedAudio>>, String> {
        let root = self.root.clone();
        let key = self.key.clone();
        let etag = etag.to_owned();
        let probe = cancel.clone();
        // Blocking worker: hashing up to 250 MiB never runs on the UI thread.
        let lookup = tokio::task::spawn_blocking(move || {
            media_cache::open_verified(&root, &key, expected, &etag, &|| probe.is_cancelled())
        })
        .await
        .map_err(|_| AUDIO_ERROR.to_owned())??;
        let Some(cached) = lookup else {
            return Ok(None);
        };
        #[cfg(target_os = "linux")]
        {
            // The lease is the very descriptor that was hashed and rewound.
            Ok(Some(Arc::new(PreparedAudio::from_file(
                cached.file,
                cached.length_bytes,
            ))))
        }
        #[cfg(not(target_os = "linux"))]
        {
            // Outside Linux the lease is a named private temporary file, so a cached
            // entry cannot become a decoder lease and the verified GET is used.
            let _ = cached;
            Ok(None)
        }
    }

    /// Warm this namespace from an already verified lease. Never fails playback: the
    /// caller keeps its own lease and any error here is deliberately dropped.
    async fn store(&self, prepared: &Arc<PreparedAudio>, etag: &str, cancel: &CancellationToken) {
        let root = self.root.clone();
        let key = self.key.clone();
        let original = prepared.clone();
        let etag = etag.to_owned();
        let probe = cancel.clone();
        let _ = tokio::task::spawn_blocking(move || {
            media_cache::store(&root, &key, &original, &etag, &|| probe.is_cancelled())
        })
        .await;
    }
}

async fn bounded<T>(
    cancel: &CancellationToken,
    lifetime: &CancellationToken,
    duration: Duration,
    future: impl Future<Output = Result<T, String>>,
) -> Result<T, String> {
    tokio::select! {
        biased;
        _ = cancel.cancelled() => Err(CANCELED.into()),
        _ = lifetime.cancelled() => Err(SIGNED_OUT.into()),
        result = tokio::time::timeout(duration, future) => result.map_err(|_| "The Vela request timed out. Retry when the server is available.".to_owned())?,
    }
}

pub(crate) fn canonical_origin(input: &str) -> Result<Url, String> {
    let error = || "Use an HTTPS server root, or numeric loopback HTTP for local Vela.".to_owned();
    if input.is_empty()
        || input.len() > 2048
        || input.chars().any(|c| c.is_control() || c.is_whitespace())
        || input.contains('\\')
    {
        return Err(error());
    }
    let (scheme, rest) = input.split_once("://").ok_or_else(error)?;
    if !matches!(scheme.to_ascii_lowercase().as_str(), "https" | "http") {
        return Err(error());
    }
    let (authority, path) = rest.split_once('/').unwrap_or((rest, ""));
    if authority.contains('@') || !path.is_empty() {
        return Err(error());
    }
    let url = Url::parse(input).map_err(|_| error())?;
    if url.host().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || url.path() != "/"
        || url.port_or_known_default().is_none_or(|port| port == 0)
    {
        return Err(error());
    }
    if url.scheme() == "http"
        && !matches!(url.host(), Some(Host::Ipv4(ip)) if ip.is_loopback())
        && !matches!(url.host(), Some(Host::Ipv6(ip)) if ip.is_loopback())
    {
        return Err(error());
    }
    Ok(url)
}
fn playlist_name(name: &str) -> Result<&str, String> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 120 || name.chars().any(char::is_control) {
        return Err("Playlist name must be 1–120 characters.".into());
    }
    Ok(name)
}
fn endpoint(origin: &Url, path: &str) -> Url {
    let mut url = origin.clone();
    url.set_path(path);
    url
}
/// A lower-case canonical UUID. Cache namespaces never accept anything else.
pub(crate) fn canonical_uuid(value: &str) -> bool {
    uuid::Uuid::parse_str(value).is_ok_and(|id| id.to_string() == value)
}
/// A lower-case 64-hex BLAKE3 value (fingerprint, ETag, digest, session token).
pub(crate) fn lower_hex64(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
fn session_cookie(headers: &header::HeaderMap) -> Result<header::HeaderValue, String> {
    let mut token = None;
    for value in headers.get_all(header::SET_COOKIE) {
        let value = value.to_str().map_err(|_| INVALID_RESPONSE.to_owned())?;
        let first = value.split(';').next().unwrap_or("");
        let Some((name, value)) = first.split_once('=') else {
            continue;
        };
        if name.trim() == "vela_session" {
            if token.is_some() || !lower_hex64(value) {
                return Err(INVALID_RESPONSE.into());
            }
            token = Some(value);
        }
    }
    let mut cookie = header::HeaderValue::from_str(&format!(
        "vela_session={}",
        token.ok_or_else(|| INVALID_RESPONSE.to_owned())?
    ))
    .map_err(|_| INVALID_RESPONSE.to_owned())?;
    cookie.set_sensitive(true);
    Ok(cookie)
}
fn status_error(status: StatusCode) -> String {
    match status {
        StatusCode::UNAUTHORIZED => SIGNED_OUT,
        StatusCode::FORBIDDEN => {
            "The server denied this action. Sign in again or check your permissions."
        }
        StatusCode::NOT_FOUND => "The selected owned track is unavailable.",
        StatusCode::CONFLICT => "The server state changed. Refresh before trying again.",
        StatusCode::TOO_MANY_REQUESTS => "The server is busy. Wait before retrying.",
        _ if status.is_redirection() => {
            "The server redirected a Vela request. Use its direct HTTPS origin."
        }
        _ => "The Vela request failed. Check the server and retry.",
    }
    .into()
}
async fn json<T: DeserializeOwned + Send + 'static>(
    response: Response,
    maximum: usize,
) -> Result<T, String> {
    if response
        .content_length()
        .is_some_and(|length| length > maximum as u64)
    {
        return Err(INVALID_RESPONSE.into());
    }
    let mut bytes = Vec::new();
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|_| NETWORK_ERROR.to_owned())?;
        if bytes.len().saturating_add(chunk.len()) > maximum {
            return Err(INVALID_RESPONSE.into());
        }
        bytes.extend_from_slice(&chunk);
    }
    static JSON_SLOTS: OnceLock<Arc<Semaphore>> = OnceLock::new();
    let permit = JSON_SLOTS
        .get_or_init(|| Arc::new(Semaphore::new(2)))
        .clone()
        .acquire_owned()
        .await
        .map_err(|_| INVALID_RESPONSE.to_owned())?;
    tokio::task::spawn_blocking(move || {
        let _permit = permit;
        serde_json::from_slice(&bytes).map_err(|_| INVALID_RESPONSE.to_owned())
    })
    .await
    .map_err(|_| INVALID_RESPONSE.to_owned())?
}
fn validate_page(page: &TrackPage, cursor: Option<&str>) -> Result<(), String> {
    if page.items.len() > 50
        || page.items.windows(2).any(|rows| rows[0].id >= rows[1].id)
        || page.next_cursor.as_ref().is_some_and(|next| {
            !canonical_uuid(next) || page.items.last().is_none_or(|last| last.id != *next)
        })
        || page.items.iter().any(|track| {
            !canonical_uuid(&track.id)
                || cursor.is_some_and(|cursor| track.id.as_str() <= cursor)
                || track
                    .artwork_id
                    .as_deref()
                    .is_some_and(|id| !canonical_uuid(id))
                || track.size_bytes < 0
                || track.duration_ms < 0
        })
    {
        return Err(INVALID_RESPONSE.into());
    }
    Ok(())
}

fn write_original(
    mut receiver: mpsc::Receiver<Vec<u8>>,
    expected: u64,
    expected_hash: String,
    cancel: CancellationToken,
    _permit: OwnedSemaphorePermit,
    directory: Option<PathBuf>,
) -> Result<Arc<PreparedAudio>, String> {
    #[cfg(target_os = "linux")]
    let mut file = match directory {
        Some(directory) => tempfile::tempfile_in(directory),
        None => tempfile::tempfile(),
    }
    .map_err(|_| AUDIO_ERROR.to_owned())?;
    #[cfg(not(target_os = "linux"))]
    let mut file = {
        let mut builder = tempfile::Builder::new();
        builder.prefix("vela-native-");
        match directory {
            Some(directory) => builder.tempfile_in(directory),
            None => builder.tempfile(),
        }
        .map_err(|_| AUDIO_ERROR.to_owned())?
    };
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::fs::PermissionsExt;
        file.set_permissions(std::fs::Permissions::from_mode(0o600))
            .map_err(|_| AUDIO_ERROR.to_owned())?;
    }
    #[cfg(all(unix, not(target_os = "linux")))]
    {
        use std::os::unix::fs::PermissionsExt;
        file.as_file()
            .set_permissions(std::fs::Permissions::from_mode(0o600))
            .map_err(|_| AUDIO_ERROR.to_owned())?;
    }
    let mut size = 0_u64;
    let mut hash = blake3::Hasher::new();
    while let Some(bytes) = receiver.blocking_recv() {
        if cancel.is_cancelled() {
            return Err(CANCELED.into());
        }
        size = size
            .checked_add(bytes.len() as u64)
            .filter(|size| *size <= expected)
            .ok_or_else(|| AUDIO_ERROR.to_owned())?;
        file.write_all(&bytes).map_err(|_| AUDIO_ERROR.to_owned())?;
        hash.update(&bytes);
    }
    if cancel.is_cancelled() {
        return Err(CANCELED.into());
    }
    if size != expected || hash.finalize().to_hex().as_str() != expected_hash {
        return Err("The downloaded original failed its size or BLAKE3 fingerprint check.".into());
    }
    #[cfg(target_os = "linux")]
    file.rewind().map_err(|_| AUDIO_ERROR.to_owned())?;
    #[cfg(not(target_os = "linux"))]
    file.as_file_mut()
        .rewind()
        .map_err(|_| AUDIO_ERROR.to_owned())?;
    #[cfg(target_os = "linux")]
    {
        Ok(Arc::new(PreparedAudio::from_file(file, size)))
    }
    #[cfg(not(target_os = "linux"))]
    {
        Ok(Arc::new(PreparedAudio::new(file, size)))
    }
}

/// Native user-selected HTTPS origins may resolve privately. Admission is still
/// bounded through actual system-resolver completion, including after cancel.
struct NativeResolver;
impl reqwest::dns::Resolve for NativeResolver {
    fn resolve(&self, name: reqwest::dns::Name) -> reqwest::dns::Resolving {
        let host = name.as_str().to_owned();
        static DNS_SLOTS: OnceLock<Arc<Semaphore>> = OnceLock::new();
        let slots = DNS_SLOTS
            .get_or_init(|| Arc::new(Semaphore::new(4)))
            .clone();
        Box::pin(async move {
            let result = tokio::time::timeout(Duration::from_secs(10), async {
                let permit = slots
                    .acquire_owned()
                    .await
                    .map_err(|_| std::io::Error::other("DNS unavailable"))?;
                tokio::task::spawn_blocking(move || {
                    let _permit = permit;
                    let addresses: Vec<_> =
                        (host.as_str(), 0).to_socket_addrs()?.take(33).collect();
                    if addresses.is_empty() || addresses.len() > 32 {
                        return Err(std::io::Error::other("DNS answer rejected"));
                    }
                    Ok(addresses)
                })
                .await
                .map_err(|_| std::io::Error::other("DNS unavailable"))?
            })
            .await
            .map_err(|_| std::io::Error::other("DNS deadline"))?;
            result
                .map(|addresses| Box::new(addresses.into_iter()) as reqwest::dns::Addrs)
                .map_err(|error| Box::new(error) as Box<dyn std::error::Error + Send + Sync>)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        Router,
        body::Body,
        extract::Path,
        http::{HeaderMap, Response as HttpResponse},
        routing::{get, post},
    };
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use tokio::{net::TcpListener, task::JoinHandle};
    use vela_core::config::Config;
    use vela_server::AppState;

    const AUDIO: &[u8] = include_bytes!("../../../crates/media/tests/fixtures/signal.flac");
    const PASSWORD: &str = "isolated native test password";
    const USER_ID: &str = "10000000-0000-4000-8000-000000000001";
    const TRACK_ID: &str = "20000000-0000-4000-8000-000000000001";

    struct HttpFixture {
        origin: String,
        task: JoinHandle<()>,
    }
    impl HttpFixture {
        async fn start(router: Router) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
            let origin = format!("http://{}", listener.local_addr().unwrap());
            let task = tokio::spawn(async move {
                axum::serve(
                    listener,
                    router.into_make_service_with_connect_info::<std::net::SocketAddr>(),
                )
                .await
                .unwrap();
            });
            Self { origin, task }
        }
    }
    impl Drop for HttpFixture {
        fn drop(&mut self) {
            self.task.abort();
        }
    }
    struct Node {
        http: HttpFixture,
        state: AppState,
        own: Track,
        foreign: Track,
        _directory: tempfile::TempDir,
    }
    impl Node {
        async fn new() -> Self {
            let directory = tempfile::tempdir().unwrap();
            let state = AppState::initialize(Config {
                bind: "127.0.0.1:0".parse().unwrap(),
                data_dir: directory.path().join("original-fixture"),
                web_dir: directory.path().join("web"),
                cookie_secure: false,
                max_file_bytes: 1024 * 1024,
                max_storage_bytes: 10 * 1024 * 1024,
                scan_roots: Vec::new(),
            })
            .await
            .unwrap();
            let admin = state
                .auth
                .setup(Credentials {
                    username: "native_admin".into(),
                    password: PASSWORD.into(),
                })
                .await
                .unwrap();
            let other = state
                .auth
                .create_user(
                    &admin.response.user,
                    Credentials {
                        username: "native_other".into(),
                        password: PASSWORD.into(),
                    },
                )
                .await
                .unwrap();
            let mut tracks = Vec::new();
            for owner in [&admin.response.user.id, &other.id] {
                let staged = state.config.data_dir.join("staging/native-fixture.flac");
                tokio::fs::write(&staged, AUDIO).await.unwrap();
                tracks.push(
                    state
                        .media
                        .ingest(owner, &staged, "native-signal.flac")
                        .await
                        .unwrap()
                        .track,
                );
            }
            let own = tracks.remove(0);
            let foreign = tracks.remove(0);
            let http = HttpFixture::start(vela_server::router(state.clone())).await;
            Self {
                http,
                state,
                own,
                foreign,
                _directory: directory,
            }
        }
        async fn connect(&self) -> (VelaClient, AuthResponse) {
            VelaClient::connect(
                &self.http.origin,
                "native_admin".into(),
                PASSWORD.into(),
                CancellationToken::new(),
            )
            .await
            .unwrap_or_else(|error| panic!("client login failed: {error}"))
        }
    }
    fn sample_track() -> Track {
        Track {
            id: TRACK_ID.into(),
            title: "Explicit synthetic transport fixture".into(),
            artist: String::new(),
            album: String::new(),
            album_artist: String::new(),
            genre: String::new(),
            year: None,
            track_number: None,
            disc_number: None,
            duration_ms: 1000,
            codec: "PCM".into(),
            bitrate: None,
            sample_rate: None,
            bit_depth: None,
            channels: None,
            size_bytes: 16 * 1024,
            artwork_id: None,
            added_at: 1,
            favorite: false,
        }
    }
    fn fake_auth() -> AuthResponse {
        AuthResponse {
            user: vela_domain::User {
                id: USER_ID.into(),
                username: "fixture".into(),
                is_admin: false,
            },
            csrf_token: "c".repeat(64),
        }
    }
    fn with_login(router: Router) -> Router {
        router
            .route(
                "/api/login",
                post(|| async {
                    HttpResponse::builder()
                        .header(
                            header::SET_COOKIE,
                            format!(
                                "vela_session={}; Path=/; HttpOnly; SameSite=Strict",
                                "a".repeat(64)
                            ),
                        )
                        .header(
                            header::SET_COOKIE,
                            "unrelated_cookie=never-retained; Path=/",
                        )
                        .header(header::CONTENT_TYPE, "application/json")
                        .body(Body::from(serde_json::to_vec(&fake_auth()).unwrap()))
                        .unwrap()
                }),
            )
            .route(
                "/api/logout",
                post(|headers: HeaderMap| async move {
                    assert_eq!(
                        headers[header::COOKIE],
                        format!("vela_session={}", "a".repeat(64))
                    );
                    assert_eq!(headers["X-CSRF-Token"], "c".repeat(64));
                    StatusCode::NO_CONTENT
                }),
            )
    }
    /// A fixture client whose private temporary originals and reusable media cache are
    /// both isolated under caller-owned directories: no test reads or writes a live
    /// `$XDG_CACHE_HOME`/`$XDG_DATA_HOME` and every test can assert its own leftovers.
    async fn fixture_client(
        http: &HttpFixture,
        directory: &tempfile::TempDir,
        cache: &tempfile::TempDir,
    ) -> VelaClient {
        let (mut client, _) = VelaClient::connect(
            &http.origin,
            "fixture".into(),
            PASSWORD.into(),
            CancellationToken::new(),
        )
        .await
        .unwrap_or_else(|error| panic!("fixture login failed: {error}"));
        {
            let inner = Arc::get_mut(&mut client.inner).unwrap();
            inner.temporary_directory = Some(directory.path().to_owned());
            inner.cache_directory = Some(cache.path().to_owned());
        }
        client
    }

    /// The media cache identity the client derives for one fixture origin/account/track.
    fn cache_key(origin: &str, account: &str, track_id: &str) -> media_cache::CacheKey {
        media_cache::CacheKey::new(
            canonical_origin(origin).unwrap().as_str(),
            account,
            track_id,
        )
        .unwrap()
    }

    fn sample_track_with(id: &str, length: usize) -> Track {
        Track {
            id: id.into(),
            size_bytes: length as i64,
            ..sample_track()
        }
    }
    async fn eventually(mut condition: impl FnMut() -> bool) {
        tokio::time::timeout(Duration::from_secs(3), async {
            while !condition() {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("isolated cleanup condition timed out");
    }
    fn progress() -> (
        watch::Sender<DownloadProgress>,
        watch::Receiver<DownloadProgress>,
    ) {
        watch::channel(DownloadProgress::default())
    }
    /// Wait until the background publication of this key ended and released its
    /// lease on the verified original. The cache is warmed in the background, so
    /// a test that reads the published bytes waits for the publication instead of
    /// assuming it already happened.
    async fn await_publication(client: &VelaClient, key: &media_cache::CacheKey) {
        tokio::time::timeout(Duration::from_secs(5), async {
            while client.in_flight_publication(key).is_some() {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("the background cache publication did not finish");
    }

    #[tokio::test]
    async fn real_vela_login_owned_catalog_original_favorite_and_logout_are_authorized() {
        let node = Node::new().await;
        let (mut client, auth) = node.connect().await;
        let temporary = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();
        {
            let inner = Arc::get_mut(&mut client.inner).unwrap();
            inner.temporary_directory = Some(temporary.path().to_owned());
            // A separate private root keeps a live `$XDG_CACHE_HOME` untouched.
            inner.cache_directory = Some(cache.path().to_owned());
        }
        assert!(canonical_uuid(&auth.user.id));
        assert!(lower_hex64(&auth.csrf_token));
        let catalog = client
            .tracks(None, None, false, CancellationToken::new())
            .await
            .unwrap();
        assert_eq!(catalog.items.len(), 1);
        assert_eq!(catalog.items[0].id, node.own.id);
        assert_ne!(catalog.items[0].id, node.foreign.id);
        assert_eq!(
            client
                .tracks(Some(&node.own.title), None, false, CancellationToken::new())
                .await
                .unwrap()
                .items
                .len(),
            1
        );
        let albums = client.albums(None, CancellationToken::new()).await.unwrap();
        assert!(!albums.items.is_empty());
        assert_eq!(
            client
                .album(&albums.items[0].id, CancellationToken::new())
                .await
                .unwrap()
                .id,
            albums.items[0].id
        );
        assert!(
            !client
                .album_tracks(&albums.items[0].id, None, CancellationToken::new())
                .await
                .unwrap()
                .items
                .is_empty()
        );
        let artists = client
            .artists(None, CancellationToken::new())
            .await
            .unwrap();
        assert!(!artists.items.is_empty());
        assert_eq!(
            client
                .artist(&artists.items[0].id, CancellationToken::new())
                .await
                .unwrap()
                .id,
            artists.items[0].id
        );
        assert!(
            !client
                .artist_tracks(&artists.items[0].id, None, CancellationToken::new())
                .await
                .unwrap()
                .items
                .is_empty()
        );
        assert!(
            client
                .tracks(None, Some("../private"), false, CancellationToken::new())
                .await
                .is_err()
        );
        assert!(
            client
                .prepare(&node.foreign, CancellationToken::new(), progress().0)
                .await
                .is_err()
        );
        assert!(
            client
                .favorite(&node.foreign.id, true, CancellationToken::new())
                .await
                .is_err()
        );
        client
            .favorite(&node.own.id, true, CancellationToken::new())
            .await
            .unwrap();
        assert!(
            client
                .tracks(None, None, false, CancellationToken::new())
                .await
                .unwrap()
                .items[0]
                .favorite
        );
        let favorites = client
            .tracks(None, None, true, CancellationToken::new())
            .await
            .unwrap();
        assert_eq!(favorites.items.len(), 1);
        assert_eq!(favorites.items[0].id, node.own.id);
        client
            .favorite(&node.own.id, false, CancellationToken::new())
            .await
            .unwrap();
        assert!(
            client
                .tracks(None, None, true, CancellationToken::new())
                .await
                .unwrap()
                .items
                .is_empty()
        );
        client
            .favorite(&node.own.id, true, CancellationToken::new())
            .await
            .unwrap();
        let prepared = client
            .prepare(&node.own, CancellationToken::new(), progress().0)
            .await
            .unwrap();
        let path = prepared.path().to_owned();
        assert_eq!(prepared.length_bytes, AUDIO.len() as u64);
        assert_eq!(tokio::fs::read(&path).await.unwrap(), AUDIO);
        #[cfg(target_os = "linux")]
        let inode = {
            use std::os::unix::fs::MetadataExt;
            let metadata = path.metadata().unwrap();
            assert_eq!(metadata.nlink(), 0, "completed originals must be unlinked");
            assert_eq!(
                std::fs::read_dir(temporary.path()).unwrap().count(),
                0,
                "completed preparation must leave no named original"
            );
            (metadata.dev(), metadata.ino())
        };
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(path.metadata().unwrap().permissions().mode() & 0o777, 0o600);
        }
        let audio_lease = prepared.clone();
        drop(prepared);
        assert!(
            path.exists(),
            "the native source's lease must preserve its original"
        );
        assert_eq!(tokio::fs::read(&path).await.unwrap(), AUDIO);
        drop(audio_lease);
        // The background cache publication holds its own lease on the verified
        // original while it copies it: the private file stays unnamed and out of
        // the directory until the publication ends.
        assert_eq!(
            std::fs::read_dir(temporary.path()).unwrap().count(),
            0,
            "a running publication keeps the original unnamed"
        );
        await_publication(
            &client,
            &cache_key(&node.http.origin, &auth.user.id, &node.own.id),
        )
        .await;
        #[cfg(target_os = "linux")]
        if let Ok(metadata) = path.metadata() {
            use std::os::unix::fs::MetadataExt;
            // Parallel tests can immediately reuse the released descriptor
            // number. Its new inode must not be mistaken for our old lease.
            assert_ne!(
                (metadata.dev(), metadata.ino()),
                inode,
                "the final lease must close the original descriptor"
            );
        }
        #[cfg(not(target_os = "linux"))]
        assert!(
            !path.exists(),
            "the final native source lease must remove the temporary original"
        );
        assert_eq!(std::fs::read_dir(temporary.path()).unwrap().count(), 0);
        let session = client.session().unwrap();
        let clone = client.clone();
        let raw_cookie = session.cookie.to_str().unwrap().to_owned();
        assert!(!format!("{:?}", session.cookie).contains(&raw_cookie));
        client.logout(CancellationToken::new()).await.unwrap();
        assert!(clone.inner.session.lock().unwrap().is_none());
        assert_eq!(
            clone
                .tracks(None, None, false, CancellationToken::new())
                .await
                .unwrap_err(),
            SIGNED_OUT
        );
        assert_eq!(
            Client::new()
                .get(format!("{}/api/me", node.http.origin))
                .header(header::COOKIE, raw_cookie)
                .send()
                .await
                .unwrap()
                .status(),
            StatusCode::UNAUTHORIZED
        );
        clone.logout(CancellationToken::new()).await.unwrap();
        // No private-listening ingestion is hidden in this adapter.
        assert!(
            !node
                .state
                .listening
                .preferences(&auth.user, &uuid::Uuid::new_v4().to_string())
                .await
                .unwrap()
                .enabled
        );
    }

    #[tokio::test]
    async fn real_vela_server_reauthorizes_then_reuses_the_complete_original() {
        let node = Node::new().await;
        let (mut client, auth) = node.connect().await;
        let temporary = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();
        {
            let inner = Arc::get_mut(&mut client.inner).unwrap();
            inner.temporary_directory = Some(temporary.path().to_owned());
            inner.cache_directory = Some(cache.path().to_owned());
        }
        let key = cache_key(&node.http.origin, &auth.user.id, &node.own.id);
        let cached_audio = media_cache::audio_path(cache.path(), &key);
        let cached_sidecar = media_cache::sidecar_path(cache.path(), &key);

        // Cold: one authorized full transfer, verified, then published to the cache.
        let cold = client
            .prepare(&node.own, CancellationToken::new(), progress().0)
            .await
            .unwrap();
        assert_eq!(std::fs::read(cold.path()).unwrap(), AUDIO);
        assert_eq!(cold.length_bytes, AUDIO.len() as u64);
        #[cfg(target_os = "linux")]
        {
            use std::os::unix::fs::MetadataExt;
            assert_eq!(
                cold.path().metadata().unwrap().nlink(),
                0,
                "a cold lease stays an unnamed private original"
            );
        }
        drop(cold);
        // The cache is warmed in the background: the publication publishes the
        // verified bytes after the lease was already handed out.
        await_publication(&client, &key).await;
        assert_eq!(std::fs::read(&cached_audio).unwrap(), AUDIO);
        let sidecar: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&cached_sidecar).unwrap()).unwrap();
        assert_eq!(sidecar["version"], 1);
        assert_eq!(
            sidecar["origin"],
            canonical_origin(&node.http.origin).unwrap().as_str(),
            "the namespace records the canonical origin it belongs to"
        );
        assert_eq!(sidecar["account_id"], auth.user.id);
        assert_eq!(sidecar["track_id"], node.own.id);
        assert_eq!(sidecar["size_bytes"], AUDIO.len() as u64);
        // The sidecar digest is the authoritative origin ETag, which the real server
        // computes as the BLAKE3 of the managed original.
        assert_eq!(sidecar["etag"], blake3::hash(AUDIO).to_hex().as_str());
        assert_eq!(sidecar["digest"], sidecar["etag"]);
        assert!(sidecar["saved_at"].as_u64().unwrap() >= 1);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = |path: &std::path::Path| {
                std::fs::metadata(path).unwrap().permissions().mode() & 0o777
            };
            assert_eq!(mode(&media_cache::account_root(cache.path(), &key)), 0o700);
            assert_eq!(mode(&cached_audio), 0o600);
            assert_eq!(mode(&cached_sidecar), 0o600);
        }

        // The second resolve revalidates with a bounded HEAD and then hands back the
        // published cache file itself instead of another full transfer.
        let (sender, receiver) = progress();
        let warm = client
            .prepare(&node.own, CancellationToken::new(), sender)
            .await
            .unwrap();
        assert_eq!(receiver.borrow().received_bytes, AUDIO.len() as u64);
        assert_eq!(std::fs::read(warm.path()).unwrap(), AUDIO);
        #[cfg(target_os = "linux")]
        {
            use std::os::unix::fs::MetadataExt;
            let lease = warm.path().metadata().unwrap();
            let published = cached_audio.metadata().unwrap();
            assert_eq!(lease.nlink(), 1, "a hit leases the published cache file");
            assert_eq!(
                (lease.dev(), lease.ino()),
                (published.dev(), published.ino()),
                "a hit decodes the very file the cache verified"
            );
        }
        drop(warm);
        assert_eq!(std::fs::read(&cached_audio).unwrap(), AUDIO);

        // A foreign track is denied by the origin first and never reads this namespace.
        assert!(
            client
                .prepare(&node.foreign, CancellationToken::new(), progress().0)
                .await
                .is_err()
        );
        assert_eq!(std::fs::read(&cached_audio).unwrap(), AUDIO);
        assert_eq!(std::fs::read_dir(temporary.path()).unwrap().count(), 0);
    }

    #[test]
    fn server_origins_and_closed_identifiers_reject_credential_and_path_injection() {
        for origin in [
            "http://localhost:3210",
            "http://10.0.0.1",
            "http://example.org",
            "http://[::ffff:127.0.0.1]",
            "https://user:private-password@example.org",
            "https://@example.org",
            "https://example.org/private",
            "https://example.org/a/..",
            "https://example.org/?private-token",
            "https://example.org#fragment",
            "https://example.org:0",
            "file:///etc/passwd",
            " https://example.org",
            "https:\\example.org",
        ] {
            let error = canonical_origin(origin).unwrap_err();
            assert!(!error.contains(origin));
            assert!(!error.contains("private-password"));
        }
        for origin in [
            "https://music.example.org",
            "https://localhost:8443/",
            "https://[::1]:8443",
            "http://127.0.0.1:3210",
            "http://[::1]:3210/",
        ] {
            assert!(
                canonical_origin(origin).is_ok(),
                "valid explicit origin was rejected"
            );
        }
        assert!(!canonical_uuid("../../api/public/media/anything"));
        assert!(!canonical_uuid("20000000-0000-4000-8000-00000000000A"));
    }

    struct Dropped(Arc<AtomicBool>);
    impl Drop for Dropped {
        fn drop(&mut self) {
            self.0.store(true, Ordering::SeqCst);
        }
    }

    #[tokio::test]
    async fn cancel_busy_retry_and_logout_drop_actual_transfer_and_private_temporary_files() {
        let directory = tempfile::tempdir().unwrap();
        let bytes = vec![7_u8; 16 * 1024];
        let fingerprint = blake3::hash(&bytes).to_hex().to_string();
        let calls = Arc::new(AtomicUsize::new(0));
        let dropped = Arc::new(AtomicBool::new(false));
        let handler_calls = calls.clone();
        let handler_dropped = dropped.clone();
        let router = Router::new().route(
            &format!("/api/tracks/{TRACK_ID}/stream"),
            get(move |method: Method| {
                let bytes = bytes.clone();
                let fingerprint = fingerprint.clone();
                // Only the body transfers are numbered: the cache revalidation of the
                // same route must not change which GET is the deliberately slow one.
                let slow = if method == Method::HEAD {
                    false
                } else {
                    handler_calls.fetch_add(1, Ordering::SeqCst) != 1
                };
                let dropped = handler_dropped.clone();
                async move {
                    let body = if slow {
                        let guard = Dropped(dropped);
                        Body::from_stream(futures_util::stream::unfold(
                            (false, guard),
                            move |(sent, guard)| async move {
                                if sent {
                                    std::future::pending::<()>().await;
                                }
                                Some((Ok::<_, std::io::Error>(vec![7_u8; 8 * 1024]), (true, guard)))
                            },
                        ))
                    } else {
                        Body::from(bytes)
                    };
                    HttpResponse::builder()
                        .header(header::ETAG, format!("\"{fingerprint}\""))
                        .header(header::CONTENT_LENGTH, 16 * 1024)
                        .body(body)
                        .unwrap()
                }
            }),
        );
        let http = HttpFixture::start(with_login(router)).await;
        let cache = tempfile::tempdir().unwrap();
        let client = fixture_client(&http, &directory, &cache).await;
        let track = sample_track();
        let (sender, mut receiver) = progress();
        let token = CancellationToken::new();
        let running_client = client.clone();
        let running_track = track.clone();
        let running_token = token.clone();
        let running = tokio::spawn(async move {
            running_client
                .prepare(&running_track, running_token, sender)
                .await
        });
        tokio::time::timeout(Duration::from_secs(3), async {
            while receiver.borrow().received_bytes == 0 {
                receiver.changed().await.unwrap();
            }
        })
        .await
        .unwrap();
        assert_eq!(
            std::fs::read_dir(directory.path()).unwrap().count(),
            0,
            "an in-progress original must never have a directory entry"
        );
        assert_eq!(
            client
                .prepare(&track, CancellationToken::new(), progress().0)
                .await
                .unwrap_err(),
            PREPARE_BUSY
        );
        assert_eq!(
            calls.load(Ordering::SeqCst),
            1,
            "Busy must not start a second transfer"
        );
        token.cancel();
        assert_eq!(
            tokio::time::timeout(Duration::from_secs(1), running)
                .await
                .unwrap()
                .unwrap()
                .unwrap_err(),
            CANCELED
        );
        eventually(|| {
            dropped.load(Ordering::SeqCst)
                && std::fs::read_dir(directory.path()).unwrap().count() == 0
                && client.inner.preparing.available_permits() == 1
        })
        .await;
        let prepared = client
            .prepare(&track, CancellationToken::new(), progress().0)
            .await
            .unwrap();
        drop(prepared);
        assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 0);
        // The previous prepare warmed the reusable cache in the background. This phase
        // deliberately needs an *in-flight* transfer, so the warm entry is dropped
        // instead of being served.
        await_publication(&client, &cache_key(&http.origin, USER_ID, TRACK_ID)).await;
        std::fs::remove_dir_all(cache.path()).unwrap();
        dropped.store(false, Ordering::SeqCst);
        let (sender, mut receiver) = progress();
        let running_client = client.clone();
        let running = tokio::spawn(async move {
            running_client
                .prepare(&track, CancellationToken::new(), sender)
                .await
        });
        tokio::time::timeout(Duration::from_secs(3), async {
            while receiver.borrow().received_bytes == 0 {
                receiver.changed().await.unwrap();
            }
        })
        .await
        .unwrap();
        client.logout(CancellationToken::new()).await.unwrap();
        assert_eq!(running.await.unwrap().unwrap_err(), SIGNED_OUT);
        eventually(|| {
            dropped.load(Ordering::SeqCst)
                && std::fs::read_dir(directory.path()).unwrap().count() == 0
        })
        .await;
        assert!(client.inner.session.lock().unwrap().is_none());
    }

    #[tokio::test]
    async fn redirects_never_receive_login_credentials_or_owned_session_headers() {
        let target = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let destination = format!(
            "http://{}/must-not-receive-secrets",
            target.local_addr().unwrap()
        );
        let next = destination.clone();
        let http = HttpFixture::start(Router::new().route(
            "/api/login",
            post(move || {
                let next = next.clone();
                async move {
                    (
                        StatusCode::TEMPORARY_REDIRECT,
                        [(header::LOCATION, next)],
                        "upstream private-password diagnostic",
                    )
                }
            }),
        ))
        .await;
        let result = VelaClient::connect(
            &http.origin,
            "fixture".into(),
            "private-password".into(),
            CancellationToken::new(),
        )
        .await;
        let error = match result {
            Ok(_) => panic!("login redirect must be rejected"),
            Err(error) => error,
        };
        assert!(!error.contains("private-password"));
        assert!(!error.contains(&destination));
        assert!(
            tokio::time::timeout(Duration::from_millis(100), target.accept())
                .await
                .is_err()
        );
        let next = destination.clone();
        let router = Router::new().route(
            &format!("/api/tracks/{TRACK_ID}/stream"),
            get(move || {
                let next = next.clone();
                async move {
                    (
                        StatusCode::FOUND,
                        [(header::LOCATION, next)],
                        "private upstream details",
                    )
                }
            }),
        );
        let http = HttpFixture::start(with_login(router)).await;
        let directory = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();
        let client = fixture_client(&http, &directory, &cache).await;
        let error = client
            .prepare(&sample_track(), CancellationToken::new(), progress().0)
            .await
            .unwrap_err();
        assert!(!error.contains("upstream"));
        assert!(!error.contains(&destination));
        assert!(
            tokio::time::timeout(Duration::from_millis(100), target.accept())
                .await
                .is_err()
        );
        assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 0);
    }

    #[tokio::test]
    async fn canceled_json_body_closes_its_connection_and_oversized_json_is_rejected() {
        let dropped = Arc::new(AtomicBool::new(false));
        let started = Arc::new(tokio::sync::Notify::new());
        let handler_dropped = dropped.clone();
        let handler_started = started.clone();
        let router = Router::new().route(
            "/api/tracks",
            get(move || {
                let guard = Dropped(handler_dropped.clone());
                let started = handler_started.clone();
                async move {
                    let stream =
                        futures_util::stream::unfold((false, guard), move |(sent, guard)| {
                            let started = started.clone();
                            async move {
                                if sent {
                                    std::future::pending::<()>().await;
                                }
                                started.notify_one();
                                Some((
                                    Ok::<_, std::io::Error>(b"{\"items\":[".to_vec()),
                                    (true, guard),
                                ))
                            }
                        });
                    HttpResponse::builder()
                        .body(Body::from_stream(stream))
                        .unwrap()
                }
            }),
        );
        let http = HttpFixture::start(with_login(router)).await;
        let directory = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();
        let client = fixture_client(&http, &directory, &cache).await;
        let cancel = CancellationToken::new();
        let running_client = client.clone();
        let running_cancel = cancel.clone();
        let request = tokio::spawn(async move {
            running_client
                .tracks(None, None, false, running_cancel)
                .await
        });
        tokio::time::timeout(Duration::from_secs(3), started.notified())
            .await
            .unwrap();
        cancel.cancel();
        assert_eq!(
            tokio::time::timeout(Duration::from_secs(1), request)
                .await
                .unwrap()
                .unwrap()
                .unwrap_err(),
            CANCELED
        );
        eventually(|| dropped.load(Ordering::SeqCst)).await;

        for declared in [false, true] {
            let router = Router::new().route(
                "/api/tracks",
                get(move || async move {
                    let mut response = HttpResponse::builder();
                    if declared {
                        response = response.header(header::CONTENT_LENGTH, MAX_JSON_BYTES + 1);
                    }
                    response
                        .body(Body::from_stream(futures_util::stream::once(async move {
                            Ok::<_, std::io::Error>(vec![b' '; MAX_JSON_BYTES + 1])
                        })))
                        .unwrap()
                }),
            );
            let http = HttpFixture::start(with_login(router)).await;
            let cache = tempfile::tempdir().unwrap();
            let client = fixture_client(&http, &directory, &cache).await;
            assert_eq!(
                client
                    .tracks(None, None, false, CancellationToken::new())
                    .await
                    .unwrap_err(),
                INVALID_RESPONSE
            );
        }
    }

    #[tokio::test]
    async fn size_hash_encoding_and_catalog_bounds_fail_without_publishing_a_temp_lease() {
        for case in [
            "missing-hash",
            "weak-hash",
            "wrong-hash",
            "short-body",
            "long-body",
            "encoding",
            "length",
        ] {
            let router = Router::new().route(
                &format!("/api/tracks/{TRACK_ID}/stream"),
                get(move || async move {
                    let bytes = match case {
                        "short-body" => vec![7_u8; 8 * 1024],
                        "long-body" => vec![7_u8; 32 * 1024],
                        _ => vec![7_u8; 16 * 1024],
                    };
                    let hash = if case == "wrong-hash" {
                        "0".repeat(64)
                    } else {
                        blake3::hash(&bytes).to_hex().to_string()
                    };
                    let mut response = HttpResponse::builder();
                    if case != "missing-hash" {
                        response = response.header(
                            header::ETAG,
                            if case == "weak-hash" {
                                format!("W/\"{hash}\"")
                            } else {
                                format!("\"{hash}\"")
                            },
                        );
                    }
                    if case == "encoding" {
                        response = response.header(header::CONTENT_ENCODING, "gzip");
                    }
                    if case == "length" {
                        response = response.header(header::CONTENT_LENGTH, 8 * 1024);
                    }
                    // A streaming body leaves Content-Length absent for size checks.
                    response
                        .body(Body::from_stream(futures_util::stream::once(async move {
                            Ok::<_, std::io::Error>(bytes)
                        })))
                        .unwrap()
                }),
            );
            let http = HttpFixture::start(with_login(router)).await;
            let directory = tempfile::tempdir().unwrap();
            let cache = tempfile::tempdir().unwrap();
            let client = fixture_client(&http, &directory, &cache).await;
            assert!(
                client
                    .prepare(&sample_track(), CancellationToken::new(), progress().0)
                    .await
                    .is_err(),
                "accepted hostile audio case {case}"
            );
            eventually(|| {
                client.inner.preparing.available_permits() == 1
                    && std::fs::read_dir(directory.path()).unwrap().count() == 0
            })
            .await;
        }
        for case in ["duplicate", "too-many", "cursor", "id", "negative"] {
            let mut page = TrackPage {
                items: vec![sample_track()],
                next_cursor: None,
            };
            match case {
                "duplicate" => page.items.push(sample_track()),
                "too-many" => page.items = vec![sample_track(); 51],
                "cursor" => page.next_cursor = Some(USER_ID.into()),
                "id" => page.items[0].id = "../../private".into(),
                "negative" => page.items[0].size_bytes = -1,
                _ => unreachable!(),
            }
            let router = Router::new().route(
                "/api/tracks",
                get(move || {
                    let page = page.clone();
                    async move { axum::Json(page) }
                }),
            );
            let http = HttpFixture::start(with_login(router)).await;
            let directory = tempfile::tempdir().unwrap();
            let cache = tempfile::tempdir().unwrap();
            let client = fixture_client(&http, &directory, &cache).await;
            assert_eq!(
                client
                    .tracks(None, None, false, CancellationToken::new())
                    .await
                    .unwrap_err(),
                INVALID_RESPONSE
            );
        }
    }

    /// Byte-level evidence of what one stream route actually transferred.
    #[derive(Default)]
    struct StreamCounters {
        gets: AtomicUsize,
        heads: AtomicUsize,
        body_bytes: AtomicUsize,
    }

    /// Two independently counted stream routes, so a cached replay is measured by
    /// transferred body bytes instead of by a shorter spinner.
    fn tracked_streams(tracks: &[(String, Vec<u8>, Arc<StreamCounters>)]) -> Router {
        let mut router = Router::new();
        for (id, bytes, counters) in tracks {
            let bytes = bytes.clone();
            let fingerprint = blake3::hash(&bytes).to_hex().to_string();
            let counters = counters.clone();
            router = router.route(
                &format!("/api/tracks/{id}/stream"),
                get(move |method: Method, headers: HeaderMap| {
                    let bytes = bytes.clone();
                    let fingerprint = fingerprint.clone();
                    let counters = counters.clone();
                    async move {
                        // A revalidation is an authenticated request, exactly like the GET.
                        assert_eq!(
                            headers[header::COOKIE],
                            format!("vela_session={}", "a".repeat(64)),
                            "revalidation must carry the current session"
                        );
                        if method == Method::HEAD {
                            counters.heads.fetch_add(1, Ordering::SeqCst);
                        } else {
                            counters.gets.fetch_add(1, Ordering::SeqCst);
                            counters.body_bytes.fetch_add(bytes.len(), Ordering::SeqCst);
                        }
                        HttpResponse::builder()
                            .header(header::ETAG, format!("\"{fingerprint}\""))
                            .header(header::CONTENT_LENGTH, bytes.len())
                            .body(Body::from(bytes))
                            .unwrap()
                    }
                }),
            );
        }
        router
    }

    #[tokio::test]
    async fn a_b_a_reuses_the_complete_original_without_a_second_body_transfer() {
        let first_bytes = vec![11_u8; 16 * 1024];
        let second_bytes = vec![22_u8; 12 * 1024];
        let first_counters = Arc::new(StreamCounters::default());
        let second_counters = Arc::new(StreamCounters::default());
        let router = tracked_streams(&[
            (
                "20000000-0000-4000-8000-0000000000a1".to_owned(),
                first_bytes.clone(),
                first_counters.clone(),
            ),
            (
                "20000000-0000-4000-8000-0000000000b2".to_owned(),
                second_bytes.clone(),
                second_counters.clone(),
            ),
        ]);
        let http = HttpFixture::start(with_login(router)).await;
        let temporary = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();
        let client = fixture_client(&http, &temporary, &cache).await;
        let first = sample_track_with("20000000-0000-4000-8000-0000000000a1", first_bytes.len());
        let second = sample_track_with("20000000-0000-4000-8000-0000000000b2", second_bytes.len());

        // A: cold, one authorized full transfer.
        let prepared = client
            .prepare(&first, CancellationToken::new(), progress().0)
            .await
            .unwrap();
        assert_eq!(std::fs::read(prepared.path()).unwrap(), first_bytes);
        drop(prepared);
        // B: cold, one authorized full transfer.
        let prepared = client
            .prepare(&second, CancellationToken::new(), progress().0)
            .await
            .unwrap();
        assert_eq!(std::fs::read(prepared.path()).unwrap(), second_bytes);
        drop(prepared);
        assert_eq!(first_counters.gets.load(Ordering::SeqCst), 1);
        assert_eq!(
            first_counters.body_bytes.load(Ordering::SeqCst),
            first_bytes.len()
        );
        assert_eq!(second_counters.gets.load(Ordering::SeqCst), 1);
        assert_eq!(
            second_counters.body_bytes.load(Ordering::SeqCst),
            second_bytes.len()
        );

        // A again: the origin re-authorizes, and the verified local bytes are reused.
        let (sender, receiver) = progress();
        let warm = client
            .prepare(&first, CancellationToken::new(), sender)
            .await
            .unwrap();
        assert_eq!(std::fs::read(warm.path()).unwrap(), first_bytes);
        assert_eq!(
            receiver.borrow().received_bytes,
            first_bytes.len() as u64,
            "a warm hit reports the complete original, not a partial transfer"
        );
        drop(warm);
        assert_eq!(
            first_counters.gets.load(Ordering::SeqCst),
            1,
            "A→B→A must not re-download A's body"
        );
        assert_eq!(
            first_counters.body_bytes.load(Ordering::SeqCst),
            first_bytes.len()
        );
        assert_eq!(
            first_counters.heads.load(Ordering::SeqCst),
            2,
            "a hit is still re-authorized by the origin"
        );
        assert_eq!(second_counters.gets.load(Ordering::SeqCst), 1);

        // Both complete originals are cached for this account+origin namespace only.
        for (id, expected) in [
            ("20000000-0000-4000-8000-0000000000a1", &first_bytes),
            ("20000000-0000-4000-8000-0000000000b2", &second_bytes),
        ] {
            let key = cache_key(&http.origin, USER_ID, id);
            await_publication(&client, &key).await;
            assert_eq!(
                std::fs::read(media_cache::audio_path(cache.path(), &key)).unwrap(),
                *expected
            );
            assert!(media_cache::sidecar_path(cache.path(), &key).is_file());
        }
        // The decoder leases stayed unnamed private originals; nothing is left behind.
        assert_eq!(std::fs::read_dir(temporary.path()).unwrap().count(), 0);
    }

    #[tokio::test]
    async fn cached_bytes_never_answer_a_denied_or_changed_origin() {
        let original = vec![5_u8; 8 * 1024];
        let counters = Arc::new(StreamCounters::default());
        // 200 means the origin currently authorizes this representation.
        let head_status = Arc::new(AtomicUsize::new(200));
        let representation = Arc::new(Mutex::new(original.clone()));
        let router = {
            let counters = counters.clone();
            let head_status = head_status.clone();
            let representation = representation.clone();
            Router::new().route(
                &format!("/api/tracks/{TRACK_ID}/stream"),
                get(move |method: Method| {
                    let counters = counters.clone();
                    let head_status = head_status.clone();
                    let representation = representation.clone();
                    async move {
                        let bytes = representation.lock().unwrap().clone();
                        let fingerprint = blake3::hash(&bytes).to_hex().to_string();
                        if method == Method::HEAD {
                            counters.heads.fetch_add(1, Ordering::SeqCst);
                            let status =
                                StatusCode::from_u16(head_status.load(Ordering::SeqCst) as u16)
                                    .unwrap();
                            if status != StatusCode::OK {
                                return HttpResponse::builder()
                                    .status(status)
                                    .body(Body::empty())
                                    .unwrap();
                            }
                            return HttpResponse::builder()
                                .header(header::ETAG, format!("\"{fingerprint}\""))
                                .header(header::CONTENT_LENGTH, bytes.len())
                                .body(Body::empty())
                                .unwrap();
                        }
                        counters.gets.fetch_add(1, Ordering::SeqCst);
                        counters.body_bytes.fetch_add(bytes.len(), Ordering::SeqCst);
                        HttpResponse::builder()
                            .header(header::ETAG, format!("\"{fingerprint}\""))
                            .header(header::CONTENT_LENGTH, bytes.len())
                            .body(Body::from(bytes))
                            .unwrap()
                    }
                }),
            )
        };
        let http = HttpFixture::start(with_login(router)).await;
        let temporary = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();
        let client = fixture_client(&http, &temporary, &cache).await;
        let track = sample_track_with(TRACK_ID, original.len());
        let key = cache_key(&http.origin, USER_ID, TRACK_ID);

        // Cold: the origin authorizes the representation once and the cache warms.
        let prepared = client
            .prepare(&track, CancellationToken::new(), progress().0)
            .await
            .unwrap();
        assert_eq!(std::fs::read(prepared.path()).unwrap(), original);
        drop(prepared);
        assert_eq!(counters.gets.load(Ordering::SeqCst), 1);
        await_publication(&client, &key).await;
        let cached = std::fs::read(media_cache::audio_path(cache.path(), &key)).unwrap();
        assert_eq!(cached, original);

        // A denial is the same error the GET would give, never a cached success.
        for (denied, status) in [401_u16, 403, 404].into_iter().enumerate() {
            let denied = denied + 1;
            head_status.store(status as usize, Ordering::SeqCst);
            assert_eq!(
                client
                    .prepare(&track, CancellationToken::new(), progress().0)
                    .await
                    .unwrap_err(),
                status_error(StatusCode::from_u16(status).unwrap())
            );
            assert_eq!(
                counters.gets.load(Ordering::SeqCst),
                1,
                "a denied request must not transfer or reuse cached bytes"
            );
            assert_eq!(
                counters.heads.load(Ordering::SeqCst),
                denied + 1,
                "every candidate hit is revalidated by the origin"
            );
            assert_eq!(
                std::fs::read(media_cache::audio_path(cache.path(), &key)).unwrap(),
                cached,
                "a denial must not delete or serve the local entry"
            );
        }
        // An expired local digest cannot hide a revocation either: the cache is only
        // consulted after the origin answered 200 for the current session.

        // An origin without HEAD support still gets the full authorized GET.
        head_status.store(405, Ordering::SeqCst);
        let prepared = client
            .prepare(&track, CancellationToken::new(), progress().0)
            .await
            .unwrap();
        assert_eq!(std::fs::read(prepared.path()).unwrap(), original);
        drop(prepared);
        assert_eq!(
            counters.gets.load(Ordering::SeqCst),
            2,
            "HEAD 405 falls back to the authorized GET"
        );

        // A changed validator is a different representation: never the old bytes.
        head_status.store(200, Ordering::SeqCst);
        let replacement = vec![6_u8; 8 * 1024 + 4];
        *representation.lock().unwrap() = replacement.clone();
        let changed = sample_track_with(TRACK_ID, replacement.len());
        let prepared = client
            .prepare(&changed, CancellationToken::new(), progress().0)
            .await
            .unwrap();
        assert_eq!(std::fs::read(prepared.path()).unwrap(), replacement);
        drop(prepared);
        assert_eq!(
            counters.gets.load(Ordering::SeqCst),
            3,
            "a changed validator must re-read the origin"
        );
        await_publication(&client, &key).await;
        assert_eq!(
            std::fs::read(media_cache::audio_path(cache.path(), &key)).unwrap(),
            replacement,
            "the replacement is published as the new cached representation"
        );
        let superseded = blake3::hash(&original).to_hex();
        assert!(
            media_cache::open_verified(
                cache.path(),
                &key,
                original.len() as u64,
                superseded.as_str(),
                &|| false,
            )
            .unwrap()
            .is_none(),
            "the superseded representation must not be reusable"
        );
    }

    /// One stream route that delivers its body in bounded chunks with a real
    /// delay, so a test can measure a preparation whose transfer is still in
    /// flight.
    fn slow_streams(
        tracks: &[(String, Vec<u8>, Arc<StreamCounters>)],
        chunk: usize,
        delay: Duration,
    ) -> Router {
        let mut router = Router::new();
        for (id, bytes, counters) in tracks {
            let bytes = Arc::new(bytes.clone());
            let fingerprint = blake3::hash(bytes.as_slice()).to_hex().to_string();
            let counters = counters.clone();
            router = router.route(
                &format!("/api/tracks/{id}/stream"),
                get(move |method: Method, headers: HeaderMap| {
                    let bytes = bytes.clone();
                    let fingerprint = fingerprint.clone();
                    let counters = counters.clone();
                    async move {
                        assert_eq!(
                            headers[header::COOKIE],
                            format!("vela_session={}", "a".repeat(64)),
                            "revalidation must carry the current session"
                        );
                        if method == Method::HEAD {
                            counters.heads.fetch_add(1, Ordering::SeqCst);
                            return HttpResponse::builder()
                                .header(header::ETAG, format!("\"{fingerprint}\""))
                                .header(header::CONTENT_LENGTH, bytes.len())
                                .body(Body::from(bytes.as_ref().clone()))
                                .unwrap();
                        }
                        counters.gets.fetch_add(1, Ordering::SeqCst);
                        counters.body_bytes.fetch_add(bytes.len(), Ordering::SeqCst);
                        let total = bytes.len();
                        let stream = futures_util::stream::unfold(0_usize, {
                            let bytes = bytes.clone();
                            move |state| {
                                let bytes = bytes.clone();
                                async move {
                                    if state >= total {
                                        return None;
                                    }
                                    let end = (state + chunk).min(total);
                                    tokio::time::sleep(delay).await;
                                    Some((Ok::<_, std::io::Error>(bytes[state..end].to_vec()), end))
                                }
                            }
                        });
                        HttpResponse::builder()
                            .header(header::ETAG, format!("\"{fingerprint}\""))
                            .body(Body::from_stream(stream))
                            .unwrap()
                    }
                }),
            );
        }
        router
    }

    /// A fixture client whose private temporary originals, reusable media cache and
    /// background publication gate are all isolated: no test reads or writes a live
    /// `$XDG_CACHE_HOME`/`$XDG_DATA_HOME` and every test can assert its own
    /// leftovers.
    async fn fixture_client_with_pause(
        http: &HttpFixture,
        directory: &tempfile::TempDir,
        cache: &tempfile::TempDir,
        pause: &Arc<PublicationPause>,
    ) -> VelaClient {
        let mut client = fixture_client(http, directory, cache).await;
        let inner = Arc::get_mut(&mut client.inner).unwrap();
        inner.publication_pause = std::sync::Mutex::new(Some(pause.clone()));
        client
    }

    #[tokio::test]
    async fn tampered_cached_bytes_are_downloaded_again_instead_of_served() {
        let original = vec![7_u8; 4 * 1024];
        let counters = Arc::new(StreamCounters::default());
        let router = tracked_streams(&[(TRACK_ID.to_owned(), original.clone(), counters.clone())]);
        let http = HttpFixture::start(with_login(router)).await;
        let temporary = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();
        let client = fixture_client(&http, &temporary, &cache).await;
        let track = sample_track_with(TRACK_ID, original.len());
        let key = cache_key(&http.origin, USER_ID, TRACK_ID);
        let audio = media_cache::audio_path(cache.path(), &key);
        let sidecar = media_cache::sidecar_path(cache.path(), &key);

        let prepared = client
            .prepare(&track, CancellationToken::new(), progress().0)
            .await
            .unwrap();
        assert_eq!(std::fs::read(prepared.path()).unwrap(), original);
        drop(prepared);
        // The entry is published in the background, so the tamper below acts on
        // the published bytes and not on a half-written file.
        await_publication(&client, &key).await;

        // Same-size tampering is rejected by the local BLAKE3 and re-downloaded.
        let mut tampered = original.clone();
        tampered[10] ^= 0xff;
        std::fs::write(&audio, &tampered).unwrap();
        let prepared = client
            .prepare(&track, CancellationToken::new(), progress().0)
            .await
            .unwrap();
        assert_eq!(
            std::fs::read(prepared.path()).unwrap(),
            original,
            "a tampered cache entry must never reach the decoder"
        );
        drop(prepared);
        assert_eq!(counters.gets.load(Ordering::SeqCst), 2);
        await_publication(&client, &key).await;
        assert_eq!(
            std::fs::read(&audio).unwrap(),
            original,
            "the republished entry is healthy again"
        );

        // Rewriting the sidecar as well cannot forge a hit: the origin's ETag is the
        // authoritative validator, so local bytes never outvote it.
        let tampered_digest = blake3::hash(&tampered).to_hex().to_string();
        std::fs::write(&audio, &tampered).unwrap();
        let mut record: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&sidecar).unwrap()).unwrap();
        record["digest"] = serde_json::Value::String(tampered_digest);
        std::fs::write(&sidecar, serde_json::to_vec(&record).unwrap()).unwrap();
        let prepared = client
            .prepare(&track, CancellationToken::new(), progress().0)
            .await
            .unwrap();
        assert_eq!(std::fs::read(prepared.path()).unwrap(), original);
        drop(prepared);
        assert_eq!(
            counters.gets.load(Ordering::SeqCst),
            3,
            "a self-consistent tampered pair must still be re-downloaded"
        );
    }

    #[tokio::test]
    async fn the_cache_publication_does_not_delay_the_prepared_lease() {
        let original = vec![41_u8; 64 * 1024];
        let counters = Arc::new(StreamCounters::default());
        let router = tracked_streams(&[(TRACK_ID.to_owned(), original.clone(), counters.clone())]);
        let http = HttpFixture::start(with_login(router)).await;
        let temporary = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();
        let pause = PublicationPause::new();
        let client = fixture_client_with_pause(&http, &temporary, &cache, &pause).await;
        let track = sample_track_with(TRACK_ID, original.len());
        let key = cache_key(&http.origin, USER_ID, TRACK_ID);
        let cached_audio = media_cache::audio_path(cache.path(), &key);

        // The lease is returned while the cache publication is still running: the
        // local copy of the verified original is not part of the path to the
        // decoder.
        let prepared = client
            .prepare(&track, CancellationToken::new(), progress().0)
            .await
            .unwrap();
        assert_eq!(std::fs::read(prepared.path()).unwrap(), original);
        eventually(|| pause.entered.load(Ordering::SeqCst) == 1).await;
        assert_eq!(
            pause.finished.load(Ordering::SeqCst),
            0,
            "the publication must still be running when the lease is returned"
        );
        assert!(
            !cached_audio.exists(),
            "the cache entry is published after the lease, not before it"
        );
        assert_eq!(counters.gets.load(Ordering::SeqCst), 1);

        // Releasing the publication publishes exactly the verified bytes.
        pause.release();
        eventually(|| pause.finished.load(Ordering::SeqCst) == 1).await;
        eventually(|| cached_audio.exists()).await;
        assert_eq!(std::fs::read(&cached_audio).unwrap(), original);
        assert!(media_cache::sidecar_path(cache.path(), &key).is_file());
    }

    #[tokio::test]
    async fn a_second_prepare_waits_for_the_running_publication_instead_of_a_second_transfer() {
        let original = vec![42_u8; 64 * 1024];
        let counters = Arc::new(StreamCounters::default());
        let router = tracked_streams(&[(TRACK_ID.to_owned(), original.clone(), counters.clone())]);
        let http = HttpFixture::start(with_login(router)).await;
        let temporary = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();
        let pause = PublicationPause::new();
        let client = fixture_client_with_pause(&http, &temporary, &cache, &pause).await;
        let track = sample_track_with(TRACK_ID, original.len());
        let key = cache_key(&http.origin, USER_ID, TRACK_ID);

        let first = client
            .prepare(&track, CancellationToken::new(), progress().0)
            .await
            .unwrap();
        assert_eq!(std::fs::read(first.path()).unwrap(), original);
        drop(first);
        eventually(|| pause.entered.load(Ordering::SeqCst) == 1).await;

        // A second prepare of the same track must not start a second transfer
        // while the publication of the first one is still running.
        let waiting = {
            let client = client.clone();
            let track = track.clone();
            tokio::spawn(async move {
                client
                    .prepare(&track, CancellationToken::new(), progress().0)
                    .await
            })
        };
        tokio::time::sleep(Duration::from_millis(200)).await;
        assert_eq!(
            counters.gets.load(Ordering::SeqCst),
            1,
            "the second prepare must not transfer the body again"
        );
        assert!(
            !waiting.is_finished(),
            "the second prepare waits for the running publication"
        );

        // The publication ends: the second prepare is served from the cache.
        pause.release();
        let second = tokio::time::timeout(Duration::from_secs(5), waiting)
            .await
            .expect("the second prepare did not finish")
            .unwrap()
            .unwrap();
        assert_eq!(std::fs::read(second.path()).unwrap(), original);
        assert_eq!(
            counters.gets.load(Ordering::SeqCst),
            1,
            "one body per track is transferred once"
        );
        assert_eq!(counters.body_bytes.load(Ordering::SeqCst), original.len());
        assert!(media_cache::sidecar_path(cache.path(), &key).is_file());
    }

    #[tokio::test]
    async fn a_failed_publication_keeps_playback_working_and_costs_exactly_one_more_transfer() {
        let original = vec![43_u8; 32 * 1024];
        let counters = Arc::new(StreamCounters::default());
        let router = tracked_streams(&[(TRACK_ID.to_owned(), original.clone(), counters.clone())]);
        let http = HttpFixture::start(with_login(router)).await;
        let temporary = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();
        // The cache root is a regular file: the publication cannot create its
        // namespace, so the warm-up fails while playback keeps working.
        let blocked = cache.path().join("blocked");
        std::fs::write(&blocked, b"not a directory").unwrap();
        let mut client = fixture_client(&http, &temporary, &cache).await;
        let inner = Arc::get_mut(&mut client.inner).unwrap();
        inner.cache_directory = Some(blocked);
        let track = sample_track_with(TRACK_ID, original.len());

        let first = client
            .prepare(&track, CancellationToken::new(), progress().0)
            .await
            .unwrap();
        assert_eq!(
            std::fs::read(first.path()).unwrap(),
            original,
            "a failed publication must never invalidate playback"
        );
        drop(first);
        assert_eq!(counters.gets.load(Ordering::SeqCst), 1);

        // The publication failed, so the cache stays cold and the next prepare
        // makes exactly one new transfer.
        let second = client
            .prepare(&track, CancellationToken::new(), progress().0)
            .await
            .unwrap();
        assert_eq!(std::fs::read(second.path()).unwrap(), original);
        assert_eq!(
            counters.gets.load(Ordering::SeqCst),
            2,
            "a failed publication costs exactly one new transfer"
        );
        assert_eq!(
            counters.body_bytes.load(Ordering::SeqCst),
            original.len() * 2
        );
    }

    #[tokio::test]
    async fn a_sign_out_stops_a_running_publication() {
        let original = vec![44_u8; 64 * 1024];
        let counters = Arc::new(StreamCounters::default());
        let router = tracked_streams(&[(TRACK_ID.to_owned(), original.clone(), counters.clone())]);
        let http = HttpFixture::start(with_login(router)).await;
        let temporary = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();
        let pause = PublicationPause::new();
        let client = fixture_client_with_pause(&http, &temporary, &cache, &pause).await;
        let track = sample_track_with(TRACK_ID, original.len());
        let key = cache_key(&http.origin, USER_ID, TRACK_ID);
        let cached_audio = media_cache::audio_path(cache.path(), &key);
        let cached_sidecar = media_cache::sidecar_path(cache.path(), &key);

        let prepared = client
            .prepare(&track, CancellationToken::new(), progress().0)
            .await
            .unwrap();
        drop(prepared);
        eventually(|| pause.entered.load(Ordering::SeqCst) == 1).await;

        // The sign-out ends the client's lifetime: the publication stops and a
        // late publication must not come back to life.
        client.logout(CancellationToken::new()).await.unwrap();
        pause.release();
        eventually(|| pause.finished.load(Ordering::SeqCst) == 1).await;
        assert!(
            !cached_audio.exists(),
            "a sign-out must not publish a late cache entry"
        );
        assert!(!cached_sidecar.exists());
        // The canceled publication leaves no staging file behind.
        let account = media_cache::account_root(cache.path(), &key);
        if account.exists() {
            assert!(
                std::fs::read_dir(&account).unwrap().all(|entry| !entry
                    .unwrap()
                    .file_name()
                    .to_string_lossy()
                    .ends_with(".tmp")),
                "a canceled publication leaves no staging file"
            );
        }
    }

    #[tokio::test]
    async fn the_lease_is_returned_before_the_cache_copy_of_a_slow_body_finishes() {
        let original = vec![47_u8; 8 * 1024 * 1024];
        let counters = Arc::new(StreamCounters::default());
        let router = slow_streams(
            &[(TRACK_ID.to_owned(), original.clone(), counters.clone())],
            1024 * 1024,
            Duration::from_millis(25),
        );
        let http = HttpFixture::start(with_login(router)).await;
        let temporary = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();
        let client = fixture_client(&http, &temporary, &cache).await;
        let track = sample_track_with(TRACK_ID, original.len());
        let key = cache_key(&http.origin, USER_ID, TRACK_ID);
        let cached_audio = media_cache::audio_path(cache.path(), &key);

        let started = std::time::Instant::now();
        let prepared = client
            .prepare(&track, CancellationToken::new(), progress().0)
            .await
            .unwrap();
        let leased = started.elapsed();
        assert_eq!(std::fs::read(prepared.path()).unwrap(), original);
        assert_eq!(counters.gets.load(Ordering::SeqCst), 1);
        // The local copy of eight mebibytes cannot finish before the very next
        // system call, so the entry is provably not published yet: the lease was
        // handed out before the copy, not after it.
        assert!(
            !cached_audio.exists(),
            "the lease must be returned before the cache copy, not after it"
        );

        // The cache entry appears only after the local copy of the verified
        // original finished, so the lease was handed out before that copy.
        eventually(|| cached_audio.exists()).await;
        let published = started.elapsed();
        eprintln!(
            "TL-01 measurement: 8 MiB original, lease returned after {leased:?}, cache entry published after {published:?}, publication cost {:?}",
            published.saturating_sub(leased)
        );
        assert!(
            leased < published,
            "the lease must not wait for the cache copy: leased after {leased:?}, published after {published:?}"
        );
    }

    #[tokio::test]
    async fn the_registry_of_running_publications_is_bounded() {
        let original = vec![53_u8; 16 * 1024];
        let counters = Arc::new(StreamCounters::default());
        let router = tracked_streams(&[(TRACK_ID.to_owned(), original.clone(), counters.clone())]);
        let http = HttpFixture::start(with_login(router)).await;
        let temporary = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();
        let client = fixture_client(&http, &temporary, &cache).await;
        let track = sample_track_with(TRACK_ID, original.len());
        let cached_audio =
            media_cache::audio_path(cache.path(), &cache_key(&http.origin, USER_ID, TRACK_ID));

        // Fill the registry with the maximum number of running publications.
        let mut gates = Vec::new();
        {
            let mut registry = client.inner.publications.lock().unwrap();
            for index in 0..MAX_IN_FLIGHT_PUBLICATIONS {
                let other = cache_key(
                    &http.origin,
                    USER_ID,
                    &format!("30000000-0000-4000-8000-{index:012x}"),
                );
                let gate = Arc::new(tokio::sync::Mutex::new(()));
                registry.insert(other, Arc::downgrade(&gate));
                gates.push(gate);
            }
            assert_eq!(registry.len(), MAX_IN_FLIGHT_PUBLICATIONS);
        }

        // The next publication is refused, so the warm-up stays inside prepare
        // instead of starting one more background copy.
        let prepared = client
            .prepare(&track, CancellationToken::new(), progress().0)
            .await
            .unwrap();
        assert_eq!(std::fs::read(prepared.path()).unwrap(), original);
        drop(prepared);
        assert!(
            cached_audio.exists(),
            "a full registry keeps the previous synchronous publication"
        );
        assert_eq!(counters.gets.load(Ordering::SeqCst), 1);
    }

    const ARTWORK_ID: &str = "30000000-0000-4000-8000-000000000001";
    const OTHER_ARTWORK_ID: &str = "30000000-0000-4000-8000-000000000002";
    const OTHER_USER_ID: &str = "10000000-0000-4000-8000-000000000009";
    /// A structurally valid JPEG header/footer. The native client never re-decodes art;
    /// it only proves the bytes are the declared raster type.
    const COVER: &[u8] = &[
        0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, b'J', b'F', b'I', b'F', 0x00, 0x01, 0x01, 0x00, 0x00,
        0x01, 0x00, 0x01, 0x00, 0x00, 0xFF, 0xD9,
    ];
    const PNG_COVER: &[u8] = b"\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR";
    const WEBP_COVER: &[u8] = b"RIFF\x24\x00\x00\x00WEBPVP8 ";

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum ArtworkMode {
        Cover,
        HeadOnly,
        NoHead,
        JsonContentType,
        MismatchedMagic,
        Oversized,
        Redirect,
        Denied,
        PngCover,
        WebpCover,
    }

    /// A fixture origin for the artwork route that records GET/HEAD counts, so a test can
    /// prove a revalidated cache hit transfers no body.
    struct ArtworkServer {
        mode: Mutex<ArtworkMode>,
        gets: AtomicUsize,
        heads: AtomicUsize,
    }
    impl ArtworkServer {
        fn new(mode: ArtworkMode) -> Arc<Self> {
            Arc::new(Self {
                mode: Mutex::new(mode),
                gets: AtomicUsize::new(0),
                heads: AtomicUsize::new(0),
            })
        }
        fn set_mode(&self, mode: ArtworkMode) {
            *self.mode.lock().unwrap() = mode;
            self.gets.store(0, Ordering::SeqCst);
            self.heads.store(0, Ordering::SeqCst);
        }
        fn router(self: &Arc<Self>) -> Router {
            let server = self.clone();
            with_login(Router::new().route(
                "/api/artwork/{id}",
                get(
                    move |method: axum::http::Method,
                          Path(id): Path<String>,
                          headers: HeaderMap| {
                        let server = server.clone();
                        async move { server.answer(method, &id, &headers) }
                    },
                ),
            ))
        }
        fn answer(
            &self,
            method: axum::http::Method,
            id: &str,
            headers: &HeaderMap,
        ) -> HttpResponse<Body> {
            // The route only ever accepts the current in-memory session and a raster
            // Accept; no test request may carry credentials anywhere else.
            if headers
                .get(header::COOKIE)
                .and_then(|value| value.to_str().ok())
                != Some(format!("vela_session={}", "a".repeat(64)).as_str())
            {
                return status_response(StatusCode::UNAUTHORIZED);
            }
            let accept = headers
                .get(header::ACCEPT)
                .and_then(|value| value.to_str().ok())
                .unwrap_or_default();
            if !accept.contains("image/jpeg") || accept.contains("application/json") {
                return status_response(StatusCode::NOT_ACCEPTABLE);
            }
            if !canonical_uuid(id) {
                return status_response(StatusCode::NOT_FOUND);
            }
            let mode = *self.mode.lock().unwrap();
            let head = method == axum::http::Method::HEAD;
            if head {
                self.heads.fetch_add(1, Ordering::SeqCst);
            } else {
                self.gets.fetch_add(1, Ordering::SeqCst);
            }
            match mode {
                ArtworkMode::Cover => cover_response(StatusCode::OK, "image/jpeg", COVER, head),
                // A cache hit must be served by the revalidating HEAD alone.
                ArtworkMode::HeadOnly if head => {
                    cover_response(StatusCode::OK, "image/jpeg", COVER, true)
                }
                ArtworkMode::HeadOnly => status_response(StatusCode::INTERNAL_SERVER_ERROR),
                ArtworkMode::NoHead if head => status_response(StatusCode::METHOD_NOT_ALLOWED),
                ArtworkMode::NoHead => cover_response(StatusCode::OK, "image/jpeg", COVER, false),
                ArtworkMode::PngCover => {
                    cover_response(StatusCode::OK, "image/png", PNG_COVER, head)
                }
                ArtworkMode::WebpCover => {
                    cover_response(StatusCode::OK, "image/webp", WEBP_COVER, head)
                }
                ArtworkMode::JsonContentType => {
                    let body = if head {
                        Body::empty()
                    } else {
                        Body::from(COVER.to_vec())
                    };
                    mime_response(StatusCode::OK, "application/json", body)
                }
                ArtworkMode::MismatchedMagic => {
                    let mut bytes = b"\x89PNG\r\n\x1a\n".to_vec();
                    bytes.extend_from_slice(COVER);
                    cover_response(StatusCode::OK, "image/jpeg", &bytes, head)
                }
                ArtworkMode::Oversized => {
                    let mut bytes = COVER.to_vec();
                    bytes.resize(MAX_ARTWORK_BYTES as usize + 1, 0);
                    cover_response(StatusCode::OK, "image/jpeg", &bytes, head)
                }
                ArtworkMode::Redirect => HttpResponse::builder()
                    .status(StatusCode::TEMPORARY_REDIRECT)
                    .header(header::LOCATION, "/api/login")
                    .body(Body::empty())
                    .unwrap(),
                ArtworkMode::Denied => status_response(StatusCode::NOT_FOUND),
            }
        }
    }
    fn status_response(status: StatusCode) -> HttpResponse<Body> {
        HttpResponse::builder()
            .status(status)
            .body(Body::empty())
            .unwrap()
    }
    fn mime_response(status: StatusCode, mime: &'static str, body: Body) -> HttpResponse<Body> {
        HttpResponse::builder()
            .status(status)
            .header(header::CONTENT_TYPE, mime)
            .body(body)
            .unwrap()
    }
    fn cover_response(
        status: StatusCode,
        mime: &'static str,
        body: &[u8],
        head: bool,
    ) -> HttpResponse<Body> {
        let body = if head {
            Body::empty()
        } else {
            Body::from(body.to_vec())
        };
        mime_response(status, mime, body)
    }
    fn artwork_key(origin: &str, account: &str, artwork: &str) -> artwork_cache::Key {
        artwork_cache::Key::new(canonical_origin(origin).unwrap().as_str(), account, artwork)
            .unwrap()
    }

    #[tokio::test]
    async fn authorized_artwork_is_bounded_typed_private_and_cached() {
        let server = ArtworkServer::new(ArtworkMode::Cover);
        let http = HttpFixture::start(server.router()).await;
        let directory = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();
        let client = fixture_client(&http, &directory, &cache).await;

        let art = client
            .artwork(ARTWORK_ID, CancellationToken::new())
            .await
            .unwrap();
        assert_eq!(art.mime(), ArtworkMime::Jpeg);
        assert_eq!(art.bytes(), COVER);
        assert!(art.len() <= MAX_ARTWORK_BYTES as usize);
        let data_url = art.data_url();
        assert!(data_url.starts_with("data:image/jpeg;base64,"));
        assert!(!data_url.contains("vela_session"));
        assert!(!data_url.contains(&http.origin));
        {
            use base64::Engine as _;
            let encoded = data_url
                .strip_prefix("data:image/jpeg;base64,")
                .expect("a cover data URL carries its media type");
            assert_eq!(
                base64::engine::general_purpose::STANDARD
                    .decode(encoded)
                    .unwrap(),
                COVER,
                "the renderer receives exactly the authorized image bytes"
            );
        }
        assert_eq!(server.heads.load(Ordering::SeqCst), 1);
        assert_eq!(server.gets.load(Ordering::SeqCst), 1);

        // The private cache lives in this account/origin namespace only.
        let key = artwork_key(&http.origin, USER_ID, ARTWORK_ID);
        let file = artwork_cache::path(&cache.path().join("artwork"), &key, ArtworkMime::Jpeg);
        assert_eq!(std::fs::read(&file).unwrap(), COVER);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = |path: &std::path::Path| {
                std::fs::metadata(path).unwrap().permissions().mode() & 0o777
            };
            assert_eq!(mode(&file), 0o600);
            assert_eq!(mode(file.parent().unwrap()), 0o700);
            assert_eq!(
                mode(&artwork_cache::origin_root(
                    &cache.path().join("artwork"),
                    &key
                )),
                0o700
            );
        }
        let foreign_account = artwork_key(&http.origin, OTHER_USER_ID, ARTWORK_ID);
        assert_ne!(
            artwork_cache::path(&cache.path().join("artwork"), &key, ArtworkMime::Jpeg),
            artwork_cache::path(
                &cache.path().join("artwork"),
                &foreign_account,
                ArtworkMime::Jpeg
            )
        );
        assert!(
            artwork_cache::read(
                &cache.path().join("artwork"),
                &foreign_account,
                ArtworkMime::Jpeg,
                &|| false
            )
            .unwrap()
            .is_none(),
            "another account can never read this cover"
        );

        // Arbitrary paths, non-canonical ids and uppercase UUIDs never reach a request.
        let non_canonical = "30000000-0000-4000-8000-00000000000a".to_ascii_uppercase();
        for invalid in ["../private", "not-a-uuid", non_canonical.as_str()] {
            assert_eq!(
                client
                    .artwork(invalid, CancellationToken::new())
                    .await
                    .unwrap_err(),
                ArtworkError::Failed
            );
        }
        assert_eq!(server.heads.load(Ordering::SeqCst), 1);
        assert_eq!(server.gets.load(Ordering::SeqCst), 1);
        assert!(!ArtworkError::Denied.retryable());
        assert!(!ArtworkError::Canceled.retryable());
        assert!(ArtworkError::Failed.retryable());

        // A canceled request is canceled, not denied and never served from cache.
        let canceled = CancellationToken::new();
        canceled.cancel();
        assert_eq!(
            client.artwork(ARTWORK_ID, canceled).await.unwrap_err(),
            ArtworkError::Canceled
        );
    }

    #[tokio::test]
    async fn cached_artwork_is_revalidated_and_denial_never_returns_bytes() {
        let server = ArtworkServer::new(ArtworkMode::Cover);
        let http = HttpFixture::start(server.router()).await;
        let directory = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();
        let client = fixture_client(&http, &directory, &cache).await;

        let first = client
            .artwork(ARTWORK_ID, CancellationToken::new())
            .await
            .unwrap();
        assert_eq!(server.gets.load(Ordering::SeqCst), 1);

        // (b) The origin re-authorizes with HEAD; the cached body is reused without a GET.
        server.set_mode(ArtworkMode::HeadOnly);
        let cached = client
            .artwork(ARTWORK_ID, CancellationToken::new())
            .await
            .unwrap();
        assert_eq!(cached.bytes(), first.bytes());
        assert_eq!(server.gets.load(Ordering::SeqCst), 0);
        assert_eq!(server.heads.load(Ordering::SeqCst), 1);

        // (c) A denial is never answered from the disk cache.
        server.set_mode(ArtworkMode::Denied);
        assert_eq!(
            client
                .artwork(ARTWORK_ID, CancellationToken::new())
                .await
                .unwrap_err(),
            ArtworkError::Denied
        );
        assert_eq!(server.gets.load(Ordering::SeqCst), 0);
        assert!(
            artwork_cache::path(
                &cache.path().join("artwork"),
                &artwork_key(&http.origin, USER_ID, ARTWORK_ID),
                ArtworkMime::Jpeg
            )
            .exists(),
            "the denied cover stays cache-only and is never returned"
        );

        // (d) An origin that cannot answer HEAD still gets the authorized GET.
        server.set_mode(ArtworkMode::NoHead);
        let fresh = client
            .artwork(ARTWORK_ID, CancellationToken::new())
            .await
            .unwrap();
        assert_eq!(fresh.bytes(), COVER);
        assert_eq!(server.gets.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn artwork_rejects_wrong_type_bad_magic_redirect_and_oversize() {
        for (mode, expected) in [
            (ArtworkMode::JsonContentType, ArtworkError::Failed),
            (ArtworkMode::MismatchedMagic, ArtworkError::Failed),
            (ArtworkMode::Redirect, ArtworkError::Failed),
            (ArtworkMode::Denied, ArtworkError::Denied),
        ] {
            let server = ArtworkServer::new(mode);
            let http = HttpFixture::start(server.router()).await;
            let directory = tempfile::tempdir().unwrap();
            let cache = tempfile::tempdir().unwrap();
            let client = fixture_client(&http, &directory, &cache).await;
            assert_eq!(
                client
                    .artwork(OTHER_ARTWORK_ID, CancellationToken::new())
                    .await
                    .unwrap_err(),
                expected,
                "{mode:?}"
            );
            let published = std::fs::read_dir(cache.path().join("artwork"))
                .map(|entries| entries.count())
                .unwrap_or(0);
            assert_eq!(published, 0, "{mode:?} must not publish any cache file");
        }

        // An endless or lying response trips the 8 MiB stream cap and is never cached.
        let server = ArtworkServer::new(ArtworkMode::Oversized);
        let http = HttpFixture::start(server.router()).await;
        let directory = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();
        let client = fixture_client(&http, &directory, &cache).await;
        assert_eq!(
            client
                .artwork(OTHER_ARTWORK_ID, CancellationToken::new())
                .await
                .unwrap_err(),
            ArtworkError::Failed
        );
        assert!(
            !artwork_cache::path(
                &cache.path().join("artwork"),
                &artwork_key(&http.origin, USER_ID, OTHER_ARTWORK_ID),
                ArtworkMime::Jpeg
            )
            .exists()
        );
    }

    #[tokio::test]
    async fn artwork_allowlist_accepts_png_and_webp_with_matching_magic() {
        for (mode, mime, bytes) in [
            (ArtworkMode::PngCover, ArtworkMime::Png, PNG_COVER),
            (ArtworkMode::WebpCover, ArtworkMime::Webp, WEBP_COVER),
        ] {
            let server = ArtworkServer::new(mode);
            let http = HttpFixture::start(server.router()).await;
            let directory = tempfile::tempdir().unwrap();
            let cache = tempfile::tempdir().unwrap();
            let client = fixture_client(&http, &directory, &cache).await;
            let art = client
                .artwork(OTHER_ARTWORK_ID, CancellationToken::new())
                .await
                .unwrap();
            assert_eq!(art.mime(), mime, "{mode:?}");
            assert_eq!(art.bytes(), bytes, "{mode:?}");
            assert!(
                art.data_url()
                    .starts_with(&format!("data:{};base64,", mime.content_type())),
                "{mode:?}"
            );
            let key = artwork_key(&http.origin, USER_ID, OTHER_ARTWORK_ID);
            let file = artwork_cache::path(&cache.path().join("artwork"), &key, mime);
            assert_eq!(std::fs::read(&file).unwrap(), bytes, "{mode:?}");
            assert_eq!(file.extension().unwrap(), mime.extension(), "{mode:?}");
        }
    }

    #[tokio::test]
    async fn real_artwork_route_is_owner_authorized_and_native_cached() {
        let node = Node::new().await;
        let (mut client, auth) = node.connect().await;
        let cache = tempfile::tempdir().unwrap();
        Arc::get_mut(&mut client.inner).unwrap().cache_directory = Some(cache.path().to_owned());
        let artwork_id = node
            .own
            .artwork_id
            .clone()
            .expect("the committed FLAC fixture embeds cover art");
        let art = client
            .artwork(&artwork_id, CancellationToken::new())
            .await
            .unwrap();
        assert_eq!(art.mime(), ArtworkMime::Jpeg);
        assert!(art.len() > 3 && art.len() <= MAX_ARTWORK_BYTES as usize);
        assert_eq!(&art.bytes()[..3], &[0xFF, 0xD8, 0xFF]);

        let key = artwork_key(&node.http.origin, &auth.user.id, &artwork_id);
        let file = artwork_cache::path(&cache.path().join("artwork"), &key, ArtworkMime::Jpeg);
        assert_eq!(std::fs::read(&file).unwrap(), art.bytes());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(&file).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }

        // The same cover is served by a revalidated cache hit.
        let again = client
            .artwork(&artwork_id, CancellationToken::new())
            .await
            .unwrap();
        assert_eq!(again.bytes(), art.bytes());

        // An unknown/foreign artwork id is a denial, not an error page or cached bytes.
        let missing = uuid::Uuid::new_v4().to_string();
        assert_eq!(
            client
                .artwork(&missing, CancellationToken::new())
                .await
                .unwrap_err(),
            ArtworkError::Denied
        );
    }
}
