use amanclaw_traits::memory::{HistoryMessage, MemoryBackend};
use anyhow::Result;
use moka::future::Cache;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

/// LRU cache wrapper around any MemoryBackend.
/// Caches history, facts, and summaries. Invalidates on writes.
pub struct CachedMemory {
    inner: Arc<dyn MemoryBackend>,
    history_cache: Cache<String, Vec<HistoryMessage>>,
    facts_cache: Cache<String, HashMap<String, String>>,
    summary_cache: Cache<String, Option<String>>,
}

impl CachedMemory {
    pub fn new(inner: Arc<dyn MemoryBackend>, max_entries: u64, ttl_seconds: u64) -> Self {
        let ttl = Duration::from_secs(ttl_seconds);
        Self {
            inner,
            history_cache: Cache::builder()
                .max_capacity(max_entries)
                .time_to_live(ttl)
                .build(),
            facts_cache: Cache::builder()
                .max_capacity(max_entries)
                .time_to_live(ttl)
                .build(),
            summary_cache: Cache::builder()
                .max_capacity(max_entries)
                .time_to_live(ttl)
                .build(),
        }
    }

    fn cache_key(ns: &str, user_id: &str) -> String {
        format!("{}:{}", ns, user_id)
    }
}

#[async_trait::async_trait]
impl MemoryBackend for CachedMemory {
    async fn save_exchange(
        &self,
        ns: &str,
        user_id: &str,
        platform: &str,
        user_msg: &str,
        assistant_msg: &str,
    ) -> Result<()> {
        self.history_cache
            .invalidate(&Self::cache_key(ns, user_id))
            .await;
        self.inner
            .save_exchange(ns, user_id, platform, user_msg, assistant_msg)
            .await
    }

    async fn get_history(
        &self,
        ns: &str,
        user_id: &str,
        limit: i64,
    ) -> Result<Vec<HistoryMessage>> {
        let key = Self::cache_key(ns, user_id);
        if let Some(cached) = self.history_cache.get(&key).await {
            return Ok(cached);
        }
        let result = self.inner.get_history(ns, user_id, limit).await?;
        self.history_cache.insert(key, result.clone()).await;
        Ok(result)
    }

    async fn clear_history(&self, ns: &str, user_id: &str) -> Result<()> {
        self.history_cache
            .invalidate(&Self::cache_key(ns, user_id))
            .await;
        self.inner.clear_history(ns, user_id).await
    }

    async fn get_message_count(&self, ns: &str, user_id: &str) -> Result<i64> {
        // Not cached — cheap query
        self.inner.get_message_count(ns, user_id).await
    }

    async fn save_fact(&self, user_id: &str, key: &str, value: &str) -> Result<()> {
        self.facts_cache.invalidate(user_id).await;
        self.inner.save_fact(user_id, key, value).await
    }

    async fn get_facts(&self, user_id: &str) -> Result<HashMap<String, String>> {
        if let Some(cached) = self.facts_cache.get(user_id).await {
            return Ok(cached);
        }
        let result = self.inner.get_facts(user_id).await?;
        self.facts_cache
            .insert(user_id.to_string(), result.clone())
            .await;
        Ok(result)
    }

    async fn delete_fact(&self, user_id: &str, key: &str) -> Result<bool> {
        self.facts_cache.invalidate(user_id).await;
        self.inner.delete_fact(user_id, key).await
    }

    async fn get_summary(&self, ns: &str, user_id: &str) -> Result<Option<String>> {
        let key = Self::cache_key(ns, user_id);
        if let Some(cached) = self.summary_cache.get(&key).await {
            return Ok(cached);
        }
        let result = self.inner.get_summary(ns, user_id).await?;
        self.summary_cache.insert(key, result.clone()).await;
        Ok(result)
    }

