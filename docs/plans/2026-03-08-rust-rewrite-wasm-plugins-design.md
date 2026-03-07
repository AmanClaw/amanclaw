# AmanClaw Rust Rewrite with WASM Plugin Architecture

**Date:** 2026-03-08
**Status:** Approved
**Goal:** Rewrite AmanClaw in Rust with a WASM-based plugin system that allows anyone to write skills/channels in any language (Rust, Python, Go, JS, etc.)

---

## 1. High-Level Architecture

```
+-----------------------------------------------------+
|                  amanclaw-cli                        |
|              (binary entrypoint)                     |
+----------------------+------------------------------+
                       |
+----------------------v------------------------------+
|                 amanclaw-core                        |
|  +---------+  +----------+  +-------------------+   |
|  | Router  |  | Pipeline |  | Plugin Registry   |   |
|  |         |--|  auth ->  |  | +---------------+ |   |
|  | msg in  |  | sanitize->|  | | WASM Runtime  | |   |
|  | msg out |  | llm ->   |  | | (wasmtime)    | |   |
|  |         |  | respond  |  | +---------------+ |   |
|  +---------+  +----------+  | | MCP Client    | |   |
|                             | +---------------+ |   |
|                             +-------------------+   |
+--+----------+-----------+-----------+---------------+
   |          |           |           |
+--v---+ +---v---+ +-----v----+ +---v----------------+
| LLM  | |Memory | |Security  | | WASM Plugins       |
|Client| |SQLite/| |Auth+Rate | | +----------------+ |
|reqwst| |Postgrs| |Sanitize  | | | skill.wasm     | |
+------+ +-------+ +----------+ | | channel.wasm   | |
                                 | +----------------+ |
                                 | + MCP servers     |
                                 +--------------------+
```

### Key Decisions

- **Core engine, LLM client, memory, security** -> Rust crates (always compiled in, maximum performance)
- **Skills and channels** -> WASM plugins loaded at runtime (polyglot, sandboxed)
- **MCP servers** -> additional escape hatch for heavy/existing external tools
- **WASM Component Model + WIT** -> the plugin contract (typed interfaces, not raw WASM)

### Plugin Types

| Plugin Type | Interface     | Example                          |
|-------------|---------------|----------------------------------|
| Skill       | `skill.wit`   | web-search, shell, documents     |
| Channel     | `channel.wit` | telegram, discord, slack         |
| Middleware  | `middleware.wit` | custom auth, logging, analytics |

---

## 2. Plugin Interface Design (WIT)

### `wit/skill.wit`

```wit
package amanclaw:skill;

interface types {
    record skill-metadata {
        name: string,
        description: string,
        timeout-ms: u32,
        version: string,
    }

    record skill-input {
        name: string,
        args: string,
        user-id: string,
        platform: string,
    }

    record skill-result {
        success: bool,
        output: string,
        error: option<string>,
    }

    record http-request {
        url: string,
        method: string,
        headers: list<tuple<string, string>>,
        body: option<string>,
    }

    record http-response {
        status: u16,
        body: string,
    }
}

interface host {
    use types.{http-request, http-response};

    http-fetch: func(req: http-request) -> result<http-response, string>;
    log: func(level: string, message: string);
    get-config: func(key: string) -> option<string>;
    get-secret: func(key: string) -> option<string>;
}

world skill {
    import host;
    use types.{skill-metadata, skill-input, skill-result};

    export metadata: func() -> skill-metadata;
    export execute: func(input: skill-input) -> skill-result;
    export parameters: func() -> string;
}
```

### `wit/channel.wit`

```wit
package amanclaw:channel;

interface types {
    record incoming-message {
        user-id: string,
        chat-id: string,
        platform: string,
        text: string,
        username: option<string>,
        image-data: option<list<u8>>,
    }

    record outgoing-message {
        chat-id: string,
        text: string,
    }

    record channel-config {
        name: string,
        version: string,
    }
}

interface engine {
    use types.{incoming-message, outgoing-message};

    process-message: func(msg: incoming-message) -> option<outgoing-message>;
}

world channel {
    import engine;
    use types.{channel-config};

    export config: func() -> channel-config;
    export start: func() -> result<_, string>;
    export stop: func() -> result<_, string>;
}
```

### WASM Sandboxing

Plugins automatically get:
- No direct filesystem access (host must expose it)
- No direct network access (must go through `host::http_fetch`)
- No access to unscoped secrets
- Panic isolation (plugin crash does not crash engine)
- Memory isolation per plugin instance
- Timeout enforcement via wasmtime fuel/epoch interruption

---

## 3. Crate Structure

