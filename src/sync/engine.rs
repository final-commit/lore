use chrono::Utc;
use git2::{AutotagOption, FetchOptions, PushOptions, RemoteCallbacks, Repository};
use rusqlite::params;
use serde::{Deserialize, Serialize};

use crate::db::{with_conn, DbConn};
use crate::error::AppError;
use crate::git::GitQueue;

// ── Data types ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncStatus {
    pub remote_url: Option<String>,
    pub branch: String,
    pub ahead: usize,
    pub behind: usize,
    pub last_pull_at: Option<String>,
    pub last_push_at: Option<String>,
    pub last_pull_commit: Option<String>,
    pub last_push_commit: Option<String>,
    pub has_conflicts: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResult {
    pub success: bool,
    pub message: String,
    pub commits_transferred: usize,
}

// ── Engine ────────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct SyncEngine {
    db: DbConn,
    repo_path: std::path::PathBuf,
    queue: GitQueue,
}

impl SyncEngine {
    pub fn new(
        db: DbConn,
        repo_path: std::path::PathBuf,
        queue: GitQueue,
    ) -> Self {
        SyncEngine { db, repo_path, queue }
    }

    /// Return the current sync status (ahead/behind counts, last sync times).
    pub async fn status(&self) -> Result<SyncStatus, AppError> {
        let db = self.db.clone();
        let row = with_conn(&db, |conn| {
            conn.query_row(
                "SELECT remote_url, branch, last_pull_at, last_push_at, last_pull_commit, last_push_commit FROM sync_state WHERE id=1",
                [],
                |row| {
                    Ok((
                        row.get::<_, Option<String>>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, Option<String>>(2)?,
                        row.get::<_, Option<String>>(3)?,
                        row.get::<_, Option<String>>(4)?,
                        row.get::<_, Option<String>>(5)?,
                    ))
                },
            )
        })
        .await
        .map_err(AppError::Db)?;

        let (remote_url, branch, last_pull_at, last_push_at, last_pull_commit, last_push_commit) =
            row;

        let (ahead, behind) = self
            .compute_ahead_behind(&branch)
            .await
            .unwrap_or((0, 0));

        Ok(SyncStatus {
            remote_url,
            branch,
            ahead,
            behind,
            last_pull_at,
            last_push_at,
            last_pull_commit,
            last_push_commit,
            has_conflicts: false,
        })
    }

