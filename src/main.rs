use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use axum::routing::get;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use forge::api;
use forge::auth::handler::AuthService;
use forge::cache::PageCache;
use forge::comments::CommentEngine;
use forge::config::Config;
use forge::db;
use forge::git::{GitEngine, GitQueue};
use forge::realtime::{new_rooms, yjs_ws_handler};
use forge::search::SearchEngine;
use forge::state::AppState;
use forge::sync::SyncEngine;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = Config::load().expect("failed to load config");

    // ── Logging ────────────────────────────────────────────────────────────
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| {
            EnvFilter::new(format!("forge={},tower_http=debug", config.log_level))
        }))
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!(port = config.port, "starting Forge server");

    // ── Database ───────────────────────────────────────────────────────────
    let db = db::open(&config.db_path)?;

    // ── Git engine ─────────────────────────────────────────────────────────
    let repo_path = PathBuf::from(&config.repo_path);
    let queue = GitQueue::new();
    let git = if repo_path.join(".git").exists() {
        GitEngine::open(repo_path.clone(), queue.clone())?
    } else {
        std::fs::create_dir_all(&repo_path)?;
        GitEngine::init(repo_path.clone(), queue.clone())?
    };

    // ── Search engine ──────────────────────────────────────────────────────
    let search = SearchEngine::open(PathBuf::from(&config.search_index_path))?;

    // ── Cache ──────────────────────────────────────────────────────────────
    let cache = PageCache::new(1000, Duration::from_secs(3600));

    // ── Auth service ───────────────────────────────────────────────────────
    let auth = AuthService::new(db.clone(), config.jwt_secret.clone());

    // ── Comments engine ────────────────────────────────────────────────────
    let comments = CommentEngine::new(db.clone());

    // ── Sync engine ────────────────────────────────────────────────────────
    let sync = SyncEngine::new(db.clone(), repo_path.clone(), queue.clone());

    // ── Realtime rooms ─────────────────────────────────────────────────────
    let rooms = new_rooms();

    let state = AppState {
        config: Arc::new(config.clone()),
        db,
        git: Arc::new(git),
        comments: Arc::new(comments),
        search: Arc::new(search),
        cache: Arc::new(cache),
        auth: Arc::new(auth),
        sync: Arc::new(sync),
        rooms,
    };

    // ── CORS ───────────────────────────────────────────────────────────────
    let cors = CorsLayer::permissive();

    // ── Router ─────────────────────────────────────────────────────────────
    let app = api::router(state.clone())
        .route(
            "/ws/yjs/:doc_path",
            get({
                let st = state.clone();
                move |ws, path| yjs_ws_handler(ws, path, axum::extract::State(st))
            }),
        )
        .layer(TraceLayer::new_for_http())
        .layer(cors);

    let addr: SocketAddr = format!("{}:{}", config.host, config.port).parse()?;
    tracing::info!(%addr, "listening");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
