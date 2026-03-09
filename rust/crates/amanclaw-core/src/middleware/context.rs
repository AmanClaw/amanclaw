use crate::middleware::sanitize::SanitizedText;
use crate::middleware::{MiddlewareChain, PipelineContext, PipelineMiddleware};
use amanclaw_traits::context::{ContextEngine, ContextRequest};
use amanclaw_traits::message::OutgoingMessage;
use anyhow::Result;
use std::sync::Arc;

/// Middleware that builds the LLM context (system prompt, history, user message, tools)
/// using the ContextEngine. Stores the result in extensions for downstream middleware.
pub struct ContextMiddleware {
    context_engine: Arc<dyn ContextEngine>,
}

impl ContextMiddleware {
    pub fn new(context_engine: Arc<dyn ContextEngine>) -> Self {
        Self { context_engine }
    }
}

#[async_trait::async_trait]
impl PipelineMiddleware for ContextMiddleware {
    async fn process(
        &self,
        mut ctx: PipelineContext,
        next: &MiddlewareChain,
    ) -> Result<Option<OutgoingMessage>> {
        let clean_text = ctx.extensions.get::<SanitizedText>()
            .map(|s| s.0.clone())
            .unwrap_or_else(|| ctx.msg.text.clone());

        let ctx_request = ContextRequest {
            user_id: ctx.msg.user_id.clone(),
            platform: ctx.msg.platform.clone(),
            namespace: ctx.profile.memory_namespace.clone(),
            user_message: clean_text,
            image_data: ctx.msg.image_data.clone(),
            agent_profile: ctx.profile.clone(),
        };

        let context_result = self.context_engine.build_context(ctx_request).await?;
        ctx.extensions.insert(context_result);

        next.execute(ctx).await
    }
}
