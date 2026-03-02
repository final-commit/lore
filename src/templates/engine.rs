use chrono::Utc;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::{with_conn, DbConn};
use crate::error::AppError;

// ── Data types ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Template {
    pub id: String,
    pub title: String,
    pub content: String,
    pub collection_id: Option<String>,
    pub created_by: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateTemplate {
    pub title: String,
    pub content: String,
    pub collection_id: Option<String>,
    pub created_by: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateTemplate {
    pub title: Option<String>,
    pub content: Option<String>,
}

// ── Engine ────────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct TemplateEngine {
    db: DbConn,
}

impl TemplateEngine {
    pub fn new(db: DbConn) -> Self {
        TemplateEngine { db }
    }

    /// List all templates ordered by title.
    pub async fn list(&self) -> Result<Vec<Template>, AppError> {
        let db = self.db.clone();
        with_conn(&db, |conn| {
            let mut stmt = conn.prepare(
                r#"SELECT id, title, content, collection_id, created_by, created_at, updated_at
                   FROM templates ORDER BY title ASC"#,
            )?;
            let rows = stmt.query_map([], row_to_template)?;
            rows.collect()
        })
        .await
    }

    /// Get a template by ID.
    pub async fn get(&self, id: &str) -> Result<Template, AppError> {
        let db = self.db.clone();
        let id = id.to_string();
        let id2 = id.clone();
        with_conn(&db, move |conn| {
            conn.query_row(
                r#"SELECT id, title, content, collection_id, created_by, created_at, updated_at
                   FROM templates WHERE id = ?1"#,
                params![id],
                row_to_template,
            )
        })
        .await
        .map_err(|e| match e {
            AppError::Db(rusqlite::Error::QueryReturnedNoRows) => {
                AppError::NotFound(format!("template {id2} not found"))
            }
            other => other,
        })
    }

    /// Create a new template.
    pub async fn create(&self, req: CreateTemplate) -> Result<Template, AppError> {
        let db = self.db.clone();
        with_conn(&db, move |conn| {
            let id = Uuid::now_v7().to_string();
            let now = Utc::now().to_rfc3339();

            conn.execute(
                r#"INSERT INTO templates (id, title, content, collection_id, created_by, created_at, updated_at)
                   VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)"#,
                params![
                    id,
                    req.title,
                    req.content,
                    req.collection_id,
                    req.created_by,
                    now,
                ],
            )?;

            conn.query_row(
                r#"SELECT id, title, content, collection_id, created_by, created_at, updated_at
                   FROM templates WHERE id = ?1"#,
                params![id],
                row_to_template,
            )
        })
        .await
    }

    /// Update a template's title and/or content.
    pub async fn update(&self, id: &str, req: UpdateTemplate) -> Result<Template, AppError> {
        let db = self.db.clone();
        let id = id.to_string();
        let id2 = id.clone();
        with_conn(&db, move |conn| {
            let now = Utc::now().to_rfc3339();
            // Only update provided fields.
            if let Some(ref title) = req.title {
                conn.execute(
                    "UPDATE templates SET title = ?1, updated_at = ?2 WHERE id = ?3",
                    params![title, now, id],
                )?;
            }
            if let Some(ref content) = req.content {
                conn.execute(
                    "UPDATE templates SET content = ?1, updated_at = ?2 WHERE id = ?3",
                    params![content, now, id],
                )?;
            }
            if req.title.is_none() && req.content.is_none() {
                // Touch updated_at even with no-op update.
                let rows = conn.execute(
                    "UPDATE templates SET updated_at = ?1 WHERE id = ?2",
                    params![now, id],
                )?;
                if rows == 0 {
                    return Err(rusqlite::Error::QueryReturnedNoRows);
                }
            }

            conn.query_row(
                r#"SELECT id, title, content, collection_id, created_by, created_at, updated_at
                   FROM templates WHERE id = ?1"#,
                params![id2],
                row_to_template,
            )
        })
        .await
        .map_err(|e| match e {
            AppError::Db(rusqlite::Error::QueryReturnedNoRows) => {
                AppError::NotFound("template not found".into())
            }
            other => other,
        })
    }

    /// Delete a template by ID.
    pub async fn delete(&self, id: &str) -> Result<(), AppError> {
        let db = self.db.clone();
        let id = id.to_string();
        let id2 = id.clone();
        with_conn(&db, move |conn| {
            let rows = conn.execute("DELETE FROM templates WHERE id = ?1", params![id])?;
            if rows == 0 {
                return Err(rusqlite::Error::QueryReturnedNoRows);
            }
            Ok(())
        })
        .await
        .map_err(|e| match e {
            AppError::Db(rusqlite::Error::QueryReturnedNoRows) => {
                AppError::NotFound(format!("template {id2} not found"))
            }
            other => other,
        })
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn row_to_template(row: &rusqlite::Row) -> rusqlite::Result<Template> {
    Ok(Template {
        id: row.get(0)?,
        title: row.get(1)?,
        content: row.get(2)?,
        collection_id: row.get(3)?,
        created_by: row.get(4)?,
        created_at: row.get(5)?,
        updated_at: row.get(6)?,
    })
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    fn make_engine() -> TemplateEngine {
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
        TemplateEngine::new(db)
    }

    fn make_req(title: &str) -> CreateTemplate {
        CreateTemplate {
            title: title.to_string(),
            content: "# {{title}}\n\nWrite here.".to_string(),
            collection_id: None,
            created_by: "user-1".to_string(),
        }
    }

    #[tokio::test]
    async fn test_create_and_list() {
        let engine = make_engine();
        let t = engine.create(make_req("Meeting Notes")).await.unwrap();
        assert_eq!(t.title, "Meeting Notes");

        let list = engine.list().await.unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, t.id);
    }

    #[tokio::test]
    async fn test_get_template() {
        let engine = make_engine();
        let t = engine.create(make_req("Sprint Retro")).await.unwrap();
        let fetched = engine.get(&t.id).await.unwrap();
        assert_eq!(fetched.title, "Sprint Retro");
    }

    #[tokio::test]
    async fn test_get_missing() {
        let engine = make_engine();
        let err = engine.get("nope").await.unwrap_err();
        assert!(matches!(err, AppError::NotFound(_)));
    }

    #[tokio::test]
    async fn test_update_template() {
        let engine = make_engine();
        let t = engine.create(make_req("Old")).await.unwrap();
        let updated = engine
            .update(&t.id, UpdateTemplate { title: Some("New".into()), content: None })
            .await
            .unwrap();
        assert_eq!(updated.title, "New");
        assert!(updated.content.contains("Write here"));
    }

    #[tokio::test]
    async fn test_delete_template() {
        let engine = make_engine();
        let t = engine.create(make_req("Bye")).await.unwrap();
        engine.delete(&t.id).await.unwrap();
        assert!(matches!(engine.get(&t.id).await.unwrap_err(), AppError::NotFound(_)));
    }
}
