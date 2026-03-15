use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
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

    #[serde(default)]
    pub cron: CronConfig,

    #[serde(default)]
    pub webhooks: WebhookConfig,

    #[serde(default)]
    pub gateway: GatewayConfig,

    #[serde(default)]
    pub subagents: SubAgentConfig,

    #[serde(default)]
    pub registry: RegistryConfig,

    #[serde(default)]
    pub channels: crate::channel_config::ChannelsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RegistryConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_registry_skills_dir")]
    pub skills_dir: String,
    #[serde(default)]
    pub remote_url: Option<String>,
    #[serde(default)]
    pub auto_update_check: bool,
    #[serde(default)]
    pub allow_unverified: bool,
}

fn default_registry_skills_dir() -> String {
    "./plugins/registry".into()
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

fn default_vector_backend() -> String {
    "sqlite-vec".into()
}

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
    #[serde(default)]
    pub base_url: String,
    #[serde(default)]
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

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            base_url: String::new(),
            model: String::new(),
            max_tokens: default_max_tokens(),
            temperature: default_temperature(),
            api_key: None,
            native_tool_calling: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfig {
    #[serde(default = "default_plugin_dir")]
    pub dir: String,

    #[serde(default)]
    pub hot_reload: bool,

    /// Maximum memory (in MB) each WASM plugin can use.
    #[serde(default = "default_wasm_memory_limit_mb")]
    pub wasm_memory_limit_mb: u64,

    /// Fuel budget for WASM plugin execution (limits CPU usage).
    #[serde(default = "default_wasm_fuel_limit")]
    pub wasm_fuel_limit: u64,
}

impl Default for PluginConfig {
    fn default() -> Self {
        Self {
            dir: default_plugin_dir(),
            hot_reload: false,
            wasm_memory_limit_mb: default_wasm_memory_limit_mb(),
            wasm_fuel_limit: default_wasm_fuel_limit(),
        }
    }
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

    /// Directory containing SOUL.md agent personality files.
    #[serde(default = "default_soul_dir")]
    pub soul_dir: String,
}

fn default_soul_dir() -> String {
    "./souls".into()
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

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CronConfig {
    #[serde(default = "default_cron_timezone")]
    pub timezone: String,

    #[serde(default)]
    pub jobs: HashMap<String, CronJobConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CronJobConfig {
    pub name: String,
    pub schedule: String,

    #[serde(default)]
    pub timezone: Option<String>,

    #[serde(rename = "type")]
    pub job_type: String,

    #[serde(default)]
    pub skill: Option<String>,

    #[serde(default)]
    pub input: Option<String>,

    #[serde(default)]
    pub prompt: Option<String>,

    #[serde(default)]
    pub template: Option<String>,

    #[serde(default)]
    pub targets: Vec<CronTargetConfig>,

    #[serde(default)]
    pub agent: Option<String>,

    #[serde(default = "default_true")]
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CronTargetConfig {
    pub platform: String,
    pub chat_id: String,
    #[serde(default)]
    pub topic_id: Option<String>,
}

fn default_cron_timezone() -> String {
    "Asia/Kuala_Lumpur".into()
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WebhookConfig {
    #[serde(default = "default_webhook_base_path")]
    pub base_path: String,

    #[serde(default)]
    pub default_secret: Option<String>,

    #[serde(default)]
    pub endpoints: HashMap<String, WebhookEndpointConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookEndpointConfig {
    pub name: String,
    pub path: String,

    #[serde(default)]
    pub auth: WebhookAuthConfig,

    #[serde(default)]
    pub transform: WebhookTransformConfig,

    #[serde(default)]
    pub targets: Vec<CronTargetConfig>,

    #[serde(default)]
    pub agent: Option<String>,

    #[serde(default)]
    pub rate_limit: Option<u32>,

    #[serde(default = "default_true")]
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookAuthConfig {
    #[serde(rename = "type", default = "default_webhook_auth_type")]
    pub auth_type: String,

    #[serde(default)]
    pub secret: Option<String>,

    #[serde(default)]
    pub header: Option<String>,

    #[serde(default)]
    pub token: Option<String>,

    #[serde(default)]
    pub value: Option<String>,
}

impl Default for WebhookAuthConfig {
    fn default() -> Self {
        Self {
            auth_type: default_webhook_auth_type(),
            secret: None,
            header: None,
            token: None,
            value: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookTransformConfig {
    #[serde(rename = "type", default = "default_webhook_transform_type")]
    pub transform_type: String,

    #[serde(default)]
    pub template: Option<String>,

    #[serde(default)]
    pub message_path: Option<String>,

    #[serde(default)]
    pub title_path: Option<String>,

    #[serde(default)]
    pub prompt_template: Option<String>,

    #[serde(default)]
    pub skill: Option<String>,

    #[serde(default)]
    pub input_template: Option<String>,
}

impl Default for WebhookTransformConfig {
    fn default() -> Self {
        Self {
            transform_type: default_webhook_transform_type(),
            template: None,
            message_path: None,
            title_path: None,
            prompt_template: None,
            skill: None,
            input_template: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GatewayConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_heartbeat")]
    pub heartbeat_interval_secs: u64,
    #[serde(default = "default_max_connections")]
    pub max_connections: usize,
    #[serde(default = "default_stale_timeout")]
    pub stale_session_timeout_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubAgentConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_max_per_session")]
    pub max_per_session: usize,
    #[serde(default = "default_max_global")]
    pub max_global: usize,
    #[serde(default = "default_max_depth")]
    pub max_depth: usize,
    #[serde(default = "default_subagent_timeout")]
    pub default_timeout_secs: u64,
}

impl Default for SubAgentConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_per_session: default_max_per_session(),
            max_global: default_max_global(),
            max_depth: default_max_depth(),
            default_timeout_secs: default_subagent_timeout(),
        }
    }
}

fn default_max_per_session() -> usize {
    5
}
fn default_max_global() -> usize {
    20
}
fn default_max_depth() -> usize {
    2
}
fn default_subagent_timeout() -> u64 {
    120
}

fn default_heartbeat() -> u64 {
    30
}
fn default_max_connections() -> usize {
    50
}
fn default_stale_timeout() -> u64 {
    60
}

fn default_webhook_base_path() -> String {
    "/hooks".into()
}
fn default_webhook_auth_type() -> String {
    "none".into()
}
fn default_webhook_transform_type() -> String {
    "raw_json".into()
}

fn default_agent_id() -> String {
    "default".into()
}
fn default_rate_limit() -> u32 {
    20
}
fn default_max_tokens() -> u32 {
    4096
}
fn default_temperature() -> f32 {
    0.7
}
fn default_plugin_dir() -> String {
    "./plugins".into()
}
fn default_injection_rules() -> String {
    "default".into()
}
fn default_true() -> bool {
    true
}
fn default_skill_timeout() -> u32 {
    30
}
fn default_wasm_memory_limit_mb() -> u64 {
    64
}
fn default_wasm_fuel_limit() -> u64 {
    1_000_000
}

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
