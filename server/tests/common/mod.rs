use std::sync::Arc;
use std::time::Duration;

use forge::attachments::AttachmentEngine;
use forge::auth::handler::AuthService;
use forge::cache::PageCache;
use forge::collections::CollectionEngine;
use forge::comments::CommentEngine;
use forge::config::Config;
use forge::db;
use forge::doc_meta::DocMetaEngine;
use forge::events::EventEngine;
use forge::git::{GitEngine, GitQueue};
use forge::groups::GroupEngine;
use forge::memberships::MembershipEngine;
use forge::notifications::NotificationEngine;
use forge::outbound_webhooks::OutboundWebhookEngine;
use forge::pins::PinEngine;
use forge::rate_limit::RateLimiter;
use forge::reactions::ReactionEngine;
use forge::realtime::new_rooms;
use forge::relationships::RelationshipEngine;
use forge::search::SearchEngine;
use forge::shares::ShareEngine;
use forge::stars::StarEngine;
use forge::state::AppState;
use forge::subscriptions::SubscriptionEngine;
use forge::sync::SyncEngine;
use forge::templates::TemplateEngine;
use forge::views::ViewEngine;
use tempfile::TempDir;

/// A self-contained test context with a temp repo, in-memory DB, and RAM search index.
pub struct TestContext {
    pub _dir: TempDir,
    pub state: AppState,
}

impl TestContext {
    pub async fn new() -> Self {
        let dir = TempDir::new().expect("temp dir");
        let repo_path = dir.path().join("repo");
        let db_path = dir.path().join("forge.db");

        std::fs::create_dir_all(&repo_path).unwrap();

        let db = db::open(db_path.to_str().unwrap()).unwrap();
        let queue = GitQueue::new();
        let git = GitEngine::init(repo_path.clone(), queue.clone()).unwrap();
        let search = SearchEngine::open_in_ram().unwrap();
        let cache = PageCache::new(100, Duration::from_secs(60));
        let auth = AuthService::new(db.clone(), "test-secret-32-chars-minimum-len!".to_string());
        let comments = CommentEngine::new(db.clone());
        let sync = SyncEngine::new(db.clone(), repo_path.clone(), queue.clone());
        let rooms = new_rooms();
        let rate_limiter = RateLimiter::new(20);
        let collections = CollectionEngine::new(db.clone());
        let doc_meta = DocMetaEngine::new(db.clone());
        let templates = TemplateEngine::new(db.clone());
        let attachments =
            AttachmentEngine::new(db.clone(), repo_path.clone(), 10 * 1024 * 1024);
        // Sprint 2 engines
        let stars = StarEngine::new(db.clone());
        let pins = PinEngine::new(db.clone());
        let views = ViewEngine::new(db.clone());
        let shares = ShareEngine::new(db.clone());
        let events = EventEngine::new(db.clone());
        // Sprint 3+4 engines
        let groups = GroupEngine::new(db.clone());
        let memberships = MembershipEngine::new(db.clone());
        let notifications = NotificationEngine::new(db.clone());
        let subscriptions = SubscriptionEngine::new(db.clone());
        let reactions = ReactionEngine::new(db.clone());
        let outbound_webhooks = OutboundWebhookEngine::new(db.clone());
        let relationships = RelationshipEngine::new(db.clone());

        let config = Config {
            jwt_secret: "test-secret-32-chars-minimum-len!".to_string(),
            ..Config::default()
        };

        let state = AppState {
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
            // Sprint 3+4
            groups: Arc::new(groups),
            memberships: Arc::new(memberships),
            notifications: Arc::new(notifications),
            subscriptions: Arc::new(subscriptions),
            reactions: Arc::new(reactions),
            outbound_webhooks: Arc::new(outbound_webhooks),
            relationships: Arc::new(relationships),
        };

        TestContext { _dir: dir, state }
    }
}
