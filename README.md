<p align="center">
  <img src="apps/desktop/src-tauri/icons/app-icon.png" width="120" alt="AmanClaw" />
  <br>
  <strong style="font-size:2em">AmanClaw</strong>
  <br>
  <em>Your AI, your rules. Built on principles you trust.</em>
</p>

<p align="center">
  <a href="LICENSE"><img src="https://img.shields.io/badge/License-MIT-yellow.svg" alt="MIT License" /></a>
  <img src="https://img.shields.io/badge/rust-1.85+-orange.svg" alt="Rust 1.85+" />
  <img src="https://img.shields.io/badge/docker-ready-blue.svg" alt="Docker" />
  <img src="https://img.shields.io/badge/platform-macOS%20%7C%20Windows%20%7C%20Linux%20%7C%20Pi-lightgrey.svg" alt="Platform" />
</p>

<p align="center">
  A sovereign AI personal agent. One binary. Runs anywhere — your laptop, Raspberry Pi, or your own cloud.
  <br>
  26 skills from coding to Islamic finance. Your data never leaves your control.
  <br>
  Built in Malaysia. Open source. No bloat. Bilingual BM + English.
</p>

---

---

## What Is This?

AmanClaw is a sovereign AI personal agent — it handles everyday tasks like coding, research, automation, and data analysis, plus world-class Islamic AI capabilities that no one else offers. Use it from the terminal, chat apps, or the desktop app. You own your data, you choose your model, you decide where it runs.

```text
You (Telegram / Discord / WhatsApp / Slack)
  │
  ├── Chat message ──► Engine ──► Agent Router ──► Pipeline:
  │                       │         │               Metrics → Auth → Commands → Rate Limit
  │                       │         │               → Sanitize → RLE Retrieve → Context
  │                       │         │               → LLM ◄──► Skills (up to 5 rounds)
  │                       │         │                                         │
  ├── Cron job ─────► Scheduler ──►─┘  (bypass auth + rate limit)            │
  │                                                                          │
  ├── Webhook ──────► Router ──► Auth (HMAC/Bearer) ──► Transform ──► Pipeline
  │                                                                          │
  ├── WebSocket ────► Gateway (JSON-RPC 2.0) ──► Session Manager             │
  │                                                                          │
  ▼                                                                          ▼
Reply  ◄─────────────────────────────────────────────────────────────────────┘
```

---

## Highlights

<table>
  <tr>
    <td width="50%">

### 31 Skills (16 Islamic + 15 General)
Quran with tafsir, Hadith with isnad grading, Fiqh multi-madhab resolver, Shariah screening, Islamic financing — plus web search, weather, JSON tools, and more

</td>
    <td width="50%">

### CLI Agent Mode
`amanclaw ask`, `amanclaw chat`, `amanclaw agent` — use from the terminal, piped input, or autonomous task execution

</td>
  </tr>
  <tr>
    <td width="50%">

### 5 Chat Channels + MCP
Telegram, Discord, WhatsApp (official + WAHA), Slack — plus full MCP protocol support (client + server + SSE)

</td>
    <td width="50%">

### Sovereign Islamic AI
Offline Quran + Hadith + Fiqh knowledge engine. Ethical AI guardrails with scholarly attribution. Hijri calendar scheduling. Your data, your rules.

</td>
  </tr>
  <tr>
    <td width="50%">

### Any LLM Backend
Ollama, vLLM, LM Studio, OpenAI, Groq, DeepSeek, Qwen, Together AI, OpenRouter — anything OpenAI-compatible

</td>
    <td width="50%">

### AmanClaw Cloud
Managed hosting with invite-only beta. Sign up, get a bot, chat from browser. K3s on Hostinger Malaysia. No terminal needed.

</td>
  </tr>
</table>

## CLI Agent Mode

Use AmanClaw directly from your terminal:

```bash
# One-shot question
amanclaw ask "What time is Maghrib in KL?"

# Interactive chat
amanclaw chat

# Autonomous agent
amanclaw agent --task "Find prayer times for today and calculate zakat on RM50,000"

# Piped input
echo "Translate this to BM" | amanclaw ask

# MCP server mode
amanclaw mcp serve --transport sse --port 3001
```

