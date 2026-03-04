use crate::auth::token::{decode_token, verify_api_token, TokenKind};
use crate::db::DbConn;
use crate::error::AppError;

/// Authenticated user extracted from JWT or API token.
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub id: String,
    pub email: String,
    pub role: String,
}

impl AuthUser {
    pub fn is_admin(&self) -> bool {
        self.role == "admin"
    }
    pub fn is_editor_or_admin(&self) -> bool {
        self.role == "editor" || self.role == "admin"
    }
}

/// State must provide the JWT secret and DB for API token lookup.
pub trait HasAuthState {
    fn auth_state(&self) -> (&str, &DbConn);
}

// ── Axum extractor ────────────────────────────────────────────────────────────

impl<S> axum::extract::FromRequestParts<S> for AuthUser
where
    S: Send + Sync + HasAuthState,
{
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        let token_str = extract_bearer_token(parts)?;
        let (secret, db) = state.auth_state();
        resolve_token(&token_str, secret, db).await
    }
}

// ── Token resolution ──────────────────────────────────────────────────────────

pub async fn resolve_token(
    token_str: &str,
    jwt_secret: &str,
    db: &DbConn,
) -> Result<AuthUser, AppError> {
    // Try JWT first.
    if let Ok(claims) = decode_token(token_str, jwt_secret) {
        if claims.kind != TokenKind::Access {
            return Err(AppError::Unauthorized(
                "refresh tokens cannot be used for API calls".into(),
            ));
        }
        return Ok(AuthUser {
            id: claims.sub,
            email: claims.email,
            role: claims.role,
        });
    }

    // Fall back to API token lookup.
    let api_tok = verify_api_token(db, token_str).await?;

    let user_id = api_tok.user_id.clone();
    let user = crate::db::with_conn(db, move |conn| {
        conn.query_row(
            "SELECT id, email, role FROM users WHERE id=?1",
            rusqlite::params![user_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )
    })
    .await
    .map_err(|e| match e {
        AppError::Db(rusqlite::Error::QueryReturnedNoRows) => {
            AppError::Unauthorized("token owner not found".into())
        }
        other => other,
    })?;

    Ok(AuthUser { id: user.0, email: user.1, role: user.2 })
}

// ── Bearer extraction helper ──────────────────────────────────────────────────

fn extract_bearer_token(parts: &axum::http::request::Parts) -> Result<String, AppError> {
    parts
        .headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(|s| s.to_string())
        .ok_or_else(|| AppError::Unauthorized("missing or invalid Authorization header".into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{auth::token::encode_access_token, db};

    const SECRET: &str = "test-secret-32-chars-minimum-len!";

    #[tokio::test]
    async fn test_resolve_valid_jwt() {
        let db = db::open(":memory:").unwrap();
        let token = encode_access_token("uid1", "a@b.com", "editor", SECRET).unwrap();
        let user = resolve_token(&token, SECRET, &db).await.unwrap();
        assert_eq!(user.id, "uid1");
        assert_eq!(user.role, "editor");
    }

    #[tokio::test]
    async fn test_resolve_invalid_token_rejected() {
        let db = db::open(":memory:").unwrap();
        let err = resolve_token("bad-token", SECRET, &db).await.unwrap_err();
        assert!(matches!(err, AppError::Unauthorized(_)));
    }

    #[tokio::test]
    async fn test_refresh_token_rejected_for_api() {
        let db = db::open(":memory:").unwrap();
        let token =
            crate::auth::token::encode_refresh_token("uid1", "a@b.com", "editor", SECRET)
                .unwrap();
        let err = resolve_token(&token, SECRET, &db).await.unwrap_err();
        assert!(matches!(err, AppError::Unauthorized(_)));
    }
}
