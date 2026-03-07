use amanclaw_traits::message::{IncomingMessage, OutgoingMessage};
use amanclaw_security::auth::{Auth, UserState};
use amanclaw_security::rate_limiter::RateLimiter;
use amanclaw_security::sanitizer::check_injection;
use amanclaw_memory::sqlite::SqliteMemory;
use amanclaw_llm::client::LlmClient;
use anyhow::Result;
use std::sync::Mutex;

/// Message processing pipeline.
///
/// Orchestrates: auth -> rate limit -> sanitize -> context -> LLM -> respond.
pub enum Pipeline {
    /// Full pipeline with all services wired in.
    Full {
        auth: Mutex<Auth>,
        rate_limiter: Mutex<RateLimiter>,
        memory: SqliteMemory,
        llm: LlmClient,
    },
    /// Stub pipeline for testing — echoes back messages.
    Stub,
}

impl Pipeline {
    pub fn new() -> Self {
        Self::Stub
    }

    pub fn with_services(
        auth: Auth,
        rate_limiter: RateLimiter,
        memory: SqliteMemory,
        llm: LlmClient,
    ) -> Self {
        Self::Full {
            auth: Mutex::new(auth),
            rate_limiter: Mutex::new(rate_limiter),
            memory,
            llm,
        }
    }

    /// Process an incoming message through the full pipeline.
    /// Returns None if the message should be silently dropped.
    pub async fn process(&self, msg: IncomingMessage) -> Result<Option<OutgoingMessage>> {
        match self {
            Self::Stub => self.process_stub(msg).await,
            Self::Full { auth, rate_limiter, memory, llm } => {
                Self::process_full(auth, rate_limiter, memory, llm, msg).await
            }
        }
    }

    async fn process_stub(&self, msg: IncomingMessage) -> Result<Option<OutgoingMessage>> {
        tracing::info!(
            user_id = %msg.user_id,
            platform = %msg.platform,
            "Processing message (stub)"
        );
        Ok(Some(OutgoingMessage {
            chat_id: msg.chat_id,
            text: format!("[pipeline placeholder] Received: {}", msg.text),
            parse_mode: None,
            reply_to: None,
        }))
    }

    async fn process_full(
        auth: &Mutex<Auth>,
        rate_limiter: &Mutex<RateLimiter>,
        memory: &SqliteMemory,
        llm: &LlmClient,
        msg: IncomingMessage,
    ) -> Result<Option<OutgoingMessage>> {
        let user_id = &msg.user_id;
        let platform = &msg.platform;

        // 1. Auth check
        let state = auth.lock().unwrap().get_user_state(user_id, platform);
        match state {
            UserState::Blocked => return Ok(None),
            UserState::New => {
                auth.lock().unwrap().register_user(user_id, platform);
                return Ok(Some(OutgoingMessage {
                    chat_id: msg.chat_id,
                    text: "Welcome! You've been registered. An admin needs to approve your access.".into(),
                    parse_mode: None,
                    reply_to: None,
                }));
            }
            UserState::Pending => {
                return Ok(Some(OutgoingMessage {
                    chat_id: msg.chat_id,
                    text: "Your registration is pending approval.".into(),
                    parse_mode: None,
                    reply_to: None,
                }));
            }
            UserState::Admin | UserState::Approved => {} // proceed
        }

        // 2. Rate limit
        if !rate_limiter.lock().unwrap().check(user_id) {
            return Ok(Some(OutgoingMessage {
                chat_id: msg.chat_id,
                text: "Slow down — too many messages. Try again in a minute.".into(),
                parse_mode: None,
                reply_to: None,
            }));
        }

        // 3. Sanitize
        let (clean_text, was_flagged) = check_injection(&msg.text);
        if was_flagged {
            tracing::warn!(user_id, "Flagged message");
        }

        // 4. Build context
        let history = memory.get_history(user_id, 20).await?;
        let history_json: Vec<serde_json::Value> = history.iter().map(|m| {
            serde_json::json!({"role": m.role, "content": m.content})
        }).collect();

        // 5. LLM call
        let response = match llm.respond(&clean_text, &history_json, &[]).await {
            Ok(r) => r,
            Err(e) => {
                tracing::error!(error = %e, "LLM error");
                "Something went wrong talking to the AI. Try again in a moment.".into()
            }
        };

        // 6. Save exchange
        memory.save_exchange(user_id, platform, &msg.text, &response).await?;

        Ok(Some(OutgoingMessage {
            chat_id: msg.chat_id,
            text: response,
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
