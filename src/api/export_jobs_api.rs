use axum::{extract::{Path, State}, http::StatusCode, Json};
use serde::Deserialize;
use crate::auth::AuthUser;
use crate::error::AppError;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct CreateExportJob { pub job_type: String, pub collection_id: Option<String> }

pub async fn create_export_job(State(state): State<AppState>, user: AuthUser, Json(req): Json<CreateExportJob>) -> Result<(StatusCode, Json<crate::export_jobs::ExportJob>), AppError> {
    let job = state.export_jobs.create(&user.id, &req.job_type, req.collection_id.as_deref()).await?;
    Ok((StatusCode::ACCEPTED, Json(job)))
}

pub async fn get_export_job(State(state): State<AppState>, user: AuthUser, Path(id): Path<String>) -> Result<Json<crate::export_jobs::ExportJob>, AppError> {
    let job = state.export_jobs.get(&id).await?;
    if job.user_id != user.id && !user.is_admin() { return Err(AppError::Forbidden("not your job".into())); }
    Ok(Json(job))
}

pub async fn download_export_job(State(state): State<AppState>, user: AuthUser, Path(id): Path<String>) -> Result<axum::response::Response, AppError> {
    let job = state.export_jobs.get(&id).await?;
    if job.user_id != user.id && !user.is_admin() { return Err(AppError::Forbidden("not your job".into())); }
    let path = state.export_jobs.get_file_path(&id).await?;
    let bytes = tokio::fs::read(&path).await.map_err(|e| AppError::Internal(e.to_string()))?;
    use axum::http::header;
    use axum::response::IntoResponse;
    Ok((
        [(header::CONTENT_TYPE, "application/zip"), (header::CONTENT_DISPOSITION, "attachment; filename=\"export.zip\"")],
        bytes,
    ).into_response())
}
