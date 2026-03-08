# AmanClaw Plugin Author Guide

AmanClaw uses WASM Component Model plugins. Write skills in **Rust**, **Python**, or **JavaScript** — they all compile to `.wasm` and run in the same sandboxed runtime.

## Plugin Types

| Type | Purpose | Example |
|------|---------|---------|
| **Skill** | Extends what the bot can do | web search, system info, shell commands |
| **Channel** | Connects to a messaging platform | Telegram, Discord, Slack |

## Writing a Rust Skill Plugin

### 1. Create the crate

```bash
cargo new --lib my-skill
cd my-skill
```

Add to `Cargo.toml`:

```toml
[dependencies]
amanclaw-plugin-sdk = { path = "../../crates/amanclaw-plugin-sdk" }
serde_json = "1"
```

### 2. Implement the skill

```rust
// src/lib.rs
use amanclaw_plugin_sdk::*;

pub fn metadata() -> SkillMetadata {
    SkillMetadata {
        name: "my_skill".into(),
        description: "Does something useful".into(),
        timeout_ms: 10000,
        version: "0.1.0".into(),
    }
}

pub fn parameters() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "properties": {
            "query": { "type": "string", "description": "Input query" }
        },
        "required": ["query"]
    })
}

pub fn execute(input: SkillInput) -> SkillResult {
    let args: serde_json::Value = serde_json::from_str(&input.args)
        .unwrap_or_default();

    let query = args.get("query")
        .and_then(|v| v.as_str())
        .unwrap_or("no query");

    SkillResult::ok(format!("Result for: {}", query))
}
```

### 3. Build and install

```bash
cargo build --release
cp target/release/libmy_skill.so /path/to/plugins/my-skill.wasm
```

## Writing a Python Skill Plugin

Python plugins use the AmanClaw Python SDK with a JSON protocol over stdin/stdout.

### 1. Install the SDK

```bash
pip install -e rust/sdks/python/
```

### 2. Create the skill

```python
#!/usr/bin/env python3
# my_skill.py
from amanclaw_sdk import plugin, SkillInput, SkillResult

@plugin(
    name="my_python_skill",
    description="A skill written in Python",
    parameters={
        "type": "object",
        "properties": {"query": {"type": "string", "description": "Search query"}},
        "required": ["query"]
    }
)
def execute(input: SkillInput) -> SkillResult:
    args = input.parse_args()
    query = args.get("query", "none")
    return SkillResult.ok(f"Hello from Python: {query}")

if __name__ == "__main__":
    execute.run()
```

### 3. Register in config

```yaml
# config.yaml
script_plugins:
  my_python_skill:
    command: "python3"
    args: ["plugins/my_skill.py"]
```

The plugin is automatically discovered on startup and available as a tool for the LLM.

## Writing a JavaScript/TypeScript Skill Plugin (AssemblyScript)

[AssemblyScript](https://www.assemblyscript.org/) compiles TypeScript-like code directly to WASM modules that match AmanClaw's ABI.

### 1. Set up the project

```bash
cp -r rust/sdks/assemblyscript/ my-js-plugin/
cd my-js-plugin
npm install
```

### 2. Edit `assembly/index.ts`

Modify `getMetadata()`, `getParametersSchema()`, and `executeSkill()` with your plugin logic.

### 3. Build

```bash
npm run build
# Output: build/plugin.wasm
```

### 4. Install

```bash
cp build/plugin.wasm /path/to/plugins/
```

The WASM plugin is loaded automatically on startup (or via hot reload).

## Available Host Functions

Plugins can call these functions provided by the host runtime:

| Function | Signature | Description |
|----------|-----------|-------------|
| `http-fetch` | `(req: http-request) -> result<http-response, string>` | Make HTTP requests (domain-restricted) |
| `log` | `(level: string, message: string)` | Log messages (info, warn, error, debug) |
| `get-config` | `(key: string) -> option<string>` | Read allowed config values |
| `get-secret` | `(key: string) -> option<string>` | Read allowed secret values |

## Sandbox Restrictions

All plugins run in a sandboxed WASM environment:

- **No filesystem access** — plugins cannot read or write files
- **No direct network access** — use `http-fetch` host function instead
- **Memory limit** — 64 MB default per plugin
- **Execution timeout** — 30 seconds default, configurable per skill
- **Domain allowlist** — `http-fetch` can be restricted to specific domains
- **Config/secret scoping** — plugins only see keys explicitly granted to them

## WIT Interface

The full contract is defined in `rust/wit/skill.wit`:

```wit
world skill {
    import host;

    export metadata: func() -> skill-metadata;
    export execute: func(input: skill-input) -> skill-result;
    export parameters: func() -> string;
}
```

## Installing Plugins

1. Place `.wasm` files in the configured `plugins.dir` (default: `./plugins`)
2. Restart AmanClaw — plugins are discovered on startup
3. If `plugins.hot_reload` is enabled, new plugins are picked up automatically

## SDK Types Reference

```rust
struct SkillMetadata {
    name: String,          // unique identifier
    description: String,   // shown to users/LLM
    timeout_ms: u32,       // max execution time
    version: String,       // semver
}

struct SkillInput {
    name: String,          // skill being called
    args: String,          // JSON arguments from LLM
    user_id: String,       // who triggered it
    platform: String,      // telegram, discord, etc.
}

struct SkillResult {
    success: bool,
    output: String,        // result text
    error: Option<String>, // error message if !success
}
```
