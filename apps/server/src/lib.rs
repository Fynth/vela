mod upload_guard;
use axum::{
    Json, Router,
    body::Body,
    extract::Request,
    extract::{ConnectInfo, DefaultBodyLimit, FromRequestParts, Multipart, Path, Query, State},
    http::{HeaderMap, HeaderValue, Method, StatusCode, header, request::Parts},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{delete, get, post, put},
};
use futures_util::StreamExt;
use serde::Deserialize;
use sqlx::SqlitePool;
use std::{
    collections::HashMap,
    net::{IpAddr, SocketAddr},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tokio::{
    io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt},
    sync::Semaphore,
};
use tokio_util::io::ReaderStream;
use vela_core::{
    auth::{AuthService, Login, SESSION_SECONDS, Session},
    config::Config,
    playlists::PlaylistService,
};
use vela_domain::*;
use vela_imports::ImportService;
use vela_media::MediaService;

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub auth: AuthService,
    pub media: MediaService,
    pub imports: ImportService,
    pub scanner: vela_scanner::Scanner,
    pub jobs: vela_core::jobs::JobService,
    pub playlists: PlaylistService,
    pub config: Config,
    pub shutdown: tokio_util::sync::CancellationToken,
    uploads: Arc<Semaphore>,
    streams: Arc<Semaphore>,
    requests: Arc<Semaphore>,
    login_limit: Arc<Mutex<LoginLimiter>>,
    _data_lock: Arc<vela_core::database::DataLock>,
}
struct LoginLimiter {
    ips: HashMap<IpAddr, (Instant, u32)>,
    global: (Instant, u32),
}
impl LoginLimiter {
    fn new() -> Self {
        Self {
            ips: HashMap::new(),
            global: (Instant::now(), 0),
        }
    }
    fn allow(&mut self, ip: IpAddr) -> bool {
        let now = Instant::now();
        let window = Duration::from_secs(900);
        self.ips.retain(|_, (t, _)| now.duration_since(*t) < window);
        if now.duration_since(self.global.0) > Duration::from_secs(60) {
            self.global = (now, 0);
        }
        if self.global.1 >= 100 {
            return false;
        }
        self.global.1 += 1;
        if self.ips.len() >= 4096 && !self.ips.contains_key(&ip) {
            return false;
        }
        let entry = self.ips.entry(ip).or_insert((now, 0));
        entry.1 += 1;
        entry.1 <= 15
    }
}
impl AppState {
    /// Drain detached atomic publication tasks before closing SQLite.
    pub async fn drain_uploads(&self) -> anyhow::Result<tokio::sync::OwnedSemaphorePermit> {
        Ok(self.uploads.clone().acquire_many_owned(2).await?)
    }
    pub async fn initialize(config: Config) -> anyhow::Result<Self> {
        config.validate_scan_roots()?;
        let data_lock = Arc::new(vela_core::database::DataLock::acquire(&config.data_dir).await?);
        let pool = vela_core::database::open(&config.data_dir).await?;
        let auth = AuthService::new(pool.clone()).await?;
        let media = MediaService::new(pool.clone(), config.data_dir.clone(), config.max_file_bytes)
            .with_storage_budget(config.max_storage_bytes);
        media.initialize().await?;
        let default_source = config.default_source_path()?;
        if config
            .scan_roots
            .iter()
            .any(|root| root.path == default_source)
        {
            tokio::fs::create_dir_all(&default_source).await?;
        }
        config.validate_scan_aliases().await?;
        let scanner = vela_scanner::Scanner::new(
            pool.clone(),
            media.clone(),
            config.data_dir.clone(),
            config.max_file_bytes,
            config
                .scan_roots
                .iter()
                .map(|r| vela_scanner::RootConfig {
                    id: r.id.clone(),
                    name: r.name.clone(),
                    path: r.path.clone(),
                })
                .collect(),
        );
        scanner.initialize().await?;
        // A server restart invalidates abandoned uploads. Never touch managed media.
        let mut staging = tokio::fs::read_dir(config.data_dir.join("staging")).await?;
        while let Some(entry) = staging.next_entry().await? {
            if entry.file_name().to_string_lossy().starts_with("upload-")
                && entry.file_type().await?.is_file()
            {
                tokio::fs::remove_file(entry.path()).await?;
            }
        }
        let imports = ImportService::new(
            pool.clone(),
            media.clone(),
            config.data_dir.clone(),
            config.max_file_bytes,
        )
        .with_limits(vela_imports::ImportLimits {
            max_storage_bytes: config.max_storage_bytes,
            ..Default::default()
        });
        Ok(Self {
            playlists: PlaylistService::new(pool.clone()),
            jobs: vela_core::jobs::JobService::new(pool.clone()),
            pool,
            auth,
            media,
            imports,
            scanner,
            config,
            shutdown: tokio_util::sync::CancellationToken::new(),
            uploads: Arc::new(Semaphore::new(2)),
            streams: Arc::new(Semaphore::new(16)),
            requests: Arc::new(Semaphore::new(64)),
            login_limit: Arc::new(Mutex::new(LoginLimiter::new())),
            _data_lock: data_lock,
        })
    }
}
pub struct HttpError(pub StatusCode, pub String);
impl IntoResponse for HttpError {
    fn into_response(self) -> Response {
        (self.0, Json(ApiError { error: self.1 })).into_response()
    }
}
impl From<vela_core::Error> for HttpError {
    fn from(e: vela_core::Error) -> Self {
        use vela_core::Error::*;
        match e {
            Invalid(s) => Self(StatusCode::BAD_REQUEST, s),
            Unauthorized => Self(StatusCode::UNAUTHORIZED, "Authentication required".into()),
            Forbidden => Self(StatusCode::FORBIDDEN, "Permission denied".into()),
            NotFound => Self(StatusCode::NOT_FOUND, "Not found".into()),
            Conflict(s) => Self(StatusCode::CONFLICT, s),
            Internal(e) => internal(e),
        }
    }
}
impl From<anyhow::Error> for HttpError {
    fn from(e: anyhow::Error) -> Self {
        internal(e)
    }
}
impl From<std::io::Error> for HttpError {
    fn from(e: std::io::Error) -> Self {
        internal(e.into())
    }
}
impl From<sqlx::Error> for HttpError {
    fn from(e: sqlx::Error) -> Self {
        internal(e.into())
    }
}
fn internal(e: anyhow::Error) -> HttpError {
    tracing::error!(error=%e,"operation failed");
    HttpError(
        StatusCode::INTERNAL_SERVER_ERROR,
        "Operation failed. Check server diagnostics.".into(),
    )
}
fn invalid(s: &str) -> HttpError {
    HttpError(StatusCode::BAD_REQUEST, s.into())
}
fn not_found() -> HttpError {
    HttpError(StatusCode::NOT_FOUND, "Not found".into())
}
type Result<T> = std::result::Result<T, HttpError>;

