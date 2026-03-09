//! MCP request handler — routes JSON-RPC methods to implementations.

use amanclaw_traits::skill::{Skill, SkillInput};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

use crate::protocol::*;

/// MCP server handler that wraps a set of skills.
pub struct McpHandler {
    skills: HashMap<String, Arc<dyn Skill>>,
    server_name: String,
    server_version: String,
}

impl McpHandler {
    pub fn new(server_name: impl Into<String>, server_version: impl Into<String>) -> Self {
        Self {
            skills: HashMap::new(),
            server_name: server_name.into(),
            server_version: server_version.into(),
        }
    }

    pub fn register_skill(&mut self, skill: Arc<dyn Skill>) {
        let meta = skill.metadata();
        self.skills.insert(meta.name.clone(), skill);
    }

    pub fn register_skills(&mut self, skills: impl IntoIterator<Item = Arc<dyn Skill>>) {
        for skill in skills {
            self.register_skill(skill);
        }
    }

    /// Handle a JSON-RPC request and return a response.
    pub async fn handle(&self, req: JsonRpcRequest) -> Option<JsonRpcResponse> {
        match req.method.as_str() {
            "initialize" => Some(self.handle_initialize(req.id)),
            "initialized" => None, // notification, no response
            "tools/list" => Some(self.handle_tools_list(req.id)),
            "tools/call" => Some(self.handle_tools_call(req.id, req.params).await),
            "ping" => Some(JsonRpcResponse::success(req.id, serde_json::json!({}))),
            _ => Some(JsonRpcResponse::error(
                req.id,
                METHOD_NOT_FOUND,
                format!("Method not found: {}", req.method),
            )),
        }
    }

    fn handle_initialize(&self, id: Option<Value>) -> JsonRpcResponse {
        JsonRpcResponse::success(
            id,
            serde_json::json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": {
                        "listChanged": false
                    }
                },
                "serverInfo": {
                    "name": self.server_name,
                    "version": self.server_version
                }
            }),
        )
    }

    fn handle_tools_list(&self, id: Option<Value>) -> JsonRpcResponse {
        let tools: Vec<McpTool> = self
            .skills
            .values()
            .map(|skill| {
                let meta = skill.metadata();
                McpTool {
                    name: meta.name,
                    description: meta.description,
                    input_schema: skill.parameters_schema(),
                }
            })
            .collect();

        JsonRpcResponse::success(id, serde_json::json!({ "tools": tools }))
    }

    async fn handle_tools_call(&self, id: Option<Value>, params: Option<Value>) -> JsonRpcResponse {
        let params = match params {
            Some(p) => p,
            None => return JsonRpcResponse::error(id, INVALID_PARAMS, "Missing params"),
        };

        let tool_name = match params.get("name").and_then(|n| n.as_str()) {
            Some(n) => n.to_string(),
            None => return JsonRpcResponse::error(id, INVALID_PARAMS, "Missing tool name"),
        };

        let arguments = params
            .get("arguments")
            .cloned()
            .unwrap_or_else(|| serde_json::json!({}));

        let skill = match self.skills.get(&tool_name) {
            Some(s) => s,
            None => {
                return JsonRpcResponse::error(
                    id,
                    INVALID_PARAMS,
                    format!("Unknown tool: {tool_name}"),
                );
            }
        };

        let input = SkillInput {
            name: tool_name,
            args: arguments.to_string(),
            user_id: "mcp".into(),
            platform: "mcp".into(),
        };

        let result = skill.execute(input).await;

        let content = vec![McpContent {
            content_type: "text".into(),
            text: if result.success {
                result.output
            } else {
                result.error.unwrap_or_else(|| "Unknown error".into())
            },
        }];

        JsonRpcResponse::success(
            id,
            serde_json::json!({
                "content": content,
                "isError": !result.success,
            }),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use amanclaw_traits::skill::{SkillMetadata, SkillResult};

    struct DummySkill;

    #[async_trait::async_trait]
    impl Skill for DummySkill {
        fn metadata(&self) -> SkillMetadata {
            SkillMetadata {
                name: "test_tool".into(),
                description: "A test tool".into(),
                timeout_ms: 5000,
                version: "0.1.0".into(),
            }
        }

        fn parameters_schema(&self) -> serde_json::Value {
            serde_json::json!({
                "type": "object",
                "properties": {
                    "input": { "type": "string" }
                },
                "required": ["input"]
            })
        }

        async fn execute(&self, input: SkillInput) -> SkillResult {
            let args: Value = serde_json::from_str(&input.args).unwrap_or_default();
            let text = args["input"].as_str().unwrap_or("none");
            SkillResult {
                success: true,
                output: format!("Got: {text}"),
                error: None,
            }
        }
    }

    fn make_handler() -> McpHandler {
        let mut handler = McpHandler::new("test-server", "0.1.0");
        handler.register_skill(Arc::new(DummySkill));
        handler
    }

    #[tokio::test]
    async fn test_initialize() {
        let handler = make_handler();
        let req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(Value::Number(1.into())),
            method: "initialize".into(),
            params: None,
        };

        let resp = handler.handle(req).await.unwrap();
        let result = resp.result.unwrap();
        assert_eq!(result["protocolVersion"], "2024-11-05");
        assert_eq!(result["serverInfo"]["name"], "test-server");
    }

    #[tokio::test]
    async fn test_tools_list() {
        let handler = make_handler();
        let req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(Value::Number(2.into())),
            method: "tools/list".into(),
            params: None,
        };

        let resp = handler.handle(req).await.unwrap();
        let result = resp.result.unwrap();
        let tools = result["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0]["name"], "test_tool");
        assert_eq!(tools[0]["description"], "A test tool");
    }

    #[tokio::test]
    async fn test_tools_call() {
        let handler = make_handler();
        let req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(Value::Number(3.into())),
            method: "tools/call".into(),
            params: Some(serde_json::json!({
                "name": "test_tool",
                "arguments": { "input": "hello" }
            })),
        };

        let resp = handler.handle(req).await.unwrap();
        let result = resp.result.unwrap();
        assert_eq!(result["isError"], false);
        let content = result["content"].as_array().unwrap();
        assert_eq!(content[0]["text"], "Got: hello");
    }

    #[tokio::test]
    async fn test_tools_call_unknown_tool() {
        let handler = make_handler();
        let req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(Value::Number(4.into())),
            method: "tools/call".into(),
            params: Some(serde_json::json!({
                "name": "nonexistent",
                "arguments": {}
            })),
        };

        let resp = handler.handle(req).await.unwrap();
        assert!(resp.error.is_some());
        assert!(resp.error.unwrap().message.contains("Unknown tool"));
    }

    #[tokio::test]
    async fn test_ping() {
        let handler = make_handler();
        let req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(Value::Number(5.into())),
            method: "ping".into(),
            params: None,
        };

        let resp = handler.handle(req).await.unwrap();
        assert!(resp.result.is_some());
    }

    #[tokio::test]
    async fn test_method_not_found() {
        let handler = make_handler();
        let req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(Value::Number(6.into())),
            method: "unknown/method".into(),
            params: None,
        };

        let resp = handler.handle(req).await.unwrap();
        assert!(resp.error.is_some());
        assert_eq!(resp.error.unwrap().code, METHOD_NOT_FOUND);
    }
}
