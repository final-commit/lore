use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;

use crate::auth::AuthUser;
use crate::error::{validate_path, AppError};
use crate::stars::Star;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct ToggleStarReq {
    pub doc_path: String,
}

/// POST /api/stars — toggle star on/off
pub async fn toggle_star(
    State(state): State<AppState>,
    user: AuthUser,
    Json(req): Json<ToggleStarReq>,
) -> Result<(StatusCode, Json<serde_json::Value>), AppError> {
    validate_path(&req.doc_path)?;
    let result = state.stars.toggle(&user.id, &req.doc_path).await?;
    match result {
        Some(star) => Ok((StatusCode::CREATED, Json(serde_json::json!(star)))),
        None => Ok((StatusCode::OK, Json(serde_json::json!({ "unstarred": true })))),
    }
}

/// GET /api/stars — list current user's starred docs
pub async fn list_stars(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<Json<Vec<Star>>, AppError> {
    let stars = state.stars.list_for_user(&user.id).await?;
    Ok(Json(stars))
}

/// DELETE /api/stars/{id}
pub async fn delete_star(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<String>,
) -> Result<StatusCode, AppError> {
    state.stars.delete(&id, &user.id).await?;
    Ok(StatusCode::NO_CONTENT)
}
