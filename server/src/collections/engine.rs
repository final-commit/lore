use chrono::Utc;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::{with_conn, DbConn};
use crate::error::AppError;

// ── Data types ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Collection {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub sort_order: i64,
    pub parent_id: Option<String>,
    pub permission: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateCollection {
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub sort_order: Option<i64>,
    pub parent_id: Option<String>,
    pub permission: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateCollection {
    pub name: Option<String>,
    pub description: Option<String>,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub sort_order: Option<i64>,
}

// ── Engine ────────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct CollectionEngine {
    db: DbConn,
}

impl CollectionEngine {
    pub fn new(db: DbConn) -> Self {
        CollectionEngine { db }
    }

    /// List all collections ordered by sort_order then name.
    pub async fn list(&self) -> Result<Vec<Collection>, AppError> {
        let db = self.db.clone();
        with_conn(&db, |conn| {
            let mut stmt = conn.prepare(
                r#"SELECT id, name, slug, description, icon, color, sort_order,
                          parent_id, permission, created_at, updated_at
                   FROM collections ORDER BY sort_order ASC, name ASC"#,
            )?;
            let rows = stmt.query_map([], row_to_collection)?;
            rows.collect()
        })
        .await
    }

    /// Get a collection by ID.
    pub async fn get(&self, id: &str) -> Result<Collection, AppError> {
        let db = self.db.clone();
        let id = id.to_string();
        let id2 = id.clone();
        with_conn(&db, move |conn| {
            conn.query_row(
                r#"SELECT id, name, slug, description, icon, color, sort_order,
                          parent_id, permission, created_at, updated_at
                   FROM collections WHERE id = ?1"#,
                params![id],
                row_to_collection,
            )
        })
        .await
        .map_err(|e| match e {
            AppError::Db(rusqlite::Error::QueryReturnedNoRows) => {
                AppError::NotFound(format!("collection {id2} not found"))
            }
            other => other,
        })
    }

