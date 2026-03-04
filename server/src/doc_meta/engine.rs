use chrono::Utc;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::{with_conn, DbConn};
use crate::error::AppError;

// ── Data types ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocMeta {
    pub id: String,
    pub doc_path: String,
    /// "draft" or "published"
    pub status: String,
    pub published_at: Option<String>,
    pub created_by: String,
    pub template_id: Option<String>,
    pub archived_at: Option<String>,
    pub deleted_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

// ── Engine ────────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct DocMetaEngine {
    db: DbConn,
}

impl DocMetaEngine {
    pub fn new(db: DbConn) -> Self {
        DocMetaEngine { db }
    }

    /// Get metadata for a document, or create a draft record if none exists.
    pub async fn get_or_create(
        &self,
        doc_path: &str,
        created_by: &str,
    ) -> Result<DocMeta, AppError> {
        let db = self.db.clone();
        let doc_path = doc_path.to_string();
        let created_by = created_by.to_string();
        with_conn(&db, move |conn| {
            let now = Utc::now().to_rfc3339();
            let id = Uuid::now_v7().to_string();
            // Upsert: insert if not exists, do nothing on conflict.
            conn.execute(
                r#"INSERT INTO document_meta
                   (id, doc_path, status, created_by, created_at, updated_at)
                   VALUES (?1, ?2, 'draft', ?3, ?4, ?4)
                   ON CONFLICT(doc_path) DO NOTHING"#,
                params![id, doc_path, created_by, now],
            )?;
            conn.query_row(
                r#"SELECT id, doc_path, status, published_at, created_by, template_id,
                          archived_at, deleted_at, created_at, updated_at
                   FROM document_meta WHERE doc_path = ?1"#,
                params![doc_path],
                row_to_meta,
            )
        })
        .await
    }

    /// Get metadata for a document by path.
    pub async fn get(&self, doc_path: &str) -> Result<DocMeta, AppError> {
        let db = self.db.clone();
        let doc_path = doc_path.to_string();
        let doc_path2 = doc_path.clone();
        with_conn(&db, move |conn| {
            conn.query_row(
                r#"SELECT id, doc_path, status, published_at, created_by, template_id,
                          archived_at, deleted_at, created_at, updated_at
                   FROM document_meta WHERE doc_path = ?1"#,
                params![doc_path],
                row_to_meta,
            )
        })
        .await
        .map_err(|e| match e {
            AppError::Db(rusqlite::Error::QueryReturnedNoRows) => {
                AppError::NotFound(format!("no metadata for {doc_path2}"))
            }
            other => other,
        })
    }

    /// Publish a document. Creates metadata if needed.
    pub async fn publish(&self, doc_path: &str, user_id: &str) -> Result<DocMeta, AppError> {
        let db = self.db.clone();
        let doc_path = doc_path.to_string();
        let user_id = user_id.to_string();
        with_conn(&db, move |conn| {
            let now = Utc::now().to_rfc3339();
            let id = Uuid::now_v7().to_string();
            conn.execute(
                r#"INSERT INTO document_meta
                   (id, doc_path, status, published_at, created_by, created_at, updated_at)
                   VALUES (?1, ?2, 'published', ?3, ?4, ?3, ?3)
                   ON CONFLICT(doc_path) DO UPDATE SET
                       status = 'published',
                       published_at = ?3,
                       archived_at = NULL,
                       deleted_at = NULL,
                       updated_at = ?3"#,
                params![id, doc_path, now, user_id],
            )?;
            conn.query_row(
                r#"SELECT id, doc_path, status, published_at, created_by, template_id,
                          archived_at, deleted_at, created_at, updated_at
                   FROM document_meta WHERE doc_path = ?1"#,
                params![doc_path],
                row_to_meta,
            )
        })
        .await
    }

