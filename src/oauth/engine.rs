use chrono::Utc;
use reqwest::Client;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use crate::db::{with_conn, DbConn};
use crate::error::AppError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthProvider {
    pub id: String,
    pub provider: String,
    pub client_id: Option<String>,
    pub enabled: bool,
    pub auth_url: String,
    pub token_url: String,
    pub userinfo_url: String,
}

#[derive(Debug, Deserialize)]
pub struct GoogleUserInfo {
    pub sub: String,
    pub email: String,
    pub name: Option<String>,
    pub picture: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
}

#[derive(Clone)]
pub struct OAuthEngine {
    db: DbConn,
    client: Client,
}

impl OAuthEngine {
    pub fn new(db: DbConn) -> Self {
        OAuthEngine {
            db,
            client: Client::builder().timeout(Duration::from_secs(10)).build().unwrap_or_default(),
        }
    }

    pub async fn list_enabled_providers(&self) -> Result<Vec<OAuthProvider>, AppError> {
        let db = self.db.clone();
        with_conn(&db, |conn| {
            let mut stmt = conn.prepare(
                "SELECT id, provider, client_id, enabled, auth_url, token_url, userinfo_url FROM oauth_providers WHERE enabled=1"
            )?;
            stmt.query_map([], row_to_provider)?.collect()
        }).await
    }

    pub async fn list_providers(&self) -> Result<Vec<OAuthProvider>, AppError> {
        let db = self.db.clone();
        with_conn(&db, |conn| {
            let mut stmt = conn.prepare(
                "SELECT id, provider, client_id, enabled, auth_url, token_url, userinfo_url FROM oauth_providers ORDER BY provider"
            )?;
            stmt.query_map([], row_to_provider)?.collect()
        }).await
    }

    pub async fn configure_provider(&self, provider: &str, client_id: &str, client_secret: &str, enabled: bool) -> Result<OAuthProvider, AppError> {
        let db = self.db.clone();
        let prov = provider.to_string();
        let cid = client_id.to_string();
        let csec = client_secret.to_string();
        let now = Utc::now().to_rfc3339();
        with_conn(&db, move |conn| {
            conn.execute(
                "UPDATE oauth_providers SET client_id=?1, client_secret=?2, enabled=?3, updated_at=?4 WHERE provider=?5",
                params![cid, csec, enabled as i64, now, prov],
            )?;
            conn.query_row(
                "SELECT id, provider, client_id, enabled, auth_url, token_url, userinfo_url FROM oauth_providers WHERE provider=?1",
                params![prov],
                row_to_provider,
            )
        }).await.map_err(|_| AppError::NotFound(format!("provider {provider} not found")))
    }

    pub async fn get_provider(&self, provider: &str) -> Result<OAuthProvider, AppError> {
        let db = self.db.clone();
        let prov = provider.to_string();
        with_conn(&db, move |conn| {
            conn.query_row(
                "SELECT id, provider, client_id, enabled, auth_url, token_url, userinfo_url FROM oauth_providers WHERE provider=?1",
                params![prov],
                row_to_provider,
            )
        }).await.map_err(|_| AppError::NotFound(format!("provider {provider} not found")))
    }

    pub async fn get_client_secret(&self, provider: &str) -> Result<String, AppError> {
        let db = self.db.clone();
        let prov = provider.to_string();
        with_conn(&db, move |conn| {
            conn.query_row(
                "SELECT client_secret FROM oauth_providers WHERE provider=?1 AND enabled=1",
                params![prov],
                |r| r.get(0),
            )
        }).await.map_err(|_| AppError::NotFound(format!("provider {provider} not configured")))
    }

    /// Exchange OAuth code for user info
    pub async fn exchange_code(&self, provider: &str, code: &str, redirect_uri: &str) -> Result<GoogleUserInfo, AppError> {
        let prov = self.get_provider(provider).await?;
        if !prov.enabled { return Err(AppError::Forbidden("provider not enabled".into())); }
        let client_id = prov.client_id.ok_or_else(|| AppError::Internal("provider not configured".into()))?;
        let client_secret = self.get_client_secret(provider).await?;

        // Exchange code for token
        let token_resp: TokenResponse = self.client
            .post(&prov.token_url)
            .form(&[
                ("code", code),
                ("client_id", &client_id),
                ("client_secret", &client_secret),
                ("redirect_uri", redirect_uri),
                ("grant_type", "authorization_code"),
            ])
            .send().await
            .map_err(|e| AppError::Internal(format!("token exchange: {e}")))?
            .json().await
            .map_err(|e| AppError::Internal(format!("token parse: {e}")))?;

        // Fetch user info
        let user_info: GoogleUserInfo = self.client
            .get(&prov.userinfo_url)
            .bearer_auth(&token_resp.access_token)
            .send().await
            .map_err(|e| AppError::Internal(format!("userinfo fetch: {e}")))?
            .json().await
            .map_err(|e| AppError::Internal(format!("userinfo parse: {e}")))?;

        Ok(user_info)
    }
}

fn row_to_provider(r: &rusqlite::Row) -> rusqlite::Result<OAuthProvider> {
    Ok(OAuthProvider {
        id: r.get(0)?,
        provider: r.get(1)?,
        client_id: r.get(2)?,
        enabled: r.get::<_,i64>(3)? != 0,
        auth_url: r.get(4)?,
        token_url: r.get(5)?,
        userinfo_url: r.get(6)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    fn engine() -> OAuthEngine {
        OAuthEngine::new(db::open(":memory:").unwrap())
    }

    #[tokio::test]
    async fn test_list_providers_initially_disabled() {
        let e = engine();
        let enabled = e.list_enabled_providers().await.unwrap();
        assert!(enabled.is_empty(), "google should be disabled by default");
    }

    #[tokio::test]
    async fn test_configure_provider() {
        let e = engine();
        let p = e.configure_provider("google", "client123", "secret456", true).await.unwrap();
        assert!(p.enabled);
        assert_eq!(p.client_id, Some("client123".into()));
    }

    #[tokio::test]
    async fn test_enabled_after_configure() {
        let e = engine();
        e.configure_provider("google", "cid", "csec", true).await.unwrap();
        let providers = e.list_enabled_providers().await.unwrap();
        assert_eq!(providers.len(), 1);
        assert_eq!(providers[0].provider, "google");
    }

    #[tokio::test]
    async fn test_disabled_provider() {
        let e = engine();
        e.configure_provider("google", "cid", "csec", true).await.unwrap();
        e.configure_provider("google", "cid", "csec", false).await.unwrap();
        let providers = e.list_enabled_providers().await.unwrap();
        assert!(providers.is_empty());
    }
}
