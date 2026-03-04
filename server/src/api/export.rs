use axum::{
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::IntoResponse,
    Json,
};
use pulldown_cmark::{html, Parser};
use serde::Deserialize;

use crate::auth::AuthUser;
use crate::error::{validate_path, AppError};
use crate::state::AppState;

#[derive(Deserialize)]
pub struct ExportDocQuery {
    pub path: String,
    pub format: Option<String>,
}

/// GET /api/export/doc?path=&format=
/// Formats: markdown (default) | html
pub async fn export_doc(
    State(state): State<AppState>,
    _user: AuthUser,
    Query(q): Query<ExportDocQuery>,
) -> Result<impl IntoResponse, AppError> {
    validate_path(&q.path)?;

    let doc = state.git.read_file(&q.path).await.map_err(|_| {
        AppError::NotFound(format!("document {} not found", q.path))
    })?;

    let format = q.format.as_deref().unwrap_or("markdown");

    match format {
        "markdown" | "md" => Ok((
            StatusCode::OK,
            [(header::CONTENT_TYPE, "text/markdown; charset=utf-8")],
            doc.content,
        )
            .into_response()),
        "html" => {
            let parser = Parser::new(&doc.content);
            let mut html_output = String::new();
            html::push_html(&mut html_output, parser);
            Ok((
                StatusCode::OK,
                [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
                html_output,
            )
                .into_response())
        }
        _ => Err(AppError::BadRequest(format!(
            "unsupported format '{}': use markdown or html",
            format
        ))),
    }
}

/// GET /api/export/collection/{id}
/// Returns JSON array of {path, content} for all docs in the collection.
pub async fn export_collection(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    // Verify collection exists.
    let _collection = state.collections.get(&id).await?;

    // Get the full doc tree.
    let tree = state.git.read_tree("").await?;

    // Collect all markdown docs and their content.
    let mut docs = Vec::new();
    for entry in &tree {
        if entry.is_dir || !entry.path.ends_with(".md") {
            continue;
        }
        match state.git.read_file(&entry.path).await {
            Ok(doc) => {
                docs.push(serde_json::json!({
                    "path": doc.path,
                    "content": doc.content,
                }));
            }
            Err(_) => {
                // Skip files we can't read.
            }
        }
    }

    Ok(Json(serde_json::json!({
        "collection_id": id,
        "docs": docs,
    })))
}
