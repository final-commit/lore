use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};

use crate::auth::AuthUser;
use crate::error::AppError;
use crate::state::AppState;
use crate::templates::{CreateTemplate, Template, UpdateTemplate};

/// GET /api/templates
pub async fn list_templates(
    State(state): State<AppState>,
    _user: AuthUser,
) -> Result<Json<Vec<Template>>, AppError> {
    let list = state.templates.list().await?;
    Ok(Json(list))
}

/// POST /api/templates
pub async fn create_template(
    State(state): State<AppState>,
    user: AuthUser,
    Json(body): Json<serde_json::Value>,
) -> Result<(StatusCode, Json<Template>), AppError> {
    if !user.is_editor_or_admin() {
        return Err(AppError::Forbidden("editor or admin required".into()));
    }
    let title = body
        .get("title")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::BadRequest("title is required".into()))?
        .to_string();
    if title.trim().is_empty() {
        return Err(AppError::BadRequest("title must not be empty".into()));
    }
    let content = body
        .get("content")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let collection_id =
        body.get("collection_id").and_then(|v| v.as_str()).map(|s| s.to_string());

    let req = CreateTemplate {
        title,
        content,
        collection_id,
        created_by: user.id.clone(),
    };
    let template = state.templates.create(req).await?;
    Ok((StatusCode::CREATED, Json(template)))
}

/// PUT /api/templates/{id}
pub async fn update_template(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<String>,
    Json(req): Json<UpdateTemplate>,
) -> Result<Json<Template>, AppError> {
    if !user.is_editor_or_admin() {
        return Err(AppError::Forbidden("editor or admin required".into()));
    }
    let updated = state.templates.update(&id, req).await?;
    Ok(Json(updated))
}

/// DELETE /api/templates/{id}
pub async fn delete_template(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<String>,
) -> Result<StatusCode, AppError> {
    if !user.is_admin() {
        return Err(AppError::Forbidden("admin required to delete templates".into()));
    }
    state.templates.delete(&id).await?;
    Ok(StatusCode::NO_CONTENT)
}
