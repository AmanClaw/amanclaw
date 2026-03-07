use amanclaw_traits::skill::{SkillMetadata, ToolDefinition};
use std::collections::HashMap;

/// Registered skill entry (metadata only — execution is handled by WASM runtime or built-in).
struct RegisteredSkill {
    metadata: SkillMetadata,
    parameters_schema: serde_json::Value,
}

/// Central registry for all available skills (WASM plugins, built-in, MCP).
pub struct PluginRegistry {
    skills: HashMap<String, RegisteredSkill>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self {
            skills: HashMap::new(),
        }
    }

    pub fn register_skill(&mut self, metadata: SkillMetadata, parameters_schema: serde_json::Value) {
        tracing::info!(name = %metadata.name, version = %metadata.version, "Registered skill");
        self.skills.insert(metadata.name.clone(), RegisteredSkill {
            metadata,
            parameters_schema,
        });
    }

    pub fn unregister_skill(&mut self, name: &str) {
        if self.skills.remove(name).is_some() {
            tracing::info!(name, "Unregistered skill");
        }
    }

    pub fn has_skill(&self, name: &str) -> bool {
        self.skills.contains_key(name)
    }

    pub fn skill_count(&self) -> usize {
        self.skills.len()
    }

    pub fn get_tool_definitions(&self) -> Vec<ToolDefinition> {
        self.skills
            .values()
            .map(|s| ToolDefinition {
                name: s.metadata.name.clone(),
                description: s.metadata.description.clone(),
                parameters_schema: s.parameters_schema.clone(),
            })
            .collect()
    }

    pub fn get_skill_metadata(&self, name: &str) -> Option<&SkillMetadata> {
        self.skills.get(name).map(|s| &s.metadata)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_and_list_skills() {
        let mut registry = PluginRegistry::new();
        let meta = SkillMetadata {
            name: "test_skill".into(),
            description: "A test skill".into(),
            timeout_ms: 5000,
            version: "0.1.0".into(),
        };
        let schema = serde_json::json!({
            "type": "object",
            "properties": { "query": { "type": "string" } },
            "required": ["query"]
        });
        registry.register_skill(meta, schema);
        assert_eq!(registry.skill_count(), 1);

        let tools = registry.get_tool_definitions();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "test_skill");
    }

    #[test]
    fn test_has_skill() {
        let mut registry = PluginRegistry::new();
        assert!(!registry.has_skill("nonexistent"));

        let meta = SkillMetadata {
            name: "exists".into(),
            description: "test".into(),
            timeout_ms: 1000,
            version: "0.1.0".into(),
        };
        registry.register_skill(meta, serde_json::json!({}));
        assert!(registry.has_skill("exists"));
    }

    #[test]
    fn test_unregister_skill() {
        let mut registry = PluginRegistry::new();
        let meta = SkillMetadata {
            name: "removable".into(),
            description: "test".into(),
            timeout_ms: 1000,
            version: "0.1.0".into(),
        };
        registry.register_skill(meta, serde_json::json!({}));
        assert_eq!(registry.skill_count(), 1);

        registry.unregister_skill("removable");
        assert_eq!(registry.skill_count(), 0);
    }
}
