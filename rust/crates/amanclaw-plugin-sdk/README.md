# amanclaw-plugin-sdk

SDK for building AmanClaw WASM plugins in Rust.

This crate provides types and macros to create skill plugins that compile to WebAssembly and run inside the AmanClaw engine.

## Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
amanclaw-plugin-sdk = "0.1"
```

### Creating a WASM Plugin

```rust
use amanclaw_plugin_sdk::*;

amanclaw_plugin!(
    metadata: SkillMetadata {
        name: "my_skill".into(),
        description: "Does something useful".into(),
        timeout_ms: 10000,
        version: "0.1.0".into(),
    },
    parameters: r#"{"type":"object","properties":{"query":{"type":"string"}},"required":["query"]}"#,
    execute: |input: SkillInput| -> SkillResult {
        let args: serde_json::Value = serde_json::from_str(&input.args)
            .unwrap_or_default();
        let query = args["query"].as_str().unwrap_or("none");
        SkillResult::ok(format!("Got query: {query}"))
    }
);
```

### Types

- `SkillMetadata` - Name, description, timeout, and version
- `SkillInput` - Input passed to a skill (name, args JSON, user_id, platform)
- `SkillResult` - Result with success/failure, output text, and optional error

### Building

Compile your plugin to WASM:

```bash
cargo build --target wasm32-wasip1 --release
```

Place the resulting `.wasm` file alongside an `amanclaw-skill.toml` manifest and install it into the AmanClaw engine.

## License

MIT
