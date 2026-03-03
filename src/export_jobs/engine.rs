use chrono::Utc;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;
use crate::db::{with_conn, DbConn};
use crate::error::AppError;
use crate::git::GitEngine;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportJob {
    pub id: String,
    pub user_id: String,
    pub job_type: String,
    pub status: String,
    pub file_path: Option<String>,
    pub error: Option<String>,
    pub created_at: String,
    pub completed_at: Option<String>,
}

#[derive(Clone)]
pub struct ExportJobEngine {
    db: DbConn,
    git: GitEngine,
    export_dir: PathBuf,
}

impl ExportJobEngine {
    pub fn new(db: DbConn, git: GitEngine, export_dir: PathBuf) -> Self {
        ExportJobEngine { db, git, export_dir }
    }

    pub async fn create(&self, user_id: &str, job_type: &str, collection_id: Option<&str>) -> Result<ExportJob, AppError> {
        let id = Uuid::now_v7().to_string();
        let now = Utc::now().to_rfc3339();
        let db = self.db.clone();
        let uid = user_id.to_string();
        let jtype = job_type.to_string();
        let eid = id.clone();
        let n = now.clone();
        with_conn(&db, move |conn| {
            conn.execute(
                "INSERT INTO export_jobs (id, user_id, job_type, status, created_at) VALUES (?1,?2,?3,'pending',?4)",
                params![eid, uid, jtype, n],
            )?;
            conn.query_row(
                "SELECT id, user_id, job_type, status, file_path, error, created_at, completed_at FROM export_jobs WHERE id=?1",
                params![eid],
                row_to_job,
            )
        }).await?;

        // Spawn background task
        let job_id = id.clone();
        let engine = self.clone();
        let col_id = collection_id.map(str::to_string);
        tokio::spawn(async move {
            if let Err(e) = engine.run_job(&job_id, col_id.as_deref()).await {
                let _ = engine.mark_failed(&job_id, &e.to_string()).await;
            }
        });

        self.get(&id).await
    }

    pub async fn get(&self, id: &str) -> Result<ExportJob, AppError> {
        let db = self.db.clone();
        let eid = id.to_string();
        with_conn(&db, move |conn| {
            conn.query_row(
                "SELECT id, user_id, job_type, status, file_path, error, created_at, completed_at FROM export_jobs WHERE id=?1",
                params![eid],
                row_to_job,
            )
        }).await.map_err(|_| AppError::NotFound("export job not found".into()))
    }

    pub async fn get_file_path(&self, id: &str) -> Result<PathBuf, AppError> {
        let job = self.get(id).await?;
        if job.status != "complete" {
            return Err(AppError::BadRequest(format!("job status is {}", job.status)));
        }
        let fname = job.file_path.ok_or_else(|| AppError::Internal("no file".into()))?;
        Ok(self.export_dir.join(fname))
    }

    async fn run_job(&self, job_id: &str, collection_id: Option<&str>) -> Result<(), AppError> {
        std::fs::create_dir_all(&self.export_dir).map_err(|e| AppError::Internal(e.to_string()))?;
        let filename = format!("{job_id}.zip");
        let out_path = self.export_dir.join(&filename);

        // Walk repo directory and collect .md files
        let repo_path = self.git.repo_path.clone();
        let col_prefix = collection_id.map(str::to_string);
        let out_path2 = out_path.clone();

        tokio::task::spawn_blocking(move || -> Result<(), String> {
            let file = std::fs::File::create(&out_path2).map_err(|e| e.to_string())?;
            let mut zip_writer = zip::ZipWriter::new(file);
            let opts = zip::write::SimpleFileOptions::default();
            collect_md_files(&repo_path, &repo_path, &col_prefix, &mut zip_writer, opts);
            zip_writer.finish().map_err(|e| e.to_string())?;
            Ok(())
        }).await
            .map_err(|e| AppError::Internal(e.to_string()))?
            .map_err(AppError::Internal)?;

        self.mark_complete(job_id, &filename).await
    }

    async fn mark_complete(&self, id: &str, filename: &str) -> Result<(), AppError> {
        let db = self.db.clone();
        let eid = id.to_string();
        let fname = filename.to_string();
        let now = Utc::now().to_rfc3339();
        with_conn(&db, move |conn| {
            conn.execute(
                "UPDATE export_jobs SET status='complete', file_path=?1, completed_at=?2 WHERE id=?3",
                params![fname, now, eid],
            )?;
            Ok(())
        }).await
    }

    async fn mark_failed(&self, id: &str, error: &str) -> Result<(), AppError> {
        let db = self.db.clone();
        let eid = id.to_string();
        let err = error.to_string();
        let now = Utc::now().to_rfc3339();
        with_conn(&db, move |conn| {
            conn.execute(
                "UPDATE export_jobs SET status='failed', error=?1, completed_at=?2 WHERE id=?3",
                params![err, now, eid],
            )?;
            Ok(())
        }).await
    }
}

fn collect_md_files(
    base: &PathBuf,
    dir: &PathBuf,
    col_prefix: &Option<String>,
    zip: &mut zip::ZipWriter<std::fs::File>,
    opts: zip::write::SimpleFileOptions,
) {
    use std::io::Write;
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
        if name.starts_with('.') || name.starts_with('_') { continue; }
        if path.is_dir() {
            collect_md_files(base, &path, col_prefix, zip, opts);
        } else if path.extension().and_then(|e| e.to_str()) == Some("md") {
            let rel = path.strip_prefix(base).unwrap_or(&path);
            let rel_str = rel.to_string_lossy().to_string();
            if let Some(prefix) = &col_prefix {
                if !rel_str.starts_with(prefix.as_str()) { continue; }
            }
            if let Ok(content) = std::fs::read_to_string(&path) {
                let _ = zip.start_file(&rel_str, opts);
                let _ = zip.write_all(content.as_bytes());
            }
        }
    }
}

fn row_to_job(r: &rusqlite::Row) -> rusqlite::Result<ExportJob> {
    Ok(ExportJob {
        id: r.get(0)?, user_id: r.get(1)?, job_type: r.get(2)?,
        status: r.get(3)?, file_path: r.get(4)?, error: r.get(5)?,
        created_at: r.get(6)?, completed_at: r.get(7)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{db, git::{GitEngine, GitQueue}};
    use tempfile::tempdir;

    fn setup() -> (ExportJobEngine, tempfile::TempDir) {
        let repo_dir = tempdir().unwrap();
        let export_dir = tempdir().unwrap();
        let git = GitEngine::init(repo_dir.path().to_path_buf(), GitQueue::new()).unwrap();
        let db_conn = db::open(":memory:").unwrap();
        { let conn = db_conn.lock().unwrap();
          conn.execute("INSERT INTO users (id,email,name,password_hash,role,created_at,updated_at) VALUES ('u1','a@b.com','A','h','admin','2024-01-01','2024-01-01')", []).unwrap(); }
        let engine = ExportJobEngine::new(db_conn, git, export_dir.path().to_path_buf());
        (engine, export_dir)
    }

    #[tokio::test]
    async fn test_create_job_pending() {
        let (e, _dir) = setup();
        let job = e.create("u1", "collection-zip", None).await.unwrap();
        assert_eq!(job.user_id, "u1");
        assert!(job.status == "pending" || job.status == "complete");
    }

    #[tokio::test]
    async fn test_get_job() {
        let (e, _dir) = setup();
        let job = e.create("u1", "collection-zip", None).await.unwrap();
        let fetched = e.get(&job.id).await.unwrap();
        assert_eq!(fetched.id, job.id);
    }
}
