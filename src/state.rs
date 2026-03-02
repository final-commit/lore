use std::sync::Arc;

use crate::attachments::AttachmentEngine;
use crate::auth::handler::AuthService;
use crate::auth::middleware::HasAuthState;
use crate::cache::PageCache;
use crate::collections::CollectionEngine;
use crate::comments::CommentEngine;
use crate::config::Config;
use crate::db::DbConn;
use crate::doc_meta::DocMetaEngine;
use crate::events::EventEngine;
use crate::git::GitEngine;
use crate::pins::PinEngine;
use crate::rate_limit::RateLimiter;
use crate::realtime::Rooms;
use crate::search::SearchEngine;
use crate::shares::ShareEngine;
use crate::stars::StarEngine;
use crate::sync::SyncEngine;
use crate::templates::TemplateEngine;
use crate::views::ViewEngine;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub db: DbConn,
    pub git: Arc<GitEngine>,
    pub comments: Arc<CommentEngine>,
    pub search: Arc<SearchEngine>,
    pub cache: Arc<PageCache>,
    pub auth: Arc<AuthService>,
    pub sync: Arc<SyncEngine>,
    pub rooms: Rooms,
    /// Rate limiter for auth endpoints (login / register).
    pub rate_limiter: RateLimiter,
    // Sprint 1 engines
    pub collections: Arc<CollectionEngine>,
    pub doc_meta: Arc<DocMetaEngine>,
    pub templates: Arc<TemplateEngine>,
    pub attachments: Arc<AttachmentEngine>,
    // Sprint 2 engines
    pub stars: Arc<StarEngine>,
    pub pins: Arc<PinEngine>,
    pub views: Arc<ViewEngine>,
    pub shares: Arc<ShareEngine>,
    pub events: Arc<EventEngine>,
}

impl HasAuthState for AppState {
    fn auth_state(&self) -> (&str, &DbConn) {
        (&self.config.jwt_secret, &self.db)
    }
}
