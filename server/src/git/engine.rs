use std::path::{Path, PathBuf};

use git2::{Repository, Signature};
use serde::{Deserialize, Serialize};

use crate::error::{validate_path, AppError};
use crate::git::queue::GitQueue;

// ── Public data types ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreeEntry {
    pub path: String,
    pub name: String,
    pub is_dir: bool,
    pub sha: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub path: String,
    pub content: String,
    pub sha: String,       // blob SHA
    pub commit_sha: String, // HEAD commit SHA at read time
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitInfo {
    pub sha: String,
    pub message: String,
    pub author: String,
    pub author_email: String,
    pub timestamp: i64,
}

// ── Engine ────────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct GitEngine {
    pub repo_path: PathBuf,
    queue: GitQueue,
}

impl GitEngine {
    /// Open an existing repository at `repo_path`.
    pub fn open(repo_path: PathBuf, queue: GitQueue) -> Result<Self, AppError> {
        // Verify the repo is accessible.
        Repository::open(&repo_path).map_err(AppError::Git)?;
        Ok(GitEngine { repo_path, queue })
    }

    /// Initialise a new bare-compatible repo at `repo_path`.
    /// Returns an engine pointing at it.
    pub fn init(repo_path: PathBuf, queue: GitQueue) -> Result<Self, AppError> {
        Repository::init(&repo_path).map_err(AppError::Git)?;
        Ok(GitEngine { repo_path, queue })
    }

    // ── Read operations ────────────────────────────────────────────────────

    /// Return a flat list of all file/directory entries under `dir_path`
    /// (recursive, pre-order walk).  Pass `""` for the repository root.
    pub async fn read_tree(&self, dir_path: &str) -> Result<Vec<TreeEntry>, AppError> {
        if !dir_path.is_empty() {
            validate_path(dir_path)?;
        }
        let repo_path = self.repo_path.clone();
        let dir_path = dir_path.to_string();

        self.queue
            .run(move || {
                let repo = Repository::open(&repo_path)?;
                let head = match repo.head() {
                    Ok(h) => h,
                    Err(_) => return Ok(vec![]), // empty repo
                };
                let tree = head.peel_to_tree()?;

                let mut entries = Vec::new();
                tree.walk(git2::TreeWalkMode::PreOrder, |root, entry| {
                    let name = match entry.name() {
                        Some(n) => n.to_string(),
                        None => return git2::TreeWalkResult::Ok,
                    };
                    let full_path = if root.is_empty() {
                        name.clone()
                    } else {
                        format!("{root}{name}")
                    };

                    // Filter to dir_path prefix
                    if !dir_path.is_empty() && !full_path.starts_with(&dir_path) {
                        return git2::TreeWalkResult::Ok;
                    }

                    let is_dir = entry.kind() == Some(git2::ObjectType::Tree);
                    entries.push(TreeEntry {
                        path: full_path,
                        name,
                        is_dir,
                        sha: entry.id().to_string(),
                    });
                    git2::TreeWalkResult::Ok
                })?;

                Ok(entries)
            })
            .await
    }

    /// Read the raw content of a file at `file_path` from HEAD.
    pub async fn read_file(&self, file_path: &str) -> Result<Document, AppError> {
        validate_path(file_path)?;
        let repo_path = self.repo_path.clone();
        let file_path = file_path.to_string();

        self.queue
            .run(move || {
                let repo = Repository::open(&repo_path)?;
                let head = repo
                    .head()
                    .map_err(|_| AppError::NotFound("repository is empty".into()))?;
                let commit = head.peel_to_commit()?;
                let commit_sha = commit.id().to_string();
                let tree = commit.tree()?;

                let entry = tree
                    .get_path(Path::new(&file_path))
                    .map_err(|_| AppError::NotFound(format!("file not found: {file_path}")))?;

                let blob = repo
                    .find_blob(entry.id())
                    .map_err(|_| AppError::NotFound(format!("blob not found for {file_path}")))?;

                let content = std::str::from_utf8(blob.content())
                    .map_err(|_| AppError::BadRequest("file is not valid UTF-8".into()))?
                    .to_string();

                Ok(Document {
                    path: file_path,
                    content,
                    sha: entry.id().to_string(),
                    commit_sha,
                })
            })
            .await
    }

    // ── Write operations ──────────────────────────────────────────────────