```
amanclaw/
├── Cargo.toml                       # workspace root
├── config.example.yaml
├── wit/
│   ├── skill.wit
│   ├── channel.wit
│   └── middleware.wit
│
├── crates/
│   ├── amanclaw-cli/                # binary: config, startup, shutdown
│   │   └── src/main.rs
│   │
│   ├── amanclaw-core/               # engine: pipeline, router, plugin registry
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── pipeline.rs
│   │       ├── router.rs
│   │       └── registry.rs
│   │
│   ├── amanclaw-traits/             # shared types (zero dependencies)
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── message.rs
│   │       ├── skill.rs
│   │       ├── channel.rs
│   │       └── config.rs
│   │
│   ├── amanclaw-llm/                # async LLM client
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── client.rs
│   │       ├── prompts.rs
│   │       ├── tools.rs
│   │       └── knowledge.rs
│   │
│   ├── amanclaw-memory/             # storage layer
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── sqlite.rs
│   │       ├── postgres.rs
│   │       └── schema.rs
│   │
│   ├── amanclaw-security/           # auth, rate limiting, sanitizer
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── auth.rs
│   │       ├── rate_limiter.rs
│   │       └── sanitizer.rs
│   │
│   ├── amanclaw-mcp/                # MCP client
│   │   └── src/lib.rs
│   │
│   ├── amanclaw-wasm-runtime/       # WASM plugin loader + executor
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── loader.rs
│   │       ├── host.rs
│   │       └── sandbox.rs
│   │
│   └── amanclaw-plugin-sdk/         # SDK for Rust plugin authors
│       └── src/
│           ├── lib.rs
│           └── macros.rs
│
├── plugins/                         # first-party plugins (compile to .wasm)
│   ├── skill-shell/
│   ├── skill-websearch/
│   ├── skill-documents/
│   ├── skill-remember/
│   ├── skill-reminder/
│   ├── skill-scheduled/
│   ├── skill-sysinfo/
│   ├── skill-webfetch/
│   ├── skill-prayer-times/
│   ├── skill-user-skills/
│   ├── channel-telegram/
│   ├── channel-discord/
│   ├── channel-slack/
│   └── channel-whatsapp/
│
├── sdks/                            # SDKs for other languages
│   ├── python/
│   ├── javascript/
│   └── go/
│
├── deploy/
│   ├── Dockerfile
│   ├── docker-compose.yml
│   └── amanclaw.service
│
└── docs/
    ├── plugin-guide.md
    ├── architecture.md
    └── api-reference.md
```

### Crate Dependency Graph

```
amanclaw-cli
  +-- amanclaw-core
  |     +-- amanclaw-traits        (zero deps, shared types)
  |     +-- amanclaw-llm
  |     |     +-- amanclaw-traits
  |     +-- amanclaw-memory
  |     |     +-- amanclaw-traits
  |     +-- amanclaw-security
  |     |     +-- amanclaw-traits
  |     +-- amanclaw-wasm-runtime
  |     |     +-- amanclaw-traits
  |     |     +-- wasmtime
  |     +-- amanclaw-mcp
  |           +-- amanclaw-traits
  +-- config (yaml loading)

amanclaw-plugin-sdk               (standalone, for plugin authors)
  +-- wit-bindgen
```

### Key Rust Dependencies

```toml
# Core
tokio = { version = "1", features = ["full"] }
reqwest = { version = "0.12", features = ["json"] }
sqlx = { version = "0.8", features = ["sqlite", "runtime-tokio"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
serde_yaml = "0.9"

# WASM
wasmtime = "29"
wasmtime-wasi = "29"
wit-bindgen = "0.38"

# Channels (in respective plugin crates)
teloxide = "0.13"        # Telegram
serenity = "0.12"        # Discord
slack-morphism = "2"     # Slack

# Observability
tracing = "0.1"
tracing-subscriber = "0.3"
```

---

## 4. Data Flow

```
User sends message
        |
        v
+-------------------+
| Channel Plugin    |  (.wasm - telegram, discord, etc.)
| receives message  |
+--------+----------+
         | IncomingMessage
+--------v----------+
|      Router       |
+--------+----------+
         |
+--------v------------------------------+
|            Pipeline                    |
|                                        |
|  1. Auth check      (security crate)  |
|  2. Rate limit      (security crate)  |
|  3. Sanitize input  (security crate)  |
|  4. Build context   (memory crate)    |
|  5. LLM call        (llm crate)      |
|  6. Tool dispatch -----> WASM Runtime |
|     |                    +-- skill.wasm|
|     | skill result       +-- MCP srv  |
|     <-----------------------------+   |
|  7. Save exchange   (memory crate)    |
|  8. Background learning               |
|                                        |
+--------+------------------------------+
         | OutgoingMessage
+--------v----------+
|      Router       |
+--------+----------+
         |
  Reply to user
```

---

## 5. Plugin Lifecycle

### Startup

1. `amanclaw-cli` loads `config.yaml`
2. Core engine initializes (memory, security, LLM client)
3. WASM runtime scans `plugins/` directory for `.wasm` files
4. Each `.wasm` is loaded, `metadata()` called, registered in registry
5. MCP servers from config are connected
6. Channel plugins start (begin receiving messages)

