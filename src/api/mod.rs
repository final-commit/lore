pub mod attachments;
pub mod auth;
pub mod collections;
pub mod comments;
pub mod docs;
pub mod drafts;
pub mod events;
pub mod export;
pub mod groups;
pub mod health;
pub mod memberships;
pub mod notifications;
pub mod pins;
pub mod reactions;
pub mod relationships;
pub mod search;
pub mod shares;
pub mod stars;
pub mod subscriptions;
pub mod sync;
pub mod templates;
pub mod trash;
pub mod tree;
pub mod users;
pub mod views;
pub mod webhook_subscriptions;
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
        // ── Sprint 2: Stars (bookmarks) ──────────────────────────────────────
        .route("/api/stars", post(stars::toggle_star))
        .route("/api/stars", get(stars::list_stars))
        .route("/api/stars/{id}", delete(stars::delete_star))
        // ── Sprint 2: Pins ───────────────────────────────────────────────────
        .route("/api/pins", post(pins::create_pin))
        .route("/api/pins", get(pins::list_pins))
        .route("/api/pins/{id}", delete(pins::delete_pin))
        .route("/api/pins/{id}", put(pins::reorder_pin))
        // ── Sprint 2: Views tracking ─────────────────────────────────────────
        .route("/api/views", post(views::record_view))
        .route("/api/views", get(views::list_views))
        .route("/api/views/recent", get(views::recent_views))
        // ── Sprint 2: Document sharing ───────────────────────────────────────
        // Public view route MUST come before /api/shares/{id} to avoid conflict.
        .route("/api/shares/view/{url_id}", get(shares::view_share))
        .route("/api/shares", post(shares::create_share))
        .route("/api/shares", get(shares::list_shares))
        .route("/api/shares/{id}", delete(shares::delete_share))
        // ── Sprint 2: Events / audit log ─────────────────────────────────────
        .route("/api/events", get(events::list_events))
        // ── Sprint 3: Groups ──────────────────────────────────────────────────
        .route("/api/groups", get(groups::list_groups))
        .route("/api/groups", post(groups::create_group))
        .route("/api/groups/{id}", put(groups::update_group))
        .route("/api/groups/{id}", delete(groups::delete_group))
        .route("/api/groups/{id}/members", get(groups::list_members))
        .route("/api/groups/{id}/members", post(groups::add_member))
        .route("/api/groups/{id}/members/{user_id}", delete(groups::remove_member))
        // ── Sprint 3: User Memberships ────────────────────────────────────────
        .route("/api/memberships", get(memberships::list_memberships))
        .route("/api/memberships", post(memberships::create_membership))
        .route("/api/memberships/{id}", put(memberships::update_membership))
        .route("/api/memberships/{id}", delete(memberships::delete_membership))
        // ── Sprint 3: Notifications ───────────────────────────────────────────
        .route("/api/notifications", get(notifications::list_notifications))
        // read-all MUST come before /{id}/read to avoid route conflict
        .route("/api/notifications/read-all", post(notifications::mark_all_notifications_read))
        .route("/api/notifications/{id}/read", post(notifications::mark_notification_read))
        // ── Sprint 3: Subscriptions ───────────────────────────────────────────
        .route("/api/subscriptions", post(subscriptions::create_subscription))
        .route("/api/subscriptions", get(subscriptions::list_subscriptions))
        .route("/api/subscriptions/{id}", delete(subscriptions::delete_subscription))
        // ── Sprint 3: Reactions ───────────────────────────────────────────────
        .route("/api/reactions", post(reactions::toggle_reaction))
        .route("/api/reactions", get(reactions::list_reactions))
        .route("/api/reactions/{id}", delete(reactions::delete_reaction))
        // ── Sprint 4: Export ──────────────────────────────────────────────────
        .route("/api/export/doc", get(export::export_doc))
        .route("/api/export/collection/{id}", get(export::export_collection))
        // ── Sprint 4: Outbound Webhook Subscriptions ──────────────────────────
        .route("/api/webhook-subscriptions", get(webhook_subscriptions::list_webhook_subscriptions))
        .route("/api/webhook-subscriptions", post(webhook_subscriptions::create_webhook_subscription))
        .route("/api/webhook-subscriptions/{id}", put(webhook_subscriptions::update_webhook_subscription))
        .route("/api/webhook-subscriptions/{id}", delete(webhook_subscriptions::delete_webhook_subscription))
        // ── Sprint 4: User search (mentions autocomplete) ─────────────────────
        .route("/api/users/search", get(users::search_users))
        // ── Sprint 4: Document Relationships ──────────────────────────────────
        .route("/api/relationships", post(relationships::create_relationship))
        .route("/api/relationships", get(relationships::list_relationships))
        .route("/api/relationships/{id}", delete(relationships::delete_relationship))
        // WebSocket
        .route("/ws/yjs/{doc_path}", get(yjs_ws_handler))
        .with_state(state)
}
