use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};

use crate::auth::AuthUser;
use crate::cache::CachedPage;
use crate::error::{validate_path, AppError};
use crate::git::engine::{CommitInfo, Document};
use crate::search::IndexDoc;
use crate::state::AppState;

// ── Request / Response types ──────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct DocResponse {
    pub path: String,
    pub content: String,
    pub sha: String,
    pub commit_sha: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateDocRequest {
    pub path: String,
    pub content: String,
    pub message: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateDocRequest {
    pub content: String,
    pub message: Option<String>,
}

// ── Handlers ──────────────────────────────────────────────────────────────────

/// GET /api/docs/*path
pub async fn get_doc(
    State(state): State<AppState>,
    Path(path): Path<String>,
) -> Result<Json<DocResponse>, AppError> {
    validate_path(&path)?;

    let head = state.git.head_sha().await?.unwrap_or_default();

    // Try cache first.
    if let Some(cached) = state.cache.get(&path, &head).await {
        return Ok(Json(DocResponse {
            path: cached.path,
            content: cached.content,
            sha: String::new(),
            commit_sha: cached.commit_sha,
        }));
    }

    let doc = state.git.read_file(&path).await?;
    let commit_sha = doc.commit_sha.clone();

    // Store in cache.
    state
        .cache
        .insert(CachedPage {
            path: doc.path.clone(),
            content: doc.content.clone(),
            commit_sha: commit_sha.clone(),
        })
        .await;

    Ok(Json(DocResponse {
        path: doc.path,
        content: doc.content,
        sha: doc.sha,
        commit_sha,
    }))
}

/// PUT /api/docs/*path
pub async fn update_doc(
    State(state): State<AppState>,
    user: AuthUser,
    Path(path): Path<String>,
    Json(req): Json<UpdateDocRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    validate_path(&path)?;
    if !user.is_editor_or_admin() {
        return Err(AppError::Forbidden("editor or admin role required".into()));
    }

    let message = req
        .message
        .unwrap_or_else(|| format!("Update {path}"));

    let commit_sha = state
        .git
        .write_file(&path, &req.content, &message, &user.email, &user.email)
        .await?;

    // Invalidate cache and update search index.
    state.cache.invalidate(&path).await;
    let _ = state
        .search
        .upsert(IndexDoc {
            path: path.clone(),
            title: extract_title(&req.content),
            body: strip_markdown(&req.content),
        })
        .await;

    Ok(Json(serde_json::json!({ "commit_sha": commit_sha })))
}

/// POST /api/docs
pub async fn create_doc(
    State(state): State<AppState>,
    user: AuthUser,
    Json(req): Json<CreateDocRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), AppError> {
    validate_path(&req.path)?;
    if !user.is_editor_or_admin() {
        return Err(AppError::Forbidden("editor or admin role required".into()));
    }

    // Check it doesn't already exist.
    if state.git.read_file(&req.path).await.is_ok() {
        return Err(AppError::Conflict(format!("document already exists: {}", req.path)));
    }

    let message = req.message.unwrap_or_else(|| format!("Create {}", req.path));
    let commit_sha = state
        .git
        .write_file(&req.path, &req.content, &message, &user.email, &user.email)
        .await?;

    let _ = state
        .search
        .upsert(IndexDoc {
            path: req.path.clone(),
            title: extract_title(&req.content),
            body: strip_markdown(&req.content),
        })
        .await;

    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({ "path": req.path, "commit_sha": commit_sha })),
    ))
}

/// DELETE /api/docs/*path
pub async fn delete_doc(
    State(state): State<AppState>,
    user: AuthUser,
    Path(path): Path<String>,
) -> Result<StatusCode, AppError> {
    validate_path(&path)?;
    if !user.is_admin() {
        return Err(AppError::Forbidden("admin role required to delete documents".into()));
    }

    state
        .git
        .delete_file(&path, &format!("Delete {path}"), &user.email, &user.email)
        .await?;

    state.cache.invalidate(&path).await;
    let _ = state.search.remove(&path).await;

    Ok(StatusCode::NO_CONTENT)
}

/// GET /api/docs/*path/history
pub async fn doc_history(
    State(state): State<AppState>,
    Path(path): Path<String>,
) -> Result<Json<Vec<CommitInfo>>, AppError> {
    validate_path(&path)?;
    let history = state.git.history(&path, 50).await?;
    Ok(Json(history))
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Extract the first heading from Markdown as the title.
fn extract_title(content: &str) -> String {
    content
        .lines()
        .find(|l| l.starts_with("# "))
        .map(|l| l.trim_start_matches("# ").trim().to_string())
        .unwrap_or_else(|| "Untitled".to_string())
}

/// Remove Markdown syntax for indexing.
fn strip_markdown(content: &str) -> String {
    // Simple stripping: remove heading markers, link syntax, emphasis.
    let re = regex::Regex::new(r"[#*_`\[\]()]|https?://\S+").unwrap();
    re.replace_all(content, " ").into_owned()
}
