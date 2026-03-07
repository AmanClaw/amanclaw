use amanclaw_traits::message::{IncomingMessage, OutgoingMessage};
use anyhow::Result;

/// Message processing pipeline.
///
/// Orchestrates: auth -> rate limit -> sanitize -> context -> LLM -> skill -> respond.
/// Each stage will be plugged in as we implement the respective crates.
pub struct Pipeline {
    // Will hold: auth, rate_limiter, memory, llm, wasm_runtime
}

impl Pipeline {
    pub fn new() -> Self {
        Self {}
    }

    /// Process an incoming message through the full pipeline.
    /// Returns None if the message should be silently dropped.
    pub async fn process(&self, msg: IncomingMessage) -> Result<Option<OutgoingMessage>> {
        tracing::info!(
            user_id = %msg.user_id,
            platform = %msg.platform,
            "Processing message"
        );

        // TODO Phase 2: auth check
        // TODO Phase 2: rate limit
        // TODO Phase 2: sanitize
        // TODO Phase 2: build context from memory
        // TODO Phase 2: LLM call
        // TODO Phase 3: skill dispatch via WASM runtime

        // Placeholder: echo back
        Ok(Some(OutgoingMessage {
            chat_id: msg.chat_id,
            text: format!("[pipeline placeholder] Received: {}", msg.text),
            parse_mode: None,
            reply_to: None,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_message(text: &str) -> IncomingMessage {
        IncomingMessage {
            user_id: "admin1".into(),
            chat_id: "admin1".into(),
            platform: "telegram".into(),
            text: text.into(),
            username: Some("testuser".into()),
            first_name: Some("Test".into()),
            is_group: false,
            image_data: None,
            reply_to: None,
        }
    }

    #[tokio::test]
    async fn test_pipeline_processes_message() {
        let pipeline = Pipeline::new();
        let msg = make_test_message("Hello bot");
        let result = pipeline.process(msg).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.is_some());
        assert!(response.unwrap().text.contains("Hello bot"));
    }
}