    /// Create a new file atomically: checks existence and writes in a single queue slot.
    /// Returns `AppError::Conflict` if the file already exists.
    pub async fn create_file(
        &self,
        file_path: &str,
        content: &str,
        message: &str,
        author_name: &str,
        author_email: &str,
    ) -> Result<String, AppError> {
        validate_path(file_path)?;
        let repo_path = self.repo_path.clone();
        let file_path = file_path.to_string();
        let content = content.to_string();
        let message = message.to_string();
        let author_name = author_name.to_string();
        let author_email = author_email.to_string();

        self.queue
            .run(move || {
                let repo = Repository::open(&repo_path)?;

                // Check existence atomically within the same queue slot.
                let parent_commit = repo.head().ok().and_then(|h| h.peel_to_commit().ok());
                if let Some(ref commit) = parent_commit {
                    if let Ok(tree) = commit.tree() {
                        if tree.get_path(Path::new(&file_path)).is_ok() {
                            return Err(AppError::Conflict(format!(
                                "document already exists: {file_path}"
                            )));
                        }
                    }
                }

                let blob_oid = repo.blob(content.as_bytes())?;
                let base_tree = parent_commit.as_ref().and_then(|c| c.tree().ok());
                let new_tree_oid = insert_blob_in_tree(
                    &repo,
                    base_tree.as_ref(),
                    &file_path,
                    blob_oid,
                    0o100644,
                )?;
                let new_tree = repo.find_tree(new_tree_oid)?;
                let sig = Signature::now(&author_name, &author_email)?;
                let parents: Vec<&git2::Commit> =
                    parent_commit.as_ref().map(|c| vec![c]).unwrap_or_default();
                let oid = repo.commit(
                    Some("HEAD"),
                    &sig,
                    &sig,
                    &message,
                    &new_tree,
                    &parents,
                )?;
                Ok(oid.to_string())
            })
            .await
    }

    /// Write or update a file and create a git commit.  Returns the new commit SHA.
    pub async fn write_file(
        &self,
        file_path: &str,
        content: &str,
        message: &str,
        author_name: &str,
        author_email: &str,
    ) -> Result<String, AppError> {
        validate_path(file_path)?;
        let repo_path = self.repo_path.clone();
        let file_path = file_path.to_string();
        let content = content.to_string();
        let message = message.to_string();
        let author_name = author_name.to_string();
        let author_email = author_email.to_string();

        self.queue
            .run(move || {
                let repo = Repository::open(&repo_path)?;

                // Write the blob
                let blob_oid = repo.blob(content.as_bytes())?;

                // Build new tree, modifying the existing HEAD tree if present
                let parent_commit = repo.head().ok().and_then(|h| h.peel_to_commit().ok());
                let base_tree = parent_commit.as_ref().and_then(|c| c.tree().ok());

                let new_tree_oid = insert_blob_in_tree(
                    &repo,
                    base_tree.as_ref(),
                    &file_path,
                    blob_oid,
                    0o100644,
                )?;
                let new_tree = repo.find_tree(new_tree_oid)?;

                let sig = Signature::now(&author_name, &author_email)?;
                let parents: Vec<&git2::Commit> =
                    parent_commit.as_ref().map(|c| vec![c]).unwrap_or_default();

                let oid = repo.commit(
                    Some("HEAD"),
                    &sig,
                    &sig,
                    &message,
                    &new_tree,
                    &parents,
                )?;

                Ok(oid.to_string())
            })
            .await
    }

    /// Delete a file and create a git commit.  Returns the new commit SHA.
    pub async fn delete_file(
        &self,
        file_path: &str,
        message: &str,
        author_name: &str,
        author_email: &str,
    ) -> Result<String, AppError> {
        validate_path(file_path)?;
        let repo_path = self.repo_path.clone();
        let file_path = file_path.to_string();
        let message = message.to_string();
        let author_name = author_name.to_string();
        let author_email = author_email.to_string();

        self.queue
            .run(move || {
                let repo = Repository::open(&repo_path)?;
                let head = repo
                    .head()
                    .map_err(|_| AppError::NotFound("repository is empty".into()))?;
                let parent_commit = head.peel_to_commit()?;
                let base_tree = parent_commit.tree()?;

                // Check file exists
                base_tree
                    .get_path(Path::new(&file_path))
                    .map_err(|_| AppError::NotFound(format!("file not found: {file_path}")))?;

                let new_tree_oid = remove_blob_from_tree(&repo, &base_tree, &file_path)?;
                let new_tree = repo.find_tree(new_tree_oid)?;

                let sig = Signature::now(&author_name, &author_email)?;
                let oid = repo.commit(
                    Some("HEAD"),
                    &sig,
                    &sig,
                    &message,
                    &new_tree,
                    &[&parent_commit],
                )?;

                Ok(oid.to_string())
            })
            .await
    }

