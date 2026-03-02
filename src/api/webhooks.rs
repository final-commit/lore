use axum::{
    body::Bytes,
    extract::{Request, State},
    http::HeaderMap,
    Json,
};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::state::AppState;

#[derive(Debug, Serialize)]
pub struct WebhookResponse {
    pub processed: bool,
    pub message: String,
}

/// POST /api/webhooks/git
/// Receives push events from GitHub/GitLab/Gitea.
/// Verifies the HMAC-SHA256 signature before processing.
pub async fn git_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<WebhookResponse>, AppError> {
    let secret = state
        .config
        .webhook_secret
        .as_deref()
        .ok_or_else(|| AppError::BadRequest("webhook secret not configured".into()))?;

    // Verify signature (supports GitHub's X-Hub-Signature-256 and Gitea).
    verify_hmac_sha256(&headers, &body, secret)?;

    // Parse the event type.
    let event_type = headers
        .get("X-GitHub-Event")
        .or_else(|| headers.get("X-Gitea-Event"))
        .or_else(|| headers.get("X-Gitlab-Event"))
        .and_then(|v| v.to_str().ok())
        .unwrap_or("push");

    if event_type == "push" {
        // Trigger a cache invalidation — the remote has new commits.
        state.cache.invalidate_all().await;

        tracing::info!("webhook push event received, cache invalidated");
        return Ok(Json(WebhookResponse {
            processed: true,
            message: "push event processed".into(),
        }));
    }

    Ok(Json(WebhookResponse {
        processed: false,
        message: format!("event type '{event_type}' not handled"),
    }))
}

// ── HMAC verification ─────────────────────────────────────────────────────────

fn verify_hmac_sha256(headers: &HeaderMap, body: &[u8], secret: &str) -> Result<(), AppError> {
    // GitHub uses `X-Hub-Signature-256: sha256=<hex>`.
    // Gitea uses `X-Gitea-Signature: <hex>`.
    let sig_header = headers
        .get("X-Hub-Signature-256")
        .or_else(|| headers.get("X-Gitea-Signature"))
        .or_else(|| headers.get("X-Gitlab-Token"))
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| AppError::Unauthorized("missing webhook signature header".into()))?;

    let provided_hex = sig_header
        .strip_prefix("sha256=")
        .unwrap_or(sig_header);

    let expected = compute_hmac_sha256(secret, body);

    // Constant-time comparison.
    if !constant_time_eq(provided_hex.as_bytes(), expected.as_bytes()) {
        return Err(AppError::Unauthorized("invalid webhook signature".into()));
    }

    Ok(())
}

fn compute_hmac_sha256(secret: &str, data: &[u8]) -> String {
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes())
        .expect("HMAC accepts any key length");
    mac.update(data);
    hex::encode(mac.finalize().into_bytes())
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b.iter()).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hmac_computation() {
        let sig = compute_hmac_sha256("secret", b"hello world");
        assert!(!sig.is_empty());
        // Deterministic.
        assert_eq!(sig, compute_hmac_sha256("secret", b"hello world"));
    }

    #[test]
    fn test_different_secrets_differ() {
        let a = compute_hmac_sha256("secret1", b"payload");
        let b = compute_hmac_sha256("secret2", b"payload");
        assert_ne!(a, b);
    }

    #[test]
    fn test_constant_time_eq() {
        assert!(constant_time_eq(b"abc", b"abc"));
        assert!(!constant_time_eq(b"abc", b"abd"));
        assert!(!constant_time_eq(b"abc", b"abcd"));
    }
}
