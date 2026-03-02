use chrono::Utc;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::{with_conn, DbConn};
use crate::error::AppError;

// ── Data types ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reaction {
    pub id: String,
    pub comment_id: String,
    pub user_id: String,
    pub emoji: String,
    pub created_at: String,
}

// ── Engine ────────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct ReactionEngine {
    db: DbConn,
}

impl ReactionEngine {
    pub fn new(db: DbConn) -> Self {
        ReactionEngine { db }
    }

    /// Toggle a reaction: add if absent, remove if present.
    /// Returns the reaction if added, None if removed.
    pub async fn toggle(
        &self,
        comment_id: &str,
        user_id: &str,
        emoji: &str,
    ) -> Result<Option<Reaction>, AppError> {
        let db = self.db.clone();
        let comment_id = comment_id.to_string();
        let user_id = user_id.to_string();
        let emoji = emoji.to_string();
        with_conn(&db, move |conn| {
            // Check if reaction exists.
            let existing: Option<String> = {
                use rusqlite::OptionalExtension;
                conn.query_row(
                    "SELECT id FROM reactions WHERE comment_id = ?1 AND user_id = ?2 AND emoji = ?3",
                    params![comment_id, user_id, emoji],
                    |r| r.get(0),
                )
                .optional()?
            };

            if let Some(id) = existing {
                conn.execute("DELETE FROM reactions WHERE id = ?1", params![id])?;
                Ok(None)
            } else {
                let id = Uuid::now_v7().to_string();
                let now = Utc::now().to_rfc3339();
                conn.execute(
                    "INSERT INTO reactions (id, comment_id, user_id, emoji, created_at)
                     VALUES (?1,?2,?3,?4,?5)",
                    params![id, comment_id, user_id, emoji, now],
                )?;
                let r = conn.query_row(
                    "SELECT id, comment_id, user_id, emoji, created_at
                     FROM reactions WHERE id = ?1",
                    params![id],
                    row_to_reaction,
                )?;
                Ok(Some(r))
            }
        })
        .await
    }

    /// List all reactions for a comment.
    pub async fn list_for_comment(&self, comment_id: &str) -> Result<Vec<Reaction>, AppError> {
        let db = self.db.clone();
        let comment_id = comment_id.to_string();
        with_conn(&db, move |conn| {
            let mut stmt = conn.prepare(
                "SELECT id, comment_id, user_id, emoji, created_at
                 FROM reactions WHERE comment_id = ?1
                 ORDER BY created_at ASC",
            )?;
            let rows = stmt.query_map(params![comment_id], row_to_reaction)?;
            rows.collect()
        })
        .await
    }

    /// Delete a reaction by ID, checking ownership.
    pub async fn delete(&self, id: &str, user_id: &str) -> Result<(), AppError> {
        let db = self.db.clone();
        let id = id.to_string();
        let id2 = id.clone();
        let user_id = user_id.to_string();
        with_conn(&db, move |conn| {
            use rusqlite::OptionalExtension;
            let owner: Option<String> = conn
                .query_row("SELECT user_id FROM reactions WHERE id = ?1", params![id], |r| r.get(0))
                .optional()?;
            match owner {
                None => Err(rusqlite::Error::QueryReturnedNoRows),
                Some(ref uid) if uid != &user_id => {
                    Err(rusqlite::Error::InvalidParameterName("forbidden".into()))
                }
                Some(_) => {
                    conn.execute("DELETE FROM reactions WHERE id = ?1", params![id])?;
                    Ok(())
                }
            }
        })
        .await
        .map_err(|e| match e {
            AppError::Db(rusqlite::Error::QueryReturnedNoRows) => {
                AppError::NotFound(format!("reaction {id2} not found"))
            }
            AppError::Db(rusqlite::Error::InvalidParameterName(_)) => {
                AppError::Forbidden("you do not own this reaction".into())
            }
            other => other,
        })
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn row_to_reaction(row: &rusqlite::Row) -> rusqlite::Result<Reaction> {
    Ok(Reaction {
        id: row.get(0)?,
        comment_id: row.get(1)?,
        user_id: row.get(2)?,
        emoji: row.get(3)?,
        created_at: row.get(4)?,
    })
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    fn make_engine() -> (ReactionEngine, String) {
        let db = db::open(":memory:").unwrap();
        let comment_id;
        {
            let conn = db.lock().unwrap();
            conn.execute_batch(
                "INSERT INTO users (id,email,name,password_hash,role,created_at,updated_at)
                 VALUES ('u1','a@b.com','Alice','hash','editor','2024-01-01T00:00:00Z','2024-01-01T00:00:00Z');
                 INSERT INTO users (id,email,name,password_hash,role,created_at,updated_at)
                 VALUES ('u2','b@c.com','Bob','hash','editor','2024-01-01T00:00:00Z','2024-01-01T00:00:00Z');
                 INSERT INTO comments (id,doc_path,author_id,body,is_agent,created_at,updated_at)
                 VALUES ('c1','doc.md','u1','hello',0,'2024-01-01T00:00:00Z','2024-01-01T00:00:00Z');",
            ).unwrap();
            comment_id = "c1".to_string();
        }
        (ReactionEngine::new(db), comment_id)
    }

    #[tokio::test]
    async fn test_toggle_add_reaction() {
        let (e, cid) = make_engine();
        let r = e.toggle(&cid, "u1", "👍").await.unwrap();
        assert!(r.is_some());
        let r = r.unwrap();
        assert_eq!(r.emoji, "👍");
    }

    #[tokio::test]
    async fn test_toggle_remove_reaction() {
        let (e, cid) = make_engine();
        e.toggle(&cid, "u1", "👍").await.unwrap();
        let r = e.toggle(&cid, "u1", "👍").await.unwrap();
        assert!(r.is_none());
    }

    #[tokio::test]
    async fn test_list_for_comment() {
        let (e, cid) = make_engine();
        e.toggle(&cid, "u1", "👍").await.unwrap();
        e.toggle(&cid, "u2", "❤️").await.unwrap();
        let reactions = e.list_for_comment(&cid).await.unwrap();
        assert_eq!(reactions.len(), 2);
    }

    #[tokio::test]
    async fn test_list_empty_for_other_comment() {
        let (e, _) = make_engine();
        let reactions = e.list_for_comment("other-comment").await.unwrap();
        assert!(reactions.is_empty());
    }

    #[tokio::test]
    async fn test_delete_reaction() {
        let (e, cid) = make_engine();
        let r = e.toggle(&cid, "u1", "👍").await.unwrap().unwrap();
        e.delete(&r.id, "u1").await.unwrap();
        let reactions = e.list_for_comment(&cid).await.unwrap();
        assert!(reactions.is_empty());
    }

    #[tokio::test]
    async fn test_delete_not_found() {
        let (e, _) = make_engine();
        let err = e.delete("no-such", "u1").await.unwrap_err();
        assert!(matches!(err, AppError::NotFound(_)));
    }

    #[tokio::test]
    async fn test_delete_wrong_owner() {
        let (e, cid) = make_engine();
        let r = e.toggle(&cid, "u1", "👍").await.unwrap().unwrap();
        let err = e.delete(&r.id, "u2").await.unwrap_err();
        assert!(matches!(err, AppError::Forbidden(_)));
    }

    #[tokio::test]
    async fn test_multiple_emojis_same_user() {
        let (e, cid) = make_engine();
        e.toggle(&cid, "u1", "👍").await.unwrap();
        e.toggle(&cid, "u1", "❤️").await.unwrap();
        let reactions = e.list_for_comment(&cid).await.unwrap();
        assert_eq!(reactions.len(), 2);
    }
}