    /// Create a new collection. Returns `Conflict` if slug already exists.
    pub async fn create(&self, req: CreateCollection) -> Result<Collection, AppError> {
        let db = self.db.clone();
        with_conn(&db, move |conn| {
            let id = Uuid::now_v7().to_string();
            let now = Utc::now().to_rfc3339();
            let sort_order = req.sort_order.unwrap_or(0);
            let permission = req.permission.unwrap_or_else(|| "read".to_string());

            conn.execute(
                r#"INSERT INTO collections
                   (id, name, slug, description, icon, color, sort_order,
                    parent_id, permission, created_at, updated_at)
                   VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?10)"#,
                params![
                    id,
                    req.name,
                    req.slug,
                    req.description,
                    req.icon,
                    req.color,
                    sort_order,
                    req.parent_id,
                    permission,
                    now,
                ],
            )?;

            conn.query_row(
                r#"SELECT id, name, slug, description, icon, color, sort_order,
                          parent_id, permission, created_at, updated_at
                   FROM collections WHERE id = ?1"#,
                params![id],
                row_to_collection,
            )
        })
        .await
        .map_err(|e| match e {
            AppError::Db(rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error { code: rusqlite::ffi::ErrorCode::ConstraintViolation, .. },
                _,
            )) => AppError::Conflict("collection slug already exists".into()),
            other => other,
        })
    }

    /// Update a collection. Returns `NotFound` if ID doesn't exist.
    pub async fn update(&self, id: &str, req: UpdateCollection) -> Result<Collection, AppError> {
        let db = self.db.clone();
        let id = id.to_string();
        let id2 = id.clone();
        with_conn(&db, move |conn| {
            let now = Utc::now().to_rfc3339();
            // Build dynamic SET clause from provided fields.
            let mut sets = vec!["updated_at = ?1".to_string()];
            let mut idx = 2usize;
            if req.name.is_some() {
                sets.push(format!("name = ?{idx}"));
                idx += 1;
            }
            // Always include these fields so they can be set to NULL.
            sets.push(format!("description = ?{idx}"));
            idx += 1;
            sets.push(format!("icon = ?{idx}"));
            idx += 1;
            sets.push(format!("color = ?{idx}"));
            idx += 1;
            if req.sort_order.is_some() {
                sets.push(format!("sort_order = ?{idx}"));
                idx += 1;
            }
            let where_idx = idx;
            let sql = format!(
                "UPDATE collections SET {} WHERE id = ?{where_idx}",
                sets.join(", ")
            );

            // Bind values in the same order.
            let rows = {
                let mut stmt = conn.prepare(&sql)?;
                let mut bind_idx = 1usize;
                stmt.raw_bind_parameter(bind_idx, &now)?;
                bind_idx += 1;
                if let Some(ref v) = req.name {
                    stmt.raw_bind_parameter(bind_idx, v)?;
                    bind_idx += 1;
                }
                // description (nullable — always bound)
                match &req.description {
                    Some(v) => stmt.raw_bind_parameter(bind_idx, v)?,
                    None => stmt.raw_bind_parameter(bind_idx, rusqlite::types::Null)?,
                }
                bind_idx += 1;
                // icon (nullable)
                match &req.icon {
                    Some(v) => stmt.raw_bind_parameter(bind_idx, v)?,
                    None => stmt.raw_bind_parameter(bind_idx, rusqlite::types::Null)?,
                }
                bind_idx += 1;
                // color (nullable)
                match &req.color {
                    Some(v) => stmt.raw_bind_parameter(bind_idx, v)?,
                    None => stmt.raw_bind_parameter(bind_idx, rusqlite::types::Null)?,
                }
                bind_idx += 1;
                if let Some(v) = req.sort_order {
                    stmt.raw_bind_parameter(bind_idx, v)?;
                    bind_idx += 1;
                }
                stmt.raw_bind_parameter(bind_idx, &id)?;
                stmt.raw_execute()?
            };

            if rows == 0 {
                return Err(rusqlite::Error::QueryReturnedNoRows);
            }

            conn.query_row(
                r#"SELECT id, name, slug, description, icon, color, sort_order,
                          parent_id, permission, created_at, updated_at
                   FROM collections WHERE id = ?1"#,
                params![id2],
                row_to_collection,
            )
        })
        .await
        .map_err(|e| match e {
            AppError::Db(rusqlite::Error::QueryReturnedNoRows) => {
                AppError::NotFound("collection not found".into())
            }
            other => other,
        })
    }

    /// Delete a collection by ID.
    pub async fn delete(&self, id: &str) -> Result<(), AppError> {
        let db = self.db.clone();
        let id = id.to_string();
        let id2 = id.clone();
        with_conn(&db, move |conn| {
            let rows = conn.execute("DELETE FROM collections WHERE id = ?1", params![id])?;
            if rows == 0 {
                return Err(rusqlite::Error::QueryReturnedNoRows);
            }
            Ok(())
        })
        .await
        .map_err(|e| match e {
            AppError::Db(rusqlite::Error::QueryReturnedNoRows) => {
                AppError::NotFound(format!("collection {id2} not found"))
            }
            other => other,
        })
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn row_to_collection(row: &rusqlite::Row) -> rusqlite::Result<Collection> {
    Ok(Collection {
        id: row.get(0)?,
        name: row.get(1)?,
        slug: row.get(2)?,
        description: row.get(3)?,
        icon: row.get(4)?,
        color: row.get(5)?,
        sort_order: row.get(6)?,
        parent_id: row.get(7)?,
        permission: row.get(8)?,
        created_at: row.get(9)?,
        updated_at: row.get(10)?,
    })
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    fn make_engine() -> CollectionEngine {
        let db = db::open(":memory:").unwrap();
        CollectionEngine::new(db)
    }

    fn make_req(name: &str, slug: &str) -> CreateCollection {
        CreateCollection {
            name: name.to_string(),
            slug: slug.to_string(),
            description: None,
            icon: None,
            color: None,
            sort_order: None,
            parent_id: None,
            permission: None,
        }
    }

    #[tokio::test]
    async fn test_create_and_list() {
        let engine = make_engine();
        let c = engine.create(make_req("Engineering", "engineering")).await.unwrap();
        assert_eq!(c.name, "Engineering");
        assert_eq!(c.slug, "engineering");
        assert_eq!(c.permission, "read");

        let list = engine.list().await.unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, c.id);
    }

    #[tokio::test]
    async fn test_get_collection() {
        let engine = make_engine();
        let c = engine.create(make_req("Docs", "docs")).await.unwrap();
        let fetched = engine.get(&c.id).await.unwrap();
        assert_eq!(fetched.name, "Docs");
    }

    #[tokio::test]
    async fn test_get_missing() {
        let engine = make_engine();
        let err = engine.get("no-such-id").await.unwrap_err();
        assert!(matches!(err, AppError::NotFound(_)));
    }

    #[tokio::test]
    async fn test_duplicate_slug_rejected() {
        let engine = make_engine();
        engine.create(make_req("A", "same-slug")).await.unwrap();
        let err = engine.create(make_req("B", "same-slug")).await.unwrap_err();
        assert!(matches!(err, AppError::Conflict(_)));
    }

    #[tokio::test]
    async fn test_update_collection() {
        let engine = make_engine();
        let c = engine.create(make_req("Old Name", "old")).await.unwrap();
        let updated = engine
            .update(
                &c.id,
                UpdateCollection {
                    name: Some("New Name".to_string()),
                    description: Some("desc".to_string()),
                    icon: None,
                    color: None,
                    sort_order: Some(5),
                },
            )
            .await
            .unwrap();
        assert_eq!(updated.name, "New Name");
        assert_eq!(updated.sort_order, 5);
    }

    #[tokio::test]
    async fn test_delete_collection() {
        let engine = make_engine();
        let c = engine.create(make_req("Temp", "temp")).await.unwrap();
        engine.delete(&c.id).await.unwrap();
        let err = engine.get(&c.id).await.unwrap_err();
        assert!(matches!(err, AppError::NotFound(_)));
    }

    #[tokio::test]
    async fn test_nested_collections() {
        let engine = make_engine();
        let parent = engine.create(make_req("Parent", "parent")).await.unwrap();
        let child = engine
            .create(CreateCollection {
                parent_id: Some(parent.id.clone()),
                ..make_req("Child", "child")
            })
            .await
            .unwrap();
        assert_eq!(child.parent_id, Some(parent.id));
    }
}
