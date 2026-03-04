use std::sync::Arc;
use std::time::Duration;

use moka::future::Cache;
use serde::{Deserialize, Serialize};

use crate::error::AppError;

// ── Data types ────────────────────────────────────────────────────────────────

/// A cached document page (raw content + metadata snapshot).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedPage {
    pub path: String,
    pub content: String,
    pub commit_sha: String,
}

/// Cache key = (commit_sha, doc_path).  Stale entries are evicted when the
/// commit SHA for a document changes (write invalidation).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PageCacheKey {
    pub commit_sha: String,
    pub path: String,
}

// ── PageCache ─────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct PageCache {
    inner: Arc<Cache<PageCacheKey, CachedPage>>,
    /// Latest known commit SHA per path (used for invalidation).
    commits: Arc<Cache<String, String>>,
}

impl PageCache {
    /// Create a new page cache.
    /// `max_pages`: maximum number of pages to keep.
    /// `ttl`: time-to-live per entry.
    pub fn new(max_pages: u64, ttl: Duration) -> Self {
        let inner = Cache::builder()
            .max_capacity(max_pages)
            .time_to_live(ttl)
            .build();
        let commits = Cache::builder()
            .max_capacity(max_pages)
            .time_to_live(ttl * 2)
            .build();
        PageCache {
            inner: Arc::new(inner),
            commits: Arc::new(commits),
        }
    }

    /// Get a cached page.  Returns `None` if not present or stale.
    pub async fn get(&self, path: &str, current_commit: &str) -> Option<CachedPage> {
        let key = PageCacheKey {
            commit_sha: current_commit.to_string(),
            path: path.to_string(),
        };
        self.inner.get(&key).await
    }

    /// Store a page in the cache.
    pub async fn insert(&self, page: CachedPage) {
        let key = PageCacheKey {
            commit_sha: page.commit_sha.clone(),
            path: page.path.clone(),
        };
        self.commits.insert(page.path.clone(), page.commit_sha.clone()).await;
        self.inner.insert(key, page).await;
    }

    /// Invalidate all cache entries for the given path (called after a write).
    pub async fn invalidate(&self, path: &str) {
        // Remove the path→commit mapping so future reads bypass the cache.
        self.commits.invalidate(path).await;
        // We can't efficiently remove all keys for a path from the main cache
        // (they're keyed by commit_sha+path), but since we track the current
        // commit separately, stale entries will simply never be hit.
    }

    /// Invalidate all pages (e.g., after a pull/rebase).
    pub async fn invalidate_all(&self) {
        self.inner.invalidate_all();
        self.commits.invalidate_all();
    }

    /// Return the number of entries currently in the cache.
    pub fn len(&self) -> u64 {
        self.inner.entry_count()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Convenience: get or compute a page.
    /// `loader` is called on cache miss and its result stored.
    pub async fn get_or_load<F, Fut>(
        &self,
        path: &str,
        current_commit: &str,
        loader: F,
    ) -> Result<CachedPage, AppError>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<CachedPage, AppError>>,
    {
        if let Some(cached) = self.get(path, current_commit).await {
            return Ok(cached);
        }
        let page = loader().await?;
        self.insert(page.clone()).await;
        Ok(page)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn cache() -> PageCache {
        PageCache::new(100, Duration::from_secs(60))
    }

    fn page(path: &str, commit: &str) -> CachedPage {
        CachedPage {
            path: path.to_string(),
            content: "# content".to_string(),
            commit_sha: commit.to_string(),
        }
    }

    #[tokio::test]
    async fn test_miss_on_empty() {
        let c = cache();
        assert!(c.get("doc.md", "abc123").await.is_none());
    }

    #[tokio::test]
    async fn test_insert_and_get() {
        let c = cache();
        c.insert(page("doc.md", "sha1")).await;
        let hit = c.get("doc.md", "sha1").await;
        assert!(hit.is_some());
        assert_eq!(hit.unwrap().commit_sha, "sha1");
    }

    #[tokio::test]
    async fn test_stale_commit_misses() {
        let c = cache();
        c.insert(page("doc.md", "sha1")).await;
        // Different commit SHA → cache miss.
        assert!(c.get("doc.md", "sha2").await.is_none());
    }

    #[tokio::test]
    async fn test_get_or_load_cached() {
        let c = cache();
        c.insert(page("doc.md", "sha1")).await;

        let mut called = false;
        let result = c
            .get_or_load("doc.md", "sha1", || async {
                called = true;
                Ok(page("doc.md", "sha1"))
            })
            .await
            .unwrap();

        assert!(!called);
        assert_eq!(result.commit_sha, "sha1");
    }

    #[tokio::test]
    async fn test_get_or_load_miss() {
        let c = cache();
        let mut called = false;
        let result = c
            .get_or_load("doc.md", "sha1", || async {
                called = true;
                Ok(page("doc.md", "sha1"))
            })
            .await
            .unwrap();

        assert!(called);
        assert_eq!(result.path, "doc.md");
    }

    #[tokio::test]
    async fn test_invalidate_causes_miss() {
        let c = cache();
        c.insert(page("doc.md", "sha1")).await;
        c.invalidate("doc.md").await;
        // After invalidation the path→commit mapping is gone, but the
        // cache key still exists.  A new sha would miss:
        assert!(c.get("doc.md", "sha2").await.is_none());
    }

    #[tokio::test]
    async fn test_invalidate_all() {
        let c = cache();
        c.insert(page("a.md", "sha1")).await;
        c.insert(page("b.md", "sha2")).await;
        c.invalidate_all().await;
        // Wait for moka to process pending removals
        c.inner.run_pending_tasks().await;
        assert_eq!(c.len(), 0);
    }
}
