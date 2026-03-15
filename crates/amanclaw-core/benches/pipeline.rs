use criterion::{Criterion, criterion_group, criterion_main};

use amanclaw_core::diagnostics::run_startup_diagnostics;
use amanclaw_core::registry::PluginRegistry;
use amanclaw_traits::config::AppConfig;
use amanclaw_traits::skill::{Skill, SkillInput, SkillMetadata, SkillResult};
use std::sync::Arc;

/// A no-op skill for benchmarking dispatch overhead.
struct NoopSkill {
    name: String,
}

impl NoopSkill {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
    }
}

#[async_trait::async_trait]
impl Skill for NoopSkill {
    fn metadata(&self) -> SkillMetadata {
        SkillMetadata {
            name: self.name.clone(),
            description: format!("Noop skill {}", self.name),
            timeout_ms: 5000,
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

    async fn execute(&self, _input: SkillInput) -> SkillResult {
        SkillResult {
            success: true,
            output: "noop".into(),
            error: None,
        }
    }
}

fn make_registry(count: usize) -> PluginRegistry {
    let mut registry = PluginRegistry::new();
    for i in 0..count {
        registry.register(Arc::new(NoopSkill::new(&format!("skill_{i}"))));
    }
    registry
}

fn bench_diagnostics(c: &mut Criterion) {
    let config = AppConfig::default();

    c.bench_function("run_startup_diagnostics", |b| {
        b.iter(|| run_startup_diagnostics(&config))
    });
}

fn bench_registry_tool_definitions(c: &mut Criterion) {
    let mut group = c.benchmark_group("registry_tool_definitions");

    for count in [0, 10, 50] {
        let registry = make_registry(count);
        group.bench_function(format!("{count}_skills"), |b| {
            b.iter(|| registry.get_tool_definitions())
        });
    }

    group.finish();
}

fn bench_registry_filtered_tool_definitions(c: &mut Criterion) {
    let registry = make_registry(50);
    let filter: Vec<String> = (0..5).map(|i| format!("skill_{i}")).collect();

    c.bench_function("filtered_tool_definitions_5_of_50", |b| {
        b.iter(|| registry.get_filtered_tool_definitions(&filter))
    });
}

fn bench_registry_has_skill(c: &mut Criterion) {
    let registry = make_registry(50);

    c.bench_function("has_skill_50_registered", |b| {
        b.iter(|| registry.has_skill("skill_25"))
    });
}

fn bench_registry_skill_metadata(c: &mut Criterion) {
    let registry = make_registry(50);

    c.bench_function("list_skill_metadata_50", |b| {
        b.iter(|| registry.list_skill_metadata())
    });
}

fn bench_tool_definition_serialization(c: &mut Criterion) {
    let registry = make_registry(10);
    let tools = registry.get_tool_definitions();

    c.bench_function("serialize_10_tool_definitions", |b| {
        b.iter(|| serde_json::to_string(&tools).unwrap())
    });
}

fn bench_incoming_message_creation(c: &mut Criterion) {
    use amanclaw_traits::message::IncomingMessage;

    c.bench_function("incoming_message_creation", |b| {
        b.iter(|| IncomingMessage {
            user_id: "12345".into(),
            chat_id: "12345".into(),
            platform: "telegram".into(),
            text: "What time is Subuh prayer in KL?".into(),
            username: Some("testuser".into()),
            first_name: Some("Test".into()),
            is_group: false,
            image_data: None,
            reply_to: None,
            topic_id: None,
            channel_context: None,
            is_cron: false,
            is_webhook: false,
            is_subagent: false,
        })
    });
}

criterion_group!(
    benches,
    bench_diagnostics,
    bench_registry_tool_definitions,
    bench_registry_filtered_tool_definitions,
    bench_registry_has_skill,
    bench_registry_skill_metadata,
    bench_tool_definition_serialization,
    bench_incoming_message_creation,
);
criterion_main!(benches);