    /// Unpublish a document (revert to draft).
    pub async fn unpublish(&self, doc_path: &str, user_id: &str) -> Result<DocMeta, AppError> {
        let db = self.db.clone();
        let doc_path = doc_path.to_string();
        let user_id = user_id.to_string();
        with_conn(&db, move |conn| {
            let now = Utc::now().to_rfc3339();
            let id = Uuid::now_v7().to_string();
            conn.execute(
                r#"INSERT INTO document_meta
                   (id, doc_path, status, created_by, created_at, updated_at)
                   VALUES (?1, ?2, 'draft', ?3, ?4, ?4)
                   ON CONFLICT(doc_path) DO UPDATE SET
                       status = 'draft',
                       published_at = NULL,
                       updated_at = ?4"#,
                params![id, doc_path, user_id, now],
            )?;
            conn.query_row(
                r#"SELECT id, doc_path, status, published_at, created_by, template_id,
                          archived_at, deleted_at, created_at, updated_at
                   FROM document_meta WHERE doc_path = ?1"#,
                params![doc_path],
                row_to_meta,
            )
        })
        .await
    }

    /// List all draft documents. Admins see all; editors see their own.
    pub async fn list_drafts(
        &self,
        user_id: &str,
        is_admin: bool,
    ) -> Result<Vec<DocMeta>, AppError> {
        let db = self.db.clone();
        let user_id = user_id.to_string();
        with_conn(&db, move |conn| {
            let sql = if is_admin {
                r#"SELECT id, doc_path, status, published_at, created_by, template_id,
                          archived_at, deleted_at, created_at, updated_at
                   FROM document_meta
                   WHERE status = 'draft' AND deleted_at IS NULL AND archived_at IS NULL
                   ORDER BY updated_at DESC"#
                    .to_string()
            } else {
                r#"SELECT id, doc_path, status, published_at, created_by, template_id,
                          archived_at, deleted_at, created_at, updated_at
                   FROM document_meta
                   WHERE status = 'draft' AND deleted_at IS NULL AND archived_at IS NULL
                         AND created_by = ?1
                   ORDER BY updated_at DESC"#
                    .to_string()
            };
            let mut stmt = conn.prepare(&sql)?;
            if is_admin {
                let rows = stmt.query_map([], row_to_meta)?;
                rows.collect()
            } else {
                let rows = stmt.query_map(params![user_id], row_to_meta)?;
                rows.collect()
            }
        })
        .await
    }

    /// Archive a document (hides from main view but keeps in archive).
    pub async fn archive(&self, doc_path: &str, user_id: &str) -> Result<DocMeta, AppError> {
        let db = self.db.clone();
        let doc_path = doc_path.to_string();
        let user_id = user_id.to_string();
        with_conn(&db, move |conn| {
            let now = Utc::now().to_rfc3339();
            let id = Uuid::now_v7().to_string();
            conn.execute(
                r#"INSERT INTO document_meta
                   (id, doc_path, status, created_by, archived_at, created_at, updated_at)
                   VALUES (?1, ?2, 'draft', ?3, ?4, ?4, ?4)
                   ON CONFLICT(doc_path) DO UPDATE SET
                       archived_at = ?4,
                       deleted_at = NULL,
                       updated_at = ?4"#,
                params![id, doc_path, user_id, now],
            )?;
            conn.query_row(
                r#"SELECT id, doc_path, status, published_at, created_by, template_id,
                          archived_at, deleted_at, created_at, updated_at
                   FROM document_meta WHERE doc_path = ?1"#,
                params![doc_path],
                row_to_meta,
            )
        })
        .await
    }

    /// Unarchive a document.
    pub async fn unarchive(&self, doc_path: &str, user_id: &str) -> Result<DocMeta, AppError> {
        let db = self.db.clone();
        let doc_path = doc_path.to_string();
        let user_id = user_id.to_string();
        with_conn(&db, move |conn| {
            let now = Utc::now().to_rfc3339();
            let id = Uuid::now_v7().to_string();
            conn.execute(
                r#"INSERT INTO document_meta
                   (id, doc_path, status, created_by, created_at, updated_at)
                   VALUES (?1, ?2, 'draft', ?3, ?4, ?4)
                   ON CONFLICT(doc_path) DO UPDATE SET
                       archived_at = NULL,
                       updated_at = ?4"#,
                params![id, doc_path, user_id, now],
            )?;
            conn.query_row(
                r#"SELECT id, doc_path, status, published_at, created_by, template_id,
                          archived_at, deleted_at, created_at, updated_at
                   FROM document_meta WHERE doc_path = ?1"#,
                params![doc_path],
                row_to_meta,
            )
        })
        .await
    }

