use serde::{Deserialize, Serialize};
use crate::git::engine::GitEngine;
use crate::error::AppError;

#[derive(Debug, Serialize)]
pub struct ImportResult {
    pub imported: usize,
    pub errors: Vec<ImportError>,
}

#[derive(Debug, Serialize)]
pub struct ImportError {
    pub path: String,
    pub reason: String,
}

/// Outline export format (simplified — only fields we care about).
#[derive(Debug, Deserialize)]
struct OutlineDoc {
    pub title: Option<String>,
    pub text: Option<String>,
    #[serde(rename = "urlId")]
    pub url_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OutlineExport {
    pub documents: Option<Vec<OutlineDoc>>,
}

#[derive(Clone)]
pub struct ImportEngine {
    pub git: GitEngine,
}

impl ImportEngine {
    pub fn new(git: GitEngine) -> Self { ImportEngine { git } }

    /// Import from Outline JSON export bytes.
    pub async fn from_outline_json(
        &self,
        data: &[u8],
        author_email: &str,
    ) -> Result<ImportResult, AppError> {
        let export: OutlineExport = serde_json::from_slice(data)
            .map_err(|e| AppError::BadRequest(format!("invalid Outline export JSON: {e}")))?;

        let docs = export.documents.unwrap_or_default();
        let mut imported = 0;
        let mut errors = vec![];

        for doc in docs {
            let title = doc.title.unwrap_or_else(|| "Untitled".into());
            let content = doc.text.unwrap_or_default();
            let slug = slugify(&title);
            let path = format!("imported/{slug}.md");

            // Prepend title as H1 if not already present
            let full_content = if content.trim_start().starts_with("# ") {
                content
            } else {
                format!("# {title}\n\n{content}")
            };

            match self.git.write_file(&path, &full_content, &format!("Import: {title}"), author_email, author_email).await {
                Ok(_) => imported += 1,
                Err(e) => errors.push(ImportError { path, reason: e.to_string() }),
            }
        }

        Ok(ImportResult { imported, errors })
    }

    /// Import from a zip of markdown files.
    pub async fn from_markdown_zip(
        &self,
        data: &[u8],
        author_email: &str,
    ) -> Result<ImportResult, AppError> {
        use std::io::Read;
        let cursor = std::io::Cursor::new(data);
        let mut archive = zip::ZipArchive::new(cursor)
            .map_err(|e| AppError::BadRequest(format!("invalid zip file: {e}")))?;

        let mut imported = 0;
        let mut errors = vec![];

        // Collect all files first (before any .await) to avoid holding !Send ZipFile across await.
        let mut file_entries: Vec<(String, String)> = vec![];
        for i in 0..archive.len() {
            let mut file = archive.by_index(i)
                .map_err(|e| AppError::Internal(e.to_string()))?;

            let name = file.name().to_string();
            if !name.ends_with(".md") || file.is_dir() {
                continue;
            }

            let mut content = String::new();
            if let Err(e) = file.read_to_string(&mut content) {
                errors.push(ImportError { path: name, reason: e.to_string() });
                continue;
            }
            // ZipFile dropped here before any await
            file_entries.push((name, content));
        }

        for (name, content) in file_entries {
            let clean_path = name.trim_start_matches('/').to_string();
            if clean_path.contains("..") {
                errors.push(ImportError { path: clean_path, reason: "path traversal".into() });
                continue;
            }
            match self.git.write_file(&clean_path, &content, &format!("Import: {clean_path}"), author_email, author_email).await {
                Ok(_) => imported += 1,
                Err(e) => errors.push(ImportError { path: clean_path, reason: e.to_string() }),
            }
        }

        Ok(ImportResult { imported, errors })
    }
}

fn slugify(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slugify() {
        assert_eq!(slugify("Hello World!"), "hello-world");
        assert_eq!(slugify("My  Doc"), "my-doc");
        assert_eq!(slugify("café"), "café"); // é is alphanumeric in Rust
    }

    #[tokio::test]
    async fn test_import_outline_json() {
        let dir = tempfile::tempdir().unwrap();
        let git = crate::git::engine::GitEngine::init(dir.path().to_path_buf(), crate::git::queue::GitQueue::new()).unwrap();
        let engine = ImportEngine::new(git);

        let json = b"{\"documents\":[{\"title\":\"Getting Started\",\"text\":\"Welcome!\"},{\"title\":\"API Docs\",\"text\":\"See below.\"}]}";
        let result = engine.from_outline_json(json, "test@test.com").await.unwrap();
        assert_eq!(result.imported, 2);
        assert!(result.errors.is_empty());
    }

    #[tokio::test]
    async fn test_import_invalid_json() {
        let dir = tempfile::tempdir().unwrap();
        let git = crate::git::engine::GitEngine::init(dir.path().to_path_buf(), crate::git::queue::GitQueue::new()).unwrap();
        let engine = ImportEngine::new(git);
        let err = engine.from_outline_json(b"not json", "test@test.com").await.unwrap_err();
        assert!(matches!(err, AppError::BadRequest(_)));
    }
}
