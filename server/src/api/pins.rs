use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;

use crate::auth::AuthUser;
use crate::error::{validate_path, AppError};
use crate::pins::Pin;
use crate::pins::engine::{CreatePin, UpdatePin};
use crate::state::AppState;

#[derive(Deserialize)]
pub struct ListPinsQuery {
    pub collection_id: Option<String>,
}

/// POST /api/pins
pub async fn create_pin(
    State(state): State<AppState>,
    user: AuthUser,
    Json(req): Json<CreatePin>,
) -> Result<(StatusCode, Json<Pin>), AppError> {
    if !user.is_editor_or_admin() {
        return Err(AppError::Forbidden("editor or admin required".into()));
    }
    validate_path(&req.doc_path)?;
    let pin = state.pins.create(req, &user.id).await?;
    Ok((StatusCode::CREATED, Json(pin)))
}

/// GET /api/pins?collection_id=
pub async fn list_pins(
    State(state): State<AppState>,
    _user: AuthUser,
    Query(q): Query<ListPinsQuery>,
) -> Result<Json<Vec<Pin>>, AppError> {
    let pins = state.pins.list_for_collection(q.collection_id.as_deref()).await?;
    Ok(Json(pins))
}

/// DELETE /api/pins/{id}
pub async fn delete_pin(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<String>,
) -> Result<StatusCode, AppError> {
    if !user.is_editor_or_admin() {
        return Err(AppError::Forbidden("editor or admin required".into()));
    }
    state.pins.delete(&id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// PUT /api/pins/{id}
pub async fn reorder_pin(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<String>,
    Json(req): Json<UpdatePin>,
) -> Result<Json<Pin>, AppError> {
    if !user.is_editor_or_admin() {
        return Err(AppError::Forbidden("editor or admin required".into()));
    }
    let pin = state.pins.reorder(&id, req.sort_order).await?;
    Ok(Json(pin))
}
