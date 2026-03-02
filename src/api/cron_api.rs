use axum::{extract::State, Json};
use serde_json::{json, Value};
use crate::auth::AuthUser;
use crate::error::AppError;
use crate::state::AppState;

pub async fn run_cron(State(state): State<AppState>, user: AuthUser) -> Result<Json<Value>, AppError> {
    if !user.is_admin() { return Err(AppError::Forbidden("admin only".into())); }
    let cleaned = crate::cron::run_all(state.db.clone()).await?;
    Ok(Json(json!({"cleaned": cleaned})))
}