    /// Return commit history for a file (most recent first).
    pub async fn history(
        &self,
        file_path: &str,
        limit: usize,
    ) -> Result<Vec<CommitInfo>, AppError> {
        validate_path(file_path)?;
        let repo_path = self.repo_path.clone();
        let file_path = file_path.to_string();

        self.queue
            .run(move || {
                let repo = Repository::open(&repo_path)?;
                let head = match repo.head() {
                    Ok(h) => h,
                    Err(_) => return Ok(vec![]),
                };

                let mut walk = repo.revwalk()?;
                walk.push(head.target().unwrap())?;
                walk.set_sorting(git2::Sort::TOPOLOGICAL | git2::Sort::TIME)?;

                let mut results = Vec::new();
                for oid in walk {
                    if results.len() >= limit {
                        break;
                    }
                    let oid = oid?;
                    let commit = repo.find_commit(oid)?;

                    // Filter commits that touch this file
                    if commit_touches_file(&repo, &commit, &file_path)? {
                        results.push(CommitInfo {
                            sha: commit.id().to_string(),
                            message: commit.message().unwrap_or("").trim().to_string(),
                            author: commit.author().name().unwrap_or("").to_string(),
                            author_email: commit.author().email().unwrap_or("").to_string(),
                            timestamp: commit.time().seconds(),
                        });
                    }
                }

                Ok(results)
            })
            .await
    }

    /// Return the content of a file at a specific commit SHA.
    pub async fn get_revision_content(
        &self,
        file_path: &str,
        sha: &str,
    ) -> Result<String, AppError> {
        validate_path(file_path)?;
        let repo_path = self.repo_path.clone();
        let file_path = file_path.to_string();
        let sha = sha.to_string();

        self.queue
            .run(move || {
                let repo = Repository::open(&repo_path)?;
                let obj = repo
                    .revparse_single(&sha)
                    .map_err(|_| AppError::NotFound("revision".into()))?;
                let commit = obj
                    .peel_to_commit()
                    .map_err(|_| AppError::NotFound("revision".into()))?;
                let tree = commit.tree()?;
                let entry = tree
                    .get_path(Path::new(&file_path))
                    .map_err(|_| AppError::NotFound("revision".into()))?;
                let blob = repo
                    .find_blob(entry.id())
                    .map_err(|_| AppError::NotFound("revision".into()))?;
                let content = std::str::from_utf8(blob.content())
                    .map_err(|_| AppError::BadRequest("file is not valid UTF-8".into()))?
                    .to_string();
                Ok(content)
            })
            .await
    }

    /// Restore a file to a previous revision by writing its content as a new commit.
    pub async fn restore_revision(
        &self,
        file_path: &str,
        sha: &str,
        author_name: &str,
        author_email: &str,
    ) -> Result<CommitInfo, AppError> {
        let content = self.get_revision_content(file_path, sha).await?;
        let short = &sha[..sha.len().min(8)];
        let message = format!("Restore to {short}");
        let commit_sha = self
            .write_file(file_path, &content, &message, author_name, author_email)
            .await?;

        // Build CommitInfo for the new commit.
        let repo_path = self.repo_path.clone();
        let commit_sha_clone = commit_sha.clone();
        self.queue
            .run(move || {
                let repo = Repository::open(&repo_path)?;
                let oid = git2::Oid::from_str(&commit_sha_clone)
                    .map_err(|_| AppError::Internal("invalid oid".into()))?;
                let commit = repo.find_commit(oid)?;
                Ok(CommitInfo {
                    sha: commit.id().to_string(),
                    message: commit.message().unwrap_or("").trim().to_string(),
                    author: commit.author().name().unwrap_or("").to_string(),
                    author_email: commit.author().email().unwrap_or("").to_string(),
                    timestamp: commit.time().seconds(),
                })
            })
            .await
    }

    /// Get current HEAD SHA, or None for empty repo.
    pub async fn head_sha(&self) -> Result<Option<String>, AppError> {
        let repo_path = self.repo_path.clone();
        self.queue
            .run(move || {
                let repo = Repository::open(&repo_path)?;
                Ok(repo.head().ok().and_then(|h| h.target()).map(|o| o.to_string()))
            })
            .await
    }
}

// ── Tree helpers ──────────────────────────────────────────────────────────────

