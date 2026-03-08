use amanclaw_traits::agent::AgentProfile;
use amanclaw_traits::context::{ContextEngine, ContextRequest, ExchangeEvent};
use amanclaw_traits::memory::MemoryBackend;
use amanclaw_traits::message::{IncomingMessage, OutgoingMessage};
use amanclaw_traits::skill::SkillInput;
use amanclaw_security::auth::{Auth, UserState};
use amanclaw_security::rate_limiter::RateLimiter;
use amanclaw_security::sanitizer::check_injection;
use amanclaw_llm::client::{LlmClient, LlmResponse};
use crate::context_engine::maybe_summarize;
use crate::registry::PluginRegistry;
use anyhow::Result;
use std::sync::{Arc, Mutex};

const MAX_TOOL_ROUNDS: usize = 5;

/// Message processing pipeline.
pub enum Pipeline {
    Full {
        auth: Arc<Mutex<Auth>>,
        rate_limiter: Mutex<RateLimiter>,
        context_engine: Arc<dyn ContextEngine>,
        memory: Arc<dyn MemoryBackend>,
        llm: Arc<LlmClient>,
    },
    Stub,
}

impl Pipeline {
    pub fn new() -> Self {
        Self::Stub
    }

    pub fn with_services(
        auth: Arc<Mutex<Auth>>,
        rate_limiter: RateLimiter,
        context_engine: Arc<dyn ContextEngine>,
        memory: Arc<dyn MemoryBackend>,
        llm: Arc<LlmClient>,
    ) -> Self {
        Self::Full {
            auth,
            rate_limiter: Mutex::new(rate_limiter),
            context_engine,
            memory,
            llm,
        }
    }

    pub async fn process(&self, msg: IncomingMessage, registry: &PluginRegistry, profile: &AgentProfile) -> Result<Option<OutgoingMessage>> {
        match self {
            Self::Stub => self.process_stub(msg).await,
            Self::Full { auth, rate_limiter, context_engine, memory, llm } => {
                Self::process_full(auth, rate_limiter, context_engine, memory, llm, registry, msg, profile).await
            }
        }
    }

    async fn process_stub(&self, msg: IncomingMessage) -> Result<Option<OutgoingMessage>> {
        Ok(Some(OutgoingMessage {
            chat_id: msg.chat_id,
            text: format!("[pipeline placeholder] Received: {}", msg.text),
            parse_mode: None,
            reply_to: None,
            platform: None,
            topic_id: None,
        }))
    }

    async fn process_full(
        auth: &Mutex<Auth>,
        rate_limiter: &Mutex<RateLimiter>,
        context_engine: &Arc<dyn ContextEngine>,
        memory: &Arc<dyn MemoryBackend>,
        llm: &Arc<LlmClient>,
        registry: &PluginRegistry,
        msg: IncomingMessage,
        profile: &AgentProfile,
    ) -> Result<Option<OutgoingMessage>> {
        let user_id = &msg.user_id;
        let platform = &msg.platform;
        let text = msg.text.trim();
        let ns = &profile.memory_namespace;

        // Handle /myid before auth
        if text == "/myid" || text == "/start" {
            let reply = format!("Your user ID: `{}`\nPlatform: {}", user_id, platform);
            return Ok(Some(OutgoingMessage {
                chat_id: msg.chat_id,
                text: reply,
                parse_mode: Some("Markdown".into()),
                reply_to: None,
                platform: None,
                topic_id: None,
            }));
        }

        // 1. Auth check
        let state = auth.lock().unwrap().get_user_state(user_id, platform);
        match state {
            UserState::Blocked => return Ok(None),
            UserState::New => {
                auth.lock().unwrap().register_user(user_id, platform);
                return Ok(Some(OutgoingMessage {
                    chat_id: msg.chat_id,
                    text: "Welcome! You've been registered. An admin needs to approve your access.".into(),
                    parse_mode: None,
                    reply_to: None,
                    platform: None,
                    topic_id: None,
                }));
            }
            UserState::Pending => {
                return Ok(Some(OutgoingMessage {
                    chat_id: msg.chat_id,
                    text: "Your registration is pending approval.".into(),
                    parse_mode: None,
                    reply_to: None,
                    platform: None,
                    topic_id: None,
                }));
            }
            UserState::Admin | UserState::Approved => {}
        }

        // Handle commands
        if text.starts_with('/') {
            if let Some(reply) = Self::handle_command(auth, memory.as_ref(), &msg, &state).await? {
                return Ok(Some(reply));
            }
        }

        // 2. Rate limit
        if !rate_limiter.lock().unwrap().check(user_id) {
            return Ok(Some(OutgoingMessage {
                chat_id: msg.chat_id,
                text: "Slow down — too many messages. Try again in a minute.".into(),
                parse_mode: None,
                reply_to: None,
                platform: None,
                topic_id: None,
            }));
        }

        // 3. Sanitize
        let (clean_text, was_flagged) = check_injection(&msg.text);
        if was_flagged {
            tracing::warn!(user_id, "Flagged message");
        }

        // 4. Build context via ContextEngine
        let ctx_request = ContextRequest {
            user_id: user_id.clone(),
            platform: platform.clone(),
            namespace: ns.clone(),
            user_message: clean_text.to_string(),
            image_data: msg.image_data.clone(),
            agent_profile: profile.clone(),
        };
        let ctx = context_engine.build_context(ctx_request).await?;
        let mut messages = ctx.messages;
        let tools = ctx.tools;

        // 5. Tool calling loop
        let response = Self::tool_calling_loop(llm, registry, &mut messages, &tools, user_id, platform).await?;

        // 6. Save exchange via ContextEngine
        context_engine.on_exchange_complete(ExchangeEvent {
            user_id: user_id.clone(),
            platform: platform.clone(),
            namespace: ns.clone(),
            user_message: msg.text.clone(),
            assistant_response: response.clone(),
        }).await?;

        // 7. Auto-summarize if history is too long
        if let Err(e) = maybe_summarize(
            memory.as_ref(), llm, ns, user_id,
            profile.context.summarize_threshold,
            profile.context.summarize_keep_recent,
        ).await {
            tracing::error!(error = %e, "Failed to auto-summarize");
        }

        Ok(Some(OutgoingMessage {
            chat_id: msg.chat_id,
            text: response,
            parse_mode: None,
            reply_to: None,
            platform: None,
            topic_id: None,
        }))
    }

