use axum::{
    extract::{Path, State},
    Json,
};

use crate::auth::AuthUser;
use crate::doc_meta::DocMeta;
use crate::error::{validate_path, AppError};
use crate::state::AppState;

/// POST /api/doc-publish/{*path}
pub async fn publish_doc(
    State(state): State<AppState>,
    user: AuthUser,
    Path(path): Path<String>,
) -> Result<Json<DocMeta>, AppError> {
    validate_path(&path)?;
    if !user.is_editor_or_admin() {
        return Err(AppError::Forbidden("editor or admin required".into()));
    }
    let meta = state.doc_meta.publish(&path, &user.id).await?;
    Ok(Json(meta))
}

/// POST /api/doc-unpublish/{*path}
pub async fn unpublish_doc(
    State(state): State<AppState>,
    user: AuthUser,
    Path(path): Path<String>,
) -> Result<Json<DocMeta>, AppError> {
    validate_path(&path)?;
    if !user.is_editor_or_admin() {
        return Err(AppError::Forbidden("editor or admin required".into()));
    }
    let meta = state.doc_meta.unpublish(&path, &user.id).await?;
    Ok(Json(meta))
}

/// GET /api/drafts
/// Admins see all drafts; editors see their own.
pub async fn list_drafts(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<Json<Vec<DocMeta>>, AppError> {
    let drafts = state.doc_meta.list_drafts(&user.id, user.is_admin()).await?;
    Ok(Json(drafts))
}