    /// Soft-delete (trash) a document.
    pub async fn trash(&self, doc_path: &str, user_id: &str) -> Result<DocMeta, AppError> {
        let db = self.db.clone();
        let doc_path = doc_path.to_string();
        let user_id = user_id.to_string();
        with_conn(&db, move |conn| {
            let now = Utc::now().to_rfc3339();
            let id = Uuid::now_v7().to_string();
            conn.execute(
                r#"INSERT INTO document_meta
                   (id, doc_path, status, created_by, deleted_at, created_at, updated_at)
                   VALUES (?1, ?2, 'draft', ?3, ?4, ?4, ?4)
                   ON CONFLICT(doc_path) DO UPDATE SET
                       deleted_at = ?4,
                       updated_at = ?4"#,
                params![id, doc_path, user_id, now],
            )?;
            conn.query_row(
                r#"SELECT id, doc_path, status, published_at, created_by, template_id,
                          archived_at, deleted_at, created_at, updated_at
                   FROM document_meta WHERE doc_path = ?1"#,
                params![doc_path],
                row_to_meta,
            )
        })
        .await
    }

    /// Restore a trashed document.
    pub async fn restore(&self, doc_path: &str, user_id: &str) -> Result<DocMeta, AppError> {
        let db = self.db.clone();
        let doc_path = doc_path.to_string();
        let user_id = user_id.to_string();
        with_conn(&db, move |conn| {
            let now = Utc::now().to_rfc3339();
            let id = Uuid::now_v7().to_string();
            conn.execute(
                r#"INSERT INTO document_meta
                   (id, doc_path, status, created_by, created_at, updated_at)
                   VALUES (?1, ?2, 'draft', ?3, ?4, ?4)
                   ON CONFLICT(doc_path) DO UPDATE SET
                       deleted_at = NULL,
                       updated_at = ?4"#,
                params![id, doc_path, user_id, now],
            )?;
            conn.query_row(
                r#"SELECT id, doc_path, status, published_at, created_by, template_id,
                          archived_at, deleted_at, created_at, updated_at
                   FROM document_meta WHERE doc_path = ?1"#,
                params![doc_path],
                row_to_meta,
            )
        })
        .await
    }

    /// List trashed documents (deleted_at IS NOT NULL).
    pub async fn list_trash(&self) -> Result<Vec<DocMeta>, AppError> {
        let db = self.db.clone();
        with_conn(&db, |conn| {
            let mut stmt = conn.prepare(
                r#"SELECT id, doc_path, status, published_at, created_by, template_id,
                          archived_at, deleted_at, created_at, updated_at
                   FROM document_meta
                   WHERE deleted_at IS NOT NULL
                   ORDER BY deleted_at DESC"#,
            )?;
            let rows = stmt.query_map([], row_to_meta)?;
            rows.collect()
        })
        .await
    }

    /// List archived documents (archived_at IS NOT NULL, deleted_at IS NULL).
    pub async fn list_archive(&self) -> Result<Vec<DocMeta>, AppError> {
        let db = self.db.clone();
        with_conn(&db, |conn| {
            let mut stmt = conn.prepare(
                r#"SELECT id, doc_path, status, published_at, created_by, template_id,
                          archived_at, deleted_at, created_at, updated_at
                   FROM document_meta
                   WHERE archived_at IS NOT NULL AND deleted_at IS NULL
                   ORDER BY archived_at DESC"#,
            )?;
            let rows = stmt.query_map([], row_to_meta)?;
            rows.collect()
        })
        .await
    }

