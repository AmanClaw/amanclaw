use crate::middleware::{MiddlewareChain, PipelineContext, PipelineMiddleware};
use amanclaw_security::sanitizer::check_injection;
use amanclaw_traits::event::EventEmitter;
use amanclaw_traits::message::OutgoingMessage;
use anyhow::Result;
use std::sync::Arc;

/// Sanitized text stored in extensions for downstream middleware to use.
pub struct SanitizedText(pub String);

/// Middleware that detects prompt injection attempts and sanitizes user input.
/// Internal messages bypass sanitization but still emit the message.received event.
pub struct SanitizeMiddleware {
    emitter: Arc<dyn EventEmitter>,
}

impl SanitizeMiddleware {
    pub fn new(emitter: Arc<dyn EventEmitter>) -> Self {
        Self { emitter }
    }
}

#[async_trait::async_trait]
impl PipelineMiddleware for SanitizeMiddleware {
    async fn process(
        &self,
        mut ctx: PipelineContext,
        next: &MiddlewareChain,
    ) -> Result<Option<OutgoingMessage>> {
        let (clean_text, was_flagged) = if ctx.is_internal {
            (ctx.msg.text.clone(), false)
        } else {
            let (ct, wf) = check_injection(&ctx.msg.text);
            (ct.to_string(), wf)
        };

        if was_flagged {
            tracing::warn!(user_id = %ctx.msg.user_id, "Flagged message");
            self.emitter.emit(
                "security.injection",
                serde_json::json!({
                    "user_id": ctx.msg.user_id, "platform": ctx.msg.platform
                }),
            );
        }

        self.emitter.emit(
            "message.received",
            serde_json::json!({
                "user_id": ctx.msg.user_id, "platform": ctx.msg.platform, "agent": ctx.profile.id
            }),
        );

        ctx.extensions.insert(SanitizedText(clean_text));

        next.execute(ctx).await
    }
}