pub struct Auth(pub Session);
impl FromRequestParts<AppState> for Auth {
    type Rejection = HttpError;
    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self> {
        let cookie = parts
            .headers
            .get(header::COOKIE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        let token = cookie
            .split(';')
            .filter_map(|c| c.trim().split_once('='))
            .find(|(name, _)| *name == "vela_session")
            .map(|(_, v)| v)
            .ok_or(HttpError(
                StatusCode::UNAUTHORIZED,
                "Authentication required".into(),
            ))?;
        Ok(Self(state.auth.session(token).await?))
    }
}
pub struct WriteAuth(pub Session);
impl FromRequestParts<AppState> for WriteAuth {
    type Rejection = HttpError;
    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self> {
        let Auth(session) = Auth::from_request_parts(parts, state).await?;
        let csrf = parts
            .headers
            .get("x-csrf-token")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        if blake3::hash(csrf.as_bytes()) != blake3::hash(session.response.csrf_token.as_bytes()) {
            return Err(HttpError(
                StatusCode::FORBIDDEN,
                "CSRF token required".into(),
            ));
        }
        Ok(Self(session))
    }
}
async fn protection(State(state): State<AppState>, req: Request, next: Next) -> Response {
    // Serve only shipped frontend assets, even if the web directory is misconfigured.
    let path = req.uri().path();
    let frontend_asset = matches!(
        path,
        "/" | "/index.html"
            | "/style.css"
            | "/bootstrap.js"
            | "/player.js"
            | "/vela.svg"
            | "/THIRD_PARTY_NOTICES.txt"
    ) || (path.starts_with("/pkg/")
        && (path.ends_with(".js") || path.ends_with(".wasm")));
    if !(path.starts_with("/api/")
        && req
            .extensions()
            .get::<axum::extract::MatchedPath>()
            .is_some())
        && path != "/health"
        && !frontend_asset
    {
        return HttpError(StatusCode::NOT_FOUND, "Not found".into()).into_response();
    }
    let Ok(_permit) = state.requests.clone().try_acquire_owned() else {
        return HttpError(
            StatusCode::TOO_MANY_REQUESTS,
            "Server is busy. Try again shortly.".into(),
        )
        .into_response();
    };
    if !matches!(*req.method(), Method::GET | Method::HEAD | Method::OPTIONS) {
        if req
            .headers()
            .get("sec-fetch-site")
            .and_then(|v| v.to_str().ok())
            == Some("cross-site")
        {
            return HttpError(StatusCode::FORBIDDEN, "Cross-site write denied".into())
                .into_response();
        }
        if let Some(origin) = req.headers().get(header::ORIGIN) {
            let valid = origin
                .to_str()
                .ok()
                .and_then(|s| url::Url::parse(s).ok())
                .is_some_and(|url| {
                    if !matches!(url.scheme(), "http" | "https")
                        || !url.username().is_empty()
                        || url.password().is_some()
                        || url.path() != "/"
                        || url.query().is_some()
                        || url.fragment().is_some()
                    {
                        return false;
                    }
                    let authority = match url.port() {
                        Some(p) => format!("{}:{p}", url.host_str().unwrap_or("")),
                        None => url.host_str().unwrap_or("").to_string(),
                    };
                    req.headers()
                        .get(header::HOST)
                        .and_then(|v| v.to_str().ok())
                        .is_some_and(|h| h.eq_ignore_ascii_case(&authority))
                });
            if !valid {
                return HttpError(StatusCode::FORBIDDEN, "Origin is not allowed".into())
                    .into_response();
            }
        }
    }
    let mut response = next.run(req).await;
    let headers = response.headers_mut();
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("no-referrer"),
    );
    headers.insert(header::CONTENT_SECURITY_POLICY,HeaderValue::from_static("default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; media-src 'self'; connect-src 'self'; object-src 'none'; base-uri 'none'; frame-ancestors 'none'"));
    response
}
pub fn router(state: AppState) -> Router {
    let upload = post(upload)
        .layer(DefaultBodyLimit::max(
            state.config.max_file_bytes as usize + 1024 * 1024,
        ))
        .layer(middleware::from_fn(upload_guard::bounded_multipart));
    Router::new()
        .route("/health", get(health))
        .route("/api/setup", get(setup_status).post(setup))
        .route("/api/login", post(login))
        .route("/api/logout", post(logout))
        .route("/api/me", get(me))
        .route("/api/users", post(create_user))
        .route("/api/tracks", get(tracks))
        .route("/api/albums", get(albums))
        .route("/api/albums/{id}", get(album))
        .route("/api/albums/{id}/tracks", get(album_tracks))
        .route("/api/artists", get(artists))
        .route("/api/artists/{id}", get(artist))
        .route("/api/artists/{id}/tracks", get(artist_tracks))
        .route("/api/stats", get(stats))
        .route("/api/jobs", get(background_jobs))
        .route("/api/catalog/filters", get(catalog_facets))
        .route("/api/upload", upload)
        .route("/api/tracks/{id}/stream", get(stream))
        .route("/api/artwork/{id}", get(artwork))
        .route("/api/tracks/{id}/favorite", post(favorite))
        .route(
            "/api/tracks/{id}/metadata",
            get(track_metadata)
                .patch(track_metadata_update)
                .layer(DefaultBodyLimit::max(16 * 1024)),
        )
        .route("/api/imports/preview", post(import_preview))
        .route("/api/imports", get(imports).post(import_create))
        .route("/api/imports/{id}/cancel", post(import_cancel))
        .route("/api/imports/{id}/retry", post(import_retry))
        .route("/api/storage/roots", get(storage_roots))
        .route(
            "/api/scan-sources",
            get(scan_sources)
                .post(scan_source_create)
                .layer(DefaultBodyLimit::max(8 * 1024)),
        )
        .route("/api/scan-sources/{id}", delete(scan_source_remove))
        .route("/api/scan-sources/{id}/scan", post(scan_enqueue))
        .route("/api/scans", get(scan_jobs))
        .route("/api/scans/{id}/cancel", post(scan_cancel))
        .route("/api/playlists", get(playlists).post(playlist_create))
        .route("/api/playlists/{id}", get(playlist))
        .route("/api/playlists/{id}/items", post(playlist_add))
        .route("/api/playlists/{id}/order", put(playlist_order))
        .route("/api/playlists/{id}/items/{item}", delete(playlist_remove))
        .route(
            "/api/playlists/{id}/collaborators",
            get(collaborators).post(collaborator),
        )
        .route(
            "/api/playlists/{id}/collaborators/{user}",
            delete(collaborator_revoke),
        )
        .route(
            "/api/playlists/{id}/shares",
            post(share).delete(share_revoke),
        )
        .route("/api/shares/{token}", get(shared))
        .fallback_service(
            tower_http::services::ServeDir::new(&state.config.web_dir)
                .append_index_html_on_directories(true),
        )
        .layer(DefaultBodyLimit::max(64 * 1024))
        .layer(middleware::from_fn_with_state(state.clone(), protection))
        .with_state(state)
}
async fn health(State(s): State<AppState>) -> Result<Json<serde_json::Value>> {
    sqlx::query("SELECT 1").execute(&s.pool).await?;
    Ok(Json(
        serde_json::json!({"status":"ok","version":env!("CARGO_PKG_VERSION")}),
    ))
}
async fn setup_status(State(s): State<AppState>) -> Result<Json<SetupStatus>> {
    Ok(Json(SetupStatus {
        needs_setup: s.auth.needs_setup().await?,
        version: env!("CARGO_PKG_VERSION").into(),
    }))
}
fn check_login_limit(state: &AppState, peer: SocketAddr) -> Result<()> {
    let ip = peer.ip();
    if !state
        .login_limit
        .lock()
        .map_err(|_| invalid("Rate limiter unavailable"))?
        .allow(ip)
    {
        return Err(HttpError(
            StatusCode::TOO_MANY_REQUESTS,
            "Too many sign-in attempts. Try again later.".into(),
        ));
    }
    Ok(())
}
fn login_response(state: &AppState, login: Login) -> Response {
    let secure = if state.config.cookie_secure {
        "; Secure"
    } else {
        ""
    };
    let cookie = format!(
        "vela_session={}; Path=/; HttpOnly; SameSite=Strict; Max-Age={SESSION_SECONDS}{secure}",
        login.token
    );
    ([(header::SET_COOKIE, cookie)], Json(login.response)).into_response()
}
async fn setup(
    State(s): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    Json(c): Json<Credentials>,
) -> Result<Response> {
    check_login_limit(&s, peer)?;
    Ok(login_response(&s, s.auth.setup(c).await?))
}
async fn login(
    State(s): State<AppState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    Json(c): Json<Credentials>,
) -> Result<Response> {
    check_login_limit(&s, peer)?;
    Ok(login_response(&s, s.auth.login(c).await?))
}
async fn logout(State(s): State<AppState>, WriteAuth(auth): WriteAuth) -> Result<Response> {
    s.auth.logout(&auth).await?;
    Ok((
        [(
            header::SET_COOKIE,
            "vela_session=; Path=/; HttpOnly; SameSite=Strict; Max-Age=0",
        )],
        StatusCode::NO_CONTENT,
    )
        .into_response())
}
async fn me(Auth(auth): Auth) -> Json<AuthResponse> {
    Json(auth.response)
}
async fn create_user(
    State(s): State<AppState>,
    WriteAuth(auth): WriteAuth,
    Json(c): Json<Credentials>,
) -> Result<Json<User>> {
    Ok(Json(s.auth.create_user(&auth.response.user, c).await?))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TrackQuery {
    q: Option<String>,
    cursor: Option<String>,
    limit: Option<u32>,
    favorites: Option<bool>,
    genre: Option<String>,
    codec: Option<String>,
    year_from: Option<i64>,
    year_to: Option<i64>,
    min_bitrate: Option<i64>,
    min_sample_rate: Option<i64>,
    min_bit_depth: Option<i64>,
    sort: Option<TrackSort>,
}
async fn tracks(
    State(s): State<AppState>,
    Auth(auth): Auth,
    query: std::result::Result<Query<TrackQuery>, axum::extract::rejection::QueryRejection>,
) -> Result<Json<TrackPage>> {
    let Query(q) = query.map_err(|_| invalid("Invalid track filters or pagination"))?;
    let filters = TrackFilters {
        genre: q.genre,
        codec: q.codec,
        year_from: q.year_from,
        year_to: q.year_to,
        min_bitrate: q.min_bitrate,
        min_sample_rate: q.min_sample_rate,
        min_bit_depth: q.min_bit_depth,
        sort: q.sort.unwrap_or_default(),
    };
    Ok(Json(
        s.media
            .list_tracks_filtered(
                &auth.response.user.id,
                q.q.as_deref(),
                q.cursor.as_deref(),
                q.limit.unwrap_or(50).clamp(1, 100),
                q.favorites.unwrap_or(false),
                &filters,
            )
            .await
            .map_err(track_filter_error)?,
    ))
}
fn track_filter_error(error: anyhow::Error) -> HttpError {
    if let Some(error) = error.downcast_ref::<vela_media::InvalidTrackFilter>() {
        invalid(&error.0)
    } else {
        error.into()
    }
}
async fn catalog_facets(
    State(s): State<AppState>,
    Auth(auth): Auth,
) -> Result<Json<CatalogFacets>> {
    Ok(Json(s.media.catalog_facets(&auth.response.user.id).await?))
}
async fn stats(State(s): State<AppState>, Auth(auth): Auth) -> Result<Json<LibraryStats>> {
    Ok(Json(s.media.stats(&auth.response.user.id).await?))
}
async fn background_jobs(
    State(s): State<AppState>,
    Auth(auth): Auth,
    query: std::result::Result<Query<JobQuery>, axum::extract::rejection::QueryRejection>,
) -> Result<Json<JobPage>> {
    let Query(query) = query.map_err(|_| invalid("Invalid task filters or cursor"))?;
    Ok(Json(s.jobs.list(&auth.response.user, &query).await?))
}
fn scanner_admin(auth: &Session) -> Result<&str> {
    if !auth.response.user.is_admin {
        return Err(HttpError(
            StatusCode::FORBIDDEN,
            "Administrator access required".into(),
        ));
    }
    Ok(&auth.response.user.id)
}
fn scan_error(error: anyhow::Error) -> HttpError {
    match error.downcast_ref::<vela_scanner::ScanError>() {
        Some(vela_scanner::ScanError::Invalid(message)) => invalid(message),
        Some(vela_scanner::ScanError::Conflict(message)) => {
            HttpError(StatusCode::CONFLICT, message.clone())
        }
        Some(vela_scanner::ScanError::NotFound) => not_found(),
        None => error.into(),
    }
}
async fn storage_roots(
    State(s): State<AppState>,
    Auth(auth): Auth,
) -> Result<Json<Vec<StorageRoot>>> {
    scanner_admin(&auth)?;
    Ok(Json(s.scanner.roots().await?))
}
async fn scan_sources(
    State(s): State<AppState>,
    Auth(auth): Auth,
) -> Result<Json<Vec<ScanSource>>> {
    Ok(Json(s.scanner.list_sources(scanner_admin(&auth)?).await?))
}
async fn scan_source_create(
    State(s): State<AppState>,
    WriteAuth(auth): WriteAuth,
    Json(input): Json<CreateScanSource>,
) -> Result<Json<ScanSource>> {
    Ok(Json(
        s.scanner
            .create_source(scanner_admin(&auth)?, input)
            .await
            .map_err(scan_error)?,
    ))
}
async fn scan_source_remove(
    State(s): State<AppState>,
    WriteAuth(auth): WriteAuth,
    Path(id): Path<String>,
) -> Result<StatusCode> {
    if !s
        .scanner
        .remove_source(scanner_admin(&auth)?, &id)
        .await
        .map_err(scan_error)?
    {
        return Err(not_found());
    }
    Ok(StatusCode::NO_CONTENT)
}
async fn scan_enqueue(
    State(s): State<AppState>,
    WriteAuth(auth): WriteAuth,
    Path(id): Path<String>,
) -> Result<Json<ScanJob>> {
    Ok(Json(
        s.scanner
            .enqueue(scanner_admin(&auth)?, &id)
            .await
            .map_err(scan_error)?,
    ))
}
async fn scan_jobs(State(s): State<AppState>, Auth(auth): Auth) -> Result<Json<Vec<ScanJob>>> {
    Ok(Json(s.scanner.jobs(scanner_admin(&auth)?).await?))
}
async fn scan_cancel(
    State(s): State<AppState>,
    WriteAuth(auth): WriteAuth,
    Path(id): Path<String>,
) -> Result<StatusCode> {
    if !s
        .scanner
        .cancel(scanner_admin(&auth)?, &id)
        .await
        .map_err(scan_error)?
    {
        return Err(not_found());
    }
    Ok(StatusCode::NO_CONTENT)
}
async fn track_metadata(
    State(s): State<AppState>,
    Auth(auth): Auth,
    Path(id): Path<String>,
) -> Result<Json<TrackMetadata>> {
    Ok(Json(
        s.media
            .metadata(&auth.response.user.id, &id)
            .await?
            .ok_or_else(not_found)?,
    ))
}
async fn track_metadata_update(
    State(s): State<AppState>,
    WriteAuth(auth): WriteAuth,
    Path(id): Path<String>,
    Json(update): Json<UpdateTrackMetadata>,
) -> Result<Json<TrackMetadata>> {
    let snapshot = s.media.update_metadata(&auth.response.user.id, &id, update).await.map_err(|error| {
        match error.downcast_ref::<vela_media::CatalogMetadataError>() {
            Some(vela_media::CatalogMetadataError::Invalid(message)) => invalid(message),
            Some(vela_media::CatalogMetadataError::Conflict) => HttpError(StatusCode::CONFLICT, "Metadata changed since you opened the editor. Reload current metadata before saving.".into()),
            None => error.into(),
        }
    })?.ok_or_else(not_found)?;
    Ok(Json(snapshot))
}
#[derive(Deserialize)]
struct CatalogQuery {
    cursor: Option<String>,
    limit: Option<u32>,
}
impl CatalogQuery {
    fn validate(&self) -> Result<()> {
        if self
            .cursor
            .as_ref()
            .is_some_and(|c| c.len() > 128 || uuid::Uuid::parse_str(c).is_err())
        {
            return Err(invalid("Invalid catalog cursor"));
        }
        Ok(())
    }
    fn limit(&self) -> u32 {
        self.limit.unwrap_or(50).clamp(1, 100)
    }
}
fn catalog_error(error: anyhow::Error) -> HttpError {
    if error
        .downcast_ref::<vela_media::InvalidCatalogCursor>()
        .is_some()
    {
        invalid("Invalid catalog cursor")
    } else {
        error.into()
    }
}
async fn albums(
    State(s): State<AppState>,
    Auth(auth): Auth,
    Query(q): Query<CatalogQuery>,
) -> Result<Json<AlbumPage>> {
    q.validate()?;
    Ok(Json(
        s.media
            .list_albums(&auth.response.user.id, q.cursor.as_deref(), q.limit())
            .await?,
    ))
}
async fn album(
    State(s): State<AppState>,
    Auth(auth): Auth,
    Path(id): Path<String>,
) -> Result<Json<AlbumSummary>> {
    Ok(Json(
        s.media
            .album(&auth.response.user.id, &id)
            .await?
            .ok_or_else(not_found)?,
    ))
}
async fn album_tracks(
    State(s): State<AppState>,
    Auth(auth): Auth,
    Path(id): Path<String>,
    Query(q): Query<CatalogQuery>,
) -> Result<Json<TrackPage>> {
    q.validate()?;
    Ok(Json(
        s.media
            .album_tracks(&auth.response.user.id, &id, q.cursor.as_deref(), q.limit())
            .await
            .map_err(catalog_error)?
            .ok_or_else(not_found)?,
    ))
}
async fn artists(
    State(s): State<AppState>,
    Auth(auth): Auth,
    Query(q): Query<CatalogQuery>,
) -> Result<Json<ArtistPage>> {
    q.validate()?;
    Ok(Json(
        s.media
            .list_artists(&auth.response.user.id, q.cursor.as_deref(), q.limit())
            .await?,
    ))
}
async fn artist(
    State(s): State<AppState>,
    Auth(auth): Auth,
    Path(id): Path<String>,
) -> Result<Json<ArtistSummary>> {
    Ok(Json(
        s.media
            .artist(&auth.response.user.id, &id)
            .await?
            .ok_or_else(not_found)?,
    ))
}
async fn artist_tracks(
    State(s): State<AppState>,
    Auth(auth): Auth,
    Path(id): Path<String>,
    Query(q): Query<CatalogQuery>,
) -> Result<Json<TrackPage>> {
    q.validate()?;
    Ok(Json(
        s.media
            .artist_tracks(&auth.response.user.id, &id, q.cursor.as_deref(), q.limit())
            .await
            .map_err(catalog_error)?
            .ok_or_else(not_found)?,
    ))
}
async fn upload(
    State(s): State<AppState>,
    WriteAuth(auth): WriteAuth,
    mut multipart: Multipart,
) -> Result<Json<UploadResult>> {
    let permit = s.uploads.clone().try_acquire_owned().map_err(|_| {
        HttpError(
            StatusCode::TOO_MANY_REQUESTS,
            "Two uploads are already active. Try again shortly.".into(),
        )
    })?;
    let staging = s.config.data_dir.join("staging");
    let path = tokio::task::spawn_blocking(move || {
        tempfile::Builder::new()
            .prefix("upload-")
            .suffix(".part")
            .tempfile_in(staging)
            .map(|f| f.into_temp_path())
    })
    .await
    .map_err(|e| internal(e.into()))??;
    let receiving = tokio::time::timeout(Duration::from_secs(600), async {
        let mut field = multipart
            .next_field()
            .await
            .map_err(|_| invalid("Invalid multipart upload"))?
            .ok_or_else(|| invalid("Select an audio file"))?;
        if field.name() != Some("file") {
            return Err(invalid("Audio field must be named file"));
        }
        let filename = field
            .file_name()
            .unwrap_or("Untitled audio")
            .rsplit(['/', '\\'])
            .next()
            .unwrap_or("Untitled audio")
            .to_owned();
        if filename.len() > 255 {
            return Err(invalid("Filename is too long"));
        }
        let mut file = tokio::fs::File::create(&path).await?;
        let mut size = 0u64;
        while let Some(chunk) = field
            .chunk()
            .await
            .map_err(|_| invalid("Upload interrupted or file limit exceeded"))?
        {
            size = size
                .checked_add(chunk.len() as u64)
                .ok_or_else(|| invalid("File is too large"))?;
            if size > s.config.max_file_bytes {
                return Err(HttpError(
                    StatusCode::PAYLOAD_TOO_LARGE,
                    "File exceeds the upload limit".into(),
                ));
            }
            file.write_all(&chunk).await?;
        }
        file.sync_all().await?;
        drop(file);
        drop(field);
        if multipart
            .next_field()
            .await
            .map_err(|_| invalid("Invalid multipart upload"))?
            .is_some()
        {
            return Err(invalid("Upload one file per request"));
        }
        if size == 0 {
            return Err(invalid("File is empty"));
        }
        Ok::<_, HttpError>(filename)
    });
    let filename = tokio::select! { result=receiving=>result.map_err(|_|HttpError(StatusCode::REQUEST_TIMEOUT,"Upload timed out".into()))??, _=s.shutdown.cancelled()=>return Err(HttpError(StatusCode::SERVICE_UNAVAILABLE,"Server is shutting down".into())) };
    // Finish catalog publication even if the HTTP client disconnects after receiving bytes.
    let media = s.media.clone();
    let owner = auth.response.user.id;
    let result = tokio::spawn(async move {
        let _permit = permit;
        let result = media.ingest(&owner, &path, &filename).await;
        drop(path);
        result
    })
    .await
    .map_err(|e| internal(e.into()))?;
    result.map(Json).map_err(|e| {
        tracing::debug!(error=%e,"audio rejected");
        invalid("Audio could not be added. Check format, file integrity, and storage budget.")
    })
}
async fn stream(
    State(s): State<AppState>,
    Auth(auth): Auth,
    Path(id): Path<String>,
    method: Method,
    headers: HeaderMap,
) -> Result<Response> {
    let asset = s
        .media
        .asset(&auth.response.user.id, &id)
        .await?
        .ok_or(HttpError(StatusCode::NOT_FOUND, "Track not found".into()))?;
    let permit = s
        .streams
        .clone()
        .try_acquire_owned()
        .map_err(|_| HttpError(StatusCode::TOO_MANY_REQUESTS, "Stream limit reached".into()))?;
    let mut file = tokio::fs::File::open(&asset.path).await?;
    let actual = file.metadata().await?.len();
    if actual != asset.size_bytes || actual == 0 {
        return Err(internal(anyhow::anyhow!("managed asset size changed")));
    }
    let etag = format!("\"{}\"", asset.hash);
    if headers
        .get(header::IF_NONE_MATCH)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| {
            v.split(',')
                .map(str::trim)
                .any(|v| v.strip_prefix("W/").unwrap_or(v) == etag || v == "*")
        })
    {
        return Ok((StatusCode::NOT_MODIFIED, [(header::ETAG, etag)]).into_response());
    }
    let can_range = headers
        .get(header::IF_RANGE)
        .is_none_or(|v| v.to_str().ok() == Some(&etag));
    let range = if can_range && method == Method::GET {
        headers
            .get(header::RANGE)
            .and_then(|v| v.to_str().ok())
            .filter(|v| !v.contains(','))
    } else {
        None
    };
    let (start, end, status) = if let Some(header) = range {
        match vela_media::select_range(header, actual) {
            Ok((start, end)) => (start, end, StatusCode::PARTIAL_CONTENT),
            Err(vela_media::RangeError::Invalid | vela_media::RangeError::MultipleRanges) => {
                (0, actual - 1, StatusCode::OK)
            }
            Err(vela_media::RangeError::Unsatisfiable) => {
                return Ok((
                    StatusCode::RANGE_NOT_SATISFIABLE,
                    [
                        (header::CONTENT_RANGE, format!("bytes */{actual}")),
                        (header::ACCEPT_RANGES, "bytes".into()),
                    ],
                )
                    .into_response());
            }
        }
    } else {
        (0, actual - 1, StatusCode::OK)
    };
    file.seek(std::io::SeekFrom::Start(start)).await?;
    let len = end - start + 1;
    // The stream state owns the semaphore permit until the body completes or is dropped.
    let reader = ReaderStream::with_capacity(file.take(len), 64 * 1024);
    let stream =
        futures_util::stream::unfold((reader, permit), |(mut reader, permit)| async move {
            reader.next().await.map(|chunk| (chunk, (reader, permit)))
        });
    let mut response = Response::new(Body::from_stream(stream));
    *response.status_mut() = status;
    let h = response.headers_mut();
    h.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_str(&asset.content_type).map_err(|e| internal(e.into()))?,
    );
    h.insert(
        header::CONTENT_LENGTH,
        HeaderValue::from_str(&len.to_string()).unwrap(),
    );
    h.insert(header::ACCEPT_RANGES, HeaderValue::from_static("bytes"));
    h.insert(header::ETAG, HeaderValue::from_str(&etag).unwrap());
    if status == StatusCode::PARTIAL_CONTENT {
        h.insert(
            header::CONTENT_RANGE,
            HeaderValue::from_str(&format!("bytes {start}-{end}/{actual}")).unwrap(),
        );
    }
    Ok(response)
}
async fn artwork(
    State(s): State<AppState>,
    Auth(auth): Auth,
    Path(id): Path<String>,
) -> Result<Response> {
    let path = s
        .media
        .artwork(&auth.response.user.id, &id)
        .await?
        .ok_or(HttpError(StatusCode::NOT_FOUND, "Artwork not found".into()))?;
    let mime = if path.extension().and_then(|s| s.to_str()) == Some("png") {
        "image/png"
    } else {
        "image/jpeg"
    };
    let permit = s
        .streams
        .clone()
        .try_acquire_owned()
        .map_err(|_| HttpError(StatusCode::TOO_MANY_REQUESTS, "Stream limit reached".into()))?;
    let file = tokio::fs::File::open(path).await?;
    let reader = ReaderStream::new(file);
    let body = futures_util::stream::unfold((reader, permit), |(mut reader, permit)| async move {
        reader.next().await.map(|chunk| (chunk, (reader, permit)))
    });
    Ok(([(header::CONTENT_TYPE, mime)], Body::from_stream(body)).into_response())
}
#[derive(Deserialize)]
struct FavoriteRequest {
    favorite: bool,
}
async fn favorite(
    State(s): State<AppState>,
    WriteAuth(auth): WriteAuth,
    Path(id): Path<String>,
    Json(input): Json<FavoriteRequest>,
) -> Result<StatusCode> {
    if !s
        .media
        .set_favorite(&auth.response.user.id, &id, input.favorite)
        .await?
    {
        return Err(HttpError(StatusCode::NOT_FOUND, "Track not found".into()));
    }
    Ok(StatusCode::NO_CONTENT)
}

