pub mod auth;
pub mod comments;
pub mod docs;
pub mod health;
pub mod search;
pub mod sync;
pub mod tree;
pub mod webhooks;

use axum::{
    routing::{delete, get, post, put},
    Router,
};

use crate::realtime::yjs_ws_handler;
use crate::state::AppState;

pub fn router(state: AppState) -> Router {
    Router::new()
        // Health (no auth — liveness probe)
        .route("/health", get(health::health_handler))
        // Auth
        .route("/api/auth/register", post(auth::register))
        .route("/api/auth/login", post(auth::login))
        .route("/api/auth/refresh", post(auth::refresh))
        .route("/api/auth/me", get(auth::me))
        .route("/api/auth/tokens", post(auth::create_token))
        .route("/api/auth/tokens/{id}", delete(auth::revoke_token))
        // Documents (reads require auth — see handler extractors)
        .route("/api/tree", get(tree::tree_handler))
        .route("/api/docs", post(docs::create_doc))
        .route("/api/docs/{*path}", get(docs::get_doc))
        .route("/api/docs/{*path}", put(docs::update_doc))
        .route("/api/docs/{*path}", delete(docs::delete_doc))
        // P1 #17: renamed from /api/docs/*/history to avoid wildcard ambiguity.
        .route("/api/docs-history/{*path}", get(docs::doc_history))
        // Comments (reads require auth — see handler extractors)
        .route("/api/comments", get(comments::list_comments))
        .route("/api/comments", post(comments::create_comment))
        .route("/api/comments/{id}", put(comments::update_comment))
        .route("/api/comments/{id}", delete(comments::delete_comment))
        .route("/api/comments/{id}/resolve", post(comments::resolve_comment))
        // Search (requires auth — see handler extractor)
        .route("/api/search", get(search::search_handler))
        // Sync
        .route("/api/sync/status", get(sync::sync_status))
        .route("/api/sync/pull", post(sync::sync_pull))
        .route("/api/sync/push", post(sync::sync_push))
        // Webhooks
        .route("/api/webhooks/git", post(webhooks::git_webhook))
        // WebSocket (auth validated inside handler via token query param)
        .route("/ws/yjs/{doc_path}", get(yjs_ws_handler))
        .with_state(state)
}
