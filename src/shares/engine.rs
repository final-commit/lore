use chrono::Utc;
use rand::distr::Alphanumeric;
use rand::Rng;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::{with_conn, DbConn};
use crate::error::AppError;

// ── Data types ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Share {
    pub id: String,
    pub doc_path: String,
    pub shared_by: String,
    pub include_child_docs: bool,
    pub published: bool,
    pub url_id: String,
    pub expires_at: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateShare {
    pub doc_path: String,
    pub include_child_docs: Option<bool>,
}

// ── Engine ────────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct ShareEngine {
    db: DbConn,
}

impl ShareEngine {
    pub fn new(db: DbConn) -> Self {
        ShareEngine { db }
    }

    /// Create a new public share link.
    pub async fn create(&self, req: CreateShare, shared_by: &str) -> Result<Share, AppError> {
        let db = self.db.clone();
        let shared_by = shared_by.to_string();
        with_conn(&db, move |conn| {
            let id = Uuid::now_v7().to_string();
            let now = Utc::now().to_rfc3339();
            let url_id = generate_url_id();
            let include_child_docs = req.include_child_docs.unwrap_or(false) as i64;

            conn.execute(
                "INSERT INTO shares (id, doc_path, shared_by, include_child_docs, published, url_id, created_at)
                 VALUES (?1,?2,?3,?4,1,?5,?6)",
                params![id, req.doc_path, shared_by, include_child_docs, url_id, now],
            )?;

            conn.query_row(
                "SELECT id, doc_path, shared_by, include_child_docs, published, url_id, expires_at, created_at
                 FROM shares WHERE id = ?1",
                params![id],
                row_to_share,
            )
        })
        .await
    }

    /// List shares for a specific document.
    pub async fn list_for_doc(&self, doc_path: &str) -> Result<Vec<Share>, AppError> {
        let db = self.db.clone();
        let doc_path = doc_path.to_string();
        with_conn(&db, move |conn| {
            let mut stmt = conn.prepare(
                "SELECT id, doc_path, shared_by, include_child_docs, published, url_id, expires_at, created_at
                 FROM shares WHERE doc_path = ?1 ORDER BY created_at DESC",
            )?;
            let rows = stmt.query_map(params![doc_path], row_to_share)?;
            rows.collect()
        })
        .await
    }

    /// Delete (revoke) a share.
    pub async fn delete(&self, id: &str) -> Result<(), AppError> {
        let db = self.db.clone();
        let id = id.to_string();
        let id2 = id.clone();
        with_conn(&db, move |conn| {
            let rows = conn.execute("DELETE FROM shares WHERE id = ?1", params![id])?;
            if rows == 0 {
                return Err(rusqlite::Error::QueryReturnedNoRows);
            }
            Ok(())
        })
        .await
        .map_err(|e| match e {
            AppError::Db(rusqlite::Error::QueryReturnedNoRows) => {
                AppError::NotFound(format!("share {id2} not found"))
            }
            other => other,
        })
    }

