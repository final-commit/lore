use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};

use crate::auth::AuthUser;
use crate::error::AppError;
use crate::git::engine::CommitInfo;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct RevisionListQuery {
    pub doc_path: String,
    pub limit: Option<usize>,
}

#[derive(Deserialize)]
pub struct RevisionQuery {
    pub doc_path: String,
}

#[derive(Debug, Serialize)]
pub struct RevisionContent {
    pub sha: String,
    pub content: String,
    pub author: String,
    pub timestamp: i64,
}

/// GET /api/revisions?doc_path=...&limit=50
pub async fn list_revisions(
    State(state): State<AppState>,
    _user: AuthUser,
    Query(params): Query<RevisionListQuery>,
) -> Result<Json<Vec<CommitInfo>>, AppError> {
    if params.doc_path.trim().is_empty() {
        return Err(AppError::BadRequest("doc_path is required".into()));
    }
    let limit = params.limit.unwrap_or(50);
    let history = state.git.history(&params.doc_path, limit).await?;
    Ok(Json(history))
}

/// GET /api/revisions/{sha}?doc_path=...
pub async fn get_revision_content(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(sha): Path<String>,
    Query(params): Query<RevisionQuery>,
) -> Result<Json<RevisionContent>, AppError> {
    if params.doc_path.trim().is_empty() {
        return Err(AppError::BadRequest("doc_path is required".into()));
    }
    let content = state
        .git
        .get_revision_content(&params.doc_path, &sha)
        .await?;

    // Get author/timestamp from commit info
    let history = state.git.history(&params.doc_path, 200).await?;
    let info = history.into_iter().find(|c| c.sha == sha);

    let (author, timestamp) = info
        .map(|c| (c.author, c.timestamp))
        .unwrap_or_default();

    Ok(Json(RevisionContent {
        sha,
        content,
        author,
        timestamp,
    }))
}

/// POST /api/revisions/{sha}/restore?doc_path=...
pub async fn restore_revision(
    State(state): State<AppState>,
    user: AuthUser,
    Path(sha): Path<String>,
    Query(params): Query<RevisionQuery>,
) -> Result<(StatusCode, Json<CommitInfo>), AppError> {
    if params.doc_path.trim().is_empty() {
        return Err(AppError::BadRequest("doc_path is required".into()));
    }
    let info = state
        .git
        .restore_revision(&params.doc_path, &sha, &user.email, &user.email)
        .await?;
    Ok((StatusCode::CREATED, Json(info)))
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use axum::extract::{Path, Query, State};
    use rusqlite::Connection;
    use std::sync::{Arc, Mutex};

    use crate::auth::AuthUser;
    use crate::db::schema::apply_migrations;
    use crate::error::AppError;
    use crate::state::AppState;

    use super::*;

    fn make_user() -> AuthUser {
        AuthUser {
            id: "user-1".to_string(),
            email: "user@example.com".to_string(),
            role: "editor".to_string(),
        }
    }

    async fn make_state() -> (AppState, tempfile::TempDir) {
        use crate::git::queue::GitQueue;
        use crate::git::GitEngine;
        use crate::search::SearchEngine;

        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
        apply_migrations(&conn).unwrap();

        let db = Arc::new(Mutex::new(conn));
        let tmp = tempfile::TempDir::new().unwrap();
        let tmp2 = tempfile::TempDir::new().unwrap();
        let queue = GitQueue::new();
        let git = GitEngine::init(tmp.path().to_path_buf(), queue).unwrap();
        let search = SearchEngine::open_in_ram().unwrap();

        let state = AppState::new_for_test(db, git, search, tmp2);
        // Return tmp to keep the git repo dir alive
        (state, tmp)
    }

    #[tokio::test]
    async fn test_list_revisions_empty_repo() {
        let (state, _tmp) = make_state().await;
        let result = list_revisions(
            State(state),
            make_user(),
            Query(RevisionListQuery {
                doc_path: "docs/test.md".to_string(),
                limit: None,
            }),
        )
        .await
        .unwrap();
        assert!(result.0.is_empty());
    }

    #[tokio::test]
    async fn test_list_revisions_with_commits() {
        let (state, _tmp) = make_state().await;
        state
            .git
            .write_file("docs/test.md", "v1", "first", "User", "user@example.com")
            .await
            .unwrap();
        state
            .git
            .write_file("docs/test.md", "v2", "second", "User", "user@example.com")
            .await
            .unwrap();

        let result = list_revisions(
            State(state),
            make_user(),
            Query(RevisionListQuery {
                doc_path: "docs/test.md".to_string(),
                limit: Some(10),
            }),
        )
        .await
        .unwrap();
        assert_eq!(result.0.len(), 2);
    }

    #[tokio::test]
    async fn test_get_revision_content() {
        let (state, _tmp) = make_state().await;
        let sha = state
            .git
            .write_file("doc.md", "version one", "v1", "User", "user@example.com")
            .await
            .unwrap();
        state
            .git
            .write_file("doc.md", "version two", "v2", "User", "user@example.com")
            .await
            .unwrap();

        let result = get_revision_content(
            State(state),
            make_user(),
            Path(sha.clone()),
            Query(RevisionQuery {
                doc_path: "doc.md".to_string(),
            }),
        )
        .await
        .unwrap();
        assert_eq!(result.0.sha, sha);
        assert_eq!(result.0.content, "version one");
    }

    #[tokio::test]
    async fn test_restore_revision() {
        let (state, _tmp) = make_state().await;
        let sha = state
            .git
            .write_file("doc.md", "original", "v1", "User", "user@example.com")
            .await
            .unwrap();
        state
            .git
            .write_file("doc.md", "changed", "v2", "User", "user@example.com")
            .await
            .unwrap();

        let (status, info) = restore_revision(
            State(state.clone()),
            make_user(),
            Path(sha.clone()),
            Query(RevisionQuery {
                doc_path: "doc.md".to_string(),
            }),
        )
        .await
        .unwrap();
        assert_eq!(status, axum::http::StatusCode::CREATED);
        assert!(info.0.message.contains("Restore to"));

        let doc = state.git.read_file("doc.md").await.unwrap();
        assert_eq!(doc.content, "original");
    }

    #[tokio::test]
    async fn test_invalid_sha_returns_error() {
        let (state, _tmp) = make_state().await;
        state
            .git
            .write_file("doc.md", "content", "init", "User", "user@example.com")
            .await
            .unwrap();

        let err = get_revision_content(
            State(state),
            make_user(),
            Path("0000000000000000000000000000000000000000".to_string()),
            Query(RevisionQuery {
                doc_path: "doc.md".to_string(),
            }),
        )
        .await
        .unwrap_err();
        assert!(matches!(err, AppError::NotFound(_)));
    }
}
