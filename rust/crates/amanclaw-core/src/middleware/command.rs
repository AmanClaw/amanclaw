use crate::middleware::{MiddlewareChain, PipelineContext, PipelineMiddleware};
use amanclaw_security::auth::{Auth, UserState};
use amanclaw_traits::memory::MemoryBackend;
use amanclaw_traits::message::OutgoingMessage;
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Middleware that handles bot commands (/myid, /start, /clear, /stats, admin commands, etc.).
/// Internal messages bypass this middleware.
pub struct CommandMiddleware {
    auth: Arc<RwLock<Auth>>,
    memory: Arc<dyn MemoryBackend>,
}

impl CommandMiddleware {
    pub fn new(auth: Arc<RwLock<Auth>>, memory: Arc<dyn MemoryBackend>) -> Self {
        Self { auth, memory }
    }
}

#[async_trait::async_trait]
impl PipelineMiddleware for CommandMiddleware {
    async fn process(
        &self,
        ctx: PipelineContext,
        next: &MiddlewareChain,
    ) -> Result<Option<OutgoingMessage>> {
        if ctx.is_internal {
            return next.execute(ctx).await;
        }

        let text = ctx.msg.text.trim();

        // Handle /myid and /start before anything else
        if text == "/myid" || text == "/start" {
            let reply = format!(
                "Your user ID: `{}`\nPlatform: {}",
                ctx.msg.user_id, ctx.msg.platform
            );
            return Ok(Some(OutgoingMessage {
                chat_id: ctx.msg.chat_id,
                text: reply,
                parse_mode: Some("Markdown".into()),
                reply_to: None,
                platform: None,
                topic_id: None,
            }));
        }

        // Handle other slash commands
        if text.starts_with('/') {
            let state = ctx
                .extensions
                .get::<UserState>()
                .cloned()
                .unwrap_or(UserState::Approved);
            if let Some(reply) =
                handle_command(&self.auth, self.memory.as_ref(), &ctx.msg, &state).await?
            {
                return Ok(Some(reply));
            }
        }

        next.execute(ctx).await
    }
}

/// Process slash commands. Returns Some(reply) if the command was handled, None to continue.
async fn handle_command(
    auth: &RwLock<Auth>,
    memory: &dyn MemoryBackend,
    msg: &amanclaw_traits::message::IncomingMessage,
    state: &UserState,
) -> Result<Option<OutgoingMessage>> {
    let text = msg.text.trim();
    let parts: Vec<&str> = text.splitn(3, ' ').collect();
    let cmd = parts[0];
    let ns = "default"; // Commands use default namespace

    let reply = match cmd {
        "/clear" => {
            memory.clear_history(ns, &msg.user_id).await?;
            Some("Conversation history cleared.".into())
        }
        "/stats" => {
            let count = memory.get_message_count(ns, &msg.user_id).await?;
            Some(format!("Messages in history: {count}"))
        }
        "/approve" if *state == UserState::Admin => {
            if let Some(target) = parts.get(1) {
                auth.write().await.approve_user(target, &msg.platform);
                Some(format!("User `{target}` approved."))
            } else {
                Some("Usage: /approve <user_id>".into())
            }
        }
        "/block" if *state == UserState::Admin => {
            if let Some(target) = parts.get(1) {
                auth.write().await.block_user(target, &msg.platform);
                Some(format!("User `{target}` blocked."))
            } else {
                Some("Usage: /block <user_id>".into())
            }
        }
        "/users" if *state == UserState::Admin => {
            let users = auth.read().await.list_users();
            if users.is_empty() {
                Some("No registered users.".into())
            } else {
                let mut lines = vec!["Registered users:".to_string()];
                for (uid, plat, st) in &users {
                    lines.push(format!("  {uid} ({plat}) — {st}"));
                }
                Some(lines.join("\n"))
            }
        }
        "/learned" => {
            let facts = memory.get_facts(&msg.user_id).await?;
            if facts.is_empty() {
                Some("I haven't learned anything about you yet.".into())
            } else {
                let mut lines = vec!["Things I know about you:".to_string()];
                for (k, v) in &facts {
                    lines.push(format!("  *{k}*: {v}"));
                }
                Some(lines.join("\n"))
            }
        }
        "/remember" => {
            if parts.len() < 3 {
                Some("Usage: /remember <key> <value>\nExample: /remember name Aman".into())
            } else {
                let key = parts[1];
                let value = parts[2];
                memory.save_fact(&msg.user_id, key, value).await?;
                Some(format!("Got it! I'll remember that your {key} is: {value}"))
            }
        }
        "/forget" => {
            if let Some(key) = parts.get(1) {
                if memory.delete_fact(&msg.user_id, key).await? {
                    Some(format!("Forgot your {key}."))
                } else {
                    Some(format!("I don't have anything stored for '{key}'."))
                }
            } else {
                Some("Usage: /forget <key>".into())
            }
        }
        "/approve" | "/block" | "/users" => Some("Admin only command.".into()),
        _ => None,
    };

    Ok(reply.map(|text| OutgoingMessage {
        chat_id: msg.chat_id.clone(),
        text,
        parse_mode: None,
        reply_to: None,
        platform: None,
        topic_id: None,
    }))
}