    /// Hard-delete a document's metadata record (permanent removal).
    pub async fn permanent_delete(&self, doc_path: &str) -> Result<(), AppError> {
        let db = self.db.clone();
        let doc_path = doc_path.to_string();
        let doc_path2 = doc_path.clone();
        with_conn(&db, move |conn| {
            let rows =
                conn.execute("DELETE FROM document_meta WHERE doc_path = ?1", params![doc_path])?;
            if rows == 0 {
                return Err(rusqlite::Error::QueryReturnedNoRows);
            }
            Ok(())
        })
        .await
        .map_err(|e| match e {
            AppError::Db(rusqlite::Error::QueryReturnedNoRows) => {
                AppError::NotFound(format!("no metadata for {doc_path2}"))
            }
            other => other,
        })
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn row_to_meta(row: &rusqlite::Row) -> rusqlite::Result<DocMeta> {
    Ok(DocMeta {
        id: row.get(0)?,
        doc_path: row.get(1)?,
        status: row.get(2)?,
        published_at: row.get(3)?,
        created_by: row.get(4)?,
        template_id: row.get(5)?,
        archived_at: row.get(6)?,
        deleted_at: row.get(7)?,
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
    })
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    fn make_engine() -> DocMetaEngine {
        let db = db::open(":memory:").unwrap();
        // Insert a test user to satisfy FK constraints.
        {
            let conn = db.lock().unwrap();
            conn.execute_batch(
                "INSERT INTO users (id, email, name, password_hash, role, created_at, updated_at)
                 VALUES ('user-1', 'u@example.com', 'U', 'hash', 'editor',
                         '2024-01-01T00:00:00Z', '2024-01-01T00:00:00Z');",
            )
            .unwrap();
        }
        DocMetaEngine::new(db)
    }

    #[tokio::test]
    async fn test_get_or_create() {
        let engine = make_engine();
        let m = engine.get_or_create("docs/page.md", "user-1").await.unwrap();
        assert_eq!(m.status, "draft");
        assert_eq!(m.doc_path, "docs/page.md");
    }

    #[tokio::test]
    async fn test_get_or_create_idempotent() {
        let engine = make_engine();
        let m1 = engine.get_or_create("a.md", "user-1").await.unwrap();
        let m2 = engine.get_or_create("a.md", "user-1").await.unwrap();
        assert_eq!(m1.id, m2.id);
    }

    #[tokio::test]
    async fn test_publish() {
        let engine = make_engine();
        let m = engine.publish("docs/a.md", "user-1").await.unwrap();
        assert_eq!(m.status, "published");
        assert!(m.published_at.is_some());
    }

    #[tokio::test]
    async fn test_unpublish() {
        let engine = make_engine();
        engine.publish("docs/a.md", "user-1").await.unwrap();
        let m = engine.unpublish("docs/a.md", "user-1").await.unwrap();
        assert_eq!(m.status, "draft");
        assert!(m.published_at.is_none());
    }

    #[tokio::test]
    async fn test_list_drafts_by_user() {
        let engine = make_engine();
        engine.get_or_create("draft.md", "user-1").await.unwrap();
        engine.publish("published.md", "user-1").await.unwrap();

        let drafts = engine.list_drafts("user-1", false).await.unwrap();
        assert_eq!(drafts.len(), 1);
        assert_eq!(drafts[0].doc_path, "draft.md");
    }

    #[tokio::test]
    async fn test_archive_and_unarchive() {
        let engine = make_engine();
        let m = engine.archive("a.md", "user-1").await.unwrap();
        assert!(m.archived_at.is_some());

        let archived = engine.list_archive().await.unwrap();
        assert_eq!(archived.len(), 1);

        let m2 = engine.unarchive("a.md", "user-1").await.unwrap();
        assert!(m2.archived_at.is_none());
        assert_eq!(engine.list_archive().await.unwrap().len(), 0);
    }

    #[tokio::test]
    async fn test_trash_and_restore() {
        let engine = make_engine();
        let m = engine.trash("b.md", "user-1").await.unwrap();
        assert!(m.deleted_at.is_some());

        let trash = engine.list_trash().await.unwrap();
        assert_eq!(trash.len(), 1);

        let m2 = engine.restore("b.md", "user-1").await.unwrap();
        assert!(m2.deleted_at.is_none());
        assert_eq!(engine.list_trash().await.unwrap().len(), 0);
    }

    #[tokio::test]
    async fn test_permanent_delete() {
        let engine = make_engine();
        engine.trash("c.md", "user-1").await.unwrap();
        engine.permanent_delete("c.md").await.unwrap();
        let err = engine.get("c.md").await.unwrap_err();
        assert!(matches!(err, AppError::NotFound(_)));
    }
}
