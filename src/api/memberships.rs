use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;

use crate::auth::AuthUser;
use crate::error::AppError;
use crate::memberships::Membership;
use crate::memberships::engine::{CreateMembership, UpdateMembership};
use crate::state::AppState;

#[derive(Deserialize)]
pub struct ListMembershipsQuery {
    pub collection_id: Option<String>,
}

#[derive(Deserialize)]
pub struct CreateMembershipReq {
    pub user_id: String,
    pub collection_id: String,
    pub permission: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateMembershipReq {
    pub permission: String,
}

/// GET /api/memberships?collection_id=
pub async fn list_memberships(
    State(state): State<AppState>,
    _user: AuthUser,
    Query(q): Query<ListMembershipsQuery>,
) -> Result<Json<Vec<Membership>>, AppError> {
    let collection_id = q.collection_id.unwrap_or_default();
    if collection_id.is_empty() {
        return Err(AppError::BadRequest("collection_id query param is required".into()));
    }
    let memberships = state.memberships.list_for_collection(&collection_id).await?;
    Ok(Json(memberships))
}

/// POST /api/memberships
pub async fn create_membership(
    State(state): State<AppState>,
    user: AuthUser,
    Json(req): Json<CreateMembershipReq>,
) -> Result<(StatusCode, Json<Membership>), AppError> {
    if !user.is_admin() {
        return Err(AppError::Forbidden("only admins can manage memberships".into()));
    }
    let membership = state.memberships.create(CreateMembership {
        user_id: req.user_id,
        collection_id: req.collection_id,
        permission: req.permission,
        created_by: Some(user.id.clone()),
    }).await?;
    Ok((StatusCode::CREATED, Json(membership)))
}

/// PUT /api/memberships/{id}
pub async fn update_membership(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<String>,
    Json(req): Json<UpdateMembershipReq>,
) -> Result<Json<Membership>, AppError> {
    if !user.is_admin() {
        return Err(AppError::Forbidden("only admins can manage memberships".into()));
    }
    let membership = state.memberships.update(&id, UpdateMembership {
        permission: req.permission,
    }).await?;
    Ok(Json(membership))
}

/// DELETE /api/memberships/{id}
pub async fn delete_membership(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<String>,
) -> Result<StatusCode, AppError> {
    if !user.is_admin() {
        return Err(AppError::Forbidden("only admins can manage memberships".into()));
    }
    state.memberships.delete(&id).await?;
    Ok(StatusCode::NO_CONTENT)
}
