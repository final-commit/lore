use chrono::Utc;
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::{with_conn, DbConn};
use crate::error::AppError;

// ── Data types ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Group {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub created_by: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupMember {
    pub id: String,
    pub group_id: String,
    pub user_id: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateGroup {
    pub name: String,
    pub description: Option<String>,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateGroup {
    pub name: Option<String>,
    pub description: Option<String>,
}

// ── Engine ────────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct GroupEngine {
    db: DbConn,
}

impl GroupEngine {
    pub fn new(db: DbConn) -> Self {
        GroupEngine { db }
    }

    pub async fn create(&self, req: CreateGroup) -> Result<Group, AppError> {
        let db = self.db.clone();
        with_conn(&db, move |conn| {
            let id = Uuid::now_v7().to_string();
            let now = Utc::now().to_rfc3339();
            conn.execute(
                "INSERT INTO groups (id, name, description, created_by, created_at, updated_at)
                 VALUES (?1,?2,?3,?4,?5,?5)",
                params![id, req.name, req.description, req.created_by, now],
            )?;
            conn.query_row(
                "SELECT id, name, description, created_by, created_at, updated_at
                 FROM groups WHERE id = ?1",
                params![id],
                row_to_group,
            )
        })
        .await
    }

    pub async fn list(&self) -> Result<Vec<Group>, AppError> {
        let db = self.db.clone();
        with_conn(&db, move |conn| {
            let mut stmt = conn.prepare(
                "SELECT id, name, description, created_by, created_at, updated_at
                 FROM groups ORDER BY name ASC",
            )?;
            let rows = stmt.query_map([], row_to_group)?;
            rows.collect()
        })
        .await
    }

    pub async fn get(&self, id: &str) -> Result<Group, AppError> {
        let db = self.db.clone();
        let id = id.to_string();
        let id2 = id.clone();
        with_conn(&db, move |conn| {
            conn.query_row(
                "SELECT id, name, description, created_by, created_at, updated_at
                 FROM groups WHERE id = ?1",
                params![id],
                row_to_group,
            )
        })
        .await
        .map_err(|e| match e {
            AppError::Db(rusqlite::Error::QueryReturnedNoRows) => {
                AppError::NotFound(format!("group {id2} not found"))
            }
            other => other,
        })
    }

    pub async fn update(&self, id: &str, req: UpdateGroup) -> Result<Group, AppError> {
        let db = self.db.clone();
        let id = id.to_string();
        let id2 = id.clone();
        with_conn(&db, move |conn| {
            let now = Utc::now().to_rfc3339();
            let mut sets = vec!["updated_at = ?1".to_string()];
            let mut idx = 2i32;

            if req.name.is_some() {
                sets.push(format!("name = ?{idx}"));
                idx += 1;
            }
            if req.description.is_some() {
                sets.push(format!("description = ?{idx}"));
                idx += 1;
            }
            let id_idx = idx;
            let sql = format!("UPDATE groups SET {} WHERE id = ?{id_idx}", sets.join(", "));

            let mut param_values: Vec<rusqlite::types::Value> = vec![
                rusqlite::types::Value::Text(now),
            ];
            if let Some(n) = req.name {
                param_values.push(rusqlite::types::Value::Text(n));
            }
            if let Some(d) = req.description {
                param_values.push(rusqlite::types::Value::Text(d));
            }
            param_values.push(rusqlite::types::Value::Text(id.clone()));

            let rows = conn.execute(&sql, rusqlite::params_from_iter(param_values.iter()))?;
            if rows == 0 {
                return Err(rusqlite::Error::QueryReturnedNoRows);
            }
            conn.query_row(
                "SELECT id, name, description, created_by, created_at, updated_at
                 FROM groups WHERE id = ?1",
                params![id],
                row_to_group,
            )
        })
        .await
        .map_err(|e| match e {
            AppError::Db(rusqlite::Error::QueryReturnedNoRows) => {
                AppError::NotFound(format!("group {id2} not found"))
            }
            other => other,
        })
    }

