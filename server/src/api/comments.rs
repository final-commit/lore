use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;

use crate::auth::AuthUser;
use crate::comments::{Comment, CreateComment};
use crate::error::AppError;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct ListParams {
    pub doc_path: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateCommentRequest {
    pub doc_path: String,
    pub parent_id: Option<String>,
    pub body: String,
    pub anchor_text: Option<String>,
    pub anchor_start: Option<i64>,
    pub anchor_end: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateCommentRequest {
    pub body: String,
}

const MAX_COMMENT_BODY: usize = 10_000;

/// GET /api/comments?doc_path=...
pub async fn list_comments(
    State(state): State<AppState>,
    _user: AuthUser,
    Query(params): Query<ListParams>,
) -> Result<Json<Vec<Comment>>, AppError> {
    let comments = state.comments.list(&params.doc_path).await?;
    Ok(Json(comments))
}

/// POST /api/comments
pub async fn create_comment(
    State(state): State<AppState>,
    user: AuthUser,
    Json(req): Json<CreateCommentRequest>,
) -> Result<(StatusCode, Json<Comment>), AppError> {
    if req.body.is_empty() {
        return Err(AppError::BadRequest("comment body must not be empty".into()));
    }
    if req.body.len() > MAX_COMMENT_BODY {
        return Err(AppError::BadRequest(
            format!("comment body must not exceed {MAX_COMMENT_BODY} characters"),
        ));
    }
    let comment = state
        .comments
        .create(CreateComment {
            doc_path: req.doc_path,
            parent_id: req.parent_id,
            author_id: user.id.clone(),
            body: req.body,
            anchor_text: req.anchor_text,
            anchor_start: req.anchor_start,
            anchor_end: req.anchor_end,
            is_agent: false,
        })
        .await?;
    Ok((StatusCode::CREATED, Json(comment)))
}

/// PUT /api/comments/:id
pub async fn update_comment(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<String>,
    Json(req): Json<UpdateCommentRequest>,
) -> Result<Json<Comment>, AppError> {
    if req.body.is_empty() {
        return Err(AppError::BadRequest("comment body must not be empty".into()));
    }
    if req.body.len() > MAX_COMMENT_BODY {
        return Err(AppError::BadRequest(
            format!("comment body must not exceed {MAX_COMMENT_BODY} characters"),
        ));
    }
    let comment = state.comments.get(&id).await?;
    if comment.author_id != user.id && !user.is_admin() {
        return Err(AppError::Forbidden("you can only edit your own comments".into()));
    }
    let updated = state.comments.update_body(&id, &req.body).await?;
    Ok(Json(updated))
}

/// DELETE /api/comments/:id
pub async fn delete_comment(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<String>,
) -> Result<StatusCode, AppError> {
    let comment = state.comments.get(&id).await?;
    if comment.author_id != user.id && !user.is_admin() {
        return Err(AppError::Forbidden("you can only delete your own comments".into()));
    }
    state.comments.delete(&id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/comments/:id/resolve
pub async fn resolve_comment(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<Comment>, AppError> {
    let resolved = state.comments.resolve(&id, &user.id).await?;
    Ok(Json(resolved))
}
