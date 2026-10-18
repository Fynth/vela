//! One explicitly selected Vela origin. Credentials remain in Rust memory;
//! owned originals become playable only after bounded size/hash verification.
//!
//! A complete-file cache of the authenticated account on this origin is reused when,
//! and only when, the origin itself re-authorizes the current session for the same
//! strong validator and the cached file still hashes to it from the descriptor the
//! decoder receives. Cached bytes are never a grant: a denial, a changed validator or
//! a local digest mismatch falls back to the existing authorized full GET.
use crate::{DownloadProgress, PreparedAudio, media_cache};
use futures_util::StreamExt;
use reqwest::{Client, Method, Response, StatusCode, header};
use serde::de::DeserializeOwned;
use std::{
    future::Future,
    io::{Seek, Write},
    net::ToSocketAddrs,
    path::PathBuf,
    sync::{Arc, Mutex, OnceLock},
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
pub(crate) const MAX_MEDIA_BYTES: u64 = 250 * 1024 * 1024;
const MAX_JSON_BYTES: usize = 4 * 1024 * 1024;
const FILE_CHUNK: usize = 64 * 1024;

// Intentionally no Debug implementation: neither request headers nor the
// selected origin/query are diagnostic output for the native UI.
#[derive(Clone)]
pub struct VelaClient {
    inner: Arc<Inner>,
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
    #[cfg(test)]
    temporary_directory: Option<PathBuf>,
    #[cfg(test)]
    cache_directory: Option<PathBuf>,
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
                    #[cfg(test)]
                    temporary_directory: None,
                    #[cfg(test)]
                    cache_directory: None,
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
                    #[cfg(test)]
                    temporary_directory: None,
                    #[cfg(test)]
                    cache_directory: None,
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
                // Warm the reusable cache from the lease that was just verified. This is
                // best effort by contract: a full cache, a full disk or a canceled copy
                // must never fail or invalidate playback that already succeeded.
                if let Some(scope) = &scope {
                    scope.store(&prepared, &verified_hash, &cancel).await;
                }
                Ok(prepared)
            },
        )
        .await
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

    fn cache_root(&self) -> Result<PathBuf, String> {
        #[cfg(test)]
        if let Some(directory) = &self.inner.cache_directory {
            return Ok(directory.clone());
        }
        media_cache::default_root()
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
        // The previous prepare warmed the reusable cache. This phase deliberately needs
        // an *in-flight* transfer, so the warm entry is dropped instead of being served.
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
}
