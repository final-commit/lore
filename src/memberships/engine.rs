use chrono::Utc;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::{with_conn, DbConn};
use crate::error::AppError;

// ── Data types ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Membership {
    pub id: String,
    pub user_id: String,
    pub collection_id: String,
    pub permission: String,
    pub created_by: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateMembership {
    pub user_id: String,
    pub collection_id: String,
    pub permission: Option<String>,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateMembership {
    pub permission: String,
}

// ── Engine ────────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct MembershipEngine {
    db: DbConn,
}

impl MembershipEngine {
    pub fn new(db: DbConn) -> Self {
        MembershipEngine { db }
    }

    pub async fn create(&self, req: CreateMembership) -> Result<Membership, AppError> {
        let db = self.db.clone();
        with_conn(&db, move |conn| {
            let id = Uuid::now_v7().to_string();
            let now = Utc::now().to_rfc3339();
            let permission = req.permission.unwrap_or_else(|| "read".to_string());
            conn.execute(
                "INSERT INTO user_memberships (id, user_id, collection_id, permission, created_by, created_at)
                 VALUES (?1,?2,?3,?4,?5,?6)",
                params![id, req.user_id, req.collection_id, permission, req.created_by, now],
            ).map_err(|e| {
                if e.to_string().contains("UNIQUE") {
                    rusqlite::Error::SqliteFailure(
                        rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_CONSTRAINT),
                        Some("membership already exists".into()),
                    )
                } else {
                    e
                }
            })?;
            conn.query_row(
                "SELECT id, user_id, collection_id, permission, created_by, created_at
                 FROM user_memberships WHERE id = ?1",
                params![id],
                row_to_membership,
            )
        })
        .await
        .map_err(|e| match e {
            AppError::Db(rusqlite::Error::SqliteFailure(ref err, _))
                if err.code == rusqlite::ffi::ErrorCode::ConstraintViolation =>
            {
                AppError::Conflict("membership already exists for this user+collection".into())
            }
            other => other,
        })
    }

    pub async fn list_for_collection(&self, collection_id: &str) -> Result<Vec<Membership>, AppError> {
        let db = self.db.clone();
        let collection_id = collection_id.to_string();
        with_conn(&db, move |conn| {
            let mut stmt = conn.prepare(
                "SELECT id, user_id, collection_id, permission, created_by, created_at
                 FROM user_memberships WHERE collection_id = ?1 ORDER BY created_at ASC",
            )?;
            let rows = stmt.query_map(params![collection_id], row_to_membership)?;
            rows.collect()
        })
        .await
    }

    pub async fn get(&self, id: &str) -> Result<Membership, AppError> {
        let db = self.db.clone();
        let id = id.to_string();
        let id2 = id.clone();
        with_conn(&db, move |conn| {
            conn.query_row(
                "SELECT id, user_id, collection_id, permission, created_by, created_at
                 FROM user_memberships WHERE id = ?1",
                params![id],
                row_to_membership,
            )
        })
        .await
        .map_err(|e| match e {
            AppError::Db(rusqlite::Error::QueryReturnedNoRows) => {
                AppError::NotFound(format!("membership {id2} not found"))
            }
            other => other,
        })
    }

    pub async fn update(&self, id: &str, req: UpdateMembership) -> Result<Membership, AppError> {
        let db = self.db.clone();
        let id = id.to_string();
        let id2 = id.clone();
        with_conn(&db, move |conn| {
            let rows = conn.execute(
                "UPDATE user_memberships SET permission = ?1 WHERE id = ?2",
                params![req.permission, id],
            )?;
            if rows == 0 {
                return Err(rusqlite::Error::QueryReturnedNoRows);
            }
            conn.query_row(
                "SELECT id, user_id, collection_id, permission, created_by, created_at
                 FROM user_memberships WHERE id = ?1",
                params![id],
                row_to_membership,
            )
        })
        .await
        .map_err(|e| match e {
            AppError::Db(rusqlite::Error::QueryReturnedNoRows) => {
                AppError::NotFound(format!("membership {id2} not found"))
            }
            other => other,
        })
    }

    pub async fn delete(&self, id: &str) -> Result<(), AppError> {
        let db = self.db.clone();
        let id = id.to_string();
        let id2 = id.clone();
        with_conn(&db, move |conn| {
            let rows = conn.execute("DELETE FROM user_memberships WHERE id = ?1", params![id])?;
            if rows == 0 {
                return Err(rusqlite::Error::QueryReturnedNoRows);
            }
            Ok(())
        })
        .await
        .map_err(|e| match e {
            AppError::Db(rusqlite::Error::QueryReturnedNoRows) => {
                AppError::NotFound(format!("membership {id2} not found"))
            }
            other => other,
        })
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn row_to_membership(row: &rusqlite::Row) -> rusqlite::Result<Membership> {
    Ok(Membership {
        id: row.get(0)?,
        user_id: row.get(1)?,
        collection_id: row.get(2)?,
        permission: row.get(3)?,
        created_by: row.get(4)?,
        created_at: row.get(5)?,
    })
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    fn make_engine() -> MembershipEngine {
        let db = db::open(":memory:").unwrap();
        {
            let conn = db.lock().unwrap();
            conn.execute_batch(
                "INSERT INTO users (id,email,name,password_hash,role,created_at,updated_at)
                 VALUES ('u1','a@b.com','Alice','hash','admin','2024-01-01T00:00:00Z','2024-01-01T00:00:00Z');
                 INSERT INTO users (id,email,name,password_hash,role,created_at,updated_at)
                 VALUES ('u2','b@c.com','Bob','hash','editor','2024-01-01T00:00:00Z','2024-01-01T00:00:00Z');
                 INSERT INTO collections (id,name,slug,permission,created_at,updated_at)
                 VALUES ('c1','Docs','docs','read','2024-01-01T00:00:00Z','2024-01-01T00:00:00Z');",
            ).unwrap();
        }
        MembershipEngine::new(db)
    }

    fn req(user: &str) -> CreateMembership {
        CreateMembership {
            user_id: user.to_string(),
            collection_id: "c1".to_string(),
            permission: Some("read".to_string()),
            created_by: Some("u1".to_string()),
        }
    }

    #[tokio::test]
    async fn test_create_membership() {
        let e = make_engine();
        let m = e.create(req("u1")).await.unwrap();
        assert_eq!(m.user_id, "u1");
        assert_eq!(m.permission, "read");
    }

    #[tokio::test]
    async fn test_list_for_collection() {
        let e = make_engine();
        e.create(req("u1")).await.unwrap();
        e.create(req("u2")).await.unwrap();
        let members = e.list_for_collection("c1").await.unwrap();
        assert_eq!(members.len(), 2);
    }

    #[tokio::test]
    async fn test_conflict_on_duplicate() {
        let e = make_engine();
        e.create(req("u1")).await.unwrap();
        let err = e.create(req("u1")).await.unwrap_err();
        assert!(matches!(err, AppError::Conflict(_)));
    }

    #[tokio::test]
    async fn test_update_permission() {
        let e = make_engine();
        let m = e.create(req("u1")).await.unwrap();
        let updated = e.update(&m.id, UpdateMembership { permission: "admin".into() }).await.unwrap();
        assert_eq!(updated.permission, "admin");
    }

    #[tokio::test]
    async fn test_delete_membership() {
        let e = make_engine();
        let m = e.create(req("u1")).await.unwrap();
        e.delete(&m.id).await.unwrap();
        let err = e.get(&m.id).await.unwrap_err();
        assert!(matches!(err, AppError::NotFound(_)));
    }

    #[tokio::test]
    async fn test_get_not_found() {
        let e = make_engine();
        let err = e.get("no-such").await.unwrap_err();
        assert!(matches!(err, AppError::NotFound(_)));
    }
}
