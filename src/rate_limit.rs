//! Simple in-memory per-key rate limiter for auth endpoints.
//!
//! Uses a moka cache with a 60-second TTL so each key's counter resets after
//! one minute.  Not suitable for distributed deployments (no Redis backing),
//! but adequate for single-process protection against brute-force.

use moka::future::Cache;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// Per-key rate limiter: allows up to `limit` requests per 60-second window.
#[derive(Clone)]
pub struct RateLimiter {
    cache: Cache<String, Arc<AtomicU32>>,
    limit: u32,
}

impl RateLimiter {
    pub fn new(limit: u32) -> Self {
        RateLimiter {
            cache: Cache::builder()
                .max_capacity(10_000)
                .time_to_live(Duration::from_secs(60))
                .build(),
            limit,
        }
    }

    /// Returns `true` if the request is allowed, `false` if rate-limited.
    pub async fn check(&self, key: &str) -> bool {
        let counter = self
            .cache
            .get_with(key.to_string(), async { Arc::new(AtomicU32::new(0)) })
            .await;
        let prev = counter.fetch_add(1, Ordering::Relaxed);
        prev < self.limit
    }
}
