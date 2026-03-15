use amanclaw_traits::config::LlmConfig;
use amanclaw_traits::skill::ToolDefinition;
use anyhow::Result;
use reqwest::Client;
use serde_json::Value;

use crate::prompts::SYSTEM_PROMPT_BASE;
use crate::tools::{parse_xml_tool_calls, strip_thinking};

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

        if let Some(tools) = tools
            && !tools.is_empty()
        {
            payload["tools"] = Value::Array(tools.to_vec());
            payload["tool_choice"] = Value::String("auto".into());
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

        // Check for JSON tool calls first (OpenAI format)
        if let Some(calls) = Self::parse_tool_calls(message) {
            return Ok(LlmResponse::ToolCalls(calls));
        }

        // Check for XML tool calls in content (Qwen and similar models)
        let content = message["content"].as_str().unwrap_or("");
        if let Some(calls) = parse_xml_tool_calls(content) {
            tracing::info!(
                count = calls.len(),
                "Parsed XML-style tool calls from LLM text"
            );
            return Ok(LlmResponse::ToolCalls(calls));
        }

        Ok(LlmResponse::Text(strip_thinking(content)))
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

        // Check for JSON tool calls first (OpenAI format)
        if let Some(calls) = Self::parse_tool_calls(&message) {
            return Ok((LlmResponse::ToolCalls(calls), message));
        }

        // Check for XML tool calls in content (Qwen and similar models)
        let content = message["content"].as_str().unwrap_or("");
        if let Some(calls) = parse_xml_tool_calls(content) {
            tracing::info!(
                count = calls.len(),
                "Parsed XML-style tool calls from LLM text"
            );
            // Reconstruct the message as if it had proper tool_calls for conversation history
            let tool_calls_json: Vec<Value> = calls
                .iter()
                .map(|c| {
                    serde_json::json!({
                        "id": c.id,
                        "type": "function",
                        "function": {
                            "name": c.name,
                            "arguments": c.arguments,
                        }
                    })
                })
                .collect();
            let synthetic_message = serde_json::json!({
                "role": "assistant",
                "content": null,
                "tool_calls": tool_calls_json,
            });
            return Ok((LlmResponse::ToolCalls(calls), synthetic_message));
        }

        Ok((LlmResponse::Text(strip_thinking(content)), message))
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

    #[test]
    fn test_format_tools_empty() {
        let tools: Vec<ToolDefinition> = vec![];
        let formatted = LlmClient::format_tools(&tools);
        assert!(formatted.is_empty());
    }

    #[test]
    fn test_format_tools_single() {
        let tools = vec![ToolDefinition {
            name: "weather".into(),
            description: "Get weather data".into(),
            parameters_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "city": { "type": "string" }
                }
            }),
        }];
        let formatted = LlmClient::format_tools(&tools);
        assert_eq!(formatted.len(), 1);
        assert_eq!(formatted[0]["type"], "function");
        assert_eq!(formatted[0]["function"]["name"], "weather");
        assert_eq!(formatted[0]["function"]["description"], "Get weather data");
        assert!(formatted[0]["function"]["parameters"]["properties"]["city"].is_object());
    }

    #[test]
    fn test_format_tools_multiple() {
        let tools = vec![
            ToolDefinition {
                name: "weather".into(),
                description: "Get weather".into(),
                parameters_schema: serde_json::json!({"type": "object"}),
            },
            ToolDefinition {
                name: "solat".into(),
                description: "Get prayer times".into(),
                parameters_schema: serde_json::json!({"type": "object"}),
            },
        ];
        let formatted = LlmClient::format_tools(&tools);
        assert_eq!(formatted.len(), 2);
        assert_eq!(formatted[0]["function"]["name"], "weather");
        assert_eq!(formatted[1]["function"]["name"], "solat");
    }

    #[test]
    fn test_parse_tool_calls_valid() {
        let message = serde_json::json!({
            "tool_calls": [{
                "id": "call_1",
                "function": {
                    "name": "weather",
                    "arguments": "{\"city\": \"KL\"}"
                }
            }]
        });
        let calls = LlmClient::parse_tool_calls(&message).unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "weather");
        assert_eq!(calls[0].id, "call_1");
    }

    #[test]
    fn test_parse_tool_calls_empty_array() {
        let message = serde_json::json!({
            "tool_calls": []
        });
        let result = LlmClient::parse_tool_calls(&message);
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_tool_calls_no_field() {
        let message = serde_json::json!({
            "content": "Hello"
        });
        let result = LlmClient::parse_tool_calls(&message);
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_tool_calls_null_field() {
        let message = serde_json::json!({
            "tool_calls": null
        });
        let result = LlmClient::parse_tool_calls(&message);
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_tool_calls_multiple() {
        let message = serde_json::json!({
            "tool_calls": [
                {
                    "id": "call_1",
                    "function": { "name": "weather", "arguments": "{}" }
                },
                {
                    "id": "call_2",
                    "function": { "name": "solat", "arguments": "{\"zone\": \"WLY01\"}" }
                }
            ]
        });
        let calls = LlmClient::parse_tool_calls(&message).unwrap();
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].name, "weather");
        assert_eq!(calls[1].name, "solat");
    }

    #[tokio::test]
    async fn test_llm_api_error_handling() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
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
        let result = client.respond("Hello", &[], &[]).await;
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("500"));
    }

    #[tokio::test]
    async fn test_llm_xml_tool_call_in_content() {
        let mock_server = MockServer::start().await;

        let response_body = serde_json::json!({
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": "<tool_call>\n{\"name\": \"solat\", \"arguments\": {\"zone\": \"WLY01\"}}\n</tool_call>"
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
        let messages = vec![serde_json::json!({"role": "user", "content": "solat time"})];
        let result = client.call(&messages, &[]).await.unwrap();

        match result {
            LlmResponse::ToolCalls(calls) => {
                assert_eq!(calls.len(), 1);
                assert_eq!(calls[0].name, "solat");
            }
            LlmResponse::Text(_) => panic!("Expected tool calls from XML content"),
        }
    }
}
