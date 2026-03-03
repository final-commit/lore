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
use crate::groups::GroupEngine;
use crate::memberships::MembershipEngine;
use crate::notifications::NotificationEngine;
use crate::outbound_webhooks::OutboundWebhookEngine;
use crate::pins::PinEngine;
use crate::rate_limit::RateLimiter;
use crate::reactions::ReactionEngine;
use crate::realtime::Rooms;
use crate::ai::AiEngine;
use crate::emojis::EmojiEngine;
use crate::export_jobs::ExportJobEngine;
use crate::import::ImportEngine;
use crate::oauth::OAuthEngine;
use crate::preferences::PreferencesEngine;
use crate::unfurl::UnfurlEngine;
use crate::relationships::RelationshipEngine;
use crate::search::SearchEngine;
use crate::settings::SettingsEngine;
use crate::shares::ShareEngine;
use crate::stars::StarEngine;
use crate::subscriptions::SubscriptionEngine;
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
    // Sprint 3+4 engines
    pub groups: Arc<GroupEngine>,
    pub memberships: Arc<MembershipEngine>,
    pub notifications: Arc<NotificationEngine>,
    pub subscriptions: Arc<SubscriptionEngine>,
    pub reactions: Arc<ReactionEngine>,
    pub outbound_webhooks: Arc<OutboundWebhookEngine>,
    pub relationships: Arc<RelationshipEngine>,
    // Sprint 5+6 engines
    pub settings: Arc<SettingsEngine>,
    pub preferences: Arc<PreferencesEngine>,
    pub import: Arc<ImportEngine>,
    // Overnight build engines
    pub ai: Arc<AiEngine>,
    pub unfurl: Arc<UnfurlEngine>,
    pub emojis: Arc<EmojiEngine>,
    pub export_jobs: Arc<ExportJobEngine>,
    pub oauth: Arc<OAuthEngine>,
}

impl HasAuthState for AppState {
    fn auth_state(&self) -> (&str, &DbConn) {
        (&self.config.jwt_secret, &self.db)
    }
}

#[cfg(test)]
impl AppState {
    /// Build a minimal AppState backed by an in-memory SQLite DB for unit tests.
    /// The `_tmp` handle keeps the temp directory alive for the lifetime of the state.
    pub fn new_for_test(
        db: DbConn,
        git: GitEngine,
        search: SearchEngine,
        _tmp: tempfile::TempDir,
    ) -> Self {
        use std::time::Duration;
        use crate::realtime::new_rooms;

        let repo_path = std::path::PathBuf::from("/tmp/forge-test");
        let git_arc = git.clone();
        let queue = crate::git::queue::GitQueue::new();

        let cache = crate::cache::PageCache::new(100, Duration::from_secs(60));
        let auth = AuthService::new(db.clone(), "test-secret-32-chars-minimum-len!".to_string());
        let comments = crate::comments::CommentEngine::new(db.clone());
        let sync = crate::sync::SyncEngine::new(db.clone(), repo_path.clone(), queue);
        let rooms = new_rooms();
        let rate_limiter = RateLimiter::new(20);
        let collections = CollectionEngine::new(db.clone());
        let doc_meta = DocMetaEngine::new(db.clone());
        let templates = TemplateEngine::new(db.clone());
        let attachments = AttachmentEngine::new(db.clone(), repo_path.clone(), 10 * 1024 * 1024);
        let stars = StarEngine::new(db.clone());
        let pins = PinEngine::new(db.clone());
        let views = ViewEngine::new(db.clone());
        let shares = ShareEngine::new(db.clone());
        let events = EventEngine::new(db.clone());
        let groups = GroupEngine::new(db.clone());
        let memberships = MembershipEngine::new(db.clone());
        let notifications = NotificationEngine::new(db.clone());
        let subscriptions = crate::subscriptions::SubscriptionEngine::new(db.clone());
        let reactions = ReactionEngine::new(db.clone());
        let outbound_webhooks = OutboundWebhookEngine::new(db.clone());
        let relationships = RelationshipEngine::new(db.clone());
        let settings = SettingsEngine::new(db.clone());
        let preferences = PreferencesEngine::new(db.clone());
        let import = ImportEngine::new(git_arc.clone());
        let ai = AiEngine::new(None, "https://api.openai.com/v1".into(), "gpt-4o-mini".into());
        let unfurl = UnfurlEngine::new();
        let emojis = EmojiEngine::new(db.clone(), repo_path.join("_emojis"));
        let export_jobs = ExportJobEngine::new(db.clone(), git_arc, repo_path.join("_exports"));
        let oauth = OAuthEngine::new(db.clone());

        let config = Config {
            jwt_secret: "test-secret-32-chars-minimum-len!".to_string(),
            ..Config::default()
        };

        AppState {
            config: Arc::new(config),
            db,
            git: Arc::new(git),
            comments: Arc::new(comments),
            search: Arc::new(search),
            cache: Arc::new(cache),
            auth: Arc::new(auth),
            sync: Arc::new(sync),
            rooms,
            rate_limiter,
            collections: Arc::new(collections),
            doc_meta: Arc::new(doc_meta),
            templates: Arc::new(templates),
            attachments: Arc::new(attachments),
            stars: Arc::new(stars),
            pins: Arc::new(pins),
            views: Arc::new(views),
            shares: Arc::new(shares),
            events: Arc::new(events),
            groups: Arc::new(groups),
            memberships: Arc::new(memberships),
            notifications: Arc::new(notifications),
            subscriptions: Arc::new(subscriptions),
            reactions: Arc::new(reactions),
            outbound_webhooks: Arc::new(outbound_webhooks),
            relationships: Arc::new(relationships),
            settings: Arc::new(settings),
            preferences: Arc::new(preferences),
            import: Arc::new(import),
            ai: Arc::new(ai),
            unfurl: Arc::new(unfurl),
            emojis: Arc::new(emojis),
            export_jobs: Arc::new(export_jobs),
            oauth: Arc::new(oauth),
        }
    }
}
