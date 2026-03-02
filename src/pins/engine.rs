use chrono::Utc;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::{with_conn, DbConn};
use crate::error::AppError;

// ── Data types ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pin {
    pub id: String,
    pub doc_path: String,
    pub collection_id: Option<String>,
    pub pinned_by: String,
    pub sort_order: i64,
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreatePin {
    pub doc_path: String,
    pub collection_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdatePin {
    pub sort_order: i64,
}

// ── Engine ────────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct PinEngine {
    db: DbConn,
}

impl PinEngine {
    pub fn new(db: DbConn) -> Self {
        PinEngine { db }
    }

    /// Create a pin. Returns Conflict if (doc_path, collection_id) already pinned.
    pub async fn create(&self, req: CreatePin, pinned_by: &str) -> Result<Pin, AppError> {
        let db = self.db.clone();
        let pinned_by = pinned_by.to_string();
        with_conn(&db, move |conn| {
            let id = Uuid::now_v7().to_string();
            let now = Utc::now().to_rfc3339();
            conn.execute(
                "INSERT INTO pins (id, doc_path, collection_id, pinned_by, sort_order, created_at)
                 VALUES (?1,?2,?3,?4,0,?5)",
                params![id, req.doc_path, req.collection_id, pinned_by, now],
            )?;
            conn.query_row(
                "SELECT id, doc_path, collection_id, pinned_by, sort_order, created_at FROM pins WHERE id = ?1",
                params![id],
                row_to_pin,
            )
        })
        .await
        .map_err(|e| match e {
            AppError::Db(rusqlite::Error::SqliteFailure(
                rusqlite::ffi::Error { code: rusqlite::ffi::ErrorCode::ConstraintViolation, .. },
                _,
            )) => AppError::Conflict("document is already pinned in this collection".into()),
            other => other,
        })
    }

    /// List pins for a collection (or all global pins if collection_id is None).
    pub async fn list_for_collection(
        &self,
        collection_id: Option<&str>,
    ) -> Result<Vec<Pin>, AppError> {
        let db = self.db.clone();
        let collection_id = collection_id.map(|s| s.to_string());
        with_conn(&db, move |conn| {
            let rows: Vec<Pin> = match collection_id {
                Some(ref cid) => {
                    let mut stmt = conn.prepare(
                        "SELECT id, doc_path, collection_id, pinned_by, sort_order, created_at
                         FROM pins WHERE collection_id = ?1 ORDER BY sort_order ASC, created_at ASC",
                    )?;
                    stmt.query_map(params![cid], row_to_pin)?.collect::<rusqlite::Result<_>>()?
                }
                None => {
                    let mut stmt = conn.prepare(
                        "SELECT id, doc_path, collection_id, pinned_by, sort_order, created_at
                         FROM pins WHERE collection_id IS NULL ORDER BY sort_order ASC, created_at ASC",
                    )?;
                    stmt.query_map([], row_to_pin)?.collect::<rusqlite::Result<_>>()?
                }
            };
            Ok(rows)
        })
        .await
    }

    /// Delete a pin by ID.
    pub async fn delete(&self, id: &str) -> Result<(), AppError> {
        let db = self.db.clone();
        let id = id.to_string();
        let id2 = id.clone();
        with_conn(&db, move |conn| {
            let rows = conn.execute("DELETE FROM pins WHERE id = ?1", params![id])?;
            if rows == 0 {
                return Err(rusqlite::Error::QueryReturnedNoRows);
            }
            Ok(())
        })
        .await
        .map_err(|e| match e {
            AppError::Db(rusqlite::Error::QueryReturnedNoRows) => {
                AppError::NotFound(format!("pin {id2} not found"))
            }
            other => other,
        })
    }

