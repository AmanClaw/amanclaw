use amanclaw_traits::skill::{Skill, SkillMetadata, SkillInput, SkillResult, ToolDefinition};
use std::collections::HashMap;
use std::sync::Arc;

/// Central registry for all available skills.
pub struct PluginRegistry {
    skills: HashMap<String, Arc<dyn Skill>>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self {
            skills: HashMap::new(),
        }
    }

    pub fn register(&mut self, skill: Arc<dyn Skill>) {
        let meta = skill.metadata();
        tracing::info!(name = %meta.name, version = %meta.version, "Registered skill");
        self.skills.insert(meta.name.clone(), skill);
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
            .map(|s| {
                let meta = s.metadata();
                ToolDefinition {
                    name: meta.name,
                    description: meta.description,
                    parameters_schema: s.parameters_schema(),
                }
            })
            .collect()
    }

    pub fn iter_skills(&self) -> impl Iterator<Item = (&String, &Arc<dyn Skill>)> {
        self.skills.iter()
    }

    pub fn get_skill_metadata(&self, name: &str) -> Option<SkillMetadata> {
        self.skills.get(name).map(|s| s.metadata())
    }

    pub async fn execute(&self, name: &str, input: SkillInput) -> Option<SkillResult> {
        if let Some(skill) = self.skills.get(name) {
            Some(skill.execute(input).await)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct DummySkill;

    #[async_trait::async_trait]
    impl Skill for DummySkill {
        fn metadata(&self) -> SkillMetadata {
            SkillMetadata {
                name: "test_skill".into(),
                description: "A test skill".into(),
                timeout_ms: 5000,
                version: "0.1.0".into(),
            }
        }

        fn parameters_schema(&self) -> serde_json::Value {
            serde_json::json!({
                "type": "object",
                "properties": { "query": { "type": "string" } },
                "required": ["query"]
            })
        }

        async fn execute(&self, _input: SkillInput) -> SkillResult {
            SkillResult {
                success: true,
                output: "test output".into(),
                error: None,
            }
        }
    }

    #[test]
    fn test_register_and_list_skills() {
        let mut registry = PluginRegistry::new();
        registry.register(Arc::new(DummySkill));
        assert_eq!(registry.skill_count(), 1);

        let tools = registry.get_tool_definitions();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "test_skill");
    }

    #[test]
    fn test_has_skill() {
        let mut registry = PluginRegistry::new();
        assert!(!registry.has_skill("nonexistent"));
        registry.register(Arc::new(DummySkill));
        assert!(registry.has_skill("test_skill"));
    }

    #[tokio::test]
    async fn test_execute_skill() {
        let mut registry = PluginRegistry::new();
        registry.register(Arc::new(DummySkill));

        let input = SkillInput {
            name: "test_skill".into(),
            args: "{}".into(),
            user_id: "test".into(),
            platform: "test".into(),
        };
        let result = registry.execute("test_skill", input).await;
        assert!(result.is_some());
        assert!(result.unwrap().success);
    }
}
