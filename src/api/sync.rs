use axum::{extract::State, Json};

use crate::auth::AuthUser;
use crate::error::AppError;
use crate::state::AppState;
use crate::sync::{SyncResult, SyncStatus};

/// GET /api/sync/status
pub async fn sync_status(
    State(state): State<AppState>,
) -> Result<Json<SyncStatus>, AppError> {
    let status = state.sync.status().await?;
    Ok(Json(status))
}

/// POST /api/sync/pull
pub async fn sync_pull(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<Json<SyncResult>, AppError> {
    if !user.is_editor_or_admin() {
        return Err(AppError::Forbidden("editor or admin role required".into()));
    }
    // Invalidate entire cache after pull (repo may have changed).
    state.cache.invalidate_all().await;
    let result = state.sync.pull().await?;
    Ok(Json(result))
}

/// POST /api/sync/push
pub async fn sync_push(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<Json<SyncResult>, AppError> {
    if !user.is_editor_or_admin() {
        return Err(AppError::Forbidden("editor or admin role required".into()));
    }
    let result = state.sync.push().await?;
    Ok(Json(result))
}
