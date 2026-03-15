use crate::middleware::{MiddlewareChain, PipelineContext, PipelineMiddleware};
use amanclaw_security::auth::{Auth, UserState};
use amanclaw_traits::message::OutgoingMessage;
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Middleware that checks user authentication and registration state.
/// Internal messages (cron, webhook, subagent) bypass this middleware.
pub struct AuthMiddleware {
    auth: Arc<RwLock<Auth>>,
}

impl AuthMiddleware {
    pub fn new(auth: Arc<RwLock<Auth>>) -> Self {
        Self { auth }
    }
}

#[async_trait::async_trait]
impl PipelineMiddleware for AuthMiddleware {
    async fn process(
        &self,
        mut ctx: PipelineContext,
        next: &MiddlewareChain,
    ) -> Result<Option<OutgoingMessage>> {
        if ctx.is_internal {
            return next.execute(ctx).await;
        }

        let user_id = &ctx.msg.user_id;
        let platform = &ctx.msg.platform;

        let state = self.auth.read().await.get_user_state(user_id, platform);
        match state {
            UserState::Blocked => return Ok(None),
            UserState::New => {
                self.auth.write().await.register_user(
                    user_id,
                    platform,
                    ctx.msg.username.as_deref(),
                    ctx.msg.first_name.as_deref(),
                );
                return Ok(Some(OutgoingMessage {
                    chat_id: ctx.msg.chat_id,
                    text: "Welcome! You've been registered. An admin needs to approve your access."
                        .into(),
                    parse_mode: None,
                    reply_to: None,
                    platform: None,
                    topic_id: None,
                    interactive: None,
                }));
            }
            UserState::Pending => {
                return Ok(Some(OutgoingMessage {
                    chat_id: ctx.msg.chat_id,
                    text: "Your registration is pending approval.".into(),
                    parse_mode: None,
                    reply_to: None,
                    platform: None,
                    topic_id: None,
                    interactive: None,
                }));
            }
            UserState::Admin | UserState::Approved => {}
        }

        // Update last_seen for active users
        self.auth.read().await.touch_last_seen(user_id, platform);

        // Store the user state in extensions for downstream middleware (e.g., CommandMiddleware)
        ctx.extensions.insert(state);

        next.execute(ctx).await
    }
}
