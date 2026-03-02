use axum::{extract::State, Json};
use serde_json::{json, Value};

use crate::state::AppState;

pub async fn health_handler(State(state): State<AppState>) -> Json<Value> {
    let db_ok = match state.db.lock() {
        Ok(conn) => conn.execute_batch("SELECT 1").is_ok(),
        Err(_) => false,
    };

    let head = state.git.head_sha().await.ok().flatten();

    Json(json!({
        "status": if db_ok { "ok" } else { "degraded" },
        "db": if db_ok { "ok" } else { "error" },
        "git": {
            "status": "ok",
            "head": head,
        },
        "version": env!("CARGO_PKG_VERSION"),
    }))
}
