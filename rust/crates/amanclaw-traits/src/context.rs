use anyhow::Result;
use crate::agent::AgentProfile;
use crate::skill::ToolDefinition;

/// Request to build context for an LLM call.
#[derive(Debug, Clone)]
pub struct ContextRequest {
    pub user_id: String,
    pub platform: String,
    pub namespace: String,
    pub user_message: String,
    pub image_data: Option<Vec<u8>>,
    pub agent_profile: AgentProfile,
}

/// Built context ready for the LLM.
pub struct ContextResult {
    /// Full message array (system + history + user message).
    pub messages: Vec<serde_json::Value>,
    /// Tools filtered per agent profile.
    pub tools: Vec<ToolDefinition>,
}

/// Event fired after a successful user-assistant exchange.
#[derive(Debug, Clone)]
pub struct ExchangeEvent {
    pub user_id: String,
    pub platform: String,
    pub namespace: String,
    pub user_message: String,
    pub assistant_response: String,
}

/// Event fired when compaction check is needed.
#[derive(Debug, Clone)]
pub struct CompactionEvent {
    pub user_id: String,
    pub namespace: String,
    pub message_count: i64,
    pub threshold: i64,
}

/// Result of a compaction decision.
#[derive(Debug, Clone)]
pub struct CompactionResult {
    pub should_compact: bool,
    pub summary: Option<String>,
    pub keep_recent: i64,
}

/// Trait for pluggable context building strategies.
#[async_trait::async_trait]
pub trait ContextEngine: Send + Sync {
    /// Build the full message context for an LLM call.
    async fn build_context(&self, request: ContextRequest) -> Result<ContextResult>;

    /// Called after a successful exchange.
    async fn on_exchange_complete(&self, exchange: ExchangeEvent) -> Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_request_creation() {
        let req = ContextRequest {
            user_id: "u1".into(),
            platform: "telegram".into(),
            namespace: "default".into(),
            user_message: "Hello".into(),
            image_data: None,
            agent_profile: AgentProfile::default_agent(),
        };
        assert_eq!(req.namespace, "default");
    }

    #[test]
    fn test_exchange_event_creation() {
        let event = ExchangeEvent {
            user_id: "u1".into(),
            platform: "telegram".into(),
            namespace: "default".into(),
            user_message: "Hello".into(),
            assistant_response: "Hi!".into(),
        };
        assert_eq!(event.assistant_response, "Hi!");
    }

    #[test]
    fn test_compaction_result() {
        let result = CompactionResult {
            should_compact: true,
            summary: Some("User asked about prayer times".into()),
            keep_recent: 10,
        };
        assert!(result.should_compact);
        assert!(result.summary.is_some());
    }
}
