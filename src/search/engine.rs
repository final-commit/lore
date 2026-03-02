use std::path::PathBuf;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tantivy::{
    collector::TopDocs,
    doc,
    query::QueryParser,
    schema::{document::Value as TantivyValue, OwnedValue, Schema, STORED, STRING, TEXT},
    Index, IndexReader, IndexWriter, ReloadPolicy, TantivyDocument, Term, TantivyError,
};
use tokio::sync::Mutex;

use crate::error::AppError;

// ── Data types ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub path: String,
    pub title: String,
    pub snippet: String,
    pub score: f32,
}

#[derive(Debug, Clone)]
pub struct IndexDoc {
    pub path: String,
    pub title: String,
    pub body: String,
}

// ── Engine ────────────────────────────────────────────────────────────────────

/// Schema field handles — cloned alongside the engine.
#[derive(Clone)]
struct Fields {
    path: tantivy::schema::Field,
    title: tantivy::schema::Field,
    body: tantivy::schema::Field,
}

pub struct SearchEngine {
    index: Index,
    reader: IndexReader,
    writer: Arc<Mutex<IndexWriter>>,
    fields: Fields,
}

impl SearchEngine {
    /// Create a new search engine backed by an on-disk index at `index_path`.
    pub fn open(index_path: PathBuf) -> Result<Self, AppError> {
        let (index, fields) = open_or_create_index(index_path, false)?;
        let (reader, writer) = make_reader_writer(&index)?;
        Ok(SearchEngine { index, reader, writer, fields })
    }

    /// Create a RAM-backed search engine (for tests).
    pub fn open_in_ram() -> Result<Self, AppError> {
        let (index, fields) = open_or_create_index(PathBuf::new(), true)?;
        let (reader, writer) = make_reader_writer(&index)?;
        Ok(SearchEngine { index, reader, writer, fields })
    }

    /// Add or update a document in the index.
    /// Uses `path` as the unique key (deletes old, inserts new).
    pub async fn upsert(&self, doc: IndexDoc) -> Result<(), AppError> {
        let writer = self.writer.clone();
        let fields = self.fields.clone();
        let path_term = Term::from_field_text(fields.path, &doc.path);

        tokio::task::spawn_blocking(move || {
            let mut w = writer.blocking_lock();
            w.delete_term(path_term);
            w.add_document(doc!(
                fields.path  => doc.path,
                fields.title => doc.title,
                fields.body  => doc.body,
            ))
            .map_err(|e: TantivyError| AppError::Search(e))?;
            w.commit().map_err(AppError::Search)?;
            Ok::<_, AppError>(())
        })
        .await
        .map_err(|e| AppError::Internal(format!("search task panic: {e}")))?
    }

    /// Remove a document from the index by path.
    pub async fn remove(&self, path: &str) -> Result<(), AppError> {
        let writer = self.writer.clone();
        let path_term = Term::from_field_text(self.fields.path, path);

        tokio::task::spawn_blocking(move || {
            let mut w = writer.blocking_lock();
            w.delete_term(path_term);
            w.commit().map_err(AppError::Search)?;
            Ok::<_, AppError>(())
        })
        .await
        .map_err(|e| AppError::Internal(format!("search task panic: {e}")))?
    }

