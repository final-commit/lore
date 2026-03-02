use axum::{extract::State, Json};
use crate::auth::AuthUser;
use crate::error::AppError;
use crate::preferences::UpdatePreferences;
use crate::state::AppState;

pub async fn get_preferences(State(state): State<AppState>, user: AuthUser) -> Result<Json<crate::preferences::UserPreferences>, AppError> {
    Ok(Json(state.preferences.get(&user.id).await?))
}

pub async fn update_preferences(State(state): State<AppState>, user: AuthUser, Json(req): Json<UpdatePreferences>) -> Result<Json<crate::preferences::UserPreferences>, AppError> {
    Ok(Json(state.preferences.update(&user.id, req).await?))
}
