use axum::{
    extract::{Query, State},
    Json,
};
use serde::Deserialize;

use crate::error::AppError;
use crate::search::SearchResult;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct SearchParams {
    pub q: String,
    pub limit: Option<usize>,
}

/// GET /api/search?q=...&limit=...
pub async fn search_handler(
    State(state): State<AppState>,
    Query(params): Query<SearchParams>,
) -> Result<Json<Vec<SearchResult>>, AppError> {
    if params.q.trim().is_empty() {
        return Err(AppError::BadRequest("search query must not be empty".into()));
    }
    let limit = params.limit.unwrap_or(20).min(100);
    let results = state.search.query(&params.q, limit).await?;
    Ok(Json(results))
}
