use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;

use crate::auth::AuthUser;
use crate::error::AppError;
use crate::notifications::Notification;
use crate::notifications::engine::ListNotificationsQuery;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct ListNotifQuery {
    pub read: Option<bool>,
    pub limit: Option<i64>,
}

/// GET /api/notifications?read=&limit=
pub async fn list_notifications(
    State(state): State<AppState>,
    user: AuthUser,
    Query(q): Query<ListNotifQuery>,
) -> Result<Json<Vec<Notification>>, AppError> {
    let notifications = state.notifications.list_for_user(
        &user.id,
        ListNotificationsQuery {
            read: q.read,
            limit: q.limit,
        },
    ).await?;
    Ok(Json(notifications))
}

/// POST /api/notifications/{id}/read
pub async fn mark_notification_read(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<Notification>, AppError> {
    let notif = state.notifications.mark_read(&id, &user.id).await?;
    Ok(Json(notif))
}

/// POST /api/notifications/read-all
pub async fn mark_all_notifications_read(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<(StatusCode, Json<serde_json::Value>), AppError> {
    let count = state.notifications.mark_all_read(&user.id).await?;
    Ok((StatusCode::OK, Json(serde_json::json!({ "marked_read": count }))))
}
