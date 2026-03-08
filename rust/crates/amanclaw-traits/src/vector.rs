use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A document to store in the vector store.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: String,
    pub content: String,
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

/// A search result from the vector store.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub id: String,
    pub content: String,
    pub score: f64,
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

/// Trait for pluggable vector store backends.
#[async_trait::async_trait]
pub trait VectorStore: Send + Sync {
    /// Insert or update documents in a collection.
    async fn upsert(&self, collection: &str, docs: &[Document]) -> Result<()>;

    /// Semantic search for similar documents.
    async fn search(&self, collection: &str, query: &str, limit: usize) -> Result<Vec<SearchResult>>;

    /// Delete documents by ID.
    async fn delete(&self, collection: &str, ids: &[String]) -> Result<()>;

    /// Insert or update documents with pre-computed embeddings.
    /// Default falls back to upsert without embeddings.
    async fn upsert_with_embeddings(
        &self, collection: &str, docs: &[Document], _embeddings: &[Vec<f32>],
    ) -> Result<()> {
        self.upsert(collection, docs).await
    }

    /// Semantic search using a pre-computed query embedding.
    /// Default falls back to text search.
    async fn search_by_embedding(
        &self, collection: &str, _query_embedding: &[f32], query_text: &str, limit: usize,
    ) -> Result<Vec<SearchResult>> {
        self.search(collection, query_text, limit).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_document_creation() {
        let doc = Document {
            id: "quran:2:255".into(),
            content: "Ayat al-Kursi".into(),
            metadata: HashMap::from([("surah".into(), "Al-Baqarah".into())]),
        };
        assert_eq!(doc.id, "quran:2:255");
        assert_eq!(doc.metadata.get("surah").unwrap(), "Al-Baqarah");
    }

    #[test]
    fn test_search_result() {
        let result = SearchResult {
            id: "quran:2:255".into(),
            content: "Ayat al-Kursi".into(),
            score: 0.95,
            metadata: HashMap::new(),
        };
        assert!(result.score > 0.9);
    }

    #[test]
    fn test_document_serialization() {
        let doc = Document {
            id: "h1".into(),
            content: "Hadith about prayer".into(),
            metadata: HashMap::new(),
        };
        let json = serde_json::to_string(&doc).unwrap();
        let deserialized: Document = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, "h1");
    }
}
