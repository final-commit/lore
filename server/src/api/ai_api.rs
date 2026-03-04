use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};
use crate::auth::AuthUser;
use crate::error::AppError;
use crate::state::AppState;

fn not_configured() -> AppError {
    AppError::Internal("AI not configured — set LORE_AI_API_KEY".into())
}

#[derive(Deserialize)]
pub struct SuggestReq { pub doc_path: String, pub content: String }
#[derive(Deserialize)]
pub struct AnswerReq { pub doc_path: String, pub question: String }
#[derive(Deserialize)]
pub struct SummarizeReq { pub content: String }
#[derive(Deserialize)]
pub struct GenerateReq { pub outline: String }

#[derive(Serialize)]
pub struct AiNotConfigured { pub error: &'static str, pub code: u16 }

pub async fn suggest(State(state): State<AppState>, _user: AuthUser, Json(req): Json<SuggestReq>) -> Result<Json<Vec<crate::ai::AiSuggestion>>, AppError> {
    if !state.ai.is_configured() { return Err(not_configured()); }
    Ok(Json(state.ai.suggest_improvements(&req.content).await?))
}

pub async fn answer(State(state): State<AppState>, _user: AuthUser, Json(req): Json<AnswerReq>) -> Result<Json<serde_json::Value>, AppError> {
    if !state.ai.is_configured() { return Err(not_configured()); }
    let answer = state.ai.answer_question("", &req.question).await?;
    Ok(Json(serde_json::json!({ "answer": answer })))
}

pub async fn summarize(State(state): State<AppState>, _user: AuthUser, Json(req): Json<SummarizeReq>) -> Result<Json<serde_json::Value>, AppError> {
    if !state.ai.is_configured() { return Err(not_configured()); }
    let summary = state.ai.summarize(&req.content).await?;
    Ok(Json(serde_json::json!({ "summary": summary })))
}

pub async fn generate(State(state): State<AppState>, _user: AuthUser, Json(req): Json<GenerateReq>) -> Result<Json<serde_json::Value>, AppError> {
    if !state.ai.is_configured() { return Err(not_configured()); }
    let content = state.ai.generate_from_outline(&req.outline).await?;
    Ok(Json(serde_json::json!({ "content": content })))
}

pub async fn ai_status(State(state): State<AppState>) -> (StatusCode, Json<serde_json::Value>) {
    if state.ai.is_configured() {
        (StatusCode::OK, Json(serde_json::json!({ "configured": true })))
    } else {
        (StatusCode::NOT_IMPLEMENTED, Json(serde_json::json!({ "configured": false, "message": "Set LORE_AI_API_KEY to enable AI features" })))
    }
}
