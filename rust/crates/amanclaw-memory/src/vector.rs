use amanclaw_traits::vector::{Document, SearchResult, VectorStore};
use anyhow::Result;
use sqlx::{SqlitePool, Row};

/// SQLite-backed vector store.
/// Stores embeddings as BLOBs and computes cosine similarity in Rust.
/// Good enough for <100K documents. Use Qdrant for larger corpora.
pub struct SqliteVectorStore {
    pool: SqlitePool,
}

impl SqliteVectorStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Store a document with its pre-computed embedding.
    pub async fn upsert_with_embeddings(
        &self, collection: &str, docs: &[Document], embeddings: &[Vec<f32>],
    ) -> Result<()> {
        for (doc, embedding) in docs.iter().zip(embeddings.iter()) {
            let metadata_json = serde_json::to_string(&doc.metadata)?;
            let embedding_bytes = embedding_to_bytes(embedding);

            sqlx::query(
                "INSERT INTO vector_documents (id, collection, content, metadata, embedding)
                 VALUES (?, ?, ?, ?, ?)
                 ON CONFLICT(collection, id) DO UPDATE SET
                    content = excluded.content,
                    metadata = excluded.metadata,
                    embedding = excluded.embedding"
            )
                .bind(&doc.id)
                .bind(collection)
                .bind(&doc.content)
                .bind(&metadata_json)
                .bind(&embedding_bytes)
                .execute(&self.pool).await?;
        }
        Ok(())
    }

    /// Search by cosine similarity against a query embedding.
    pub async fn search_by_embedding(
        &self, collection: &str, query_embedding: &[f32], limit: usize,
    ) -> Result<Vec<SearchResult>> {
        let rows = sqlx::query(
            "SELECT id, content, metadata, embedding FROM vector_documents WHERE collection = ?"
        )
            .bind(collection)
            .fetch_all(&self.pool).await?;

        let mut scored: Vec<(f64, String, String, String)> = rows.iter()
            .filter_map(|row| {
                let id: String = row.get("id");
                let content: String = row.get("content");
                let metadata: String = row.get("metadata");
                let embedding_bytes: Vec<u8> = row.get("embedding");
                let embedding = bytes_to_embedding(&embedding_bytes);
                let score = cosine_similarity(query_embedding, &embedding);
                Some((score, id, content, metadata))
            })
            .collect();

        // Sort by score descending
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(limit);

        let results = scored.into_iter().map(|(score, id, content, metadata_str)| {
            let metadata = serde_json::from_str(&metadata_str).unwrap_or_default();
            SearchResult { id, content, score, metadata }
        }).collect();

        Ok(results)
    }
}

#[async_trait::async_trait]
impl VectorStore for SqliteVectorStore {
    async fn upsert(&self, collection: &str, docs: &[Document]) -> Result<()> {
        // Without embeddings — store content only (embeddings added separately)
        for doc in docs {
            let metadata_json = serde_json::to_string(&doc.metadata)?;
            sqlx::query(
                "INSERT INTO vector_documents (id, collection, content, metadata)
                 VALUES (?, ?, ?, ?)
                 ON CONFLICT(collection, id) DO UPDATE SET
                    content = excluded.content,
                    metadata = excluded.metadata"
            )
                .bind(&doc.id)
                .bind(collection)
                .bind(&doc.content)
                .bind(&metadata_json)
                .execute(&self.pool).await?;
        }
        Ok(())
    }

    async fn search(&self, collection: &str, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
        // Use FTS5 MATCH with BM25 ranking instead of LIKE
        let rows = sqlx::query(
            "SELECT vd.id, vd.content, vd.metadata, bm25(vector_documents_fts) as rank
             FROM vector_documents vd
             JOIN vector_documents_fts fts ON vd.rowid = fts.rowid
             WHERE vd.collection = ? AND vector_documents_fts MATCH ?
             ORDER BY rank
             LIMIT ?"
        )
            .bind(collection)
            .bind(query)
            .bind(limit as i64)
            .fetch_all(&self.pool).await?;

        let results = rows.iter().map(|row| {
            let metadata_str: String = row.get("metadata");
            SearchResult {
                id: row.get("id"),
                content: row.get("content"),
                score: row.get::<f64, _>("rank"),
                metadata: serde_json::from_str(&metadata_str).unwrap_or_default(),
            }
        }).collect();

        Ok(results)
    }

    async fn delete(&self, collection: &str, ids: &[String]) -> Result<()> {
        for id in ids {
            sqlx::query("DELETE FROM vector_documents WHERE collection = ? AND id = ?")
                .bind(collection)
                .bind(id)
                .execute(&self.pool).await?;
        }
        Ok(())
    }

