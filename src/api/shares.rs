use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;

use crate::auth::AuthUser;
use crate::error::{validate_path, AppError};
use crate::shares::Share;
use crate::shares::engine::CreateShare;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct ListSharesQuery {
    pub doc_path: Option<String>,
}

/// POST /api/shares
pub async fn create_share(
    State(state): State<AppState>,
    user: AuthUser,
    Json(req): Json<CreateShare>,
) -> Result<(StatusCode, Json<Share>), AppError> {
    if !user.is_editor_or_admin() {
        return Err(AppError::Forbidden("editor or admin required".into()));
    }
    validate_path(&req.doc_path)?;
    let share = state.shares.create(req, &user.id).await?;
    Ok((StatusCode::CREATED, Json(share)))
}

/// GET /api/shares?doc_path=
pub async fn list_shares(
    State(state): State<AppState>,
    user: AuthUser,
    Query(q): Query<ListSharesQuery>,
) -> Result<Json<Vec<Share>>, AppError> {
    if !user.is_editor_or_admin() {
        return Err(AppError::Forbidden("editor or admin required".into()));
    }
    let shares = match q.doc_path {
        Some(ref path) => state.shares.list_for_doc(path).await?,
        None => state.shares.list_all().await?,
    };
    Ok(Json(shares))
}

/// DELETE /api/shares/{id}
pub async fn delete_share(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<String>,
) -> Result<StatusCode, AppError> {
    if !user.is_editor_or_admin() {
        return Err(AppError::Forbidden("editor or admin required".into()));
    }
    state.shares.delete(&id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// GET /api/shares/view/{url_id} — PUBLIC (no auth required)
/// Returns the document content for the share.
pub async fn view_share(
    State(state): State<AppState>,
    Path(url_id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let share = state.shares.get_by_url_id(&url_id).await?;
    // Read the document content from git.
    let doc = state.git.read_file(&share.doc_path).await.map_err(|_| {
        AppError::NotFound(format!("document {} not found", share.doc_path))
    })?;
    Ok(Json(serde_json::json!({
        "share": share,
        "content": doc,
    })))
}
