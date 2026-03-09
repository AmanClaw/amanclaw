use super::{MiddlewareChain, PipelineContext, PipelineMiddleware};
use amanclaw_traits::message::OutgoingMessage;
use anyhow::Result;

/// Middleware that records Prometheus metrics for every pipeline invocation.
///
/// Tracked metrics:
/// - `messages_processed_total` (counter) — per platform + agent
/// - `pipeline_duration_seconds` (histogram) — per agent
/// - `pipeline_errors_total` (counter) — per platform
pub struct MetricsMiddleware;

#[async_trait::async_trait]
impl PipelineMiddleware for MetricsMiddleware {
    async fn process(
        &self,
        ctx: PipelineContext,
        next: &MiddlewareChain,
    ) -> Result<Option<OutgoingMessage>> {
        let platform = ctx.msg.platform.clone();
        let agent = ctx.profile.id.clone();
        let start = std::time::Instant::now();

        metrics::counter!("messages_processed_total", "platform" => platform.clone(), "agent" => agent.clone())
            .increment(1);

        let result = next.execute(ctx).await;

        let duration = start.elapsed().as_secs_f64();
        metrics::histogram!("pipeline_duration_seconds", "agent" => agent).record(duration);

        if result.is_err() {
            metrics::counter!("pipeline_errors_total", "platform" => platform).increment(1);
        }

        result
    }
}