### Message Processing

1. Channel receives message, pushes `IncomingMessage` to router
2. Pipeline processes (auth, sanitize, LLM)
3. LLM picks a skill -> WASM runtime executes `skill.wasm`
   - Host functions (`http_fetch`, `log`, `get_config`) available
   - Timeout enforced via wasmtime fuel/epoch interruption
   - If skill panics -> caught, error returned, engine continues
4. Result flows back through pipeline -> channel sends reply

### Hot Reload

- Watch `plugins/` directory for new/changed `.wasm` files
- Reload without restarting the engine
- Zero downtime plugin updates

---

## 6. Plugin Author Experience

### Rust Plugin

```rust
use amanclaw_plugin_sdk::*;

#[skill(name = "weather", description = "Get weather for a city", timeout_ms = 10000)]
fn weather(city: String) -> SkillResult {
    let resp = host::http_fetch("https://wttr.in/{city}?format=3")?;
    Ok(resp.body)
}
```

Build: `cargo build --target wasm32-wasip2 --release`

### Python Plugin

```python
import amanclaw_sdk

@amanclaw_sdk.skill(name="weather", description="Get weather for a city")
def weather(city: str) -> str:
    resp = amanclaw_sdk.http_fetch(f"https://wttr.in/{city}?format=3")
    return resp.body
```

Build: `componentize-py -w skill my_skill.py -o weather.wasm`

### JavaScript Plugin

```javascript
import { httpFetch } from 'amanclaw:skill/host';

export function metadata() {
    return { name: "weather", description: "Get weather", timeoutMs: 10000, version: "0.1.0" };
}

export function execute(input) {
    const args = JSON.parse(input.args);
    const resp = httpFetch({
        url: `https://wttr.in/${args.city}?format=3`,
        method: "GET", headers: [], body: null
    });
    return { success: true, output: resp.body, error: null };
}
```

Build: `npx jco componentize my-skill.js -w skill -o weather.wasm`

### Install a Plugin

```bash
# Drop a .wasm file into plugins/
cp weather.wasm ~/.amanclaw/plugins/

# Or use CLI
amanclaw plugin install weather.wasm
amanclaw plugin list
amanclaw plugin remove weather
```

---

## 7. Migration Path

| Phase | What | Crates | Est. Effort |
|-------|------|--------|-------------|
| **1** | Foundation: traits, CLI skeleton, core pipeline | `amanclaw-traits`, `amanclaw-cli`, `amanclaw-core` | 1-2 weeks |
| **2** | Core services: LLM client, memory, security | `amanclaw-llm`, `amanclaw-memory`, `amanclaw-security` | 2-3 weeks |
| **3** | WASM runtime + plugin SDK | `amanclaw-wasm-runtime`, `amanclaw-plugin-sdk` | 2-3 weeks |
| **4** | Port first-party skills to WASM | `plugins/skill-*` | 2-3 weeks |
| **5** | Port channel adapters | `plugins/channel-*` | 1-2 weeks |
| **6** | Language SDKs for plugin authors | `sdks/python`, `sdks/javascript`, `sdks/go` | 1-2 weeks |
| **7** | Docs, hot reload, observability, polish | docs, deploy, CI | 1-2 weeks |

**Total: ~10-16 weeks**

---

## 8. Testing Strategy

| Layer | Approach |
|-------|----------|
| `amanclaw-traits` | Unit tests for types and serialization |
| `amanclaw-llm` | Mock HTTP server, test tool parsing |
| `amanclaw-memory` | In-memory SQLite, test all queries |
| `amanclaw-security` | Unit tests for auth, rate limiter, sanitizer |
| `amanclaw-core` | Integration tests with mock LLM + test plugins |
| `amanclaw-wasm-runtime` | Load test `.wasm` plugins, verify sandboxing |
| Plugins | Each plugin has its own unit tests |
| E2E | Docker compose with mock LLM, send messages, verify replies |

---

## 9. Config Format (unchanged)

```yaml
llm:
  base_url: "http://localhost:8001/v1"
  model: "Qwen/Qwen3-VL-30B-A3B-Instruct"
  max_tokens: 4096
  temperature: 0.7

admin_users:
  telegram: [123456789]
  whatsapp: ["60123456789"]

rate_limit_per_minute: 20

plugins:
  dir: "./plugins"
  hot_reload: true

skills:
  shell_allowed_commands: [ls, cat, grep, find, df]
  workspace_dir: "~/amanclaw-workspace"
  skill_timeout_seconds: 30

security:
  injection_rules: "default"
  sanitize_output: true

mcp_servers:
  filesystem:
    command: "npx"
    args: ["-y", "@modelcontextprotocol/server-filesystem", "/home/user/docs"]
```
