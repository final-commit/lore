use chrono::Utc;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::password::{hash_password, verify_password};
use crate::auth::token::{
    create_api_token, decode_token, encode_access_token, encode_refresh_token, revoke_api_token,
    sha256_hex, ApiToken, TokenKind,
};
use crate::db::{with_conn, DbConn};
use crate::error::AppError;

// ── Request / Response types ──────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub name: String,
    pub password: String,
    /// Accepted in the body for forward-compatibility but always ignored —
    /// non-first users are always registered as "editor" to prevent escalation.
    #[allow(dead_code)]
    pub role: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub user: UserInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInfo {
    pub id: String,
    pub email: String,
    pub name: String,
    pub role: String,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateApiTokenRequest {
    pub name: String,
    pub scope: String,
    pub expires_at: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CreateApiTokenResponse {
    pub token: ApiToken,
    /// Only returned once — store securely.
    pub raw_token: String,
}

// ── Handlers (business logic — framework-agnostic) ────────────────────────────

pub struct AuthService {
    db: DbConn,
    jwt_secret: String,
}

impl AuthService {
    pub fn new(db: DbConn, jwt_secret: String) -> Self {
        AuthService { db, jwt_secret }
    }

    pub async fn register(&self, req: RegisterRequest) -> Result<AuthResponse, AppError> {
        // Validate input
        if req.email.is_empty() || req.password.is_empty() || req.name.is_empty() {
            return Err(AppError::BadRequest("email, name, and password are required".into()));
        }
        // Basic email format check
        if !req.email.contains('@') || req.email.len() < 3 {
            return Err(AppError::BadRequest("invalid email format".into()));
        }
        if req.password.len() < 8 {
            return Err(AppError::BadRequest("password must be at least 8 characters".into()));
        }

        // P0 #1: Hash on a blocking thread — Argon2id is CPU-intensive.
        let pw = req.password.clone();
        let password_hash = tokio::task::spawn_blocking(move || hash_password(&pw))
            .await
            .map_err(|e| AppError::Internal(format!("hash task panicked: {e}")))?
            ?;

        let db = self.db.clone();
        let id = Uuid::now_v7().to_string();
        let now = Utc::now().to_rfc3339();

        // Determine role: first user is admin, all subsequent users are always
        // "editor" — ignore any role field in the request (P0 #6).
        let user_count = with_conn(&db, |conn| {
            conn.query_row("SELECT COUNT(*) FROM users", [], |r| r.get::<_, i64>(0))
        })
        .await?;

        let role = if user_count == 0 { "admin".to_string() } else { "editor".to_string() };

        let uid = id.clone();
        let email = req.email.clone();
        let name = req.name.clone();
        let r = role.clone();
        let n = now.clone();
        let ph = password_hash.clone();

        with_conn(&db, move |conn| {
            conn.execute(
                r#"INSERT INTO users (id, email, name, password_hash, role, created_at, updated_at)
                   VALUES (?1,?2,?3,?4,?5,?6,?6)"#,
                params![uid, email, name, ph, r, n],
            )
            .map(|_| ())
        })
        .await
        .map_err(|e| match e {
            AppError::Db(rusqlite::Error::SqliteFailure(err, _))
                if err.code == rusqlite::ErrorCode::ConstraintViolation =>
            {
                AppError::Conflict("email already registered".into())
            }
            other => other,
        })?;

        let access_token = encode_access_token(&id, &req.email, &role, &self.jwt_secret)?;
        let refresh_token = encode_refresh_token(&id, &req.email, &role, &self.jwt_secret)?;

        // Store refresh token hash
        self.store_session(&id, &refresh_token).await?;

        Ok(AuthResponse {
            access_token,
            refresh_token,
            user: UserInfo {
                id,
                email: req.email,
                name: req.name,
                role,
                created_at: now,
            },
        })
    }

    pub async fn login(&self, req: LoginRequest) -> Result<AuthResponse, AppError> {
        let db = self.db.clone();
        let email = req.email.clone();

        let row = with_conn(&db, move |conn| {
            conn.query_row(
                "SELECT id, email, name, password_hash, role, created_at FROM users WHERE email=?1",
                params![email],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, String>(5)?,
                    ))
                },
            )
        })
        .await
        .map_err(|e| match e {
            AppError::Db(rusqlite::Error::QueryReturnedNoRows) => {
                AppError::Unauthorized("invalid email or password".into())
            }
            other => other,
        })?;

        let (id, email, name, password_hash, role, created_at) = row;

        // P0 #1: Verify on a blocking thread — Argon2id is CPU-intensive.
        let pw = req.password.clone();
        let ph = password_hash.clone();
        let valid = tokio::task::spawn_blocking(move || verify_password(&pw, &ph))
            .await
            .map_err(|e| AppError::Internal(format!("verify task panicked: {e}")))?
            ?;

        if !valid {
            return Err(AppError::Unauthorized("invalid email or password".into()));
        }

        let access_token = encode_access_token(&id, &email, &role, &self.jwt_secret)?;
        let refresh_token = encode_refresh_token(&id, &email, &role, &self.jwt_secret)?;
        self.store_session(&id, &refresh_token).await?;

        Ok(AuthResponse {
            access_token,
            refresh_token,
            user: UserInfo { id, email, name, role, created_at },
        })
    }

    pub async fn refresh(&self, req: RefreshRequest) -> Result<AuthResponse, AppError> {
        let claims = decode_token(&req.refresh_token, &self.jwt_secret)?;
        if claims.kind != TokenKind::Refresh {
            return Err(AppError::Unauthorized("not a refresh token".into()));
        }

        // P0 #8: Verify the token hash AND check expiry.
        let hash = sha256_hex(&req.refresh_token);
        let db = self.db.clone();
        let uid = claims.sub.clone();
        let hash_check = hash.clone();
        let session_exists = with_conn(&db, move |conn| {
            conn.query_row(
                "SELECT COUNT(*) FROM sessions \
                 WHERE user_id=?1 AND refresh_token_hash=?2 AND expires_at > datetime('now')",
                params![uid, hash_check],
                |r| r.get::<_, i64>(0),
            )
        })
        .await?;

        if session_exists == 0 {
            return Err(AppError::Unauthorized("session revoked or expired".into()));
        }

        // P0 #9: Delete the consumed token (token rotation — no re-use of old tokens).
        let uid_del = claims.sub.clone();
        let hash_del = hash.clone();
        with_conn(&db, move |conn| {
            conn.execute(
                "DELETE FROM sessions WHERE user_id=?1 AND refresh_token_hash=?2",
                params![uid_del, hash_del],
            )
            .map(|_| ())
        })
        .await?;

        // Fetch user info in case role changed.
        let uid = claims.sub.clone();
        let user = with_conn(&self.db, move |conn| {
            conn.query_row(
                "SELECT id, email, name, role, created_at FROM users WHERE id=?1",
                params![uid],
                |row| {
                    Ok(UserInfo {
                        id: row.get(0)?,
                        email: row.get(1)?,
                        name: row.get(2)?,
                        role: row.get(3)?,
                        created_at: row.get(4)?,
                    })
                },
            )
        })
        .await?;

        let access_token =
            encode_access_token(&user.id, &user.email, &user.role, &self.jwt_secret)?;
        let new_refresh =
            encode_refresh_token(&user.id, &user.email, &user.role, &self.jwt_secret)?;
        self.store_session(&user.id, &new_refresh).await?;

        Ok(AuthResponse {
            access_token,
            refresh_token: new_refresh,
            user,
        })
    }

    pub async fn get_me(&self, user_id: &str) -> Result<UserInfo, AppError> {
        let db = self.db.clone();
        let uid = user_id.to_string();
        with_conn(&db, move |conn| {
            conn.query_row(
                "SELECT id, email, name, role, created_at FROM users WHERE id=?1",
                params![uid],
                |row| {
                    Ok(UserInfo {
                        id: row.get(0)?,
                        email: row.get(1)?,
                        name: row.get(2)?,
                        role: row.get(3)?,
                        created_at: row.get(4)?,
                    })
                },
            )
        })
        .await
        .map_err(|e| match e {
            AppError::Db(rusqlite::Error::QueryReturnedNoRows) => {
                AppError::NotFound("user not found".into())
            }
            other => other,
        })
    }

    async fn store_session(&self, user_id: &str, refresh_token: &str) -> Result<(), AppError> {
        let hash = sha256_hex(refresh_token);
        let id = Uuid::now_v7().to_string();
        let now = Utc::now();
        let expires_at = (now + chrono::Duration::days(30)).to_rfc3339();
        let created_at = now.to_rfc3339();
        let uid = user_id.to_string();
        let db = self.db.clone();

        with_conn(&db, move |conn| {
            conn.execute(
                r#"INSERT INTO sessions (id, user_id, refresh_token_hash, expires_at, created_at)
                   VALUES (?1,?2,?3,?4,?5)"#,
                params![id, uid, hash, expires_at, created_at],
            )
            .map(|_| ())
        })
        .await
    }

    pub async fn create_api_token(
        &self,
        user_id: &str,
        req: CreateApiTokenRequest,
    ) -> Result<CreateApiTokenResponse, AppError> {
        let (token, raw) = create_api_token(
            &self.db,
            user_id,
            &req.name,
            &req.scope,
            req.expires_at.as_deref(),
        )
        .await?;
        Ok(CreateApiTokenResponse { token, raw_token: raw })
    }

    pub async fn revoke_api_token(&self, token_id: &str) -> Result<(), AppError> {
        revoke_api_token(&self.db, token_id).await
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    const SECRET: &str = "test-secret-32-chars-minimum-len!";

    fn service() -> AuthService {
        let db = db::open(":memory:").unwrap();
        AuthService::new(db, SECRET.to_string())
    }

    fn reg(email: &str) -> RegisterRequest {
        RegisterRequest {
            email: email.to_string(),
            name: "Test User".to_string(),
            password: "password123".to_string(),
            role: None,
        }
    }

    #[tokio::test]
    async fn test_register_first_user_is_admin() {
        let svc = service();
        let resp = svc.register(reg("admin@example.com")).await.unwrap();
        assert_eq!(resp.user.role, "admin");
    }

    #[tokio::test]
    async fn test_register_second_user_is_editor() {
        let svc = service();
        svc.register(reg("first@example.com")).await.unwrap();
        let resp = svc.register(reg("second@example.com")).await.unwrap();
        assert_eq!(resp.user.role, "editor");
    }

    #[tokio::test]
    async fn test_register_role_escalation_blocked() {
        // Even if the client sends role:"admin", second user must be "editor".
        let svc = service();
        svc.register(reg("first@example.com")).await.unwrap();
        let resp = svc
            .register(RegisterRequest {
                email: "attacker@example.com".into(),
                name: "Attacker".into(),
                password: "password123".into(),
                role: Some("admin".into()),
            })
            .await
            .unwrap();
        assert_eq!(resp.user.role, "editor");
    }

    #[tokio::test]
    async fn test_register_duplicate_email() {
        let svc = service();
        svc.register(reg("dup@example.com")).await.unwrap();
        let err = svc.register(reg("dup@example.com")).await.unwrap_err();
        assert!(matches!(err, AppError::Conflict(_)));
    }

    #[tokio::test]
    async fn test_login_success() {
        let svc = service();
        svc.register(reg("user@example.com")).await.unwrap();
        let resp = svc
            .login(LoginRequest {
                email: "user@example.com".into(),
                password: "password123".into(),
            })
            .await
            .unwrap();
        assert!(!resp.access_token.is_empty());
    }

    #[tokio::test]
    async fn test_login_wrong_password() {
        let svc = service();
        svc.register(reg("user@example.com")).await.unwrap();
        let err = svc
            .login(LoginRequest {
                email: "user@example.com".into(),
                password: "wrong".into(),
            })
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::Unauthorized(_)));
    }

    #[tokio::test]
    async fn test_login_unknown_email() {
        let svc = service();
        let err = svc
            .login(LoginRequest {
                email: "no@example.com".into(),
                password: "pass".into(),
            })
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::Unauthorized(_)));
    }

    #[tokio::test]
    async fn test_refresh_token() {
        let svc = service();
        let reg_resp = svc.register(reg("user@example.com")).await.unwrap();
        let refresh_resp = svc
            .refresh(RefreshRequest { refresh_token: reg_resp.refresh_token.clone() })
            .await
            .unwrap();
        assert!(!refresh_resp.access_token.is_empty());

        // Old token must be invalidated (token rotation).
        let err = svc
            .refresh(RefreshRequest { refresh_token: reg_resp.refresh_token })
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::Unauthorized(_)));
    }

    #[tokio::test]
    async fn test_get_me() {
        let svc = service();
        let reg_resp = svc.register(reg("me@example.com")).await.unwrap();
        let me = svc.get_me(&reg_resp.user.id).await.unwrap();
        assert_eq!(me.email, "me@example.com");
    }

    #[tokio::test]
    async fn test_short_password_rejected() {
        let svc = service();
        let err = svc
            .register(RegisterRequest {
                email: "a@b.com".into(),
                name: "A".into(),
                password: "short".into(),
                role: None,
            })
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::BadRequest(_)));
    }

    #[tokio::test]
    async fn test_invalid_email_rejected() {
        let svc = service();
        let err = svc
            .register(RegisterRequest {
                email: "notanemail".into(),
                name: "A".into(),
                password: "password123".into(),
                role: None,
            })
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::BadRequest(_)));
    }
}
