use amanclaw_traits::message::{IncomingMessage, OutgoingMessage};
use amanclaw_traits::skill::SkillInput;
use amanclaw_security::auth::{Auth, UserState};
use amanclaw_security::rate_limiter::RateLimiter;
use amanclaw_security::sanitizer::check_injection;
use amanclaw_memory::sqlite::SqliteMemory;
use amanclaw_llm::client::{LlmClient, LlmResponse};
use crate::registry::PluginRegistry;
use anyhow::Result;
use std::sync::Mutex;
use base64::Engine as Base64Engine;

const MAX_TOOL_ROUNDS: usize = 5;
const SUMMARIZE_THRESHOLD: i64 = 40;
const SUMMARIZE_KEEP_RECENT: i64 = 10;

/// Message processing pipeline.
pub enum Pipeline {
    Full {
        auth: Mutex<Auth>,
        rate_limiter: Mutex<RateLimiter>,
        memory: SqliteMemory,
        llm: LlmClient,
    },
    Stub,
}

impl Pipeline {
    pub fn new() -> Self {
        Self::Stub
    }

    pub fn with_services(
        auth: Auth,
        rate_limiter: RateLimiter,
        memory: SqliteMemory,
        llm: LlmClient,
    ) -> Self {
        Self::Full {
            auth: Mutex::new(auth),
            rate_limiter: Mutex::new(rate_limiter),
            memory,
            llm,
        }
    }

    pub async fn process(&self, msg: IncomingMessage, registry: &PluginRegistry) -> Result<Option<OutgoingMessage>> {
        match self {
            Self::Stub => self.process_stub(msg).await,
            Self::Full { auth, rate_limiter, memory, llm } => {
                Self::process_full(auth, rate_limiter, memory, llm, registry, msg).await
            }
        }
    }

    async fn process_stub(&self, msg: IncomingMessage) -> Result<Option<OutgoingMessage>> {
        Ok(Some(OutgoingMessage {
            chat_id: msg.chat_id,
            text: format!("[pipeline placeholder] Received: {}", msg.text),
            parse_mode: None,
            reply_to: None,
        }))
    }

    async fn process_full(
        auth: &Mutex<Auth>,
        rate_limiter: &Mutex<RateLimiter>,
        memory: &SqliteMemory,
        llm: &LlmClient,
        registry: &PluginRegistry,
        msg: IncomingMessage,
    ) -> Result<Option<OutgoingMessage>> {
        let user_id = &msg.user_id;
        let platform = &msg.platform;
        let text = msg.text.trim();

        // Handle /myid before auth
        if text == "/myid" || text == "/start" {
            let reply = format!("Your user ID: `{}`\nPlatform: {}", user_id, platform);
            return Ok(Some(OutgoingMessage {
                chat_id: msg.chat_id,
                text: reply,
                parse_mode: Some("Markdown".into()),
                reply_to: None,
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
                }));
            }
            UserState::Pending => {
                return Ok(Some(OutgoingMessage {
                    chat_id: msg.chat_id,
                    text: "Your registration is pending approval.".into(),
                    parse_mode: None,
                    reply_to: None,
                }));
            }
            UserState::Admin | UserState::Approved => {}
        }

