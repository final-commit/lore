pub mod schema;

use rusqlite::Connection;
use std::sync::{Arc, Mutex};

pub type DbConn = Arc<Mutex<Connection>>;

/// Open (or create) a SQLite database at the given path and apply migrations.
/// Pass `":memory:"` for an in-memory database (tests).
pub fn open(path: &str) -> rusqlite::Result<DbConn> {
    let conn = if path == ":memory:" {
        Connection::open_in_memory()?
    } else {
        Connection::open(path)?
    };

    conn.execute_batch(
        "PRAGMA journal_mode=WAL;
         PRAGMA foreign_keys=ON;
         PRAGMA synchronous=NORMAL;
         PRAGMA busy_timeout=5000;",
    )?;

    schema::apply_migrations(&conn)?;

    Ok(Arc::new(Mutex::new(conn)))
}

/// Execute a closure with a locked database connection on a blocking thread.
/// Returns the closure's result, mapping panics to an Internal error string.
pub async fn with_conn<F, T>(db: &DbConn, f: F) -> rusqlite::Result<T>
where
    F: FnOnce(&Connection) -> rusqlite::Result<T> + Send + 'static,
    T: Send + 'static,
{
    let db = db.clone();
    tokio::task::spawn_blocking(move || {
        let conn = db.lock().expect("db mutex poisoned");
        f(&conn)
    })
    .await
    .expect("blocking task panicked")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_open_in_memory() {
        let db = open(":memory:").expect("should open in-memory DB");
        let conn = db.lock().unwrap();
        let n: i64 = conn
            .query_row("SELECT COUNT(*) FROM users", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 0);
    }

    #[test]
    fn test_open_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.db");
        let db = open(path.to_str().unwrap()).expect("should open file DB");
        let conn = db.lock().unwrap();
        let n: i64 = conn
            .query_row("SELECT COUNT(*) FROM users", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 0);
    }

    #[tokio::test]
    async fn test_with_conn() {
        let db = open(":memory:").unwrap();
        let count = with_conn(&db, |conn| {
            conn.query_row("SELECT COUNT(*) FROM users", [], |r| r.get::<_, i64>(0))
        })
        .await
        .unwrap();
        assert_eq!(count, 0);
    }
}