    pub async fn delete(&self, id: &str) -> Result<(), AppError> {
        let db = self.db.clone();
        let id = id.to_string();
        let id2 = id.clone();
        with_conn(&db, move |conn| {
            let rows = conn.execute("DELETE FROM groups WHERE id = ?1", params![id])?;
            if rows == 0 {
                return Err(rusqlite::Error::QueryReturnedNoRows);
            }
            Ok(())
        })
        .await
        .map_err(|e| match e {
            AppError::Db(rusqlite::Error::QueryReturnedNoRows) => {
                AppError::NotFound(format!("group {id2} not found"))
            }
            other => other,
        })
    }

    pub async fn list_members(&self, group_id: &str) -> Result<Vec<GroupMember>, AppError> {
        let db = self.db.clone();
        let group_id = group_id.to_string();
        with_conn(&db, move |conn| {
            let mut stmt = conn.prepare(
                "SELECT id, group_id, user_id, created_at FROM group_users
                 WHERE group_id = ?1 ORDER BY created_at ASC",
            )?;
            let rows = stmt.query_map(params![group_id], row_to_member)?;
            rows.collect()
        })
        .await
    }

    pub async fn add_member(&self, group_id: &str, user_id: &str) -> Result<GroupMember, AppError> {
        let db = self.db.clone();
        let group_id = group_id.to_string();
        let user_id = user_id.to_string();
        with_conn(&db, move |conn| {
            // Check group exists.
            let exists: Option<String> = conn
                .query_row("SELECT id FROM groups WHERE id = ?1", params![group_id], |r| r.get(0))
                .optional()?;
            if exists.is_none() {
                return Err(rusqlite::Error::QueryReturnedNoRows);
            }

            let id = Uuid::now_v7().to_string();
            let now = Utc::now().to_rfc3339();
            conn.execute(
                "INSERT OR IGNORE INTO group_users (id, group_id, user_id, created_at)
                 VALUES (?1,?2,?3,?4)",
                params![id, group_id, user_id, now],
            )?;
            // Fetch the actual row (may have been pre-existing).
            conn.query_row(
                "SELECT id, group_id, user_id, created_at FROM group_users
                 WHERE group_id = ?1 AND user_id = ?2",
                params![group_id, user_id],
                row_to_member,
            )
        })
        .await
        .map_err(|e| match e {
            AppError::Db(rusqlite::Error::QueryReturnedNoRows) => {
                AppError::NotFound("group not found".into())
            }
            other => other,
        })
    }

