use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};

use crate::auth::AuthUser;
use crate::db::{with_conn, DbConn};
use crate::error::AppError;
use crate::state::AppState;

// ── Search (existing) ─────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct UserSearchQuery {
    pub q: String,
}

#[derive(Debug, Serialize)]
pub struct UserSearchResult {
    pub id: String,
    pub name: String,
    pub email: String,
}

/// GET /api/users/search?q=
/// Search users by name or email prefix — used for @mention autocomplete.
pub async fn search_users(
    State(state): State<AppState>,
    _user: AuthUser,
    Query(q): Query<UserSearchQuery>,
) -> Result<Json<Vec<UserSearchResult>>, AppError> {
    let query = q.q.trim().to_string();
    if query.is_empty() {
        return Ok(Json(vec![]));
    }
    let db = state.db.clone();
    let pattern = format!("%{}%", query.to_lowercase());
    let results = with_conn(&db, move |conn| {
        let mut stmt = conn.prepare(
            "SELECT id, name, email FROM users
             WHERE lower(name) LIKE ?1 OR lower(email) LIKE ?1
             ORDER BY name ASC
             LIMIT 20",
        )?;
        let rows = stmt.query_map(rusqlite::params![pattern], |row| {
            Ok(UserSearchResult {
                id: row.get(0)?,
                name: row.get(1)?,
                email: row.get(2)?,
            })
        })?;
        rows.collect()
    })
    .await?;
    Ok(Json(results))
}

// ── User management structs ───────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct UserProfile {
    pub id: String,
    pub name: String,
    pub email: String,
    pub role: String,
    pub avatar_url: Option<String>,
    pub suspended_at: Option<String>,
    pub last_active_at: Option<String>,
    pub created_at: String,
}