    async fn upsert_with_embeddings(
        &self, collection: &str, docs: &[Document], embeddings: &[Vec<f32>],
    ) -> Result<()> {
        SqliteVectorStore::upsert_with_embeddings(self, collection, docs, embeddings).await
    }

    async fn search_by_embedding(
        &self, collection: &str, query_embedding: &[f32], query_text: &str, limit: usize,
    ) -> Result<Vec<SearchResult>> {
        // 1. Vector search (cosine similarity)
        let vector_results = SqliteVectorStore::search_by_embedding(self, collection, query_embedding, limit * 2).await?;
        let vector_ranked: Vec<_> = vector_results.iter()
            .map(|r| (r.id.clone(), r.score, r.content.clone(), serde_json::to_string(&r.metadata).unwrap_or_default()))
            .collect();

        // 2. FTS5 BM25 search
        let fts_rows = sqlx::query(
            "SELECT vd.id, vd.content, vd.metadata, bm25(vector_documents_fts) as rank
             FROM vector_documents vd
             JOIN vector_documents_fts fts ON vd.rowid = fts.rowid
             WHERE vd.collection = ? AND vector_documents_fts MATCH ?
             ORDER BY rank
             LIMIT ?"
        )
            .bind(collection)
            .bind(query_text)
            .bind((limit * 2) as i64)
            .fetch_all(&self.pool).await
            .unwrap_or_default(); // FTS match may fail on invalid syntax — degrade gracefully

        let fts_ranked: Vec<_> = fts_rows.iter()
            .map(|row| {
                let id: String = row.get("id");
                let content: String = row.get("content");
                let metadata: String = row.get("metadata");
                let rank: f64 = row.get("rank");
                (id, rank, content, metadata)
            })
            .collect();

        // 3. Merge with RRF (k=60)
        if fts_ranked.is_empty() {
            let mut results = vector_results;
            results.truncate(limit);
            return Ok(results);
        }

        let merged = hybrid_rrf(&vector_ranked, &fts_ranked, 60.0);

        let results: Vec<SearchResult> = merged.into_iter()
            .take(limit)
            .map(|(id, score, content, metadata_str)| {
                let metadata = serde_json::from_str(&metadata_str).unwrap_or_default();
                SearchResult { id, content, score, metadata }
            })
            .collect();

        Ok(results)
    }
}

/// Reciprocal Rank Fusion: combines two ranked lists without score normalization.
/// k=60 is the standard constant from Cormack et al. 2009.
fn hybrid_rrf(
    vector_ranked: &[(String, f64, String, String)], // (id, score, content, metadata)
    fts_ranked: &[(String, f64, String, String)],
    k: f64,
) -> Vec<(String, f64, String, String)> {
    use std::collections::HashMap;

    let mut scores: HashMap<String, f64> = HashMap::new();
    let mut data: HashMap<String, (String, String)> = HashMap::new();

    for (rank, (id, _, content, metadata)) in vector_ranked.iter().enumerate() {
        *scores.entry(id.clone()).or_default() += 1.0 / (k + rank as f64 + 1.0);
        data.entry(id.clone()).or_insert_with(|| (content.clone(), metadata.clone()));
    }
    for (rank, (id, _, content, metadata)) in fts_ranked.iter().enumerate() {
        *scores.entry(id.clone()).or_default() += 1.0 / (k + rank as f64 + 1.0);
        data.entry(id.clone()).or_insert_with(|| (content.clone(), metadata.clone()));
    }

    let mut merged: Vec<_> = scores.into_iter()
        .map(|(id, score)| {
            let (content, metadata) = data.remove(&id).unwrap_or_default();
            (id, score, content, metadata)
        })
        .collect();
    merged.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    merged
}

// --- Embedding helpers ---

fn embedding_to_bytes(embedding: &[f32]) -> Vec<u8> {
    embedding.iter().flat_map(|f| f.to_le_bytes()).collect()
}

