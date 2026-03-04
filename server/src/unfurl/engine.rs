use moka::future::Cache;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use crate::error::AppError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnfurlResult {
    pub url: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub image: Option<String>,
    pub embed_html: Option<String>,
    pub provider: Option<String>,
}

#[derive(Clone)]
pub struct UnfurlEngine {
    client: Client,
    cache: Cache<String, UnfurlResult>,
}

impl Default for UnfurlEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl UnfurlEngine {
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(5))
            .user_agent("Forge/1.0 (+https://github.com/forge)")
            .build()
            .unwrap_or_default();
        let cache = Cache::builder()
            .max_capacity(1000)
            .time_to_live(Duration::from_secs(3600))
            .build();
        UnfurlEngine { client, cache }
    }

    pub async fn unfurl(&self, url: &str) -> Result<UnfurlResult, AppError> {
        if let Some(cached) = self.cache.get(url).await {
            return Ok(cached);
        }
        let result = self.fetch(url).await?;
        self.cache.insert(url.to_string(), result.clone()).await;
        Ok(result)
    }

    async fn fetch(&self, url: &str) -> Result<UnfurlResult, AppError> {
        // Try embed providers first
        if let Some(r) = detect_embed(url) {
            return Ok(r);
        }
        // Fall back to OG scraping
        let resp = self.client.get(url).send().await
            .map_err(|e| AppError::Internal(format!("unfurl fetch: {e}")))?;
        let html = resp.text().await
            .map_err(|e| AppError::Internal(format!("unfurl body: {e}")))?;
        Ok(parse_og(url, &html))
    }
}

fn detect_embed(url: &str) -> Option<UnfurlResult> {
    // YouTube
    let yt_id = extract_youtube_id(url)?;
    Some(UnfurlResult {
        url: url.to_string(),
        title: Some("YouTube Video".to_string()),
        description: None,
        image: Some(format!("https://img.youtube.com/vi/{yt_id}/hqdefault.jpg")),
        embed_html: Some(format!(
            r#"<iframe width="560" height="315" src="https://www.youtube.com/embed/{yt_id}" frameborder="0" allowfullscreen></iframe>"#
        )),
        provider: Some("youtube".into()),
    })
}

fn extract_youtube_id(url: &str) -> Option<String> {
    if url.contains("youtube.com/watch") {
        url.split("v=").nth(1).map(|s| s.split('&').next().unwrap_or(s).to_string())
    } else if url.contains("youtu.be/") {
        url.split("youtu.be/").nth(1).map(|s| s.split('?').next().unwrap_or(s).to_string())
    } else {
        None
    }
}

fn parse_og(url: &str, html: &str) -> UnfurlResult {
    let title = extract_meta(html, "og:title")
        .or_else(|| extract_meta(html, "twitter:title"))
        .or_else(|| extract_tag(html, "title"));
    let description = extract_meta(html, "og:description")
        .or_else(|| extract_meta(html, "twitter:description"));
    let image = extract_meta(html, "og:image")
        .or_else(|| extract_meta(html, "twitter:image"));

    // Detect provider from domain
    let provider = if url.contains("github.com") { Some("github".into()) }
        else if url.contains("figma.com") { Some("figma".into()) }
        else if url.contains("loom.com") { Some("loom".into()) }
        else if url.contains("vimeo.com") { Some("vimeo".into()) }
        else { None };

    UnfurlResult { url: url.to_string(), title, description, image, embed_html: None, provider }
}

fn extract_meta(html: &str, property: &str) -> Option<String> {
    let patterns = [
        format!(r#"property="{property}" content=""#),
        format!(r#"name="{property}" content=""#),
        format!(r#"property='{property}' content='"#),
    ];
    for pat in &patterns {
        if let Some(start) = html.find(pat.as_str()) {
            let rest = &html[start + pat.len()..];
            let end = rest.find('"').or_else(|| rest.find('\''))?;
            return Some(rest[..end].trim().to_string());
        }
    }
    None
}

fn extract_tag(html: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let start = html.find(&open)? + open.len();
    let end = html[start..].find(&close)?;
    Some(html[start..start + end].trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_youtube_id_watch() {
        let id = extract_youtube_id("https://www.youtube.com/watch?v=dQw4w9WgXcQ&t=10s");
        assert_eq!(id, Some("dQw4w9WgXcQ".into()));
    }

    #[test]
    fn test_youtube_id_short() {
        let id = extract_youtube_id("https://youtu.be/dQw4w9WgXcQ?t=10");
        assert_eq!(id, Some("dQw4w9WgXcQ".into()));
    }

    #[test]
    fn test_detect_youtube_embed() {
        let r = detect_embed("https://www.youtube.com/watch?v=abc123").unwrap();
        assert_eq!(r.provider, Some("youtube".into()));
        assert!(r.embed_html.unwrap().contains("abc123"));
    }

    #[test]
    fn test_parse_og() {
        let html = r#"<html><head><meta property="og:title" content="Hello World"/><meta property="og:description" content="A test"/></head></html>"#;
        let r = parse_og("https://example.com", html);
        assert_eq!(r.title, Some("Hello World".into()));
        assert_eq!(r.description, Some("A test".into()));
    }

    #[test]
    fn test_github_provider() {
        let r = parse_og("https://github.com/rust-lang/rust", "");
        assert_eq!(r.provider, Some("github".into()));
    }
}