    /// Pull from the configured remote.
    pub async fn pull(&self) -> Result<SyncResult, AppError> {
        let repo_path = self.repo_path.clone();
        let db = self.db.clone();

        let branch = with_conn(&db, |conn| {
            conn.query_row(
                "SELECT branch FROM sync_state WHERE id=1",
                [],
                |r| r.get::<_, String>(0),
            )
        })
        .await
        .map_err(AppError::Db)?;

        let br = branch.clone();
        let result = self
            .queue
            .run(move || {
                let repo = Repository::open(&repo_path)?;
                let mut remote = repo
                    .find_remote("origin")
                    .map_err(|_| AppError::NotFound("remote 'origin' not configured".into()))?;

                let mut fetch_opts = FetchOptions::new();
                fetch_opts.download_tags(AutotagOption::Unspecified);

                remote
                    .fetch(&[&br], Some(&mut fetch_opts), None)
                    .map_err(AppError::Git)?;

                // Try fast-forward merge
                let fetch_head = repo
                    .find_reference("FETCH_HEAD")
                    .map_err(|_| AppError::Internal("FETCH_HEAD not found".into()))?;

                let fetch_commit = repo
                    .reference_to_annotated_commit(&fetch_head)
                    .map_err(AppError::Git)?;

                let analysis = repo.merge_analysis(&[&fetch_commit]).map_err(AppError::Git)?;

                if analysis.0.is_up_to_date() {
                    return Ok(SyncResult {
                        success: true,
                        message: "already up to date".into(),
                        commits_transferred: 0,
                    });
                }

                if analysis.0.is_fast_forward() {
                    let refname = format!("refs/heads/{br}");
                    let mut reference = repo.find_reference(&refname).map_err(AppError::Git)?;
                    reference
                        .set_target(fetch_commit.id(), "fast-forward")
                        .map_err(AppError::Git)?;
                    repo.set_head(&refname).map_err(AppError::Git)?;
                    repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))
                        .map_err(AppError::Git)?;

                    return Ok(SyncResult {
                        success: true,
                        message: "fast-forward merge successful".into(),
                        commits_transferred: 1,
                    });
                }

                // Diverged — report conflict
                Ok(SyncResult {
                    success: false,
                    message: "pull requires merge; manual resolution needed".into(),
                    commits_transferred: 0,
                })
            })
            .await?;

        if result.success {
            let head = self.repo_head_sha().await.ok().flatten();
            let now = Utc::now().to_rfc3339();
            with_conn(&self.db, move |conn| {
                conn.execute(
                    "UPDATE sync_state SET last_pull_at=?1, last_pull_commit=?2 WHERE id=1",
                    params![now, head],
                )
                .map(|_| ())
            })
            .await
            .map_err(AppError::Db)?;
        }

        Ok(result)
    }

    /// Push to the configured remote.
    pub async fn push(&self) -> Result<SyncResult, AppError> {
        let repo_path = self.repo_path.clone();
        let db = self.db.clone();

        let branch = with_conn(&db, |conn| {
            conn.query_row(
                "SELECT branch FROM sync_state WHERE id=1",
                [],
                |r| r.get::<_, String>(0),
            )
        })
        .await
        .map_err(AppError::Db)?;

        let br = branch.clone();
        let result = self
            .queue
            .run(move || {
                let repo = Repository::open(&repo_path)?;
                let mut remote = repo
                    .find_remote("origin")
                    .map_err(|_| AppError::NotFound("remote 'origin' not configured".into()))?;

                let refspec = format!("refs/heads/{br}:refs/heads/{br}");
                let mut push_opts = PushOptions::new();
                remote
                    .push(&[&refspec], Some(&mut push_opts))
                    .map_err(AppError::Git)?;

                Ok(SyncResult {
                    success: true,
                    message: "push successful".into(),
                    commits_transferred: 1,
                })
            })
            .await?;

        if result.success {
            let head = self.repo_head_sha().await.ok().flatten();
            let now = Utc::now().to_rfc3339();
            with_conn(&self.db, move |conn| {
                conn.execute(
                    "UPDATE sync_state SET last_push_at=?1, last_push_commit=?2 WHERE id=1",
                    params![now, head],
                )
                .map(|_| ())
            })
            .await
            .map_err(AppError::Db)?;
        }

        Ok(result)
    }

    /// Update the remote URL in the DB.
    pub async fn set_remote_url(&self, url: &str) -> Result<(), AppError> {
        let url = url.to_string();
        with_conn(&self.db, move |conn| {
            conn.execute("UPDATE sync_state SET remote_url=?1 WHERE id=1", params![url])
                .map(|_| ())
        })
        .await
        .map_err(AppError::Db)
    }

    /// Update the branch in the DB.
    pub async fn set_branch(&self, branch: &str) -> Result<(), AppError> {
        let branch = branch.to_string();
        with_conn(&self.db, move |conn| {
            conn.execute("UPDATE sync_state SET branch=?1 WHERE id=1", params![branch])
                .map(|_| ())
        })
        .await
        .map_err(AppError::Db)
    }

    // ── Helpers ────────────────────────────────────────────────────────────

    async fn compute_ahead_behind(&self, branch: &str) -> Result<(usize, usize), AppError> {
        let repo_path = self.repo_path.clone();
        let branch = branch.to_string();
        self.queue
            .run(move || {
                let repo = Repository::open(&repo_path)?;
                let local_ref = format!("refs/heads/{branch}");
                let remote_ref = format!("refs/remotes/origin/{branch}");

                let local_oid = match repo.find_reference(&local_ref) {
                    Ok(r) => r.target().unwrap_or(git2::Oid::zero()),
                    Err(_) => return Ok((0, 0)),
                };
                let remote_oid = match repo.find_reference(&remote_ref) {
                    Ok(r) => r.target().unwrap_or(git2::Oid::zero()),
                    Err(_) => return Ok((0, 0)),
                };

                let (ahead, behind) = repo.graph_ahead_behind(local_oid, remote_oid)?;
                Ok((ahead, behind))
            })
            .await
    }

    async fn repo_head_sha(&self) -> Result<Option<String>, AppError> {
        let repo_path = self.repo_path.clone();
        self.queue
            .run(move || {
                let repo = Repository::open(&repo_path)?;
                Ok(repo.head().ok().and_then(|h| h.target()).map(|o| o.to_string()))
            })
            .await
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use tempfile::TempDir;

    fn setup() -> (TempDir, SyncEngine) {
        let dir = TempDir::new().unwrap();
        git2::Repository::init(dir.path()).unwrap();
        let db = db::open(":memory:").unwrap();
        let queue = GitQueue::new();
        let engine = SyncEngine::new(db, dir.path().to_path_buf(), queue);
        (dir, engine)
    }

    #[tokio::test]
    async fn test_status_defaults() {
        let (_dir, engine) = setup();
        let status = engine.status().await.unwrap();
        assert_eq!(status.branch, "main");
        assert!(status.remote_url.is_none());
        assert_eq!(status.ahead, 0);
        assert_eq!(status.behind, 0);
    }

    #[tokio::test]
    async fn test_set_remote_url() {
        let (_dir, engine) = setup();
        engine.set_remote_url("https://github.com/user/repo.git").await.unwrap();
        let status = engine.status().await.unwrap();
        assert_eq!(
            status.remote_url.as_deref(),
            Some("https://github.com/user/repo.git")
        );
    }

    #[tokio::test]
    async fn test_set_branch() {
        let (_dir, engine) = setup();
        engine.set_branch("develop").await.unwrap();
        let status = engine.status().await.unwrap();
        assert_eq!(status.branch, "develop");
    }

    #[tokio::test]
    async fn test_pull_without_remote_returns_error() {
        let (_dir, engine) = setup();
        let err = engine.pull().await.unwrap_err();
        // No remote configured → NotFound
        assert!(matches!(err, AppError::NotFound(_)));
    }

    #[tokio::test]
    async fn test_push_without_remote_returns_error() {
        let (_dir, engine) = setup();
        let err = engine.push().await.unwrap_err();
        assert!(matches!(err, AppError::NotFound(_)));
    }
}
