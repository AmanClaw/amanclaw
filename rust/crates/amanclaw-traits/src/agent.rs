use serde::{Deserialize, Serialize};
use crate::config::LlmConfig;

/// Per-agent context configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextConfig {
    #[serde(default = "default_history_limit")]
    pub history_limit: i64,

    #[serde(default = "default_summarize_threshold")]
    pub summarize_threshold: i64,

    #[serde(default = "default_summarize_keep_recent")]
    pub summarize_keep_recent: i64,

    #[serde(default)]
    pub rag_enabled: bool,

    #[serde(default)]
    pub rag_collections: Vec<String>,

    #[serde(default = "default_rag_top_k")]
    pub rag_top_k: usize,

    #[serde(default = "default_max_tool_rounds")]
    pub max_tool_rounds: usize,
}

impl Default for ContextConfig {
    fn default() -> Self {
        Self {
            history_limit: default_history_limit(),
            summarize_threshold: default_summarize_threshold(),
            summarize_keep_recent: default_summarize_keep_recent(),
            rag_enabled: false,
            rag_collections: Vec::new(),
            rag_top_k: default_rag_top_k(),
            max_tool_rounds: default_max_tool_rounds(),
        }
    }
}

fn default_history_limit() -> i64 { 20 }
fn default_summarize_threshold() -> i64 { 40 }
fn default_summarize_keep_recent() -> i64 { 10 }
fn default_rag_top_k() -> usize { 3 }
fn default_max_tool_rounds() -> usize { 5 }

/// An agent profile defines a persona with its own system prompt,
/// skill subset, and memory namespace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentProfile {
    pub id: String,
    pub name: String,
    pub system_prompt: String,

    /// Skills this agent can use. Empty = all skills.
    #[serde(default)]
    pub allowed_skills: Vec<String>,

    /// Optional LLM override (different model for this agent).
    #[serde(default)]
    pub llm_override: Option<LlmConfig>,

    /// Path to a SOUL.md file (relative to soul_dir). Overrides system_prompt if set.
    #[serde(default)]
    pub soul_file: Option<String>,

    /// Memory namespace — isolates conversation history.
    /// Defaults to the agent id.
    #[serde(default)]
    pub memory_namespace: String,

    #[serde(default)]
    pub context: ContextConfig,
}

impl AgentProfile {
    /// Create a default agent profile that uses the base system prompt.
    pub fn default_agent() -> Self {
        Self {
            id: "default".into(),
            name: "AmanClaw".into(),
            system_prompt: String::new(), // Empty means use base prompt
            allowed_skills: Vec::new(),
            llm_override: None,
            soul_file: None,
            memory_namespace: "default".into(),
            context: ContextConfig::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_agent_profile() {
        let profile = AgentProfile::default_agent();
        assert_eq!(profile.id, "default");
        assert_eq!(profile.memory_namespace, "default");
        assert!(profile.allowed_skills.is_empty());
        assert!(profile.llm_override.is_none());
    }

    #[test]
    fn test_context_config_defaults() {
        let config = ContextConfig::default();
        assert_eq!(config.history_limit, 20);
        assert_eq!(config.summarize_threshold, 40);
        assert_eq!(config.summarize_keep_recent, 10);
        assert!(!config.rag_enabled);
        assert_eq!(config.rag_top_k, 3);
    }

    #[test]
    fn test_agent_profile_with_soul_file() {
        let yaml = r#"
id: ustazbot
name: UstazBot
system_prompt: ""
soul_file: "ustazbot.md"
allowed_skills:
  - solat
memory_namespace: ustaz
"#;
        let profile: AgentProfile = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(profile.soul_file.as_deref(), Some("ustazbot.md"));
    }

    #[test]
    fn test_agent_profile_deserialization() {
        let yaml = r#"
id: ustazbot
name: UstazBot
system_prompt: "You are an Islamic knowledge expert."
allowed_skills:
  - solat
  - qiblat
  - hijri
memory_namespace: ustaz
context:
  history_limit: 30
  rag_enabled: true
  rag_collections:
    - quran_ayat
    - hadith_texts
  rag_top_k: 5
"#;
        let profile: AgentProfile = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(profile.id, "ustazbot");
        assert_eq!(profile.allowed_skills.len(), 3);
        assert_eq!(profile.context.history_limit, 30);
        assert!(profile.context.rag_enabled);
        assert_eq!(profile.context.rag_top_k, 5);
    }
}
