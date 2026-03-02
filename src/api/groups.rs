use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;

use crate::auth::AuthUser;
use crate::error::AppError;
use crate::groups::{Group, GroupMember};
use crate::groups::engine::{CreateGroup, UpdateGroup};
use crate::state::AppState;

#[derive(Deserialize)]
pub struct CreateGroupReq {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateGroupReq {
    pub name: Option<String>,
    pub description: Option<String>,
}

#[derive(Deserialize)]
pub struct AddMemberReq {
    pub user_id: String,
}

/// GET /api/groups
pub async fn list_groups(
    State(state): State<AppState>,
    _user: AuthUser,
) -> Result<Json<Vec<Group>>, AppError> {
    let groups = state.groups.list().await?;
    Ok(Json(groups))
}

/// POST /api/groups
pub async fn create_group(
    State(state): State<AppState>,
    user: AuthUser,
    Json(req): Json<CreateGroupReq>,
) -> Result<(StatusCode, Json<Group>), AppError> {
    if req.name.trim().is_empty() {
        return Err(AppError::BadRequest("group name must not be empty".into()));
    }
    let group = state.groups.create(CreateGroup {
        name: req.name,
        description: req.description,
        created_by: Some(user.id.clone()),
    }).await?;
    Ok((StatusCode::CREATED, Json(group)))
}

/// PUT /api/groups/{id}
pub async fn update_group(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<String>,
    Json(req): Json<UpdateGroupReq>,
) -> Result<Json<Group>, AppError> {
    if !user.is_admin() {
        return Err(AppError::Forbidden("only admins can update groups".into()));
    }
    let group = state.groups.update(&id, UpdateGroup {
        name: req.name,
        description: req.description,
    }).await?;
    Ok(Json(group))
}

/// DELETE /api/groups/{id}
pub async fn delete_group(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<String>,
) -> Result<StatusCode, AppError> {
    if !user.is_admin() {
        return Err(AppError::Forbidden("only admins can delete groups".into()));
    }
    state.groups.delete(&id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// GET /api/groups/{id}/members
pub async fn list_members(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<Vec<GroupMember>>, AppError> {
    // Verify group exists.
    state.groups.get(&id).await?;
    let members = state.groups.list_members(&id).await?;
    Ok(Json(members))
}

/// POST /api/groups/{id}/members
pub async fn add_member(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<String>,
    Json(req): Json<AddMemberReq>,
) -> Result<(StatusCode, Json<GroupMember>), AppError> {
    if !user.is_admin() {
        return Err(AppError::Forbidden("only admins can add group members".into()));
    }
    let member = state.groups.add_member(&id, &req.user_id).await?;
    Ok((StatusCode::CREATED, Json(member)))
}

/// DELETE /api/groups/{id}/members/{user_id}
pub async fn remove_member(
    State(state): State<AppState>,
    user: AuthUser,
    Path((id, user_id)): Path<(String, String)>,
) -> Result<StatusCode, AppError> {
    if !user.is_admin() {
        return Err(AppError::Forbidden("only admins can remove group members".into()));
    }
    state.groups.remove_member(&id, &user_id).await?;
    Ok(StatusCode::NO_CONTENT)
}