/// Recursively insert a blob at `file_path` (relative slash-separated) into
/// the tree rooted at `base_tree`, returning the new root tree OID.
fn insert_blob_in_tree(
    repo: &Repository,
    base_tree: Option<&git2::Tree>,
    file_path: &str,
    blob_oid: git2::Oid,
    filemode: i32,
) -> Result<git2::Oid, git2::Error> {
    let parts: Vec<&str> = file_path.splitn(2, '/').collect();

    let mut builder = repo.treebuilder(base_tree)?;

    if parts.len() == 1 {
        // Leaf: insert the blob directly.
        builder.insert(parts[0], blob_oid, filemode)?;
    } else {
        let dir = parts[0];
        let rest = parts[1];

        // Get existing subtree for this directory component (if any).
        let sub_base: Option<git2::Tree> = base_tree
            .and_then(|t| t.get_name(dir))
            .and_then(|e| repo.find_tree(e.id()).ok());

        let sub_oid = insert_blob_in_tree(repo, sub_base.as_ref(), rest, blob_oid, filemode)?;
        builder.insert(dir, sub_oid, 0o040000)?;
    }

    builder.write()
}

/// Recursively remove `file_path` from the tree, returning the new root OID.
fn remove_blob_from_tree(
    repo: &Repository,
    base_tree: &git2::Tree,
    file_path: &str,
) -> Result<git2::Oid, git2::Error> {
    let parts: Vec<&str> = file_path.splitn(2, '/').collect();
    let mut builder = repo.treebuilder(Some(base_tree))?;

    if parts.len() == 1 {
        builder.remove(parts[0])?;
    } else {
        let dir = parts[0];
        let rest = parts[1];

        if let Some(entry) = base_tree.get_name(dir) {
            if let Ok(subtree) = repo.find_tree(entry.id()) {
                let sub_oid = remove_blob_from_tree(repo, &subtree, rest)?;
                builder.insert(dir, sub_oid, 0o040000)?;
            }
        }
    }

    builder.write()
}

