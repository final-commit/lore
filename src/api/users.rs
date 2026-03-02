use axum::{
    extract::{Query, State},
    Json,
};
use serde::{Deserialize, Serialize};

use crate::auth::AuthUser;
use crate::db::with_conn;
use crate::error::AppError;
use crate::state::AppState;

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
