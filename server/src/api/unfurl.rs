use axum::{extract::{Query, State}, Json};
use serde::Deserialize;
use crate::auth::AuthUser;
use crate::error::AppError;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct UnfurlQuery { pub url: String }

pub async fn unfurl_handler(
    State(state): State<AppState>,
    _user: AuthUser,
    Query(q): Query<UnfurlQuery>,
) -> Result<Json<crate::unfurl::UnfurlResult>, AppError> {
    if q.url.is_empty() { return Err(AppError::BadRequest("url required".into())); }
    Ok(Json(state.unfurl.unfurl(&q.url).await?))
}
