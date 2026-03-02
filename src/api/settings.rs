use axum::{extract::State, Json};
use crate::auth::AuthUser;
use crate::error::AppError;
use crate::settings::UpdateSettings;
use crate::state::AppState;

pub async fn get_settings(State(state): State<AppState>) -> Result<Json<crate::settings::TeamSettings>, AppError> {
    Ok(Json(state.settings.get().await?))
}

pub async fn update_settings(State(state): State<AppState>, user: AuthUser, Json(req): Json<UpdateSettings>) -> Result<Json<crate::settings::TeamSettings>, AppError> {
    if !user.is_admin() { return Err(AppError::Forbidden("admin only".into())); }
    Ok(Json(state.settings.update(req).await?))
}
