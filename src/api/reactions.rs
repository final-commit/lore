use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;

use crate::auth::AuthUser;
use crate::error::AppError;
use crate::reactions::Reaction;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct ListReactionsQuery {
    pub comment_id: Option<String>,
}

#[derive(Deserialize)]
pub struct CreateReactionReq {
    pub comment_id: String,
    pub emoji: String,
}

/// POST /api/reactions — toggle reaction (add or remove)
pub async fn toggle_reaction(
    State(state): State<AppState>,
    user: AuthUser,
    Json(req): Json<CreateReactionReq>,
) -> Result<(StatusCode, Json<serde_json::Value>), AppError> {
    if req.emoji.is_empty() {
        return Err(AppError::BadRequest("emoji must not be empty".into()));
    }
    let result = state.reactions.toggle(&req.comment_id, &user.id, &req.emoji).await?;
    match result {
        Some(reaction) => Ok((StatusCode::CREATED, Json(serde_json::json!(reaction)))),
        None => Ok((StatusCode::OK, Json(serde_json::json!({ "removed": true })))),
    }
}

/// GET /api/reactions?comment_id=
pub async fn list_reactions(
    State(state): State<AppState>,
    _user: AuthUser,
    Query(q): Query<ListReactionsQuery>,
) -> Result<Json<Vec<Reaction>>, AppError> {
    let comment_id = q.comment_id.ok_or_else(|| AppError::BadRequest("comment_id is required".into()))?;
    let reactions = state.reactions.list_for_comment(&comment_id).await?;
    Ok(Json(reactions))
}

/// DELETE /api/reactions/{id}
pub async fn delete_reaction(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<String>,
) -> Result<StatusCode, AppError> {
    state.reactions.delete(&id, &user.id).await?;
    Ok(StatusCode::NO_CONTENT)
}