## AmanClaw Cloud

Managed hosting — sign up, get a bot, chat from your browser. No terminal needed.

```bash
# Operator: setup infrastructure
./infra/scripts/setup-k3s.sh           # Install K3s + TLS on Hostinger VPS
./infra/scripts/deploy.sh              # Deploy to cloud.amanclaw.my

# Operator: generate invites
amanclaw-cloud invite create --email user@example.com
# → Code: ABC12345

# User: sign up → connect channels → chat
# 1. Sign up at cloud.amanclaw.my with invite code
# 2. Get dashboard at cloud.amanclaw.my/t/my-bot/admin/
# 3. Connect Telegram/WhatsApp via dashboard
# 4. Chat via cloud.amanclaw.my/t/my-bot/chat (web widget)
```

### Cloud Architecture

- **Multi-tenant** — shared process, per-tenant SQLite isolation
- **Lazy engines** — tenant engines start on first request, stop after 30 min idle
- **Data sovereignty** — all data on Hostinger Malaysia VPS
- **Invite-only beta** — no billing, controlled rollout
- **Daily backups** — automated K8s CronJob with 7-day retention

### Cloud CLI

```bash
amanclaw-cloud serve --port 8443       # Start cloud server
amanclaw-cloud invite create/list      # Manage invite codes
amanclaw-cloud tenant list/info/suspend # Manage tenants
```

---

## All Features

<details>
<summary><b>Click to expand full feature list</b></summary>

- **Pre-built products** — CommunityBot: deploy a ready-made community assistant to Fly.io, Railway, or Render in 3 commands
- **Blazing fast** — Rust async runtime, ~2MB memory footprint, instant startup
- **Global prayer times** — Pure-Rust calculation engine supporting MWL, ISNA, Egyptian, Karachi, Umm al-Qura, and JAKIM methods — works offline, no API needed
- **Multi-community** — One instance serves many groups with per-community config (zone, language, skills)
- **Channel Setup Hub** — Configure, start, stop, and monitor all 5 channels from the dashboard or desktop app. WhatsApp Web gets in-app QR code display for seamless phone scanning. Config persists to config.yaml with env var fallback
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
- **CLI agent mode** — `amanclaw ask` (one-shot), `amanclaw chat` (interactive REPL), `amanclaw agent` (autonomous multi-round task execution) with piped stdin support
- **15 general-purpose skills** — Web search (DuckDuckGo), URL reader, weather (Open-Meteo), datetime/timezone, unit converter, todo list, reminders, JSON tool, base64, hash, regex, HTTP client, CSV tool, summarizer, translator — all zero API keys
- **MCP integration** — Expose skills as MCP tools + consume external MCP servers as skills. SSE transport, Resources, and Prompts support
- **Islamic knowledge engine** — Offline Quran with tafsir (Ibn Kathir, Al-Jalalayn), 6 Hadith collections with isnad grading, Fiqh multi-madhab resolver with RAG — all stored locally in SQLite, synced via `amanclaw islamic sync`
- **Islamic finance** — Shariah stock screening (debt/revenue ratios, purification), expanded zakat (7 types: fitrah, income, savings, gold, business, agriculture, livestock), murabaha/musharakah/ijarah financing calculator
- **Ethical AI guardrails** — 3-layer Islamic content filtering: system prompt guidelines, scholarly attribution detection, automatic disclaimers for unattributed rulings
- **Hijri scheduling** — Calendar-aware event scheduling on Islamic dates (Ramadan reminders, Eid greetings) with automatic Hijri-to-Gregorian conversion
- **Multi-agent orchestrator** — Dependency-based parallel task execution with topological sort, configurable worker limits
- **AmanClaw Cloud** — Multi-tenant managed hosting with invite-only beta signup, per-tenant SQLite isolation, lazy engine start/stop, web chat widget, cloud management API
- **K8s deployment** — K3s manifests, Dockerfile, setup/deploy/backup scripts for Hostinger Malaysia VPS with TLS via cert-manager
- **Hybrid search** — FTS5 full-text search with BM25 ranking + cosine vector similarity via Reciprocal Rank Fusion (RRF)
- **SOUL.md agent personas** — YAML-frontmatter agent personality files with inheritance chains and variable interpolation
- **Cron scheduler** — Scheduled jobs (direct messages, skill invocations, agent prompts) with timezone support and pipeline bypass
- **Webhook triggers** — Inbound webhook routes with HMAC-SHA256/Bearer/header auth, Handlebars template transforms, and rate limiting
- **WebSocket gateway** — JSON-RPC 2.0 real-time gateway with session management, topic subscriptions, and glob-based event routing
- **Sub-agent spawning** — Parallel task execution via spawned sub-agents with per-session/global limits and max-depth control
- **Skill marketplace** — `amanclaw-skill.toml` manifest format, local registry with SQLite-backed install/search/update/remove, remote index with SHA256 verification, version pinning (`name@version`)
- **Event system** — `EventEmitter` trait for broadcasting pipeline events (message.received, message.sent, security.*)
- **Desktop admin app** — Cross-platform Tauri 2 desktop app (macOS, Windows, Linux) with system tray and native notifications
- **REST management API** — Axum-based REST API for bot status, communities, skills, users, webhooks management
- **Production-ready** — Docker with hardened containers, systemd service, structured logging
- **Cross-platform** — Runs on x86_64, ARM64 (Raspberry Pi), and anywhere Rust compiles

