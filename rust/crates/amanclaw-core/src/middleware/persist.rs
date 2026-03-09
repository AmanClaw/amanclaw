use crate::context_engine::maybe_summarize;
use crate::middleware::{MiddlewareChain, PipelineContext, PipelineMiddleware};
use amanclaw_llm::client::LlmClient;
use amanclaw_traits::context::{ContextEngine, ExchangeEvent};
use amanclaw_traits::event::EventEmitter;
use amanclaw_traits::memory::MemoryBackend;
use amanclaw_traits::message::OutgoingMessage;
use anyhow::Result;
use std::sync::Arc;

/// Middleware that persists the exchange after the LLM response is generated.
/// Wraps the downstream middleware (ToolCallingMiddleware), captures its response,
/// saves the exchange, and triggers auto-summarization.
pub struct PersistMiddleware {
    context_engine: Arc<dyn ContextEngine>,
    memory: Arc<dyn MemoryBackend>,
    llm: Arc<LlmClient>,
    emitter: Arc<dyn EventEmitter>,
}

impl PersistMiddleware {
    pub fn new(
        context_engine: Arc<dyn ContextEngine>,
        memory: Arc<dyn MemoryBackend>,
        llm: Arc<LlmClient>,
        emitter: Arc<dyn EventEmitter>,
    ) -> Self {
        Self {
            context_engine,
            memory,
            llm,
            emitter,
        }
    }
}

#[async_trait::async_trait]
impl PipelineMiddleware for PersistMiddleware {
    async fn process(
        &self,
        ctx: PipelineContext,
        next: &MiddlewareChain,
    ) -> Result<Option<OutgoingMessage>> {
        // Capture info we need before passing ctx to next
        let user_id = ctx.msg.user_id.clone();
        let platform = ctx.msg.platform.clone();
        let ns = ctx.profile.memory_namespace.clone();
        let user_message = ctx.msg.text.clone();
        let agent_id = ctx.profile.id.clone();
        let summarize_threshold = ctx.profile.context.summarize_threshold;
        let summarize_keep_recent = ctx.profile.context.summarize_keep_recent;

        // Call next middleware (ToolCallingMiddleware) to get the response
        let result = next.execute(ctx).await?;

        // If we got a response, persist the exchange
        if let Some(ref outgoing) = result {
            let response_text = &outgoing.text;

            // Save exchange via ContextEngine
            self.context_engine
                .on_exchange_complete(ExchangeEvent {
                    user_id: user_id.clone(),
                    platform: platform.clone(),
                    namespace: ns.clone(),
                    user_message,
                    assistant_response: response_text.clone(),
                })
                .await?;

            // Auto-summarize if history is too long
            if let Err(e) = maybe_summarize(
                self.memory.as_ref(),
                &self.llm,
                &ns,
                &user_id,
                summarize_threshold,
                summarize_keep_recent,
            )
            .await
            {
                tracing::error!(error = %e, "Failed to auto-summarize");
            }

            self.emitter.emit(
                "message.sent",
                serde_json::json!({
                    "user_id": user_id, "platform": platform, "agent": agent_id,
                    "response_len": response_text.len()
                }),
            );
        }

        Ok(result)
    }
}
