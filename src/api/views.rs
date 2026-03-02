use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;

use crate::auth::AuthUser;
use crate::error::{validate_path, AppError};
use crate::views::View;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct RecordViewReq {
    pub doc_path: String,
}

#[derive(Deserialize)]
pub struct ListViewsQuery {
    pub doc_path: String,
}

/// POST /api/views — record a view (upsert)
pub async fn record_view(
    State(state): State<AppState>,
    user: AuthUser,
    Json(req): Json<RecordViewReq>,
) -> Result<(StatusCode, Json<View>), AppError> {
    validate_path(&req.doc_path)?;
    let view = state.views.record(&user.id, &req.doc_path).await?;
    Ok((StatusCode::OK, Json(view)))
}

/// GET /api/views?doc_path= — list viewers for a doc (admin or any authenticated user)
pub async fn list_views(
    State(state): State<AppState>,
    user: AuthUser,
    Query(q): Query<ListViewsQuery>,
) -> Result<Json<Vec<View>>, AppError> {
    if !user.is_admin() {
        return Err(AppError::Forbidden("admin required to list doc viewers".into()));
    }
    let views = state.views.list_for_doc(&q.doc_path).await?;
    Ok(Json(views))
}

/// GET /api/views/recent — current user's recently viewed docs
pub async fn recent_views(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<Json<Vec<View>>, AppError> {
    let views = state.views.list_recent_for_user(&user.id, 50).await?;
    Ok(Json(views))
}
