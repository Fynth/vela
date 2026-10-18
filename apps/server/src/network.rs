//! Explicit owner publication, anonymous permitted reads and bounded peer inspection.
use super::*;
use reqwest::Method as RemoteMethod;

pub(super) fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/network/settings", get(settings).put(update_settings))
        .route("/api/network/preview", post(preview))
        .route("/api/publications", get(owned).post(create))
        .route("/api/publications/{id}/revoke", post(revoke))
        .route("/.well-known/vela", get(capabilities))
        .route("/api/public/catalog", get(catalog))
        .route("/api/public/catalog/{id}", get(detail))
        .route("/api/public/media/{id}/stream", get(public_stream))
        .route("/api/public/media/{id}/download", get(download))
        .layer(DefaultBodyLimit::max(16 * 1024))
}
fn policy_error(error: anyhow::Error) -> HttpError {
    match error.downcast_ref::<vela_media::PublicationError>() {
        Some(vela_media::PublicationError::Invalid) => invalid("Invalid publication request"),
        Some(vela_media::PublicationError::NotFound) => not_found(),
        Some(vela_media::PublicationError::Conflict) => HttpError(
            StatusCode::CONFLICT,
            "Publication changed, already exists, is disabled or quota is reached; refresh before retrying".into(),
        ),
        None => error.into(),
    }
}

async fn settings(State(s): State<AppState>, Auth(_): Auth) -> Result<Json<NetworkSettings>> {
    Ok(Json(s.network.settings().await?))
}
async fn update_settings(
    State(s): State<AppState>,
    WriteAuth(a): WriteAuth,
    input: std::result::Result<
        Json<UpdateNetworkSettings>,
        axum::extract::rejection::JsonRejection,
    >,
) -> Result<Json<NetworkSettings>> {
    let Json(input) = input.map_err(|_| invalid("Invalid public server settings"))?;
    Ok(Json(s.network.update(&a.response.user, input).await?))
}
fn query(
    input: std::result::Result<Query<PublicationQuery>, axum::extract::rejection::QueryRejection>,
) -> Result<PublicationQuery> {
    Ok(input
        .map_err(|_| invalid("Invalid public catalog query"))?
        .0)
}
async fn owned(
    State(s): State<AppState>,
    Auth(a): Auth,
    q: std::result::Result<Query<PublicationQuery>, axum::extract::rejection::QueryRejection>,
) -> Result<Json<PublicationPage>> {
    Ok(Json(
        s.media
            .publications(&a.response.user.id, &query(q)?)
            .await
            .map_err(policy_error)?,
    ))
}
async fn create(
    State(s): State<AppState>,
    WriteAuth(a): WriteAuth,
    input: std::result::Result<Json<CreatePublication>, axum::extract::rejection::JsonRejection>,
) -> Result<Json<Publication>> {
    let Json(input) = input.map_err(|_| invalid("Invalid publication request"))?;
    Ok(Json(
        s.media
            .create_publication(&a.response.user.id, input)
            .await
            .map_err(policy_error)?,
    ))
}
async fn revoke(
    State(s): State<AppState>,
    WriteAuth(a): WriteAuth,
    Path(id): Path<String>,
    input: std::result::Result<Json<RevokePublication>, axum::extract::rejection::JsonRejection>,
) -> Result<Json<Publication>> {
    let Json(input) = input.map_err(|_| invalid("Invalid publication revision"))?;
    Ok(Json(
        s.media
            .revoke_publication(&a.response.user.id, &id, input.revision)
            .await
            .map_err(policy_error)?,
    ))
}
async fn capabilities(State(s): State<AppState>) -> Result<Json<NetworkCapabilities>> {
    let settings = s.network.settings().await?;
    if !settings.enabled {
        return Err(not_found());
    }
    let origin = settings
        .origin
        .filter(|v| canonical_public_origin(v).is_ok_and(|canonical| canonical == *v))
        .ok_or_else(not_found)?;
    Ok(Json(NetworkCapabilities {
        protocol_version: 1,
        server_id: settings.server_id,
        origin,
        name: settings.name,
        software_version: env!("CARGO_PKG_VERSION").into(),
        public_artwork: false,
    }))
}
async fn catalog(
    State(s): State<AppState>,
    q: std::result::Result<Query<PublicationQuery>, axum::extract::rejection::QueryRejection>,
) -> Result<Json<PublicCatalog>> {
    // Disabled discovery has one response even for malformed filters.
    if !s.network.settings().await?.enabled {
        return Err(not_found());
    }
    Ok(Json(
        s.media
            .public_catalog(&query(q)?)
            .await
            .map_err(policy_error)?,
    ))
}
async fn detail(State(s): State<AppState>, Path(id): Path<String>) -> Result<Json<PublicTrack>> {
    Ok(Json(s.media.public_track(&id).await.map_err(policy_error)?))
}
async fn public_stream(
    State(s): State<AppState>,
    Path(id): Path<String>,
    method: Method,
    headers: HeaderMap,
) -> Result<Response> {
    let asset = s
        .media
        .public_asset(&id, false)
        .await
        .map_err(policy_error)?;
    serve_asset(&s, asset, method, headers, true).await
}
async fn download(
    State(s): State<AppState>,
    Path(id): Path<String>,
    method: Method,
    headers: HeaderMap,
) -> Result<Response> {
    let asset = s
        .media
        .public_asset(&id, true)
        .await
        .map_err(policy_error)?;
    let mut response = serve_asset(&s, asset, method, headers, true).await?;
    response.headers_mut().insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_str(&format!("attachment; filename=\"vela-{id}.audio\""))
            .map_err(|e| internal(e.into()))?,
    );
    Ok(response)
}
async fn preview(
    State(s): State<AppState>,
    WriteAuth(_): WriteAuth,
    input: std::result::Result<Json<PreviewPeer>, axum::extract::rejection::JsonRejection>,
) -> Result<Json<PeerPreview>> {
    let Json(input) = input.map_err(|_| invalid("Invalid public server origin"))?;
    if input
        .cursor
        .as_deref()
        .is_some_and(|cursor| cursor.is_empty() || cursor.len() > 64)
    {
        return Err(invalid("Invalid public server cursor"));
    }
    Ok(Json(
        s.peers
            .preview(&input.origin, input.cursor.as_deref())
            .await?,
    ))
}

