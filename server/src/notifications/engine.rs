use chrono::Utc;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::{with_conn, DbConn};
use crate::error::AppError;

// ── Well-known notification types ─────────────────────────────────────────────

pub mod types {
    pub const DOCUMENT_UPDATED: &str = "document.updated";
    pub const COMMENT_CREATED: &str = "comment.created";
    pub const MENTION: &str = "mention";
    pub const SHARE_CREATED: &str = "share.created";
}

// ── Data types ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub id: String,
    pub user_id: String,
    pub event_id: Option<String>,
    #[serde(rename = "type")]
    pub notification_type: String,
    pub read_at: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct CreateNotification {
    pub user_id: String,
    pub event_id: Option<String>,
    pub notification_type: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ListNotificationsQuery {
    pub read: Option<bool>,
    pub limit: Option<i64>,
}

// ── Engine ────────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct NotificationEngine {
    db: DbConn,
}

impl NotificationEngine {
    pub fn new(db: DbConn) -> Self {
        NotificationEngine { db }
    }

    pub async fn create(&self, req: CreateNotification) -> Result<Notification, AppError> {
        let db = self.db.clone();
        with_conn(&db, move |conn| {
            let id = Uuid::now_v7().to_string();
            let now = Utc::now().to_rfc3339();
            conn.execute(
                "INSERT INTO notifications (id, user_id, event_id, type, created_at)
                 VALUES (?1,?2,?3,?4,?5)",
                params![id, req.user_id, req.event_id, req.notification_type, now],
            )?;
            conn.query_row(
                "SELECT id, user_id, event_id, type, read_at, created_at
                 FROM notifications WHERE id = ?1",
                params![id],
                row_to_notification,
            )
        })
        .await
    }

    /// List notifications for a user — unread first, then by created_at desc.
    pub async fn list_for_user(
        &self,
        user_id: &str,
        q: ListNotificationsQuery,
    ) -> Result<Vec<Notification>, AppError> {
        let db = self.db.clone();
        let user_id = user_id.to_string();
        let limit = q.limit.unwrap_or(50).min(200);
        with_conn(&db, move |conn| {
            let read_filter = match q.read {
                Some(true) => " AND read_at IS NOT NULL",
                Some(false) => " AND read_at IS NULL",
                None => "",
            };
            let sql = format!(
                "SELECT id, user_id, event_id, type, read_at, created_at
                 FROM notifications
                 WHERE user_id = ?1{read_filter}
                 ORDER BY read_at ASC, created_at DESC
                 LIMIT ?2"
            );
            let mut stmt = conn.prepare(&sql)?;
            let rows = stmt.query_map(params![user_id, limit], row_to_notification)?;
            rows.collect()
        })
        .await
    }

    /// Mark a single notification as read.
    pub async fn mark_read(&self, id: &str, user_id: &str) -> Result<Notification, AppError> {
        let db = self.db.clone();
        let id = id.to_string();
        let id2 = id.clone();
        let user_id = user_id.to_string();
        with_conn(&db, move |conn| {
            let now = Utc::now().to_rfc3339();
            let rows = conn.execute(
                "UPDATE notifications SET read_at = ?1 WHERE id = ?2 AND user_id = ?3 AND read_at IS NULL",
                params![now, id, user_id],
            )?;
            if rows == 0 {
                // Check if notification exists at all.
                let exists: bool = conn
                    .query_row(
                        "SELECT COUNT(*) FROM notifications WHERE id = ?1",
                        params![id],
                        |r| r.get::<_, i64>(0),
                    )
                    .map(|c| c > 0)
                    .unwrap_or(false);
                if !exists {
                    return Err(rusqlite::Error::QueryReturnedNoRows);
                }
                // Already read — just return it.
            }
            conn.query_row(
                "SELECT id, user_id, event_id, type, read_at, created_at
                 FROM notifications WHERE id = ?1",
                params![id],
                row_to_notification,
            )
        })
        .await
        .map_err(|e| match e {
            AppError::Db(rusqlite::Error::QueryReturnedNoRows) => {
                AppError::NotFound(format!("notification {id2} not found"))
            }
            other => other,
        })
    }