    /// Get a share by its url_id (for public access, no auth required).
    /// Returns NotFound if expired or not published.
    pub async fn get_by_url_id(&self, url_id: &str) -> Result<Share, AppError> {
        let db = self.db.clone();
        let url_id = url_id.to_string();
        with_conn(&db, move |conn| {
            conn.query_row(
                "SELECT id, doc_path, shared_by, include_child_docs, published, url_id, expires_at, created_at
                 FROM shares WHERE url_id = ?1",
                params![url_id],
                row_to_share,
            )
        })
        .await
        .map_err(|e| match e {
            AppError::Db(rusqlite::Error::QueryReturnedNoRows) => {
                AppError::NotFound("share not found".into())
            }
            other => other,
        })
        .and_then(|share| {
            if !share.published {
                return Err(AppError::NotFound("share not found".into()));
            }
            if let Some(ref exp) = share.expires_at {
                if exp.as_str() < Utc::now().to_rfc3339().as_str() {
                    return Err(AppError::NotFound("share has expired".into()));
                }
            }
            Ok(share)
        })
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn row_to_share(row: &rusqlite::Row) -> rusqlite::Result<Share> {
    Ok(Share {
        id: row.get(0)?,
        doc_path: row.get(1)?,
        shared_by: row.get(2)?,
        include_child_docs: row.get::<_, i64>(3)? != 0,
        published: row.get::<_, i64>(4)? != 0,
        url_id: row.get(5)?,
        expires_at: row.get(6)?,
        created_at: row.get(7)?,
    })
}

/// Generate a random 10-character alphanumeric string for use as url_id.
fn generate_url_id() -> String {
    rand::rng().sample_iter(Alphanumeric).take(10).map(char::from).collect()
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    fn make_engine() -> ShareEngine {
        let db = db::open(":memory:").unwrap();
        {
            let conn = db.lock().unwrap();
            conn.execute(
                "INSERT INTO users (id,email,name,password_hash,role,created_at,updated_at)
                 VALUES ('u1','a@b.com','Alice','hash','editor','2024-01-01T00:00:00Z','2024-01-01T00:00:00Z')",
                [],
            ).unwrap();
        }
        ShareEngine::new(db)
    }

    #[tokio::test]
    async fn test_create_share() {
        let engine = make_engine();
        let share = engine
            .create(CreateShare { doc_path: "docs/foo.md".into(), include_child_docs: None }, "u1")
            .await
            .unwrap();
        assert_eq!(share.doc_path, "docs/foo.md");
        assert!(share.published);
        assert!(!share.include_child_docs);
        assert_eq!(share.url_id.len(), 10);
    }

    #[tokio::test]
    async fn test_create_share_with_children() {
        let engine = make_engine();
        let share = engine
            .create(
                CreateShare { doc_path: "docs/section".into(), include_child_docs: Some(true) },
                "u1",
            )
            .await
            .unwrap();
        assert!(share.include_child_docs);
    }

    #[tokio::test]
    async fn test_list_for_doc() {
        let engine = make_engine();
        engine
            .create(CreateShare { doc_path: "docs/foo.md".into(), include_child_docs: None }, "u1")
            .await
            .unwrap();
        engine
            .create(CreateShare { doc_path: "docs/foo.md".into(), include_child_docs: None }, "u1")
            .await
            .unwrap();
        let shares = engine.list_for_doc("docs/foo.md").await.unwrap();
        assert_eq!(shares.len(), 2);
    }

    #[tokio::test]
    async fn test_delete_share() {
        let engine = make_engine();
        let share = engine
            .create(CreateShare { doc_path: "docs/foo.md".into(), include_child_docs: None }, "u1")
            .await
            .unwrap();
        engine.delete(&share.id).await.unwrap();
        let err = engine.delete(&share.id).await.unwrap_err();
        assert!(matches!(err, AppError::NotFound(_)));
    }

    #[tokio::test]
    async fn test_get_by_url_id() {
        let engine = make_engine();
        let share = engine
            .create(CreateShare { doc_path: "docs/foo.md".into(), include_child_docs: None }, "u1")
            .await
            .unwrap();
        let fetched = engine.get_by_url_id(&share.url_id).await.unwrap();
        assert_eq!(fetched.doc_path, "docs/foo.md");
    }

    #[tokio::test]
    async fn test_get_by_invalid_url_id() {
        let engine = make_engine();
        let err = engine.get_by_url_id("nonexistent").await.unwrap_err();
        assert!(matches!(err, AppError::NotFound(_)));
    }

    #[tokio::test]
    async fn test_url_id_is_unique_per_share() {
        let engine = make_engine();
        let s1 = engine
            .create(CreateShare { doc_path: "docs/a.md".into(), include_child_docs: None }, "u1")
            .await
            .unwrap();
        let s2 = engine
            .create(CreateShare { doc_path: "docs/b.md".into(), include_child_docs: None }, "u1")
            .await
            .unwrap();
        assert_ne!(s1.url_id, s2.url_id);
    }
}
