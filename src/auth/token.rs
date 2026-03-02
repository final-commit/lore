use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::db::{with_conn, DbConn};
use crate::error::AppError;

// ── Claims ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,   // user id
    pub email: String,
    pub role: String,
    pub exp: i64,
    pub iat: i64,
    /// Distinguish access vs refresh tokens
    pub kind: TokenKind,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TokenKind {
    Access,
    Refresh,
}

// ── JWT encode / decode ───────────────────────────────────────────────────────

pub fn encode_access_token(
    user_id: &str,
    email: &str,
    role: &str,
    secret: &str,
) -> Result<String, AppError> {
    let now = Utc::now();
    let claims = Claims {
        sub: user_id.to_string(),
        email: email.to_string(),
        role: role.to_string(),
        exp: (now + Duration::hours(1)).timestamp(),
        iat: now.timestamp(),
        kind: TokenKind::Access,
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| AppError::Internal(format!("JWT encode error: {e}")))
}

pub fn encode_refresh_token(
    user_id: &str,
    email: &str,
    role: &str,
    secret: &str,
) -> Result<String, AppError> {
    let now = Utc::now();
    let claims = Claims {
        sub: user_id.to_string(),
        email: email.to_string(),
        role: role.to_string(),
        exp: (now + Duration::days(30)).timestamp(),
        iat: now.timestamp(),
        kind: TokenKind::Refresh,
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| AppError::Internal(format!("JWT encode error: {e}")))
}

pub fn decode_token(token: &str, secret: &str) -> Result<Claims, AppError> {
    let mut validation = Validation::default();
    validation.validate_exp = true;

    decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )
    .map(|d| d.claims)
    .map_err(|e| {
        use jsonwebtoken::errors::ErrorKind;
        match e.kind() {
            ErrorKind::ExpiredSignature => AppError::Unauthorized("token expired".into()),
            _ => AppError::Unauthorized(format!("invalid token: {e}")),
        }
    })
}

// ── API token management ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiToken {
    pub id: String,
    pub user_id: String,
    pub name: String,
    pub scope: String,
    pub expires_at: Option<String>,
    pub created_at: String,
}

/// Create a new API token and store its hash in the DB.
/// Returns (token_record, plaintext_token) — the plaintext is only shown once.
pub async fn create_api_token(
    db: &DbConn,
    user_id: &str,
    name: &str,
    scope: &str,
    expires_at: Option<&str>,
) -> Result<(ApiToken, String), AppError> {
    // Generate random token using thread-local RNG (rand 0.9)
    let raw = {
        use rand::RngCore;
        let mut bytes = [0u8; 32];
        rand::rng().fill_bytes(&mut bytes);
        hex::encode(bytes)
    };

    let hash = sha256_hex(&raw);
    let id = Uuid::now_v7().to_string();
    let now = Utc::now().to_rfc3339();

    let token = ApiToken {
        id: id.clone(),
        user_id: user_id.to_string(),
        name: name.to_string(),
        scope: scope.to_string(),
        expires_at: expires_at.map(|s| s.to_string()),
        created_at: now.clone(),
    };

    let t = token.clone();
    let expires = expires_at.map(|s| s.to_string());
    let uid = user_id.to_string();
    let nm = name.to_string();
    let sc = scope.to_string();

    with_conn(db, move |conn| {
        conn.execute(
            r#"INSERT INTO api_tokens (id, user_id, name, token_hash, scope, expires_at, created_at)
               VALUES (?1,?2,?3,?4,?5,?6,?7)"#,
            params![t.id, uid, nm, hash, sc, expires, now],
        )?;
        Ok(())
    })
    .await
    .map_err(AppError::Db)?;

    Ok((token, raw))
}

/// Verify an API token.  Returns the token record if valid.
pub async fn verify_api_token(db: &DbConn, raw_token: &str) -> Result<ApiToken, AppError> {
    let hash = sha256_hex(raw_token);

    with_conn(db, move |conn| {
        conn.query_row(
            r#"SELECT id, user_id, name, scope, expires_at, created_at
               FROM api_tokens
               WHERE token_hash = ?1"#,
            params![hash],
            |row| {
                Ok(ApiToken {
                    id: row.get(0)?,
                    user_id: row.get(1)?,
                    name: row.get(2)?,
                    scope: row.get(3)?,
                    expires_at: row.get(4)?,
                    created_at: row.get(5)?,
                })
            },
        )
    })
    .await
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => AppError::Unauthorized("invalid API token".into()),
        other => AppError::Db(other),
    })
}

