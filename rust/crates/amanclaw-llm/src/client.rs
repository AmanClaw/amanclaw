use amanclaw_traits::config::LlmConfig;
use amanclaw_traits::skill::ToolDefinition;
use anyhow::Result;
use reqwest::Client;
use serde_json::Value;

use crate::prompts::SYSTEM_PROMPT_BASE;
use crate::tools::strip_thinking;

/// A tool call requested by the LLM.
#[derive(Debug, Clone)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: String,
}

/// Result of an LLM call — either a text response or tool call requests.
#[derive(Debug)]
pub enum LlmResponse {
    Text(String),
    ToolCalls(Vec<ToolCall>),
}

pub struct LlmClient {
    client: Client,
    config: LlmConfig,
}

impl LlmClient {
    pub fn new(config: LlmConfig) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .expect("Failed to create HTTP client");

        tracing::info!(model = %config.model, base_url = %config.base_url, "LLM client initialized");

        Self { client, config }
    }

    async fn call_api(&self, messages: &[Value], tools: Option<&[Value]>) -> Result<Value> {
        let mut payload = serde_json::json!({
            "model": self.config.model,
            "messages": messages,
            "max_tokens": self.config.max_tokens,
            "temperature": self.config.temperature,
        });

        if let Some(tools) = tools {
            if !tools.is_empty() {
                payload["tools"] = Value::Array(tools.to_vec());
                payload["tool_choice"] = Value::String("auto".into());
            }
        }

        let api_key = self.config.api_key.as_deref().unwrap_or("no-key");
        let url = format!("{}/chat/completions", self.config.base_url);

        let resp = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {api_key}"))
            .json(&payload)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("LLM API error {status}: {body}");
        }

        Ok(resp.json().await?)
    }

    /// Convert ToolDefinitions to OpenAI tool format.
    fn format_tools(tools: &[ToolDefinition]) -> Vec<Value> {
        tools
            .iter()
            .map(|t| {
                serde_json::json!({
                    "type": "function",
                    "function": {
                        "name": t.name,
                        "description": t.description,
                        "parameters": t.parameters_schema,
                    }
                })
            })
            .collect()
    }

    /// Parse tool calls from an LLM response message.
    fn parse_tool_calls(message: &Value) -> Option<Vec<ToolCall>> {
        let tool_calls = message.get("tool_calls")?;
        let arr = tool_calls.as_array()?;
        if arr.is_empty() {
            return None;
        }

        let calls: Vec<ToolCall> = arr
            .iter()
            .filter_map(|tc| {
                let id = tc.get("id")?.as_str()?.to_string();
                let function = tc.get("function")?;
                let name = function.get("name")?.as_str()?.to_string();
                let arguments = function.get("arguments")?.as_str()?.to_string();
                Some(ToolCall {
                    id,
                    name,
                    arguments,
                })
            })
            .collect();

        if calls.is_empty() { None } else { Some(calls) }
    }

    /// Call the LLM and return either text or tool call requests.
    pub async fn call(&self, messages: &[Value], tools: &[ToolDefinition]) -> Result<LlmResponse> {
        let formatted_tools = Self::format_tools(tools);
        let tool_ref = if formatted_tools.is_empty() {
            None
        } else {
            Some(formatted_tools.as_slice())
        };

        let data = self.call_api(messages, tool_ref).await?;
        let message = &data["choices"][0]["message"];

        // Check for tool calls first
        if let Some(calls) = Self::parse_tool_calls(message) {
            return Ok(LlmResponse::ToolCalls(calls));
        }

        // Otherwise extract text
        let content = message["content"].as_str().unwrap_or("").to_string();

        Ok(LlmResponse::Text(strip_thinking(&content)))
    }

    /// Get the raw assistant message value (for appending to conversation).
    pub async fn call_raw(
        &self,
        messages: &[Value],
        tools: &[ToolDefinition],
    ) -> Result<(LlmResponse, Value)> {
        let formatted_tools = Self::format_tools(tools);
        let tool_ref = if formatted_tools.is_empty() {
            None
        } else {
            Some(formatted_tools.as_slice())
        };

        let data = self.call_api(messages, tool_ref).await?;
        let message = data["choices"][0]["message"].clone();

        if let Some(calls) = Self::parse_tool_calls(&message) {
            return Ok((LlmResponse::ToolCalls(calls), message));
        }

        let content = message["content"].as_str().unwrap_or("").to_string();

        Ok((LlmResponse::Text(strip_thinking(&content)), message))
    }

    /// Simple respond: send message with history, get text back (no tool calling).
    pub async fn respond(
        &self,
        message: &str,
        history: &[Value],
        _tools: &[ToolDefinition],
    ) -> Result<String> {
        let now = chrono::Local::now().format("%Y-%m-%d %H:%M %A").to_string();
        let system = SYSTEM_PROMPT_BASE.replace("{datetime}", &now);

        let mut messages = vec![serde_json::json!({"role": "system", "content": system})];
        messages.extend_from_slice(history);
        messages.push(serde_json::json!({"role": "user", "content": message}));

        let data = self.call_api(&messages, None).await?;

        let content = data["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        Ok(strip_thinking(&content))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn test_llm_respond_simple() {
        let mock_server = MockServer::start().await;

        let response_body = serde_json::json!({
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": "Hello! How can I help you?",
                    "tool_calls": null
                },
                "finish_reason": "stop"
            }]
        });

        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&response_body))
            .mount(&mock_server)
            .await;

        let config = amanclaw_traits::config::LlmConfig {
            base_url: format!("{}/v1", mock_server.uri()),
            model: "test-model".into(),
            max_tokens: 100,
            temperature: 0.7,
            api_key: Some("test-key".into()),
            native_tool_calling: Some(false),
        };

        let client = LlmClient::new(config);
        let result = client.respond("Hello", &[], &[]).await.unwrap();
        assert_eq!(result, "Hello! How can I help you?");
    }

    #[tokio::test]
    async fn test_llm_tool_call_parsing() {
        let mock_server = MockServer::start().await;

        let response_body = serde_json::json!({
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": null,
                    "tool_calls": [{
                        "id": "call_123",
                        "type": "function",
                        "function": {
                            "name": "system_info",
                            "arguments": "{}"
                        }
                    }]
                },
                "finish_reason": "tool_calls"
            }]
        });

        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&response_body))
            .mount(&mock_server)
            .await;

        let config = amanclaw_traits::config::LlmConfig {
            base_url: format!("{}/v1", mock_server.uri()),
            model: "test-model".into(),
            max_tokens: 100,
            temperature: 0.7,
            api_key: Some("test-key".into()),
            native_tool_calling: Some(true),
        };

        let client = LlmClient::new(config);
        let tools = vec![ToolDefinition {
            name: "system_info".into(),
            description: "Get system info".into(),
            parameters_schema: serde_json::json!({"type": "object", "properties": {}}),
        }];

        let messages = vec![serde_json::json!({"role": "user", "content": "check system"})];
        let result = client.call(&messages, &tools).await.unwrap();

        match result {
            LlmResponse::ToolCalls(calls) => {
                assert_eq!(calls.len(), 1);
                assert_eq!(calls[0].name, "system_info");
                assert_eq!(calls[0].id, "call_123");
            }
            LlmResponse::Text(_) => panic!("Expected tool calls"),
        }
    }

    #[test]
    fn test_strip_thinking_tags() {
        use crate::tools::strip_thinking;
        assert_eq!(
            strip_thinking("<think>reasoning here</think>Hello!"),
            "Hello!"
        );
        assert_eq!(strip_thinking("Some text</think>Hello!"), "Hello!");
        assert_eq!(strip_thinking("No tags here"), "No tags here");
    }
}