    /// Execute the LLM tool calling loop: call LLM, execute tools, repeat until text response.
    async fn tool_calling_loop(
        llm: &LlmClient,
        registry: &PluginRegistry,
        messages: &mut Vec<serde_json::Value>,
        tools: &[amanclaw_traits::skill::ToolDefinition],
        user_id: &str,
        platform: &str,
    ) -> Result<String> {
        for round in 0..MAX_TOOL_ROUNDS {
            let (response, raw_message) = match llm.call_raw(messages, tools).await {
                Ok(r) => r,
                Err(e) => {
                    tracing::error!(error = %e, "LLM error");
                    return Ok("Something went wrong talking to the AI. Try again in a moment.".into());
                }
            };

            match response {
                LlmResponse::Text(text) => {
                    return Ok(text);
                }
                LlmResponse::ToolCalls(calls) => {
                    tracing::info!(round, count = calls.len(), "LLM requested tool calls");

                    // Append assistant message with tool calls
                    messages.push(raw_message);

                    // Execute each tool call and append results
                    for call in &calls {
                        tracing::info!(tool = %call.name, id = %call.id, "Executing skill");

                        let input = SkillInput {
                            name: call.name.clone(),
                            args: call.arguments.clone(),
                            user_id: user_id.to_string(),
                            platform: platform.to_string(),
                        };

                        let result = if let Some(r) = registry.execute(&call.name, input).await {
                            if r.success {
                                format!("[SKILL OUTPUT]\n{}", r.output)
                            } else {
                                format!("[SKILL ERROR]\n{}", r.error.unwrap_or_else(|| "Unknown error".into()))
                            }
                        } else {
                            format!("Skill '{}' not found", call.name)
                        };

                        messages.push(serde_json::json!({
                            "role": "tool",
                            "tool_call_id": call.id,
                            "content": result,
                        }));
                    }
                }
            }
        }

        // Exceeded max rounds — ask LLM for final answer without tools
        match llm.call(messages, &[]).await {
            Ok(LlmResponse::Text(text)) => Ok(text),
            Ok(LlmResponse::ToolCalls(_)) => Ok("I got stuck in a tool loop. Please try rephrasing your question.".into()),
            Err(e) => {
                tracing::error!(error = %e, "LLM error in final round");
                Ok("Something went wrong. Try again.".into())
            }
        }
    }

    async fn handle_command(
        auth: &Mutex<Auth>,
        memory: &dyn MemoryBackend,
        msg: &IncomingMessage,
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
                Some(format!("Messages in history: {}", count))
            }
            "/approve" if *state == UserState::Admin => {
                if let Some(target) = parts.get(1) {
                    auth.lock().unwrap().approve_user(target, &msg.platform);
                    Some(format!("User `{}` approved.", target))
                } else {
                    Some("Usage: /approve <user_id>".into())
                }
            }
            "/block" if *state == UserState::Admin => {
                if let Some(target) = parts.get(1) {
                    auth.lock().unwrap().block_user(target, &msg.platform);
                    Some(format!("User `{}` blocked.", target))
                } else {
                    Some("Usage: /block <user_id>".into())
                }
            }
            "/users" if *state == UserState::Admin => {
                let users = auth.lock().unwrap().list_users();
                if users.is_empty() {
                    Some("No registered users.".into())
                } else {
                    let mut lines = vec!["Registered users:".to_string()];
                    for (uid, plat, st) in &users {
                        lines.push(format!("  {} ({}) — {}", uid, plat, st));
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
                        lines.push(format!("  *{}*: {}", k, v));
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
                    Some(format!("Got it! I'll remember that your {} is: {}", key, value))
                }
            }
            "/forget" => {
                if let Some(key) = parts.get(1) {
                    if memory.delete_fact(&msg.user_id, key).await? {
                        Some(format!("Forgot your {}.", key))
                    } else {
                        Some(format!("I don't have anything stored for '{}'.", key))
                    }
                } else {
                    Some("Usage: /forget <key>".into())
                }
            }
            "/approve" | "/block" | "/users" => {
                Some("Admin only command.".into())
            }
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
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_message(text: &str) -> IncomingMessage {
        IncomingMessage {
            user_id: "admin1".into(),
            chat_id: "admin1".into(),
            platform: "telegram".into(),
            text: text.into(),
            username: Some("testuser".into()),
            first_name: Some("Test".into()),
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

    #[tokio::test]
    async fn test_pipeline_processes_message() {
        let pipeline = Pipeline::new();
        let registry = PluginRegistry::new();
        let profile = amanclaw_traits::agent::AgentProfile::default_agent();
        let msg = make_test_message("Hello bot");
        let result = pipeline.process(msg, &registry, &profile).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.is_some());
        assert!(response.unwrap().text.contains("Hello bot"));
    }
}
