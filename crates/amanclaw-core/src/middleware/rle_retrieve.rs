use crate::middleware::{MiddlewareChain, PipelineContext, PipelineMiddleware};
use amanclaw_memory::knowledge_store::{CorrectionMatch, CorrectionQuery, KnowledgeStore};
use amanclaw_traits::message::OutgoingMessage;
use anyhow::Result;
use std::sync::Arc;

/// Learned corrections retrieved for this request, stored in extensions.
pub struct LearnedCorrections(pub Vec<CorrectionMatch>);

pub struct RleRetrieveMiddleware {
    store: Arc<KnowledgeStore>,
}

impl RleRetrieveMiddleware {
    pub fn new(store: Arc<KnowledgeStore>) -> Self {
        Self { store }
    }
}

#[async_trait::async_trait]
impl PipelineMiddleware for RleRetrieveMiddleware {
    async fn process(
        &self,
        mut ctx: PipelineContext,
        next: &MiddlewareChain,
    ) -> Result<Option<OutgoingMessage>> {
        let query = CorrectionQuery {
            user_message: ctx.msg.text.clone(),
            user_id: ctx.msg.user_id.clone(),
            community_id: ctx.msg.channel_context.clone(),
            topic: None,
        };

        match self.store.query(&query).await {
            Ok(matches) if !matches.is_empty() => {
                for m in &matches {
                    if let Err(e) = self.store.record_hit(m.rule.id).await {
                        tracing::warn!(
                            rule_id = m.rule.id,
                            error = %e,
                            "Failed to record hit for correction rule"
                        );
                    }
                }
                ctx.extensions.insert(LearnedCorrections(matches));
            }
            Ok(_) => {}
            Err(e) => {
                tracing::warn!(
                    user_id = %ctx.msg.user_id,
                    error = %e,
                    "RleRetrieveMiddleware: failed to query KnowledgeStore, skipping correction injection"
                );
            }
        }

        next.execute(ctx).await
    }
}
