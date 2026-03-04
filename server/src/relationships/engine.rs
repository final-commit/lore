use chrono::Utc;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::{with_conn, DbConn};
use crate::error::AppError;

// ── Data types ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationship {
    pub id: String,
    pub source_doc_path: String,
    pub target_doc_path: String,
    pub rel_type: String,
    pub created_by: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateRelationship {
    pub source_doc_path: String,
    pub target_doc_path: String,
    pub rel_type: Option<String>,
    pub created_by: Option<String>,
}

// ── Engine ────────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct RelationshipEngine {
    db: DbConn,
}

impl RelationshipEngine {
    pub fn new(db: DbConn) -> Self {
        RelationshipEngine { db }
    }

    pub async fn create(&self, req: CreateRelationship) -> Result<Relationship, AppError> {
        if req.source_doc_path == req.target_doc_path {
            return Err(AppError::BadRequest("source and target doc paths must differ".into()));
        }
        let db = self.db.clone();
        with_conn(&db, move |conn| {
            let id = Uuid::now_v7().to_string();
            let now = Utc::now().to_rfc3339();
            let rel_type = req.rel_type.unwrap_or_else(|| "related".to_string());
            conn.execute(
                "INSERT INTO relationships (id, source_doc_path, target_doc_path, rel_type, created_by, created_at)
                 VALUES (?1,?2,?3,?4,?5,?6)",
                params![id, req.source_doc_path, req.target_doc_path, rel_type, req.created_by, now],
            ).map_err(|e| {
                if e.to_string().contains("UNIQUE") {
                    rusqlite::Error::SqliteFailure(
                        rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_CONSTRAINT),
                        Some("relationship already exists".into()),
                    )
                } else {
                    e
                }
            })?;
            conn.query_row(
                "SELECT id, source_doc_path, target_doc_path, rel_type, created_by, created_at
                 FROM relationships WHERE id = ?1",
                params![id],
                row_to_relationship,
            )
        })
        .await
        .map_err(|e| match e {
            AppError::Db(rusqlite::Error::SqliteFailure(ref err, _))
                if err.code == rusqlite::ffi::ErrorCode::ConstraintViolation =>
            {
                AppError::Conflict("relationship already exists".into())
            }
            other => other,
        })
    }

    /// List all relationships involving a given doc (as source or target).
    pub async fn list_for_doc(&self, doc_path: &str) -> Result<Vec<Relationship>, AppError> {
        let db = self.db.clone();
        let doc_path = doc_path.to_string();
        with_conn(&db, move |conn| {
            let mut stmt = conn.prepare(
                "SELECT id, source_doc_path, target_doc_path, rel_type, created_by, created_at
                 FROM relationships
                 WHERE source_doc_path = ?1 OR target_doc_path = ?1
                 ORDER BY created_at DESC",
            )?;
            let rows = stmt.query_map(params![doc_path], row_to_relationship)?;
            rows.collect()
        })
        .await
    }

    pub async fn delete(&self, id: &str) -> Result<(), AppError> {
        let db = self.db.clone();
        let id = id.to_string();
        let id2 = id.clone();
        with_conn(&db, move |conn| {
            let rows = conn.execute("DELETE FROM relationships WHERE id = ?1", params![id])?;
            if rows == 0 { return Err(rusqlite::Error::QueryReturnedNoRows); }
            Ok(())
        })
        .await
        .map_err(|e| match e {
            AppError::Db(rusqlite::Error::QueryReturnedNoRows) => {
                AppError::NotFound(format!("relationship {id2} not found"))
            }
            other => other,
        })
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn row_to_relationship(row: &rusqlite::Row) -> rusqlite::Result<Relationship> {
    Ok(Relationship {
        id: row.get(0)?,
        source_doc_path: row.get(1)?,
        target_doc_path: row.get(2)?,
        rel_type: row.get(3)?,
        created_by: row.get(4)?,
        created_at: row.get(5)?,
    })
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    fn make_engine() -> RelationshipEngine {
        let db = db::open(":memory:").unwrap();
        {
            let conn = db.lock().unwrap();
            conn.execute(
                "INSERT INTO users (id,email,name,password_hash,role,created_at,updated_at)
                 VALUES ('u1','a@b.com','Alice','hash','admin','2024-01-01T00:00:00Z','2024-01-01T00:00:00Z')",
                [],
            ).unwrap();
        }
        RelationshipEngine::new(db)
    }

    fn req(src: &str, tgt: &str) -> CreateRelationship {
        CreateRelationship {
            source_doc_path: src.to_string(),
            target_doc_path: tgt.to_string(),
            rel_type: None,
            created_by: Some("u1".to_string()),
        }
    }

    #[tokio::test]
    async fn test_create_relationship() {
        let e = make_engine();
        let r = e.create(req("a.md", "b.md")).await.unwrap();
        assert_eq!(r.source_doc_path, "a.md");
        assert_eq!(r.target_doc_path, "b.md");
        assert_eq!(r.rel_type, "related");
    }

    #[tokio::test]
    async fn test_create_self_reference_error() {
        let e = make_engine();
        let err = e.create(req("a.md", "a.md")).await.unwrap_err();
        assert!(matches!(err, AppError::BadRequest(_)));
    }

    #[tokio::test]
    async fn test_create_conflict() {
        let e = make_engine();
        e.create(req("a.md", "b.md")).await.unwrap();
        let err = e.create(req("a.md", "b.md")).await.unwrap_err();
        assert!(matches!(err, AppError::Conflict(_)));
    }

    #[tokio::test]
    async fn test_list_for_doc() {
        let e = make_engine();
        e.create(req("a.md", "b.md")).await.unwrap();
        e.create(req("c.md", "a.md")).await.unwrap();
        let rels = e.list_for_doc("a.md").await.unwrap();
        assert_eq!(rels.len(), 2);
    }

    #[tokio::test]
    async fn test_list_empty_for_unknown_doc() {
        let e = make_engine();
        let rels = e.list_for_doc("unknown.md").await.unwrap();
        assert!(rels.is_empty());
    }

    #[tokio::test]
    async fn test_delete_relationship() {
        let e = make_engine();
        let r = e.create(req("a.md", "b.md")).await.unwrap();
        e.delete(&r.id).await.unwrap();
        let rels = e.list_for_doc("a.md").await.unwrap();
        assert!(rels.is_empty());
    }

    #[tokio::test]
    async fn test_delete_not_found() {
        let e = make_engine();
        let err = e.delete("no-such").await.unwrap_err();
        assert!(matches!(err, AppError::NotFound(_)));
    }

    #[tokio::test]
    async fn test_custom_rel_type() {
        let e = make_engine();
        let r = e.create(CreateRelationship {
            source_doc_path: "a.md".into(),
            target_doc_path: "b.md".into(),
            rel_type: Some("prerequisite".into()),
            created_by: Some("u1".into()),
        }).await.unwrap();
        assert_eq!(r.rel_type, "prerequisite");
    }
}