fn bytes_to_embedding(bytes: &[u8]) -> Vec<f32> {
    bytes.chunks_exact(4)
        .map(|chunk| f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect()
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f64 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let dot: f64 = a.iter().zip(b.iter()).map(|(x, y)| (*x as f64) * (*y as f64)).sum();
    let norm_a: f64 = a.iter().map(|x| (*x as f64) * (*x as f64)).sum::<f64>().sqrt();
    let norm_b: f64 = b.iter().map(|x| (*x as f64) * (*x as f64)).sum::<f64>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    dot / (norm_a * norm_b)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;
    use std::collections::HashMap;

    async fn make_pool() -> SqlitePool {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await.unwrap();
        sqlx::raw_sql(crate::schema::INIT_SQL).execute(&pool).await.unwrap();
        pool
    }

    #[tokio::test]
    async fn test_upsert_and_search_text() {
        let pool = make_pool().await;
        let store = SqliteVectorStore::new(pool);

        let docs = vec![
            Document {
                id: "q1".into(),
                content: "Bismillah ar-Rahman ar-Rahim".into(),
                metadata: HashMap::from([("surah".into(), "Al-Fatihah".into())]),
            },
            Document {
                id: "q2".into(),
                content: "Alhamdulillah Rabbil Alamin".into(),
                metadata: HashMap::new(),
            },
        ];

        store.upsert("quran", &docs).await.unwrap();

        let results = store.search("quran", "Rahman", 5).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "q1");
        assert_eq!(results[0].metadata.get("surah").unwrap(), "Al-Fatihah");
        assert!(results[0].score < 0.0); // BM25 returns negative scores (lower = better match)
    }

    #[tokio::test]
    async fn test_upsert_with_embeddings_and_search() {
        let pool = make_pool().await;
        let store = SqliteVectorStore::new(pool);

        let docs = vec![
            Document { id: "d1".into(), content: "Prayer times".into(), metadata: HashMap::new() },
            Document { id: "d2".into(), content: "Fasting rules".into(), metadata: HashMap::new() },
        ];
        let embeddings = vec![
            vec![1.0, 0.0, 0.0],
            vec![0.0, 1.0, 0.0],
        ];

        store.upsert_with_embeddings("test", &docs, &embeddings).await.unwrap();

        // Query closer to d1
        let results = store.search_by_embedding("test", &[0.9, 0.1, 0.0], 2).await.unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].id, "d1"); // Highest similarity
        assert!(results[0].score > results[1].score);
    }

    #[tokio::test]
    async fn test_delete() {
        let pool = make_pool().await;
        let store = SqliteVectorStore::new(pool);

        let docs = vec![
            Document { id: "d1".into(), content: "test".into(), metadata: HashMap::new() },
        ];
        store.upsert("col", &docs).await.unwrap();

        store.delete("col", &["d1".to_string()]).await.unwrap();

        let results = store.search("col", "test", 10).await.unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 0.001);

        let c = vec![0.0, 1.0, 0.0];
        assert!((cosine_similarity(&a, &c) - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_embedding_roundtrip() {
        let original = vec![1.5, -2.3, 0.0, 42.0];
        let bytes = embedding_to_bytes(&original);
        let restored = bytes_to_embedding(&bytes);
        assert_eq!(original, restored);
    }

    #[tokio::test]
    async fn test_fts5_text_search_with_bm25() {
        let pool = make_pool().await;
        let store = SqliteVectorStore::new(pool);

        let docs = vec![
            Document { id: "q1".into(), content: "Bismillah ar-Rahman ar-Rahim".into(), metadata: HashMap::new() },
            Document { id: "q2".into(), content: "Alhamdulillah Rabbil Alamin".into(), metadata: HashMap::new() },
            Document { id: "q3".into(), content: "The most merciful and compassionate".into(), metadata: HashMap::new() },
        ];
        store.upsert("quran", &docs).await.unwrap();

        let results = store.search("quran", "Rahman", 5).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "q1");
        assert!(results[0].score < 0.0); // BM25 returns negative scores (lower = better match)
    }

    #[tokio::test]
    async fn test_hybrid_rrf_search() {
        let pool = make_pool().await;
        let store = SqliteVectorStore::new(pool);

        let docs = vec![
            Document { id: "d1".into(), content: "prayer prayer prayer times".into(), metadata: HashMap::new() },
            Document { id: "d2".into(), content: "Fasting rules during Ramadan".into(), metadata: HashMap::new() },
            Document { id: "d3".into(), content: "Solat schedule Malaysia".into(), metadata: HashMap::new() },
        ];
        let embeddings = vec![
            vec![1.0, 0.0, 0.0],  // d1: exact match direction
            vec![0.0, 1.0, 0.0],  // d2: orthogonal
            vec![0.1, 0.9, 0.0],  // d3: far from query
        ];
        store.upsert_with_embeddings("test", &docs, &embeddings).await.unwrap();

        // Query embedding matches d1, text query "prayer" only matches d1
        // RRF should rank d1 highest (top in both vector AND FTS)
        let store_trait: &dyn VectorStore = &store;
        let results = store_trait.search_by_embedding("test", &[0.9, 0.1, 0.0], "prayer", 3).await.unwrap();
        assert!(!results.is_empty());
        assert_eq!(results[0].id, "d1"); // Best in both ranking lists
    }
}