    pub async fn remove_member(&self, group_id: &str, user_id: &str) -> Result<(), AppError> {
        let db = self.db.clone();
        let group_id = group_id.to_string();
        let user_id = user_id.to_string();
        with_conn(&db, move |conn| {
            let rows = conn.execute(
                "DELETE FROM group_users WHERE group_id = ?1 AND user_id = ?2",
                params![group_id, user_id],
            )?;
            if rows == 0 {
                return Err(rusqlite::Error::QueryReturnedNoRows);
            }
            Ok(())
        })
        .await
        .map_err(|e| match e {
            AppError::Db(rusqlite::Error::QueryReturnedNoRows) => {
                AppError::NotFound("membership not found".into())
            }
            other => other,
        })
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn row_to_group(row: &rusqlite::Row) -> rusqlite::Result<Group> {
    Ok(Group {
        id: row.get(0)?,
        name: row.get(1)?,
        description: row.get(2)?,
        created_by: row.get(3)?,
        created_at: row.get(4)?,
        updated_at: row.get(5)?,
    })
}

fn row_to_member(row: &rusqlite::Row) -> rusqlite::Result<GroupMember> {
    Ok(GroupMember {
        id: row.get(0)?,
        group_id: row.get(1)?,
        user_id: row.get(2)?,
        created_at: row.get(3)?,
    })
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    fn make_engine() -> GroupEngine {
        let db = db::open(":memory:").unwrap();
        {
            let conn = db.lock().unwrap();
            conn.execute_batch(
                "INSERT INTO users (id,email,name,password_hash,role,created_at,updated_at)
                 VALUES ('u1','a@b.com','Alice','hash','admin','2024-01-01T00:00:00Z','2024-01-01T00:00:00Z');
                 INSERT INTO users (id,email,name,password_hash,role,created_at,updated_at)
                 VALUES ('u2','b@c.com','Bob','hash','editor','2024-01-01T00:00:00Z','2024-01-01T00:00:00Z');",
            ).unwrap();
        }
        GroupEngine::new(db)
    }

    fn req(name: &str) -> CreateGroup {
        CreateGroup {
            name: name.to_string(),
            description: None,
            created_by: Some("u1".to_string()),
        }
    }

    #[tokio::test]
    async fn test_create_and_list() {
        let e = make_engine();
        let g = e.create(req("Engineering")).await.unwrap();
        assert_eq!(g.name, "Engineering");
        let list = e.list().await.unwrap();
        assert_eq!(list.len(), 1);
    }

    #[tokio::test]
    async fn test_get_group() {
        let e = make_engine();
        let g = e.create(req("Eng")).await.unwrap();
        let fetched = e.get(&g.id).await.unwrap();
        assert_eq!(fetched.id, g.id);
    }

    #[tokio::test]
    async fn test_get_not_found() {
        let e = make_engine();
        let err = e.get("no-such").await.unwrap_err();
        assert!(matches!(err, AppError::NotFound(_)));
    }

    #[tokio::test]
    async fn test_update_group() {
        let e = make_engine();
        let g = e.create(req("Old Name")).await.unwrap();
        let updated = e.update(&g.id, UpdateGroup {
            name: Some("New Name".into()),
            description: Some("Desc".into()),
        }).await.unwrap();
        assert_eq!(updated.name, "New Name");
        assert_eq!(updated.description, Some("Desc".into()));
    }

    #[tokio::test]
    async fn test_delete_group() {
        let e = make_engine();
        let g = e.create(req("To Delete")).await.unwrap();
        e.delete(&g.id).await.unwrap();
        let err = e.get(&g.id).await.unwrap_err();
        assert!(matches!(err, AppError::NotFound(_)));
    }

    #[tokio::test]
    async fn test_add_and_list_members() {
        let e = make_engine();
        let g = e.create(req("Team")).await.unwrap();
        e.add_member(&g.id, "u1").await.unwrap();
        e.add_member(&g.id, "u2").await.unwrap();
        let members = e.list_members(&g.id).await.unwrap();
        assert_eq!(members.len(), 2);
    }

    #[tokio::test]
    async fn test_add_member_idempotent() {
        let e = make_engine();
        let g = e.create(req("Team")).await.unwrap();
        e.add_member(&g.id, "u1").await.unwrap();
        e.add_member(&g.id, "u1").await.unwrap(); // idempotent
        let members = e.list_members(&g.id).await.unwrap();
        assert_eq!(members.len(), 1);
    }

    #[tokio::test]
    async fn test_remove_member() {
        let e = make_engine();
        let g = e.create(req("Team")).await.unwrap();
        e.add_member(&g.id, "u1").await.unwrap();
        e.remove_member(&g.id, "u1").await.unwrap();
        let members = e.list_members(&g.id).await.unwrap();
        assert!(members.is_empty());
    }

    #[tokio::test]
    async fn test_remove_member_not_found() {
        let e = make_engine();
        let g = e.create(req("Team")).await.unwrap();
        let err = e.remove_member(&g.id, "u1").await.unwrap_err();
        assert!(matches!(err, AppError::NotFound(_)));
    }

    #[tokio::test]
    async fn test_delete_group_cascades_members() {
        let e = make_engine();
        let g = e.create(req("Team")).await.unwrap();
        e.add_member(&g.id, "u1").await.unwrap();
        e.delete(&g.id).await.unwrap();
        // Members should be gone (cascade)
        let members = e.list_members(&g.id).await.unwrap();
        assert!(members.is_empty());
    }
}
