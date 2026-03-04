use axum::{
    extract::{Multipart, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use crate::error::AppError;
use crate::import::ImportResult;
use crate::state::AppState;
use crate::auth::middleware::resolve_token;

pub async fn import_outline(
    State(state): State<AppState>,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> Result<(StatusCode, Json<ImportResult>), AppError> {
    let user_email = {
        let token = headers
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .ok_or_else(|| AppError::Unauthorized("missing Authorization header".into()))?;
        let user = resolve_token(token, &state.config.jwt_secret, &state.db).await?;
        if !user.is_admin() { return Err(AppError::Forbidden("admin only".into())); }
        user.email
    };

    let mut data = None;
    while let Some(field) = multipart.next_field().await.map_err(|e| AppError::BadRequest(e.to_string()))? {
        if field.name() == Some("file") {
            data = Some(field.bytes().await.map_err(|e| AppError::BadRequest(e.to_string()))?);
        }
    }
    let bytes = data.ok_or_else(|| AppError::BadRequest("missing file field".into()))?;
    let result = state.import.from_outline_json(&bytes, &user_email).await?;
    Ok((StatusCode::OK, Json(result)))
}

pub async fn import_markdown(
    State(state): State<AppState>,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> Result<(StatusCode, Json<ImportResult>), AppError> {
    let user_email = {
        let token = headers
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .ok_or_else(|| AppError::Unauthorized("missing Authorization header".into()))?;
        let user = resolve_token(token, &state.config.jwt_secret, &state.db).await?;
        if !user.is_admin() { return Err(AppError::Forbidden("admin only".into())); }
        user.email
    };

    let mut data = None;
    while let Some(field) = multipart.next_field().await.map_err(|e| AppError::BadRequest(e.to_string()))? {
        if field.name() == Some("file") {
            data = Some(field.bytes().await.map_err(|e| AppError::BadRequest(e.to_string()))?);
        }
    }
    let bytes = data.ok_or_else(|| AppError::BadRequest("missing file field".into()))?;
    let result = state.import.from_markdown_zip(&bytes, &user_email).await?;
    Ok((StatusCode::OK, Json(result)))
}
