//! MCP Bridge — wraps external MCP tools as `Arc<dyn Skill>`.
//!
//! Each discovered tool from an external MCP server is wrapped as a
//! `McpBridgeSkill` that implements `Skill`, making it seamlessly
//! available to the LLM alongside built-in and WASM skills.

use amanclaw_traits::config::McpServerConfig;
use amanclaw_traits::skill::{Skill, SkillInput, SkillMetadata, SkillResult};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

use crate::client::McpClient;

/// A skill backed by an external MCP server tool.
pub struct McpBridgeSkill {
    /// Namespaced name: "{server}__{tool_name}"
    namespaced_name: String,
    /// Original tool name on the MCP server
    original_name: String,
    description: String,
    input_schema: Value,
    client: Arc<McpClient>,
}

#[async_trait::async_trait]
impl Skill for McpBridgeSkill {
    fn metadata(&self) -> SkillMetadata {
        SkillMetadata {
            name: self.namespaced_name.clone(),
            description: self.description.clone(),
            timeout_ms: 30000,
            version: "mcp".into(),
        }
    }

    fn parameters_schema(&self) -> Value {
        self.input_schema.clone()
    }

    async fn execute(&self, input: SkillInput) -> SkillResult {
        let arguments: Value =
            serde_json::from_str(&input.args).unwrap_or_else(|_| serde_json::json!({}));

        match self.client.call_tool(&self.original_name, arguments).await {
            Ok(output) => SkillResult {
                success: true,
                output,
                error: None,
            },
            Err(e) => SkillResult {
                success: false,
                output: String::new(),
                error: Some(format!("MCP tool error: {e}")),
            },
        }
    }
}

/// Connect to all configured MCP servers and return bridge skills.
pub async fn connect_all(configs: &HashMap<String, McpServerConfig>) -> Vec<Arc<dyn Skill>> {
    let mut all_skills: Vec<Arc<dyn Skill>> = Vec::new();

    for (server_name, config) in configs {
        match connect_one(server_name, config).await {
            Ok(skills) => {
                tracing::info!(
                    server = %server_name,
                    tools = skills.len(),
                    "MCP server connected"
                );
                all_skills.extend(skills);
            }
            Err(e) => {
                tracing::error!(
                    server = %server_name,
                    error = %e,
                    "Failed to connect to MCP server"
                );
            }
        }
    }

    all_skills
}

/// Connect to a single MCP server and return bridge skills for its tools.
async fn connect_one(
    server_name: &str,
    config: &McpServerConfig,
) -> anyhow::Result<Vec<Arc<dyn Skill>>> {
    let client = if let Some(ref url) = config.url {
        // HTTP transport
        McpClient::connect_http(server_name, url)?
    } else if let Some(ref command) = config.command {
        // Stdio transport
        McpClient::connect_stdio(server_name, command, &config.args, &config.env).await?
    } else {
        anyhow::bail!("MCP server '{server_name}' must have either 'command' or 'url'");
    };

    // Initialize handshake
    let init_result = client.initialize().await?;
    tracing::debug!(
        server = %server_name,
        protocol_version = %init_result.get("protocolVersion").and_then(|v| v.as_str()).unwrap_or("unknown"),
        "MCP server initialized"
    );

    // Send initialized notification
    client.send_initialized().await?;

    // Discover tools
    let tools = client.list_tools().await?;

    let client = Arc::new(client);
    let mut skills: Vec<Arc<dyn Skill>> = Vec::new();

    for tool in tools {
        let namespaced_name = format!("{}__{}", server_name, tool.name);

        tracing::debug!(
            server = %server_name,
            tool = %tool.name,
            namespaced = %namespaced_name,
            "Registered MCP tool"
        );

        skills.push(Arc::new(McpBridgeSkill {
            namespaced_name,
            original_name: tool.name,
            description: tool.description,
            input_schema: tool.input_schema,
            client: client.clone(),
        }));
    }

    Ok(skills)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_namespacing() {
        let name = format!("{}__{}", "github", "create_issue");
        assert_eq!(name, "github__create_issue");
    }

    #[tokio::test]
    async fn test_connect_all_empty() {
        let configs = HashMap::new();
        let skills = connect_all(&configs).await;
        assert!(skills.is_empty());
    }

    #[tokio::test]
    async fn test_connect_all_invalid_server() {
        let mut configs = HashMap::new();
        configs.insert(
            "bad".to_string(),
            McpServerConfig {
                command: Some("nonexistent-command-xyz".into()),
                args: vec![],
                env: HashMap::new(),
                url: None,
            },
        );
        // Should not panic, just log error
        let skills = connect_all(&configs).await;
        assert!(skills.is_empty());
    }

    #[tokio::test]
    async fn test_bridge_skill_metadata() {
        let client = McpClient::connect_http("test", "http://localhost:99999/mcp").unwrap();
        let skill = McpBridgeSkill {
            namespaced_name: "test__echo".into(),
            original_name: "echo".into(),
            description: "Echo tool".into(),
            input_schema: serde_json::json!({"type": "object", "properties": {}}),
            client: Arc::new(client),
        };

        let meta = skill.metadata();
        assert_eq!(meta.name, "test__echo");
        assert_eq!(meta.description, "Echo tool");
        assert_eq!(meta.version, "mcp");
    }
}
