//! Database backend abstraction.
//!
//! `DbBackend` is the async trait that all database backends must implement.
//! It captures the two core primitives of the database layer:
//! - Opening (or creating) a database at a given path and applying migrations.
//! - Executing a blocking closure against a live connection.
//!
//! ## Backends
//!
//! | Backend            | Status              | Notes                          |
//! |--------------------|---------------------|--------------------------------|
//! | `SqliteBackend`    | Full (default)      | Wraps `Arc<Mutex<Connection>>` |
//! | `PostgresBackend`  | Scaffold (Phase 5)  | All methods return errors      |
//!
//! ## Design note (D-004)
//!
//! Native async trait (Rust 1.75+) is used — zero overhead vs `async_trait`'s
//! per-call heap allocations. The trait is not currently object-safe
//! (`async fn` returns an opaque future); `dyn DbBackend` dispatch is deferred
//! to Phase 5 when the Postgres backend is activated and we switch to
//! `#[trait_variant::make(DbBackendSend)]` or manual boxing.

use rusqlite::Connection;

use crate::db::DbConn;
use crate::error::AppError;

// ── Trait ────────────────────────────────────────────────────────────────────

/// Core database backend trait.
///
/// All database I/O ultimately routes through an implementation of this trait.
/// Handlers and engines do not call it directly — they use the `DbConn` /
/// `with_conn` helpers that `SqliteBackend` exposes.  The trait exists so that
/// `PostgresBackend` can slot in during Phase 5 without touching handler code.
pub trait DbBackend: Send + Sync + 'static {
    /// Open the database at `path` and apply all migrations.
    ///
    /// Pass `":memory:"` for an in-process test database.
    fn open(path: &str) -> rusqlite::Result<Self>
    where
        Self: Sized;

    /// Execute a closure with a live database connection on a blocking thread.
    ///
    /// Returns `AppError::Db` for SQLite errors returned by the closure, or
    /// `AppError::Internal` if the backend is unavailable (e.g. Postgres stub).
    fn with_conn<F, T>(
        &self,
        f: F,
    ) -> impl std::future::Future<Output = Result<T, AppError>> + Send
    where
        F: FnOnce(&Connection) -> rusqlite::Result<T> + Send + 'static,
        T: Send + 'static;
}

// ── SqliteBackend ─────────────────────────────────────────────────────────────

/// SQLite backend — the default for Lore.
///
/// Wraps `DbConn` (`Arc<Mutex<rusqlite::Connection>>`).  All I/O is dispatched
/// via `tokio::task::spawn_blocking` so async callers are never blocked.
pub struct SqliteBackend {
    conn: DbConn,
}

impl DbBackend for SqliteBackend {
    fn open(path: &str) -> rusqlite::Result<Self> {
        let conn = super::open(path)?;
        Ok(Self { conn })
    }

    async fn with_conn<F, T>(&self, f: F) -> Result<T, AppError>
    where
        F: FnOnce(&Connection) -> rusqlite::Result<T> + Send + 'static,
        T: Send + 'static,
    {
        super::with_conn(&self.conn, f).await
    }
}

impl SqliteBackend {
    /// Return the underlying `DbConn` handle.
    ///
    /// Engines that were written before the `DbBackend` trait existed still
    /// accept `DbConn` directly; this accessor bridges the two layers.
    pub fn conn(&self) -> &DbConn {
        &self.conn
    }
}

// ── PostgresBackend ───────────────────────────────────────────────────────────

/// PostgreSQL backend scaffold — **not yet implemented**.
///
/// Activate in Phase 5.  Until then, every method returns an error so that
/// any accidental use is caught at runtime rather than silently ignored.
pub struct PostgresBackend;

impl DbBackend for PostgresBackend {
    fn open(_path: &str) -> rusqlite::Result<Self> {
        unimplemented!("PostgresBackend is not enabled — activate in Phase 5")
    }

    async fn with_conn<F, T>(&self, _f: F) -> Result<T, AppError>
    where
        F: FnOnce(&Connection) -> rusqlite::Result<T> + Send + 'static,
        T: Send + 'static,
    {
        Err(AppError::Internal(
            "PostgresBackend is not enabled — activate in Phase 5".into(),
        ))
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sqlite_backend_open_memory() {
        let backend = SqliteBackend::open(":memory:").expect("should open in-memory");
        let conn = backend.conn();
        let c = conn.lock().unwrap();
        let n: i64 = c
            .query_row("SELECT COUNT(*) FROM users", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 0);
    }

    #[tokio::test]
    async fn sqlite_backend_with_conn() {
        let backend = SqliteBackend::open(":memory:").unwrap();
        let count = backend
            .with_conn(|conn| conn.query_row("SELECT COUNT(*) FROM users", [], |r| r.get::<_, i64>(0)))
            .await
            .unwrap();
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn postgres_backend_with_conn_returns_error() {
        let backend = PostgresBackend;
        let result = backend
            .with_conn(|_conn| Ok::<i64, rusqlite::Error>(1))
            .await;
        assert!(result.is_err());
        if let Err(AppError::Internal(msg)) = result {
            assert!(msg.contains("Phase 5"));
        } else {
            panic!("expected AppError::Internal");
        }
    }
}
