pub mod attachments;
pub mod auth;
pub mod collections;
pub mod comments;
pub mod docs;
pub mod drafts;
pub mod health;
pub mod search;
pub mod sync;
pub mod templates;
pub mod trash;
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
        // Documents
        .route("/api/tree", get(tree::tree_handler))
        .route("/api/docs", post(docs::create_doc))
        .route("/api/docs/{*path}", get(docs::get_doc))
        .route("/api/docs/{*path}", put(docs::update_doc))
        .route("/api/docs/{*path}", delete(docs::delete_doc))
        // P1 #17: renamed from /api/docs/*/history to avoid wildcard ambiguity.
        .route("/api/docs-history/{*path}", get(docs::doc_history))
        // Collections
        .route("/api/collections", get(collections::list_collections))
        .route("/api/collections", post(collections::create_collection))
        .route("/api/collections/{id}", put(collections::update_collection))
        .route("/api/collections/{id}", delete(collections::delete_collection))
        // Drafts + Publish workflow
        // (prefixed routes to avoid wildcard conflict with /api/docs/{*path})
        .route("/api/doc-publish/{*path}", post(drafts::publish_doc))
        .route("/api/doc-unpublish/{*path}", post(drafts::unpublish_doc))
        .route("/api/drafts", get(drafts::list_drafts))
        // Templates
        .route("/api/templates", get(templates::list_templates))
        .route("/api/templates", post(templates::create_template))
        .route("/api/templates/{id}", put(templates::update_template))
        .route("/api/templates/{id}", delete(templates::delete_template))
        // Trash / Archive / Restore
        .route("/api/doc-archive/{*path}", post(trash::archive_doc))
        .route("/api/doc-unarchive/{*path}", post(trash::unarchive_doc))
        .route("/api/doc-trash/{*path}", post(trash::trash_doc))
        .route("/api/doc-restore/{*path}", post(trash::restore_doc))
        .route("/api/doc-delete/{*path}", delete(trash::permanent_delete_doc))
        .route("/api/trash", get(trash::list_trash))
        .route("/api/archive", get(trash::list_archive))
        // File attachments
        .route("/api/attachments/upload", post(attachments::upload_attachment))
        .route("/api/attachments/{id}", get(attachments::get_attachment))
        // Comments
        .route("/api/comments", get(comments::list_comments))
        .route("/api/comments", post(comments::create_comment))
        .route("/api/comments/{id}", put(comments::update_comment))
        .route("/api/comments/{id}", delete(comments::delete_comment))
        .route("/api/comments/{id}/resolve", post(comments::resolve_comment))
        // Search
        .route("/api/search", get(search::search_handler))
        // Sync
        .route("/api/sync/status", get(sync::sync_status))
        .route("/api/sync/pull", post(sync::sync_pull))
        .route("/api/sync/push", post(sync::sync_push))
        // Webhooks
        .route("/api/webhooks/git", post(webhooks::git_webhook))
        // WebSocket
        .route("/ws/yjs/{doc_path}", get(yjs_ws_handler))
        .with_state(state)
}