/// Revoke an API token by ID.
pub async fn revoke_api_token(db: &DbConn, token_id: &str) -> Result<(), AppError> {
    let id = token_id.to_string();
    with_conn(db, move |conn| {
        let n = conn.execute("DELETE FROM api_tokens WHERE id=?1", params![id])?;
        if n == 0 {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        Ok(())
    })
    .await
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => AppError::NotFound("api token not found".into()),
        other => AppError::Db(other),
    })
}

// ── Helpers ───────────────────────────────────────────────────────────────────

pub fn sha256_hex(s: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(s.as_bytes());
    hex::encode(hasher.finalize())
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    const SECRET: &str = "test-secret-key-32-chars-minimum!!";

    #[test]
    fn test_encode_decode_access_token() {
        let token = encode_access_token("uid1", "a@b.com", "editor", SECRET).unwrap();
        let claims = decode_token(&token, SECRET).unwrap();
        assert_eq!(claims.sub, "uid1");
        assert_eq!(claims.email, "a@b.com");
        assert_eq!(claims.role, "editor");
        assert_eq!(claims.kind, TokenKind::Access);
    }

    #[test]
    fn test_encode_decode_refresh_token() {
        let token = encode_refresh_token("uid1", "a@b.com", "admin", SECRET).unwrap();
        let claims = decode_token(&token, SECRET).unwrap();
        assert_eq!(claims.kind, TokenKind::Refresh);
    }

    #[test]
    fn test_wrong_secret_rejected() {
        let token = encode_access_token("uid1", "a@b.com", "editor", SECRET).unwrap();
        let err = decode_token(&token, "wrong-secret").unwrap_err();
        assert!(matches!(err, AppError::Unauthorized(_)));
    }

    #[test]
    fn test_sha256_hex_deterministic() {
        assert_eq!(sha256_hex("hello"), sha256_hex("hello"));
        assert_ne!(sha256_hex("hello"), sha256_hex("world"));
    }

    #[tokio::test]
    async fn test_create_and_verify_api_token() {
        let db = db::open(":memory:").unwrap();

        // Create a user first (FK constraint)
        {
            let d = db.clone();
            with_conn(&d, |conn| {
                conn.execute(
                    "INSERT INTO users (id, email, name, password_hash, role, created_at, updated_at)
                     VALUES ('u1','a@b.com','Test','hash','editor','2024-01-01','2024-01-01')",
                    [],
                )?;
                Ok(())
            })
            .await
            .unwrap();
        }

        let (record, raw) = create_api_token(&db, "u1", "CI token", "read", None)
            .await
            .unwrap();

        assert!(!raw.is_empty());
        assert_eq!(record.user_id, "u1");

        let verified = verify_api_token(&db, &raw).await.unwrap();
        assert_eq!(verified.id, record.id);
    }

    #[tokio::test]
    async fn test_verify_invalid_token_rejected() {
        let db = db::open(":memory:").unwrap();
        let err = verify_api_token(&db, "invalid-raw-token").await.unwrap_err();
        assert!(matches!(err, AppError::Unauthorized(_)));
    }

    #[tokio::test]
    async fn test_revoke_api_token() {
        let db = db::open(":memory:").unwrap();
        with_conn(&db, |conn| {
            conn.execute(
                "INSERT INTO users (id, email, name, password_hash, role, created_at, updated_at)
                 VALUES ('u1','a@b.com','Test','hash','editor','2024-01-01','2024-01-01')",
                [],
            )?;
            Ok(())
        })
        .await
        .unwrap();

        let (record, raw) = create_api_token(&db, "u1", "tok", "read", None).await.unwrap();
        revoke_api_token(&db, &record.id).await.unwrap();

        let err = verify_api_token(&db, &raw).await.unwrap_err();
        assert!(matches!(err, AppError::Unauthorized(_)));
    }
}
