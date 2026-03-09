use crate::middleware::{MiddlewareChain, PipelineContext, PipelineMiddleware};
use amanclaw_security::rate_limiter::RateLimiter;
use amanclaw_traits::event::EventEmitter;
use amanclaw_traits::message::OutgoingMessage;
use anyhow::Result;
use std::sync::Arc;

/// Middleware that enforces per-user rate limiting.
/// Internal messages bypass this middleware.
pub struct RateLimitMiddleware {
    rate_limiter: RateLimiter,
    emitter: Arc<dyn EventEmitter>,
}

impl RateLimitMiddleware {
    pub fn new(rate_limiter: RateLimiter, emitter: Arc<dyn EventEmitter>) -> Self {
        Self { rate_limiter, emitter }
    }
}

#[async_trait::async_trait]
impl PipelineMiddleware for RateLimitMiddleware {
    async fn process(
        &self,
        ctx: PipelineContext,
        next: &MiddlewareChain,
    ) -> Result<Option<OutgoingMessage>> {
        if ctx.is_internal {
            return next.execute(ctx).await;
        }

        if !self.rate_limiter.check(&ctx.msg.user_id) {
            self.emitter.emit("security.rate_limited", serde_json::json!({
                "user_id": ctx.msg.user_id, "platform": ctx.msg.platform
            }));
            return Ok(Some(OutgoingMessage {
                chat_id: ctx.msg.chat_id,
                text: "Slow down — too many messages. Try again in a minute.".into(),
                parse_mode: None,
                reply_to: None,
                platform: None,
                topic_id: None,
            }));
        }

        next.execute(ctx).await
    }
}