    async fn save_summary_and_prune(
        &self,
        ns: &str,
        user_id: &str,
        summary: &str,
        keep_recent: i64,
    ) -> Result<()> {
        let key = Self::cache_key(ns, user_id);
        self.summary_cache.invalidate(&key).await;
        self.history_cache.invalidate(&key).await;
        self.inner
            .save_summary_and_prune(ns, user_id, summary, keep_recent)
            .await
    }

    async fn needs_summarization(&self, ns: &str, user_id: &str, threshold: i64) -> Result<bool> {
        // Not cached — cheap query
        self.inner.needs_summarization(ns, user_id, threshold).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    /// Mock memory backend that counts calls to verify cache behavior.
    struct CountingMemory {
        get_history_calls: AtomicU64,
        get_facts_calls: AtomicU64,
        get_summary_calls: AtomicU64,
    }

    impl CountingMemory {
        fn new() -> Self {
            Self {
                get_history_calls: AtomicU64::new(0),
                get_facts_calls: AtomicU64::new(0),
                get_summary_calls: AtomicU64::new(0),
            }
        }
    }

    #[async_trait::async_trait]
    impl MemoryBackend for CountingMemory {
        async fn save_exchange(
            &self,
            _ns: &str,
            _user_id: &str,
            _platform: &str,
            _user_msg: &str,
            _assistant_msg: &str,
        ) -> Result<()> {
            Ok(())
        }

        async fn get_history(
            &self,
            _ns: &str,
            _user_id: &str,
            _limit: i64,
        ) -> Result<Vec<HistoryMessage>> {
            self.get_history_calls.fetch_add(1, Ordering::SeqCst);
            Ok(vec![HistoryMessage {
                role: "user".into(),
                content: "hello".into(),
            }])
        }

        async fn clear_history(&self, _ns: &str, _user_id: &str) -> Result<()> {
            Ok(())
        }

        async fn get_message_count(&self, _ns: &str, _user_id: &str) -> Result<i64> {
            Ok(1)
        }

        async fn save_fact(&self, _user_id: &str, _key: &str, _value: &str) -> Result<()> {
            Ok(())
        }

        async fn get_facts(&self, _user_id: &str) -> Result<HashMap<String, String>> {
            self.get_facts_calls.fetch_add(1, Ordering::SeqCst);
            let mut m = HashMap::new();
            m.insert("name".into(), "test".into());
            Ok(m)
        }

        async fn delete_fact(&self, _user_id: &str, _key: &str) -> Result<bool> {
            Ok(true)
        }

        async fn get_summary(&self, _ns: &str, _user_id: &str) -> Result<Option<String>> {
            self.get_summary_calls.fetch_add(1, Ordering::SeqCst);
            Ok(Some("summary".into()))
        }

        async fn save_summary_and_prune(
            &self,
            _ns: &str,
            _user_id: &str,
            _summary: &str,
            _keep_recent: i64,
        ) -> Result<()> {
            Ok(())
        }

        async fn needs_summarization(
            &self,
            _ns: &str,
            _user_id: &str,
            _threshold: i64,
        ) -> Result<bool> {
            Ok(false)
        }
    }

    #[tokio::test]
    async fn test_history_cache_hit() {
        let counting = Arc::new(CountingMemory::new());
        let cached = CachedMemory::new(counting.clone(), 100, 60);

        // First call: cache miss — hits inner
        let _ = cached.get_history("ns", "user1", 10).await.unwrap();
        assert_eq!(counting.get_history_calls.load(Ordering::SeqCst), 1);

        // Second call: cache hit — does NOT hit inner
        let _ = cached.get_history("ns", "user1", 10).await.unwrap();
        assert_eq!(counting.get_history_calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_history_cache_invalidated_on_save() {
        let counting = Arc::new(CountingMemory::new());
        let cached = CachedMemory::new(counting.clone(), 100, 60);

        // Populate cache
        let _ = cached.get_history("ns", "user1", 10).await.unwrap();
        assert_eq!(counting.get_history_calls.load(Ordering::SeqCst), 1);

        // Save exchange invalidates cache
        cached
            .save_exchange("ns", "user1", "test", "hi", "hello")
            .await
            .unwrap();

        // Next get should hit inner again
        let _ = cached.get_history("ns", "user1", 10).await.unwrap();
        assert_eq!(counting.get_history_calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn test_facts_cache_hit() {
        let counting = Arc::new(CountingMemory::new());
        let cached = CachedMemory::new(counting.clone(), 100, 60);

        let _ = cached.get_facts("user1").await.unwrap();
        assert_eq!(counting.get_facts_calls.load(Ordering::SeqCst), 1);

        let _ = cached.get_facts("user1").await.unwrap();
        assert_eq!(counting.get_facts_calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_facts_cache_invalidated_on_save() {
        let counting = Arc::new(CountingMemory::new());
        let cached = CachedMemory::new(counting.clone(), 100, 60);

        let _ = cached.get_facts("user1").await.unwrap();
        assert_eq!(counting.get_facts_calls.load(Ordering::SeqCst), 1);

        cached.save_fact("user1", "k", "v").await.unwrap();

        let _ = cached.get_facts("user1").await.unwrap();
        assert_eq!(counting.get_facts_calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn test_facts_cache_invalidated_on_delete() {
        let counting = Arc::new(CountingMemory::new());
        let cached = CachedMemory::new(counting.clone(), 100, 60);

        let _ = cached.get_facts("user1").await.unwrap();
        assert_eq!(counting.get_facts_calls.load(Ordering::SeqCst), 1);

        cached.delete_fact("user1", "k").await.unwrap();

        let _ = cached.get_facts("user1").await.unwrap();
        assert_eq!(counting.get_facts_calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn test_summary_cache_hit() {
        let counting = Arc::new(CountingMemory::new());
        let cached = CachedMemory::new(counting.clone(), 100, 60);

        let _ = cached.get_summary("ns", "user1").await.unwrap();
        assert_eq!(counting.get_summary_calls.load(Ordering::SeqCst), 1);

        let _ = cached.get_summary("ns", "user1").await.unwrap();
        assert_eq!(counting.get_summary_calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_summary_cache_invalidated_on_prune() {
        let counting = Arc::new(CountingMemory::new());
        let cached = CachedMemory::new(counting.clone(), 100, 60);

        let _ = cached.get_summary("ns", "user1").await.unwrap();
        assert_eq!(counting.get_summary_calls.load(Ordering::SeqCst), 1);

        cached
            .save_summary_and_prune("ns", "user1", "new summary", 5)
            .await
            .unwrap();

        let _ = cached.get_summary("ns", "user1").await.unwrap();
        assert_eq!(counting.get_summary_calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn test_different_users_independent_caches() {
        let counting = Arc::new(CountingMemory::new());
        let cached = CachedMemory::new(counting.clone(), 100, 60);

        let _ = cached.get_history("ns", "user1", 10).await.unwrap();
        let _ = cached.get_history("ns", "user2", 10).await.unwrap();
        assert_eq!(counting.get_history_calls.load(Ordering::SeqCst), 2);

        // Each user cached independently
        let _ = cached.get_history("ns", "user1", 10).await.unwrap();
        let _ = cached.get_history("ns", "user2", 10).await.unwrap();
        assert_eq!(counting.get_history_calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn test_clear_history_invalidates_cache() {
        let counting = Arc::new(CountingMemory::new());
        let cached = CachedMemory::new(counting.clone(), 100, 60);

        let _ = cached.get_history("ns", "user1", 10).await.unwrap();
        assert_eq!(counting.get_history_calls.load(Ordering::SeqCst), 1);

        cached.clear_history("ns", "user1").await.unwrap();

        let _ = cached.get_history("ns", "user1", 10).await.unwrap();
        assert_eq!(counting.get_history_calls.load(Ordering::SeqCst), 2);
    }
}
