use amanclaw_traits::agent::AgentProfile;
use amanclaw_traits::context::ContextEngine;
use amanclaw_traits::memory::MemoryBackend;
use amanclaw_traits::message::{IncomingMessage, OutgoingMessage};
use amanclaw_traits::event::EventEmitter;
use amanclaw_security::auth::Auth;
use amanclaw_security::rate_limiter::RateLimiter;
use amanclaw_llm::client::LlmClient;
use crate::middleware::{MiddlewareChain, PipelineContext};
use crate::middleware::auth::AuthMiddleware;
use crate::middleware::command::CommandMiddleware;
use crate::middleware::rate_limit::RateLimitMiddleware;
use crate::middleware::sanitize::SanitizeMiddleware;
use crate::middleware::context::ContextMiddleware;
use crate::middleware::persist::PersistMiddleware;
use crate::middleware::tool_calling::ToolCallingMiddleware;
use crate::middleware::MetricsMiddleware;
use crate::registry::PluginRegistry;
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Message processing pipeline.
pub enum Pipeline {
    Full { chain: MiddlewareChain },
    Stub,
}

impl Pipeline {
    pub fn new() -> Self {
        Self::Stub
    }

    pub fn with_services(
        auth: Arc<RwLock<Auth>>,
        rate_limiter: RateLimiter,
        context_engine: Arc<dyn ContextEngine>,
        memory: Arc<dyn MemoryBackend>,
        llm: Arc<LlmClient>,
        emitter: Arc<dyn EventEmitter>,
    ) -> Self {
        let chain = MiddlewareChain::new(vec![
            Box::new(MetricsMiddleware),
            Box::new(AuthMiddleware::new(auth.clone())),
            Box::new(CommandMiddleware::new(auth, memory.clone())),
            Box::new(RateLimitMiddleware::new(rate_limiter, emitter.clone())),
            Box::new(SanitizeMiddleware::new(emitter.clone())),
            Box::new(ContextMiddleware::new(context_engine.clone())),
            Box::new(PersistMiddleware::new(context_engine, memory, llm.clone(), emitter)),
            Box::new(ToolCallingMiddleware::new(llm)),
        ]);
        Self::Full { chain }
    }

    pub async fn process(&self, msg: IncomingMessage, registry: &Arc<PluginRegistry>, profile: &AgentProfile) -> Result<Option<OutgoingMessage>> {
        match self {
            Self::Stub => self.process_stub(msg).await,
            Self::Full { chain } => {
                let mut ctx = PipelineContext::new(msg, profile.clone());
                ctx.extensions.insert(Arc::clone(registry));
                chain.execute(ctx).await
            }
        }
    }

    async fn process_stub(&self, msg: IncomingMessage) -> Result<Option<OutgoingMessage>> {
        Ok(Some(OutgoingMessage {
            chat_id: msg.chat_id,
            text: format!("[pipeline placeholder] Received: {}", msg.text),
            parse_mode: None,
            reply_to: None,
            platform: None,
            topic_id: None,
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
            topic_id: None,
            channel_context: None,
            is_cron: false,
            is_webhook: false,
            is_subagent: false,
        }
    }

    #[tokio::test]
    async fn test_pipeline_processes_message() {
        let pipeline = Pipeline::new();
        let registry = Arc::new(PluginRegistry::new());
        let profile = amanclaw_traits::agent::AgentProfile::default_agent();
        let msg = make_test_message("Hello bot");
        let result = pipeline.process(msg, &registry, &profile).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.is_some());
        assert!(response.unwrap().text.contains("Hello bot"));
    }
}
