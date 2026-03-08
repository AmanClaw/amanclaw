# Embedded Engine Desktop App Design

**Date:** 2026-03-08
**Status:** Approved

## Overview

Embed the amanclaw-core Engine directly into the Tauri 2 desktop app as a single binary. Users download one installer, configure via GUI, and run the bot — no terminal needed. The CLI remains for headless/server deployments.

## Architecture

The Engine runs in-process as a Tokio background task inside the Tauri app. The Svelte frontend communicates with it via Tauri IPC commands.

```
┌─────────────────────────────────────────────┐
│  Tauri Desktop App (single binary)          │
│                                             │
│  ┌──────────┐    ┌───────────────────────┐  │
│  │ Svelte   │◄──►│ Tauri IPC Commands    │  │
│  │ Frontend │    │ (start, stop, config) │  │
│  └──────────┘    └───────┬───────────────┘  │
│                          │                  │
│                 ┌────────▼────────┐         │
│                 │  Engine (tokio) │         │
│                 │  ├─ Pipeline    │         │
│                 │  ├─ Auth        │         │
│                 │  ├─ LLM Client  │         │
│                 │  ├─ Memory (DB) │         │
│                 │  ├─ Skills      │         │
│                 │  └─ Channels    │         │
│                 └─────────────────┘         │
│                                             │
│  Config + DB: OS app data directory         │
└─────────────────────────────────────────────┘
```

## Startup Flow

1. Tauri `setup()` hook checks for `config.yaml` in OS app data dir
2. If no config → show first-run wizard
3. If config exists → load config, start engine automatically
4. Engine runs in `Arc<Mutex<Option<Engine>>>` — `None` when stopped, `Some` when running

### First-Run Wizard (2 steps)

- **Step 1:** LLM config — base URL, model name, API key
- **Step 2:** Summary + "Start AmanClaw" button

Channels, zones, and plugins configured later in Settings. Minimal friction.

### Subsequent Launches

Load saved config → auto-start engine → show Dashboard.

## Data Storage

OS standard application data directories:

- macOS: `~/Library/Application Support/AmanClaw/`
- Windows: `%APPDATA%\AmanClaw\`
- Linux: `~/.config/amanclaw/`

```
<app-data-dir>/
├── config.yaml          # AppConfig (LLM, admin users, rate limit, plugins, skills)
├── secrets.env          # Channel tokens (TELEGRAM_BOT_TOKEN, DISCORD_BOT_TOKEN, etc.)
├── memory.db            # SQLite conversation + facts + summaries
└── plugins/             # WASM and script plugins
```

## IPC Commands

### New Commands

| Command | Purpose |
|---------|---------|
| `check_first_run` | Returns true if no config.yaml exists |
| `save_config` | Write config.yaml + secrets.env to app data dir |
| `start_engine` | Load config, init Engine, spawn run() in background |
| `stop_engine` | Graceful shutdown |
| `restart_engine` | Stop + start |
| `get_engine_status` | Running/stopped/error with uptime |
| `get_config` | Return current config for Settings page |

### Modified Commands

Existing commands (`get_communities`, `get_skills`, `get_users`, etc.) already switch between local and remote mode. Local mode will now access the Engine's Auth, SqlitePool, and PluginRegistry directly instead of returning empty stubs.

## Channel Support

All channels supported (Telegram, Discord, Slack, WhatsApp), enabled one-by-one via GUI toggles in Settings. Each channel has a toggle switch and its required token fields. Channels with empty tokens simply don't start.

## Connection Modes

- **Local mode** (default) — Engine runs in-process, all commands access it directly
- **Remote mode** — Connects to external CLI instance via REST management API (existing behavior, preserved)

## UI Changes

### Settings Page (expanded)

- **LLM section** — base URL, model, API key, max tokens, temperature
- **Channels section** — toggle + token fields for Telegram, Discord, Slack, WhatsApp
- **Engine section** — rate limit, plugin dir, skill timeout
- **Connection mode** — Local/Remote switch (existing)
- **Data section** — shows config/DB path, "Open folder" button, reset option

### Dashboard (enhanced)

- Start/Stop/Restart buttons
- Engine status indicator (running/stopped/error)
- Uptime, message count, active channels
- Last error if engine crashed

### System Tray

- Status indicator: green (running), red (stopped), yellow (starting)

### Unchanged Pages

Communities, Skills, Users, Content, Logs — already built, just need local mode to return real Engine data.

## Error Handling

- **Engine crash** — catch panic, show error on Dashboard with Restart button, log to Logs page
- **Invalid config** — validate before saving (LLM URL format, non-empty model), inline errors
- **Missing tokens** — channels with empty tokens don't start, no error
- **DB locked** — show "Database in use" error if another instance is running, suggest Remote mode
- **Port conflicts** — management API port taken → skip API startup (non-fatal)

## Dependencies Added to Tauri Cargo.toml

```toml
amanclaw-core = { path = "../../rust/crates/amanclaw-core" }
amanclaw-traits = { path = "../../rust/crates/amanclaw-traits" }
amanclaw-api = { path = "../../rust/crates/amanclaw-api" }
serde_yaml = "0.9"
dotenvy = "0.15"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
```

No circular dependency issues — all dependencies flow downward.

## Architecture Decisions

1. **In-process engine, not sidecar** — single binary, simpler lifecycle, shared memory
2. **OS app data dirs** — standard convention, survives updates, no permission issues
3. **2-step wizard** — LLM config is the only blocker, everything else optional
4. **Auto-start on subsequent launches** — zero-click experience after setup
5. **Keep Remote mode** — free to maintain, useful for VPS/Pi management
6. **Secrets in separate file** — `secrets.env` not in `config.yaml`, cleaner separation
