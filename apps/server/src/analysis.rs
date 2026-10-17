//! Explicit owner-authorized analysis and bounded local radio; no stream-path decoding.
use super::*;

pub(super) fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/audio-analysis", get(summary).post(enqueue))
        .route("/api/tracks/{id}/analysis", get(detail))
        .route("/api/tracks/{id}/radio", get(radio))
        .layer(DefaultBodyLimit::max(16 * 1024))
}
fn analysis_error(error: anyhow::Error) -> HttpError {
    use vela_media::analysis_service::AnalysisError;
    match error.downcast_ref::<AnalysisError>() {
        Some(AnalysisError::Invalid(message)) => invalid(message),
        Some(AnalysisError::Conflict(message)) => HttpError(StatusCode::CONFLICT, message.clone()),
        Some(AnalysisError::NotFound) => not_found(),
        None => error.into(),
    }
}
async fn summary(
    State(s): State<AppState>,
    Auth(auth): Auth,
) -> Result<Json<AnalysisLibraryStatus>> {
    Ok(Json(
        s.analysis
            .summary(&auth.response.user.id)
            .await
            .map_err(analysis_error)?,
    ))
}
async fn enqueue(
    State(s): State<AppState>,
    WriteAuth(auth): WriteAuth,
    input: std::result::Result<Json<EnqueueAnalysis>, axum::extract::rejection::JsonRejection>,
) -> Result<Json<AnalysisReceipt>> {
    let Json(input) = input.map_err(|_| invalid("Invalid audio analysis selection"))?;
    Ok(Json(
        s.analysis
            .enqueue(&auth.response.user.id, input)
            .await
            .map_err(analysis_error)?,
    ))
}
async fn detail(
    State(s): State<AppState>,
    Auth(auth): Auth,
    Path(id): Path<String>,
) -> Result<Json<TrackAnalysis>> {
    Ok(Json(
        s.analysis
            .get(&auth.response.user.id, &id)
            .await
            .map_err(analysis_error)?
            .ok_or_else(not_found)?,
    ))
}
async fn radio(
    State(s): State<AppState>,
    Auth(auth): Auth,
    Path(id): Path<String>,
    query: std::result::Result<Query<RadioQuery>, axum::extract::rejection::QueryRejection>,
) -> Result<Json<TrackRadio>> {
    let Query(query) = query.map_err(|_| invalid("Invalid radio query"))?;
    let result = s
        .media
        .track_radio(&auth.response.user.id, &id, query.limit.unwrap_or(20))
        .await
        .map_err(
            |error| match error.downcast_ref::<vela_media::DiscoveryError>() {
                Some(vela_media::DiscoveryError::Invalid) => {
                    invalid("Radio size must be 1–50 and the seed must be a valid track UUID")
                }
                Some(vela_media::DiscoveryError::NotFound) => not_found(),
                None => error.into(),
            },
        )?;
    Ok(Json(result))
}
