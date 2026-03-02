use std::sync::Arc;
use tokio::sync::Mutex;

use crate::error::AppError;

/// Serialises all git operations through a single async mutex so concurrent
/// HTTP handlers never interleave git writes.
#[derive(Clone)]
pub struct GitQueue {
    lock: Arc<Mutex<()>>,
}

impl GitQueue {
    pub fn new() -> Self {
        GitQueue {
            lock: Arc::new(Mutex::new(())),
        }
    }

    /// Run a blocking closure exclusively, serialised behind the queue lock.
    /// The closure is executed on a tokio blocking thread.
    pub async fn run<F, T>(&self, f: F) -> Result<T, AppError>
    where
        F: FnOnce() -> Result<T, AppError> + Send + 'static,
        T: Send + 'static,
    {
        let _guard = self.lock.lock().await;
        tokio::task::spawn_blocking(f)
            .await
            .map_err(|e| AppError::Internal(format!("git task panicked: {e}")))?
    }
}

impl Default for GitQueue {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc as StdArc;

    #[tokio::test]
    async fn test_queue_serialises_tasks() {
        let queue = GitQueue::new();
        let counter = StdArc::new(AtomicUsize::new(0));

        let mut handles = vec![];
        for _ in 0..5 {
            let q = queue.clone();
            let c = counter.clone();
            let h = tokio::spawn(async move {
                q.run(move || {
                    c.fetch_add(1, Ordering::SeqCst);
                    Ok::<_, AppError>(())
                })
                .await
            });
            handles.push(h);
        }

        for h in handles {
            h.await.unwrap().unwrap();
        }

        assert_eq!(counter.load(Ordering::SeqCst), 5);
    }

    #[tokio::test]
    async fn test_queue_propagates_error() {
        let queue = GitQueue::new();
        let result = queue
            .run(|| Err::<(), _>(AppError::Internal("test error".into())))
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("test error"));
    }

    #[tokio::test]
    async fn test_queue_returns_value() {
        let queue = GitQueue::new();
        let val = queue.run(|| Ok::<_, AppError>(42)).await.unwrap();
        assert_eq!(val, 42);
    }
}
