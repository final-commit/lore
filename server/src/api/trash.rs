use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};

use crate::auth::AuthUser;
use crate::doc_meta::DocMeta;
use crate::error::{validate_path, AppError};
use crate::state::AppState;

/// POST /api/doc-archive/{*path}
pub async fn archive_doc(
    State(state): State<AppState>,
    user: AuthUser,
    Path(path): Path<String>,
) -> Result<Json<DocMeta>, AppError> {
    validate_path(&path)?;
    if !user.is_editor_or_admin() {
        return Err(AppError::Forbidden("editor or admin required".into()));
    }
    let meta = state.doc_meta.archive(&path, &user.id).await?;
    Ok(Json(meta))
}

/// POST /api/doc-unarchive/{*path}
pub async fn unarchive_doc(
    State(state): State<AppState>,
    user: AuthUser,
    Path(path): Path<String>,
) -> Result<Json<DocMeta>, AppError> {
    validate_path(&path)?;
    if !user.is_editor_or_admin() {
        return Err(AppError::Forbidden("editor or admin required".into()));
    }
    let meta = state.doc_meta.unarchive(&path, &user.id).await?;
    Ok(Json(meta))
}

/// POST /api/doc-trash/{*path}
pub async fn trash_doc(
    State(state): State<AppState>,
    user: AuthUser,
    Path(path): Path<String>,
) -> Result<Json<DocMeta>, AppError> {
    validate_path(&path)?;
    if !user.is_editor_or_admin() {
        return Err(AppError::Forbidden("editor or admin required".into()));
    }
    let meta = state.doc_meta.trash(&path, &user.id).await?;
    Ok(Json(meta))
}

/// POST /api/doc-restore/{*path}
pub async fn restore_doc(
    State(state): State<AppState>,
    user: AuthUser,
    Path(path): Path<String>,
) -> Result<Json<DocMeta>, AppError> {
    validate_path(&path)?;
    if !user.is_editor_or_admin() {
        return Err(AppError::Forbidden("editor or admin required".into()));
    }
    let meta = state.doc_meta.restore(&path, &user.id).await?;
    Ok(Json(meta))
}

/// DELETE /api/doc-delete/{*path}  — permanent hard delete (admin only)
pub async fn permanent_delete_doc(
    State(state): State<AppState>,
    user: AuthUser,
    Path(path): Path<String>,
) -> Result<StatusCode, AppError> {
    validate_path(&path)?;
    if !user.is_admin() {
        return Err(AppError::Forbidden("admin required for permanent deletion".into()));
    }
    // Remove the file from git.
    state
        .git
        .delete_file(&path, &format!("Permanently delete {path}"), &user.email, &user.email)
        .await
        // Ignore NotFound — file may already be gone.
        .or_else(|e| if matches!(e, AppError::NotFound(_)) { Ok(String::new()) } else { Err(e) })?;

    // Remove metadata record.
    state
        .doc_meta
        .permanent_delete(&path)
        .await
        // Ignore NotFound — metadata may not exist.
        .or_else(|e| if matches!(e, AppError::NotFound(_)) { Ok(()) } else { Err(e) })?;

    Ok(StatusCode::NO_CONTENT)
}

/// GET /api/trash
pub async fn list_trash(
    State(state): State<AppState>,
    _user: AuthUser,
) -> Result<Json<Vec<DocMeta>>, AppError> {
    let list = state.doc_meta.list_trash().await?;
    Ok(Json(list))
}

/// GET /api/archive
pub async fn list_archive(
    State(state): State<AppState>,
    _user: AuthUser,
) -> Result<Json<Vec<DocMeta>>, AppError> {
    let list = state.doc_meta.list_archive().await?;
    Ok(Json(list))
}
