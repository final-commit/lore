use std::path::PathBuf;

use chrono::Utc;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::{with_conn, DbConn};
use crate::error::AppError;

// ── Data types ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    pub id: String,
    pub doc_path: String,
    pub filename: String,
    pub content_type: String,
    pub size_bytes: i64,
    /// Relative path within the repo dir (e.g. `_attachments/abc123`)
    pub git_path: String,
    pub created_by: String,
    pub created_at: String,
}

// ── Engine ────────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct AttachmentEngine {
    db: DbConn,
    repo_path: PathBuf,
    max_upload_bytes: usize,
}

impl AttachmentEngine {
    pub fn new(db: DbConn, repo_path: PathBuf, max_upload_bytes: usize) -> Self {
        AttachmentEngine { db, repo_path, max_upload_bytes }
    }

    /// Store a file on disk (in `_attachments/`) and record metadata in SQLite.
    pub async fn upload(
        &self,
        doc_path: &str,
        filename: &str,
        content_type: &str,
        data: Vec<u8>,
        created_by: &str,
    ) -> Result<Attachment, AppError> {
        if data.len() > self.max_upload_bytes {
            return Err(AppError::BadRequest(format!(
                "file too large: {} bytes (max {})",
                data.len(),
                self.max_upload_bytes
            )));
        }

        let id = Uuid::now_v7().to_string();
        let git_path = format!("_attachments/{id}");
        let full_path = self.repo_path.join(&git_path);

        // Ensure parent directory exists.
        if let Some(parent) = full_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| AppError::Internal(format!("cannot create attachments dir: {e}")))?;
        }

        std::fs::write(&full_path, &data)
            .map_err(|e| AppError::Internal(format!("failed to write attachment: {e}")))?;

        let db = self.db.clone();
        let doc_path = doc_path.to_string();
        let filename = filename.to_string();
        let content_type = content_type.to_string();
        let size_bytes = data.len() as i64;
        let created_by = created_by.to_string();
        let git_path2 = git_path.clone();
        let id2 = id.clone();

        with_conn(&db, move |conn| {
            let now = Utc::now().to_rfc3339();
            conn.execute(
                r#"INSERT INTO attachments
                   (id, doc_path, filename, content_type, size_bytes, git_path, created_by, created_at)
                   VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)"#,
                params![
                    id2,
                    doc_path,
                    filename,
                    content_type,
                    size_bytes,
                    git_path2,
                    created_by,
                    now,
                ],
            )?;
            conn.query_row(
                r#"SELECT id, doc_path, filename, content_type, size_bytes,
                          git_path, created_by, created_at
                   FROM attachments WHERE id = ?1"#,
                params![id2],
                row_to_attachment,
            )
        })
        .await
    }

    /// Get metadata for an attachment by ID.
    pub async fn get_meta(&self, id: &str) -> Result<Attachment, AppError> {
        let db = self.db.clone();
        let id = id.to_string();
        let id2 = id.clone();
        with_conn(&db, move |conn| {
            conn.query_row(
                r#"SELECT id, doc_path, filename, content_type, size_bytes,
                          git_path, created_by, created_at
                   FROM attachments WHERE id = ?1"#,
                params![id],
                row_to_attachment,
            )
        })
        .await
        .map_err(|e| match e {
            AppError::Db(rusqlite::Error::QueryReturnedNoRows) => {
                AppError::NotFound(format!("attachment {id2} not found"))
            }
            other => other,
        })
    }

    /// Read the file bytes for an attachment.
    pub async fn read_bytes(&self, id: &str) -> Result<(Attachment, Vec<u8>), AppError> {
        let meta = self.get_meta(id).await?;
        let full_path = self.repo_path.join(&meta.git_path);
        let bytes = std::fs::read(&full_path)
            .map_err(|e| AppError::Internal(format!("failed to read attachment file: {e}")))?;
        Ok((meta, bytes))
    }

    /// List all attachments for a given document path.
    pub async fn list_for_doc(&self, doc_path: &str) -> Result<Vec<Attachment>, AppError> {
        let db = self.db.clone();
        let doc_path = doc_path.to_string();
        with_conn(&db, move |conn| {
            let mut stmt = conn.prepare(
                r#"SELECT id, doc_path, filename, content_type, size_bytes,
                          git_path, created_by, created_at
                   FROM attachments WHERE doc_path = ?1 ORDER BY created_at ASC"#,
            )?;
            let rows = stmt.query_map(params![doc_path], row_to_attachment)?;
            rows.collect()
        })
        .await
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn row_to_attachment(row: &rusqlite::Row) -> rusqlite::Result<Attachment> {
    Ok(Attachment {
        id: row.get(0)?,
        doc_path: row.get(1)?,
        filename: row.get(2)?,
        content_type: row.get(3)?,
        size_bytes: row.get(4)?,
        git_path: row.get(5)?,
        created_by: row.get(6)?,
        created_at: row.get(7)?,
    })
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use tempfile::TempDir;

    fn make_engine() -> (TempDir, AttachmentEngine) {
        let dir = TempDir::new().unwrap();
        let db = db::open(":memory:").unwrap();
        {
            let conn = db.lock().unwrap();
            conn.execute_batch(
                "INSERT INTO users (id, email, name, password_hash, role, created_at, updated_at)
                 VALUES ('user-1', 'u@example.com', 'U', 'hash', 'editor',
                         '2024-01-01T00:00:00Z', '2024-01-01T00:00:00Z');",
            )
            .unwrap();
        }
        let engine = AttachmentEngine::new(db, dir.path().to_path_buf(), 10 * 1024 * 1024);
        (dir, engine)
    }

    #[tokio::test]
    async fn test_upload_and_read() {
        let (_dir, engine) = make_engine();
        let data = b"hello attachment".to_vec();
        let att = engine
            .upload("docs/a.md", "hello.txt", "text/plain", data.clone(), "user-1")
            .await
            .unwrap();

        assert_eq!(att.filename, "hello.txt");
        assert_eq!(att.size_bytes, data.len() as i64);

        let (meta, bytes) = engine.read_bytes(&att.id).await.unwrap();
        assert_eq!(meta.id, att.id);
        assert_eq!(bytes, data);
    }

    #[tokio::test]
    async fn test_get_meta() {
        let (_dir, engine) = make_engine();
        let att = engine
            .upload("docs/b.md", "img.png", "image/png", vec![1, 2, 3], "user-1")
            .await
            .unwrap();
        let meta = engine.get_meta(&att.id).await.unwrap();
        assert_eq!(meta.content_type, "image/png");
    }

    #[tokio::test]
    async fn test_get_missing_meta() {
        let (_dir, engine) = make_engine();
        let err = engine.get_meta("no-such-id").await.unwrap_err();
        assert!(matches!(err, AppError::NotFound(_)));
    }

    #[tokio::test]
    async fn test_size_limit() {
        let (_dir, engine) = make_engine();
        // Limit is 10 MB; send 11 MB.
        let big = vec![0u8; 11 * 1024 * 1024];
        let err = engine
            .upload("docs/c.md", "big.bin", "application/octet-stream", big, "user-1")
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::BadRequest(_)));
    }

    #[tokio::test]
    async fn test_list_for_doc() {
        let (_dir, engine) = make_engine();
        engine.upload("docs/x.md", "a.txt", "text/plain", b"a".to_vec(), "user-1").await.unwrap();
        engine.upload("docs/x.md", "b.txt", "text/plain", b"b".to_vec(), "user-1").await.unwrap();
        engine
            .upload("docs/other.md", "c.txt", "text/plain", b"c".to_vec(), "user-1")
            .await
            .unwrap();

        let list = engine.list_for_doc("docs/x.md").await.unwrap();
        assert_eq!(list.len(), 2);
    }
}
