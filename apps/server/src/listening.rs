//! Owner-private transport. Consent and idempotency belong to core.
use super::*;

pub(super) fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/listening/preferences",
            get(preferences).put(update_preferences),
        )
        .route("/api/listening/events", post(record))
        .route("/api/listening/history", get(history).delete(clear))
        .route("/api/listening/export", get(export))
        .layer(DefaultBodyLimit::max(16 * 1024))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DeviceQuery {
    device_id: String,
}

type DeviceResult =
    std::result::Result<Query<DeviceQuery>, axum::extract::rejection::QueryRejection>;
type JsonResult<T> = std::result::Result<Json<T>, axum::extract::rejection::JsonRejection>;

async fn preferences(
    State(s): State<AppState>,
    Auth(auth): Auth,
    query: DeviceResult,
) -> Result<Json<ListeningPreferences>> {
    let Query(q) = query.map_err(|_| invalid("Invalid listening device"))?;
    Ok(Json(
        s.listening
            .preferences(&auth.response.user, &q.device_id)
            .await?,
    ))
}
async fn update_preferences(
    State(s): State<AppState>,
    WriteAuth(auth): WriteAuth,
    input: JsonResult<UpdateListeningPreferences>,
) -> Result<Json<ListeningPreferences>> {
    let Json(input) = input.map_err(|_| invalid("Invalid listening preferences"))?;
    Ok(Json(
        s.listening
            .update_preferences(&auth.response.user, input)
            .await?,
    ))
}
async fn record(
    State(s): State<AppState>,
    WriteAuth(auth): WriteAuth,
    input: JsonResult<ListeningBatch>,
) -> Result<Json<ListeningReceipt>> {
    let Json(input) = input.map_err(|_| invalid("Invalid listening events"))?;
    Ok(Json(s.listening.record(&auth.response.user, input).await?))
}
async fn history(
    State(s): State<AppState>,
    Auth(auth): Auth,
    query: std::result::Result<
        Query<ListeningHistoryQuery>,
        axum::extract::rejection::QueryRejection,
    >,
) -> Result<Json<ListeningHistoryPage>> {
    let Query(q) = query.map_err(|_| invalid("Invalid listening history query"))?;
    Ok(Json(s.listening.history(&auth.response.user, &q).await?))
}
async fn export(State(s): State<AppState>, Auth(auth): Auth) -> Result<Response> {
    let export = s.listening.export(&auth.response.user).await?;
    let mut response = Json(export).into_response();
    response.headers_mut().insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_static("attachment; filename=\"vela-listening-history.json\""),
    );
    Ok(response)
}
async fn clear(
    State(s): State<AppState>,
    WriteAuth(auth): WriteAuth,
    query: DeviceResult,
) -> Result<Json<ListeningPreferences>> {
    let Query(q) = query.map_err(|_| invalid("Invalid listening device"))?;
    Ok(Json(
        s.listening.clear(&auth.response.user, &q.device_id).await?,
    ))
}
