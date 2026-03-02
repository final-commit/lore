//! Background scheduled tasks.
use std::sync::Arc;
use tokio::time::{interval, Duration};
use tracing::info;
use crate::db::{with_conn, DbConn};

pub async fn run_all(db: DbConn) -> Result<usize, crate::error::AppError> {
    let mut count = 0usize;
    let db2 = db.clone();
    // 1. Permanently delete docs trashed > 30 days
    let n = with_conn(&db2, |conn| {
        Ok(conn.execute(
            "DELETE FROM document_meta WHERE deleted_at IS NOT NULL AND deleted_at < datetime('now', '-30 days')",
            [],
        )?)
    }).await?;
    count += n;

    // 2. Clean expired sessions
    let n = with_conn(&db, |conn| {
        Ok(conn.execute("DELETE FROM sessions WHERE expires_at < datetime('now')", [])?)
    }).await?;
    count += n;

    // 3. Clean expired API tokens
    let n = with_conn(&db, |conn| {
        Ok(conn.execute("DELETE FROM api_tokens WHERE expires_at IS NOT NULL AND expires_at < datetime('now')", [])?)
    }).await?;
    count += n;

    // 4. Clean events older than 90 days
    let n = with_conn(&db, |conn| {
        Ok(conn.execute("DELETE FROM events WHERE created_at < datetime('now', '-90 days')", [])?)
    }).await?;
    count += n;

    info!(cleaned = count, "cron: cleanup complete");
    Ok(count)
}

/// Start background cron loop. Call from main after server starts.
pub fn start(db: DbConn) {
    tokio::spawn(async move {
        let mut cleanup_interval = interval(Duration::from_secs(300)); // 5 min
        loop {
            cleanup_interval.tick().await;
            if let Err(e) = run_all(db.clone()).await {
                tracing::error!(error=%e, "cron: cleanup error");
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    #[tokio::test]
    async fn test_run_all_empty_db() {
        let db = db::open(":memory:").unwrap();
        let n = run_all(db).await.unwrap();
        assert_eq!(n, 0);
    }

    #[tokio::test]
    async fn test_run_all_cleans_expired_sessions() {
        let db = db::open(":memory:").unwrap();
        { let conn = db.lock().unwrap();
          conn.execute("INSERT INTO users (id,email,name,password_hash,role,created_at,updated_at) VALUES ('u1','a@b.com','A','h','editor','2024-01-01','2024-01-01')", []).unwrap();
          conn.execute("INSERT INTO sessions (id,user_id,refresh_token_hash,expires_at,created_at) VALUES ('s1','u1','hash','2020-01-01T00:00:00Z','2020-01-01T00:00:00Z')", []).unwrap();
        }
        run_all(db.clone()).await.unwrap();
        let count: i64 = { let conn = db.lock().unwrap(); conn.query_row("SELECT COUNT(*) FROM sessions", [], |r| r.get(0)).unwrap() };
        assert_eq!(count, 0);
    }
}
