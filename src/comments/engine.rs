use chrono::Utc;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::{with_conn, DbConn};
use crate::error::AppError;

// ── Data types ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comment {
    pub id: String,
    pub doc_path: String,
    pub parent_id: Option<String>,
    pub author_id: String,
    pub body: String,
    pub anchor_text: Option<String>,
    pub anchor_start: Option<i64>,
    pub anchor_end: Option<i64>,
    pub resolved_at: Option<String>,
    pub resolved_by: Option<String>,
    pub is_agent: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateComment {
    pub doc_path: String,
    pub parent_id: Option<String>,
    pub author_id: String,
    pub body: String,
    pub anchor_text: Option<String>,
    pub anchor_start: Option<i64>,
    pub anchor_end: Option<i64>,
    pub is_agent: bool,
}

// ── Engine ────────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct CommentEngine {
    db: DbConn,
}

impl CommentEngine {
    pub fn new(db: DbConn) -> Self {
        CommentEngine { db }
    }

    /// Create a new comment.
    pub async fn create(&self, req: CreateComment) -> Result<Comment, AppError> {
        let db = self.db.clone();
        with_conn(&db, move |conn| {
            let id = Uuid::now_v7().to_string();
            let now = Utc::now().to_rfc3339();

            conn.execute(
                r#"INSERT INTO comments
                   (id, doc_path, parent_id, author_id, body, anchor_text,
                    anchor_start, anchor_end, is_agent, created_at, updated_at)
                   VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?10)"#,
                params![
                    id,
                    req.doc_path,
                    req.parent_id,
                    req.author_id,
                    req.body,
                    req.anchor_text,
                    req.anchor_start,
                    req.anchor_end,
                    req.is_agent as i64,
                    now,
                ],
            )?;

            Ok(Comment {
                id,
                doc_path: req.doc_path,
                parent_id: req.parent_id,
                author_id: req.author_id,
                body: req.body,
                anchor_text: req.anchor_text,
                anchor_start: req.anchor_start,
                anchor_end: req.anchor_end,
                resolved_at: None,
                resolved_by: None,
                is_agent: req.is_agent,
                created_at: now.clone(),
                updated_at: now,
            })
        })
        .await
        .map_err(AppError::Db)
    }

    /// List all comments for a document, ordered by creation time.
    pub async fn list(&self, doc_path: &str) -> Result<Vec<Comment>, AppError> {
        let db = self.db.clone();
        let doc_path = doc_path.to_string();
        with_conn(&db, move |conn| {
            let mut stmt = conn.prepare(
                r#"SELECT id, doc_path, parent_id, author_id, body, anchor_text,
                          anchor_start, anchor_end, resolved_at, resolved_by,
                          is_agent, created_at, updated_at
                   FROM comments
                   WHERE doc_path = ?1
                   ORDER BY created_at ASC"#,
            )?;

            let rows = stmt.query_map(params![doc_path], row_to_comment)?;
            rows.collect()
        })
        .await
        .map_err(AppError::Db)
    }

    /// Get a single comment by ID.
    pub async fn get(&self, id: &str) -> Result<Comment, AppError> {
        let db = self.db.clone();
        let id = id.to_string();
        let id2 = id.clone();
        with_conn(&db, move |conn| {
            conn.query_row(
                r#"SELECT id, doc_path, parent_id, author_id, body, anchor_text,
                          anchor_start, anchor_end, resolved_at, resolved_by,
                          is_agent, created_at, updated_at
                   FROM comments WHERE id = ?1"#,
                params![id],
                row_to_comment,
            )
        })
        .await
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => AppError::NotFound(format!("comment {id2} not found")),
            other => AppError::Db(other),
        })
    }

    /// Update the body of a comment.  Returns the updated comment.
    pub async fn update_body(&self, id: &str, new_body: &str) -> Result<Comment, AppError> {
        let db = self.db.clone();
        let id = id.to_string();
        let id2 = id.clone();
        let new_body = new_body.to_string();
        with_conn(&db, move |conn| {
            let now = Utc::now().to_rfc3339();
            let rows = conn.execute(
                "UPDATE comments SET body=?1, updated_at=?2 WHERE id=?3",
                params![new_body, now, id],
            )?;
            if rows == 0 {
                return Err(rusqlite::Error::QueryReturnedNoRows);
            }
            conn.query_row(
                r#"SELECT id, doc_path, parent_id, author_id, body, anchor_text,
                          anchor_start, anchor_end, resolved_at, resolved_by,
                          is_agent, created_at, updated_at
                   FROM comments WHERE id = ?1"#,
                params![id2],
                row_to_comment,
            )
        })
        .await
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => AppError::NotFound(format!("comment not found")),
            other => AppError::Db(other),
        })
    }

    /// Delete a comment (cascades to replies).
    pub async fn delete(&self, id: &str) -> Result<(), AppError> {
        let db = self.db.clone();
        let id = id.to_string();
        let id2 = id.clone();
        with_conn(&db, move |conn| {
            let rows = conn.execute("DELETE FROM comments WHERE id=?1", params![id])?;
            if rows == 0 {
                return Err(rusqlite::Error::QueryReturnedNoRows);
            }
            Ok(())
        })
        .await
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => AppError::NotFound(format!("comment {id2} not found")),
            other => AppError::Db(other),
        })
    }

    /// Resolve a comment thread (marks root comment resolved).
    pub async fn resolve(&self, id: &str, resolved_by: &str) -> Result<Comment, AppError> {
        let db = self.db.clone();
        let id = id.to_string();
        let resolved_by = resolved_by.to_string();
        with_conn(&db, move |conn| {
            let now = Utc::now().to_rfc3339();
            let id2 = id.clone();
            let rows = conn.execute(
                "UPDATE comments SET resolved_at=?1, resolved_by=?2 WHERE id=?3",
                params![now, resolved_by, id],
            )?;
            if rows == 0 {
                return Err(rusqlite::Error::QueryReturnedNoRows);
            }
            conn.query_row(
                r#"SELECT id, doc_path, parent_id, author_id, body, anchor_text,
                          anchor_start, anchor_end, resolved_at, resolved_by,
                          is_agent, created_at, updated_at
                   FROM comments WHERE id = ?1"#,
                params![id2],
                row_to_comment,
            )
        })
        .await
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => AppError::NotFound("comment not found".into()),
            other => AppError::Db(other),
        })
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn row_to_comment(row: &rusqlite::Row) -> rusqlite::Result<Comment> {
    Ok(Comment {
        id: row.get(0)?,
        doc_path: row.get(1)?,
        parent_id: row.get(2)?,
        author_id: row.get(3)?,
        body: row.get(4)?,
        anchor_text: row.get(5)?,
        anchor_start: row.get(6)?,
        anchor_end: row.get(7)?,
        resolved_at: row.get(8)?,
        resolved_by: row.get(9)?,
        is_agent: row.get::<_, i64>(10)? != 0,
        created_at: row.get(11)?,
        updated_at: row.get(12)?,
    })
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    fn make_engine() -> CommentEngine {
        let db = db::open(":memory:").unwrap();
        // Insert test users to satisfy FK constraints on comments.author_id / resolved_by.
        {
            let conn = db.lock().unwrap();
            conn.execute_batch(
                "INSERT INTO users (id, email, name, password_hash, role, created_at, updated_at) \
                 VALUES ('user-1', 'user1@example.com', 'User One', 'hash', 'editor', '2024-01-01T00:00:00Z', '2024-01-01T00:00:00Z'); \
                 INSERT INTO users (id, email, name, password_hash, role, created_at, updated_at) \
                 VALUES ('user-2', 'user2@example.com', 'User Two', 'hash', 'editor', '2024-01-01T00:00:00Z', '2024-01-01T00:00:00Z');",
            ).unwrap();
        }
        CommentEngine::new(db)
    }

    fn make_req(doc_path: &str, body: &str) -> CreateComment {
        CreateComment {
            doc_path: doc_path.to_string(),
            parent_id: None,
            author_id: "user-1".to_string(),
            body: body.to_string(),
            anchor_text: None,
            anchor_start: None,
            anchor_end: None,
            is_agent: false,
        }
    }

    #[tokio::test]
    async fn test_create_and_list() {
        let engine = make_engine();
        let c = engine.create(make_req("docs/a.md", "hello")).await.unwrap();
        assert_eq!(c.body, "hello");

        let list = engine.list("docs/a.md").await.unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, c.id);
    }

    #[tokio::test]
    async fn test_list_different_docs() {
        let engine = make_engine();
        engine.create(make_req("a.md", "on a")).await.unwrap();
        engine.create(make_req("b.md", "on b")).await.unwrap();

        let for_a = engine.list("a.md").await.unwrap();
        assert_eq!(for_a.len(), 1);
        let for_b = engine.list("b.md").await.unwrap();
        assert_eq!(for_b.len(), 1);
    }

    #[tokio::test]
    async fn test_reply_threading() {
        let engine = make_engine();
        let root = engine.create(make_req("doc.md", "root")).await.unwrap();

        let reply_req = CreateComment {
            parent_id: Some(root.id.clone()),
            ..make_req("doc.md", "reply")
        };
        let reply = engine.create(reply_req).await.unwrap();

        assert_eq!(reply.parent_id, Some(root.id.clone()));

        let list = engine.list("doc.md").await.unwrap();
        assert_eq!(list.len(), 2);
    }

    #[tokio::test]
    async fn test_get_comment() {
        let engine = make_engine();
        let c = engine.create(make_req("doc.md", "body")).await.unwrap();
        let fetched = engine.get(&c.id).await.unwrap();
        assert_eq!(fetched.body, "body");
    }

    #[tokio::test]
    async fn test_get_missing_comment() {
        let engine = make_engine();
        let err = engine.get("no-such-id").await.unwrap_err();
        assert!(matches!(err, AppError::NotFound(_)));
    }

    #[tokio::test]
    async fn test_update_body() {
        let engine = make_engine();
        let c = engine.create(make_req("doc.md", "original")).await.unwrap();
        let updated = engine.update_body(&c.id, "revised").await.unwrap();
        assert_eq!(updated.body, "revised");
    }

    #[tokio::test]
    async fn test_delete_comment() {
        let engine = make_engine();
        let c = engine.create(make_req("doc.md", "bye")).await.unwrap();
        engine.delete(&c.id).await.unwrap();
        let err = engine.get(&c.id).await.unwrap_err();
        assert!(matches!(err, AppError::NotFound(_)));
    }

    #[tokio::test]
    async fn test_resolve_comment() {
        let engine = make_engine();
        let c = engine.create(make_req("doc.md", "needs resolve")).await.unwrap();
        let resolved = engine.resolve(&c.id, "user-2").await.unwrap();
        assert!(resolved.resolved_at.is_some());
        assert_eq!(resolved.resolved_by, Some("user-2".to_string()));
    }

    #[tokio::test]
    async fn test_anchor_stored() {
        let engine = make_engine();
        let req = CreateComment {
            anchor_text: Some("important text".to_string()),
            anchor_start: Some(10),
            anchor_end: Some(24),
            ..make_req("doc.md", "anchored comment")
        };
        let c = engine.create(req).await.unwrap();
        assert_eq!(c.anchor_text, Some("important text".to_string()));
        assert_eq!(c.anchor_start, Some(10));
        assert_eq!(c.anchor_end, Some(24));
    }
}
