# AmanClaw

A high-performance, modular AI assistant built with Rust. Connect it to Telegram (more channels coming) — powered by any OpenAI-compatible LLM backend. Extend it with WASM plugins written in Rust, Python, or JavaScript.

Built in Malaysia. Open source. No bloat.

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.85+-orange.svg)](https://www.rust-lang.org/)
[![Docker](https://img.shields.io/badge/docker-ready-blue.svg)](rust/docker-compose.yml)

---

## What Is This?

AmanClaw is a personal AI assistant that lives in your chat apps. You message it, it thinks using an LLM, optionally calls skills (tools), and replies back.

```
You (Telegram)
  │
  ▼
AmanClaw Engine ──► Auth ──► Rate Limit ──► Sanitize ──► LLM ──► Skills (WASM)
  │                                                                    │
  ▼                                                                    ▼
Reply  ◄──────────────────────────────────────────────────────────────-┘
```

One binary. One SQLite database. One config file. ~2MB RAM on a Raspberry Pi.

---

## Features

- **Blazing fast** — Rust async runtime, ~2MB memory footprint, instant startup
- **Plugin system** — WASM Component Model plugins in Rust, Python, or JavaScript
- **Multi-channel** — Telegram, Discord, WhatsApp (Slack planned)
- **Any LLM backend** — vLLM, Ollama, LM Studio, LocalAI, OpenAI, Anthropic, etc.
- **Security-first** — user allowlist, rate limiting, prompt injection detection, output sanitization
- **Conversation memory** — SQLite-backed history, facts, and summaries
- **Production-ready** — Docker with hardened containers, systemd service, structured logging
- **Cross-platform** — runs on x86_64, ARM64 (Raspberry Pi), and anywhere Rust compiles

---

## Quick Start

### Prerequisites

- [Rust 1.85+](https://rustup.rs/) (for building from source)
- An OpenAI-compatible LLM API (local or remote)
- A Telegram bot token (from [@BotFather](https://t.me/BotFather))

### 1. Clone and build

```bash
git clone https://github.com/amanasmuei/amanclaw.git
cd amanclaw/rust
cargo build --release
```

The binary is at `target/release/amanclaw`.

### 2. Configure secrets

Create a `.env` file in the project root:

```bash
TELEGRAM_BOT_TOKEN=your-telegram-bot-token
LLM_API_KEY=your-llm-api-key
```

### 3. Configure settings

```bash
cp config.example.yaml config.yaml
```

Edit `config.yaml`:

```yaml
llm:
  base_url: "http://localhost:8001/v1"
  model: "your-model-name"

admin_users:
  telegram: ["YOUR_TELEGRAM_USER_ID"]
```

### 4. Run

```bash
./target/release/amanclaw
```

---

## Architecture

AmanClaw is a Cargo workspace with modular crates:

```
rust/
├── Cargo.toml                 # Workspace root
├── crates/
│   ├── amanclaw-traits/       # Core types, traits, config
│   ├── amanclaw-cli/          # Binary entry point
│   ├── amanclaw-core/         # Engine, pipeline, router, registry
│   ├── amanclaw-security/     # Auth, rate limiter, sanitizer
│   ├── amanclaw-memory/       # SQLite conversation & fact storage
│   ├── amanclaw-llm/          # OpenAI-compatible LLM client
│   ├── amanclaw-wasm-runtime/ # WASM plugin loader & sandbox
│   └── amanclaw-plugin-sdk/   # SDK types for plugin authors
├── plugins/
│   ├── skill-sysinfo/         # System info skill
│   ├── skill-websearch/       # DuckDuckGo search skill
│   ├── skill-shell/           # Whitelisted shell commands
│   └── channel-telegram/      # Telegram adapter
├── wit/
│   └── skill.wit              # WASM Interface Types contract
├── Dockerfile
├── docker-compose.yml
└── docs/
    └── plugin-guide.md        # Plugin authoring guide
```

### How It Works

1. **Channel adapters** receive messages from platforms and push them into the engine via an async channel
2. **Engine** pulls messages and runs them through the **pipeline**
3. **Pipeline** checks auth → rate limit → sanitize input → build context from memory → call LLM → save exchange
4. **LLM** may request tool calls, which are executed via the **plugin registry**
5. **Response** is routed back to the correct channel adapter

---

## Writing Plugins

AmanClaw uses the [WASM Component Model](https://component-model.bytecodealliance.org/) for plugins. Write skills in **Rust**, **Python**, or **JavaScript** — they all compile to `.wasm` and run sandboxed.

### Rust Example

```rust
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
    let args: serde_json::Value = serde_json::from_str(&input.args).unwrap_or_default();
    let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("no query");
    SkillResult::ok(format!("Result for: {}", query))
}
```

### Plugin Sandbox

All plugins run with strict limits:
- No filesystem access
- No direct network — use `http-fetch` host function
- 64MB memory limit per plugin
- Configurable execution timeout
- Domain allowlist for HTTP requests

See [`rust/docs/plugin-guide.md`](rust/docs/plugin-guide.md) for the full guide including Python and JavaScript examples.

---

## Configuration

### Secrets (`.env`)

Never commit this file. Set `chmod 600 .env`.

| Variable | Required | Purpose |
|----------|----------|---------|
| `TELEGRAM_BOT_TOKEN` | Yes | Telegram bot token |
| `LLM_API_KEY` | Yes | LLM API key |

### Settings (`config.yaml`)

See [`config.example.yaml`](config.example.yaml) for all options. Key sections:

```yaml
llm:
  base_url: "http://localhost:8001/v1"
  model: "Qwen/Qwen3-VL-30B-A3B-Instruct"
  max_tokens: 4096
  temperature: 0.7

admin_users:
  telegram: ["123456789"]

rate_limit_per_minute: 20

plugins:
  dir: "./plugins"
  hot_reload: false

security:
  injection_rules: "default"
  sanitize_output: true
```

### Environment Overrides

| Variable | Purpose |
|----------|---------|
| `MEMORY_DB_PATH` | Override SQLite database path |
| `LOG_FORMAT` | `text` or `json` |
| `RUST_LOG` | Log level filter (e.g. `amanclaw=debug`) |

---

## Deployment

### Docker (recommended)

```bash
cd rust
docker compose up -d

# View logs
docker compose logs -f

# Stop
docker compose down
```

The container runs as non-root with hardened security:
- All capabilities dropped (`cap_drop: ALL`)
- Read-only filesystem
- No new privileges
- 512MB memory / 1.0 CPU limit

### Systemd (bare metal / Raspberry Pi)

```bash
# Copy binary and config
mkdir -p ~/amanclaw/plugins ~/amanclaw/data
cp target/release/amanclaw ~/amanclaw/
cp config.yaml .env ~/amanclaw/

# Create service
sudo tee /etc/systemd/system/amanclaw.service << 'EOF'
[Unit]
Description=AmanClaw Bot
After=network.target

[Service]
Type=simple
User=your-user
WorkingDirectory=/home/your-user/amanclaw
ExecStart=/home/your-user/amanclaw/amanclaw
EnvironmentFile=/home/your-user/amanclaw/.env
Restart=on-failure
RestartSec=5

[Install]
WantedBy=multi-user.target
EOF

sudo systemctl daemon-reload
sudo systemctl enable --now amanclaw
```

### Cross-Compilation (for Raspberry Pi)

Build on your dev machine, deploy to Pi:

```bash
# Install cross-compilation target
rustup target add aarch64-unknown-linux-gnu

# Build (requires aarch64 linker or use Docker/cross)
cargo build --release --target aarch64-unknown-linux-gnu -p amanclaw-cli

# Copy to Pi
scp target/aarch64-unknown-linux-gnu/release/amanclaw user@pi:~/amanclaw/
```

---

## Security

### Authentication & Authorization
- User allowlist with admin approval flow
- New users are registered but must be approved by an admin
- Per-user sliding window rate limiting

### Input Protection
- Regex-based prompt injection detection
- Input sanitization before LLM processing
- Skill output sandboxing — marked as external data to the LLM

### Plugin Sandboxing
- WASM plugins run in isolated memory spaces
- No filesystem or direct network access
- Configurable timeouts and memory limits
- Domain allowlist for HTTP requests

### Infrastructure
- Docker: non-root, `cap_drop: ALL`, read-only filesystem, resource limits
- Secrets in `.env` (never in config or code)

---

## Development

### Build and test

```bash
cd rust
cargo build
cargo test --workspace
```

### Run with debug logging

```bash
RUST_LOG=amanclaw=debug cargo run -p amanclaw-cli
```

### Project test coverage

```
53 tests across 12 crates
├── amanclaw-traits       11 tests (config, messages, skills, channels)
├── amanclaw-core          7 tests (pipeline, router, registry, integration)
├── amanclaw-security     11 tests (auth, rate limiter, sanitizer)
├── amanclaw-memory        4 tests (history, facts, upsert, counting)
├── amanclaw-llm           2 tests (LLM client, thinking tag stripping)
├── amanclaw-wasm-runtime  7 tests (loader, host state, sandbox config)
├── skill-sysinfo          2 tests
├── skill-websearch        2 tests
├── skill-shell            4 tests
└── channel-telegram       1 test
```

---

## Contributing

Contributions are welcome! Here's how to get started:

### Getting Set Up

```bash
# Fork and clone
git clone https://github.com/YOUR_USERNAME/amanclaw.git
cd amanclaw/rust

# Build and verify tests pass
cargo build
cargo test --workspace

# Create a feature branch
git checkout -b feature/my-feature
```

### Making Changes

1. **Write tests first** — we use TDD. Add failing tests, then implement
2. **Run the full test suite** — `cargo test --workspace`
3. **Keep commits focused** — one logical change per commit
4. **Use conventional commits** — `feat:`, `fix:`, `docs:`, `chore:`

### Pull Request Process

1. Fork the repo and create your branch from `master`
2. Add tests for any new functionality
3. Ensure `cargo test --workspace` passes with zero failures
4. Ensure `cargo clippy` has no warnings
5. Update documentation if you changed public APIs
6. Open a PR with a clear description of what and why

### Areas Where Help Is Appreciated

| Area | Description | Difficulty |
|------|-------------|------------|
| **New skill plugins** | Web scraping, calendar, weather, translation, etc. | Easy |
| **Channel adapters** | Discord, Slack, WhatsApp, Matrix, Signal | Medium |
| **Python/JS plugin SDK** | componentize-py and jco integration | Medium |
| **WASM runtime integration** | Wire PluginLoader to actually instantiate .wasm files | Medium |
| **Tool calling** | LLM tool call parsing and skill execution loop | Medium |
| **Vision support** | Image handling in messages | Medium |
| **Conversation summarization** | Auto-compress long conversations | Medium |
| **Hot reload** | Watch plugin directory and reload on changes | Medium |
| **Admin commands** | `/approve`, `/block`, `/clear`, `/learned` | Easy |
| **Documentation** | Tutorials, examples, architecture docs | Easy |
| **Security review** | Audit injection detection, auth flow, sandbox | Hard |
| **i18n / localization** | Malay, Mandarin, and other languages | Easy |

### Writing a Plugin

The easiest way to contribute is by writing a new skill plugin. See the [Plugin Author Guide](rust/docs/plugin-guide.md) for step-by-step instructions in Rust, Python, and JavaScript.

---

## Roadmap

- [x] Core engine with async pipeline
- [x] Telegram channel adapter
- [x] LLM client (OpenAI-compatible)
- [x] SQLite conversation memory
- [x] Security (auth, rate limiting, injection detection)
- [x] WASM plugin runtime (loader, sandbox, SDK)
- [x] Built-in skills (sysinfo, websearch, shell)
- [x] Docker & systemd deployment
- [x] LLM tool calling loop (skill execution)
- [x] Admin commands (`/approve`, `/block`, `/stats`, `/users`)
- [x] Conversation auto-summarization
- [x] Learning engine (`/remember`, `/forget`, `/learned`)
- [x] Vision support (image analysis via multimodal LLM)
- [x] Plugin hot reload (filesystem watcher)
- [x] Full WASM plugin instantiation and execution
- [x] Discord channel adapter
- [x] WhatsApp Cloud API channel adapter
- [ ] Slack channel
- [ ] Python and JavaScript plugin SDKs
- [ ] MCP server integration

---

## FAQ

**Q: What LLM should I use?**
Any OpenAI-compatible API. Local: Ollama, vLLM, LM Studio. Cloud: OpenAI, Together AI, Groq, etc.

**Q: Can I use this with multiple people?**
Yes. Add user IDs to `admin_users` in config. Non-admin users go through an approval flow.

**Q: How much resources does it need?**
~2MB RAM, <1% CPU idle on a Raspberry Pi 4. It's Rust — it's fast.

**Q: Can I write plugins in Python?**
Yes! Plugins use the WASM Component Model. Write in Python, compile with [componentize-py](https://github.com/bytecodealliance/componentize-py), and drop the `.wasm` file in the plugins directory.

**Q: Is my data stored?**
Conversations are stored in a local SQLite database. Nothing leaves your server except LLM API calls.

---

## License

MIT License. See [LICENSE](LICENSE) for details.

---

## Acknowledgments

Built with care from Puncak Alam, Malaysia. Made possible by the Rust ecosystem and the communities behind teloxide, wasmtime, tokio, and the WASM Component Model.

*Malaysia boleh!* 🇲🇾
