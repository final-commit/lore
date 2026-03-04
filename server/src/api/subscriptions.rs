use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;

use crate::auth::AuthUser;
use crate::error::{validate_path, AppError};
use crate::subscriptions::Subscription;
use crate::subscriptions::engine::CreateSubscription;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct ListSubscriptionsQuery {
    pub doc_path: Option<String>,
}

#[derive(Deserialize)]
pub struct CreateSubscriptionReq {
    pub doc_path: String,
    pub event: Option<String>,
}

/// POST /api/subscriptions
pub async fn create_subscription(
    State(state): State<AppState>,
    user: AuthUser,
    Json(req): Json<CreateSubscriptionReq>,
) -> Result<(StatusCode, Json<Subscription>), AppError> {
    validate_path(&req.doc_path)?;
    let sub = state.subscriptions.subscribe(CreateSubscription {
        user_id: user.id.clone(),
        doc_path: req.doc_path,
        event: req.event,
    }).await?;
    Ok((StatusCode::CREATED, Json(sub)))
}

/// GET /api/subscriptions?doc_path=
pub async fn list_subscriptions(
    State(state): State<AppState>,
    user: AuthUser,
    Query(q): Query<ListSubscriptionsQuery>,
) -> Result<Json<Vec<Subscription>>, AppError> {
    let subs = state.subscriptions
        .list_for_user(&user.id, q.doc_path.as_deref())
        .await?;
    Ok(Json(subs))
}

/// DELETE /api/subscriptions/{id}
pub async fn delete_subscription(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<String>,
) -> Result<StatusCode, AppError> {
    state.subscriptions.delete(&id, &user.id).await?;
    Ok(StatusCode::NO_CONTENT)
}
