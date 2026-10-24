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
/// Background recency marks that may run at the same time. A mark is one small
/// sidecar rewrite of a few hundred bytes, and a confirmed hit produces one mark
/// at a time: four of them are already more than a playback can produce. The
/// bound keeps the blocking pool of the runtime free for the transfers and the
/// publications the first sound depends on. A mark that cannot take a slot is
/// dropped, never queued: the mark is an optimization, and the lease must never
/// wait for it.
const MAX_CONCURRENT_MARKS: usize = 4;
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
/// The single-flight slot of one running cache publication.
///
/// A second prepare of the same key waits for the slot instead of starting a
/// second full transfer. The slot is registered in the registry before the
/// publication starts and released when it ends, so the registry counts live
/// publications and not dictionary entries.
struct Publication {
    /// Flips once the publication ended. The cache entry was published (or the
    /// copy failed) before that, so a waiter that observes the end knows the
    /// state of the cache.
    ended: watch::Sender<bool>,
}
impl Publication {
    fn new() -> Arc<Self> {
        let (ended, _) = watch::channel(false);
        Arc::new(Self { ended })
    }
    /// True once the publication ended.
    fn ended(&self) -> bool {
        *self.ended.borrow()
    }
    /// Mark the publication as ended.
    fn finish(&self) {
        self.ended.send_replace(true);
    }
}

