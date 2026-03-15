//! MCP request handler — routes JSON-RPC methods to implementations.

use amanclaw_traits::skill::{Skill, SkillInput};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

use crate::protocol::*;

/// MCP server handler that wraps a set of skills.
pub struct McpHandler {
    skills: HashMap<String, Arc<dyn Skill>>,
    resources: Vec<McpResource>,
    prompts: Vec<McpPrompt>,
    server_name: String,
    server_version: String,
}

impl McpHandler {
    pub fn new(server_name: impl Into<String>, server_version: impl Into<String>) -> Self {
        Self {
            skills: HashMap::new(),
            resources: Vec::new(),
            prompts: Vec::new(),
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

    pub fn add_resource(&mut self, resource: McpResource) {
        self.resources.push(resource);
    }

    pub fn add_prompt(&mut self, prompt: McpPrompt) {
        self.prompts.push(prompt);
    }

    /// Handle a JSON-RPC request and return a response.
    pub async fn handle(&self, req: JsonRpcRequest) -> Option<JsonRpcResponse> {
        match req.method.as_str() {
            "initialize" => Some(self.handle_initialize(req.id)),
            "initialized" => None, // notification, no response
            "tools/list" => Some(self.handle_tools_list(req.id)),
            "tools/call" => Some(self.handle_tools_call(req.id, req.params).await),
            "resources/list" => Some(self.handle_resources_list(req.id)),
            "resources/read" => Some(self.handle_resources_read(req.id, req.params)),
            "prompts/list" => Some(self.handle_prompts_list(req.id)),
            "prompts/get" => Some(self.handle_prompts_get(req.id, req.params)),
            "ping" => Some(JsonRpcResponse::success(req.id, serde_json::json!({}))),
            _ => Some(JsonRpcResponse::error(
                req.id,
                METHOD_NOT_FOUND,
                format!("Method not found: {}", req.method),
            )),
        }
    }

    fn handle_initialize(&self, id: Option<Value>) -> JsonRpcResponse {
        let mut capabilities = serde_json::json!({
            "tools": { "listChanged": false }
        });

        if !self.resources.is_empty() {
            capabilities["resources"] = serde_json::json!({
                "listChanged": false,
                "subscribe": false,
            });
        }

        if !self.prompts.is_empty() {
            capabilities["prompts"] = serde_json::json!({
                "listChanged": false,
            });
        }

        JsonRpcResponse::success(
            id,
            serde_json::json!({
                "protocolVersion": "2024-11-05",
                "capabilities": capabilities,
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

    fn handle_resources_list(&self, id: Option<Value>) -> JsonRpcResponse {
        JsonRpcResponse::success(id, serde_json::json!({
            "resources": self.resources,
        }))
    }

    fn handle_resources_read(&self, id: Option<Value>, params: Option<Value>) -> JsonRpcResponse {
        let uri = params
            .as_ref()
            .and_then(|p| p.get("uri"))
            .and_then(|u| u.as_str());

        let Some(uri) = uri else {
            return JsonRpcResponse::error(id, INVALID_PARAMS, "Missing 'uri' parameter");
        };

        // Check if resource exists
        if !self.resources.iter().any(|r| r.uri == uri) {
            return JsonRpcResponse::error(id, INVALID_PARAMS, format!("Unknown resource: {uri}"));
        }

        // For now, return empty content — resource backends will be added later
        JsonRpcResponse::success(id, serde_json::json!({
            "contents": [ResourceContent {
                uri: uri.to_string(),
                mime_type: Some("text/plain".into()),
                text: Some(String::new()),
                blob: None,
            }]
        }))
    }

    fn handle_prompts_list(&self, id: Option<Value>) -> JsonRpcResponse {
        JsonRpcResponse::success(id, serde_json::json!({
            "prompts": self.prompts,
        }))
    }

    fn handle_prompts_get(&self, id: Option<Value>, params: Option<Value>) -> JsonRpcResponse {
        let name = params
            .as_ref()
            .and_then(|p| p.get("name"))
            .and_then(|n| n.as_str());

        let Some(name) = name else {
            return JsonRpcResponse::error(id, INVALID_PARAMS, "Missing 'name' parameter");
        };

        let prompt = self.prompts.iter().find(|p| p.name == name);
        let Some(prompt) = prompt else {
            return JsonRpcResponse::error(id, INVALID_PARAMS, format!("Unknown prompt: {name}"));
        };

        // Return prompt with placeholder messages — actual template rendering comes later
        JsonRpcResponse::success(id, serde_json::json!({
            "description": prompt.description,
            "messages": [PromptMessage {
                role: "user".into(),
                content: McpContent {
                    content_type: "text".into(),
                    text: format!("Prompt: {name}"),
                },
            }]
        }))
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

    #[tokio::test]
    async fn test_resources_list() {
        let mut handler = McpHandler::new("test", "0.1.0");
        handler.add_resource(McpResource {
            uri: "file:///test.txt".into(),
            name: "test".into(),
            description: None,
            mime_type: Some("text/plain".into()),
        });

        let req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(serde_json::json!(1)),
            method: "resources/list".into(),
            params: None,
        };

        let resp = handler.handle(req).await.unwrap();
        let result = resp.result.unwrap();
        let resources = result["resources"].as_array().unwrap();
        assert_eq!(resources.len(), 1);
    }

    #[tokio::test]
    async fn test_resources_read() {
        let mut handler = McpHandler::new("test", "0.1.0");
        handler.add_resource(McpResource {
            uri: "file:///test.txt".into(),
            name: "test".into(),
            description: None,
            mime_type: Some("text/plain".into()),
        });

        let req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(serde_json::json!(2)),
            method: "resources/read".into(),
            params: Some(serde_json::json!({"uri": "file:///test.txt"})),
        };

        let resp = handler.handle(req).await.unwrap();
        let result = resp.result.unwrap();
        let contents = result["contents"].as_array().unwrap();
        assert_eq!(contents.len(), 1);
        assert_eq!(contents[0]["uri"], "file:///test.txt");
    }

    #[tokio::test]
    async fn test_resources_read_unknown() {
        let handler = McpHandler::new("test", "0.1.0");
        let req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(serde_json::json!(3)),
            method: "resources/read".into(),
            params: Some(serde_json::json!({"uri": "file:///nonexistent.txt"})),
        };

        let resp = handler.handle(req).await.unwrap();
        assert!(resp.error.is_some());
    }

    #[tokio::test]
    async fn test_prompts_list() {
        let mut handler = McpHandler::new("test", "0.1.0");
        handler.add_prompt(McpPrompt {
            name: "greeting".into(),
            description: Some("Say hello".into()),
            arguments: vec![],
        });

        let req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(serde_json::json!(4)),
            method: "prompts/list".into(),
            params: None,
        };

        let resp = handler.handle(req).await.unwrap();
        let result = resp.result.unwrap();
        let prompts = result["prompts"].as_array().unwrap();
        assert_eq!(prompts.len(), 1);
    }

    #[tokio::test]
    async fn test_prompts_get() {
        let mut handler = McpHandler::new("test", "0.1.0");
        handler.add_prompt(McpPrompt {
            name: "greeting".into(),
            description: Some("Say hello".into()),
            arguments: vec![],
        });

        let req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(serde_json::json!(5)),
            method: "prompts/get".into(),
            params: Some(serde_json::json!({"name": "greeting"})),
        };

        let resp = handler.handle(req).await.unwrap();
        let result = resp.result.unwrap();
        let messages = result["messages"].as_array().unwrap();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0]["role"], "user");
    }

    #[tokio::test]
    async fn test_prompts_get_unknown() {
        let handler = McpHandler::new("test", "0.1.0");
        let req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(serde_json::json!(6)),
            method: "prompts/get".into(),
            params: Some(serde_json::json!({"name": "nonexistent"})),
        };

        let resp = handler.handle(req).await.unwrap();
        assert!(resp.error.is_some());
    }
}
