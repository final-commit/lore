use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("not found: {0}")]
    NotFound(String),

    #[error("unauthorized: {0}")]
    Unauthorized(String),

    #[error("forbidden: {0}")]
    Forbidden(String),

    #[error("bad request: {0}")]
    BadRequest(String),

    #[error("conflict: {0}")]
    Conflict(String),

    #[error("internal error: {0}")]
    Internal(String),

    #[error("git error: {0}")]
    Git(#[from] git2::Error),

    #[error("database error: {0}")]
    Db(#[from] rusqlite::Error),

    #[error("search error: {0}")]
    Search(#[from] tantivy::TantivyError),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = match &self {
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
            AppError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            AppError::Forbidden(_) => StatusCode::FORBIDDEN,
            AppError::BadRequest(_) => StatusCode::BAD_REQUEST,
            AppError::Conflict(_) => StatusCode::CONFLICT,
            AppError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::Git(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::Db(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::Search(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };

        let message = self.to_string();
        tracing::error!(error = %message, "request error");
        (status, Json(json!({ "error": message }))).into_response()
    }
}

/// Validate a document path for safety (no path traversal, no absolute paths, no null bytes).
pub fn validate_path(path: &str) -> Result<(), AppError> {
    if path.is_empty() {
        return Err(AppError::BadRequest("path must not be empty".into()));
    }
    if path.contains("..") {
        return Err(AppError::BadRequest("path traversal not allowed".into()));
    }
    if path.starts_with('/') {
        return Err(AppError::BadRequest("absolute paths not allowed".into()));
    }
    if path.contains('\0') {
        return Err(AppError::BadRequest("null bytes not allowed in path".into()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;

    #[test]
    fn test_validate_path_valid() {
        assert!(validate_path("docs/getting-started.md").is_ok());
        assert!(validate_path("README.md").is_ok());
        assert!(validate_path("a/b/c/d.md").is_ok());
    }

    #[test]
    fn test_validate_path_traversal() {
        assert!(validate_path("../etc/passwd").is_err());
        assert!(validate_path("docs/../secret").is_err());
        assert!(validate_path("..").is_err());
    }

    #[test]
    fn test_validate_path_absolute() {
        assert!(validate_path("/etc/passwd").is_err());
        assert!(validate_path("/docs/file.md").is_err());
    }

    #[test]
    fn test_validate_path_null_byte() {
        assert!(validate_path("docs/\0file.md").is_err());
    }

    #[test]
    fn test_validate_path_empty() {
        assert!(validate_path("").is_err());
    }

    #[tokio::test]
    async fn test_not_found_response() {
        let err = AppError::NotFound("document not found".into());
        let resp = err.into_response();
        assert_eq!(resp.status(), StatusCode::NOT_FOUND);
        let body = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["error"], "not found: document not found");
    }

    #[tokio::test]
    async fn test_unauthorized_response() {
        let err = AppError::Unauthorized("invalid token".into());
        let resp = err.into_response();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_bad_request_response() {
        let err = AppError::BadRequest("missing field".into());
        let resp = err.into_response();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }
}
