use axum::{
    body::Body,
    extract::{Multipart, Path, State},
    http::{header, StatusCode},
    response::Response,
    Json,
};

use crate::attachments::Attachment;
use crate::auth::AuthUser;
use crate::error::AppError;
use crate::state::AppState;

/// POST /api/attachments/upload  (multipart/form-data)
///
/// Form fields:
///   - `doc_path`  (text)  — path of the associated document
///   - `file`      (file)  — the file to upload
pub async fn upload_attachment(
    State(state): State<AppState>,
    user: AuthUser,
    mut multipart: Multipart,
) -> Result<(StatusCode, Json<Attachment>), AppError> {
    let mut doc_path: Option<String> = None;
    let mut filename: Option<String> = None;
    let mut content_type: Option<String> = None;
    let mut data: Option<Vec<u8>> = None;

    while let Some(field) =
        multipart.next_field().await.map_err(|e| AppError::BadRequest(e.to_string()))?
    {
        let field_name = field.name().unwrap_or("").to_string();
        match field_name.as_str() {
            "doc_path" => {
                doc_path = Some(
                    field.text().await.map_err(|e| AppError::BadRequest(e.to_string()))?,
                );
            }
            "file" => {
                filename = field.file_name().map(|s| s.to_string());
                content_type = field.content_type().map(|s| s.to_string());
                let bytes =
                    field.bytes().await.map_err(|e| AppError::BadRequest(e.to_string()))?;
                data = Some(bytes.to_vec());
            }
            _ => {}
        }
    }

    let doc_path =
        doc_path.ok_or_else(|| AppError::BadRequest("doc_path field is required".into()))?;
    let data = data.ok_or_else(|| AppError::BadRequest("file field is required".into()))?;
    let filename = filename.unwrap_or_else(|| "upload".to_string());
    let content_type = content_type.unwrap_or_else(|| "application/octet-stream".to_string());

    let attachment =
        state.attachments.upload(&doc_path, &filename, &content_type, data, &user.id).await?;

    Ok((StatusCode::CREATED, Json(attachment)))
}

/// GET /api/attachments/{id}
pub async fn get_attachment(
    State(state): State<AppState>,
    _user: AuthUser,
    Path(id): Path<String>,
) -> Result<Response, AppError> {
    let (meta, bytes) = state.attachments.read_bytes(&id).await?;

    let content_len = bytes.len().to_string();
    // Sanitize filename to prevent Content-Disposition header injection
    let safe_filename = meta.filename
        .replace('"', "")
        .replace('\n', "")
        .replace('\r', "");
    let disposition = format!("inline; filename=\"{safe_filename}\"");

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, meta.content_type)
        .header(header::CONTENT_DISPOSITION, disposition)
        .header(header::CONTENT_LENGTH, content_len)
        .body(Body::from(bytes))
        .map_err(|e| AppError::Internal(format!("response builder: {e}")))
}
