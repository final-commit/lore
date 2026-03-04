use axum::{extract::{Multipart, Path, State}, http::StatusCode, Json};
use crate::auth::AuthUser;
use crate::error::AppError;
use crate::state::AppState;

pub async fn list_emojis(State(state): State<AppState>) -> Result<Json<Vec<crate::emojis::CustomEmoji>>, AppError> {
    Ok(Json(state.emojis.list().await?))
}

pub async fn upload_emoji(State(state): State<AppState>, user: AuthUser, mut multipart: Multipart) -> Result<(StatusCode, Json<crate::emojis::CustomEmoji>), AppError> {
    if !user.is_admin() { return Err(AppError::Forbidden("admin only".into())); }
    let mut shortcode = None;
    let mut data = None;
    let mut ext = "png".to_string();
    while let Some(field) = multipart.next_field().await.map_err(|e| AppError::BadRequest(e.to_string()))? {
        match field.name() {
            Some("shortcode") => { shortcode = Some(field.text().await.map_err(|e| AppError::BadRequest(e.to_string()))?); }
            Some("file") => {
                let ct = field.content_type().unwrap_or("image/png").to_string();
                ext = if ct.contains("gif") { "gif".into() } else if ct.contains("webp") { "webp".into() } else { "png".into() };
                data = Some(field.bytes().await.map_err(|e| AppError::BadRequest(e.to_string()))?.to_vec());
            }
            _ => {}
        }
    }
    let sc = shortcode.ok_or_else(|| AppError::BadRequest("missing shortcode field".into()))?;
    let bytes = data.ok_or_else(|| AppError::BadRequest("missing file field".into()))?;
    let emoji = state.emojis.create(&sc, &user.id, bytes, &ext).await?;
    Ok((StatusCode::CREATED, Json(emoji)))
}

pub async fn delete_emoji(State(state): State<AppState>, user: AuthUser, Path(id): Path<String>) -> Result<StatusCode, AppError> {
    if !user.is_admin() { return Err(AppError::Forbidden("admin only".into())); }
    state.emojis.delete(&id).await?;
    Ok(StatusCode::NO_CONTENT)
}
