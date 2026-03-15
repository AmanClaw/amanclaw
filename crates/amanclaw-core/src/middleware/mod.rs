pub mod auth;
pub mod command;
pub mod context;
pub mod metrics;
pub mod persist;
pub mod rate_limit;
pub mod rle_detect;
pub mod rle_retrieve;
pub mod sanitize;
pub mod tool_calling;

pub use metrics::MetricsMiddleware;

use amanclaw_traits::agent::AgentProfile;
use amanclaw_traits::message::{IncomingMessage, OutgoingMessage};
use anyhow::Result;
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::Arc;

/// Type-safe extension map for middleware to share data.
#[derive(Default)]
pub struct Extensions {
    map: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
}

impl Extensions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert<T: Send + Sync + 'static>(&mut self, val: T) {
        self.map.insert(TypeId::of::<T>(), Box::new(val));
    }

    pub fn get<T: Send + Sync + 'static>(&self) -> Option<&T> {
        self.map
            .get(&TypeId::of::<T>())
            .and_then(|b| b.downcast_ref())
    }

    pub fn get_mut<T: Send + Sync + 'static>(&mut self) -> Option<&mut T> {
        self.map
            .get_mut(&TypeId::of::<T>())
            .and_then(|b| b.downcast_mut())
    }
}

/// Context passed through the middleware chain.
pub struct PipelineContext {
    pub msg: IncomingMessage,
    pub profile: AgentProfile,
    pub is_internal: bool,
    pub extensions: Extensions,
}

impl PipelineContext {
    pub fn new(msg: IncomingMessage, profile: AgentProfile) -> Self {
        let is_internal = msg.is_cron || msg.is_webhook || msg.is_subagent;
        Self {
            msg,
            profile,
            is_internal,
            extensions: Extensions::new(),
        }
    }
}

/// Middleware trait. Each middleware processes a request and optionally delegates to the next.
#[async_trait::async_trait]
pub trait PipelineMiddleware: Send + Sync {
    /// Process the context. Call `next.execute(ctx)` to continue the chain,
    /// or return directly to short-circuit.
    async fn process(
        &self,
        ctx: PipelineContext,
        next: &MiddlewareChain,
    ) -> Result<Option<OutgoingMessage>>;
}

/// A chain of middleware, executed in order via index tracking.
pub struct MiddlewareChain {
    middlewares: Arc<Vec<Box<dyn PipelineMiddleware>>>,
    start_index: usize,
}

impl MiddlewareChain {
    pub fn new(middlewares: Vec<Box<dyn PipelineMiddleware>>) -> Self {
        Self {
            middlewares: Arc::new(middlewares),
            start_index: 0,
        }
    }