</details>

---

## Quick Start

### Prerequisites

- [Rust 1.85+](https://rustup.rs/) (for building from source)
- An OpenAI-compatible LLM API (local or remote)
- A Telegram bot token (from [@BotFather](https://t.me/BotFather))

### 1. Clone and build

```bash
git clone https://github.com/AmanClaw/amanclaw.git
cd amanclaw
cargo build --release
```

The binary is at `target/release/amanclaw`.

### Quick Setup (alternative)

```bash
# Initialize project with defaults
amanclaw init

# CLI Agent Mode
amanclaw ask "What time is Maghrib in KL?"
amanclaw chat
amanclaw agent --task "Find weather for Kuala Lumpur"

# Start in development mode (no API key needed)
amanclaw dev

# Start with live reload
amanclaw dev --watch

# Open interactive playground
amanclaw playground

# Islamic knowledge database
amanclaw islamic sync              # Download Quran + Hadith + tafsir
amanclaw islamic sync quran        # Sync Quran only
amanclaw islamic status            # Show sync status

# MCP server mode
amanclaw mcp serve --transport sse
amanclaw mcp list
amanclaw mcp tools filesystem

# Skill management
amanclaw skill search "prayer"
amanclaw skill install skill-solat
amanclaw skill install skill-solat@1.2.3    # version pinning
amanclaw skill install-pack islamic
amanclaw skill list-installed
amanclaw skill info web_search
amanclaw skill update all
amanclaw skill remove web_search
amanclaw skill packs

# Create a new skill (generates CI, README, LICENSE, tests)
amanclaw skill new my-skill --lang rust
amanclaw skill new my-skill --lang python
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

## Specialized Products

Pre-configured bot distributions — deploy without touching code.

### CommunityBot

A friendly AI assistant for community group chats. Multilingual, all skills enabled, ready to deploy.

```bash
# Deploy with Docker
cd products/communitybot
cp .env.example .env        # Add your bot token
docker compose up -d

# Or deploy to Fly.io
fly launch
fly secrets set TELEGRAM_BOT_TOKEN=your-token

# Or scaffold a new product
amanclaw product new communitybot
amanclaw product list
```

See [products/communitybot/README.md](products/communitybot/README.md) for full setup guide.

---

## Architecture

AmanClaw is a Cargo workspace with 30 crates organized by role, plus a web dashboard and a Tauri desktop app:

```text
.
├── Cargo.toml                        # Workspace root
├── crates/                           # Library crates
│   ├── amanclaw-traits/              # Core types, traits, config, EventEmitter
│   ├── amanclaw-core/                # Engine, pipeline, router, scheduler, webhooks, soul loader, sub-agents
│   ├── amanclaw-security/            # Auth, rate limiter, sanitizer
│   ├── amanclaw-memory/              # SQLite conversation, facts, summaries, vector store, FTS5 hybrid search
│   ├── amanclaw-llm/                 # OpenAI-compatible LLM client + tool calling + embeddings
│   ├── amanclaw-islamic-db/          # Islamic knowledge database (Quran, Hadith, Fiqh)
│   ├── amanclaw-wasm-runtime/        # WASM plugin loader, sandbox, runtime, watcher
│   ├── amanclaw-plugin-sdk/          # SDK + macro for WASM plugin authors
│   ├── amanclaw-mcp/                 # MCP server + client bridge (stdio + HTTP + SSE), Resources, Prompts
│   ├── amanclaw-script-runtime/      # Script plugin loader (Python/JS via subprocess)
│   ├── amanclaw-api/                 # REST management API + embedded dashboard (Axum)
│   ├── amanclaw-gateway/             # WebSocket gateway (JSON-RPC 2.0, session management)
│   ├── amanclaw-registry/            # Plugin registry for loading and managing skills/channels
│   ├── amanclaw-prayer-times/        # Pure-Rust prayer time calculator (6 methods)
│   └── amanclaw-skill-index/         # Skill marketplace index, search, curated packs, SHA256 verification
├── apps/                             # Application binaries
│   ├── cli/                          # CLI entry point (amanclaw binary)
│   ├── cloud/                        # AmanClaw Cloud — multi-tenant managed hosting
│   ├── dashboard/                    # Svelte 5 + Vite web dashboard
│   └── desktop/                      # Tauri 2 desktop admin app (macOS, Windows, Linux)
├── skills/                           # Built-in Rust skills
│   ├── skill-solat/                  # Prayer times via JAKIM
│   ├── skill-quran/                  # Quran lookup & search with tafsir
│   ├── skill-qiblat/                 # Qiblat direction
│   ├── skill-hijri/                  # Islamic calendar
│   ├── skill-doa/                    # Doa & zikir collection
│   ├── skill-hadith-rs/              # Hadith with isnad grading
│   ├── skill-fiqh/                   # Multi-madhab fiqh resolver
│   ├── skill-sysinfo/                # System info
│   ├── skill-shell/                  # Whitelisted shell commands
│   └── skill-echo-wasm/              # Example WASM plugin
├── channels/                         # Chat platform adapters
│   ├── channel-telegram/             # Telegram adapter (teloxide)
│   ├── channel-discord/              # Discord adapter (serenity)
│   ├── channel-whatsapp/             # WhatsApp Cloud API adapter
│   ├── channel-whatsapp-web/         # Unofficial WhatsApp via WAHA bridge
│   └── channel-slack/                # Slack adapter (Socket Mode)
├── plugins/                          # Python script plugins (23 skills)
├── sdks/                             # Plugin SDKs
│   ├── assemblyscript/               # AssemblyScript (JS/TS) plugin SDK
│   └── python/                       # Python plugin SDK
├── souls/                            # SOUL.md agent personality files
├── infra/                            # Infrastructure
│   ├── docker/                       # Dockerfile, docker-compose.yml
│   ├── k3s/                          # K3s manifests (namespace, deployment, service, ingress, PVC, secrets, backup)
│   └── scripts/                      # setup-k3s.sh, deploy.sh, backup.sh
├── products/                         # Pre-configured bot distributions
├── wit/                              # WASM Interface Types contract
└── docs/                             # Design specs, plans, images
```

### How It Works

1. **Channel adapters** receive messages from platforms and push them into the engine via async channels
2. **Engine** multiplexes chat messages and scheduler events via `tokio::select!`
3. **Agent router** resolves which agent profile handles the message (per-platform, per-topic, per-group, or default)
4. **SOUL.md loader** resolves agent personality files with frontmatter, inheritance, and variable interpolation
5. **Pipeline** runs middleware chain in order:
   - **Metrics** — record pipeline timing and counters
   - **Auth** — check user allowlist + JWT
   - **Commands** — handle special commands (`/remember`, `/forget`, `/learned`, etc.)
   - **Rate Limit** — enforce per-user rate limits
   - **Sanitize** — input sanitization and prompt injection detection
   - **RLE Retrieve** — (optional) Recency-weighted Long-term Embeddings — retrieves learned corrections and knowledge base entries via vector similarity, records hits for recency scoring
   - **Context** — build context window (summary + facts + history + FTS5/vector hybrid search)
   - **Persist** — call LLM, store results, auto-summarize when history exceeds 40 messages
   - **Tool Calling** — execute skills iteratively (up to 5 rounds)
6. Internal messages (cron, webhook, sub-agent) bypass auth, rate limiting, and sanitization
7. **EventEmitter** broadcasts pipeline events (`message.received`, `message.sent`, `security.*`) to WebSocket subscribers
8. **Response** is routed back to the correct channel adapter by platform

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

AmanClaw can expose its skills as [Model Context Protocol](https://modelcontextprotocol.io/) tools, making them available to any MCP client (Claude Code, Claude Desktop, etc.). Supports Tools, Resources, and Prompts.

### SSE Transport (recommended)

```bash
amanclaw mcp serve --transport sse --port 3001
```

Clients connect to `http://your-server:3001/mcp/sse` for streaming, send requests via POST to `http://your-server:3001/mcp`.

### HTTP Transport

Set `MCP_HTTP_PORT` to start the MCP HTTP server alongside the bot:

```bash
MCP_HTTP_PORT=3001 ./amanclaw
```

### Stdio Transport

For local use with Claude Code:

```json
{
  "mcpServers": {
    "amanclaw": {
      "command": "/path/to/amanclaw",
      "args": ["mcp", "serve"]
    }
  }
}
```

### MCP CLI Commands

```bash
amanclaw mcp list              # List configured MCP servers
amanclaw mcp tools filesystem  # List tools from a server
amanclaw mcp serve             # Start as MCP server (stdio)
amanclaw mcp serve -t sse      # Start as MCP server (SSE)
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
| Channels | `/api/channels` | GET | List all channels with status |
| Channels | `/api/channels/{id}` | GET | Get channel config + status |
| Channels | `/api/channels/{id}` | PUT | Update channel config (persists to config.yaml) |
| Channels | `/api/channels/{id}/start` | POST | Start a channel |
| Channels | `/api/channels/{id}/stop` | POST | Stop a channel |
| Channels | `/api/channels/whatsapp-web/qr` | GET | Proxy WAHA QR code (base64 PNG) |
| Channels | `/api/channels/whatsapp-web/session` | GET | Proxy WAHA session status |
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

<p align="center">
  <img src="docs/images/desktop-full.png" alt="AmanClaw Desktop App" width="800" />
  <br><sub>AmanClaw Desktop — manage your bot from a native app on macOS, Windows, or Linux</sub>
</p>

### Features

- **Dashboard** — Stats overview, bot status, quick actions
- **Channel Setup Hub** — Configure, start, stop, and monitor all 5 channels from one page. WhatsApp Web gets in-app QR code display for seamless phone scanning
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
cd apps/desktop
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

## Islamic Skills

AmanClaw comes with 16 Islamic skills — the most comprehensive Islamic AI skill set available. All data can be synced locally for offline use.

### Islamic Knowledge Engine (Rust — offline after sync)

| Skill | Description | Data Source |
| ----- | ----------- | ----------- |
| `quran` | Verse lookup, FTS5 search, tafsir (Ibn Kathir, Al-Jalalayn), thematic search | Local SQLite (synced from Quran.com) |
| `hadith` | Search across 6 collections with isnad grading (sahih/hasan/da'if), grade filtering | Local SQLite (synced from Sunnah.com) |
| `fiqh` | Multi-madhab resolver (Shafi'i, Hanafi, Maliki, Hanbali) with Quran/Hadith citations | Local SQLite + RAG |

### Community Skills (Rust — built-in)

| Skill | Description | Data Source |
| ----- | ----------- | ----------- |
| `solat` | Prayer times by JAKIM zone + 6 global methods, offline calculation | Pure Rust (no API) |
| `qiblat` | Qiblat direction and distance to Kaaba | Great Circle calculation |
| `hijri` | Hijri date conversion, Islamic events, Ramadan countdown | Hijri algorithm |
| `doa` | Daily doa, morning/evening azkar, 9 categories | Local collection |

### Islamic Finance (Python)

| Skill | Description | Data Source |
| ----- | ----------- | ----------- |
| `shariah_screen` | Shariah-compliant stock screening (debt ratio, revenue, purification) | Local calculation |
| `zakat` | 7 types: fitrah, pendapatan, simpanan, emas, perniagaan, pertanian, ternakan | JAKIM rates |
| `murabaha` | Islamic financing calculator: Murabaha, Musharakah Mutanaqisah, Ijarah | Local calculation |

### Community Services (Python)

| Skill | Description | Data Source |
| ----- | ----------- | ----------- |
| `halal` | Verify product/restaurant halal status | JAKIM Halal Portal |
| `masjid` | Find nearest masjid/surau by location | Google Places API |
| `khutbah` | Latest weekly Friday khutbah from JAKIM | JAKIM portal |
| `jakim` | JAKIM services directory, fatwa search | JAKIM portal |

### Ethical AI Guardrails

- **Scholarly attribution** — Islamic rulings always cite sources (Quran, Hadith, scholarly books)
- **Multi-madhab** — Never presents single opinion on disputed matters
- **Automatic disclaimers** — "Consult a qualified scholar" appended when discussing rulings without proper attribution
- **Sensitivity detection** — High-sensitivity topics handled with extra care
- **System prompt guidelines** — Islamic knowledge context injected when Islamic skills are active

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
cd infra/docker
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
docker run --rm -v "$(pwd)":/app -w /app rust:1.85-slim bash -c "
  apt-get update -qq && apt-get install -y -qq gcc-aarch64-linux-gnu
  rustup target add aarch64-unknown-linux-gnu
  CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc \
    cargo build --release --target aarch64-unknown-linux-gnu -p cli
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
cargo build
cargo test --workspace
```

### Build a WASM plugin

```bash
rustup target add wasm32-unknown-unknown
cargo build --target wasm32-unknown-unknown --release -p skill-echo-wasm
```

### Run with debug logging

```bash
RUST_LOG=amanclaw=debug cargo run -p cli
```

### Test coverage

```text
570+ tests across 30+ crates
├── amanclaw-traits        21 tests (config, messages, skills, channels, agents, events, channel_config)
├── amanclaw-core          86 tests (pipeline, router, registry, scheduler, webhooks, soul, subagent, orchestrator, hijri_scheduler)
├── amanclaw-security      48 tests (auth, rate limiter, sanitizer, injection detection, islamic guardrails)
├── amanclaw-memory        78 tests (history, facts, summaries, pruning, FTS5, hybrid RRF, community CRUD)
├── amanclaw-llm           37 tests (LLM client, tool call parsing, thinking tags, embeddings, prompts, islamic guidelines)
├── amanclaw-islamic-db    20 tests (schema, quran queries, hadith queries, fiqh queries, sync metadata, seed data)
├── amanclaw-wasm-runtime  13 tests (loader, host, sandbox, runtime, watcher)
├── amanclaw-mcp           25 tests (protocol, handler, HTTP, client, bridge, resources, prompts)
├── amanclaw-gateway       17 tests (protocol, session, handler, subscriptions)
├── amanclaw-registry      24 tests (manifest, local install/uninstall/search, dependencies, remote index)
├── amanclaw-cli           57 tests (CLI parsing, skill management, MCP, islamic, ask/chat/agent)
├── amanclaw-plugin-sdk     5 tests
├── skill-quran            23 tests (verse lookup, search, tafsir, thematic, IslamicDb integration)
├── skill-hadith           18 tests (lookup, search, browse, grade filtering, empty DB handling)
├── skill-fiqh             13 tests (ask multi-madhab, browse, topics, RAG evidence, disclaimers)
├── skill-solat              9 tests (prayer times, JAKIM zones)
├── skill-qiblat             7 tests (direction, distance)
├── skill-hijri             13 tests (conversion, events)
├── skill-doa               17 tests (categories, search)
├── skill-sysinfo           2 tests
├── skill-shell             4 tests
├── channel-telegram        1 test
├── channel-discord         1 test
├── channel-whatsapp        2 tests
├── channel-whatsapp-web    4 tests
├── channel-slack           4 tests (platform, env, envelope parsing)
└── amanclaw-script-runtime 2 tests (config parsing, discovery)
```

---

## Contributing

Contributions are welcome! Here's how to get started:

### Getting Set Up

```bash
# Fork and clone
git clone https://github.com/YOUR_USERNAME/amanclaw.git
cd amanclaw

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
| **Model fine-tuning** | Fine-tune open models (Qwen, Llama) on Islamic corpus | Hard |
| **Billing integration** | Stripe/payment for cloud subscriptions (freemium → paid) | Medium |
| **New skill plugins** | More general-purpose skills (PDF analysis, calendar, email) | Easy |
| **LINE / Viber adapter** | Channel adapters for more messaging platforms | Medium |
| **Arabic / Urdu / Turkish** | Expand language support beyond BM + English | Easy |
| **Fiqh seed data** | Expand curated fiqh rulings database (currently ~12 entries) | Easy |
| **Cloud dashboard** | Team management, usage analytics, onboarding wizard | Medium |
| **Documentation** | Tutorials, examples, architecture docs | Easy |
| **Security review** | Audit cloud auth, tenant isolation, sandbox | Hard |

### Writing a Plugin

The easiest way to contribute is by writing a new skill plugin. See the [WASM Plugins](#writing-wasm-plugins) section above, or create a built-in Rust skill by implementing the `Skill` trait.

---

## Roadmap

> *"Your AI, your rules. Built on principles you trust."*
>
> AmanClaw is the world's first sovereign AI personal agent with Islamic AI capabilities. Full design spec: [`docs/superpowers/specs/2026-03-14-sovereign-islamic-ai-agent-design.md`](docs/superpowers/specs/2026-03-14-sovereign-islamic-ai-agent-design.md)

### Foundation (Done)

- [x] Rust async engine with pipeline, agent router, and middleware chain
- [x] 5 chat channels: Telegram, Discord, WhatsApp (official + WAHA), Slack
- [x] Any OpenAI-compatible LLM (Ollama, vLLM, OpenAI, Groq, DeepSeek, Qwen, etc.)
- [x] SQLite conversation memory with auto-summarization and hybrid search (FTS5 + vector)
- [x] Security: auth, rate limiting, prompt injection detection, output sanitization
- [x] WASM plugin runtime with sandbox, hot reload, and SDK
- [x] Python/JS script runtime with auto-discovery
- [x] Desktop admin app (Tauri 2 + Svelte 5) with system tray
- [x] Web dashboard with channel setup hub, user management, live logs
- [x] SOUL.md agent personas, cron scheduler, webhooks, WebSocket gateway
- [x] Sub-agent spawning, event system, RAG with embeddings

### Phase 1: General Agent Parity (Done)

- [x] **CLI Agent Mode** — `amanclaw ask` (one-shot), `amanclaw chat` (REPL), `amanclaw agent` (autonomous), piped stdin
- [x] **15 general-purpose skills** — web search, URL reader, weather, datetime, unit converter, todo, reminders, JSON tool, base64, hash, regex, HTTP client, CSV tool, summarizer, translator (all zero API keys)
- [x] **11 Islamic skills** — solat, quran, qiblat, hijri, doa (Rust) + hadith, halal, zakat, masjid, khutbah, jakim (Python)
- [x] **MCP enhancements** — SSE transport, Resources, Prompts, `amanclaw mcp list/tools/serve` commands
- [x] **Skill marketplace CLI** — `list-installed`, `info`, `update`, `remove`, SHA256 checksums, version pinning (`name@version`)
- [x] **460+ tests** across all crates with CI coverage reporting
- [x] **Benchmarks** for pipeline and MCP protocol
- [x] **crates.io ready** — `amanclaw-traits` and `amanclaw-plugin-sdk` prepared for publishing

### Phase 2: Islamic Sovereign Core (Done)

- [x] **Islamic Knowledge Engine** — Offline Quran with tafsir (Ibn Kathir, Al-Jalalayn), thematic FTS5 search, 6 Hadith collections with isnad grading, Fiqh multi-madhab resolver with RAG evidence
- [x] **Islamic Finance** — Shariah stock screening, expanded zakat (7 types), murabaha/musharakah/ijarah financing calculator
- [x] **Ethical AI Guardrails** — 3-layer content filtering (system prompt, attribution detection, post-processing disclaimers), madhab awareness from user preferences
- [x] **Hijri Calendar Scheduling** — Event scheduling on Islamic dates with Hijri-to-Gregorian conversion
- [x] **Multi-Agent Orchestrator** — Dependency-based parallel task execution with topological sort
- [x] **Data Sync** — `amanclaw islamic sync` CLI + dashboard "Sync All Data" button + REST API endpoints

### Phase 3: Cloud & Community (Done)

- [x] **AmanClaw Cloud** — Multi-tenant managed hosting with K3s on Hostinger Malaysia, invite-only beta
- [x] **Cloud API** — Signup (with invite code), login (JWT), tenant management, engine status
- [x] **Web Chat Widget** — Browser-based chat at `/t/{slug}/chat` with dark/light mode, markdown rendering, auto-reconnect
- [x] **Tenant Isolation** — Per-tenant SQLite databases, config, plugins, and soul files. Lazy engine start/stop with 30-min idle timeout
- [x] **K8s Deployment** — Dockerfile, 7 K3s manifests (namespace, deployment, service, ingress with TLS, PVC, secrets, backup CronJob), setup/deploy/backup scripts
- [x] **Cloud CLI** — `amanclaw-cloud serve/invite/tenant` commands for operators

### Phase 4: Sovereign Infrastructure (Next)

- [ ] Experimental fine-tune of `amanclaw-islamic-7b` (scholar review required)
- [ ] Self-hosted model registry
- [ ] OIC cloud partnerships (Malaysia, Indonesia, Saudi Arabia)
- [ ] Government compliance certifications
- [ ] Formalize university partnerships for model validation
- [ ] Advanced multi-agent orchestration (parallel agents, complex workflows)

### Phase 5: Ecosystem

- [ ] Specialized bots (UstazBot, HalalBot, FinanceBot)
- [ ] Mobile app (companion to chat channels)
- [ ] Enterprise features (audit logs, SSO, RBAC)
- [ ] Open marketplace with revenue sharing
- [ ] Developer conference / community events

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

**Q: What is AmanClaw Cloud?**
Managed hosting for AmanClaw. Sign up with an invite code, get your own bot instance, connect chat channels via dashboard, and chat from your browser. Currently invite-only beta on Hostinger Malaysia. Self-hosting remains fully supported — cloud sells convenience, not capability.

**Q: Is my data stored?**
Self-hosted: conversations stored in a local SQLite database. Nothing leaves your server except LLM API calls. Cloud: data stored on Hostinger Malaysia VPS with per-tenant isolation. Each tenant gets their own SQLite databases.

**Q: How does auto-summarization work?**
When a user's message count exceeds 40, the engine asks the LLM to summarize the conversation, saves the summary, and prunes old messages (keeping the 10 most recent). The summary is included in future prompts as context.

---

## License

MIT License. See [LICENSE](LICENSE) for details.

---

## Acknowledgments

Built with care from Puncak Alam, Malaysia. Made possible by the Rust ecosystem and the communities behind teloxide, serenity, wasmtime, axum, tokio, and sqlx.

*Your AI, your rules. Malaysia boleh!*