#[derive(Deserialize)]
pub struct UpdateUserBody {
    pub name: Option<String>,
    pub avatar_url: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateRoleBody {
    pub role: String,
}

#[derive(Deserialize)]
pub struct InviteUserBody {
    pub email: String,
    pub name: String,
    pub role: Option<String>,
}

#[derive(Deserialize)]
pub struct ListUsersQuery {
    pub role: Option<String>,
    pub filter: Option<String>,
    pub query: Option<String>,
    pub offset: Option<i64>,
    pub limit: Option<i64>,
}

// ── Private helper ────────────────────────────────────────────────────────────

async fn fetch_profile(db: &DbConn, id: String) -> Result<UserProfile, AppError> {
    with_conn(db, move |conn| {
        conn.query_row(
            "SELECT id, name, email, role, avatar_url, suspended_at, last_active_at, created_at
             FROM users WHERE id = ?1",
            rusqlite::params![id],
            |row| {
                Ok(UserProfile {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    email: row.get(2)?,
                    role: row.get(3)?,
                    avatar_url: row.get(4)?,
                    suspended_at: row.get(5)?,
                    last_active_at: row.get(6)?,
                    created_at: row.get(7)?,
                })
            },
        )
    })
    .await
    .map_err(|e| match e {
        AppError::Db(rusqlite::Error::QueryReturnedNoRows) => {
            AppError::NotFound("user not found".into())
        }
        other => other,
    })
}

// ── Handlers ──────────────────────────────────────────────────────────────────

/// GET /api/users
pub async fn list_users(
    State(state): State<AppState>,
    user: AuthUser,
    Query(params): Query<ListUsersQuery>,
) -> Result<Json<Vec<UserProfile>>, AppError> {
    let db = state.db.clone();
    let is_admin = user.is_admin();
    let limit = params.limit.unwrap_or(25);
    let offset = params.offset.unwrap_or(0);
    let role_filter = params.role.clone();
    let filter = params.filter.clone();
    let query_str = params.query.clone();

    let results = with_conn(&db, move |conn| {
        let mut conditions: Vec<String> = Vec::new();
        let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        // Non-admin only sees non-suspended users
        if !is_admin {
            conditions.push("suspended_at IS NULL".to_string());
        }

        if let Some(ref r) = role_filter {
            param_values.push(Box::new(r.clone()));
            conditions.push(format!("role = ?{}", param_values.len()));
        }

        match filter.as_deref() {
            Some("suspended") => conditions.push("suspended_at IS NOT NULL".to_string()),
            Some("invited") => conditions.push("invite_token IS NOT NULL".to_string()),
            Some("active") => {
                conditions.push("suspended_at IS NULL".to_string());
                conditions.push("invite_token IS NULL".to_string());
            }
            _ => {}
        }

        if let Some(ref q) = query_str {
            let pattern = format!("%{}%", q.to_lowercase());
            param_values.push(Box::new(pattern));
            conditions.push(format!(
                "(lower(name) LIKE ?{0} OR lower(email) LIKE ?{0})",
                param_values.len()
            ));
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };

        param_values.push(Box::new(limit));
        let limit_idx = param_values.len();
        param_values.push(Box::new(offset));
        let offset_idx = param_values.len();

        let sql = format!(
            "SELECT id, name, email, role, avatar_url, suspended_at, last_active_at, created_at
             FROM users
             {where_clause}
             ORDER BY name ASC
             LIMIT ?{limit_idx} OFFSET ?{offset_idx}"
        );

        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(
            rusqlite::params_from_iter(param_values.iter().map(|p| p.as_ref())),
            |row| {
                Ok(UserProfile {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    email: row.get(2)?,
                    role: row.get(3)?,
                    avatar_url: row.get(4)?,
                    suspended_at: row.get(5)?,
                    last_active_at: row.get(6)?,
                    created_at: row.get(7)?,
                })
            },
        )?;
        rows.collect()
    })
    .await?;

    Ok(Json(results))
}

/// GET /api/users/{id}
pub async fn get_user(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<UserProfile>, AppError> {
    let profile = fetch_profile(&state.db, id).await?;
    Ok(Json(profile))
}

/// PUT /api/users/{id}
pub async fn update_user(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<String>,
    Json(body): Json<UpdateUserBody>,
) -> Result<Json<UserProfile>, AppError> {
    // Users can update themselves; admins can update anyone
    if !user.is_admin() && user.id != id {
        return Err(AppError::Forbidden("cannot update another user's profile".into()));
    }

    let db = state.db.clone();
    let id_clone = id.clone();
    let now = chrono::Utc::now().to_rfc3339();
    with_conn(&db, move |conn| {
        if let Some(ref name) = body.name {
            conn.execute(
                "UPDATE users SET name = ?1, updated_at = ?2 WHERE id = ?3",
                rusqlite::params![name, now, id_clone],
            )?;
        }
        if let Some(ref avatar) = body.avatar_url {
            conn.execute(
                "UPDATE users SET avatar_url = ?1, updated_at = ?2 WHERE id = ?3",
                rusqlite::params![avatar, now, id_clone],
            )?;
        }
        Ok(())
    })
    .await?;

    let profile = fetch_profile(&state.db, id).await?;
    Ok(Json(profile))
}

/// PUT /api/users/{id}/role
pub async fn update_user_role(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<String>,
    Json(body): Json<UpdateRoleBody>,
) -> Result<Json<UserProfile>, AppError> {
    if !user.is_admin() {
        return Err(AppError::BadRequest("admin only".into()));
    }
    let valid_roles = ["admin", "member", "viewer", "editor"];
    if !valid_roles.contains(&body.role.as_str()) {
        return Err(AppError::BadRequest(format!(
            "invalid role '{}', must be one of: admin, member, viewer, editor",
            body.role
        )));
    }

    let db = state.db.clone();
    let id_clone = id.clone();
    let role = body.role.clone();
    let now = chrono::Utc::now().to_rfc3339();
    with_conn(&db, move |conn| {
        let rows = conn.execute(
            "UPDATE users SET role = ?1, updated_at = ?2 WHERE id = ?3",
            rusqlite::params![role, now, id_clone],
        )?;
        if rows == 0 {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        Ok(())
    })
    .await
    .map_err(|e| match e {
        AppError::Db(rusqlite::Error::QueryReturnedNoRows) => AppError::NotFound("user not found".into()),
        other => other,
    })?;

    let profile = fetch_profile(&state.db, id).await?;
    Ok(Json(profile))
}

/// POST /api/users/{id}/suspend
pub async fn suspend_user(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<UserProfile>, AppError> {
    if !user.is_admin() {
        return Err(AppError::Forbidden("admin only".into()));
    }
    if user.id == id {
        return Err(AppError::BadRequest("cannot suspend yourself".into()));
    }

    let db = state.db.clone();
    let id_clone = id.clone();
    let now = chrono::Utc::now().to_rfc3339();
    with_conn(&db, move |conn| {
        let rows = conn.execute(
            "UPDATE users SET suspended_at = ?1, updated_at = ?1 WHERE id = ?2",
            rusqlite::params![now, id_clone],
        )?;
        if rows == 0 {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        Ok(())
    })
    .await
    .map_err(|e| match e {
        AppError::Db(rusqlite::Error::QueryReturnedNoRows) => AppError::NotFound("user not found".into()),
        other => other,
    })?;

    let profile = fetch_profile(&state.db, id).await?;
    Ok(Json(profile))
}

/// POST /api/users/{id}/activate
pub async fn activate_user(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<UserProfile>, AppError> {
    if !user.is_admin() {
        return Err(AppError::Forbidden("admin only".into()));
    }

    let db = state.db.clone();
    let id_clone = id.clone();
    let now = chrono::Utc::now().to_rfc3339();
    with_conn(&db, move |conn| {
        let rows = conn.execute(
            "UPDATE users SET suspended_at = NULL, updated_at = ?1 WHERE id = ?2",
            rusqlite::params![now, id_clone],
        )?;
        if rows == 0 {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        Ok(())
    })
    .await
    .map_err(|e| match e {
        AppError::Db(rusqlite::Error::QueryReturnedNoRows) => AppError::NotFound("user not found".into()),
        other => other,
    })?;

    let profile = fetch_profile(&state.db, id).await?;
    Ok(Json(profile))
}

#[derive(Serialize)]
pub struct InviteResponse {
    pub id: String,
    pub email: String,
    pub name: String,
    pub invite_token: String,
}

/// POST /api/users/invite
pub async fn invite_user(
    State(state): State<AppState>,
    user: AuthUser,
    Json(body): Json<InviteUserBody>,
) -> Result<(StatusCode, Json<InviteResponse>), AppError> {
    if !user.is_admin() {
        return Err(AppError::Forbidden("admin only".into()));
    }
    if body.email.trim().is_empty() {
        return Err(AppError::BadRequest("email is required".into()));
    }
    if body.name.trim().is_empty() {
        return Err(AppError::BadRequest("name is required".into()));
    }

    let id = uuid::Uuid::now_v7().to_string();
    let invite_token = uuid::Uuid::now_v7().to_string();
    let email = body.email.trim().to_lowercase();
    let name = body.name.trim().to_string();
    let role = body.role.unwrap_or_else(|| "member".to_string());
    let now = chrono::Utc::now().to_rfc3339();

    let id_c = id.clone();
    let email_c = email.clone();
    let name_c = name.clone();
    let token_c = invite_token.clone();

    with_conn(&state.db, move |conn| {
        conn.execute(
            "INSERT INTO users (id, email, name, password_hash, role, invite_token, created_at, updated_at)
             VALUES (?1, ?2, ?3, 'pending', ?4, ?5, ?6, ?6)",
            rusqlite::params![id_c, email_c, name_c, role, token_c, now],
        )?;
        Ok(())
    })
    .await?;

    Ok((
        StatusCode::CREATED,
        Json(InviteResponse {
            id,
            email,
            name,
            invite_token,
        }),
    ))
}

/// DELETE /api/users/{id}
pub async fn delete_user(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<String>,
) -> Result<StatusCode, AppError> {
    if !user.is_admin() {
        return Err(AppError::Forbidden("admin only".into()));
    }
    if user.id == id {
        return Err(AppError::BadRequest("cannot delete yourself".into()));
    }

    with_conn(&state.db, move |conn| {
        let rows = conn.execute("DELETE FROM users WHERE id = ?1", rusqlite::params![id])?;
        if rows == 0 {
            return Err(rusqlite::Error::QueryReturnedNoRows);
        }
        Ok(())
    })
    .await
    .map_err(|e| match e {
        AppError::Db(rusqlite::Error::QueryReturnedNoRows) => AppError::NotFound("user not found".into()),
        other => other,
    })?;

    Ok(StatusCode::NO_CONTENT)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use axum::extract::{Path, Query, State};
    use axum::Json;
    use rusqlite::Connection;
    use std::sync::{Arc, Mutex};

    use crate::auth::AuthUser;
    use crate::db::schema::apply_migrations;
    use crate::error::AppError;
    use crate::state::AppState;

    use super::*;

    fn make_admin() -> AuthUser {
        AuthUser {
            id: "admin-1".to_string(),
            email: "admin@example.com".to_string(),
            role: "admin".to_string(),
        }
    }

    fn make_member(id: &str) -> AuthUser {
        AuthUser {
            id: id.to_string(),
            email: format!("{id}@example.com"),
            role: "member".to_string(),
        }
    }

    fn insert_user(conn: &Connection, id: &str, email: &str, name: &str, role: &str) {
        conn.execute(
            "INSERT INTO users (id, email, name, password_hash, role, created_at, updated_at)
             VALUES (?1, ?2, ?3, 'hash', ?4, datetime('now'), datetime('now'))",
            rusqlite::params![id, email, name, role],
        )
        .unwrap();
    }

    async fn make_state() -> AppState {
        use crate::git::queue::GitQueue;
        use crate::git::GitEngine;
        use crate::search::SearchEngine;

        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
        apply_migrations(&conn).unwrap();

        let db = Arc::new(Mutex::new(conn));
        let tmp = tempfile::TempDir::new().unwrap();
        let queue = GitQueue::new();
        let git = GitEngine::init(tmp.path().to_path_buf(), queue).unwrap();
        let search = SearchEngine::open_in_ram().unwrap();

        AppState::new_for_test(db, git, search, tmp)
    }

    #[tokio::test]
    async fn test_list_users_empty() {
        let state = make_state().await;
        let result = list_users(
            State(state),
            make_admin(),
            Query(ListUsersQuery {
                role: None,
                filter: None,
                query: None,
                offset: None,
                limit: None,
            }),
        )
        .await
        .unwrap();
        assert!(result.0.is_empty());
    }

    #[tokio::test]
    async fn test_list_users_returns_users() {
        let state = make_state().await;
        {
            let db = state.db.lock().unwrap();
            insert_user(&db, "u1", "alice@test.com", "Alice", "editor");
            insert_user(&db, "u2", "bob@test.com", "Bob", "member");
        }
        let result = list_users(
            State(state),
            make_admin(),
            Query(ListUsersQuery {
                role: None,
                filter: None,
                query: None,
                offset: None,
                limit: None,
            }),
        )
        .await
        .unwrap();
        assert_eq!(result.0.len(), 2);
    }

    #[tokio::test]
    async fn test_get_user_not_found() {
        let state = make_state().await;
        let err = get_user(State(state), make_admin(), Path("no-such-id".to_string()))
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::NotFound(_)));
    }

    #[tokio::test]
    async fn test_get_user_found() {
        let state = make_state().await;
        {
            let db = state.db.lock().unwrap();
            insert_user(&db, "u1", "charlie@test.com", "Charlie", "editor");
        }
        let result = get_user(State(state), make_admin(), Path("u1".to_string()))
            .await
            .unwrap();
        assert_eq!(result.0.name, "Charlie");
        assert_eq!(result.0.role, "editor");
    }

    #[tokio::test]
    async fn test_invite_user_creates_record() {
        let state = make_state().await;
        let (status, Json(resp)) = invite_user(
            State(state.clone()),
            make_admin(),
            Json(InviteUserBody {
                email: "newbie@example.com".to_string(),
                name: "Newbie".to_string(),
                role: None,
            }),
        )
        .await
        .unwrap();
        assert_eq!(status, axum::http::StatusCode::CREATED);
        assert_eq!(resp.email, "newbie@example.com");
        assert!(!resp.invite_token.is_empty());

        // verify in DB
        let db = state.db.lock().unwrap();
        let token: String = db
            .query_row(
                "SELECT invite_token FROM users WHERE id = ?1",
                rusqlite::params![resp.id],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(token, resp.invite_token);
    }

    #[tokio::test]
    async fn test_suspend_and_activate_user() {
        let state = make_state().await;
        {
            let db = state.db.lock().unwrap();
            insert_user(&db, "target", "target@test.com", "Target", "member");
        }
        // Suspend
        let _ = suspend_user(State(state.clone()), make_admin(), Path("target".to_string()))
            .await
            .unwrap();
        let profile = fetch_profile(&state.db, "target".to_string()).await.unwrap();
        assert!(profile.suspended_at.is_some());

        // Activate
        let _ = activate_user(State(state.clone()), make_admin(), Path("target".to_string()))
            .await
            .unwrap();
        let profile2 = fetch_profile(&state.db, "target".to_string()).await.unwrap();
        assert!(profile2.suspended_at.is_none());
    }

    #[tokio::test]
    async fn test_suspend_self_rejected() {
        let state = make_state().await;
        let err = suspend_user(State(state), make_admin(), Path("admin-1".to_string()))
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::BadRequest(_)));
    }

    #[tokio::test]
    async fn test_delete_user() {
        let state = make_state().await;
        {
            let db = state.db.lock().unwrap();
            insert_user(&db, "del-me", "del@test.com", "Del", "member");
        }
        let status = delete_user(State(state.clone()), make_admin(), Path("del-me".to_string()))
            .await
            .unwrap();
        assert_eq!(status, axum::http::StatusCode::NO_CONTENT);

        let err = fetch_profile(&state.db, "del-me".to_string()).await.unwrap_err();
        assert!(matches!(err, AppError::NotFound(_)));
    }

    #[tokio::test]
    async fn test_delete_self_rejected() {
        let state = make_state().await;
        let err = delete_user(State(state), make_admin(), Path("admin-1".to_string()))
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::BadRequest(_)));
    }

    #[tokio::test]
    async fn test_update_role_non_admin_rejected() {
        let state = make_state().await;
        {
            let db = state.db.lock().unwrap();
            insert_user(&db, "u1", "u1@test.com", "U1", "member");
        }
        let err = update_user_role(
            State(state),
            make_member("other"),
            Path("u1".to_string()),
            Json(UpdateRoleBody {
                role: "admin".to_string(),
            }),
        )
        .await
        .unwrap_err();
        assert!(matches!(err, AppError::BadRequest(_)));
    }

    #[tokio::test]
    async fn test_update_role_invalid_role() {
        let state = make_state().await;
        {
            let db = state.db.lock().unwrap();
            insert_user(&db, "u1", "u1@test.com", "U1", "member");
        }
        let err = update_user_role(
            State(state),
            make_admin(),
            Path("u1".to_string()),
            Json(UpdateRoleBody {
                role: "superuser".to_string(),
            }),
        )
        .await
        .unwrap_err();
        assert!(matches!(err, AppError::BadRequest(_)));
    }

    #[tokio::test]
    async fn test_non_admin_cannot_see_suspended() {
        let state = make_state().await;
        {
            let db = state.db.lock().unwrap();
            insert_user(&db, "active1", "active@test.com", "Active", "member");
            db.execute(
                "INSERT INTO users (id, email, name, password_hash, role, suspended_at, created_at, updated_at)
                 VALUES ('susp1', 'susp@test.com', 'Suspended', 'hash', 'member', datetime('now'), datetime('now'), datetime('now'))",
                [],
            ).unwrap();
        }
        let result = list_users(
            State(state),
            make_member("someone"),
            Query(ListUsersQuery {
                role: None,
                filter: None,
                query: None,
                offset: None,
                limit: None,
            }),
        )
        .await
        .unwrap();
        // Suspended user should not appear for non-admin
        assert!(result.0.iter().all(|u| u.suspended_at.is_none()));
    }
}
