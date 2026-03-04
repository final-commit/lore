use axum::{
    extract::{Query, State},
    Json,
};

use crate::auth::AuthUser;
use crate::error::AppError;
use crate::events::Event;
use crate::events::engine::ListEventsQuery;
use crate::state::AppState;

/// GET /api/events — paginated audit log (admin only)
pub async fn list_events(
    State(state): State<AppState>,
    user: AuthUser,
    Query(q): Query<ListEventsQuery>,
) -> Result<Json<Vec<Event>>, AppError> {
    if !user.is_admin() {
        return Err(AppError::Forbidden("admin required to view audit log".into()));
    }
    let events = state.events.list(q).await?;
    Ok(Json(events))
}
