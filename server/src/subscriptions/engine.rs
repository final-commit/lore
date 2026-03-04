use chrono::Utc;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::{with_conn, DbConn};
use crate::error::AppError;

// ── Data types ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subscription {
    pub id: String,
    pub user_id: String,
    pub doc_path: String,
    pub event: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateSubscription {
    pub user_id: String,
    pub doc_path: String,
    pub event: Option<String>,
}

// ── Engine ────────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct SubscriptionEngine {
    db: DbConn,
}

impl SubscriptionEngine {
    pub fn new(db: DbConn) -> Self {
        SubscriptionEngine { db }
    }

    /// Subscribe a user to a doc+event. Idempotent — returns existing if already subscribed.
    pub async fn subscribe(&self, req: CreateSubscription) -> Result<Subscription, AppError> {
        let db = self.db.clone();
        let event = req.event.unwrap_or_else(|| "documents.update".to_string());
        with_conn(&db, move |conn| {
            let id = Uuid::now_v7().to_string();
            let now = Utc::now().to_rfc3339();
            conn.execute(
                "INSERT OR IGNORE INTO subscriptions (id, user_id, doc_path, event, created_at)
                 VALUES (?1,?2,?3,?4,?5)",
                params![id, req.user_id, req.doc_path, event, now],
            )?;
            conn.query_row(
                "SELECT id, user_id, doc_path, event, created_at
                 FROM subscriptions WHERE user_id = ?1 AND doc_path = ?2 AND event = ?3",
                params![req.user_id, req.doc_path, event],
                row_to_subscription,
            )
        })
        .await
    }

    /// List subscriptions for a user, optionally filtered by doc_path.
    pub async fn list_for_user(
        &self,
        user_id: &str,
        doc_path: Option<&str>,
    ) -> Result<Vec<Subscription>, AppError> {
        let db = self.db.clone();
        let user_id = user_id.to_string();
        let doc_path = doc_path.map(|s| s.to_string());
        with_conn(&db, move |conn| {
            if let Some(ref dp) = doc_path {
                let mut stmt = conn.prepare(
                    "SELECT id, user_id, doc_path, event, created_at
                     FROM subscriptions WHERE user_id = ?1 AND doc_path = ?2
                     ORDER BY created_at DESC",
                )?;
                let rows = stmt.query_map(params![user_id, dp], row_to_subscription)?;
                rows.collect()
            } else {
                let mut stmt = conn.prepare(
                    "SELECT id, user_id, doc_path, event, created_at
                     FROM subscriptions WHERE user_id = ?1
                     ORDER BY created_at DESC",
                )?;
                let rows = stmt.query_map(params![user_id], row_to_subscription)?;
                rows.collect()
            }
        })
        .await
    }

    /// List all subscribers for a given doc_path+event (used to fan-out notifications).
    pub async fn list_subscribers(
        &self,
        doc_path: &str,
        event: &str,
    ) -> Result<Vec<Subscription>, AppError> {
        let db = self.db.clone();
        let doc_path = doc_path.to_string();
        let event = event.to_string();
        with_conn(&db, move |conn| {
            let mut stmt = conn.prepare(
                "SELECT id, user_id, doc_path, event, created_at
                 FROM subscriptions WHERE doc_path = ?1 AND event = ?2",
            )?;
            let rows = stmt.query_map(params![doc_path, event], row_to_subscription)?;
            rows.collect()
        })
        .await
    }

