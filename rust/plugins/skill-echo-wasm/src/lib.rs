//! Example WASM plugin: echoes back input.
//! Build with: cargo build --target wasm32-unknown-unknown --release
//! Copy to plugins/ dir and the engine will auto-load it.

use amanclaw_plugin_sdk::*;

amanclaw_plugin!(
    metadata: SkillMetadata {
        name: "echo".into(),
        description: "Echoes back the user's input (example WASM plugin)".into(),
        timeout_ms: 5000,
        version: "0.1.0".into(),
    },
    parameters: r#"{"type":"object","properties":{"text":{"type":"string","description":"Text to echo back"}},"required":["text"]}"#,
    execute: |input: SkillInput| -> SkillResult {
        let args: serde_json::Value = serde_json::from_str(&input.args)
            .unwrap_or_default();
        let text = args["text"].as_str().unwrap_or("(no text)");
        SkillResult::ok(format!("Echo: {}", text))
    }
);
