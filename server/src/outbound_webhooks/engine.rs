use chrono::Utc;
use hmac::{Hmac, Mac};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use uuid::Uuid;

use crate::db::{with_conn, DbConn};
use crate::error::AppError;

// ── Data types ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookSubscription {
    pub id: String,
    pub url: String,
    pub secret: Option<String>,
    pub events: String,
    pub enabled: bool,
    pub created_by: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CreateWebhookSubscription {
    pub url: String,
    pub secret: Option<String>,
    pub events: Option<String>,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateWebhookSubscription {
    pub url: Option<String>,
    pub secret: Option<String>,
    pub events: Option<String>,
    pub enabled: Option<bool>,
}

// ── Engine ────────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct OutboundWebhookEngine {
    db: DbConn,
    http: reqwest::Client,
}

impl OutboundWebhookEngine {
    pub fn new(db: DbConn) -> Self {
        OutboundWebhookEngine {
            db,
            http: reqwest::Client::new(),
        }
    }

    pub async fn create(&self, req: CreateWebhookSubscription) -> Result<WebhookSubscription, AppError> {
        let db = self.db.clone();
        with_conn(&db, move |conn| {
            let id = Uuid::now_v7().to_string();
            let now = Utc::now().to_rfc3339();
            let events = req.events.unwrap_or_else(|| "*".to_string());
            conn.execute(
                "INSERT INTO webhook_subscriptions (id, url, secret, events, enabled, created_by, created_at, updated_at)
                 VALUES (?1,?2,?3,?4,1,?5,?6,?6)",
                params![id, req.url, req.secret, events, req.created_by, now],
            )?;
            conn.query_row(
                "SELECT id, url, secret, events, enabled, created_by, created_at, updated_at
                 FROM webhook_subscriptions WHERE id = ?1",
                params![id],
                row_to_webhook,
            )
        })
        .await
    }

    pub async fn list(&self) -> Result<Vec<WebhookSubscription>, AppError> {
        let db = self.db.clone();
        with_conn(&db, move |conn| {
            let mut stmt = conn.prepare(
                "SELECT id, url, secret, events, enabled, created_by, created_at, updated_at
                 FROM webhook_subscriptions ORDER BY created_at DESC",
            )?;
            let rows = stmt.query_map([], row_to_webhook)?;
            rows.collect()
        })
        .await
    }

    pub async fn update(&self, id: &str, req: UpdateWebhookSubscription) -> Result<WebhookSubscription, AppError> {
        let db = self.db.clone();
        let id = id.to_string();
        let id2 = id.clone();
        with_conn(&db, move |conn| {
            let now = Utc::now().to_rfc3339();
            let mut sets = vec!["updated_at = ?1".to_string()];
            let mut idx = 2i32;

            if req.url.is_some() { sets.push(format!("url = ?{idx}")); idx += 1; }
            if req.secret.is_some() { sets.push(format!("secret = ?{idx}")); idx += 1; }
            if req.events.is_some() { sets.push(format!("events = ?{idx}")); idx += 1; }
            if req.enabled.is_some() { sets.push(format!("enabled = ?{idx}")); idx += 1; }
            let id_idx = idx;

            let sql = format!("UPDATE webhook_subscriptions SET {} WHERE id = ?{id_idx}", sets.join(", "));
            let mut param_values: Vec<rusqlite::types::Value> = vec![
                rusqlite::types::Value::Text(now),
            ];
            if let Some(v) = req.url { param_values.push(rusqlite::types::Value::Text(v)); }
            if let Some(v) = req.secret { param_values.push(rusqlite::types::Value::Text(v)); }
            if let Some(v) = req.events { param_values.push(rusqlite::types::Value::Text(v)); }
            if let Some(v) = req.enabled { param_values.push(rusqlite::types::Value::Integer(v as i64)); }
            param_values.push(rusqlite::types::Value::Text(id.clone()));

            let rows = conn.execute(&sql, rusqlite::params_from_iter(param_values.iter()))?;
            if rows == 0 { return Err(rusqlite::Error::QueryReturnedNoRows); }

            conn.query_row(
                "SELECT id, url, secret, events, enabled, created_by, created_at, updated_at
                 FROM webhook_subscriptions WHERE id = ?1",
                params![id],
                row_to_webhook,
            )
        })
        .await
        .map_err(|e| match e {
            AppError::Db(rusqlite::Error::QueryReturnedNoRows) => {
                AppError::NotFound(format!("webhook subscription {id2} not found"))
            }
            other => other,
        })
    }

    pub async fn delete(&self, id: &str) -> Result<(), AppError> {
        let db = self.db.clone();
        let id = id.to_string();
        let id2 = id.clone();
        with_conn(&db, move |conn| {
            let rows = conn.execute("DELETE FROM webhook_subscriptions WHERE id = ?1", params![id])?;
            if rows == 0 { return Err(rusqlite::Error::QueryReturnedNoRows); }
            Ok(())
        })
        .await
        .map_err(|e| match e {
            AppError::Db(rusqlite::Error::QueryReturnedNoRows) => {
                AppError::NotFound(format!("webhook subscription {id2} not found"))
            }
            other => other,
        })
    }