    /// Full-text search.  Returns up to `limit` results ordered by relevance.
    pub async fn query(&self, q: &str, limit: usize) -> Result<Vec<SearchResult>, AppError> {
        let fields = self.fields.clone();
        let index = self.index.clone();
        let reader = self.reader.clone();
        let q = q.to_string();

        tokio::task::spawn_blocking(move || {
            // P1 #20: reload inside spawn_blocking — IndexReader::reload may do I/O.
            reader.reload().map_err(AppError::Search)?;
            let searcher = reader.searcher();

            let query_parser =
                QueryParser::for_index(&index, vec![fields.title, fields.body]);
            let query = query_parser
                .parse_query(&q)
                .map_err(|e| AppError::Search(TantivyError::from(e)))?;

            let top_docs = searcher
                .search(&query, &TopDocs::with_limit(limit))
                .map_err(AppError::Search)?;

            let mut results = Vec::with_capacity(top_docs.len());
            for (score, addr) in top_docs {
                let doc: TantivyDocument =
                    searcher.doc::<TantivyDocument>(addr).map_err(AppError::Search)?;
                let path = doc
                    .get_first(fields.path)
                    .and_then(|v: &OwnedValue| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let title = doc
                    .get_first(fields.title)
                    .and_then(|v: &OwnedValue| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let snippet = doc
                    .get_first(fields.body)
                    .and_then(|v: &OwnedValue| v.as_str())
                    .map(|b| truncate_snippet(b, 200))
                    .unwrap_or_default();

                results.push(SearchResult { path, title, snippet, score });
            }

            Ok(results)
        })
        .await
        .map_err(|e| AppError::Internal(format!("search task panic: {e}")))?
    }

    /// Index all documents in a batch, dropping the existing index contents.
    pub async fn reindex(&self, docs: Vec<IndexDoc>) -> Result<(), AppError> {
        let writer = self.writer.clone();
        let fields = self.fields.clone();

        tokio::task::spawn_blocking(move || {
            let mut w = writer.blocking_lock();
            w.delete_all_documents().map_err(AppError::Search)?;
            for doc in docs {
                w.add_document(doc!(
                    fields.path  => doc.path,
                    fields.title => doc.title,
                    fields.body  => doc.body,
                ))
                .map_err(|e: TantivyError| AppError::Search(e))?;
            }
            w.commit().map_err(AppError::Search)?;
            Ok::<_, AppError>(())
        })
        .await
        .map_err(|e| AppError::Internal(format!("search task panic: {e}")))?
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn build_schema() -> (Schema, Fields) {
    let mut sb = Schema::builder();
    let path = sb.add_text_field("path", STRING | STORED);
    let title = sb.add_text_field("title", TEXT | STORED);
    let body = sb.add_text_field("body", TEXT | STORED);
    let schema = sb.build();
    (schema, Fields { path, title, body })
}

fn open_or_create_index(
    path: PathBuf,
    in_ram: bool,
) -> Result<(Index, Fields), AppError> {
    let (schema, fields) = build_schema();
    let index = if in_ram {
        Index::create_in_ram(schema)
    } else {
        std::fs::create_dir_all(&path)
            .map_err(|e| AppError::Internal(format!("cannot create search dir: {e}")))?;
        let dir = tantivy::directory::MmapDirectory::open(&path)
            .map_err(|e| AppError::Internal(format!("open search dir: {e}")))?;
        Index::open_or_create(dir, schema).map_err(AppError::Search)?
    };
    Ok((index, fields))
}

fn make_reader_writer(index: &Index) -> Result<(IndexReader, Arc<Mutex<IndexWriter>>), AppError> {
    let reader = index
        .reader_builder()
        .reload_policy(ReloadPolicy::OnCommitWithDelay)
        .try_into()
        .map_err(AppError::Search)?;
    let writer: IndexWriter = index.writer(50_000_000).map_err(AppError::Search)?;
    Ok((reader, Arc::new(Mutex::new(writer))))
}

/// Truncate to at most `max` bytes on a char boundary to avoid UTF-8 panics.
fn truncate_snippet(s: &str, max: usize) -> String {
    if s.len() <= max {
        return s.to_string();
    }
    // Walk char boundaries to find the largest safe cut point ≤ max.
    let boundary = s
        .char_indices()
        .map(|(i, _)| i)
        .take_while(|&i| i <= max)
        .last()
        .unwrap_or(0);
    format!("{}…", &s[..boundary])
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn engine() -> SearchEngine {
        SearchEngine::open_in_ram().unwrap()
    }

    fn make_doc(path: &str, title: &str, body: &str) -> IndexDoc {
        IndexDoc {
            path: path.to_string(),
            title: title.to_string(),
            body: body.to_string(),
        }
    }

    #[tokio::test]
    async fn test_empty_query() {
        let engine = engine();
        let results = engine.query("hello", 10).await.unwrap();
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn test_upsert_and_search() {
        let engine = engine();
        engine
            .upsert(make_doc("docs/intro.md", "Introduction", "Welcome to Forge documentation"))
            .await
            .unwrap();

        let results = engine.query("Forge", 10).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].path, "docs/intro.md");
    }

    #[tokio::test]
    async fn test_upsert_updates_existing() {
        let engine = engine();
        engine
            .upsert(make_doc("doc.md", "Title v1", "old content"))
            .await
            .unwrap();
        engine
            .upsert(make_doc("doc.md", "Title v2", "new content"))
            .await
            .unwrap();

        let results = engine.query("new", 10).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Title v2");
    }

    #[tokio::test]
    async fn test_remove() {
        let engine = engine();
        engine
            .upsert(make_doc("to-remove.md", "Temp", "temporary doc"))
            .await
            .unwrap();

        engine.remove("to-remove.md").await.unwrap();

        let results = engine.query("temporary", 10).await.unwrap();
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn test_multiple_docs_ranking() {
        let engine = engine();
        engine
            .upsert(make_doc("a.md", "Rust Guide", "Rust is a systems programming language"))
            .await
            .unwrap();
        engine
            .upsert(make_doc("b.md", "Python Guide", "Python is a scripting language"))
            .await
            .unwrap();

        let results = engine.query("Rust", 10).await.unwrap();
        assert!(!results.is_empty());
        assert_eq!(results[0].path, "a.md");
    }

    #[tokio::test]
    async fn test_reindex() {
        let engine = engine();
        engine
            .upsert(make_doc("old.md", "Old", "old content"))
            .await
            .unwrap();

        engine
            .reindex(vec![make_doc("new.md", "New", "new content")])
            .await
            .unwrap();

        let no_old = engine.query("old", 10).await.unwrap();
        assert!(no_old.is_empty());

        let new_results = engine.query("new", 10).await.unwrap();
        assert_eq!(new_results.len(), 1);
    }

    #[tokio::test]
    async fn test_limit_respected() {
        let engine = engine();
        for i in 0..10 {
            engine
                .upsert(make_doc(
                    &format!("doc{i}.md"),
                    &format!("Doc {i}"),
                    "common keyword everywhere",
                ))
                .await
                .unwrap();
        }

        let results = engine.query("common", 3).await.unwrap();
        assert!(results.len() <= 3);
    }
}
