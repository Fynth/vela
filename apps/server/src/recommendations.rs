//! Private on-demand mixes and explicit owner recommendation preferences.
use super::*;

pub(super) fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/recommendations", get(recommendations))
        .route(
            "/api/recommendations/feedback",
            get(exclusions).put(feedback).delete(clear),
        )
        .layer(DefaultBodyLimit::max(4 * 1024))
}

fn recommendation_error(error: anyhow::Error) -> HttpError {
    use vela_media::RecommendationError;
    match error.downcast_ref::<RecommendationError>() {
        Some(RecommendationError::Invalid) => invalid("Invalid recommendation request"),
        Some(RecommendationError::NotFound) => not_found(),
        Some(RecommendationError::Conflict) => HttpError(
            StatusCode::CONFLICT,
            "Too many hidden recommendations; restore some choices first".into(),
        ),
        None => error.into(),
    }
}

async fn recommendations(
    State(s): State<AppState>,
    Auth(auth): Auth,
    query: std::result::Result<
        Query<RecommendationQuery>,
        axum::extract::rejection::QueryRejection,
    >,
) -> Result<Json<Recommendations>> {
    let Query(query) = query.map_err(|_| invalid("Invalid recommendation query"))?;
    Ok(Json(
        s.media
            .recommendations(&auth.response.user.id, query.limit.unwrap_or(12))
            .await
            .map_err(recommendation_error)?,
    ))
}

async fn feedback(
    State(s): State<AppState>,
    WriteAuth(auth): WriteAuth,
    input: std::result::Result<
        Json<RecommendationFeedback>,
        axum::extract::rejection::JsonRejection,
    >,
) -> Result<Json<RecommendationFeedbackReceipt>> {
    let Json(input) = input.map_err(|_| invalid("Invalid recommendation feedback"))?;
    Ok(Json(
        s.media
            .set_recommendation_feedback(&auth.response.user.id, input)
            .await
            .map_err(recommendation_error)?,
    ))
}

async fn clear(
    State(s): State<AppState>,
    WriteAuth(auth): WriteAuth,
) -> Result<Json<RecommendationFeedbackReceipt>> {
    Ok(Json(
        s.media
            .clear_recommendation_feedback(&auth.response.user.id)
            .await
            .map_err(recommendation_error)?,
    ))
}

async fn exclusions(
    State(s): State<AppState>,
    Auth(auth): Auth,
) -> Result<Json<RecommendationExclusions>> {
    Ok(Json(
        s.media
            .recommendation_exclusions(&auth.response.user.id)
            .await
            .map_err(recommendation_error)?,
    ))
}
