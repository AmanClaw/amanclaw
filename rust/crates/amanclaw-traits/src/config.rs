use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub llm: LlmConfig,

    #[serde(default)]
    pub admin_users: HashMap<String, Vec<String>>,

    #[serde(default = "default_rate_limit")]
    pub rate_limit_per_minute: u32,

    #[serde(default)]
    pub plugins: PluginConfig,

    #[serde(default)]
    pub security: SecurityConfig,

    #[serde(default)]
    pub skills: SkillsConfig,

    #[serde(default)]
    pub mcp_servers: HashMap<String, McpServerConfig>,

    #[serde(default)]
    pub script_plugins: HashMap<String, ScriptPluginConfig>,

    #[serde(default)]
    pub agents: HashMap<String, crate::agent::AgentProfile>,

    #[serde(default)]
    pub routing: RoutingConfig,

    #[serde(default)]
    pub embeddings: Option<EmbeddingConfig>,

    #[serde(default)]
    pub vector: Option<VectorConfig>,

    #[serde(default)]
    pub knowledge_bases: HashMap<String, KnowledgeBaseConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingConfig {
    pub base_url: String,
    pub model: String,
    #[serde(default)]
    pub api_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorConfig {
    #[serde(default = "default_vector_backend")]
    pub backend: String,
    #[serde(default)]
    pub qdrant_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeBaseConfig {
    pub collection: String,
    pub source: String,
}

fn default_vector_backend() -> String { "sqlite-vec".into() }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptPluginConfig {
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    pub base_url: String,
    pub model: String,

    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,

    #[serde(default = "default_temperature")]
    pub temperature: f32,

    #[serde(default)]
    pub api_key: Option<String>,

    #[serde(default)]
    pub native_tool_calling: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PluginConfig {
    #[serde(default = "default_plugin_dir")]
    pub dir: String,

    #[serde(default)]
    pub hot_reload: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SecurityConfig {
    #[serde(default = "default_injection_rules")]
    pub injection_rules: String,

    #[serde(default = "default_true")]
    pub sanitize_output: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SkillsConfig {
    #[serde(default)]
    pub shell_allowed_commands: Vec<String>,

    #[serde(default)]
    pub workspace_dir: Option<String>,

    #[serde(default = "default_skill_timeout")]
    pub skill_timeout_seconds: u32,

    /// List of skill names to disable (not registered with the engine).
    #[serde(default)]
    pub disabled: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    /// Command to spawn (stdio transport). Mutually exclusive with `url`.
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub args: Vec<String>,
    /// Environment variables to set for the spawned process.
    #[serde(default)]
    pub env: HashMap<String, String>,
    /// HTTP URL for remote MCP servers. Mutually exclusive with `command`.
    #[serde(default)]
    pub url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RoutingConfig {
    #[serde(default)]
    pub rules: Vec<RoutingRule>,

    #[serde(default = "default_agent_id")]
    pub default_agent: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingRule {
    #[serde(rename = "match")]
    pub match_criteria: RoutingMatch,
    pub agent: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RoutingMatch {
    #[serde(default)]
    pub platform: Option<String>,
    #[serde(default)]
    pub topic_id: Option<String>,
    #[serde(default)]
    pub channel_id: Option<String>,
    #[serde(default)]
    pub group_id: Option<String>,
}

fn default_agent_id() -> String { "default".into() }
fn default_rate_limit() -> u32 { 20 }
fn default_max_tokens() -> u32 { 4096 }
fn default_temperature() -> f32 { 0.7 }
fn default_plugin_dir() -> String { "./plugins".into() }
fn default_injection_rules() -> String { "default".into() }
fn default_true() -> bool { true }
fn default_skill_timeout() -> u32 { 30 }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_from_yaml() {
        let yaml = r#"
llm:
  base_url: "http://localhost:8001/v1"
  model: "Qwen/Qwen3-VL-30B-A3B-Instruct"
  max_tokens: 4096
  temperature: 0.7

admin_users:
  telegram: ["12345"]

rate_limit_per_minute: 20

plugins:
  dir: "./plugins"
  hot_reload: true

security:
  injection_rules: "default"
  sanitize_output: true
"#;
        let config: AppConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.llm.model, "Qwen/Qwen3-VL-30B-A3B-Instruct");
        assert_eq!(config.llm.max_tokens, 4096);
        assert_eq!(config.rate_limit_per_minute, 20);
        assert!(config.plugins.hot_reload);
        assert_eq!(config.admin_users["telegram"], vec!["12345"]);
    }

    #[test]
    fn test_config_defaults() {
        let yaml = r#"
llm:
  base_url: "http://localhost:8001/v1"
  model: "test-model"
"#;
        let config: AppConfig = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(config.llm.max_tokens, 4096);
        assert_eq!(config.llm.temperature, 0.7);
        assert_eq!(config.rate_limit_per_minute, 20);
    }
}