        // Handle commands
        if text.starts_with('/') {
            if let Some(reply) = Self::handle_command(auth, memory, &msg, &state).await? {
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
            }));
        }

        // 3. Sanitize
        let (clean_text, was_flagged) = check_injection(&msg.text);
        if was_flagged {
            tracing::warn!(user_id, "Flagged message");
        }

        // 4. Build context (with summary + facts)
        let history = memory.get_history(user_id, 20).await?;
        let history_json: Vec<serde_json::Value> = history.iter().map(|m| {
            serde_json::json!({"role": m.role, "content": m.content})
        }).collect();

        let now = chrono::Local::now().format("%Y-%m-%d %H:%M %A").to_string();
        let mut system = amanclaw_llm::prompts::SYSTEM_PROMPT_BASE.replace("{datetime}", &now);

        // Prepend summary if available
        if let Ok(Some(summary)) = memory.get_summary(user_id).await {
            system.push_str(&format!("\n\n## Previous conversation summary\n{}", summary));
        }

        // Append known facts about the user
        if let Ok(facts) = memory.get_facts(user_id).await {
            if !facts.is_empty() {
                system.push_str("\n\n## Known facts about this user");
                for (k, v) in &facts {
                    system.push_str(&format!("\n- {}: {}", k, v));
                }
            }
        }

        let mut messages = vec![serde_json::json!({"role": "system", "content": system})];
        messages.extend_from_slice(&history_json);

        // Build user message — multimodal if image is present
        if let Some(ref image_data) = msg.image_data {
            let b64 = base64::engine::general_purpose::STANDARD.encode(image_data);
            let content = serde_json::json!([
                {"type": "text", "text": if clean_text.is_empty() { "What's in this image?" } else { &clean_text }},
                {"type": "image_url", "image_url": {"url": format!("data:image/jpeg;base64,{}", b64)}}
            ]);
            messages.push(serde_json::json!({"role": "user", "content": content}));
        } else {
            messages.push(serde_json::json!({"role": "user", "content": clean_text}));
        }

        // 5. Tool calling loop
        let tools = registry.get_tool_definitions();
        let response = Self::tool_calling_loop(llm, registry, &mut messages, &tools, user_id, platform).await?;

        // 6. Save exchange
        memory.save_exchange(user_id, platform, &msg.text, &response).await?;

        // 7. Auto-summarize if history is too long
        if memory.needs_summarization(user_id, SUMMARIZE_THRESHOLD).await.unwrap_or(false) {
            let sum_history = memory.get_history(user_id, 100).await?;
            let sum_text: Vec<String> = sum_history.iter()
                .map(|m| format!("{}: {}", m.role, m.content))
                .collect();
            let sum_prompt = format!(
                "Summarize the following conversation concisely. Focus on key topics, decisions, and important context. Reply with ONLY the summary:\n\n{}",
                sum_text.join("\n")
            );
            let sum_messages = vec![
                serde_json::json!({"role": "system", "content": "You are a conversation summarizer. Output only a concise summary."}),
                serde_json::json!({"role": "user", "content": sum_prompt}),
            ];
            match llm.call(&sum_messages, &[]).await {
                Ok(LlmResponse::Text(summary)) => {
                    if let Err(e) = memory.save_summary_and_prune(user_id, &summary, SUMMARIZE_KEEP_RECENT).await {
                        tracing::error!(error = %e, "Failed to save summary");
                    }
                }
                _ => {
                    tracing::warn!("Failed to generate summary for {}", user_id);
                }
            }
        }

        Ok(Some(OutgoingMessage {
            chat_id: msg.chat_id,
            text: response,
            parse_mode: None,
            reply_to: None,
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
        memory: &SqliteMemory,
        msg: &IncomingMessage,
        state: &UserState,
    ) -> Result<Option<OutgoingMessage>> {
        let text = msg.text.trim();
        let parts: Vec<&str> = text.splitn(3, ' ').collect();
        let cmd = parts[0];

        let reply = match cmd {
            "/clear" => {
                memory.clear_history(&msg.user_id).await?;
                Some("Conversation history cleared.".into())
            }
            "/stats" => {
                let count = memory.get_message_count(&msg.user_id).await?;
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
        }
    }

    #[tokio::test]
    async fn test_pipeline_processes_message() {
        let pipeline = Pipeline::new();
        let registry = PluginRegistry::new();
        let msg = make_test_message("Hello bot");
        let result = pipeline.process(msg, &registry).await;
        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.is_some());
        assert!(response.unwrap().text.contains("Hello bot"));
    }
}
