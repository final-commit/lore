use axum::{extract::State, Json};

use crate::error::AppError;
use crate::git::engine::TreeEntry;
use crate::state::AppState;

pub async fn tree_handler(
    State(state): State<AppState>,
) -> Result<Json<Vec<TreeEntry>>, AppError> {
    let entries = state.git.read_tree("").await?;
    Ok(Json(entries))
}
