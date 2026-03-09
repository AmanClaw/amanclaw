<p align="center"><img src="desktop/src-tauri/icons/app-icon.png" width="150" alt="AmanClaw mascot" /><br><strong style="font-size:2em">AmanClaw</strong></p>

A high-performance, modular AI assistant built with Rust for Malaysian Muslim communities. Connect it to Telegram, Discord, Slack, or WhatsApp — powered by any OpenAI-compatible LLM backend. Comes with 11 Islamic skills (solat, Quran, halal, zakat, and more) out of the box.

Built in Malaysia. Open source. No bloat. Bilingual BM + English.

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.85+-orange.svg)](https://www.rust-lang.org/)
[![Docker](https://img.shields.io/badge/docker-ready-blue.svg)](rust/docker-compose.yml)

---

## What Is This?

AmanClaw is a personal AI assistant that lives in your chat apps. You message it, it thinks using an LLM, optionally calls skills (tools), and replies back.

```text
You (Telegram / Discord / WhatsApp / Slack)
  │
  ├── Chat message ──► Engine ──► Auth ──► Rate Limit ──► Sanitize ──► LLM ◄──► Skills
  │                                                                               │
  ├── Cron job ─────► Scheduler ──► Pipeline (bypass auth) ──► LLM ◄──► Skills   │
  │                                                                               │
  ├── Webhook ──────► Router ──► Auth (HMAC/Bearer) ──► Transform ──► Pipeline   │
  │                                                                               │
  ├── WebSocket ────► Gateway (JSON-RPC 2.0) ──► Session Manager                 │
  │                                                                               │
  ▼                                                                               ▼
Reply  ◄──────────────────────────────────────────────────────────────────────────┘
```

One binary. One SQLite database. One config file. ~2MB RAM on a Raspberry Pi.

---

## Features

- **Blazing fast** — Rust async runtime, ~2MB memory footprint, instant startup
- **Islamic community skills** — Prayer times (JAKIM + 6 global methods), Quran, Halal check, Zakat calculator, Qiblat, Hijri calendar, Doa & Zikir, Hadith, Masjid finder, Khutbah, JAKIM services
- **Global prayer times** — Pure-Rust calculation engine supporting MWL, ISNA, Egyptian, Karachi, Umm al-Qura, and JAKIM methods — works offline, no API needed
- **Multi-community** — One instance serves many groups with per-community config (zone, language, skills)
- **Plugin system** — WASM plugins (Rust/AssemblyScript) + script plugins (Python/JS) + built-in Rust skills
- **Multi-channel** — Telegram, Discord, WhatsApp (official + unofficial), Slack
- **Any LLM backend** — vLLM, Ollama, LM Studio, LocalAI, OpenAI, Anthropic, etc.
- **Bilingual** — Bahasa Melayu + English with natural rojak-style responses
- **Tool calling** — LLM function calling with multi-round tool execution loop (max 5 rounds)
- **Vision support** — Send images to multimodal LLMs via base64 encoding
- **Security-first** — User allowlist, rate limiting, prompt injection detection, output sanitization
- **Conversation memory** — SQLite-backed history with auto-summarization and pruning
- **RAG (Retrieval-Augmented Generation)** — Load knowledge bases at startup, index with embeddings, retrieve relevant context during conversations
- **Learning engine** — Remembers facts about users across conversations (`/remember`, `/forget`, `/learned`)
- **Skill ecosystem** — Search, install, and publish skills via GitHub-powered index with curated packs (islamic, malaysian, islamic-core)
- **Skill quality tiers** — Community, Verified, and Official tiers with automated validation (`amanclaw skill publish`)
- **Skill scaffolding** — `amanclaw skill new my-skill --lang rust|python` generates complete skill projects with CI, README, LICENSE, and tests
- **Interactive playground** — `amanclaw playground` serves a local web UI for testing skills interactively
- **Live reload** — `amanclaw dev --watch` watches plugins, souls, and config for changes
- **WhatsApp interactive messages** — Buttons and list messages for rich WhatsApp UX (beyond plain text)
- **Plugin hot reload** — Filesystem watcher detects new/modified `.wasm` plugins
- **MCP integration** — Expose skills as MCP tools + consume external MCP servers as skills
- **Hybrid search** — FTS5 full-text search with BM25 ranking + cosine vector similarity via Reciprocal Rank Fusion (RRF)
- **SOUL.md agent personas** — YAML-frontmatter agent personality files with inheritance chains and variable interpolation
- **Cron scheduler** — Scheduled jobs (direct messages, skill invocations, agent prompts) with timezone support and pipeline bypass
- **Webhook triggers** — Inbound webhook routes with HMAC-SHA256/Bearer/header auth, Handlebars template transforms, and rate limiting
- **WebSocket gateway** — JSON-RPC 2.0 real-time gateway with session management, topic subscriptions, and glob-based event routing
- **Sub-agent spawning** — Parallel task execution via spawned sub-agents with per-session/global limits and max-depth control
- **Skill marketplace** — `amanclaw-skill.toml` manifest format, local registry with SQLite-backed install/search, remote index with SHA256 verification
- **Event system** — `EventEmitter` trait for broadcasting pipeline events (message.received, message.sent, security.*)
- **Desktop admin app** — Cross-platform Tauri 2 desktop app (macOS, Windows, Linux) with system tray and native notifications
- **REST management API** — Axum-based REST API for bot status, communities, skills, users, webhooks management
- **Production-ready** — Docker with hardened containers, systemd service, structured logging
- **Cross-platform** — Runs on x86_64, ARM64 (Raspberry Pi), and anywhere Rust compiles

---

## Quick Start

### Prerequisites

- [Rust 1.85+](https://rustup.rs/) (for building from source)
- An OpenAI-compatible LLM API (local or remote)
- A Telegram bot token (from [@BotFather](https://t.me/BotFather))

### 1. Clone and build

```bash
git clone https://github.com/AmanClaw/amanclaw.git
cd amanclaw/rust
cargo build --release
```

The binary is at `target/release/amanclaw`.

### Quick Setup (alternative)

```bash
# Initialize project with defaults
amanclaw init

# Start in development mode (no API key needed)
amanclaw dev

# Start with live reload
amanclaw dev --watch

# Open interactive playground
amanclaw playground

# Create a new skill (generates CI, README, LICENSE, tests)
amanclaw skill new my-skill --lang rust
amanclaw skill new my-skill --lang python

# Search the skill index
amanclaw skill search "prayer"

# Install a skill from the index
amanclaw skill install skill-solat

# Install a curated skill pack
amanclaw skill install-pack islamic

# List available packs
amanclaw skill packs

# Validate a skill for publishing
amanclaw skill publish ./my-skill
```

### 2. Configure secrets

Create a `.env` file in the project root:

```bash
# Required
TELEGRAM_BOT_TOKEN=your-telegram-bot-token
LLM_API_KEY=your-llm-api-key

# Optional channels
DISCORD_BOT_TOKEN=your-discord-bot-token
WHATSAPP_ACCESS_TOKEN=your-whatsapp-access-token
WHATSAPP_PHONE_NUMBER_ID=your-phone-number-id
WAHA_API_URL=http://localhost:3000   # For unofficial WhatsApp via WAHA
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

## Global Prayer Time Methods

AmanClaw includes a pure-Rust prayer time calculation engine — works offline, no API dependency:

| Method | Region | Fajr | Isha |
| ------ | ------ | ---- | ---- |
| MWL | Muslim World League | 18° | 17° |
| ISNA | North America | 15° | 15° |
| Egyptian | Egypt | 19.5° | 17.5° |
| Karachi | Pakistan/India | 18° | 18° |
| Umm al-Qura | Saudi Arabia | 18.5° | 90 min after Maghrib |
| JAKIM | Malaysia | 20° | 18° |

```bash
# Via skill
{"action": "calculate", "latitude": 40.7128, "longitude": -74.006, "timezone": -4, "method": "isna"}

# List all methods
{"action": "list_methods"}
```

---

## Skill Ecosystem

AmanClaw has a GitHub-powered skill marketplace. No custom infrastructure — skills are GitHub repos with an `amanclaw-skill.toml` manifest.

### Quality Tiers

| Tier          | Badge         | Requirements                                          |
|---------------|---------------|-------------------------------------------------------|
| **Official**  | `[official]`  | Maintained by AmanClaw team, guaranteed compatibility |
| **Verified**  | `[verified]`  | Tests pass, permissions declared, docs present        |
| **Community** | `[community]` | Anyone can publish, no review required                |

### Curated Packs

```bash
amanclaw skill install-pack islamic       # 11 skills: solat, quran, hadith, halal, zakat, etc.
amanclaw skill install-pack islamic-core  # 5 core Rust skills: solat, qiblat, hijri, doa, quran
amanclaw skill install-pack malaysian     # 4 Malaysia-specific: solat, halal, jakim, masjid
```

### Publishing a Skill

```bash
# 1. Create from template (generates CI, README, LICENSE, tests)
amanclaw skill new my-skill --lang rust

# 2. Develop and test
cd skill-my-skill && cargo test

# 3. Validate for publishing
amanclaw skill publish .

# 4. Push to GitHub, create a release, submit PR to skill-index
```

### SOUL.md Personas

Define AI personalities in `souls/` directory:

```bash
souls/
├── ustaz.md          # Islamic knowledge assistant
├── masjid-admin.md   # Mosque management bot
└── community.md      # General community assistant
```

Each persona defines personality, capabilities, and guidelines. The LLM uses these to shape its responses.

---

## Architecture

AmanClaw is a Cargo workspace with 29 crates (16 core + 13 plugins) plus a Tauri desktop app:

```text
rust/
├── Cargo.toml                    # Workspace root
├── crates/
│   ├── amanclaw-traits/          # Core types, traits, config, EventEmitter
│   ├── amanclaw-cli/             # Binary entry point
│   ├── amanclaw-core/            # Engine, pipeline, router, scheduler, webhooks, soul loader, sub-agents
│   ├── amanclaw-security/        # Auth, rate limiter, sanitizer
│   ├── amanclaw-memory/          # SQLite conversation, facts, summaries, vector store, FTS5 hybrid search
│   ├── amanclaw-llm/             # OpenAI-compatible LLM client + tool calling + embeddings
│   ├── amanclaw-wasm-runtime/    # WASM plugin loader, sandbox, runtime, watcher
│   ├── amanclaw-plugin-sdk/      # SDK + macro for WASM plugin authors
│   ├── amanclaw-mcp/            # MCP server + client bridge (stdio + HTTP)
│   ├── amanclaw-script-runtime/ # Script plugin loader (Python/JS via subprocess)
│   ├── amanclaw-api/            # REST management API + WebSocket gateway (Axum)
│   ├── amanclaw-gateway/        # WebSocket gateway (JSON-RPC 2.0, session management)
│   └── amanclaw-registry/       # Skill marketplace (manifest, local/remote registry)
├── plugins/
│   ├── skill-sysinfo/            # System info skill (built-in)
│   ├── skill-shell/              # Whitelisted shell commands (built-in)
│   ├── skill-solat/              # Prayer times via JAKIM (built-in)
│   ├── skill-qiblat/             # Qiblat direction (built-in)
│   ├── skill-hijri/              # Islamic calendar (built-in)
│   ├── skill-doa/                # Doa & zikir collection (built-in)
│   ├── skill-quran/              # Quran lookup & search (built-in)
│   ├── skill-echo-wasm/          # Example WASM plugin (153KB compiled)
│   ├── channel-telegram/         # Telegram adapter (teloxide)
│   ├── channel-discord/          # Discord adapter (serenity)
│   ├── channel-whatsapp/         # WhatsApp Cloud API adapter
│   ├── channel-whatsapp-web/     # Unofficial WhatsApp via WAHA bridge
│   └── channel-slack/           # Slack adapter (Socket Mode)
├── sdks/
│   ├── assemblyscript/           # AssemblyScript (JS/TS) plugin SDK
│   └── python/                   # Python plugin SDK
├── souls/
│   └── default.md                # Default agent personality (SOUL.md)
├── wit/
│   └── skill.wit                 # WASM Interface Types contract
├── Dockerfile
└── docker-compose.yml
desktop/                           # Tauri 2 desktop admin app
├── src/                           # Svelte 5 + Tailwind CSS 4 frontend
│   ├── lib/
│   │   ├── components/            # Sidebar, UI components
│   │   ├── pages/                 # Dashboard, Communities, Skills, etc.
│   │   ├── stores/                # Svelte stores (state management)
│   │   └── api.ts                 # API client (Tauri IPC / REST)
│   └── routes/                    # SvelteKit routes
├── src-tauri/                     # Rust backend (Tauri 2)
│   └── src/
│       ├── commands.rs            # IPC commands (Svelte ↔ Rust)
│       ├── tray.rs                # System tray setup
│       ├── notifications.rs       # Native notification manager
│       ├── logs.rs                # Log broadcasting
│       └── state.rs               # App state (local/remote mode)
├── package.json
└── svelte.config.js
```

### How It Works

1. **Channel adapters** receive messages from platforms and push them into the engine via async channels
2. **Engine** multiplexes chat messages and scheduler events via `tokio::select!`
3. **Agent router** resolves which agent profile handles the message (per-platform, per-topic, or default)
4. **SOUL.md loader** resolves agent personality files with frontmatter, inheritance, and variable interpolation
5. **Pipeline** checks auth → rate limit → sanitize input → build context (summary + facts + history + FTS5/vector hybrid search) → call LLM
6. Internal messages (cron, webhook, sub-agent) bypass auth, rate limiting, and sanitization
7. **LLM** may request tool calls, which are executed via the **plugin registry** (up to 5 rounds)
8. **EventEmitter** broadcasts pipeline events (`message.received`, `message.sent`, `security.*`) to WebSocket subscribers
9. **Auto-summarization** kicks in when history exceeds 40 messages — LLM summarizes, old messages are pruned
10. **Response** is routed back to the correct channel adapter by platform

### Bot Commands

| Command | Description | Access |
| ------- | ----------- | ------ |
| `/start`, `/myid` | Show your user ID and platform | Everyone |
| `/clear` | Clear conversation history | Approved |
| `/stats` | Show message count | Approved |
| `/learned` | Show stored facts about you | Approved |
| `/remember <key> <value>` | Save a fact (e.g. `/remember name Aman`) | Approved |
| `/forget <key>` | Delete a stored fact | Approved |
| `/approve <user_id>` | Approve a pending user | Admin |
| `/block <user_id>` | Block a user | Admin |
| `/users` | List all registered users | Admin |

### Islamic Commands

| Command | Description |
| ------- | ----------- |
| `/solat` | Today's prayer times for your community zone |
| `/solat <zone>` | Prayer times for a specific JAKIM zone (e.g. SGR01) |
| `/quran <surah>:<ayat>` | Look up a Quran verse with translation |
| `/cari <keyword>` | Search Quran and Hadith |
| `/halal <name>` | Check halal status via JAKIM database |
| `/zakat` | Interactive zakat calculator (fitrah, pendapatan, simpanan, emas) |
| `/qiblat` | Qiblat direction from your location |
| `/doa <category>` | Doa and zikir lookup (harian, pagi, petang, musafir, etc.) |
| `/masjid` | Find nearest masjid/surau |
| `/hijri` | Today's Hijri date and upcoming Islamic events |
| `/khutbah` | Latest weekly JAKIM khutbah |

Natural language also works: "Bila waktu Maghrib?", "Is KFC halal?", "Doa sebelum makan"

### Community Admin Commands

| Command | Description |
| ------- | ----------- |
| `/admin` | Claim admin for this group (during onboarding) |
| `/setzone <zone>` | Change prayer time zone (e.g. `/setzone SGR01`) |
| `/setlang <bm\|en\|rojak>` | Set community language |
| `/enable <skill>` | Enable a skill for this community |
| `/disable <skill>` | Disable a skill for this community |
| `/notify <on\|off>` | Toggle push notifications (solat reminders, daily doa, etc.) |
| `/community` | Show current community settings |

---

## Channel Adapters

### Telegram

Set `TELEGRAM_BOT_TOKEN` and the bot starts automatically.

### Discord

Set `DISCORD_BOT_TOKEN`. Requires `MESSAGE_CONTENT` intent enabled in the Discord Developer Portal.

### WhatsApp (Official — Cloud API)

Requires a Meta Business account. Set:

```bash
WHATSAPP_ACCESS_TOKEN=your-access-token
WHATSAPP_PHONE_NUMBER_ID=your-phone-number-id
WHATSAPP_VERIFY_TOKEN=your-verify-token     # default: amanclaw_verify
WHATSAPP_WEBHOOK_PORT=8080                   # default: 8080
```

Configure Meta's webhook to point to `http://your-server:8080/webhook`.

### WhatsApp (Unofficial — WAHA)

Uses [WAHA](https://waha.devlike.pro) (WhatsApp HTTP API), a self-hosted WhatsApp Web bridge. No Business account needed.

```bash
# Start WAHA
docker run -p 3000:3000 devlikeapro/waha

# Set env vars
WAHA_API_URL=http://localhost:3000
WAHA_API_KEY=your-api-key              # optional
WAHA_SESSION=default                    # default: default
WAHA_WEBHOOK_PORT=8081                  # default: 8081
```

Configure WAHA's webhook to point to `http://your-server:8081/webhook`.

### Slack (Socket Mode)

Uses [Slack Socket Mode](https://api.slack.com/apis/socket-mode) (WebSocket) — no public URL needed.

**Setup:**

1. Create a Slack app at [api.slack.com/apps](https://api.slack.com/apps)
2. Enable **Socket Mode** in your app settings
3. Add **Bot Token Scopes**: `chat:write`, `channels:history`, `groups:history`, `im:history`, `mpim:history`
4. Subscribe to **Events**: `message.channels`, `message.groups`, `message.im`, `message.mpim`
5. Generate an **App-Level Token** with `connections:write` scope
6. Install the app to your workspace

```bash
SLACK_BOT_TOKEN=xoxb-your-bot-token
SLACK_APP_TOKEN=xapp-your-app-level-token
```

The bot auto-detects its own user ID to avoid replying to itself. Messages in channels, DMs, and threads are all supported.

---

## Writing WASM Plugins

AmanClaw loads `.wasm` files from the plugins directory on startup. Plugins use a simple JSON-based ABI.

### Using the SDK Macro

```rust
// Cargo.toml: [lib] crate-type = ["cdylib"]
use amanclaw_plugin_sdk::*;

amanclaw_plugin!(
    metadata: SkillMetadata {
        name: "my_skill".into(),
        description: "Does something useful".into(),
        timeout_ms: 10000,
        version: "0.1.0".into(),
    },
    parameters: r#"{"type":"object","properties":{"query":{"type":"string","description":"Search query"}},"required":["query"]}"#,
    execute: |input: SkillInput| -> SkillResult {
        let args: serde_json::Value = serde_json::from_str(&input.args).unwrap_or_default();
        let query = args["query"].as_str().unwrap_or("none");
        SkillResult::ok(format!("Result for: {}", query))
    }
);
```

### Build and Deploy

```bash
# Build for WASM
cargo build --target wasm32-unknown-unknown --release -p my-skill

# Copy to plugins directory
cp target/wasm32-unknown-unknown/release/my_skill.wasm ./plugins/

# Restart the engine (or wait for hot reload)
```

### WASM ABI Contract

Plugins must export these functions:

| Export | Signature | Description |
| ------ | --------- | ----------- |
| `alloc` | `(size: i32) -> ptr: i32` | Allocate memory for input |
| `dealloc` | `(ptr: i32, size: i32)` | Free allocated memory |
| `metadata` | `() -> ptr: i32` | Return null-terminated JSON `SkillMetadata` |
| `parameters` | `() -> ptr: i32` | Return null-terminated JSON schema string |
| `execute` | `(ptr: i32, len: i32) -> ptr: i32` | Take JSON `SkillInput`, return JSON `SkillResult` |

### Plugin Sandbox

All WASM plugins run with strict limits:

- No filesystem access
- No direct network access
- 64MB memory limit per plugin
- 30-second execution timeout (configurable)
- Epoch-based interruption for runaway plugins

### Writing Plugins in Python

Python plugins use a JSON protocol over stdin/stdout. Install the SDK and write your plugin:

```python
# my_plugin.py
from amanclaw_sdk import plugin, SkillInput, SkillResult

@plugin(
    name="weather",
    description="Get weather for a city",
    parameters={
        "type": "object",
        "properties": {"city": {"type": "string"}},
        "required": ["city"]
    }
)
def execute(input: SkillInput) -> SkillResult:
    args = input.parse_args()
    city = args.get("city", "unknown")
    return SkillResult.ok(f"Weather in {city}: Sunny, 32°C")

if __name__ == "__main__":
    execute.run()
```

Register in `config.yaml`:

```yaml
script_plugins:
  weather:
    command: "python3"
    args: ["plugins/my_plugin.py"]
```

The SDK is at `sdks/python/`. Install with `pip install -e sdks/python/`.

### Writing Plugins in JavaScript (AssemblyScript)

AssemblyScript compiles TypeScript-like code directly to WASM modules matching AmanClaw's ABI.

```typescript
// assembly/index.ts
import { SkillMetadata, SkillInput, SkillResult, stringToPtr } from "./sdk";

function getMetadata(): SkillMetadata {
  const meta = new SkillMetadata();
  meta.name = "hello_js";
  meta.description = "A greeting skill";
  meta.timeout_ms = 10000;
  meta.version = "0.1.0";
  return meta;
}

function executeSkill(input: SkillInput): SkillResult {
  const args = JSON.parse<Map<string, string>>(input.args);
  const name = args.has("name") ? args.get("name") : "World";
  return SkillResult.ok("Hello, " + name + "!");
}
```

Build: `npm run build` → produces `build/plugin.wasm`. Copy to the plugins directory.

Template project at `sdks/assemblyscript/`.

---

## MCP Server

AmanClaw can expose its skills as [Model Context Protocol](https://modelcontextprotocol.io/) tools, making them available to any MCP client (Claude Code, Claude Desktop, etc.).

### HTTP Transport

Set `MCP_HTTP_PORT` to start the MCP HTTP server alongside the bot:

```bash
MCP_HTTP_PORT=3001 ./amanclaw
```

Then configure your MCP client to connect to `http://your-server:3001/mcp`.

### Stdio Transport

For local use with Claude Code, add to your MCP config:

```json
{
  "mcpServers": {
    "amanclaw": {
      "command": "/path/to/amanclaw",
      "args": ["--mcp-stdio"]
    }
  }
}
```

### Available MCP Tools

All registered skills (built-in + WASM plugins) are automatically exposed as MCP tools with their parameter schemas. For example:

- `system_info` — Get system information
- `shell` — Execute whitelisted shell commands
- `solat`, `quran`, `qiblat`, `hijri`, `doa` — Islamic community skills
- Any custom WASM or script plugins

### MCP Client Bridge (Consuming External MCP Servers)

AmanClaw can also act as an MCP **client**, connecting to external MCP servers and importing their tools as skills. This lets you use tools from any MCP server (GitHub, Linear, filesystem, databases, etc.) directly through AmanClaw.

Configure external MCP servers in `config.yaml`:

```yaml
mcp_servers:
  # Stdio transport — spawns a child process
  github:
    command: "npx"
    args: ["-y", "@modelcontextprotocol/server-github"]
    env:
      GITHUB_PERSONAL_ACCESS_TOKEN: "${GITHUB_TOKEN}"

  filesystem:
    command: "npx"
    args: ["-y", "@modelcontextprotocol/server-filesystem", "/home/user/docs"]

  # HTTP transport — connects to a remote MCP server
  remote-tools:
    url: "https://mcp.example.com/mcp"
```

Key features:
- **Auto-discovery** — Tools are discovered via `tools/list` on startup
- **Namespacing** — Tools are namespaced as `{server}__{tool_name}` to avoid collisions (e.g. `github__create_issue`)
- **Env var resolution** — Use `${VAR}` syntax to inject secrets from process environment
- **Dual transport** — Supports both stdio (child process) and HTTP transports
- **Seamless integration** — External tools appear alongside built-in and WASM skills in the LLM's tool list

---

## REST Management API

AmanClaw includes a REST management API (`amanclaw-api` crate) built with Axum. Set `API_PORT` to enable it:

```bash
API_PORT=8090 API_TOKEN=my-secret-token ./amanclaw
```

### Endpoints

| Group | Endpoint | Method | Description |
| ----- | -------- | ------ | ----------- |
| Bot | `/api/status` | GET | Bot status, uptime, stats |
| Communities | `/api/communities` | GET | List all communities |
| Communities | `/api/communities` | POST | Create community |
| Communities | `/api/communities/{id}` | GET | Get community |
| Communities | `/api/communities/{id}` | DELETE | Delete community |
| Communities | `/api/communities/{id}/skills` | PUT | Update community skills |
| Skills | `/api/skills` | GET | List registered skills |
| Users | `/api/users` | GET | List all users |
| Users | `/api/users/{platform}/{id}/approve` | POST | Approve a user |
| Users | `/api/users/{platform}/{id}/block` | POST | Block a user |
| Webhooks | `/api/webhooks` | GET | List configured webhooks |
| Webhooks | `/hooks/{webhook_id}` | POST | Receive inbound webhook (own auth) |
| Gateway | `/ws` | GET | WebSocket upgrade (JSON-RPC 2.0) |

Authenticated endpoints require `Authorization: Bearer <token>`. Webhook receiver (`/hooks/*`) and WebSocket (`/ws`) use their own auth mechanisms. The API binds to `127.0.0.1` only (not exposed to network by default).

---

## Desktop Admin App

Cross-platform desktop app for managing AmanClaw bot instances. Built with Tauri 2 (Rust) + Svelte 5 + Tailwind CSS 4.

### Features

- **Dashboard** — Stats overview, bot status, quick actions
- **Communities** — List, add, edit communities with zone/language/skills config
- **Skills** — Global skill toggle with status
- **Users** — User list with approve/block actions and status badges
- **Content** — Manage doa collection, zakat rates, khutbah cache
- **Logs** — Live log stream with filtering
- **Settings** — Local/remote mode switch, server URL + token config
- **System tray** — Background operation with native notifications (solat, user pending, skill errors)

### Running the Desktop App

```bash
# Development
cd desktop
npm install
cargo tauri dev

# Build for production
cargo tauri build
# Outputs: .dmg (macOS), .msi (Windows), .AppImage (Linux)
```

### Connection Modes

- **Local mode** — Desktop app embeds the bot engine, manages it directly
- **Remote mode** — Connects to a remote AmanClaw instance via the REST management API

---

## Islamic Community Skills

AmanClaw comes with 11 Islamic skills designed for Malaysian Muslim communities. All skills use official JAKIM (Jabatan Kemajuan Islam Malaysia) data sources where applicable.

### Built-in Skills (Rust)

| Skill | Description | Data Source |
| ----- | ----------- | ----------- |
| `solat` | Prayer times by JAKIM zone, proactive azan reminders | JAKIM e-Solat API |
| `quran` | Verse lookup, search, tafsir (BM + English), daily verse | Quran.com API |
| `qiblat` | Qiblat direction and distance to Kaaba | Great Circle calculation |
| `hijri` | Hijri date conversion, Islamic events, Ramadan countdown | Hijri algorithm + JAKIM |
| `doa` | Daily doa, morning/evening azkar, categorized collection | Local database |

### Script Plugins (Python)

| Skill | Description | Data Source |
| ----- | ----------- | ----------- |
| `hadith` | Search hadith by keyword across major collections | sunnah.com API |
| `halal` | Verify product/restaurant halal status by name or cert number | JAKIM Halal Portal |
| `zakat` | Calculate zakat fitrah, pendapatan, simpanan, emas | JAKIM rates |
| `masjid` | Find nearest masjid/surau by location | Google Places API |
| `khutbah` | Latest weekly Friday khutbah from JAKIM | JAKIM portal |
| `jakim` | JAKIM services directory, fatwa search, events calendar | JAKIM portal |

### Multi-Community Support

One AmanClaw instance can serve many community groups (masjid committees, usrah groups, school Islamic societies). Each community gets:

- **Prayer zone** — JAKIM zone for accurate solat times (e.g. SGR01, WLY01, JHR02)
- **Language preference** — BM, English, or rojak (natural mix)
- **Skill selection** — Enable/disable specific skills per community
- **Push notifications** — Solat reminders, daily doa, weekly khutbah

Communities self-onboard when the bot is added to a group. An in-chat wizard guides the admin through zone, language, and skills setup.

### JAKIM Prayer Zones

All 14 states + WP are supported with their JAKIM zone codes:

| State | Zones |
| ----- | ----- |
| Johor | JHR01-JHR04 |
| Kedah | KDH01-KDH07 |
| Kelantan | KTN01-KTN02 |
| Melaka | MLK01 |
| Negeri Sembilan | NGS01-NGS02 |
| Pahang | PHG01-PHG05 |
| Perak | PRK01-PRK07 |
| Perlis | PLS01 |
| Pulau Pinang | PNG01 |
| Sabah | SBH01-SBH09 |
| Sarawak | SWK01-SWK09 |
| Selangor | SGR01-SGR03 |
| Terengganu | TRG01-TRG04 |
| WP KL/Putrajaya | WLY01 |
| WP Labuan | WLY02 |

---

## Configuration

### Secrets (`.env`)

Never commit this file. Set `chmod 600 .env`.

| Variable | Required | Purpose |
| -------- | -------- | ------- |
| `TELEGRAM_BOT_TOKEN` | For Telegram | Telegram bot token |
| `LLM_API_KEY` | Yes | LLM API key |
| `DISCORD_BOT_TOKEN` | For Discord | Discord bot token |
| `WHATSAPP_ACCESS_TOKEN` | For WhatsApp | WhatsApp Cloud API token |
| `WHATSAPP_PHONE_NUMBER_ID` | For WhatsApp | WhatsApp phone number ID |
| `WAHA_API_URL` | For WAHA | WAHA bridge base URL |
| `SUNNAH_API_KEY` | For Hadith | sunnah.com API key |
| `GOOGLE_PLACES_API_KEY` | For Masjid | Google Places API key |

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

skills:
  soul_dir: "./souls"           # SOUL.md agent personality files
  disabled: []                  # Skills to disable by name
  skill_timeout_seconds: 30

# Agent profiles with SOUL.md support
agents:
  ustazbot:
    id: ustazbot
    name: UstazBot
    system_prompt: ""
    soul_file: "ustazbot.md"    # Loaded from soul_dir
    allowed_skills: [solat, quran, doa]
    memory_namespace: ustaz

# Agent routing rules
routing:
  default_agent: default
  rules:
    - match: { platform: telegram, topic_id: "islamic" }
      agent: ustazbot

# Scheduled jobs
cron:
  timezone: "Asia/Kuala_Lumpur"
  jobs:
    morning_doa:
      name: "Morning Doa"
      schedule: "0 6 * * *"
      type: skill_invocation
      skill: doa
      input: '{"category": "pagi"}'
      targets:
        - platform: telegram
          chat_id: "-1001234567890"

# Inbound webhooks
webhooks:
  base_path: "/hooks"
  endpoints:
    github:
      name: "GitHub Events"
      path: "/github"
      auth:
        type: hmac_sha256
        secret: "${GITHUB_WEBHOOK_SECRET}"
      transform:
        type: template
        template: "{{action}} on {{repository.full_name}}: {{pull_request.title}}"
      targets:
        - platform: telegram
          chat_id: "-1001234567890"

# WebSocket gateway
gateway:
  enabled: true
  heartbeat_interval_secs: 30
  max_connections: 50

# Sub-agent spawning
subagents:
  enabled: true
  max_per_session: 5
  max_global: 20
  max_depth: 2
  default_timeout_secs: 120

# Skill marketplace registry
registry:
  enabled: false
  skills_dir: "./plugins/registry"
  remote_url: "https://registry.amanclaw.my"
```

### Environment Overrides

| Variable | Purpose |
| -------- | ------- |
| `MEMORY_DB_PATH` | Override SQLite database path |
| `LOG_FORMAT` | `text` or `json` |
| `RUST_LOG` | Log level filter (e.g. `amanclaw=debug`) |
| `API_PORT` | Start REST management API on this port (e.g. `8090`) |
| `API_TOKEN` | Bearer token for management API (auto-generated if not set) |

---

## LLM Providers

AmanClaw works with **any OpenAI-compatible API**. It calls `POST {base_url}/chat/completions` with Bearer token auth. This means you can use local models, cloud APIs, or any proxy that speaks this format.

### Local Models (Free, Private)

#### Ollama

Run open-source models locally. Best for getting started.

```bash
# Install and run a model
ollama run qwen3:8b
```

```yaml
# config.yaml
llm:
  base_url: "http://localhost:11434/v1"
  model: "qwen3:8b"
```

```bash
# .env — no API key needed
LLM_API_KEY=ollama
```

Recommended models: `qwen3:8b`, `llama3.1:8b`, `mistral`, `deepseek-r1:8b`, `gemma3:12b`

#### vLLM

High-throughput serving for production. Supports tool calling natively.

```bash
vllm serve Qwen/Qwen3-30B-A3B --port 8001
```

```yaml
llm:
  base_url: "http://localhost:8001/v1"
  model: "Qwen/Qwen3-30B-A3B"
```

#### LM Studio

Desktop app with GUI. Download models from Hugging Face, click "Start Server".

```yaml
llm:
  base_url: "http://localhost:1234/v1"
  model: "loaded-model-name"
```

#### LocalAI

Drop-in OpenAI replacement with GGUF model support.

```bash
docker run -p 8080:8080 -v ./models:/models localai/localai
```

```yaml
llm:
  base_url: "http://localhost:8080/v1"
  model: "your-model"
```

#### llama.cpp Server

Minimal C++ inference server. Lowest resource usage.

```bash
./llama-server -m model.gguf --port 8080
```

```yaml
llm:
  base_url: "http://localhost:8080/v1"
  model: "model"
```

### Cloud Providers

#### OpenAI

```yaml
llm:
  base_url: "https://api.openai.com/v1"
  model: "gpt-4o"
```

```bash
LLM_API_KEY=sk-your-openai-key
```

#### Anthropic (via OpenAI-compatible proxy)

Anthropic's native API is not OpenAI-compatible. Use a proxy like [LiteLLM](https://github.com/BerriAI/litellm):

```bash
litellm --model anthropic/claude-sonnet-4-20250514 --port 4000
```

```yaml
llm:
  base_url: "http://localhost:4000/v1"
  model: "anthropic/claude-sonnet-4-20250514"
```

```bash
ANTHROPIC_API_KEY=sk-ant-your-key
LLM_API_KEY=sk-1234  # LiteLLM proxy key
```

Or use any other proxy that translates OpenAI format to Anthropic's API (OpenRouter, AWS Bedrock proxy, etc.).

#### Qwen (Alibaba Cloud / DashScope)

Qwen models via Alibaba's DashScope API (OpenAI-compatible endpoint):

```yaml
llm:
  base_url: "https://dashscope.aliyuncs.com/compatible-mode/v1"
  model: "qwen-plus"
```

```bash
LLM_API_KEY=sk-your-dashscope-key
```

Available models: `qwen-turbo`, `qwen-plus`, `qwen-max`, `qwen-vl-max` (vision)

#### Kimi (Moonshot AI)

```yaml
llm:
  base_url: "https://api.moonshot.cn/v1"
  model: "moonshot-v1-8k"
```

```bash
LLM_API_KEY=sk-your-moonshot-key
```

Available models: `moonshot-v1-8k`, `moonshot-v1-32k`, `moonshot-v1-128k`

#### DeepSeek

```yaml
llm:
  base_url: "https://api.deepseek.com/v1"
  model: "deepseek-chat"
```

```bash
LLM_API_KEY=sk-your-deepseek-key
```

Available models: `deepseek-chat`, `deepseek-reasoner`

#### Groq (Fast Inference)

```yaml
llm:
  base_url: "https://api.groq.com/openai/v1"
  model: "llama-3.1-70b-versatile"
```

```bash
LLM_API_KEY=gsk_your-groq-key
```

#### Together AI

```yaml
llm:
  base_url: "https://api.together.xyz/v1"
  model: "meta-llama/Llama-3.1-70B-Instruct-Turbo"
```

```bash
LLM_API_KEY=your-together-key
```

#### OpenRouter (Multi-Provider Gateway)

Access 200+ models from one API. Pay per token across providers.

```yaml
llm:
  base_url: "https://openrouter.ai/api/v1"
  model: "anthropic/claude-sonnet-4-20250514"  # or any model on OpenRouter
```

```bash
LLM_API_KEY=sk-or-your-openrouter-key
```

#### Hugging Face (Inference API)

Use models hosted on Hugging Face's serverless inference:

```yaml
llm:
  base_url: "https://api-inference.huggingface.co/v1"
  model: "meta-llama/Llama-3.1-8B-Instruct"
```

```bash
LLM_API_KEY=hf_your-huggingface-token
```

For dedicated endpoints (Inference Endpoints), use the endpoint URL as `base_url`.

### Feature Compatibility

Not all providers support all features equally:

| Feature | Requirement | Providers |
| ------- | ----------- | --------- |
| **Basic chat** | `/v1/chat/completions` | All listed above |
| **Tool calling** | `tool_calls` in response | OpenAI, vLLM, Groq, Together, DeepSeek, Qwen, Ollama (most models) |
| **Vision** | Multimodal `content` array | OpenAI (gpt-4o), Qwen-VL, LLaVA via Ollama, vLLM with VL models |
| **Streaming** | Not used by AmanClaw | N/A |

> **Tip:** For the best experience with tool calling and vision, use models that natively support OpenAI's function calling format. Qwen3, GPT-4o, and Llama 3.1+ work well.

### Using LiteLLM as a Universal Proxy

[LiteLLM](https://github.com/BerriAI/litellm) can proxy 100+ providers behind a single OpenAI-compatible endpoint:

```bash
pip install litellm
litellm --model ollama/qwen3:8b --port 4000
# or
litellm --model anthropic/claude-sonnet-4-20250514 --port 4000
# or
litellm --model deepseek/deepseek-chat --port 4000
```

```yaml
llm:
  base_url: "http://localhost:4000/v1"
  model: "ollama/qwen3:8b"
```

This is the easiest way to use providers that don't natively support the OpenAI format.

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

```bash
# Using Docker for cross-compilation
cd rust
docker run --rm -v "$(pwd)":/app -w /app rust:1.85-slim bash -c "
  apt-get update -qq && apt-get install -y -qq gcc-aarch64-linux-gnu
  rustup target add aarch64-unknown-linux-gnu
  CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc \
    cargo build --release --target aarch64-unknown-linux-gnu -p amanclaw-cli
"

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
- Epoch-based interruption for runaway plugins

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

### Build a WASM plugin

```bash
rustup target add wasm32-unknown-unknown
cargo build --target wasm32-unknown-unknown --release -p amanclaw-skill-echo-wasm
```

### Run with debug logging

```bash
RUST_LOG=amanclaw=debug cargo run -p amanclaw-cli
```

### Test coverage

```text
210+ tests across 27 crates
├── amanclaw-traits        17 tests (config, messages, skills, channels, agents, events)
├── amanclaw-core          31 tests (pipeline, router, registry, scheduler, webhooks, soul, subagent, integration)
├── amanclaw-security      11 tests (auth, rate limiter, sanitizer)
├── amanclaw-memory        17 tests (history, facts, summaries, pruning, FTS5, hybrid RRF)
├── amanclaw-llm            5 tests (LLM client, tool call parsing, thinking tags, embeddings)
├── amanclaw-wasm-runtime  13 tests (loader, host, sandbox, runtime, watcher)
├── amanclaw-mcp           17 tests (protocol, handler, HTTP, client, bridge)
├── amanclaw-gateway       17 tests (protocol, session, handler, subscriptions)
├── amanclaw-registry       9 tests (manifest, local install/uninstall/search, remote index)
├── amanclaw-plugin-sdk     5 tests
├── skill-sysinfo           2 tests
├── skill-shell             4 tests
├── channel-telegram        1 test
├── channel-discord         1 test
├── channel-whatsapp        2 tests
├── channel-whatsapp-web    4 tests
├── channel-slack           4 tests (platform, env, envelope parsing)
├── amanclaw-script-runtime 2 tests (config parsing, discovery)
├── skill-solat              9 tests (prayer times, JAKIM zones)
├── skill-qiblat             7 tests (direction, distance)
├── skill-hijri             13 tests (conversion, events)
├── skill-doa               17 tests (categories, search)
└── skill-quran             11 tests (lookup, search, surahs)
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

1. Fork the repo and create your branch from `main`
2. Add tests for any new functionality
3. Ensure `cargo test --workspace` passes with zero failures
4. Ensure `cargo clippy` has no warnings
5. Update documentation if you changed public APIs
6. Open a PR with a clear description of what and why

### Areas Where Help Is Appreciated

| Area | Description | Difficulty |
| ---- | ----------- | ---------- |
| **Islamic skill plugins** | Improve doa collection, add more hadith sources, refine halal scraping | Easy |
| **New skill plugins** | Weather, translation, news, etc. | Easy |
| **LINE / Viber adapter** | Channel adapters for more messaging platforms | Medium |
| **Web dashboard** | Web-based admin panel (desktop app already done) | Medium |
| **Documentation** | Tutorials, examples, architecture docs | Easy |
| **Security review** | Audit injection detection, auth flow, sandbox | Hard |
| **i18n / localization** | Improve BM translations, add Jawi script support | Easy |
| **JAKIM API research** | Document and test official JAKIM API endpoints | Easy |

### Writing a Plugin

The easiest way to contribute is by writing a new skill plugin. See the [WASM Plugins](#writing-wasm-plugins) section above, or create a built-in Rust skill by implementing the `Skill` trait.

---

## Roadmap

### Core Platform (Done)

- [x] Core engine with async pipeline
- [x] Telegram channel adapter
- [x] LLM client (OpenAI-compatible)
- [x] SQLite conversation memory
- [x] Security (auth, rate limiting, injection detection)
- [x] WASM plugin runtime (loader, sandbox, SDK)
- [x] Built-in skills (sysinfo, shell)
- [x] Docker & systemd deployment
- [x] LLM tool calling loop (multi-round skill execution)
- [x] Admin commands (`/approve`, `/block`, `/stats`, `/users`)
- [x] Conversation auto-summarization
- [x] Learning engine (`/remember`, `/forget`, `/learned`)
- [x] Vision support (image analysis via multimodal LLM)
- [x] Plugin hot reload (filesystem watcher)
- [x] Full WASM plugin instantiation and execution
- [x] Discord channel adapter
- [x] WhatsApp Cloud API channel adapter
- [x] WhatsApp Web adapter (unofficial, via WAHA)
- [x] MCP server integration (stdio + HTTP transports)
- [x] MCP client bridge (consume external MCP server tools)
- [x] Slack channel adapter (Socket Mode)
- [x] Python and JavaScript (AssemblyScript) plugin SDKs
- [x] RAG with SQLite vector store (knowledge base indexing + cosine similarity search)
- [x] Embedding client for vector store indexing and retrieval

### Phase 1: Islamic Community Skills (Done)

- [x] skill-solat — Prayer times via JAKIM e-Solat API (Rust)
- [x] skill-quran — Quran search & lookup via Quran.com API (Rust)
- [x] skill-qiblat — Qiblat direction calculation (Rust)
- [x] skill-hijri — Islamic calendar & date conversion (Rust)
- [x] skill-doa — Doa & zikir collection (Rust)
- [x] skill-hadith — Hadith search via sunnah.com (Python)
- [x] skill-halal — JAKIM halal verification (Python)
- [x] skill-zakat — Zakat calculator (Python)
- [x] skill-masjid — Mosque finder via Google Places (Python)
- [x] skill-khutbah — Weekly JAKIM khutbah (Python)
- [x] skill-jakim — JAKIM services & fatwa search (Python)
- [x] Multi-community model (per-group zone, language, skills config)

### Phase 1.5: Desktop Admin App (Done)

- [x] REST management API (amanclaw-api crate, Axum, 10 endpoints)
- [x] Tauri 2 desktop app with Svelte 5 + Tailwind CSS 4
- [x] Dashboard, Communities, Skills, Users, Content, Logs, Settings pages
- [x] System tray with native notifications (solat, users, skill errors)
- [x] Local/remote mode switching
- [x] Apple-style clean minimal UI

### Phase 1.75: OpenClaw Parity — Advanced Engine Features (Done)

- [x] FTS5 hybrid search — BM25 full-text + cosine vector similarity via Reciprocal Rank Fusion
- [x] SOUL.md agent personas — YAML frontmatter, inheritance chains, variable interpolation
- [x] Cron scheduler — Timezone-aware jobs (direct message, skill invocation, agent prompt) with pipeline bypass
- [x] Webhook triggers — Inbound routes with HMAC-SHA256/Bearer/header auth, Handlebars transforms
- [x] WebSocket gateway — JSON-RPC 2.0 real-time protocol, session management, glob-based topic subscriptions
- [x] Sub-agent spawning — Parallel task execution with per-session/global limits and max-depth control
- [x] Skill marketplace — `amanclaw-skill.toml` manifest, local SQLite registry, remote index with checksum verification
- [x] Event system — `EventEmitter` trait for broadcasting pipeline events to WebSocket subscribers
- [x] Schema migrations — `cron_history` and `webhook_history` tables for execution tracking

### Phase 2: Community Onboarding

- [ ] In-chat onboarding wizard (bot added to group → setup flow)
- [ ] Web dashboard for community admins (dashboard.amanclaw.my)
- [ ] Proactive notifications (solat reminders, daily doa, weekly khutbah)

### Phase 3: AmanClaw Cloud

- [ ] Managed hosting platform for non-technical communities
- [ ] Freemium model (free: solat/doa/hijri/qiblat, paid: halal/quran/zakat)
- [ ] Self-hosted open source + managed cloud

### Phase 4: Specialized Bots

- [ ] UstazBot — Islamic Q&A focused bot
- [ ] HalalBot — Halal verification focused bot
- [ ] SolatBot — Prayer times & reminders focused bot

### Phase 5: Plugin Marketplace

- [x] Skill manifest format (`amanclaw-skill.toml`) with semver, dependencies, metadata
- [x] Local registry — install, uninstall, search installed skills (SQLite-backed)
- [x] Remote registry — index refresh, search, download with SHA256 checksum verification
- [ ] Public registry hosting (registry.amanclaw.my)
- [ ] Skill discovery and installation via bot commands (`/install`, `/search`)
- [ ] Skill ratings and reviews

---

## FAQ

**Q: What LLM should I use?**
Any OpenAI-compatible API works. See the [LLM Providers](#llm-providers) section for detailed setup guides. Quick picks: Ollama (free, local), Groq (fast, free tier), OpenAI GPT-4o (best tool calling), Qwen3 (best open-source).

**Q: Can I use this with multiple people?**
Yes. Add user IDs to `admin_users` in config. Non-admin users go through an approval flow.

**Q: How much resources does it need?**
~2MB RAM, <1% CPU idle on a Raspberry Pi 4. It's Rust — it's fast.

**Q: Can I write plugins in Python?**
Yes! Python plugins use a subprocess-based protocol (JSON over stdin/stdout). Write a Python script with the `@plugin` decorator, register it in `config.yaml` under `script_plugins`, and it works alongside WASM and built-in skills. See the [Python plugin section](#writing-plugins-in-python) and `sdks/python/` for the SDK.

**Q: Is my data stored?**
Conversations are stored in a local SQLite database. Nothing leaves your server except LLM API calls.

**Q: How does auto-summarization work?**
When a user's message count exceeds 40, the engine asks the LLM to summarize the conversation, saves the summary, and prunes old messages (keeping the 10 most recent). The summary is included in future prompts as context.

---

## License

MIT License. See [LICENSE](LICENSE) for details.

---

## Acknowledgments

Built with care from Puncak Alam, Malaysia. Made possible by the Rust ecosystem and the communities behind teloxide, serenity, wasmtime, axum, tokio, and sqlx.

*Malaysia boleh!*
