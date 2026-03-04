use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use crate::error::AppError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiSuggestion {
    pub suggestion_type: String,  // "improvement", "grammar", "clarity"
    pub text: String,
    pub original: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Debug, Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: Vec<ChatMessage>,
    max_tokens: u32,
    temperature: f32,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatMessage,
}

#[derive(Clone)]
pub struct AiEngine {
    client: Client,
    api_key: Option<String>,
    base_url: String,
    model: String,
}

impl AiEngine {
    pub fn new(api_key: Option<String>, base_url: String, model: String) -> Self {
        AiEngine {
            client: Client::builder().timeout(Duration::from_secs(30)).build().unwrap_or_default(),
            api_key,
            base_url,
            model,
        }
    }

    pub fn is_configured(&self) -> bool {
        self.api_key.is_some()
    }

    pub async fn suggest_improvements(&self, content: &str) -> Result<Vec<AiSuggestion>, AppError> {
        let prompt = format!(
            "Review this documentation and suggest specific improvements. For each suggestion output a JSON array of objects with fields: suggestion_type (improvement/grammar/clarity), text (your suggestion), original (the text to replace, if applicable). Output ONLY valid JSON array.\n\nDocument:\n{content}"
        );
        let response = self.complete(&prompt).await?;
        serde_json::from_str::<Vec<AiSuggestion>>(&response)
            .map_err(|_| AppError::Internal("AI returned invalid JSON".into()))
    }

    pub async fn answer_question(&self, doc_content: &str, question: &str) -> Result<String, AppError> {
        let prompt = format!(
            "You are a helpful assistant. Answer the following question based on this documentation.\n\nDocumentation:\n{doc_content}\n\nQuestion: {question}\n\nAnswer concisely:"
        );
        self.complete(&prompt).await
    }

    pub async fn summarize(&self, content: &str) -> Result<String, AppError> {
        let prompt = format!(
            "Summarize the following documentation in 2-3 sentences. Be concise and clear.\n\n{content}"
        );
        self.complete(&prompt).await
    }

    pub async fn generate_from_outline(&self, outline: &str) -> Result<String, AppError> {
        let prompt = format!(
            "You are a technical writer. Generate well-structured Markdown documentation from this outline. Use proper headings, lists, and code blocks where appropriate.\n\nOutline:\n{outline}\n\nMarkdown documentation:"
        );
        self.complete(&prompt).await
    }

    async fn complete(&self, prompt: &str) -> Result<String, AppError> {
        let api_key = self.api_key.as_ref()
            .ok_or_else(|| AppError::Internal("AI not configured".into()))?;

        let req = ChatRequest {
            model: &self.model,
            messages: vec![ChatMessage { role: "user".into(), content: prompt.into() }],
            max_tokens: 1024,
            temperature: 0.3,
        };

        let resp = self.client
            .post(format!("{}/chat/completions", self.base_url))
            .bearer_auth(api_key)
            .json(&req)
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("AI request failed: {e}")))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(AppError::Internal(format!("AI API error {status}: {body}")));
        }

        let data: ChatResponse = resp.json().await
            .map_err(|e| AppError::Internal(format!("AI response parse: {e}")))?;

        data.choices.into_iter().next()
            .map(|c| c.message.content.trim().to_string())
            .ok_or_else(|| AppError::Internal("AI returned no choices".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unconfigured_engine() -> AiEngine {
        AiEngine::new(None, "https://api.openai.com/v1".into(), "gpt-4o-mini".into())
    }

    #[test]
    fn test_not_configured_when_no_key() {
        let e = unconfigured_engine();
        assert!(!e.is_configured());
    }

    #[test]
    fn test_configured_with_key() {
        let e = AiEngine::new(Some("test-key".into()), "https://api.openai.com/v1".into(), "gpt-4o-mini".into());
        assert!(e.is_configured());
    }

    #[tokio::test]
    async fn test_complete_returns_err_without_key() {
        let e = unconfigured_engine();
        let result = e.complete("test prompt").await;
        assert!(result.is_err());
    }
}