    /// Mark all unread notifications for a user as read.
    pub async fn mark_all_read(&self, user_id: &str) -> Result<u64, AppError> {
        let db = self.db.clone();
        let user_id = user_id.to_string();
        with_conn(&db, move |conn| {
            let now = Utc::now().to_rfc3339();
            let rows = conn.execute(
                "UPDATE notifications SET read_at = ?1 WHERE user_id = ?2 AND read_at IS NULL",
                params![now, user_id],
            )?;
            Ok(rows as u64)
        })
        .await
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn row_to_notification(row: &rusqlite::Row) -> rusqlite::Result<Notification> {
    Ok(Notification {
        id: row.get(0)?,
        user_id: row.get(1)?,
        event_id: row.get(2)?,
        notification_type: row.get(3)?,
        read_at: row.get(4)?,
        created_at: row.get(5)?,
    })
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    fn make_engine() -> NotificationEngine {
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
        NotificationEngine::new(db)
    }

    fn notif(user_id: &str, kind: &str) -> CreateNotification {
        CreateNotification {
            user_id: user_id.to_string(),
            event_id: None,
            notification_type: kind.to_string(),
        }
    }

    #[tokio::test]
    async fn test_create_notification() {
        let e = make_engine();
        let n = e.create(notif("u1", types::DOCUMENT_UPDATED)).await.unwrap();
        assert_eq!(n.user_id, "u1");
        assert_eq!(n.notification_type, types::DOCUMENT_UPDATED);
        assert!(n.read_at.is_none());
    }

    #[tokio::test]
    async fn test_list_unread() {
        let e = make_engine();
        e.create(notif("u1", types::DOCUMENT_UPDATED)).await.unwrap();
        e.create(notif("u1", types::COMMENT_CREATED)).await.unwrap();
        let list = e.list_for_user("u1", ListNotificationsQuery { read: Some(false), limit: None }).await.unwrap();
        assert_eq!(list.len(), 2);
    }

    #[tokio::test]
    async fn test_list_read_filter() {
        let e = make_engine();
        let n = e.create(notif("u1", types::DOCUMENT_UPDATED)).await.unwrap();
        e.create(notif("u1", types::COMMENT_CREATED)).await.unwrap();
        e.mark_read(&n.id, "u1").await.unwrap();

        let unread = e.list_for_user("u1", ListNotificationsQuery { read: Some(false), limit: None }).await.unwrap();
        assert_eq!(unread.len(), 1);

        let read = e.list_for_user("u1", ListNotificationsQuery { read: Some(true), limit: None }).await.unwrap();
        assert_eq!(read.len(), 1);
    }

    #[tokio::test]
    async fn test_mark_read() {
        let e = make_engine();
        let n = e.create(notif("u1", types::MENTION)).await.unwrap();
        let updated = e.mark_read(&n.id, "u1").await.unwrap();
        assert!(updated.read_at.is_some());
    }

    #[tokio::test]
    async fn test_mark_read_not_found() {
        let e = make_engine();
        let err = e.mark_read("no-such", "u1").await.unwrap_err();
        assert!(matches!(err, AppError::NotFound(_)));
    }

    #[tokio::test]
    async fn test_mark_all_read() {
        let e = make_engine();
        e.create(notif("u1", types::DOCUMENT_UPDATED)).await.unwrap();
        e.create(notif("u1", types::COMMENT_CREATED)).await.unwrap();
        let count = e.mark_all_read("u1").await.unwrap();
        assert_eq!(count, 2);
        let unread = e.list_for_user("u1", ListNotificationsQuery { read: Some(false), limit: None }).await.unwrap();
        assert!(unread.is_empty());
    }

    #[tokio::test]
    async fn test_notifications_isolated_per_user() {
        let e = make_engine();
        e.create(notif("u1", types::DOCUMENT_UPDATED)).await.unwrap();
        let list = e.list_for_user("u2", ListNotificationsQuery { read: None, limit: None }).await.unwrap();
        assert!(list.is_empty());
    }
}
