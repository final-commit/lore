use chrono::Utc;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use crate::db::{with_conn, DbConn};
use crate::error::AppError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPreferences {
    pub user_id: String,
    pub theme: String,
    pub language: String,
    pub notification_email: bool,
    pub notification_web: bool,
    pub keyboard_shortcuts: bool,
    pub code_block_language: String,
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePreferences {
    pub theme: Option<String>,
    pub language: Option<String>,
    pub notification_email: Option<bool>,
    pub notification_web: Option<bool>,
    pub keyboard_shortcuts: Option<bool>,
    pub code_block_language: Option<String>,
}

#[derive(Clone)]
pub struct PreferencesEngine { db: DbConn }

impl PreferencesEngine {
    pub fn new(db: DbConn) -> Self { PreferencesEngine { db } }

    pub async fn get(&self, user_id: &str) -> Result<UserPreferences, AppError> {
        let db = self.db.clone();
        let uid = user_id.to_string();
        with_conn(&db, move |conn| {
            // Upsert default row if missing
            let now = Utc::now().to_rfc3339();
            conn.execute(
                "INSERT OR IGNORE INTO user_preferences (user_id, theme, language, notification_email, notification_web, keyboard_shortcuts, code_block_language, updated_at) VALUES (?1,'system','en',1,1,1,'auto',?2)",
                params![uid, now],
            )?;
            conn.query_row(
                "SELECT user_id, theme, language, notification_email, notification_web, keyboard_shortcuts, code_block_language, updated_at FROM user_preferences WHERE user_id=?1",
                params![uid],
                |r| Ok(UserPreferences {
                    user_id: r.get(0)?, theme: r.get(1)?, language: r.get(2)?,
                    notification_email: r.get::<_,i64>(3)? != 0,
                    notification_web: r.get::<_,i64>(4)? != 0,
                    keyboard_shortcuts: r.get::<_,i64>(5)? != 0,
                    code_block_language: r.get(6)?, updated_at: r.get(7)?,
                }),
            )
        }).await
    }

    pub async fn update(&self, user_id: &str, req: UpdatePreferences) -> Result<UserPreferences, AppError> {
        let db = self.db.clone();
        let uid = user_id.to_string();
        let now = Utc::now().to_rfc3339();
        // Ensure row exists first
        self.get(user_id).await?;
        let n = now.clone();
        with_conn(&db, move |conn| {
            if let Some(v) = &req.theme { conn.execute("UPDATE user_preferences SET theme=?1,updated_at=?2 WHERE user_id=?3", params![v,n,uid])?; }
            if let Some(v) = &req.language { conn.execute("UPDATE user_preferences SET language=?1,updated_at=?2 WHERE user_id=?3", params![v,n,uid])?; }
            if let Some(v) = req.notification_email { conn.execute("UPDATE user_preferences SET notification_email=?1,updated_at=?2 WHERE user_id=?3", params![v as i64,n,uid])?; }
            if let Some(v) = req.notification_web { conn.execute("UPDATE user_preferences SET notification_web=?1,updated_at=?2 WHERE user_id=?3", params![v as i64,n,uid])?; }
            if let Some(v) = req.keyboard_shortcuts { conn.execute("UPDATE user_preferences SET keyboard_shortcuts=?1,updated_at=?2 WHERE user_id=?3", params![v as i64,n,uid])?; }
            if let Some(v) = &req.code_block_language { conn.execute("UPDATE user_preferences SET code_block_language=?1,updated_at=?2 WHERE user_id=?3", params![v,n,uid])?; }
            conn.query_row(
                "SELECT user_id, theme, language, notification_email, notification_web, keyboard_shortcuts, code_block_language, updated_at FROM user_preferences WHERE user_id=?1",
                params![uid],
                |r| Ok(UserPreferences {
                    user_id: r.get(0)?, theme: r.get(1)?, language: r.get(2)?,
                    notification_email: r.get::<_,i64>(3)? != 0,
                    notification_web: r.get::<_,i64>(4)? != 0,
                    keyboard_shortcuts: r.get::<_,i64>(5)? != 0,
                    code_block_language: r.get(6)?, updated_at: r.get(7)?,
                }),
            )
        }).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    fn engine_with_user() -> (PreferencesEngine, String) {
        let db = db::open(":memory:").unwrap();
        { let conn = db.lock().unwrap();
          conn.execute("INSERT INTO users (id,email,name,password_hash,role,created_at,updated_at) VALUES ('u1','a@b.com','A','h','editor','2024-01-01','2024-01-01')", []).unwrap(); }
        (PreferencesEngine::new(db), "u1".into())
    }

    #[tokio::test]
    async fn test_get_creates_defaults() {
        let (e, uid) = engine_with_user();
        let p = e.get(&uid).await.unwrap();
        assert_eq!(p.theme, "system");
        assert!(p.keyboard_shortcuts);
    }

    #[tokio::test]
    async fn test_update_theme() {
        let (e, uid) = engine_with_user();
        let p = e.update(&uid, UpdatePreferences { theme: Some("dark".into()), language: None, notification_email: None, notification_web: None, keyboard_shortcuts: None, code_block_language: None }).await.unwrap();
        assert_eq!(p.theme, "dark");
    }
}
