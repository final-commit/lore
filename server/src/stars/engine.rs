use chrono::Utc;
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::{with_conn, DbConn};
use crate::error::AppError;

// ── Data types ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Star {
    pub id: String,
    pub user_id: String,
    pub doc_path: String,
    pub created_at: String,
}

// ── Engine ────────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct StarEngine {
    db: DbConn,
}

impl StarEngine {
    pub fn new(db: DbConn) -> Self {
        StarEngine { db }
    }

    /// Toggle a star for the given user+doc_path.
    /// Returns the star if it was created, or None if it was removed.
    pub async fn toggle(&self, user_id: &str, doc_path: &str) -> Result<Option<Star>, AppError> {
        let db = self.db.clone();
        let user_id = user_id.to_string();
        let doc_path = doc_path.to_string();
        with_conn(&db, move |conn| {
            // Check if star exists already.
            let existing: Option<String> = conn
                .query_row(
                    "SELECT id FROM stars WHERE user_id = ?1 AND doc_path = ?2",
                    params![user_id, doc_path],
                    |row| row.get(0),
                )
                .optional()?;

            if let Some(id) = existing {
                // Remove star.
                conn.execute("DELETE FROM stars WHERE id = ?1", params![id])?;
                Ok(None)
            } else {
                // Create star.
                let id = Uuid::now_v7().to_string();
                let now = Utc::now().to_rfc3339();
                conn.execute(
                    "INSERT INTO stars (id, user_id, doc_path, created_at) VALUES (?1,?2,?3,?4)",
                    params![id, user_id, doc_path, now],
                )?;
                let star = conn.query_row(
                    "SELECT id, user_id, doc_path, created_at FROM stars WHERE id = ?1",
                    params![id],
                    row_to_star,
                )?;
                Ok(Some(star))
            }
        })
        .await
    }

    /// List all stars for the given user, most recent first.
    pub async fn list_for_user(&self, user_id: &str) -> Result<Vec<Star>, AppError> {
        let db = self.db.clone();
        let user_id = user_id.to_string();
        with_conn(&db, move |conn| {
            let mut stmt = conn.prepare(
                "SELECT id, user_id, doc_path, created_at FROM stars WHERE user_id = ?1 ORDER BY created_at DESC",
            )?;
            let rows = stmt.query_map(params![user_id], row_to_star)?;
            rows.collect()
        })
        .await
    }

    /// Delete a star by ID, checking ownership.
    pub async fn delete(&self, id: &str, user_id: &str) -> Result<(), AppError> {
        let db = self.db.clone();
        let id = id.to_string();
        let id2 = id.clone();
        let user_id = user_id.to_string();
        with_conn(&db, move |conn| {
            // Verify ownership first.
            let owner: Option<String> = conn
                .query_row("SELECT user_id FROM stars WHERE id = ?1", params![id], |row| row.get(0))
                .optional()?;
            match owner {
                None => Err(rusqlite::Error::QueryReturnedNoRows),
                Some(ref uid) if uid != &user_id => {
                    Err(rusqlite::Error::InvalidParameterName("forbidden".into()))
                }
                Some(_) => {
                    conn.execute("DELETE FROM stars WHERE id = ?1", params![id])?;
                    Ok(())
                }
            }
        })
        .await
        .map_err(|e| match e {
            AppError::Db(rusqlite::Error::QueryReturnedNoRows) => {
                AppError::NotFound(format!("star {id2} not found"))
            }
            AppError::Db(rusqlite::Error::InvalidParameterName(_)) => {
                AppError::Forbidden("you do not own this star".into())
            }
            other => other,
        })
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn row_to_star(row: &rusqlite::Row) -> rusqlite::Result<Star> {
    Ok(Star {
        id: row.get(0)?,
        user_id: row.get(1)?,
        doc_path: row.get(2)?,
        created_at: row.get(3)?,
    })
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    fn make_engine() -> StarEngine {
        let db = db::open(":memory:").unwrap();
        // Insert a test user so FK constraints are satisfied.
        {
            let conn = db.lock().unwrap();
            conn.execute(
                "INSERT INTO users (id, email, name, password_hash, role, created_at, updated_at)
                 VALUES ('u1','a@b.com','Alice','hash','editor','2024-01-01T00:00:00Z','2024-01-01T00:00:00Z')",
                [],
            ).unwrap();
        }
        StarEngine::new(db)
    }

    #[tokio::test]
    async fn test_toggle_creates_star() {
        let engine = make_engine();
        let star = engine.toggle("u1", "docs/foo.md").await.unwrap();
        assert!(star.is_some());
        let star = star.unwrap();
        assert_eq!(star.user_id, "u1");
        assert_eq!(star.doc_path, "docs/foo.md");
    }

    #[tokio::test]
    async fn test_toggle_removes_star() {
        let engine = make_engine();
        // First toggle creates.
        engine.toggle("u1", "docs/foo.md").await.unwrap();
        // Second toggle removes.
        let result = engine.toggle("u1", "docs/foo.md").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_list_for_user() {
        let engine = make_engine();
        engine.toggle("u1", "docs/a.md").await.unwrap();
        engine.toggle("u1", "docs/b.md").await.unwrap();
        let stars = engine.list_for_user("u1").await.unwrap();
        assert_eq!(stars.len(), 2);
    }

    #[tokio::test]
    async fn test_list_empty_for_other_user() {
        let engine = make_engine();
        engine.toggle("u1", "docs/a.md").await.unwrap();
        let stars = engine.list_for_user("other").await.unwrap();
        assert!(stars.is_empty());
    }

    #[tokio::test]
    async fn test_delete_star() {
        let engine = make_engine();
        let star = engine.toggle("u1", "docs/foo.md").await.unwrap().unwrap();
        engine.delete(&star.id, "u1").await.unwrap();
        let remaining = engine.list_for_user("u1").await.unwrap();
        assert!(remaining.is_empty());
    }

    #[tokio::test]
    async fn test_delete_not_found() {
        let engine = make_engine();
        let err = engine.delete("no-such-id", "u1").await.unwrap_err();
        assert!(matches!(err, AppError::NotFound(_)));
    }

    #[tokio::test]
    async fn test_delete_wrong_owner() {
        let engine = make_engine();
        let star = engine.toggle("u1", "docs/foo.md").await.unwrap().unwrap();
        let err = engine.delete(&star.id, "other-user").await.unwrap_err();
        assert!(matches!(err, AppError::Forbidden(_)));
    }
}
