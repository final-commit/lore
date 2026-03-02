use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use axum::http::{header, HeaderValue, Method};
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use forge::api;
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

    // ── Rate limiter (20 req/min per IP on auth endpoints) ─────────────────
    let rate_limiter = RateLimiter::new(20);

    // ── Sprint 1 engines ───────────────────────────────────────────────────
    let collections = CollectionEngine::new(db.clone());
    let doc_meta = DocMetaEngine::new(db.clone());
    let templates = TemplateEngine::new(db.clone());
    let attachments =
        AttachmentEngine::new(db.clone(), repo_path.clone(), config.max_upload_bytes);

    // ── Sprint 2 engines ───────────────────────────────────────────────────
    let stars = StarEngine::new(db.clone());
    let pins = PinEngine::new(db.clone());
    let views = ViewEngine::new(db.clone());
    let shares = ShareEngine::new(db.clone());
    let events = EventEngine::new(db.clone());

    // ── Sprint 3+4 engines ─────────────────────────────────────────────────
    let groups = GroupEngine::new(db.clone());
    let memberships = MembershipEngine::new(db.clone());
    let notifications = NotificationEngine::new(db.clone());
    let subscriptions = SubscriptionEngine::new(db.clone());
    let reactions = ReactionEngine::new(db.clone());
    let outbound_webhooks = OutboundWebhookEngine::new(db.clone());
    let relationships = RelationshipEngine::new(db.clone());

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

    // ── P1 #13: CORS — build from configured origins ───────────────────────
    let allowed_origins: Vec<HeaderValue> = config
        .cors_origins
        .iter()
        .filter_map(|o| HeaderValue::from_str(o).ok())
        .collect();

    let cors = CorsLayer::new()
        .allow_origin(allowed_origins)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE]);

    // ── Router ─────────────────────────────────────────────────────────────
    // WebSocket route is now inside api::router (with state) — no extra route needed.
    let app = api::router(state)
        .layer(TraceLayer::new_for_http())
        .layer(cors);

    let addr: SocketAddr = format!("{}:{}", config.host, config.port).parse()?;
    tracing::info!(%addr, "listening");

    let listener = tokio::net::TcpListener::bind(addr).await?;

    // P1 #14: graceful shutdown on SIGINT / SIGTERM.
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    tracing::info!("server shut down cleanly");
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c().await.expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => { tracing::info!("received Ctrl+C"); },
        _ = terminate => { tracing::info!("received SIGTERM"); },
    }
}
