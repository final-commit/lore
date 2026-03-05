use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    response::Redirect,
    Json,
};
use serde::Deserialize;
use crate::auth::AuthUser;
use crate::error::AppError;
use crate::state::AppState;

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Extract the best available client-IP for rate limiting (mirrors auth.rs).
fn rate_limit_key(headers: &HeaderMap) -> String {
    headers
        .get("x-forwarded-for")
        .or_else(|| headers.get("x-real-ip"))
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(',').next())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

// ── Provider management ───────────────────────────────────────────────────────

pub async fn list_providers(
    State(state): State<AppState>,
) -> Result<Json<Vec<crate::oauth::OAuthProvider>>, AppError> {
    Ok(Json(state.oauth.list_enabled_providers().await?))
}

pub async fn list_all_providers(
    State(state): State<AppState>,
    _user: AuthUser,
) -> Result<Json<Vec<crate::oauth::OAuthProvider>>, AppError> {
    Ok(Json(state.oauth.list_providers().await?))
}

#[derive(Deserialize)]
pub struct ConfigureProvider {
    pub client_id: String,
    pub client_secret: String,
    pub enabled: bool,
}

pub async fn configure_provider(
    State(state): State<AppState>,
    user: AuthUser,
    Path(provider): Path<String>,
    Json(req): Json<ConfigureProvider>,
) -> Result<Json<crate::oauth::OAuthProvider>, AppError> {
    if !user.is_admin() {
        return Err(AppError::Forbidden("admin only".into()));
    }
    Ok(Json(
        state.oauth.configure_provider(&provider, &req.client_id, &req.client_secret, req.enabled).await?,
    ))
}

// ── OAuth flow ────────────────────────────────────────────────────────────────

pub async fn oauth_redirect(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(provider): Path<String>,
) -> Result<Redirect, AppError> {
    // Rate limiting — same budget as login (20 req/min per IP)
    let key = rate_limit_key(&headers);
    if !state.rate_limiter.check(&format!("oauth_redirect:{key}")).await {
        return Err(AppError::TooManyRequests("too many OAuth requests".into()));
    }

    let prov = state.oauth.get_provider(&provider).await?;
    if !prov.enabled {
        return Err(AppError::NotFound("provider not enabled".into()));
    }
    let client_id = prov.client_id.ok_or_else(|| AppError::Internal("provider not configured".into()))?;
    let redirect_uri = format!("{}/api/auth/oauth/{provider}/callback", state.config.base_url);

    // Generate CSRF state token and store in cache (10-minute TTL)
    let csrf_state = state.oauth.generate_state(&provider).await;

    let auth_url = format!(
        "{}?client_id={}&redirect_uri={}&response_type=code&scope=openid+email+profile&access_type=offline&state={}",
        prov.auth_url,
        urlencoding::encode(&client_id),
        urlencoding::encode(&redirect_uri),
        urlencoding::encode(&csrf_state),
    );

    Ok(Redirect::temporary(&auth_url))
}

#[derive(Deserialize)]
pub struct OAuthCallback {
    pub code: Option<String>,
    pub state: Option<String>,
    pub error: Option<String>,
}

pub async fn oauth_callback(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(provider): Path<String>,
    Query(q): Query<OAuthCallback>,
) -> Result<axum::response::Response, AppError> {
    use axum::response::IntoResponse;

    // Rate limiting — OAuth callback is sensitive; same budget as login
    let key = rate_limit_key(&headers);
    if !state.rate_limiter.check(&format!("oauth_callback:{key}")).await {
        return Err(AppError::TooManyRequests("too many OAuth requests".into()));
    }

    if let Some(err) = q.error {
        return Ok(Redirect::temporary(&format!("/login?error={}", urlencoding::encode(&err))).into_response());
    }

    let code = q.code.ok_or_else(|| AppError::BadRequest("missing code".into()))?;

    // Verify CSRF state parameter — reject if missing or invalid/expired
    let csrf_state = q.state.ok_or_else(|| AppError::BadRequest("missing state parameter".into()))?;
    if !state.oauth.verify_and_consume_state(&provider, &csrf_state).await {
        return Err(AppError::BadRequest("invalid or expired OAuth state".into()));
    }

    let redirect_uri = format!("{}/api/auth/oauth/{provider}/callback", state.config.base_url);
    let user_info = state.oauth.exchange_code(&provider, &code, &redirect_uri).await?;

    // Find or create user
    let db = state.db.clone();
    let email = user_info.email.clone();
    let name = user_info.name.clone().unwrap_or_else(|| {
        email.split('@').next().unwrap_or("User").to_string()
    });
    let access_token = {
        use crate::db::with_conn;
        use chrono::Utc;
        use uuid::Uuid;
        let email2 = email.clone();
        let name2 = name.clone();
        let now = Utc::now().to_rfc3339();
        let (user_id, user_role) = with_conn(&db, move |conn| {
            let existing: Option<(String, String)> = conn.query_row(
                "SELECT id, role FROM users WHERE email=?1",
                rusqlite::params![email2],
                |r| Ok((r.get(0)?, r.get(1)?)),
            ).ok();
            if let Some(pair) = existing {
                return Ok(pair);
            }
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
