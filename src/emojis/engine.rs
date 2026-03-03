use chrono::Utc;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;
use crate::db::{with_conn, DbConn};
use crate::error::AppError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomEmoji {
    pub id: String,
    pub shortcode: String,
    pub image_url: String,
    pub creator_id: String,
    pub created_at: String,
}

#[derive(Clone)]
pub struct EmojiEngine {
    db: DbConn,
    storage_path: PathBuf,
}

impl EmojiEngine {
    pub fn new(db: DbConn, storage_path: PathBuf) -> Self {
        EmojiEngine { db, storage_path }
    }

    pub async fn list(&self) -> Result<Vec<CustomEmoji>, AppError> {
        let db = self.db.clone();
        with_conn(&db, |conn| {
            let mut stmt = conn.prepare(
                "SELECT id, shortcode, image_path, creator_id, created_at FROM custom_emojis ORDER BY shortcode"
            )?;
            stmt.query_map([], |r| Ok(CustomEmoji {
                id: r.get(0)?,
                shortcode: r.get(1)?,
                image_url: format!("/api/emojis/{}/image", r.get::<_,String>(0)?),
                creator_id: r.get(3)?,
                created_at: r.get(4)?,
            }))?.collect()
        }).await
    }

    pub async fn create(&self, shortcode: &str, creator_id: &str, data: Vec<u8>, ext: &str) -> Result<CustomEmoji, AppError> {
        // Validate shortcode
        if shortcode.is_empty() || shortcode.len() > 50 || !shortcode.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
            return Err(AppError::BadRequest("invalid shortcode: alphanumeric, _ and - only, max 50 chars".into()));
        }
        let id = Uuid::now_v7().to_string();
        let filename = format!("{id}.{ext}");
        let path = self.storage_path.join(&filename);
        std::fs::create_dir_all(&self.storage_path).map_err(|e| AppError::Internal(e.to_string()))?;
        std::fs::write(&path, &data).map_err(|e| AppError::Internal(e.to_string()))?;

        let db = self.db.clone();
        let sc = shortcode.to_string();
        let cid = creator_id.to_string();
        let fname = filename.clone();
        let now = Utc::now().to_rfc3339();
        let eid = id.clone();
        with_conn(&db, move |conn| {
            conn.execute(
                "INSERT INTO custom_emojis (id, shortcode, image_path, creator_id, created_at) VALUES (?1,?2,?3,?4,?5)",
                params![eid, sc, fname, cid, now],
            )?;
            conn.query_row(
                "SELECT id, shortcode, image_path, creator_id, created_at FROM custom_emojis WHERE id=?1",
                params![eid],
                |r| Ok(CustomEmoji {
                    id: r.get(0)?,
                    shortcode: r.get(1)?,
                    image_url: format!("/api/emojis/{}/image", r.get::<_,String>(0)?),
                    creator_id: r.get(3)?,
                    created_at: r.get(4)?,
                }),
            )
        }).await
    }

    pub async fn delete(&self, id: &str) -> Result<(), AppError> {
        let db = self.db.clone();
        let eid = id.to_string();
        let path = with_conn(&db, move |conn| {
            let path: Option<String> = conn.query_row(
                "SELECT image_path FROM custom_emojis WHERE id=?1",
                params![eid],
                |r| r.get(0),
            ).ok();
            conn.execute("DELETE FROM custom_emojis WHERE id=?1", params![eid])?;
            Ok(path)
        }).await?;
        if let Some(fname) = path {
            let _ = std::fs::remove_file(self.storage_path.join(fname));
        }
        Ok(())
    }

    pub async fn get_image_path(&self, id: &str) -> Result<PathBuf, AppError> {
        let db = self.db.clone();
        let eid = id.to_string();
        let fname: String = with_conn(&db, move |conn| {
            conn.query_row("SELECT image_path FROM custom_emojis WHERE id=?1", params![eid], |r| r.get(0))
        }).await.map_err(|_| AppError::NotFound("emoji not found".into()))?;
        Ok(self.storage_path.join(fname))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use tempfile::tempdir;

    fn engine() -> (EmojiEngine, tempfile::TempDir) {
        let dir = tempdir().unwrap();
        let db = db::open(":memory:").unwrap();
        { let conn = db.lock().unwrap();
          conn.execute("INSERT INTO users (id,email,name,password_hash,role,created_at,updated_at) VALUES ('u1','a@b.com','A','h','admin','2024-01-01','2024-01-01')", []).unwrap(); }
        (EmojiEngine::new(db, dir.path().to_path_buf()), dir)
    }

    #[tokio::test]
    async fn test_list_empty() {
        let (e, _dir) = engine();
        let emojis = e.list().await.unwrap();
        assert!(emojis.is_empty());
    }

    #[tokio::test]
    async fn test_create_and_list() {
        let (e, _dir) = engine();
        e.create("thumbs-up", "u1", vec![1,2,3], "png").await.unwrap();
        let emojis = e.list().await.unwrap();
        assert_eq!(emojis.len(), 1);
        assert_eq!(emojis[0].shortcode, "thumbs-up");
    }

    #[tokio::test]
    async fn test_invalid_shortcode() {
        let (e, _dir) = engine();
        let err = e.create("bad emoji!", "u1", vec![], "png").await.unwrap_err();
        assert!(matches!(err, AppError::BadRequest(_)));
    }

    #[tokio::test]
    async fn test_delete() {
        let (e, _dir) = engine();
        e.create("wave", "u1", vec![1,2,3], "png").await.unwrap();
        let emojis = e.list().await.unwrap();
        e.delete(&emojis[0].id).await.unwrap();
        assert!(e.list().await.unwrap().is_empty());
    }
}
