use chrono::Utc;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::{with_conn, DbConn};
use crate::error::AppError;

// ── Well-known event names ────────────────────────────────────────────────────

pub mod names {
    pub const DOCUMENTS_CREATE: &str = "documents.create";
    pub const DOCUMENTS_UPDATE: &str = "documents.update";
    pub const DOCUMENTS_DELETE: &str = "documents.delete";
    pub const DOCUMENTS_PUBLISH: &str = "documents.publish";
    pub const DOCUMENTS_ARCHIVE: &str = "documents.archive";
    pub const DOCUMENTS_RESTORE: &str = "documents.restore";
    pub const COLLECTIONS_CREATE: &str = "collections.create";
    pub const COLLECTIONS_UPDATE: &str = "collections.update";
    pub const COLLECTIONS_DELETE: &str = "collections.delete";
    pub const USERS_LOGIN: &str = "users.login";
    pub const SHARES_CREATE: &str = "shares.create";
    pub const SHARES_REVOKE: &str = "shares.revoke";
}

// ── Data types ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: String,
    pub name: String,
    pub actor_id: Option<String>,
    pub doc_path: Option<String>,
    pub collection_id: Option<String>,
    pub data: Option<String>,
    pub ip_address: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct EmitEvent {
    pub name: String,
    pub actor_id: Option<String>,
    pub doc_path: Option<String>,
    pub collection_id: Option<String>,
    pub data: Option<String>,
    pub ip_address: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ListEventsQuery {
    pub doc_path: Option<String>,
    pub collection_id: Option<String>,
    pub actor_id: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

// ── Engine ────────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct EventEngine {
    db: DbConn,
}

impl EventEngine {
    pub fn new(db: DbConn) -> Self {
        EventEngine { db }
    }

    /// Emit (record) an event. Fire-and-forget from handlers; errors are logged.
    pub async fn emit(&self, ev: EmitEvent) -> Result<Event, AppError> {
        let db = self.db.clone();
        with_conn(&db, move |conn| {
            let id = Uuid::now_v7().to_string();
            let now = Utc::now().to_rfc3339();
            conn.execute(
                "INSERT INTO events (id, name, actor_id, doc_path, collection_id, data, ip_address, created_at)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
                params![id, ev.name, ev.actor_id, ev.doc_path, ev.collection_id, ev.data, ev.ip_address, now],
            )?;
            conn.query_row(
                "SELECT id, name, actor_id, doc_path, collection_id, data, ip_address, created_at
                 FROM events WHERE id = ?1",
                params![id],
                row_to_event,
            )
        })
        .await
    }

    /// List events with optional filters, paginated.
    pub async fn list(&self, q: ListEventsQuery) -> Result<Vec<Event>, AppError> {
        let db = self.db.clone();
        let limit = q.limit.unwrap_or(50).min(200);
        let offset = q.offset.unwrap_or(0);
        with_conn(&db, move |conn| {
            // Build a dynamic WHERE clause.
            let mut conditions: Vec<String> = vec![];
            let mut idx = 1i32;

            if q.doc_path.is_some() {
                conditions.push(format!("doc_path = ?{idx}"));
                idx += 1;
            }
            if q.collection_id.is_some() {
                conditions.push(format!("collection_id = ?{idx}"));
                idx += 1;
            }
            if q.actor_id.is_some() {
                conditions.push(format!("actor_id = ?{idx}"));
                idx += 1;
            }

            let where_clause = if conditions.is_empty() {
                String::new()
            } else {
                format!("WHERE {}", conditions.join(" AND "))
            };

            let limit_idx = idx;
            let offset_idx = idx + 1;
            let sql = format!(
                "SELECT id, name, actor_id, doc_path, collection_id, data, ip_address, created_at
                 FROM events {where_clause}
                 ORDER BY created_at DESC
                 LIMIT ?{limit_idx} OFFSET ?{offset_idx}"
            );

            // Collect params as owned Values so we can pass to query_map.
            let mut param_values: Vec<rusqlite::types::Value> = vec![];
            if let Some(ref v) = q.doc_path {
                param_values.push(rusqlite::types::Value::Text(v.clone()));
            }
            if let Some(ref v) = q.collection_id {
                param_values.push(rusqlite::types::Value::Text(v.clone()));
            }
            if let Some(ref v) = q.actor_id {
                param_values.push(rusqlite::types::Value::Text(v.clone()));
            }
            param_values.push(rusqlite::types::Value::Integer(limit));
            param_values.push(rusqlite::types::Value::Integer(offset));

            let mut stmt = conn.prepare(&sql)?;
            let rows = stmt.query_map(rusqlite::params_from_iter(param_values.iter()), row_to_event)?;
            rows.collect()
        })
        .await
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn row_to_event(row: &rusqlite::Row) -> rusqlite::Result<Event> {
    Ok(Event {
        id: row.get(0)?,
        name: row.get(1)?,
        actor_id: row.get(2)?,
        doc_path: row.get(3)?,
        collection_id: row.get(4)?,
        data: row.get(5)?,
        ip_address: row.get(6)?,
        created_at: row.get(7)?,
    })
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    fn make_engine() -> EventEngine {
        let db = db::open(":memory:").unwrap();
        {
            let conn = db.lock().unwrap();
            conn.execute(
                "INSERT INTO users (id,email,name,password_hash,role,created_at,updated_at)
                 VALUES ('u1','a@b.com','Alice','hash','editor','2024-01-01T00:00:00Z','2024-01-01T00:00:00Z')",
                [],
            ).unwrap();
        }
        EventEngine::new(db)
    }

    fn doc_event(name: &str, doc: &str) -> EmitEvent {
        EmitEvent {
            name: name.to_string(),
            actor_id: Some("u1".to_string()),
            doc_path: Some(doc.to_string()),
            collection_id: None,
            data: None,
            ip_address: None,
        }
    }

    #[tokio::test]
    async fn test_emit_event() {
        let engine = make_engine();
        let ev = engine.emit(doc_event(names::DOCUMENTS_CREATE, "docs/foo.md")).await.unwrap();
        assert_eq!(ev.name, names::DOCUMENTS_CREATE);
        assert_eq!(ev.doc_path, Some("docs/foo.md".into()));
    }

    #[tokio::test]
    async fn test_emit_no_actor() {
        let engine = make_engine();
        let ev = engine
            .emit(EmitEvent {
                name: names::USERS_LOGIN.to_string(),
                actor_id: None,
                doc_path: None,
                collection_id: None,
                data: None,
                ip_address: Some("127.0.0.1".into()),
            })
            .await
            .unwrap();
        assert_eq!(ev.name, names::USERS_LOGIN);
        assert_eq!(ev.ip_address, Some("127.0.0.1".into()));
    }

    #[tokio::test]
    async fn test_list_all_events() {
        let engine = make_engine();
        engine.emit(doc_event(names::DOCUMENTS_CREATE, "docs/a.md")).await.unwrap();
        engine.emit(doc_event(names::DOCUMENTS_UPDATE, "docs/b.md")).await.unwrap();
        let events = engine.list(ListEventsQuery {
            doc_path: None, collection_id: None, actor_id: None, limit: None, offset: None,
        }).await.unwrap();
        assert_eq!(events.len(), 2);
    }

    #[tokio::test]
    async fn test_list_filter_by_doc() {
        let engine = make_engine();
        engine.emit(doc_event(names::DOCUMENTS_CREATE, "docs/a.md")).await.unwrap();
        engine.emit(doc_event(names::DOCUMENTS_CREATE, "docs/b.md")).await.unwrap();
        let events = engine.list(ListEventsQuery {
            doc_path: Some("docs/a.md".into()),
            collection_id: None, actor_id: None, limit: None, offset: None,
        }).await.unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].doc_path, Some("docs/a.md".into()));
    }

    #[tokio::test]
    async fn test_list_filter_by_actor() {
        let engine = make_engine();
        engine.emit(doc_event(names::DOCUMENTS_CREATE, "docs/a.md")).await.unwrap();
        let events = engine.list(ListEventsQuery {
            doc_path: None,
            collection_id: None,
            actor_id: Some("u1".into()),
            limit: None, offset: None,
        }).await.unwrap();
        assert_eq!(events.len(), 1);
    }

    #[tokio::test]
    async fn test_list_pagination() {
        let engine = make_engine();
        for i in 0..10 {
            engine.emit(doc_event(names::DOCUMENTS_CREATE, &format!("docs/{i}.md"))).await.unwrap();
        }
        let page1 = engine.list(ListEventsQuery {
            doc_path: None, collection_id: None, actor_id: None, limit: Some(3), offset: Some(0),
        }).await.unwrap();
        let page2 = engine.list(ListEventsQuery {
            doc_path: None, collection_id: None, actor_id: None, limit: Some(3), offset: Some(3),
        }).await.unwrap();
        assert_eq!(page1.len(), 3);
        assert_eq!(page2.len(), 3);
        // Pages should not overlap.
        assert_ne!(page1[0].id, page2[0].id);
    }

    #[tokio::test]
    async fn test_list_limit_cap() {
        let engine = make_engine();
        // Requesting more than 200 should be capped to 200.
        let events = engine.list(ListEventsQuery {
            doc_path: None, collection_id: None, actor_id: None, limit: Some(9999), offset: None,
        }).await.unwrap();
        // No data, so 0 results — just verify no panic.
        assert_eq!(events.len(), 0);
    }
}
