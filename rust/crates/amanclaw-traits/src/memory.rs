use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A single message from conversation history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryMessage {
    pub role: String,
    pub content: String,
}

/// Trait for pluggable memory backends.
///
/// The `ns` (namespace) parameter isolates data per agent profile.
/// For backward compatibility, use `"default"` as the namespace.
#[async_trait::async_trait]
pub trait MemoryBackend: Send + Sync {
    // Conversation history
    async fn save_exchange(
        &self, ns: &str, user_id: &str, platform: &str,
        user_msg: &str, assistant_msg: &str,
    ) -> Result<()>;

    async fn get_history(
        &self, ns: &str, user_id: &str, limit: i64,
    ) -> Result<Vec<HistoryMessage>>;

    async fn clear_history(&self, ns: &str, user_id: &str) -> Result<()>;

    async fn get_message_count(&self, ns: &str, user_id: &str) -> Result<i64>;

    // Facts (not namespaced — facts are per-user across all agents)
    async fn save_fact(&self, user_id: &str, key: &str, value: &str) -> Result<()>;
    async fn get_facts(&self, user_id: &str) -> Result<HashMap<String, String>>;
    async fn delete_fact(&self, user_id: &str, key: &str) -> Result<bool>;

    // Summarization
    async fn get_summary(&self, ns: &str, user_id: &str) -> Result<Option<String>>;

    async fn save_summary_and_prune(
        &self, ns: &str, user_id: &str, summary: &str, keep_recent: i64,
    ) -> Result<()>;

    async fn needs_summarization(
        &self, ns: &str, user_id: &str, threshold: i64,
    ) -> Result<bool>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_history_message_creation() {
        let msg = HistoryMessage {
            role: "user".into(),
            content: "Hello".into(),
        };
        assert_eq!(msg.role, "user");
        assert_eq!(msg.content, "Hello");
    }

    #[test]
    fn test_history_message_serialization() {
        let msg = HistoryMessage {
            role: "assistant".into(),
            content: "Hi there".into(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: HistoryMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.role, "assistant");
    }
}