    pub async fn delete(&self, id: &str, user_id: &str) -> Result<(), AppError> {
        let db = self.db.clone();
        let id = id.to_string();
        let id2 = id.clone();
        let user_id = user_id.to_string();
        with_conn(&db, move |conn| {
            let rows = conn.execute(
                "DELETE FROM subscriptions WHERE id = ?1 AND user_id = ?2",
                params![id, user_id],
            )?;
            if rows == 0 {
                return Err(rusqlite::Error::QueryReturnedNoRows);
            }
            Ok(())
        })
        .await
        .map_err(|e| match e {
            AppError::Db(rusqlite::Error::QueryReturnedNoRows) => {
                AppError::NotFound(format!("subscription {id2} not found"))
            }
            other => other,
        })
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn row_to_subscription(row: &rusqlite::Row) -> rusqlite::Result<Subscription> {
    Ok(Subscription {
        id: row.get(0)?,
        user_id: row.get(1)?,
        doc_path: row.get(2)?,
        event: row.get(3)?,
        created_at: row.get(4)?,
    })
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    fn make_engine() -> SubscriptionEngine {
        let db = db::open(":memory:").unwrap();
        {
            let conn = db.lock().unwrap();
            conn.execute_batch(
                "INSERT INTO users (id,email,name,password_hash,role,created_at,updated_at)
                 VALUES ('u1','a@b.com','Alice','hash','editor','2024-01-01T00:00:00Z','2024-01-01T00:00:00Z');
                 INSERT INTO users (id,email,name,password_hash,role,created_at,updated_at)
                 VALUES ('u2','b@c.com','Bob','hash','editor','2024-01-01T00:00:00Z','2024-01-01T00:00:00Z');",
            ).unwrap();
        }
        SubscriptionEngine::new(db)
    }

    #[tokio::test]
    async fn test_subscribe() {
        let e = make_engine();
        let s = e.subscribe(CreateSubscription {
            user_id: "u1".into(),
            doc_path: "docs/foo.md".into(),
            event: None,
        }).await.unwrap();
        assert_eq!(s.user_id, "u1");
        assert_eq!(s.event, "documents.update");
    }

    #[tokio::test]
    async fn test_subscribe_idempotent() {
        let e = make_engine();
        let s1 = e.subscribe(CreateSubscription { user_id: "u1".into(), doc_path: "a.md".into(), event: None }).await.unwrap();
        let s2 = e.subscribe(CreateSubscription { user_id: "u1".into(), doc_path: "a.md".into(), event: None }).await.unwrap();
        assert_eq!(s1.id, s2.id);
    }

    #[tokio::test]
    async fn test_list_for_user() {
        let e = make_engine();
        e.subscribe(CreateSubscription { user_id: "u1".into(), doc_path: "a.md".into(), event: None }).await.unwrap();
        e.subscribe(CreateSubscription { user_id: "u1".into(), doc_path: "b.md".into(), event: None }).await.unwrap();
        let list = e.list_for_user("u1", None).await.unwrap();
        assert_eq!(list.len(), 2);
    }

    #[tokio::test]
    async fn test_list_for_user_filter_doc() {
        let e = make_engine();
        e.subscribe(CreateSubscription { user_id: "u1".into(), doc_path: "a.md".into(), event: None }).await.unwrap();
        e.subscribe(CreateSubscription { user_id: "u1".into(), doc_path: "b.md".into(), event: None }).await.unwrap();
        let list = e.list_for_user("u1", Some("a.md")).await.unwrap();
        assert_eq!(list.len(), 1);
    }

    #[tokio::test]
    async fn test_list_subscribers() {
        let e = make_engine();
        e.subscribe(CreateSubscription { user_id: "u1".into(), doc_path: "a.md".into(), event: Some("documents.update".into()) }).await.unwrap();
        e.subscribe(CreateSubscription { user_id: "u2".into(), doc_path: "a.md".into(), event: Some("documents.update".into()) }).await.unwrap();
        let subs = e.list_subscribers("a.md", "documents.update").await.unwrap();
        assert_eq!(subs.len(), 2);
    }

    #[tokio::test]
    async fn test_delete_subscription() {
        let e = make_engine();
        let s = e.subscribe(CreateSubscription { user_id: "u1".into(), doc_path: "a.md".into(), event: None }).await.unwrap();
        e.delete(&s.id, "u1").await.unwrap();
        let list = e.list_for_user("u1", None).await.unwrap();
        assert!(list.is_empty());
    }

    #[tokio::test]
    async fn test_delete_wrong_user() {
        let e = make_engine();
        let s = e.subscribe(CreateSubscription { user_id: "u1".into(), doc_path: "a.md".into(), event: None }).await.unwrap();
        let err = e.delete(&s.id, "u2").await.unwrap_err();
        assert!(matches!(err, AppError::NotFound(_)));
    }
}
