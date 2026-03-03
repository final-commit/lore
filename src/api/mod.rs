pub mod ai_api;
pub mod attachments;
pub mod auth;
pub mod emojis_api;
pub mod export_jobs_api;
pub mod oauth_api;
pub mod unfurl;
pub mod collections;
pub mod comments;
pub mod cron_api;
pub mod docs;
pub mod drafts;
pub mod events;
pub mod export;
pub mod groups;
pub mod health;
pub mod import;
pub mod installation;
pub mod memberships;
pub mod notifications;
pub mod pins;
pub mod preferences;
pub mod reactions;
pub mod relationships;
pub mod search;
pub mod settings;
pub mod shares;
pub mod shortcuts;
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
        // Health
        .route("/health", get(health::health_handler))
        // Installation (no auth — setup check)
        .route("/api/installation", get(installation::installation_status))
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
        .route("/api/docs-history/{*path}", get(docs::doc_history))
        // Collections
        .route("/api/collections", get(collections::list_collections))
        .route("/api/collections", post(collections::create_collection))
        .route("/api/collections/{id}", put(collections::update_collection))
        .route("/api/collections/{id}", delete(collections::delete_collection))
        // Drafts + Publish
        .route("/api/doc-publish/{*path}", post(drafts::publish_doc))
        .route("/api/doc-unpublish/{*path}", post(drafts::unpublish_doc))
        .route("/api/drafts", get(drafts::list_drafts))
        // Templates
        .route("/api/templates", get(templates::list_templates))
        .route("/api/templates", post(templates::create_template))
        .route("/api/templates/{id}", put(templates::update_template))
        .route("/api/templates/{id}", delete(templates::delete_template))
        // Trash / Archive
        .route("/api/doc-archive/{*path}", post(trash::archive_doc))
        .route("/api/doc-unarchive/{*path}", post(trash::unarchive_doc))
        .route("/api/doc-trash/{*path}", post(trash::trash_doc))
        .route("/api/doc-restore/{*path}", post(trash::restore_doc))
        .route("/api/doc-delete/{*path}", delete(trash::permanent_delete_doc))
        .route("/api/trash", get(trash::list_trash))
        .route("/api/archive", get(trash::list_archive))
        // Attachments
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
        // Webhooks (inbound)
        .route("/api/webhooks/git", post(webhooks::git_webhook))
        // Stars
        .route("/api/stars", post(stars::toggle_star))
        .route("/api/stars", get(stars::list_stars))
        .route("/api/stars/{id}", delete(stars::delete_star))
        // Pins
        .route("/api/pins", post(pins::create_pin))
        .route("/api/pins", get(pins::list_pins))
        .route("/api/pins/{id}", delete(pins::delete_pin))
        .route("/api/pins/{id}", put(pins::reorder_pin))
        // Views
        .route("/api/views", post(views::record_view))
        .route("/api/views", get(views::list_views))
        .route("/api/views/recent", get(views::recent_views))
        // Shares (public view before /{id})
        .route("/api/shares/view/{url_id}", get(shares::view_share))
        .route("/api/shares", post(shares::create_share))
        .route("/api/shares", get(shares::list_shares))
        .route("/api/shares/{id}", delete(shares::delete_share))
        // Events / audit log
        .route("/api/events", get(events::list_events))
        // Groups
        .route("/api/groups", get(groups::list_groups))
        .route("/api/groups", post(groups::create_group))
        .route("/api/groups/{id}", put(groups::update_group))
        .route("/api/groups/{id}", delete(groups::delete_group))
        .route("/api/groups/{id}/members", get(groups::list_members))
        .route("/api/groups/{id}/members", post(groups::add_member))
        .route("/api/groups/{id}/members/{user_id}", delete(groups::remove_member))
        // User Memberships
        .route("/api/memberships", get(memberships::list_memberships))
        .route("/api/memberships", post(memberships::create_membership))
        .route("/api/memberships/{id}", put(memberships::update_membership))
        .route("/api/memberships/{id}", delete(memberships::delete_membership))
        // Notifications
        .route("/api/notifications", get(notifications::list_notifications))
        .route("/api/notifications/read-all", post(notifications::mark_all_notifications_read))
        .route("/api/notifications/{id}/read", post(notifications::mark_notification_read))
        // Subscriptions
        .route("/api/subscriptions", post(subscriptions::create_subscription))
        .route("/api/subscriptions", get(subscriptions::list_subscriptions))
        .route("/api/subscriptions/{id}", delete(subscriptions::delete_subscription))
        // Reactions
        .route("/api/reactions", post(reactions::toggle_reaction))
        .route("/api/reactions", get(reactions::list_reactions))
        .route("/api/reactions/{id}", delete(reactions::delete_reaction))
        // Export
        .route("/api/export/doc", get(export::export_doc))
        .route("/api/export/collection/{id}", get(export::export_collection))
        // Outbound webhooks
        .route("/api/webhook-subscriptions", get(webhook_subscriptions::list_webhook_subscriptions))
        .route("/api/webhook-subscriptions", post(webhook_subscriptions::create_webhook_subscription))
        .route("/api/webhook-subscriptions/{id}", put(webhook_subscriptions::update_webhook_subscription))
        .route("/api/webhook-subscriptions/{id}", delete(webhook_subscriptions::delete_webhook_subscription))
        // User search (mentions autocomplete)
        .route("/api/users/search", get(users::search_users))
        // Document Relationships
        .route("/api/relationships", post(relationships::create_relationship))
        .route("/api/relationships", get(relationships::list_relationships))
        .route("/api/relationships/{id}", delete(relationships::delete_relationship))
        // Settings
        .route("/api/settings", get(settings::get_settings))
        .route("/api/settings", put(settings::update_settings))
        // User Preferences
        .route("/api/preferences", get(preferences::get_preferences))
        .route("/api/preferences", put(preferences::update_preferences))
        // Keyboard shortcuts
        .route("/api/shortcuts", get(shortcuts::list_shortcuts))
        // Import
        .route("/api/import/outline", post(import::import_outline))
        .route("/api/import/markdown", post(import::import_markdown))
        // Cron (manual trigger)
        .route("/api/cron/run", post(cron_api::run_cron))
        // OAuth providers
        .route("/api/auth/providers", get(oauth_api::list_providers))
        .route("/api/auth/providers/all", get(oauth_api::list_all_providers))
        .route("/api/auth/providers/{provider}", put(oauth_api::configure_provider))
        .route("/api/auth/oauth/{provider}", get(oauth_api::oauth_redirect))
        .route("/api/auth/oauth/{provider}/callback", get(oauth_api::oauth_callback))
        // URL unfurling
        .route("/api/unfurl", get(unfurl::unfurl_handler))
        // AI
        .route("/api/ai/status", get(ai_api::ai_status))
        .route("/api/ai/suggest", post(ai_api::suggest))
        .route("/api/ai/answer", post(ai_api::answer))
        .route("/api/ai/summarize", post(ai_api::summarize))
        .route("/api/ai/generate", post(ai_api::generate))
        // Custom emoji
        .route("/api/emojis", get(emojis_api::list_emojis))
        .route("/api/emojis/upload", post(emojis_api::upload_emoji))
        .route("/api/emojis/{id}", delete(emojis_api::delete_emoji))
        // Async export jobs
        .route("/api/export-jobs", post(export_jobs_api::create_export_job))
        .route("/api/export-jobs/{id}", get(export_jobs_api::get_export_job))
        .route("/api/export-jobs/{id}/download", get(export_jobs_api::download_export_job))
        // WebSocket
        .route("/ws/yjs/{doc_path}", get(yjs_ws_handler))
        .with_state(state)
}