#[derive(Clone)]
pub(crate) struct PeerClient {
    slots: Arc<Semaphore>,
    #[cfg(test)]
    fixture: Option<(String, SocketAddr)>,
}
impl Default for PeerClient {
    fn default() -> Self {
        Self {
            slots: Arc::new(Semaphore::new(2)),
            #[cfg(test)]
            fixture: None,
        }
    }
}
impl PeerClient {
    async fn preview(&self, input: &str, cursor: Option<&str>) -> Result<PeerPreview> {
        let origin = canonical_public_origin(input).map_err(invalid)?;
        if let Some(cursor) = cursor
            && !canonical_id(cursor)
        {
            return Err(remote_error());
        }
        let _permit = self.slots.clone().try_acquire_owned().map_err(|_| {
            HttpError(
                StatusCode::TOO_MANY_REQUESTS,
                "Remote preview limit reached".into(),
            )
        })?;
        tokio::time::timeout(Duration::from_secs(5), async {
            let caps: NetworkCapabilities =
                self.json(&origin, "/.well-known/vela", 16 * 1024).await?;
            if caps.protocol_version != 1
                || caps.origin != origin
                || !canonical_id(&caps.server_id)
                || caps.public_artwork
                || !safe_text(&caps.name, 80, false)
                || caps.name.trim().is_empty()
                || !safe_text(&caps.software_version, 64, false)
            {
                return Err(remote_error());
            }
            let mut catalog_path = String::from("/api/public/catalog?limit=20");
            if let Some(cursor) = cursor {
                catalog_path.push_str("&cursor=");
                catalog_path.push_str(cursor);
            }
            let catalog: PublicCatalog = self.json(&origin, &catalog_path, 128 * 1024).await?;
            if catalog.origin != origin
                || catalog.server_id != caps.server_id
                || catalog.items.len() > 20
                || catalog
                    .next_cursor
                    .as_ref()
                    .is_some_and(|id| !canonical_id(id))
            {
                return Err(remote_error());
            }
            if catalog
                .items
                .windows(2)
                .any(|rows| rows[0].id >= rows[1].id)
                || cursor.is_some_and(|cursor| {
                    catalog
                        .items
                        .first()
                        .is_some_and(|track| track.id.as_str() <= cursor)
                })
                || catalog.next_cursor.as_ref().is_some_and(|cursor| {
                    catalog.items.last().is_none_or(|track| track.id != *cursor)
                })
            {
                return Err(remote_error());
            }
            let mut ids = std::collections::HashSet::new();
            for track in &catalog.items {
                if !canonical_id(&track.id)
                    || !ids.insert(&track.id)
                    || track.revision < 1
                    || !(1..=604800000).contains(&track.duration_ms)
                    || track.can_download && !track.can_stream
                    || track.year.is_some_and(|year| !(1..=9999).contains(&year))
                    || !safe_text(&track.title, 512, false)
                    || track.title.trim().is_empty()
                    || !safe_text(&track.artist, 512, false)
                    || !safe_text(&track.album, 512, false)
                    || !safe_text(&track.genre, 256, false)
                    || !safe_text(&track.codec, 64, false)
                    || !safe_text(&track.attribution, 2000, true)
                    || track.attribution.trim().is_empty()
                {
                    return Err(remote_error());
                }
            }
            Ok(PeerPreview {
                capabilities: caps,
                catalog,
            })
        })
        .await
        .map_err(|_| {
            HttpError(
                StatusCode::GATEWAY_TIMEOUT,
                "Public server preview timed out".into(),
            )
        })?
    }
    async fn json<T: serde::de::DeserializeOwned>(
        &self,
        origin: &str,
        path: &str,
        limit: usize,
    ) -> Result<T> {
        let url = url::Url::parse(&format!("{origin}{path}")).map_err(|_| remote_error())?;
        let response = self.request(url).await?;
        if response.status() != reqwest::StatusCode::OK
            || response
                .content_length()
                .is_some_and(|len| len > limit as u64)
        {
            return Err(remote_error());
        }
        let mut body = Vec::new();
        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|_| remote_error())?;
            if body.len().saturating_add(chunk.len()) > limit {
                return Err(remote_error());
            }
            body.extend_from_slice(&chunk);
        }
        serde_json::from_slice(&body).map_err(|_| remote_error())
    }
    async fn request(&self, url: url::Url) -> Result<reqwest::Response> {
        #[cfg(test)]
        if let Some((origin, address)) = &self.fixture
            && url.origin().ascii_serialization() == *origin
        {
            // Unit-test-only loopback transport. No production flag, route or constructor exposes it.
            let mut fixture = url.clone();
            fixture.set_scheme("http").map_err(|_| remote_error())?;
            fixture
                .set_host(Some(&address.ip().to_string()))
                .map_err(|_| remote_error())?;
            fixture
                .set_port(Some(address.port()))
                .map_err(|_| remote_error())?;
            return reqwest::Client::builder()
                .no_proxy()
                .redirect(reqwest::redirect::Policy::none())
                .timeout(Duration::from_secs(3))
                .build()
                .map_err(|_| remote_error())?
                .get(fixture)
                .send()
                .await
                .map_err(|_| remote_error());
        }
        let transport = vela_imports::ControlledHttp::default();
        let origin = url.origin();
        let response = transport
            .request_same_origin(url, RemoteMethod::GET)
            .await
            .map_err(|_| remote_error())?;
        if response.url().origin() != origin {
            return Err(remote_error());
        }
        Ok(response)
    }
}
fn canonical_id(id: &str) -> bool {
    uuid::Uuid::parse_str(id).is_ok_and(|v| v.to_string() == id)
}
fn safe_text(value: &str, max: usize, newline: bool) -> bool {
    value.len() <= max
        && !value
            .chars()
            .any(|c| c.is_control() && !(newline && c == '\n'))
}
fn remote_error() -> HttpError {
    HttpError(
        StatusCode::BAD_GATEWAY,
        "Public server returned an unavailable or invalid catalog".into(),
    )
}

#[cfg(test)]
mod tests;
