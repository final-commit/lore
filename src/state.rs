use std::sync::Arc;

use crate::auth::handler::AuthService;
use crate::auth::middleware::HasAuthState;
use crate::cache::PageCache;
use crate::comments::CommentEngine;
use crate::config::Config;
use crate::db::DbConn;
use crate::git::GitEngine;
use crate::rate_limit::RateLimiter;
use crate::realtime::Rooms;
use crate::search::SearchEngine;
use crate::sync::SyncEngine;

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
}

impl HasAuthState for AppState {
    fn auth_state(&self) -> (&str, &DbConn) {
        (&self.config.jwt_secret, &self.db)
    }
}
