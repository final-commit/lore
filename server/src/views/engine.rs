use chrono::Utc;
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::{with_conn, DbConn};
use crate::error::AppError;

// ── Data types ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct View {
    pub id: String,
    pub doc_path: String,
    pub user_id: String,
    pub count: i64,
    pub last_viewed_at: String,
    pub created_at: String,
}

// ── Engine ────────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct ViewEngine {
    db: DbConn,
}

impl ViewEngine {
    pub fn new(db: DbConn) -> Self {
        ViewEngine { db }
    }

    /// Record a view for user+doc (upsert: increment count, update last_viewed_at).
    /// Returns the updated view record.
    pub async fn record(&self, user_id: &str, doc_path: &str) -> Result<View, AppError> {
        let db = self.db.clone();
        let user_id = user_id.to_string();
        let doc_path = doc_path.to_string();
        with_conn(&db, move |conn| {
            let now = Utc::now().to_rfc3339();
            // Try to find existing record.
            let existing: Option<String> = conn
                .query_row(
                    "SELECT id FROM views WHERE doc_path = ?1 AND user_id = ?2",
                    params![doc_path, user_id],
                    |row| row.get(0),
                )
                .optional()?;

            if let Some(ref id) = existing {
                conn.execute(
                    "UPDATE views SET count = count + 1, last_viewed_at = ?1 WHERE id = ?2",
                    params![now, id],
                )?;
                conn.query_row(
                    "SELECT id, doc_path, user_id, count, last_viewed_at, created_at FROM views WHERE id = ?1",
                    params![id],
                    row_to_view,
                )
            } else {
                let id = Uuid::now_v7().to_string();
                conn.execute(
                    "INSERT INTO views (id, doc_path, user_id, count, last_viewed_at, created_at)
                     VALUES (?1,?2,?3,1,?4,?4)",
                    params![id, doc_path, user_id, now],
                )?;
                conn.query_row(
                    "SELECT id, doc_path, user_id, count, last_viewed_at, created_at FROM views WHERE id = ?1",
                    params![id],
                    row_to_view,
                )
            }
        })
        .await
    }

    /// List all viewers for a specific document.
    pub async fn list_for_doc(&self, doc_path: &str) -> Result<Vec<View>, AppError> {
        let db = self.db.clone();
        let doc_path = doc_path.to_string();
        with_conn(&db, move |conn| {
            let mut stmt = conn.prepare(
                "SELECT id, doc_path, user_id, count, last_viewed_at, created_at
                 FROM views WHERE doc_path = ?1 ORDER BY last_viewed_at DESC",
            )?;
            let rows = stmt.query_map(params![doc_path], row_to_view)?;
            rows.collect()
        })
        .await
    }

    /// List recently viewed docs for a user, ordered by last_viewed_at desc.
    pub async fn list_recent_for_user(&self, user_id: &str, limit: i64) -> Result<Vec<View>, AppError> {
        let db = self.db.clone();
        let user_id = user_id.to_string();
        with_conn(&db, move |conn| {
            let mut stmt = conn.prepare(
                "SELECT id, doc_path, user_id, count, last_viewed_at, created_at
                 FROM views WHERE user_id = ?1 ORDER BY last_viewed_at DESC LIMIT ?2",
            )?;
            let rows = stmt.query_map(params![user_id, limit], row_to_view)?;
            rows.collect()
        })
        .await
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn row_to_view(row: &rusqlite::Row) -> rusqlite::Result<View> {
    Ok(View {
        id: row.get(0)?,
        doc_path: row.get(1)?,
        user_id: row.get(2)?,
        count: row.get(3)?,
        last_viewed_at: row.get(4)?,
        created_at: row.get(5)?,
    })
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    fn make_engine() -> ViewEngine {
        let db = db::open(":memory:").unwrap();
        {
            let conn = db.lock().unwrap();
            conn.execute(
                "INSERT INTO users (id,email,name,password_hash,role,created_at,updated_at)
                 VALUES ('u1','a@b.com','Alice','hash','editor','2024-01-01T00:00:00Z','2024-01-01T00:00:00Z')",
                [],
            ).unwrap();
        }
        ViewEngine::new(db)
    }

    #[tokio::test]
    async fn test_record_creates_view() {
        let engine = make_engine();
        let v = engine.record("u1", "docs/foo.md").await.unwrap();
        assert_eq!(v.count, 1);
        assert_eq!(v.doc_path, "docs/foo.md");
    }

    #[tokio::test]
    async fn test_record_increments_count() {
        let engine = make_engine();
        engine.record("u1", "docs/foo.md").await.unwrap();
        let v = engine.record("u1", "docs/foo.md").await.unwrap();
        assert_eq!(v.count, 2);
    }

    #[tokio::test]
    async fn test_list_for_doc() {
        let engine = make_engine();
        engine.record("u1", "docs/foo.md").await.unwrap();
        let views = engine.list_for_doc("docs/foo.md").await.unwrap();
        assert_eq!(views.len(), 1);
    }

    #[tokio::test]
    async fn test_list_for_doc_empty() {
        let engine = make_engine();
        let views = engine.list_for_doc("docs/nonexistent.md").await.unwrap();
        assert!(views.is_empty());
    }

    #[tokio::test]
    async fn test_list_recent_for_user() {
        let engine = make_engine();
        engine.record("u1", "docs/a.md").await.unwrap();
        engine.record("u1", "docs/b.md").await.unwrap();
        let views = engine.list_recent_for_user("u1", 10).await.unwrap();
        assert_eq!(views.len(), 2);
    }

    #[tokio::test]
    async fn test_list_recent_limit() {
        let engine = make_engine();
        for i in 0..5 {
            engine.record("u1", &format!("docs/{i}.md")).await.unwrap();
        }
        let views = engine.list_recent_for_user("u1", 3).await.unwrap();
        assert_eq!(views.len(), 3);
    }
}
