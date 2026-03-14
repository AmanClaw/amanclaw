# amanclaw-traits

Core trait definitions for the AmanClaw AI agent framework.

This crate provides the fundamental traits, types, and interfaces used across all AmanClaw components including skills, memory backends, configuration, and message types.

## Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
amanclaw-traits = "0.1"
```

### Implementing a Skill

```rust
use amanclaw_traits::skill::{Skill, SkillInput, SkillResult, SkillMetadata};

struct MySkill;

#[async_trait::async_trait]
impl Skill for MySkill {
    fn metadata(&self) -> SkillMetadata {
        SkillMetadata {
            name: "my_skill".into(),
            description: "Does something useful".into(),
            timeout_ms: 10000,
            version: "0.1.0".into(),
        }
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "query": { "type": "string" }
            }
        })
    }

    async fn execute(&self, input: SkillInput) -> SkillResult {
        SkillResult {
            success: true,
            output: format!("Got: {}", input.args),
            error: None,
        }
    }
}
```

### Key Traits

- `Skill` - Interface for built-in Rust skills
- `MemoryBackend` - Conversation history and fact storage
- `VectorStore` - Document storage and semantic search
- `ContextEngine` - Context assembly for LLM calls
- `EventEmitter` - Event publishing for observability

## License

MIT