    /// Execute the chain starting from the current index.
    pub async fn execute(&self, ctx: PipelineContext) -> Result<Option<OutgoingMessage>> {
        if self.start_index >= self.middlewares.len() {
            return Ok(None); // end of chain
        }
        let next = MiddlewareChain {
            middlewares: self.middlewares.clone(),
            start_index: self.start_index + 1,
        };
        self.middlewares[self.start_index].process(ctx, &next).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use amanclaw_traits::agent::AgentProfile;
    use amanclaw_traits::message::IncomingMessage;

    fn test_msg() -> IncomingMessage {
        IncomingMessage {
            user_id: "u1".into(),
            chat_id: "c1".into(),
            platform: "test".into(),
            text: "hello".into(),
            username: None,
            first_name: None,
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

    fn test_profile() -> AgentProfile {
        AgentProfile::default_agent()
    }

    #[test]
    fn extensions_insert_and_get() {
        let mut ext = Extensions::new();
        ext.insert(42u32);
        ext.insert("hello".to_string());

        assert_eq!(ext.get::<u32>(), Some(&42));
        assert_eq!(ext.get::<String>(), Some(&"hello".to_string()));
        assert_eq!(ext.get::<bool>(), None);
    }

    #[test]
    fn extensions_get_mut() {
        let mut ext = Extensions::new();
        ext.insert(10u32);

        if let Some(val) = ext.get_mut::<u32>() {
            *val = 20;
        }
        assert_eq!(ext.get::<u32>(), Some(&20));
    }

    #[test]
    fn pipeline_context_is_internal_false_for_normal_msg() {
        let ctx = PipelineContext::new(test_msg(), test_profile());
        assert!(!ctx.is_internal);
    }

    #[test]
    fn pipeline_context_is_internal_true_for_cron() {
        let mut msg = test_msg();
        msg.is_cron = true;
        let ctx = PipelineContext::new(msg, test_profile());
        assert!(ctx.is_internal);
    }

    #[tokio::test]
    async fn empty_chain_returns_none() {
        let chain = MiddlewareChain::new(vec![]);
        let ctx = PipelineContext::new(test_msg(), test_profile());
        let result = chain.execute(ctx).await.unwrap();
        assert!(result.is_none());
    }

    /// Middleware that appends its name to the extensions and delegates to next.
    struct TrackingMiddleware {
        name: &'static str,
    }

    #[async_trait::async_trait]
    impl PipelineMiddleware for TrackingMiddleware {
        async fn process(
            &self,
            mut ctx: PipelineContext,
            next: &MiddlewareChain,
        ) -> Result<Option<OutgoingMessage>> {
            let log = ctx
                .extensions
                .get::<Vec<String>>()
                .cloned()
                .unwrap_or_default();
            let mut log = log;
            log.push(self.name.to_string());
            ctx.extensions.insert(log);
            next.execute(ctx).await
        }
    }

    /// Terminal middleware that returns the tracking log as the response text.
    struct TerminalMiddleware;

    #[async_trait::async_trait]
    impl PipelineMiddleware for TerminalMiddleware {
        async fn process(
            &self,
            ctx: PipelineContext,
            _next: &MiddlewareChain,
        ) -> Result<Option<OutgoingMessage>> {
            let log = ctx
                .extensions
                .get::<Vec<String>>()
                .cloned()
                .unwrap_or_default();
            Ok(Some(OutgoingMessage {
                chat_id: ctx.msg.chat_id.clone(),
                text: log.join(","),
                parse_mode: None,
                platform: None,
                reply_to: None,
                topic_id: None,
                interactive: None,
            }))
        }
    }

    #[tokio::test]
    async fn chain_executes_in_order() {
        let chain = MiddlewareChain::new(vec![
            Box::new(TrackingMiddleware { name: "first" }),
            Box::new(TrackingMiddleware { name: "second" }),
            Box::new(TerminalMiddleware),
        ]);
        let ctx = PipelineContext::new(test_msg(), test_profile());
        let result = chain.execute(ctx).await.unwrap().unwrap();
        assert_eq!(result.text, "first,second");
    }

    /// Middleware that short-circuits without calling next.
    struct ShortCircuitMiddleware;

    #[async_trait::async_trait]
    impl PipelineMiddleware for ShortCircuitMiddleware {
        async fn process(
            &self,
            ctx: PipelineContext,
            _next: &MiddlewareChain,
        ) -> Result<Option<OutgoingMessage>> {
            Ok(Some(OutgoingMessage {
                chat_id: ctx.msg.chat_id.clone(),
                text: "blocked".into(),
                parse_mode: None,
                platform: None,
                reply_to: None,
                topic_id: None,
                interactive: None,
            }))
        }
    }

    #[tokio::test]
    async fn middleware_can_short_circuit() {
        let chain = MiddlewareChain::new(vec![
            Box::new(ShortCircuitMiddleware),
            Box::new(TrackingMiddleware { name: "never" }),
            Box::new(TerminalMiddleware),
        ]);
        let ctx = PipelineContext::new(test_msg(), test_profile());
        let result = chain.execute(ctx).await.unwrap().unwrap();
        // ShortCircuitMiddleware returns "blocked" and never calls next,
        // so TrackingMiddleware and TerminalMiddleware are never reached.
        assert_eq!(result.text, "blocked");
    }
}
