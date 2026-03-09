use crate::middleware::{MiddlewareChain, PipelineContext, PipelineMiddleware};
use crate::registry::PluginRegistry;
use amanclaw_llm::client::{LlmClient, LlmResponse};
use amanclaw_traits::context::ContextResult;
use amanclaw_traits::message::OutgoingMessage;
use amanclaw_traits::skill::SkillInput;
use anyhow::Result;
use std::sync::Arc;

/// The LLM response text, stored in extensions for PersistMiddleware to read.
pub struct LlmResponseText(pub String);

/// Terminal middleware that runs the LLM tool-calling loop and produces the response.
pub struct ToolCallingMiddleware {
    llm: Arc<LlmClient>,
}

impl ToolCallingMiddleware {
    pub fn new(llm: Arc<LlmClient>) -> Self {
        Self { llm }
    }
}

#[async_trait::async_trait]
impl PipelineMiddleware for ToolCallingMiddleware {
    async fn process(
        &self,
        mut ctx: PipelineContext,
        _next: &MiddlewareChain,
    ) -> Result<Option<OutgoingMessage>> {
        // Clone Arc<PluginRegistry> first (immutable borrow), then get mutable context_result
        let registry = ctx
            .extensions
            .get::<Arc<PluginRegistry>>()
            .expect("PluginRegistry must be in extensions")
            .clone();

        let context_result = ctx
            .extensions
            .get_mut::<ContextResult>()
            .expect("ContextMiddleware must run before ToolCallingMiddleware");

        let max_rounds = ctx.profile.context.max_tool_rounds;
        let response = tool_calling_loop(
            &self.llm,
            &registry,
            &mut context_result.messages,
            &context_result.tools,
            &ctx.msg.user_id,
            &ctx.msg.platform,
            max_rounds,
        )
        .await?;

        ctx.extensions.insert(LlmResponseText(response.clone()));

        Ok(Some(OutgoingMessage {
            chat_id: ctx.msg.chat_id.clone(),
            text: response,
            parse_mode: None,
            reply_to: None,
            platform: None,
            topic_id: None,
            interactive: None,
        }))
    }
}

/// Execute the LLM tool calling loop: call LLM, execute tools, repeat until text response.
async fn tool_calling_loop(
    llm: &LlmClient,
    registry: &PluginRegistry,
    messages: &mut Vec<serde_json::Value>,
    tools: &[amanclaw_traits::skill::ToolDefinition],
    user_id: &str,
    platform: &str,
    max_rounds: usize,
) -> Result<String> {
    for round in 0..max_rounds {
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
                            format!(
                                "[SKILL ERROR]\n{}",
                                r.error.unwrap_or_else(|| "Unknown error".into())
                            )
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
        Ok(LlmResponse::ToolCalls(_)) => {
            Ok("I got stuck in a tool loop. Please try rephrasing your question.".into())
        }
        Err(e) => {
            tracing::error!(error = %e, "LLM error in final round");
            Ok("Something went wrong. Try again.".into())
        }
    }
}
