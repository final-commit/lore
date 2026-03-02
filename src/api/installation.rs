use axum::{extract::State, Json};
use serde::Serialize;
use crate::state::AppState;

#[derive(Serialize)]
pub struct InstallationStatus {
    pub setup_complete: bool,
    pub user_count: i64,
    pub version: String,
    pub git_connected: bool,
}

pub async fn installation_status(State(state): State<AppState>) -> Json<InstallationStatus> {
    let user_count = {
        let conn = state.db.lock().unwrap();
        conn.query_row("SELECT COUNT(*) FROM users", [], |r| r.get::<_,i64>(0)).unwrap_or(0)
    };
    let git_connected = state.git.head_sha().await.ok().flatten().is_some();
    Json(InstallationStatus {
        setup_complete: user_count > 0,
        user_count,
        version: env!("CARGO_PKG_VERSION").to_string(),
        git_connected,
    })
}