/// Releases the single-flight slot of a publication when it ends, even when the
/// publication task is aborted.
struct PublicationRun {
    inner: Weak<Inner>,
    key: media_cache::CacheKey,
    publication: Arc<Publication>,
}
impl Drop for PublicationRun {
    fn drop(&mut self) {
        // The publication ended: the cache entry was published (or the copy
        // failed) before this point, so a waiter that observes the end can
        // trust the state of the cache.
        self.publication.finish();
        let Some(inner) = self.inner.upgrade() else {
            return;
        };
        let Ok(mut registry) = inner.publications.lock() else {
            return;
        };
        // Only this publication's own entry is removed: a newer publication of
        // the same key must never be dropped.
        if registry
            .get(&self.key)
            .is_some_and(|entry| Weak::ptr_eq(entry, &Arc::downgrade(&self.publication)))
        {
            registry.remove(&self.key);
        }
    }
}
struct Inner {
    http: Client,
    origin: Url,
    /// The authenticated account this client's catalog, queue and cache belong to.
    user_id: String,
    session: Mutex<Option<Session>>,
    token: String,
    lifetime: CancellationToken,
    preparing: Arc<Semaphore>,
    /// Cache publications that are still running, keyed by the cache key they
    /// publish. A second prepare of the same key waits for the running
    /// publication instead of starting a second full transfer, so one body per
    /// track is transferred once. The entry is a weak reference to the
    /// publication's single-flight slot: it disappears with the publication it
    /// describes.
    publications: Mutex<HashMap<media_cache::CacheKey, Weak<Publication>>>,
    /// The bounded wait for a running publication of the same key. Production
    /// always uses `PUBLICATION_WAIT`; the tests shorten it to keep the suite
    /// fast.
    publication_wait: Duration,
    /// The background recency marks that may run at the same time. A mark that
    /// cannot take a slot is dropped: the mark is an optimization and the
    /// prepared lease must never wait for it.
    marks: Arc<Semaphore>,
    #[cfg(test)]
    temporary_directory: Option<PathBuf>,
    #[cfg(test)]
    cache_directory: Option<PathBuf>,
    #[cfg(test)]
    publication_pause: Mutex<Option<Arc<PublicationPause>>>,
    /// The gate of the background recency mark, so a test can hold a mark
    /// that a confirmed cache hit scheduled.
    #[cfg(test)]
    recency_pause: Mutex<Option<Arc<RecencyPause>>>,
    /// How many recency marks were dropped because the bound of concurrent
    /// marks was reached. A test-only counter: a dropped mark is a lost
    /// optimization, not a user-visible event.
    #[cfg(test)]
    dropped_marks: AtomicUsize,
    #[cfg(test)]
    cache_miss_barrier: Mutex<Option<Arc<CacheMissBarrier>>>,
    /// The barrier of the post-admission cache read. It is a separate instance
    /// from the cold-cache barrier, so a test can hold the first read and the
    /// post-admission read of the same prepare independently.
    #[cfg(test)]
    recheck_barrier: Mutex<Option<Arc<CacheMissBarrier>>>,
    /// The barrier of the *second* post-admission read of the same path: the
    /// one that runs after a publication of the key was seen alive and then
    /// ended. It is separate from the miss barrier, so a test can hold that
    /// read and watch what the transfer admission does while it runs.
    #[cfg(test)]
    ended_read_barrier: Mutex<Option<Arc<CacheMissBarrier>>>,
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
                    publication_wait: PUBLICATION_WAIT,
                    marks: Arc::new(Semaphore::new(MAX_CONCURRENT_MARKS)),
                    #[cfg(test)]
                    temporary_directory: None,
                    #[cfg(test)]
                    cache_directory: None,
                    #[cfg(test)]
                    publication_pause: Mutex::new(None),
                    #[cfg(test)]
                    recency_pause: Mutex::new(None),
                    #[cfg(test)]
                    dropped_marks: AtomicUsize::new(0),
                    #[cfg(test)]
                    cache_miss_barrier: Mutex::new(None),
                    #[cfg(test)]
                    recheck_barrier: Mutex::new(None),
                    #[cfg(test)]
                    ended_read_barrier: Mutex::new(None),
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
                    publication_wait: PUBLICATION_WAIT,
                    marks: Arc::new(Semaphore::new(MAX_CONCURRENT_MARKS)),
                    #[cfg(test)]
                    temporary_directory: None,
                    #[cfg(test)]
                    cache_directory: None,
                    #[cfg(test)]
                    publication_pause: Mutex::new(None),
                    #[cfg(test)]
                    recency_pause: Mutex::new(None),
                    #[cfg(test)]
                    dropped_marks: AtomicUsize::new(0),
                    #[cfg(test)]
                    cache_miss_barrier: Mutex::new(None),
                    #[cfg(test)]
                    recheck_barrier: Mutex::new(None),
                    #[cfg(test)]
                    ended_read_barrier: Mutex::new(None),
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
                // At most one wait for a running publication runs per prepare:
                // a publication that does not end in time must not turn into a
                // second body per track, and the ordinary verified transfer
                // decides after that.
                let mut waited = false;
                if let (Some(scope), Some(etag)) = (&scope, &validator) {
                    // (b) The reusable cache of this account on this origin. A
                    // running publication of the same key is waited for instead
                    // of a second full transfer, and that wait never holds the
                    // transfer admission: a pending publication of one track can
                    // never block the play of another track.
                    if let Some(cached) = self
                        .cached_lease(scope, expected, etag, &cancel, &mut waited)
                        .await?
                    {
                        progress.send_replace(DownloadProgress {
                            received_bytes: expected,
                            total_bytes: expected,
                        });
                        return Ok(cached);
                    }
                }
                // The transfer admission covers the transfer itself only. It is
                // taken after the wait above, so a pending publication of one
                // track can never block the play of another track.
                let mut admission = Some(self.admit().await?);
                if let (Some(scope), Some(etag)) = (&scope, &validator) {
                    // The admission is not a promise about the world: the cache
                    // and the running publication are re-checked, not assumed.
                    if let Some(cached) = self
                        .recheck_cache(scope, expected, etag, &cancel, &mut admission, &mut waited)
                        .await?
                    {
                        progress.send_replace(DownloadProgress {
                            received_bytes: expected,
                            total_bytes: expected,
                        });
                        return Ok(cached);
                    }
                }
                // The transfer itself always runs under the admission. A wait
                // above may have released it, and the transfer takes it again.
                let permit = match admission {
                    Some(permit) => permit,
                    None => self.admit().await?,
                };
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
                if let Some(scope) = &scope {
                    match self
                        .publish(scope, prepared.clone(), verified_hash.clone())
                        .await
                    {
                        // The copy runs in the background.
                        PublicationStart::Started(_) => {}
                        // A live publication of the same key already publishes the
                        // very same bytes: a second copy would only duplicate the
                        // work, and the running publication keeps the single-flight
                        // slot of the key.
                        PublicationStart::Joined => {}
                        // The registry of running publications is full: keep the
                        // previous synchronous publication instead of silently
                        // skipping the warm-up.
                        PublicationStart::Refused => {
                            scope.store(&prepared, &verified_hash, &cancel).await;
                        }
                    }
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
    /// The registry holds a weak reference to the publication's single-flight
    /// slot, so an entry disappears as soon as its publication ended. A slot
    /// that already ended is not a running publication: its bytes are in the
    /// cache (or the copy failed), and the cache answers for the key.
    fn in_flight_publication(&self, key: &media_cache::CacheKey) -> Option<Arc<Publication>> {
        let registry = self.inner.publications.lock().ok()?;
        let publication = registry.get(key)?.upgrade()?;
        if publication.ended() {
            return None;
        }
        Some(publication)
    }

    /// Register a publication that is about to run.
    ///
    /// A live publication of the same key is never replaced: the new
    /// publication joins it, because a second publication would copy the very
    /// same bytes. The registry is bounded by the number of *live* publications
    /// and not by the number of entries: an entry of a publication that ended
    /// counts for nothing.
    fn register_publication(&self, key: &media_cache::CacheKey) -> PublicationStart {
        let Ok(mut registry) = self.inner.publications.lock() else {
            return PublicationStart::Refused;
        };
        // A publication that already ended is not in flight any more.
        registry.retain(|_, publication| {
            publication
                .upgrade()
                .is_some_and(|publication| !publication.ended())
        });
        if registry.contains_key(key) {
            // A live publication of this key exists: it is never replaced.
            return PublicationStart::Joined;
        }
        if registry.len() >= MAX_IN_FLIGHT_PUBLICATIONS {
            return PublicationStart::Refused;
        }
        let publication = Publication::new();
        registry.insert(key.clone(), Arc::downgrade(&publication));
        PublicationStart::Started(publication)
    }

    /// Wait for a running publication of the same key to end.
    ///
    /// The wait is bounded and honors the caller's cancellation and the client's
    /// lifetime. `Ok(true)` means the publication ended and the cache may be
    /// re-checked; `Ok(false)` means the wait ran out and the caller continues with
    /// an ordinary verified transfer.
    async fn wait_for_publication(
        &self,
        publication: &Arc<Publication>,
        cancel: &CancellationToken,
    ) -> Result<bool, String> {
        let mut ended = publication.ended.subscribe();
        tokio::select! {
            biased;
            _ = cancel.cancelled() => Err(CANCELED.into()),
            _ = self.inner.lifetime.cancelled() => Err(SIGNED_OUT.into()),
            result = tokio::time::timeout(self.inner.publication_wait, async {
                loop {
                    if *ended.borrow_and_update() {
                        break;
                    }
                    if ended.changed().await.is_err() {
                        break;
                    }
                }
            }) => {
                match result {
                    // The publication is over: its bytes are in the cache (or the
                    // copy failed) and the next publication of this key may run.
                    Ok(()) => Ok(true),
                    Err(_elapsed) => Ok(false),
                }
            },
        }
    }

    /// A verified cache hit, and the recency mark it implies.
    ///
    /// The mark is scheduled in the background: the path to the first sound
    /// never waits for it.
    async fn hit(
        &self,
        scope: &CacheScope,
        expected: u64,
        etag: &str,
        cancel: &CancellationToken,
    ) -> Result<Option<Arc<PreparedAudio>>, String> {
        let Some(cached) = scope.open_verified(expected, etag, cancel).await? else {
            return Ok(None);
        };
        self.mark_recency(scope, expected, etag);
        Ok(Some(cached))
    }

    /// Mark a confirmed cache hit as recently played.
    ///
    /// The mark is an optimization and never a condition of playback: it runs
    /// in the background, its result is discarded, and it can neither fail nor
    /// delay the lease.
    ///
    /// It is bound to the client's `lifetime` token — a sign-out stops it —
    /// and deliberately not to the caller's per-selection cancel token: the UI
    /// cancels that token as soon as another track is selected, and the track
    /// that just played must keep its place in the cache.
    ///
    /// A live publication of the same key is never marked: the publication
    /// writes a fresh `saved_at` of its own.
    ///
    /// The number of marks that run at the same time is bounded. The slot is
    /// taken **before** the task starts and without waiting: a mark that cannot
    /// take a slot is dropped, because the mark is an optimization and a
    /// dropped mark costs one recency label, while a queued mark would delay
    /// the next transfer on the blocking pool. The slot lives as long as the
    /// task, so it is released when the task ends, is aborted or panics.
    fn mark_recency(&self, scope: &CacheScope, expected: u64, etag: &str) {
        if self.in_flight_publication(&scope.key).is_some() {
            return;
        }
        let Ok(permit) = Arc::clone(&self.inner.marks).try_acquire_owned() else {
            // The bound is reached: the mark is dropped, and the lease that is
            // already on its way to the decoder is not delayed by it.
            #[cfg(test)]
            self.inner.dropped_marks.fetch_add(1, Ordering::SeqCst);
            return;
        };
        let root = scope.root.clone();
        let key = scope.key.clone();
        let etag = etag.to_owned();
        let lifetime = self.inner.lifetime.clone();
        #[cfg(test)]
        let pause = self
            .inner
            .recency_pause
            .lock()
            .ok()
            .and_then(|slot| slot.clone());
        tokio::spawn(async move {
            // The slot is held for the whole run of the task and is released
            // with it, also when the task is aborted or panics.
            let _permit = permit;
            #[cfg(test)]
            if let Some(pause) = &pause {
                pause.enter().await;
            }
            let _ = tokio::task::spawn_blocking(move || {
                media_cache::touch(&root, &key, expected, &etag, &|| lifetime.is_cancelled())
            })
            .await;
            #[cfg(test)]
            if let Some(pause) = &pause {
                pause.finished();
            }
        });
    }

    /// The verified lease of a warm cache entry for this key, or `None` when the
    /// cache is cold and the caller has to do an ordinary verified transfer.
    ///
    /// The publication of a key is single-flight: while a publication of this
    /// exact key runs, this waits for it instead of letting the caller start a
    /// second full transfer, and the cache is read again afterwards. The
    /// single-flight slot is observed *before* the cache is read, so a
    /// publication that ends between the two is never mistaken for a cold
    /// cache: the cache is read again instead of a second body being
    /// transferred.
    ///
    /// The wait is bounded, honors the caller's cancellation and the client's
    /// lifetime, and never holds the transfer admission: a pending publication
    /// of one track can never block the play of another track.
    async fn cached_lease(
        &self,
        scope: &CacheScope,
        expected: u64,
        etag: &str,
        cancel: &CancellationToken,
        waited: &mut bool,
    ) -> Result<Option<Arc<PreparedAudio>>, String> {
        loop {
            let publication = self.in_flight_publication(&scope.key);
            // A hit re-hashes its own bytes from the very descriptor the decoder
            // receives, so a hit is a verified complete original.
            if let Some(cached) = self.hit(scope, expected, etag, cancel).await? {
                return Ok(Some(cached));
            }
            #[cfg(test)]
            self.reach_cache_miss().await;
            let Some(publication) = publication else {
                // Nothing publishes this key and the cache is cold: the caller
                // does the ordinary verified transfer.
                return Ok(None);
            };
            if *waited {
                // One wait already ran out for this key: the caller does the
                // ordinary verified transfer. The running publication keeps the
                // single-flight slot of the key; it is never replaced.
                return Ok(None);
            }
            *waited = true;
            if self.wait_for_publication(&publication, cancel).await? {
                // The publication ended: its bytes are in the cache now, and the
                // cache is read again at the top of the loop.
                continue;
            }
            // The wait ran out: the caller does the ordinary verified transfer.
            return Ok(None);
        }
    }

    /// Re-check the reusable cache with the transfer admission held.
    ///
    /// An admission is not a promise about the world: the cache is read again,
    /// and a publication of the same key that runs now is waited for instead of
    /// a second full transfer. That wait never holds the admission, so a
    /// pending publication of one track can never block the play of another
    /// track: the admission is released for the wait and taken again for the
    /// transfer.
    ///
    /// The single-flight slot is observed *before* the cache is read, exactly
    /// as in [`Self::cached_lease`]. A publication that ends between that
    /// observation and the read is never mistaken for a cold cache: the slot
    /// carries an explicit *ended* flag, so an ended publication is answered
    /// with a second read of the cache instead of a second body.
    async fn recheck_cache(
        &self,
        scope: &CacheScope,
        expected: u64,
        etag: &str,
        cancel: &CancellationToken,
        admission: &mut Option<OwnedSemaphorePermit>,
        waited: &mut bool,
    ) -> Result<Option<Arc<PreparedAudio>>, String> {
        // The publication slot is observed before the cache is read: a
        // publication that ends while the cache is read is still held by this
        // handle, so the miss below is answered with a bounded wait and a
        // second read, never with a second body.
        let publication = self.in_flight_publication(&scope.key);
        // A hit re-hashes its own bytes from the very descriptor the decoder
        // receives, so a hit is a verified complete original.
        if let Some(cached) = self.hit(scope, expected, etag, cancel).await? {
            return Ok(Some(cached));
        }
        #[cfg(test)]
        self.reach_recheck_miss().await;
        // No publication of this key was running when the cache was read: the
        // cache is cold for real and the ordinary verified transfer decides.
        let Some(publication) = publication else {
            return Ok(None);
        };
        // The slot was live when the read started and has ended since: its
        // bytes are in the cache now, whatever the earlier wait did. This is
        // the case an expired wait reaches — the wait ran out on a publication
        // that finished inside this read, so the cache must be read again
        // before a second body is transferred.
        if publication.ended() {
            // A warm hit must never be answered with `PREPARE_BUSY`: this read
            // re-hashes the whole published original (up to 250 MiB), so it is
            // released from the transfer admission exactly like the wait below.
            // The transfer takes the admission again if the re-read misses.
            *admission = None;
            #[cfg(test)]
            self.reach_ended_read().await;
            return self.hit(scope, expected, etag, cancel).await;
        }
        if *waited {
            // One wait already ran out for a publication that is *still*
            // running: the ordinary verified transfer below decides. The
            // running publication keeps the single-flight slot of the key; it
            // is never replaced.
            return Ok(None);
        }
        *waited = true;
        // The wait does not hold the admission.
        *admission = None;
        if !self.wait_for_publication(&publication, cancel).await? {
            // The wait ran out: the ordinary verified transfer below decides. The
            // running publication keeps the single-flight slot of the key; it is
            // never replaced.
            return Ok(None);
        }
        // The publication ended: its bytes are in the cache now, so the cache
        // is read again instead of a second body being transferred.
        self.hit(scope, expected, etag, cancel).await
    }

    /// Take the transfer admission.
    ///
    /// The admission covers the transfer itself only. A canceled prior writer
    /// retains it through its actual cleanup, so normal rapid selections get a
    /// short, cancellable window.
    async fn admit(&self) -> Result<OwnedSemaphorePermit, String> {
        tokio::time::timeout(
            Duration::from_secs(1),
            self.inner.preparing.clone().acquire_owned(),
        )
        .await
        .map_err(|_| PREPARE_BUSY.to_owned())?
        .map_err(|_| PREPARE_BUSY.to_owned())
    }

    /// Hold a cold cache right before the single-flight decision, so a test can
    /// end a running publication exactly between the cache miss and the
    /// decision.
    #[cfg(test)]
    async fn reach_cache_miss(&self) {
        let barrier = self
            .inner
            .cache_miss_barrier
            .lock()
            .ok()
            .and_then(|slot| slot.clone());
        if let Some(barrier) = barrier {
            barrier.reach().await;
        }
    }

    /// Hold the post-admission cache read right after it missed, so a test can
    /// end a running publication exactly between that read and the
    /// single-flight decision.
    #[cfg(test)]
    async fn reach_recheck_miss(&self) {
        let barrier = self
            .inner
            .recheck_barrier
            .lock()
            .ok()
            .and_then(|slot| slot.clone());
        if let Some(barrier) = barrier {
            barrier.reach().await;
        }
    }

    /// Hold the second post-admission cache read — the one that runs after a
    /// publication of the key was seen alive and has ended since — so a test can
    /// assert what the transfer admission does while that read runs.
    #[cfg(test)]
    async fn reach_ended_read(&self) {
        let barrier = self
            .inner
            .ended_read_barrier
            .lock()
            .ok()
            .and_then(|slot| slot.clone());
        if let Some(barrier) = barrier {
            barrier.reach().await;
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
    /// A live publication of the same key is never replaced: the caller joins
    /// it, because a second publication would copy the very same bytes. When
    /// the registry of running publications is full the publication is refused
    /// and the caller keeps the previous synchronous publication.
    async fn publish(
        &self,
        scope: &CacheScope,
        prepared: Arc<PreparedAudio>,
        etag: String,
    ) -> PublicationStart {
        let start = self.register_publication(&scope.key);
        let PublicationStart::Started(publication) = start else {
            return start;
        };
        // The caller only needs to know which way the warm-up went; the slot
        // itself belongs to the publication task.
        let outcome = PublicationStart::Started(publication.clone());
        let root = scope.root.clone();
        let key = scope.key.clone();
        let lifetime = self.inner.lifetime.clone();
        let inner = Arc::downgrade(&self.inner);
        let lease = prepared.clone();
        #[cfg(test)]
        let pause = self
            .inner
            .publication_pause
            .lock()
            .ok()
            .and_then(|slot| slot.clone());
        tokio::spawn(async move {
            // The single-flight slot is held for the whole publication, so a
            // second prepare of the same key waits for it instead of starting a
            // second transfer. It is released when the task ends, even when the
            // task is aborted.
            let _run = PublicationRun {
                inner,
                key: key.clone(),
                publication,
            };
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
        outcome
    }
}

/// How the warm-up of one verified original was placed.
enum PublicationStart {
    /// The publication owns the single-flight slot of its key and copies the
    /// verified bytes in the background.
    Started(Arc<Publication>),
    /// A live publication of the same key already publishes the very same
    /// bytes: a second copy would only duplicate the work, and the running
    /// publication keeps the single-flight slot of the key.
    Joined,
    /// The registry of running publications is full: the caller keeps the
    /// previous synchronous publication.
    Refused,
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
    /// Hold the next publication again, so a test can observe a second
    /// publication of the same key.
    fn arm(&self) {
        self.released.send_replace(false);
    }
}

/// A test-only gate that holds the background recency mark of a confirmed
/// cache hit, so a test can observe that the prepared lease is returned long
/// before the mark of the hit is written.
#[cfg(test)]
struct RecencyPause {
    entered: AtomicUsize,
    finished: AtomicUsize,
    released: watch::Sender<bool>,
}
#[cfg(test)]
impl RecencyPause {
    fn new() -> Arc<Self> {
        let (released, _) = watch::channel(false);
        Arc::new(Self {
            entered: AtomicUsize::new(0),
            finished: AtomicUsize::new(0),
            released,
        })
    }
    /// Hold the mark until the test releases it.
    async fn enter(&self) {
        self.entered.fetch_add(1, Ordering::SeqCst);
        let mut released = self.released.subscribe();
        while !*released.borrow_and_update() {
            if released.changed().await.is_err() {
                break;
            }
        }
    }
    /// Count a mark that ran to its end.
    fn finished(&self) {
        self.finished.fetch_add(1, Ordering::SeqCst);
    }
    /// Let the held mark continue.
    fn release(&self) {
        self.released.send_replace(true);
    }
}

/// A test-only barrier that holds a cold cache right before the single-flight
/// decision, so a test can end a running publication exactly between the cache
/// miss and that decision. The same barrier type holds the first cache read
/// and the post-admission cache read of a prepare; they are separate
/// instances, so a test can hold one read without holding the other.
#[cfg(test)]
struct CacheMissBarrier {
    /// How many cold caches this barrier still holds. `usize::MAX` holds every
    /// cold cache until the barrier is released, `1` holds exactly the next
    /// one.
    holds: AtomicUsize,
    reached: AtomicUsize,
    released: watch::Sender<bool>,
}
#[cfg(test)]
impl CacheMissBarrier {
    fn new() -> Arc<Self> {
        let (released, _) = watch::channel(true);
        Arc::new(Self {
            holds: AtomicUsize::new(0),
            reached: AtomicUsize::new(0),
            released,
        })
    }
    /// Arm the barrier: every cold cache holds until the test releases it.
    fn arm(&self) {
        self.holds.store(usize::MAX, Ordering::SeqCst);
        self.released.send_replace(false);
    }
    /// Arm the barrier for exactly the next cold cache: a later cold cache of
    /// the same prepare is not held.
    fn arm_next(&self) {
        self.holds.store(1, Ordering::SeqCst);
        self.released.send_replace(false);
    }
    /// Let the held cache continue.
    fn release(&self) {
        self.holds.store(0, Ordering::SeqCst);
        self.released.send_replace(true);
    }
    /// How many cold caches were held.
    fn reached(&self) -> usize {
        self.reached.load(Ordering::SeqCst)
    }
    /// Hold the cold cache until the test releases it.
    async fn reach(&self) {
        // A held cold cache consumes one of the holds of the barrier.
        let mut holds = self.holds.load(Ordering::SeqCst);
        loop {
            if holds == 0 {
                return;
            }
            if self
                .holds
                .compare_exchange(
                    holds,
                    holds.saturating_sub(1),
                    Ordering::SeqCst,
                    Ordering::SeqCst,
                )
                .is_ok()
            {
                break;
            }
            holds = self.holds.load(Ordering::SeqCst);
        }
        self.reached.fetch_add(1, Ordering::SeqCst);
        let mut released = self.released.subscribe();
        while !*released.borrow_and_update() {
            if released.changed().await.is_err() {
                break;
            }
        }
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
    const OTHER_TRACK_ID: &str = "20000000-0000-4000-8000-000000000002";
    /// A recency label of the past: an entry that was published long ago, so a
    /// confirmed hit is observed to move the label to the current clock
    /// without a sleep.
    const AGED_RECENCY: u64 = 1_000_000;

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
    /// How many staging files of this cache namespace are on disk right now.
    fn staging_files(account: &std::path::Path) -> usize {
        let Ok(entries) = std::fs::read_dir(account) else {
            return 0;
        };
        entries
            .filter_map(Result::ok)
            .filter(|entry| entry.file_name().to_string_lossy().ends_with(".tmp"))
            .count()
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

    /// A fixture client whose private temporary originals, reusable media cache
    /// and background recency gate are all isolated: a test can hold the
    /// recency mark of a confirmed cache hit and observe that the prepared
    /// lease does not wait for it.
    async fn fixture_client_with_recency_pause(
        http: &HttpFixture,
        directory: &tempfile::TempDir,
        cache: &tempfile::TempDir,
        pause: &Arc<RecencyPause>,
    ) -> VelaClient {
        let mut client = fixture_client(http, directory, cache).await;
        let inner = Arc::get_mut(&mut client.inner).unwrap();
        inner.recency_pause = std::sync::Mutex::new(Some(pause.clone()));
        client
    }

    /// The recency the sidecar of one cached entry carries.
    fn sidecar_saved_at(cache: &std::path::Path, key: &media_cache::CacheKey) -> u64 {
        let record: serde_json::Value =
            serde_json::from_slice(&std::fs::read(media_cache::sidecar_path(cache, key)).unwrap())
                .unwrap();
        record["saved_at"]
            .as_u64()
            .expect("the recency of a published entry")
    }

    /// Age the recency label of one published entry.
    ///
    /// The label of a confirmed hit is the wall clock of the moment of the
    /// mark, so a refresh inside the same second is not observable. An entry
    /// whose label was written in the past makes the refresh observable
    /// without a sleep, and the test still asserts the real contract: the
    /// label moves to the current clock and never to a value from the future.
    fn age_sidecar_recency(cache: &std::path::Path, key: &media_cache::CacheKey, value: u64) {
        let path = media_cache::sidecar_path(cache, key);
        let mut record: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        record["saved_at"] = serde_json::Value::from(value);
        std::fs::write(&path, serde_json::to_vec(&record).unwrap()).unwrap();
    }

    /// A fixture client with the background publication gate and the cold-cache
    /// barrier: a test can end a running publication exactly between the cache
    /// miss and the single-flight decision.
    async fn fixture_client_with_barrier(
        http: &HttpFixture,
        directory: &tempfile::TempDir,
        cache: &tempfile::TempDir,
        pause: &Arc<PublicationPause>,
        barrier: &Arc<CacheMissBarrier>,
    ) -> VelaClient {
        let mut client = fixture_client_with_pause(http, directory, cache, pause).await;
        let inner = Arc::get_mut(&mut client.inner).unwrap();
        inner.cache_miss_barrier = std::sync::Mutex::new(Some(barrier.clone()));
        client
    }

    /// A fixture client with the background publication gate, the cold-cache
    /// barrier and the separate barrier of the post-admission cache read: a
    /// test can end a running publication exactly between that read and the
    /// single-flight decision.
    async fn fixture_client_with_recheck_barrier(
        http: &HttpFixture,
        directory: &tempfile::TempDir,
        cache: &tempfile::TempDir,
        pause: &Arc<PublicationPause>,
        barrier: &Arc<CacheMissBarrier>,
        recheck: &Arc<CacheMissBarrier>,
    ) -> VelaClient {
        let mut client = fixture_client_with_barrier(http, directory, cache, pause, barrier).await;
        let inner = Arc::get_mut(&mut client.inner).unwrap();
        inner.recheck_barrier = std::sync::Mutex::new(Some(recheck.clone()));
        client
    }

    /// A fixture client that also holds the second post-admission read, the one
    /// answered after a publication of the key was seen alive and has ended.
    async fn fixture_client_with_ended_read_barrier(
        http: &HttpFixture,
        directory: &tempfile::TempDir,
        cache: &tempfile::TempDir,
        pause: &Arc<PublicationPause>,
        barrier: &Arc<CacheMissBarrier>,
        recheck: &Arc<CacheMissBarrier>,
        ended: &Arc<CacheMissBarrier>,
    ) -> VelaClient {
        let mut client =
            fixture_client_with_recheck_barrier(http, directory, cache, pause, barrier, recheck)
                .await;
        let inner = Arc::get_mut(&mut client.inner).unwrap();
        inner.ended_read_barrier = std::sync::Mutex::new(Some(ended.clone()));
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

    /// A confirmed hit refreshes the recency of the entry it played, and only
    /// that entry: the track that was never played again keeps the recency of
    /// its publication. The body of a track is still transferred exactly once.
    #[tokio::test]
    async fn a_confirmed_hit_refreshes_the_recency_of_the_entry_it_played() {
        let first_bytes = vec![51_u8; 16 * 1024];
        let second_bytes = vec![52_u8; 12 * 1024];
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
        let first_key = cache_key(
            &http.origin,
            USER_ID,
            "20000000-0000-4000-8000-0000000000a1",
        );
        let second_key = cache_key(
            &http.origin,
            USER_ID,
            "20000000-0000-4000-8000-0000000000b2",
        );

        // A and B are published; the recency of both is the one of their
        // publication.
        for (track, key) in [(&first, &first_key), (&second, &second_key)] {
            let prepared = client
                .prepare(track, CancellationToken::new(), progress().0)
                .await
                .unwrap();
            assert_eq!(
                std::fs::read(prepared.path()).unwrap().len(),
                track.size_bytes as usize
            );
            drop(prepared);
            await_publication(&client, key).await;
        }
        let first_recency = sidecar_saved_at(cache.path(), &first_key);
        let second_recency = sidecar_saved_at(cache.path(), &second_key);
        assert!(
            first_recency > AGED_RECENCY,
            "a publication writes the current clock as the recency of the entry"
        );
        // The entry of A is aged: the label of a confirmed hit is the current
        // clock, and a refresh inside the same second is not observable. An
        // entry whose label was written in the past makes the refresh
        // observable without a sleep.
        age_sidecar_recency(cache.path(), &first_key, AGED_RECENCY);

        // A again: the confirmed hit refreshes the recency of A alone.
        let warm = client
            .prepare(&first, CancellationToken::new(), progress().0)
            .await
            .unwrap();
        assert_eq!(std::fs::read(warm.path()).unwrap(), first_bytes);
        drop(warm);
        eventually(|| sidecar_saved_at(cache.path(), &first_key) > AGED_RECENCY).await;
        assert!(
            sidecar_saved_at(cache.path(), &first_key) <= media_cache::saved_at_now(),
            "the recency of a confirmed hit is the current clock, never a value from the future"
        );
        assert_eq!(
            sidecar_saved_at(cache.path(), &second_key),
            second_recency,
            "an entry that was not played again keeps the recency of its publication"
        );
        assert_eq!(
            first_counters.gets.load(Ordering::SeqCst),
            1,
            "A→B→A must not re-download A's body"
        );
        assert_eq!(second_counters.gets.load(Ordering::SeqCst), 1);
    }

    /// The recency mark of a confirmed hit is written in the background: the
    /// prepared lease is returned long before the mark is on disk.
    #[tokio::test]
    async fn a_confirmed_hit_is_marked_in_the_background_and_never_delays_the_lease() {
        let original = vec![61_u8; 16 * 1024];
        let counters = Arc::new(StreamCounters::default());
        let router = tracked_streams(&[(TRACK_ID.to_owned(), original.clone(), counters.clone())]);
        let http = HttpFixture::start(with_login(router)).await;
        let temporary = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();
        let pause = RecencyPause::new();
        let client = fixture_client_with_recency_pause(&http, &temporary, &cache, &pause).await;
        let track = sample_track_with(TRACK_ID, original.len());
        let key = cache_key(&http.origin, USER_ID, TRACK_ID);

        // Cold: the entry is published in the background.
        let prepared = client
            .prepare(&track, CancellationToken::new(), progress().0)
            .await
            .unwrap();
        assert_eq!(std::fs::read(prepared.path()).unwrap(), original);
        drop(prepared);
        await_publication(&client, &key).await;
        let published = sidecar_saved_at(cache.path(), &key);
        assert!(
            published > AGED_RECENCY,
            "a publication writes the current clock as the recency of the entry"
        );
        // The entry is aged, so the refresh of the recency is observable
        // without a sleep: the label of a confirmed hit is the current clock.
        age_sidecar_recency(cache.path(), &key, AGED_RECENCY);

        // Warm: the lease is returned while the mark of the hit is still held.
        let warm = client
            .prepare(&track, CancellationToken::new(), progress().0)
            .await
            .unwrap();
        assert_eq!(std::fs::read(warm.path()).unwrap(), original);
        eventually(|| pause.entered.load(Ordering::SeqCst) == 1).await;
        assert_eq!(
            pause.finished.load(Ordering::SeqCst),
            0,
            "the mark must still be running when the lease is returned"
        );
        assert_eq!(
            counters.gets.load(Ordering::SeqCst),
            1,
            "a warm hit transfers no body"
        );
        assert_eq!(
            sidecar_saved_at(cache.path(), &key),
            AGED_RECENCY,
            "the held mark has written nothing yet"
        );

        // Releasing the mark refreshes the recency of the entry.
        pause.release();
        eventually(|| pause.finished.load(Ordering::SeqCst) == 1).await;
        eventually(|| sidecar_saved_at(cache.path(), &key) > AGED_RECENCY).await;
        assert!(
            sidecar_saved_at(cache.path(), &key) <= media_cache::saved_at_now(),
            "the recency of a confirmed hit is the current clock, never a value from the future"
        );
    }

    /// A mark that cannot be applied changes nothing: the prepared lease stays
    /// the verified original and the cache keeps no staging file.
    #[tokio::test]
    async fn a_recency_mark_that_cannot_be_applied_does_not_affect_the_lease() {
        let original = vec![62_u8; 16 * 1024];
        let counters = Arc::new(StreamCounters::default());
        let router = tracked_streams(&[(TRACK_ID.to_owned(), original.clone(), counters.clone())]);
        let http = HttpFixture::start(with_login(router)).await;
        let temporary = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();
        let pause = RecencyPause::new();
        let client = fixture_client_with_recency_pause(&http, &temporary, &cache, &pause).await;
        let track = sample_track_with(TRACK_ID, original.len());
        let key = cache_key(&http.origin, USER_ID, TRACK_ID);

        let prepared = client
            .prepare(&track, CancellationToken::new(), progress().0)
            .await
            .unwrap();
        drop(prepared);
        await_publication(&client, &key).await;

        // The warm hit is held in its mark; the record disappears meanwhile.
        let warm = client
            .prepare(&track, CancellationToken::new(), progress().0)
            .await
            .unwrap();
        assert_eq!(std::fs::read(warm.path()).unwrap(), original);
        eventually(|| pause.entered.load(Ordering::SeqCst) == 1).await;
        std::fs::remove_file(media_cache::sidecar_path(cache.path(), &key)).unwrap();
        pause.release();
        eventually(|| pause.finished.load(Ordering::SeqCst) == 1).await;

        // The mark found nothing and wrote nothing: the lease is unaffected.
        assert_eq!(std::fs::read(warm.path()).unwrap(), original);
        assert!(!media_cache::sidecar_path(cache.path(), &key).exists());
        assert_eq!(
            staging_files(&media_cache::account_root(cache.path(), &key)),
            0,
            "a mark that cannot be applied leaves no staging file"
        );
    }

    /// A sign-out stops a running recency mark: the mark observes the client
    /// lifetime, writes nothing and leaves no staging file behind.
    #[tokio::test]
    async fn a_sign_out_stops_a_running_recency_mark() {
        let original = vec![63_u8; 16 * 1024];
        let counters = Arc::new(StreamCounters::default());
        let router = tracked_streams(&[(TRACK_ID.to_owned(), original.clone(), counters.clone())]);
        let http = HttpFixture::start(with_login(router)).await;
        let temporary = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();
        let pause = RecencyPause::new();
        let client = fixture_client_with_recency_pause(&http, &temporary, &cache, &pause).await;
        let track = sample_track_with(TRACK_ID, original.len());
        let key = cache_key(&http.origin, USER_ID, TRACK_ID);

        let prepared = client
            .prepare(&track, CancellationToken::new(), progress().0)
            .await
            .unwrap();
        drop(prepared);
        await_publication(&client, &key).await;
        let published = sidecar_saved_at(cache.path(), &key);
        assert!(
            published > AGED_RECENCY,
            "a publication writes the current clock as the recency of the entry"
        );
        // The entry is aged, so a mark that had written would be visible.
        age_sidecar_recency(cache.path(), &key, AGED_RECENCY);

        let warm = client
            .prepare(&track, CancellationToken::new(), progress().0)
            .await
            .unwrap();
        assert_eq!(std::fs::read(warm.path()).unwrap(), original);
        eventually(|| pause.entered.load(Ordering::SeqCst) == 1).await;

        // The sign-out cancels the client lifetime; the mark stops with it.
        client.logout(CancellationToken::new()).await.unwrap();
        pause.release();
        eventually(|| pause.finished.load(Ordering::SeqCst) == 1).await;
        assert_eq!(
            sidecar_saved_at(cache.path(), &key),
            AGED_RECENCY,
            "a stopped mark writes nothing"
        );
        assert_eq!(
            staging_files(&media_cache::account_root(cache.path(), &key)),
            0,
            "a stopped mark leaves no staging file"
        );
    }

    /// The background recency marks are bounded: when the bound is reached the
    /// next mark is dropped, and the prepared lease is neither delayed nor
    /// changed.
    ///
    /// A mark is an optimization and the recency of an entry is a cache fact,
    /// not a grant: a dropped mark costs one recency label, while a queued
    /// mark would occupy the blocking pool that the next transfer needs.
    #[tokio::test]
    async fn a_saturated_recency_mark_is_dropped_and_the_lease_is_not_delayed() {
        let original = vec![71_u8; 16 * 1024];
        let counters = Arc::new(StreamCounters::default());
        let router = tracked_streams(&[(TRACK_ID.to_owned(), original.clone(), counters.clone())]);
        let http = HttpFixture::start(with_login(router)).await;
        let temporary = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();
        let pause = RecencyPause::new();
        let client = fixture_client_with_recency_pause(&http, &temporary, &cache, &pause).await;
        let track = sample_track_with(TRACK_ID, original.len());
        let key = cache_key(&http.origin, USER_ID, TRACK_ID);

        // Cold: the entry is published in the background.
        let prepared = client
            .prepare(&track, CancellationToken::new(), progress().0)
            .await
            .unwrap();
        assert_eq!(std::fs::read(prepared.path()).unwrap(), original);
        drop(prepared);
        await_publication(&client, &key).await;

        // Every confirmed hit takes a mark slot, and the gate holds the marks.
        for _ in 0..MAX_CONCURRENT_MARKS {
            let warm = client
                .prepare(&track, CancellationToken::new(), progress().0)
                .await
                .unwrap();
            assert_eq!(std::fs::read(warm.path()).unwrap(), original);
        }
        eventually(|| pause.entered.load(Ordering::SeqCst) == MAX_CONCURRENT_MARKS).await;
        assert_eq!(
            client.inner.marks.available_permits(),
            0,
            "every mark slot is taken by a held mark"
        );

        // The next confirmed hit cannot take a slot: its mark is dropped and
        // the lease is returned anyway.
        let warm = client
            .prepare(&track, CancellationToken::new(), progress().0)
            .await
            .unwrap();
        assert_eq!(std::fs::read(warm.path()).unwrap(), original);
        assert_eq!(
            client.inner.dropped_marks.load(Ordering::SeqCst),
            1,
            "the mark of a saturated client is dropped"
        );
        assert_eq!(
            pause.entered.load(Ordering::SeqCst),
            MAX_CONCURRENT_MARKS,
            "the dropped mark never started"
        );
        assert_eq!(
            counters.gets.load(Ordering::SeqCst),
            1,
            "a warm hit transfers no body"
        );
    }

    /// A mark slot is not leaked: a held mark that ended gives its slot back,
    /// and the next confirmed hit is marked again.
    #[tokio::test]
    async fn a_recency_mark_slot_is_released_when_the_held_marks_end() {
        let original = vec![72_u8; 16 * 1024];
        let counters = Arc::new(StreamCounters::default());
        let router = tracked_streams(&[(TRACK_ID.to_owned(), original.clone(), counters.clone())]);
        let http = HttpFixture::start(with_login(router)).await;
        let temporary = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();
        let pause = RecencyPause::new();
        let client = fixture_client_with_recency_pause(&http, &temporary, &cache, &pause).await;
        let track = sample_track_with(TRACK_ID, original.len());
        let key = cache_key(&http.origin, USER_ID, TRACK_ID);

        let prepared = client
            .prepare(&track, CancellationToken::new(), progress().0)
            .await
            .unwrap();
        assert_eq!(std::fs::read(prepared.path()).unwrap(), original);
        drop(prepared);
        await_publication(&client, &key).await;

        // The bound is reached and the next mark is dropped.
        for _ in 0..MAX_CONCURRENT_MARKS {
            let warm = client
                .prepare(&track, CancellationToken::new(), progress().0)
                .await
                .unwrap();
            assert_eq!(std::fs::read(warm.path()).unwrap(), original);
        }
        eventually(|| pause.entered.load(Ordering::SeqCst) == MAX_CONCURRENT_MARKS).await;
        let warm = client
            .prepare(&track, CancellationToken::new(), progress().0)
            .await
            .unwrap();
        assert_eq!(std::fs::read(warm.path()).unwrap(), original);
        assert_eq!(client.inner.dropped_marks.load(Ordering::SeqCst), 1);

        // The held marks end: every slot is free again.
        pause.release();
        eventually(|| client.inner.marks.available_permits() == MAX_CONCURRENT_MARKS).await;

        // The next confirmed hit is marked again: the package does not lose the
        // recency of an entry forever.
        let warm = client
            .prepare(&track, CancellationToken::new(), progress().0)
            .await
            .unwrap();
        assert_eq!(std::fs::read(warm.path()).unwrap(), original);
        eventually(|| pause.entered.load(Ordering::SeqCst) == MAX_CONCURRENT_MARKS + 1).await;
        assert_eq!(
            client.inner.dropped_marks.load(Ordering::SeqCst),
            1,
            "no mark was dropped after the slots were free again"
        );
        assert_eq!(
            counters.gets.load(Ordering::SeqCst),
            1,
            "a warm hit transfers no body"
        );
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

    /// A sign-out during a copy that has already written its first block stops that
    /// copy: no entry, no sidecar and no staging file survives.
    ///
    /// The copy is held *inside* `copy_verified` after its first block is on disk,
    /// so the assertion is about a copy that really started. Holding it before the
    /// copy (as a publication gate does) would only prove that a copy which never
    /// began leaves nothing behind.
    #[tokio::test]
    async fn a_sign_out_stops_a_running_publication() {
        // The copy gate is a process-global slot: only one test may install a
        // gate at a time, or the copy of this test would run unheld.
        let _section = media_cache::copy_gate::exclusive();
        let original = vec![44_u8; 3 * media_cache::BLOCK_BYTES];
        let counters = Arc::new(StreamCounters::default());
        let router = tracked_streams(&[(TRACK_ID.to_owned(), original.clone(), counters.clone())]);
        let http = HttpFixture::start(with_login(router)).await;
        let temporary = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();
        let client = fixture_client(&http, &temporary, &cache).await;
        let track = sample_track_with(TRACK_ID, original.len());
        let key = cache_key(&http.origin, USER_ID, TRACK_ID);
        let cached_audio = media_cache::audio_path(cache.path(), &key);
        let cached_sidecar = media_cache::sidecar_path(cache.path(), &key);
        let account = media_cache::account_root(cache.path(), &key);
        let gate = media_cache::copy_gate::CopyGate::new(cache.path());
        // The guard removes the gate again even if an assertion below panics.
        let _installed = media_cache::copy_gate::install_scoped(gate.clone());

        let prepared = client
            .prepare(&track, CancellationToken::new(), progress().0)
            .await
            .unwrap();
        assert_eq!(counters.gets.load(Ordering::SeqCst), 1);
        drop(prepared);
        // The copy really started: its first block is on disk, held by the gate.
        eventually(|| gate.entered() == 1).await;
        let staged_while_held = staging_files(&account);
        assert_eq!(
            staged_while_held, 1,
            "the copy must have written a staging file before the sign-out"
        );

        // The sign-out ends the client's lifetime while the copy is mid-block.
        client.logout(CancellationToken::new()).await.unwrap();
        gate.release();
        eventually(|| gate.entered() == 1 && staging_files(&account) == 0).await;

        assert_eq!(staging_files(&account), 0, "no staging file survives");
        assert!(
            !cached_audio.exists(),
            "a sign-out must not publish a late cache entry"
        );
        assert!(!cached_sidecar.exists());
        assert_eq!(counters.gets.load(Ordering::SeqCst), 1);
    }

    /// The lease of a slow prepared body is returned while the cache copy of that
    /// body is still running.
    ///
    /// The order is proved by the publication gate and not by comparing two
    /// wall-clock instants: the gate is entered before the copy starts and only
    /// released after the lease was handed out, so "the entry does not exist yet
    /// while the publication is still running" is a deterministic observation.
    /// The timing print below is a metric, not an assertion.
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
        let pause = PublicationPause::new();
        let client = fixture_client_with_pause(&http, &temporary, &cache, &pause).await;
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
        // The publication of this very body is still running when the lease is
        // already in the caller's hands.
        eventually(|| pause.entered.load(Ordering::SeqCst) == 1).await;
        assert_eq!(
            pause.finished.load(Ordering::SeqCst),
            0,
            "the lease must be handed out before the cache copy of that body finished"
        );
        assert!(
            !cached_audio.exists(),
            "the lease must be returned before the cache copy, not after it"
        );

        // The entry appears only once the copy of the verified original ran.
        pause.release();
        eventually(|| pause.finished.load(Ordering::SeqCst) == 1).await;
        eventually(|| cached_audio.exists()).await;
        let published = started.elapsed();
        eprintln!(
            "TL-01 measurement: 8 MiB original, lease returned after {leased:?}, cache entry published after {published:?}, publication cost {:?}",
            published.saturating_sub(leased)
        );
    }

    #[tokio::test]
    async fn the_registry_of_running_publications_is_bounded() {
        let original = vec![53_u8; 16 * 1024];
        let other_original = vec![54_u8; 16 * 1024];
        let counters = Arc::new(StreamCounters::default());
        let other_counters = Arc::new(StreamCounters::default());
        let router = tracked_streams(&[
            (TRACK_ID.to_owned(), original.clone(), counters.clone()),
            (
                OTHER_TRACK_ID.to_owned(),
                other_original.clone(),
                other_counters.clone(),
            ),
        ]);
        let http = HttpFixture::start(with_login(router)).await;
        let temporary = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();
        let pause = PublicationPause::new();
        let client = fixture_client_with_pause(&http, &temporary, &cache, &pause).await;
        let track = sample_track_with(TRACK_ID, original.len());
        let cached_audio =
            media_cache::audio_path(cache.path(), &cache_key(&http.origin, USER_ID, TRACK_ID));

        // Fill the registry with the maximum number of running publications.
        let mut running = Vec::new();
        {
            let mut registry = client.inner.publications.lock().unwrap();
            for index in 0..MAX_IN_FLIGHT_PUBLICATIONS {
                let other = cache_key(
                    &http.origin,
                    USER_ID,
                    &format!("30000000-0000-4000-8000-{index:012x}"),
                );
                let publication = Publication::new();
                registry.insert(other, Arc::downgrade(&publication));
                running.push(publication);
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
        assert_eq!(
            pause.entered.load(Ordering::SeqCst),
            0,
            "a refused publication starts no background copy"
        );
        assert_eq!(counters.gets.load(Ordering::SeqCst), 1);

        // The limit counts live publications, not dictionary entries: the
        // entries of publications that ended describe nothing that runs and do
        // not block a new publication.
        drop(running);
        {
            let mut registry = client.inner.publications.lock().unwrap();
            for index in 0..MAX_IN_FLIGHT_PUBLICATIONS {
                let other = cache_key(
                    &http.origin,
                    USER_ID,
                    &format!("30000000-0000-4000-8000-{index:012x}"),
                );
                let publication = Publication::new();
                registry.insert(other, Arc::downgrade(&publication));
                // The publication ends without a task: its entry stays in the
                // dictionary but describes nothing that runs.
                publication.finish();
            }
            assert_eq!(registry.len(), MAX_IN_FLIGHT_PUBLICATIONS);
        }
        let other = sample_track_with(OTHER_TRACK_ID, other_original.len());
        let prepared = client
            .prepare(&other, CancellationToken::new(), progress().0)
            .await
            .unwrap();
        drop(prepared);
        assert_eq!(other_counters.gets.load(Ordering::SeqCst), 1);
        eventually(|| pause.entered.load(Ordering::SeqCst) == 1).await;
        assert_eq!(
            client.inner.publications.lock().unwrap().len(),
            1,
            "the limit counts the live publications, not the dictionary entries"
        );
        pause.release();
    }

    /// A pending cache publication must never block the play of another track:
    /// the wait for a running publication never holds the transfer admission.
    #[tokio::test]
    async fn a_pending_publication_does_not_block_the_play_of_another_track() {
        let first_bytes = vec![61_u8; 64 * 1024];
        let second_bytes = vec![62_u8; 64 * 1024];
        let first_counters = Arc::new(StreamCounters::default());
        let second_counters = Arc::new(StreamCounters::default());
        let router = tracked_streams(&[
            (
                TRACK_ID.to_owned(),
                first_bytes.clone(),
                first_counters.clone(),
            ),
            (
                OTHER_TRACK_ID.to_owned(),
                second_bytes.clone(),
                second_counters.clone(),
            ),
        ]);
        let http = HttpFixture::start(with_login(router)).await;
        let temporary = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();
        let pause = PublicationPause::new();
        let client = fixture_client_with_pause(&http, &temporary, &cache, &pause).await;
        let first = sample_track_with(TRACK_ID, first_bytes.len());
        let second = sample_track_with(OTHER_TRACK_ID, second_bytes.len());

        // The first track is prepared and its cache publication is held.
        let prepared = client
            .prepare(&first, CancellationToken::new(), progress().0)
            .await
            .unwrap();
        drop(prepared);
        eventually(|| pause.entered.load(Ordering::SeqCst) == 1).await;

        // The offline preparation of the very same track waits for the running
        // publication: it must not hold the transfer admission while it waits.
        let offline = {
            let client = client.clone();
            let first = first.clone();
            tokio::spawn(async move {
                client
                    .prepare(&first, CancellationToken::new(), progress().0)
                    .await
            })
        };
        tokio::time::sleep(Duration::from_millis(200)).await;
        assert!(
            !offline.is_finished(),
            "the offline preparation waits for the running publication"
        );

        // The transfer admission gives up after one second. The play of
        // another track must not be refused as busy because a publication of
        // the first track is still pending.
        tokio::time::sleep(Duration::from_millis(1_100)).await;
        let other = tokio::time::timeout(
            Duration::from_secs(10),
            client.prepare(&second, CancellationToken::new(), progress().0),
        )
        .await
        .expect("the play of another track must not be blocked by a pending publication")
        .expect("the play of another track must not be refused as busy");
        assert_eq!(std::fs::read(other.path()).unwrap(), second_bytes);
        assert_eq!(
            second_counters.gets.load(Ordering::SeqCst),
            1,
            "the other track is transferred once"
        );
        assert_eq!(
            first_counters.gets.load(Ordering::SeqCst),
            1,
            "the waiting preparation transfers nothing"
        );

        // The publication ends: the waiting preparation is served from the
        // cache and no track ever saw a second body.
        pause.release();
        let offline_lease = tokio::time::timeout(Duration::from_secs(5), offline)
            .await
            .expect("the offline preparation did not finish")
            .unwrap()
            .unwrap();
        assert_eq!(std::fs::read(offline_lease.path()).unwrap(), first_bytes);
        assert_eq!(
            first_counters.gets.load(Ordering::SeqCst),
            1,
            "one body per track is transferred once"
        );
        assert_eq!(
            first_counters.body_bytes.load(Ordering::SeqCst),
            first_bytes.len()
        );
        assert_eq!(
            second_counters.body_bytes.load(Ordering::SeqCst),
            second_bytes.len()
        );
    }

    /// A publication that ends exactly between the cache miss and the
    /// single-flight decision must not cost a second body: the cache is read
    /// again instead of the transfer being started.
    #[tokio::test]
    async fn a_publication_that_ends_between_the_cache_miss_and_the_decision_does_not_transfer_twice()
     {
        let original = vec![63_u8; 64 * 1024];
        let counters = Arc::new(StreamCounters::default());
        let router = tracked_streams(&[(TRACK_ID.to_owned(), original.clone(), counters.clone())]);
        let http = HttpFixture::start(with_login(router)).await;
        let temporary = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();
        let pause = PublicationPause::new();
        let barrier = CacheMissBarrier::new();
        let client = fixture_client_with_barrier(&http, &temporary, &cache, &pause, &barrier).await;
        let track = sample_track_with(TRACK_ID, original.len());
        let key = cache_key(&http.origin, USER_ID, TRACK_ID);

        // The first prepare transfers the body and starts the cache publication.
        let first = client
            .prepare(&track, CancellationToken::new(), progress().0)
            .await
            .unwrap();
        drop(first);
        eventually(|| pause.entered.load(Ordering::SeqCst) == 1).await;

        // The second prepare finds a cold cache and is held right before the
        // single-flight decision.
        barrier.arm();
        let second = {
            let client = client.clone();
            let track = track.clone();
            tokio::spawn(async move {
                client
                    .prepare(&track, CancellationToken::new(), progress().0)
                    .await
            })
        };
        eventually(|| barrier.reached() >= 1).await;

        // The publication ends exactly between the cache miss and the decision.
        pause.release();
        await_publication(&client, &key).await;
        barrier.release();

        let second = tokio::time::timeout(Duration::from_secs(5), second)
            .await
            .expect("the second prepare did not finish")
            .unwrap()
            .unwrap();
        assert_eq!(std::fs::read(second.path()).unwrap(), original);
        assert_eq!(
            counters.gets.load(Ordering::SeqCst),
            1,
            "a publication that ended between the miss and the decision must not cost a second body"
        );
        assert_eq!(counters.body_bytes.load(Ordering::SeqCst), original.len());
    }

    /// A publication that ends exactly between the *post-admission* cache read
    /// and the single-flight decision must not cost a second body either: the
    /// publication slot is observed before that read, so an ended publication
    /// is answered with a second read of the cache and not with a transfer.
    #[tokio::test]
    async fn a_publication_that_ends_between_the_post_admission_miss_and_the_decision_does_not_transfer_twice()
     {
        let original = vec![65_u8; 64 * 1024];
        let counters = Arc::new(StreamCounters::default());
        let router = tracked_streams(&[(TRACK_ID.to_owned(), original.clone(), counters.clone())]);
        let http = HttpFixture::start(with_login(router)).await;
        let temporary = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();
        let pause = PublicationPause::new();
        let miss = CacheMissBarrier::new();
        let recheck = CacheMissBarrier::new();
        let client =
            fixture_client_with_recheck_barrier(&http, &temporary, &cache, &pause, &miss, &recheck)
                .await;
        let track = sample_track_with(TRACK_ID, original.len());
        let key = cache_key(&http.origin, USER_ID, TRACK_ID);

        // The second prepare is held at its first cache read, before any
        // publication of this key exists: it observes no publication slot at
        // all and must not lose one that starts while it waits.
        miss.arm_next();
        let second = {
            let client = client.clone();
            let track = track.clone();
            tokio::spawn(async move {
                client
                    .prepare(&track, CancellationToken::new(), progress().0)
                    .await
            })
        };
        eventually(|| miss.reached() >= 1).await;

        // The first prepare transfers the body and starts the cache
        // publication, which is held so it can end inside the window below.
        let first = client
            .prepare(&track, CancellationToken::new(), progress().0)
            .await
            .unwrap();
        drop(first);
        eventually(|| pause.entered.load(Ordering::SeqCst) == 1).await;

        // The second prepare continues and reaches the post-admission read:
        // the publication slot is observed (the publication is alive) and the
        // cache is still cold.
        recheck.arm();
        miss.release();
        eventually(|| recheck.reached() >= 1).await;

        // The publication ends exactly between the post-admission miss and the
        // single-flight decision.
        pause.release();
        await_publication(&client, &key).await;
        recheck.release();

        let second = tokio::time::timeout(Duration::from_secs(5), second)
            .await
            .expect("the second prepare did not finish")
            .unwrap()
            .unwrap();
        assert_eq!(std::fs::read(second.path()).unwrap(), original);
        assert_eq!(
            counters.gets.load(Ordering::SeqCst),
            1,
            "a publication that ended between the post-admission miss and the decision must not cost a second body"
        );
        assert_eq!(counters.body_bytes.load(Ordering::SeqCst), original.len());
    }

    /// An expired wait followed by a publication that ends inside the
    /// post-admission cache read must not cost a second body either.
    ///
    /// The first wait runs out while the publication is still copying, so
    /// `prepare` continues with its own transfer; the publication then finishes
    /// inside the post-admission read. The slot was live when that read started,
    /// so the cache is read again instead of a second body being transferred.
    #[tokio::test]
    async fn a_publication_that_ends_after_an_expired_wait_does_not_transfer_twice() {
        let original = vec![66_u8; 64 * 1024];
        let counters = Arc::new(StreamCounters::default());
        let router = tracked_streams(&[(TRACK_ID.to_owned(), original.clone(), counters.clone())]);
        let http = HttpFixture::start(with_login(router)).await;
        let temporary = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();
        let pause = PublicationPause::new();
        let miss = CacheMissBarrier::new();
        let recheck = CacheMissBarrier::new();
        let mut client =
            fixture_client_with_recheck_barrier(&http, &temporary, &cache, &pause, &miss, &recheck)
                .await;
        {
            let inner = Arc::get_mut(&mut client.inner).unwrap();
            // The first wait for the running publication runs out quickly.
            inner.publication_wait = Duration::from_millis(200);
        }
        let track = sample_track_with(TRACK_ID, original.len());
        let key = cache_key(&http.origin, USER_ID, TRACK_ID);

        // The first prepare transfers the body and starts the cache publication,
        // which stays held before its copy.
        let first = client
            .prepare(&track, CancellationToken::new(), progress().0)
            .await
            .unwrap();
        assert_eq!(std::fs::read(first.path()).unwrap(), original);
        drop(first);
        eventually(|| pause.entered.load(Ordering::SeqCst) == 1).await;

        // The second prepare misses the cache, waits 200 ms for the running
        // publication, runs out, and reaches the post-admission read, where it is
        // held. It has transferred nothing yet.
        recheck.arm();
        let second = {
            let client = client.clone();
            let track = track.clone();
            tokio::spawn(async move {
                client
                    .prepare(&track, CancellationToken::new(), progress().0)
                    .await
            })
        };
        eventually(|| recheck.reached() >= 1).await;
        assert_eq!(
            counters.gets.load(Ordering::SeqCst),
            1,
            "the held post-admission read must not have started a second transfer"
        );

        // The publication finishes inside that read: the cache is warm now.
        pause.release();
        await_publication(&client, &key).await;
        recheck.release();

        let second = tokio::time::timeout(Duration::from_secs(10), second)
            .await
            .expect("the second prepare did not finish")
            .unwrap()
            .unwrap();
        assert_eq!(std::fs::read(second.path()).unwrap(), original);
        assert_eq!(
            counters.gets.load(Ordering::SeqCst),
            1,
            "a publication that ended after an expired wait must not cost a second body"
        );
        assert_eq!(counters.body_bytes.load(Ordering::SeqCst), original.len());
    }

    /// The second post-admission read — the one answered after a publication of
    /// the key was seen alive and has ended — must not hold the transfer
    /// admission either. It re-hashes a whole published original (up to 250 MiB),
    /// so a warm hit would otherwise refuse the play of another track with
    /// `PREPARE_BUSY`, exactly the symptom the round-2 correction removed from
    /// the wait.
    #[tokio::test]
    async fn the_warm_reread_after_an_expired_wait_does_not_hold_the_admission() {
        let original = vec![67_u8; 64 * 1024];
        let other_original = vec![68_u8; 32 * 1024];
        let counters = Arc::new(StreamCounters::default());
        let other_counters = Arc::new(StreamCounters::default());
        let router = tracked_streams(&[
            (TRACK_ID.to_owned(), original.clone(), counters.clone()),
            (
                OTHER_TRACK_ID.to_owned(),
                other_original.clone(),
                other_counters.clone(),
            ),
        ]);
        let http = HttpFixture::start(with_login(router)).await;
        let temporary = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();
        let pause = PublicationPause::new();
        let miss = CacheMissBarrier::new();
        let recheck = CacheMissBarrier::new();
        let ended = CacheMissBarrier::new();
        let mut client = fixture_client_with_ended_read_barrier(
            &http, &temporary, &cache, &pause, &miss, &recheck, &ended,
        )
        .await;
        {
            let inner = Arc::get_mut(&mut client.inner).unwrap();
            inner.publication_wait = Duration::from_millis(200);
        }
        let track = sample_track_with(TRACK_ID, original.len());
        let other = sample_track_with(OTHER_TRACK_ID, other_original.len());
        let key = cache_key(&http.origin, USER_ID, TRACK_ID);

        // The first prepare transfers the body and starts the cache publication,
        // which stays held.
        let first = client
            .prepare(&track, CancellationToken::new(), progress().0)
            .await
            .unwrap();
        drop(first);
        eventually(|| pause.entered.load(Ordering::SeqCst) == 1).await;

        // The second prepare misses, runs out of its 200 ms wait and reaches the
        // post-admission miss, where it is held.
        recheck.arm();
        ended.arm();
        let second = {
            let client = client.clone();
            let track = track.clone();
            tokio::spawn(async move {
                client
                    .prepare(&track, CancellationToken::new(), progress().0)
                    .await
            })
        };
        eventually(|| recheck.reached() >= 1).await;

        // The publication ends inside that read, so the second read answers a
        // warm hit — and that read is now held.
        pause.release();
        await_publication(&client, &key).await;
        recheck.release();
        eventually(|| ended.reached() >= 1).await;

        // While the warm re-read runs, the play of another track must not be
        // refused as busy. The admission gives up after one second, so a held
        // admission fails this assertion instead of merely slowing it down.
        let other_lease = tokio::time::timeout(
            Duration::from_secs(10),
            client.prepare(&other, CancellationToken::new(), progress().0),
        )
        .await
        .expect("the play of another track must not wait for the warm re-read")
        .expect("the play of another track must not be refused as busy");
        assert_eq!(std::fs::read(other_lease.path()).unwrap(), other_original);
        assert_eq!(other_counters.gets.load(Ordering::SeqCst), 1);

        ended.release();
        let second = tokio::time::timeout(Duration::from_secs(10), second)
            .await
            .expect("the second prepare did not finish")
            .unwrap()
            .unwrap();
        assert_eq!(std::fs::read(second.path()).unwrap(), original);
        assert_eq!(
            counters.gets.load(Ordering::SeqCst),
            1,
            "the warm re-read must not cost a second body"
        );
    }

    /// An expired wait must not start a second publisher: the running
    /// publication keeps the single-flight slot of its key, and exactly one new
    /// publication may run after it ended.
    #[tokio::test]
    async fn an_expired_wait_does_not_start_a_second_publisher() {
        let original = vec![64_u8; 32 * 1024];
        let counters = Arc::new(StreamCounters::default());
        let router = tracked_streams(&[(TRACK_ID.to_owned(), original.clone(), counters.clone())]);
        let http = HttpFixture::start(with_login(router)).await;
        let temporary = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();
        // The cache root is a regular file: every publication fails, so the
        // cache stays cold and a later prepare is a new transfer.
        let blocked = cache.path().join("blocked");
        std::fs::write(&blocked, b"not a directory").unwrap();
        let pause = PublicationPause::new();
        let mut client = fixture_client_with_pause(&http, &temporary, &cache, &pause).await;
        {
            let inner = Arc::get_mut(&mut client.inner).unwrap();
            inner.cache_directory = Some(blocked);
            // The wait for a running publication runs out quickly here.
            inner.publication_wait = Duration::from_millis(200);
        }
        let track = sample_track_with(TRACK_ID, original.len());
        let key = cache_key(&http.origin, USER_ID, TRACK_ID);

        // The first prepare transfers the body and starts the cache publication,
        // which is held.
        let first = client
            .prepare(&track, CancellationToken::new(), progress().0)
            .await
            .unwrap();
        drop(first);
        eventually(|| pause.entered.load(Ordering::SeqCst) == 1).await;

        // The second prepare waits for the running publication, the wait runs
        // out and the ordinary transfer decides.
        let second = {
            let client = client.clone();
            let track = track.clone();
            tokio::spawn(async move {
                client
                    .prepare(&track, CancellationToken::new(), progress().0)
                    .await
            })
        };
        tokio::time::timeout(Duration::from_secs(10), second)
            .await
            .expect("the second prepare did not finish")
            .unwrap()
            .unwrap();
        assert_eq!(
            counters.gets.load(Ordering::SeqCst),
            2,
            "the expired wait costs exactly one new transfer"
        );

        // The running publication keeps the single-flight slot: the expired
        // wait started no second publisher.
        assert_eq!(
            pause.entered.load(Ordering::SeqCst),
            1,
            "an expired wait must not start a second publisher"
        );
        {
            let registry = client.inner.publications.lock().unwrap();
            assert_eq!(registry.len(), 1, "the registry holds one live entry");
            assert!(registry.get(&key).is_some());
        }

        // The first publication ends: the slot is free again.
        pause.release();
        await_publication(&client, &key).await;
        assert!(
            client.inner.publications.lock().unwrap().is_empty(),
            "the ended publication released its slot"
        );

        // A later prepare of the same key starts exactly one new publication.
        pause.arm();
        let third = client
            .prepare(&track, CancellationToken::new(), progress().0)
            .await
            .unwrap();
        drop(third);
        eventually(|| pause.entered.load(Ordering::SeqCst) == 2).await;
        assert_eq!(
            pause.entered.load(Ordering::SeqCst),
            2,
            "exactly one new publication runs after the first one ended"
        );
        assert_eq!(client.inner.publications.lock().unwrap().len(), 1);
        pause.release();
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
