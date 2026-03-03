use axum::{extract::{Path, Query, State}, http::{HeaderMap, StatusCode}, Json, response::Redirect};
use serde::Deserialize;
use crate::auth::AuthUser;
use crate::error::AppError;
use crate::state::AppState;

pub async fn list_providers(State(state): State<AppState>) -> Result<Json<Vec<crate::oauth::OAuthProvider>>, AppError> {
    Ok(Json(state.oauth.list_enabled_providers().await?))
}

pub async fn list_all_providers(State(state): State<AppState>, _user: AuthUser) -> Result<Json<Vec<crate::oauth::OAuthProvider>>, AppError> {
    Ok(Json(state.oauth.list_providers().await?))
}

#[derive(Deserialize)]
pub struct ConfigureProvider { pub client_id: String, pub client_secret: String, pub enabled: bool }

pub async fn configure_provider(State(state): State<AppState>, user: AuthUser, Path(provider): Path<String>, Json(req): Json<ConfigureProvider>) -> Result<Json<crate::oauth::OAuthProvider>, AppError> {
    if !user.is_admin() { return Err(AppError::Forbidden("admin only".into())); }
    Ok(Json(state.oauth.configure_provider(&provider, &req.client_id, &req.client_secret, req.enabled).await?))
}

pub async fn oauth_redirect(State(state): State<AppState>, Path(provider): Path<String>) -> Result<Redirect, AppError> {
    let prov = state.oauth.get_provider(&provider).await?;
    if !prov.enabled { return Err(AppError::NotFound("provider not enabled".into())); }
    let client_id = prov.client_id.ok_or_else(|| AppError::Internal("provider not configured".into()))?;
    let redirect_uri = format!("{}/api/auth/oauth/{provider}/callback", state.config.base_url);
    let auth_url = format!(
        "{}?client_id={}&redirect_uri={}&response_type=code&scope=openid+email+profile&access_type=offline",
        prov.auth_url,
        urlencoding::encode(&client_id),
        urlencoding::encode(&redirect_uri),
    );
    Ok(Redirect::temporary(&auth_url))
}

#[derive(Deserialize)]
pub struct OAuthCallback { pub code: Option<String>, pub error: Option<String> }

pub async fn oauth_callback(State(state): State<AppState>, Path(provider): Path<String>, Query(q): Query<OAuthCallback>) -> Result<axum::response::Response, AppError> {
    use axum::response::IntoResponse;
    if let Some(err) = q.error {
        return Ok(Redirect::temporary(&format!("/login?error={}", urlencoding::encode(&err))).into_response());
    }
    let code = q.code.ok_or_else(|| AppError::BadRequest("missing code".into()))?;
    let redirect_uri = format!("{}/api/auth/oauth/{provider}/callback", state.config.base_url);
    let user_info = state.oauth.exchange_code(&provider, &code, &redirect_uri).await?;

    // Find or create user
    let db = state.db.clone();
    let email = user_info.email.clone();
    let name = user_info.name.clone().unwrap_or_else(|| email.split('@').next().unwrap_or("User").to_string());
    let access_token = {
        use crate::db::with_conn;
        use chrono::Utc;
        use uuid::Uuid;
        let email2 = email.clone();
        let name2 = name.clone();
        let now = Utc::now().to_rfc3339();
        // Find or create user, return (user_id, role)
        let (user_id, user_role) = with_conn(&db, move |conn| {
            let existing: Option<(String, String)> = conn.query_row(
                "SELECT id, role FROM users WHERE email=?1", rusqlite::params![email2], |r| Ok((r.get(0)?, r.get(1)?))
            ).ok();
            if let Some(pair) = existing { return Ok(pair); }
            let count: i64 = conn.query_row("SELECT COUNT(*) FROM users", [], |r| r.get(0))?;
            let role = if count == 0 { "admin" } else { "editor" };
            let id = Uuid::now_v7().to_string();
            conn.execute(
                "INSERT INTO users (id, email, name, password_hash, role, created_at, updated_at) VALUES (?1,?2,?3,'oauth',?4,?5,?5)",
                rusqlite::params![id, email2, name2, role, now],
            )?;
            Ok((id, role.to_string()))
        }).await?;
        crate::auth::token::encode_access_token(&user_id, &email, &user_role, &state.config.jwt_secret)?
    };

    // Redirect to frontend with token
    Ok(Redirect::temporary(&format!("/?token={access_token}")).into_response())
}
