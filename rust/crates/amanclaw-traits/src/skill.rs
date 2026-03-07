use serde::{Deserialize, Serialize};

/// Metadata describing a skill plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillMetadata {
    pub name: String,
    pub description: String,
    pub timeout_ms: u32,
    pub version: String,
}

/// Input passed to a skill when it is executed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillInput {
    pub name: String,
    pub args: String, // JSON string
    pub user_id: String,
    pub platform: String,
}

/// Result returned by a skill after execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillResult {
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
}

/// Tool definition exposed to the LLM for function calling.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters_schema: serde_json::Value,
}

/// Trait for built-in Rust skills (non-WASM, compiled in).
#[async_trait::async_trait]
pub trait Skill: Send + Sync {
    fn metadata(&self) -> SkillMetadata;
    fn parameters_schema(&self) -> serde_json::Value;
    async fn execute(&self, input: SkillInput) -> SkillResult;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skill_metadata() {
        let meta = SkillMetadata {
            name: "web_search".into(),
            description: "Search the web".into(),
            timeout_ms: 15000,
            version: "0.1.0".into(),
        };
        assert_eq!(meta.name, "web_search");
        assert_eq!(meta.timeout_ms, 15000);
    }

    #[test]
    fn test_skill_input_args_parsing() {
        let input = SkillInput {
            name: "web_search".into(),
            args: r#"{"query": "weather KL"}"#.into(),
            user_id: "12345".into(),
            platform: "telegram".into(),
        };
        let args: serde_json::Value = serde_json::from_str(&input.args).unwrap();
        assert_eq!(args["query"], "weather KL");
    }

    #[test]
    fn test_skill_result_success() {
        let result = SkillResult {
            success: true,
            output: "It's sunny in KL".into(),
            error: None,
        };
        assert!(result.success);
        assert!(result.error.is_none());
    }

    #[test]
    fn test_skill_result_failure() {
        let result = SkillResult {
            success: false,
            output: String::new(),
            error: Some("Timed out".into()),
        };
        assert!(!result.success);
        assert_eq!(result.error.unwrap(), "Timed out");
    }

    #[test]
    fn test_tool_definition_serialization() {
        let tool = ToolDefinition {
            name: "web_search".into(),
            description: "Search the web".into(),
            parameters_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "Search query" }
                },
                "required": ["query"]
            }),
        };
        let json = serde_json::to_string(&tool).unwrap();
        assert!(json.contains("web_search"));
    }
}