    /// Dispatch an event payload to all matching enabled webhook subscriptions.
    /// Fire-and-forget — errors are logged but not propagated.
    pub fn dispatch(&self, event_name: &str, payload: serde_json::Value) {
        let db = self.db.clone();
        let http = self.http.clone();
        let event_name = event_name.to_string();

        tokio::spawn(async move {
            // Fetch matching webhooks from DB.
            let webhooks = with_conn(&db, move |conn| {
                let mut stmt = conn.prepare(
                    "SELECT id, url, secret, events, enabled, created_by, created_at, updated_at
                     FROM webhook_subscriptions WHERE enabled = 1",
                )?;
                let rows = stmt.query_map([], row_to_webhook)?;
                rows.collect::<rusqlite::Result<Vec<WebhookSubscription>>>()
            })
            .await;

            let webhooks = match webhooks {
                Ok(w) => w,
                Err(e) => {
                    tracing::error!(error = %e, "failed to load webhook subscriptions");
                    return;
                }
            };

            for wh in webhooks {
                // Check if this webhook matches the event.
                if wh.events != "*" {
                    let matched = wh.events.split(',').any(|e| e.trim() == event_name);
                    if !matched {
                        continue;
                    }
                }

                let mut req = http.post(&wh.url).json(&payload);

                // Add HMAC signature if secret is configured.
                if let Some(ref secret) = wh.secret {
                    if let Ok(body) = serde_json::to_string(&payload) {
                        let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes())
                            .expect("HMAC accepts any key length");
                        mac.update(body.as_bytes());
                        let sig = hex::encode(mac.finalize().into_bytes());
                        req = req.header("X-Lore-Signature", format!("sha256={sig}"));
                    }
                }

                req = req.header("X-Lore-Event", &event_name);

                match req.send().await {
                    Ok(resp) => {
                        tracing::debug!(url = %wh.url, status = %resp.status(), "webhook delivered");
                    }
                    Err(e) => {
                        tracing::warn!(url = %wh.url, error = %e, "webhook delivery failed");
                    }
                }
            }
        });
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn row_to_webhook(row: &rusqlite::Row) -> rusqlite::Result<WebhookSubscription> {
    Ok(WebhookSubscription {
        id: row.get(0)?,
        url: row.get(1)?,
        secret: row.get(2)?,
        events: row.get(3)?,
        enabled: row.get::<_, i64>(4)? != 0,
        created_by: row.get(5)?,
        created_at: row.get(6)?,
        updated_at: row.get(7)?,
    })
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    fn make_engine() -> OutboundWebhookEngine {
        let db = db::open(":memory:").unwrap();
        {
            let conn = db.lock().unwrap();
            conn.execute(
                "INSERT INTO users (id,email,name,password_hash,role,created_at,updated_at)
                 VALUES ('u1','a@b.com','Alice','hash','admin','2024-01-01T00:00:00Z','2024-01-01T00:00:00Z')",
                [],
            ).unwrap();
        }
        OutboundWebhookEngine::new(db)
    }

    fn req(url: &str) -> CreateWebhookSubscription {
        CreateWebhookSubscription {
            url: url.to_string(),
            secret: None,
            events: None,
            created_by: Some("u1".to_string()),
        }
    }

    #[tokio::test]
    async fn test_create_webhook() {
        let e = make_engine();
        let w = e.create(req("https://example.com/hook")).await.unwrap();
        assert_eq!(w.url, "https://example.com/hook");
        assert_eq!(w.events, "*");
        assert!(w.enabled);
    }

    #[tokio::test]
    async fn test_list_webhooks() {
        let e = make_engine();
        e.create(req("https://a.com/hook")).await.unwrap();
        e.create(req("https://b.com/hook")).await.unwrap();
        let list = e.list().await.unwrap();
        assert_eq!(list.len(), 2);
    }

    #[tokio::test]
    async fn test_update_webhook() {
        let e = make_engine();
        let w = e.create(req("https://a.com/hook")).await.unwrap();
        let updated = e.update(&w.id, UpdateWebhookSubscription {
            url: Some("https://b.com/hook".into()),
            secret: Some("mysecret".into()),
            events: Some("documents.create,documents.update".into()),
            enabled: Some(false),
        }).await.unwrap();
        assert_eq!(updated.url, "https://b.com/hook");
        assert_eq!(updated.events, "documents.create,documents.update");
        assert!(!updated.enabled);
    }

    #[tokio::test]
    async fn test_delete_webhook() {
        let e = make_engine();
        let w = e.create(req("https://a.com/hook")).await.unwrap();
        e.delete(&w.id).await.unwrap();
        let list = e.list().await.unwrap();
        assert!(list.is_empty());
    }

    #[tokio::test]
    async fn test_delete_not_found() {
        let e = make_engine();
        let err = e.delete("no-such").await.unwrap_err();
        assert!(matches!(err, AppError::NotFound(_)));
    }

    #[tokio::test]
    async fn test_create_with_secret_and_events() {
        let e = make_engine();
        let w = e.create(CreateWebhookSubscription {
            url: "https://example.com/hook".into(),
            secret: Some("s3cr3t".into()),
            events: Some("documents.create".into()),
            created_by: Some("u1".into()),
        }).await.unwrap();
        assert_eq!(w.secret, Some("s3cr3t".into()));
        assert_eq!(w.events, "documents.create");
    }
}
