use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    Json,
};

use crate::auth::{
    handler::{
        AuthResponse, CreateApiTokenRequest, CreateApiTokenResponse, LoginRequest, RefreshRequest,
        RegisterRequest, UserInfo,
    },
    AuthUser,
};
use crate::error::AppError;
use crate::state::AppState;

/// Extract the best available client-IP key for rate limiting.
/// Falls back to "unknown" when behind a proxy that doesn't set headers.
fn rate_limit_key(headers: &HeaderMap) -> String {
    headers
        .get("x-forwarded-for")
        .or_else(|| headers.get("x-real-ip"))
        .and_then(|v| v.to_str().ok())
        // X-Forwarded-For may be a comma-separated list; take only the first entry.
        .and_then(|v| v.split(',').next())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

/// POST /api/auth/register
pub async fn register(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<RegisterRequest>,
) -> Result<(StatusCode, Json<AuthResponse>), AppError> {
    let key = rate_limit_key(&headers);
    if !state.rate_limiter.check(&format!("register:{key}")).await {
        return Err(AppError::TooManyRequests("too many registration attempts".into()));
    }
    let resp = state.auth.register(req).await?;
    Ok((StatusCode::CREATED, Json(resp)))
}

/// POST /api/auth/login
pub async fn login(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<LoginRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    let key = rate_limit_key(&headers);
    if !state.rate_limiter.check(&format!("login:{key}")).await {
        return Err(AppError::TooManyRequests("too many login attempts".into()));
    }
    let resp = state.auth.login(req).await?;
    Ok(Json(resp))
}

/// POST /api/auth/refresh
pub async fn refresh(
    State(state): State<AppState>,
    Json(req): Json<RefreshRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    let resp = state.auth.refresh(req).await?;
    Ok(Json(resp))
}

/// GET /api/auth/me
pub async fn me(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<Json<UserInfo>, AppError> {
    let info = state.auth.get_me(&user.id).await?;
    Ok(Json(info))
}

/// POST /api/auth/tokens
pub async fn create_token(
    State(state): State<AppState>,
    user: AuthUser,
    Json(req): Json<CreateApiTokenRequest>,
) -> Result<(StatusCode, Json<CreateApiTokenResponse>), AppError> {
    if !user.is_admin() {
        return Err(AppError::Forbidden("admin role required to create API tokens".into()));
    }
    let resp = state.auth.create_api_token(&user.id, req).await?;
    Ok((StatusCode::CREATED, Json(resp)))
}

/// DELETE /api/auth/tokens/:id
pub async fn revoke_token(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<String>,
) -> Result<StatusCode, AppError> {
    if !user.is_admin() {
        return Err(AppError::Forbidden("admin role required to revoke API tokens".into()));
    }
    state.auth.revoke_api_token(&id).await?;
    Ok(StatusCode::NO_CONTENT)
}