    /// Reorder a pin.
    pub async fn reorder(&self, id: &str, sort_order: i64) -> Result<Pin, AppError> {
        let db = self.db.clone();
        let id = id.to_string();
        let id2 = id.clone();
        with_conn(&db, move |conn| {
            let rows = conn.execute(
                "UPDATE pins SET sort_order = ?1 WHERE id = ?2",
                params![sort_order, id],
            )?;
            if rows == 0 {
                return Err(rusqlite::Error::QueryReturnedNoRows);
            }
            conn.query_row(
                "SELECT id, doc_path, collection_id, pinned_by, sort_order, created_at FROM pins WHERE id = ?1",
                params![id],
                row_to_pin,
            )
        })
        .await
        .map_err(|e| match e {
            AppError::Db(rusqlite::Error::QueryReturnedNoRows) => {
                AppError::NotFound(format!("pin {id2} not found"))
            }
            other => other,
        })
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn row_to_pin(row: &rusqlite::Row) -> rusqlite::Result<Pin> {
    Ok(Pin {
        id: row.get(0)?,
        doc_path: row.get(1)?,
        collection_id: row.get(2)?,
        pinned_by: row.get(3)?,
        sort_order: row.get(4)?,
        created_at: row.get(5)?,
    })
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    fn make_engine() -> PinEngine {
        let db = db::open(":memory:").unwrap();
        {
            let conn = db.lock().unwrap();
            conn.execute(
                "INSERT INTO users (id,email,name,password_hash,role,created_at,updated_at)
                 VALUES ('u1','a@b.com','Alice','hash','editor','2024-01-01T00:00:00Z','2024-01-01T00:00:00Z')",
                [],
            ).unwrap();
            conn.execute(
                "INSERT INTO collections (id,name,slug,permission,sort_order,created_at,updated_at)
                 VALUES ('c1','Eng','eng','read',0,'2024-01-01T00:00:00Z','2024-01-01T00:00:00Z')",
                [],
            ).unwrap();
        }
        PinEngine::new(db)
    }

    #[tokio::test]
    async fn test_create_pin() {
        let engine = make_engine();
        let pin = engine
            .create(CreatePin { doc_path: "docs/a.md".into(), collection_id: Some("c1".into()) }, "u1")
            .await
            .unwrap();
        assert_eq!(pin.doc_path, "docs/a.md");
        assert_eq!(pin.collection_id, Some("c1".into()));
        assert_eq!(pin.sort_order, 0);
    }

    #[tokio::test]
    async fn test_duplicate_pin_rejected() {
        let engine = make_engine();
        engine
            .create(CreatePin { doc_path: "docs/a.md".into(), collection_id: Some("c1".into()) }, "u1")
            .await
            .unwrap();
        let err = engine
            .create(CreatePin { doc_path: "docs/a.md".into(), collection_id: Some("c1".into()) }, "u1")
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::Conflict(_)));
    }

    #[tokio::test]
    async fn test_list_for_collection() {
        let engine = make_engine();
        engine
            .create(CreatePin { doc_path: "docs/a.md".into(), collection_id: Some("c1".into()) }, "u1")
            .await
            .unwrap();
        engine
            .create(CreatePin { doc_path: "docs/b.md".into(), collection_id: Some("c1".into()) }, "u1")
            .await
            .unwrap();
        let pins = engine.list_for_collection(Some("c1")).await.unwrap();
        assert_eq!(pins.len(), 2);
    }

    #[tokio::test]
    async fn test_list_for_null_collection() {
        let engine = make_engine();
        engine
            .create(CreatePin { doc_path: "docs/a.md".into(), collection_id: None }, "u1")
            .await
            .unwrap();
        let pins = engine.list_for_collection(None).await.unwrap();
        assert_eq!(pins.len(), 1);
    }

    #[tokio::test]
    async fn test_delete_pin() {
        let engine = make_engine();
        let pin = engine
            .create(CreatePin { doc_path: "docs/a.md".into(), collection_id: Some("c1".into()) }, "u1")
            .await
            .unwrap();
        engine.delete(&pin.id).await.unwrap();
        let err = engine.delete(&pin.id).await.unwrap_err();
        assert!(matches!(err, AppError::NotFound(_)));
    }

    #[tokio::test]
    async fn test_reorder_pin() {
        let engine = make_engine();
        let pin = engine
            .create(CreatePin { doc_path: "docs/a.md".into(), collection_id: Some("c1".into()) }, "u1")
            .await
            .unwrap();
        let updated = engine.reorder(&pin.id, 5).await.unwrap();
        assert_eq!(updated.sort_order, 5);
    }

    #[tokio::test]
    async fn test_reorder_not_found() {
        let engine = make_engine();
        let err = engine.reorder("no-such", 3).await.unwrap_err();
        assert!(matches!(err, AppError::NotFound(_)));
    }
}