fn import_error(e: anyhow::Error) -> HttpError {
    // Never reflect origin URLs, secrets, resolver errors or internal paths.
    if let Some(safe) = e.downcast_ref::<vela_imports::ImportError>() {
        return HttpError(StatusCode::BAD_REQUEST, safe.to_string());
    }
    invalid(
        "Import unavailable. Use a public direct audio URL on port 80 or 443 and check file limits.",
    )
}
async fn import_preview(
    State(s): State<AppState>,
    WriteAuth(_): WriteAuth,
    Json(input): Json<ImportRequest>,
) -> Result<Json<ImportPreview>> {
    Ok(Json(
        s.imports.preview(&input.url).await.map_err(import_error)?,
    ))
}
async fn import_create(
    State(s): State<AppState>,
    WriteAuth(auth): WriteAuth,
    Json(input): Json<ImportRequest>,
) -> Result<Json<ImportJob>> {
    Ok(Json(
        s.imports
            .enqueue(&auth.response.user.id, &input.url)
            .await
            .map_err(import_error)?,
    ))
}
async fn imports(State(s): State<AppState>, Auth(auth): Auth) -> Result<Json<Vec<ImportJob>>> {
    Ok(Json(s.imports.list(&auth.response.user.id).await?))
}
async fn import_cancel(
    State(s): State<AppState>,
    WriteAuth(auth): WriteAuth,
    Path(id): Path<String>,
) -> Result<StatusCode> {
    if s.imports.cancel(&auth.response.user.id, &id).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(HttpError(
            StatusCode::CONFLICT,
            "Job cannot be cancelled after processing begins".into(),
        ))
    }
}
async fn import_retry(
    State(s): State<AppState>,
    WriteAuth(auth): WriteAuth,
    Path(id): Path<String>,
) -> Result<StatusCode> {
    if s.imports.retry(&auth.response.user.id, &id).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(invalid("Job cannot be retried"))
    }
}
async fn playlists(
    State(s): State<AppState>,
    Auth(auth): Auth,
) -> Result<Json<Vec<PlaylistSummary>>> {
    Ok(Json(s.playlists.list(&auth.response.user).await?))
}
async fn playlist_create(
    State(s): State<AppState>,
    WriteAuth(auth): WriteAuth,
    Json(input): Json<CreatePlaylist>,
) -> Result<Json<PlaylistSummary>> {
    Ok(Json(s.playlists.create(&auth.response.user, input).await?))
}
async fn playlist(
    State(s): State<AppState>,
    Auth(auth): Auth,
    Path(id): Path<String>,
) -> Result<Json<PlaylistDetail>> {
    Ok(Json(s.playlists.detail(&auth.response.user, &id).await?))
}
async fn playlist_add(
    State(s): State<AppState>,
    WriteAuth(auth): WriteAuth,
    Path(id): Path<String>,
    Json(input): Json<PlaylistMutation>,
) -> Result<Json<PlaylistDetail>> {
    Ok(Json(
        s.playlists.add(&auth.response.user, &id, input).await?,
    ))
}
async fn playlist_order(
    State(s): State<AppState>,
    WriteAuth(auth): WriteAuth,
    Path(id): Path<String>,
    Json(input): Json<PlaylistMutation>,
) -> Result<Json<PlaylistDetail>> {
    Ok(Json(
        s.playlists.reorder(&auth.response.user, &id, input).await?,
    ))
}
#[derive(Deserialize)]
struct RemoveQuery {
    revision: i64,
    mutation_id: String,
}
async fn playlist_remove(
    State(s): State<AppState>,
    WriteAuth(auth): WriteAuth,
    Path((id, item)): Path<(String, String)>,
    Query(input): Query<RemoveQuery>,
) -> Result<Json<PlaylistDetail>> {
    Ok(Json(
        s.playlists
            .remove(
                &auth.response.user,
                &id,
                &item,
                PlaylistMutation {
                    revision: input.revision,
                    mutation_id: input.mutation_id,
                    track_id: None,
                    item_ids: None,
                },
            )
            .await?,
    ))
}
async fn collaborator(
    State(s): State<AppState>,
    WriteAuth(auth): WriteAuth,
    Path(id): Path<String>,
    Json(input): Json<CollaboratorRequest>,
) -> Result<StatusCode> {
    s.playlists
        .collaborate(&auth.response.user, &id, input)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
async fn collaborator_revoke(
    State(s): State<AppState>,
    WriteAuth(auth): WriteAuth,
    Path((id, user)): Path<(String, String)>,
) -> Result<StatusCode> {
    s.playlists
        .revoke_collaborator(&auth.response.user, &id, &user)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
async fn share(
    State(s): State<AppState>,
    WriteAuth(auth): WriteAuth,
    Path(id): Path<String>,
    Json(input): Json<ShareRequest>,
) -> Result<Json<ShareResponse>> {
    Ok(Json(
        s.playlists.share(&auth.response.user, &id, input).await?,
    ))
}
async fn share_revoke(
    State(s): State<AppState>,
    WriteAuth(auth): WriteAuth,
    Path(id): Path<String>,
) -> Result<StatusCode> {
    s.playlists.revoke_shares(&auth.response.user, &id).await?;
    Ok(StatusCode::NO_CONTENT)
}
async fn shared(
    State(s): State<AppState>,
    Path(token): Path<String>,
) -> Result<Json<PlaylistDetail>> {
    Ok(Json(s.playlists.shared(&token).await?))
}

async fn collaborators(
    State(s): State<AppState>,
    Auth(auth): Auth,
    Path(id): Path<String>,
) -> Result<Json<Vec<PlaylistCollaborator>>> {
    Ok(Json(
        s.playlists.collaborators(&auth.response.user, &id).await?,
    ))
}
