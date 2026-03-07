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

Requires [componentize-py](https://github.com/bytecodealliance/componentize-py).

### 1. Create the skill

```python
# my_skill.py
class Skill:
    def metadata(self):
        return {
            "name": "my_python_skill",
            "description": "A skill written in Python",
            "timeout_ms": 10000,
            "version": "0.1.0"
        }

    def parameters(self):
        return '{"type": "object", "properties": {"query": {"type": "string"}}, "required": ["query"]}'

    def execute(self, input):
        import json
        args = json.loads(input.args)
        return {"success": True, "output": f"Hello from Python: {args.get('query')}", "error": None}
```

### 2. Build

```bash
componentize-py -d ../../wit/skill.wit -w skill componentize my_skill -o my-python-skill.wasm
```

### 3. Install

```bash
cp my-python-skill.wasm /path/to/plugins/
```

## Writing a JavaScript Skill Plugin

Requires [jco](https://github.com/bytecodealliance/jco).

### 1. Create the skill

```javascript
// my-skill.js
export function metadata() {
    return {
        name: "my_js_skill",
        description: "A skill written in JavaScript",
        timeoutMs: 10000,
        version: "0.1.0"
    };
}

export function parameters() {
    return JSON.stringify({
        type: "object",
        properties: { query: { type: "string" } },
        required: ["query"]
    });
}

export function execute(input) {
    const args = JSON.parse(input.args);
    return {
        success: true,
        output: `Hello from JS: ${args.query}`,
        error: null
    };
}
```

### 2. Build

```bash
jco componentize my-skill.js -d ../../wit/skill.wit -w skill -o my-js-skill.wasm
```

### 3. Install

```bash
cp my-js-skill.wasm /path/to/plugins/
```

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
