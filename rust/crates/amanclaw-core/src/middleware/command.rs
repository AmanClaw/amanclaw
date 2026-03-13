use crate::middleware::{MiddlewareChain, PipelineContext, PipelineMiddleware};
use amanclaw_memory::knowledge_store::{DetectedCorrection, KnowledgeStore};
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
    knowledge_store: Option<Arc<KnowledgeStore>>,
}

impl CommandMiddleware {
    pub fn new(auth: Arc<RwLock<Auth>>, memory: Arc<dyn MemoryBackend>) -> Self {
        Self {
            auth,
            memory,
            knowledge_store: None,
        }
    }

    pub fn with_knowledge_store(mut self, store: Arc<KnowledgeStore>) -> Self {
        self.knowledge_store = Some(store);
        self
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
                interactive: None,
            }));
        }

        // Handle knowledge store commands (/learned, /forget, /teach)
        if (text == "/learned"
            || text.starts_with("/learned ")
            || text == "/forget"
            || text.starts_with("/forget ")
            || text.starts_with("/teach "))
            && let Some(reply) = handle_knowledge_command(&self.knowledge_store, &ctx).await?
        {
            return Ok(Some(reply));
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
        interactive: None,
    }))
}

fn make_reply(chat_id: String, text: String) -> OutgoingMessage {
    OutgoingMessage {
        chat_id,
        text,
        parse_mode: None,
        reply_to: None,
        platform: None,
        topic_id: None,
        interactive: None,
    }
}

/// Handle knowledge-store commands: /learned, /forget all, /teach
async fn handle_knowledge_command(
    knowledge_store: &Option<Arc<KnowledgeStore>>,
    ctx: &PipelineContext,
) -> Result<Option<OutgoingMessage>> {
    let store = match knowledge_store {
        Some(s) => s,
        None => {
            return Ok(Some(make_reply(
                ctx.msg.chat_id.clone(),
                "Learning feature is not available.".into(),
            )));
        }
    };

    let text = ctx.msg.text.trim();
    let chat_id = ctx.msg.chat_id.clone();
    let user_id = &ctx.msg.user_id;

    if text == "/learned" {
        let rules = store.get_user_rules(user_id).await?;
        if rules.is_empty() {
            return Ok(Some(make_reply(
                chat_id,
                "I haven't learned anything specific about you yet.".into(),
            )));
        }
        let mut lines = vec!["Things I've learned about you:".to_string()];
        for r in &rules {
            let pct = (r.confidence * 100.0) as u32;
            lines.push(format!(
                "- **{}**: {} ({}% confident, used {}x)",
                r.trigger_pattern, r.correct_response, pct, r.hit_count
            ));
        }
        return Ok(Some(make_reply(chat_id, lines.join("\n"))));
    }

    if text == "/learned community" {
        let community_id = match &ctx.msg.channel_context {
            Some(cid) if !cid.is_empty() => cid.clone(),
            _ => {
                return Ok(Some(make_reply(
                    chat_id,
                    "Community learning is available in group chats.".into(),
                )));
            }
        };
        let rules = store.get_community_rules(&community_id).await?;
        if rules.is_empty() {
            return Ok(Some(make_reply(
                chat_id,
                "No community-level learnings yet.".into(),
            )));
        }
        let mut lines = vec!["Community learnings:".to_string()];
        for r in &rules {
            let pct = (r.confidence * 100.0) as u32;
            lines.push(format!(
                "- **{}**: {} ({}% confident, used {}x)",
                r.trigger_pattern, r.correct_response, pct, r.hit_count
            ));
        }
        return Ok(Some(make_reply(chat_id, lines.join("\n"))));
    }

    if text == "/forget all" {
        let count = store.retract_all_user_rules(user_id).await?;
        return Ok(Some(make_reply(
            chat_id,
            format!("Done. Forgot {count} learned items."),
        )));
    }

    if let Some(fact) = text.strip_prefix("/teach ") {
        let fact = fact.trim();
        if fact.is_empty() {
            return Ok(Some(make_reply(chat_id, "Usage: /teach <fact>".into())));
        }
        let correction = DetectedCorrection {
            trigger: fact.to_string(),
            wrong_response: None,
            correct_response: fact.to_string(),
            topic: None,
            confidence: 0.95,
            signal_type: "explicit_teach".into(),
        };
        store
            .upsert_rule(&correction, Some(user_id), None, "user")
            .await?;
        return Ok(Some(make_reply(
            chat_id,
            "Got it, I'll remember that.".into(),
        )));
    }

    // Not a knowledge command we handle — fall through
    Ok(None)
}
