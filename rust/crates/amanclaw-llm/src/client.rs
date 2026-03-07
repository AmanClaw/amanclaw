use amanclaw_traits::config::LlmConfig;
use amanclaw_traits::skill::ToolDefinition;
use anyhow::Result;
use reqwest::Client;
use serde_json::Value;

use crate::tools::strip_thinking;
use crate::prompts::SYSTEM_PROMPT_BASE;

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
            payload["tools"] = Value::Array(tools.to_vec());
            payload["tool_choice"] = Value::String("auto".into());
        }

        let api_key = self.config.api_key.as_deref().unwrap_or("no-key");
        let url = format!("{}/chat/completions", self.config.base_url);

        let resp = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&payload)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("LLM API error {}: {}", status, body);
        }

        Ok(resp.json().await?)
    }

    /// Simple respond: send message with history, get text back.
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
    use wiremock::{MockServer, Mock, ResponseTemplate};
    use wiremock::matchers::{method, path};

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

    #[test]
    fn test_strip_thinking_tags() {
        assert_eq!(
            strip_thinking("<think>reasoning here</think>Hello!"),
            "Hello!"
        );
        assert_eq!(
            strip_thinking("Some text</think>Hello!"),
            "Hello!"
        );
        assert_eq!(strip_thinking("No tags here"), "No tags here");
    }
}
