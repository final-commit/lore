use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};

use crate::auth::AuthUser;
use crate::collections::{Collection, CreateCollection, UpdateCollection};
use crate::error::AppError;
use crate::state::AppState;

/// GET /api/collections
pub async fn list_collections(
    State(state): State<AppState>,
    _user: AuthUser,
) -> Result<Json<Vec<Collection>>, AppError> {
    let list = state.collections.list().await?;
    Ok(Json(list))
}

/// POST /api/collections
pub async fn create_collection(
    State(state): State<AppState>,
    user: AuthUser,
    Json(req): Json<CreateCollection>,
) -> Result<(StatusCode, Json<Collection>), AppError> {
    if !user.is_editor_or_admin() {
        return Err(AppError::Forbidden("editor or admin required".into()));
    }
    if req.name.trim().is_empty() {
        return Err(AppError::BadRequest("name must not be empty".into()));
    }
    if req.slug.trim().is_empty() {
        return Err(AppError::BadRequest("slug must not be empty".into()));
    }
    let collection = state.collections.create(req).await?;
    Ok((StatusCode::CREATED, Json(collection)))
}

/// PUT /api/collections/{id}
pub async fn update_collection(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<String>,
    Json(req): Json<UpdateCollection>,
) -> Result<Json<Collection>, AppError> {
    if !user.is_editor_or_admin() {
        return Err(AppError::Forbidden("editor or admin required".into()));
    }
    let updated = state.collections.update(&id, req).await?;
    Ok(Json(updated))
}

/// DELETE /api/collections/{id}
pub async fn delete_collection(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<String>,
) -> Result<StatusCode, AppError> {
    if !user.is_admin() {
        return Err(AppError::Forbidden("admin required to delete collections".into()));
    }
    state.collections.delete(&id).await?;
    Ok(StatusCode::NO_CONTENT)
}
