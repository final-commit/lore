use chrono::Utc;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use crate::db::{with_conn, DbConn};
use crate::error::AppError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamSettings {
    pub name: String,
    pub description: Option<String>,
    pub logo_url: Option<String>,
    pub allow_signups: bool,
    pub default_role: String,
    pub sharing_enabled: bool,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateSettings {
    pub name: Option<String>,
    pub description: Option<String>,
    pub logo_url: Option<String>,
    pub allow_signups: Option<bool>,
    pub default_role: Option<String>,
    pub sharing_enabled: Option<bool>,
}

#[derive(Clone)]
pub struct SettingsEngine { db: DbConn }

impl SettingsEngine {
    pub fn new(db: DbConn) -> Self { SettingsEngine { db } }

    pub async fn get(&self) -> Result<TeamSettings, AppError> {
        let db = self.db.clone();
        with_conn(&db, |conn| {
            conn.query_row(
                "SELECT name, description, logo_url, allow_signups, default_role, sharing_enabled, updated_at FROM team_settings WHERE id=1",
                [],
                |r| Ok(TeamSettings {
                    name: r.get(0)?,
                    description: r.get(1)?,
                    logo_url: r.get(2)?,
                    allow_signups: r.get::<_,i64>(3)? != 0,
                    default_role: r.get(4)?,
                    sharing_enabled: r.get::<_,i64>(5)? != 0,
                    updated_at: r.get(6)?,
                }),
            )
        }).await
    }

    pub async fn update(&self, req: UpdateSettings) -> Result<TeamSettings, AppError> {
        let db = self.db.clone();
        let now = Utc::now().to_rfc3339();
        let n = now.clone();
        with_conn(&db, move |conn| {
            if let Some(v) = &req.name { conn.execute("UPDATE team_settings SET name=?1,updated_at=?2 WHERE id=1", params![v,n])?; }
            if let Some(v) = &req.description { conn.execute("UPDATE team_settings SET description=?1,updated_at=?2 WHERE id=1", params![v,n])?; }
            if let Some(v) = &req.logo_url { conn.execute("UPDATE team_settings SET logo_url=?1,updated_at=?2 WHERE id=1", params![v,n])?; }
            if let Some(v) = req.allow_signups { conn.execute("UPDATE team_settings SET allow_signups=?1,updated_at=?2 WHERE id=1", params![v as i64,n])?; }
            if let Some(v) = &req.default_role { conn.execute("UPDATE team_settings SET default_role=?1,updated_at=?2 WHERE id=1", params![v,n])?; }
            if let Some(v) = req.sharing_enabled { conn.execute("UPDATE team_settings SET sharing_enabled=?1,updated_at=?2 WHERE id=1", params![v as i64,n])?; }
            conn.query_row(
                "SELECT name, description, logo_url, allow_signups, default_role, sharing_enabled, updated_at FROM team_settings WHERE id=1",
                [], |r| Ok(TeamSettings {
                    name: r.get(0)?, description: r.get(1)?, logo_url: r.get(2)?,
                    allow_signups: r.get::<_,i64>(3)? != 0, default_role: r.get(4)?,
                    sharing_enabled: r.get::<_,i64>(5)? != 0, updated_at: r.get(6)?,
                }),
            )
        }).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    fn engine() -> SettingsEngine { SettingsEngine::new(db::open(":memory:").unwrap()) }

    #[tokio::test]
    async fn test_get_defaults() {
        let e = engine();
        let s = e.get().await.unwrap();
        assert_eq!(s.name, "Forge");
        assert!(s.allow_signups);
        assert!(s.sharing_enabled);
    }

    #[tokio::test]
    async fn test_update_name() {
        let e = engine();
        let s = e.update(UpdateSettings { name: Some("Acme Docs".into()), description: None, logo_url: None, allow_signups: None, default_role: None, sharing_enabled: None }).await.unwrap();
        assert_eq!(s.name, "Acme Docs");
    }

    #[tokio::test]
    async fn test_update_signups() {
        let e = engine();
        let s = e.update(UpdateSettings { allow_signups: Some(false), name: None, description: None, logo_url: None, default_role: None, sharing_enabled: None }).await.unwrap();
        assert!(!s.allow_signups);
    }
}