/// Return true if the given commit changed the file at `file_path`.
fn commit_touches_file(
    repo: &Repository,
    commit: &git2::Commit,
    file_path: &str,
) -> Result<bool, git2::Error> {
    let commit_tree = commit.tree()?;

    if commit.parent_count() == 0 {
        // Initial commit: file is "touched" if it exists.
        return Ok(commit_tree.get_path(Path::new(file_path)).is_ok());
    }

    for i in 0..commit.parent_count() {
        let parent = commit.parent(i)?;
        let parent_tree = parent.tree()?;

        let diff = repo.diff_tree_to_tree(Some(&parent_tree), Some(&commit_tree), None)?;
        for delta in diff.deltas() {
            let old = delta.old_file().path().map(|p| p.to_str().unwrap_or(""));
            let new = delta.new_file().path().map(|p| p.to_str().unwrap_or(""));
            if old == Some(file_path) || new == Some(file_path) {
                return Ok(true);
            }
        }
    }

    Ok(false)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup() -> (TempDir, GitEngine) {
        let dir = TempDir::new().unwrap();
        let queue = GitQueue::new();
        let engine = GitEngine::init(dir.path().to_path_buf(), queue).unwrap();
        (dir, engine)
    }

    #[tokio::test]
    async fn test_read_tree_empty_repo() {
        let (_dir, engine) = setup();
        let entries = engine.read_tree("").await.unwrap();
        assert!(entries.is_empty());
    }

    #[tokio::test]
    async fn test_write_and_read_file() {
        let (_dir, engine) = setup();

        let sha = engine
            .write_file(
                "docs/hello.md",
                "# Hello\nWorld",
                "init docs",
                "Test User",
                "test@example.com",
            )
            .await
            .unwrap();

        assert!(!sha.is_empty());

        let doc = engine.read_file("docs/hello.md").await.unwrap();
        assert_eq!(doc.content, "# Hello\nWorld");
        assert_eq!(doc.path, "docs/hello.md");
    }

    #[tokio::test]
    async fn test_read_tree_after_write() {
        let (_dir, engine) = setup();

        engine
            .write_file("README.md", "readme", "add readme", "Author", "a@b.com")
            .await
            .unwrap();
        engine
            .write_file("docs/page.md", "page", "add page", "Author", "a@b.com")
            .await
            .unwrap();

        let entries = engine.read_tree("").await.unwrap();
        let paths: Vec<_> = entries.iter().map(|e| e.path.as_str()).collect();
        assert!(paths.contains(&"README.md"));
        assert!(paths.contains(&"docs"));
        assert!(paths.contains(&"docs/page.md"));
    }

    #[tokio::test]
    async fn test_read_nonexistent_file() {
        let (_dir, engine) = setup();
        engine
            .write_file("a.md", "a", "init", "A", "a@b.com")
            .await
            .unwrap();
        let err = engine.read_file("nonexistent.md").await.unwrap_err();
        assert!(matches!(err, AppError::NotFound(_)));
    }

    #[tokio::test]
    async fn test_delete_file() {
        let (_dir, engine) = setup();

        engine
            .write_file("to_delete.md", "bye", "add", "A", "a@b.com")
            .await
            .unwrap();

        engine
            .delete_file("to_delete.md", "remove", "A", "a@b.com")
            .await
            .unwrap();

        let err = engine.read_file("to_delete.md").await.unwrap_err();
        assert!(matches!(err, AppError::NotFound(_)));
    }

    #[tokio::test]
    async fn test_delete_nonexistent_file() {
        let (_dir, engine) = setup();
        engine
            .write_file("exists.md", "x", "add", "A", "a@b.com")
            .await
            .unwrap();
        let err = engine
            .delete_file("ghost.md", "rm", "A", "a@b.com")
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::NotFound(_)));
    }

    #[tokio::test]
    async fn test_history() {
        let (_dir, engine) = setup();

        engine
            .write_file("doc.md", "v1", "first version", "A", "a@b.com")
            .await
            .unwrap();
        engine
            .write_file("doc.md", "v2", "second version", "A", "a@b.com")
            .await
            .unwrap();

        let hist = engine.history("doc.md", 10).await.unwrap();
        assert_eq!(hist.len(), 2);
        assert_eq!(hist[0].message, "second version");
        assert_eq!(hist[1].message, "first version");
    }

    #[tokio::test]
    async fn test_history_limit() {
        let (_dir, engine) = setup();

        for i in 0..5 {
            engine
                .write_file("doc.md", &format!("v{i}"), &format!("commit {i}"), "A", "a@b.com")
                .await
                .unwrap();
        }

        let hist = engine.history("doc.md", 3).await.unwrap();
        assert_eq!(hist.len(), 3);
    }

    #[tokio::test]
    async fn test_path_traversal_rejected() {
        let (_dir, engine) = setup();
        let err = engine.read_file("../etc/passwd").await.unwrap_err();
        assert!(matches!(err, AppError::BadRequest(_)));
    }

    #[tokio::test]
    async fn test_head_sha_empty_repo() {
        let (_dir, engine) = setup();
        let sha = engine.head_sha().await.unwrap();
        assert!(sha.is_none());
    }

    #[tokio::test]
    async fn test_head_sha_after_commit() {
        let (_dir, engine) = setup();
        let commit_sha = engine
            .write_file("a.md", "a", "init", "A", "a@b.com")
            .await
            .unwrap();
        let head = engine.head_sha().await.unwrap().unwrap();
        assert_eq!(head, commit_sha);
    }

    #[tokio::test]
    async fn test_get_revision_content() {
        let (_dir, engine) = setup();
        let sha = engine
            .write_file("doc.md", "version one", "v1", "A", "a@b.com")
            .await
            .unwrap();
        engine
            .write_file("doc.md", "version two", "v2", "A", "a@b.com")
            .await
            .unwrap();

        let content = engine.get_revision_content("doc.md", &sha).await.unwrap();
        assert_eq!(content, "version one");
    }

    #[tokio::test]
    async fn test_get_revision_content_invalid_sha() {
        let (_dir, engine) = setup();
        engine
            .write_file("doc.md", "content", "init", "A", "a@b.com")
            .await
            .unwrap();
        let err = engine
            .get_revision_content("doc.md", "0000000000000000000000000000000000000000")
            .await
            .unwrap_err();
        assert!(matches!(err, AppError::NotFound(_)));
    }

    #[tokio::test]
    async fn test_restore_revision() {
        let (_dir, engine) = setup();
        let sha = engine
            .write_file("doc.md", "original content", "v1", "A", "a@b.com")
            .await
            .unwrap();
        engine
            .write_file("doc.md", "changed content", "v2", "A", "a@b.com")
            .await
            .unwrap();

        let info = engine
            .restore_revision("doc.md", &sha, "A", "a@b.com")
            .await
            .unwrap();
        assert!(info.message.contains("Restore to"));

        let doc = engine.read_file("doc.md").await.unwrap();
        assert_eq!(doc.content, "original content");
    }
}
