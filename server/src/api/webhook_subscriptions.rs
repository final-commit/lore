use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;

use crate::auth::AuthUser;
use crate::error::AppError;
use crate::outbound_webhooks::WebhookSubscription;
use crate::outbound_webhooks::engine::{CreateWebhookSubscription, UpdateWebhookSubscription};
use crate::state::AppState;

#[derive(Deserialize)]
pub struct CreateWebhookReq {
    pub url: String,
    pub secret: Option<String>,
    pub events: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateWebhookReq {
    pub url: Option<String>,
    pub secret: Option<String>,
    pub events: Option<String>,
    pub enabled: Option<bool>,
}

/// GET /api/webhook-subscriptions
pub async fn list_webhook_subscriptions(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<Json<Vec<WebhookSubscription>>, AppError> {
    if !user.is_admin() {
        return Err(AppError::Forbidden("only admins can manage webhook subscriptions".into()));
    }
    let webhooks = state.outbound_webhooks.list().await?;
    Ok(Json(webhooks))
}

/// POST /api/webhook-subscriptions
pub async fn create_webhook_subscription(
    State(state): State<AppState>,
    user: AuthUser,
    Json(req): Json<CreateWebhookReq>,
) -> Result<(StatusCode, Json<WebhookSubscription>), AppError> {
    if !user.is_admin() {
        return Err(AppError::Forbidden("only admins can manage webhook subscriptions".into()));
    }
    if req.url.is_empty() {
        return Err(AppError::BadRequest("url must not be empty".into()));
    }
    let webhook = state.outbound_webhooks.create(CreateWebhookSubscription {
        url: req.url,
        secret: req.secret,
        events: req.events,
        created_by: Some(user.id.clone()),
    }).await?;
    Ok((StatusCode::CREATED, Json(webhook)))
}

/// PUT /api/webhook-subscriptions/{id}
pub async fn update_webhook_subscription(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<String>,
    Json(req): Json<UpdateWebhookReq>,
) -> Result<Json<WebhookSubscription>, AppError> {
    if !user.is_admin() {
        return Err(AppError::Forbidden("only admins can manage webhook subscriptions".into()));
    }
    let webhook = state.outbound_webhooks.update(&id, UpdateWebhookSubscription {
        url: req.url,
        secret: req.secret,
        events: req.events,
        enabled: req.enabled,
    }).await?;
    Ok(Json(webhook))
}

/// DELETE /api/webhook-subscriptions/{id}
pub async fn delete_webhook_subscription(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<String>,
) -> Result<StatusCode, AppError> {
    if !user.is_admin() {
        return Err(AppError::Forbidden("only admins can manage webhook subscriptions".into()));
    }
    state.outbound_webhooks.delete(&id).await?;
    Ok(StatusCode::NO_CONTENT)
}
