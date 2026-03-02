use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use forge::auth::handler::AuthService;
use forge::cache::PageCache;
use forge::comments::CommentEngine;
use forge::config::Config;
use forge::db;
use forge::git::{GitEngine, GitQueue};
use forge::realtime::new_rooms;
use forge::search::SearchEngine;
use forge::state::AppState;
use forge::sync::SyncEngine;
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
        };

        TestContext { _dir: dir, state }
    }
}
