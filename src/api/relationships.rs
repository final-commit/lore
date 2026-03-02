use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;

use crate::auth::AuthUser;
use crate::error::{validate_path, AppError};
use crate::relationships::Relationship;
use crate::relationships::engine::CreateRelationship;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct ListRelationshipsQuery {
    pub doc_path: Option<String>,
}

#[derive(Deserialize)]
pub struct CreateRelationshipReq {
    pub source_doc_path: String,
    pub target_doc_path: String,
    pub rel_type: Option<String>,
}

/// POST /api/relationships
pub async fn create_relationship(
    State(state): State<AppState>,
    user: AuthUser,
    Json(req): Json<CreateRelationshipReq>,
) -> Result<(StatusCode, Json<Relationship>), AppError> {
    validate_path(&req.source_doc_path)?;
    validate_path(&req.target_doc_path)?;
    let rel = state.relationships.create(CreateRelationship {
        source_doc_path: req.source_doc_path,
        target_doc_path: req.target_doc_path,
        rel_type: req.rel_type,
        created_by: Some(user.id.clone()),
    }).await?;
    Ok((StatusCode::CREATED, Json(rel)))
}

/// GET /api/relationships?doc_path=
pub async fn list_relationships(
    State(state): State<AppState>,
    _user: AuthUser,
    Query(q): Query<ListRelationshipsQuery>,
) -> Result<Json<Vec<Relationship>>, AppError> {
    let doc_path = q.doc_path.ok_or_else(|| AppError::BadRequest("doc_path is required".into()))?;
    let rels = state.relationships.list_for_doc(&doc_path).await?;
    Ok(Json(rels))
}

/// DELETE /api/relationships/{id}
pub async fn delete_relationship(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(id): Path<String>,
) -> Result<StatusCode, AppError> {
    state.relationships.delete(&id).await?;
    Ok(StatusCode::NO_CONTENT)
}
