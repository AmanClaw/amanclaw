# AmanClaw Desktop Admin App Design

**Date:** 2026-03-08
**Status:** Approved

## Overview

Cross-platform desktop app (macOS, Windows, Linux) for managing AmanClaw bot instances. Built with Tauri 2 (Rust + Svelte + Tailwind). Supports local mode (embedded bot engine) and remote mode (REST API to server/cloud). Runs in system tray with native notifications.

## Goals

1. One-click bot management for non-technical admins (masjid committees, usrah groups)
2. Build REST management API that serves as foundation for Phase 2 web dashboard and future clients
3. Cross-platform desktop app with system tray, solat notifications, Apple-style clean minimal UI

## Architecture

```
┌─────────────────────────────────────────────┐
│              AmanClaw Desktop               │
│  ┌───────────────────────────────────────┐  │
│  │         Svelte + Tailwind UI          │  │
│  │  (Dashboard, Communities, Skills,     │  │
│  │   Users, Content, Logs, Settings)     │  │
│  └──────────────┬────────────────────────┘  │
│                 │ Tauri IPC                  │
│  ┌──────────────▼────────────────────────┐  │
│  │          Rust Backend (Tauri)          │  │
│  │                                       │  │
│  │  ┌─────────────┐  ┌───────────────┐   │  │
│  │  │ Local Mode  │  │ Remote Mode   │   │  │
│  │  │ Embeds      │  │ REST client   │   │  │
│  │  │ amanclaw-   │  │ connects to   │   │  │
│  │  │ core engine │  │ remote bot    │   │  │
│  │  └─────────────┘  └───────────────┘   │  │
│  └───────────────────────────────────────┘  │
│                                             │
│  System Tray: status icon + notifications   │
└─────────────────────────────────────────────┘
```

Two modes, one UI:
- **Local mode:** Tauri backend imports amanclaw-core, runs bot engine in-process. Reads/writes config.yaml, .env, SQLite directly.
- **Remote mode:** Tauri backend calls REST API on a remote AmanClaw instance. For cloud/server management.

---

## UI Layout & Pages

App shell: Sidebar navigation (left) + content area (right). Apple-style with translucent sidebar, SF-style typography, subtle shadows.

### Pages

| Page | What it shows |
|------|--------------|
| **Dashboard** | Stats overview, recent activity, bot status, quick actions |
| **Communities** | List of groups, add/edit community, zone/language/skills per community |
| **Skills** | Global skill toggle, API key config, skill health status, usage stats |
| **Users** | User list, approve/block, role management, per-user stats |
| **Content** | Edit doa collection, update zakat rates, manage khutbah cache |
| **Logs** | Live log stream, filter by level/skill/community, search |
| **Settings** | LLM config, bot token, mode switch (local/remote), theme, notifications |

---

## System Tray & Notifications

### Tray icon states

- Green: bot running, all healthy
- Yellow: bot running, warnings (API rate limit, skill error)
- Red: bot stopped or critical error
- Grey: disconnected (remote mode, server unreachable)

### Right-click menu

```
AmanClaw
─────────────
Bot Running
  12 communities · 148 users
─────────────
  Open Dashboard
  Pause Bot
  View Logs
─────────────
  Solat: Asar 3:45 PM (SGR01)
─────────────
  Quit AmanClaw
```

### Native notifications

- Solat time reminders (admin's own zone)
- New user pending approval
- Skill errors (JAKIM API down, etc.)
- Community onboarded successfully

---

## REST Management API

New crate `amanclaw-api` using Axum, runs alongside bot engine.

### Endpoints

| Group | Endpoint | Method | Description |
|-------|----------|--------|-------------|
| Bot | `/api/status` | GET | Bot status, uptime, stats |
| Bot | `/api/start` | POST | Start bot engine |
| Bot | `/api/stop` | POST | Stop bot engine |
| Bot | `/api/logs` | GET | Stream logs (SSE) |
| Communities | `/api/communities` | GET | List all communities |
| Communities | `/api/communities` | POST | Create community |
| Communities | `/api/communities/:id` | GET/PUT/DELETE | CRUD |
| Communities | `/api/communities/:id/skills` | PUT | Enable/disable skills |
| Communities | `/api/communities/:id/notifications` | PUT | Configure notifications |
| Skills | `/api/skills` | GET | List all skills + status |
| Skills | `/api/skills/:name/toggle` | POST | Enable/disable globally |
| Skills | `/api/skills/config` | GET/PUT | API keys, settings |
| Users | `/api/users` | GET | List users |
| Users | `/api/users/:id/approve` | POST | Approve user |
| Users | `/api/users/:id/block` | POST | Block user |
| Content | `/api/content/doa` | GET/PUT | Manage doa collection |
| Content | `/api/content/zakat-rates` | GET/PUT | Update zakat rates |
| Config | `/api/config` | GET/PUT | LLM settings, bot config |

Auth: Bearer token. Axum shared tokio runtime with bot engine.

---

## Security

### Authentication

| Mode | Auth Method | Details |
|------|------------|---------|
| Local | Auto-generated token | 256-bit random token saved to ~/.amanclaw/admin.key. Tauri reads automatically. No login screen. |
| Remote | Login (token) | Admin enters server URL + API token. Stored in OS keychain (macOS Keychain, Windows Credential Manager, Linux Secret Service). |
| Cloud (future) | OAuth / JWT | Proper auth with session management. |

### API security

- All endpoints require `Authorization: Bearer <token>`
- Rate limiting: 60 req/min default
- Local mode: API binds to 127.0.0.1 only (not exposed to network)
- Remote mode: requires explicit `api_bind: 0.0.0.0` in config + token
- HTTPS enforced for remote connections

### Data security

- Sensitive config encrypted at rest using OS keychain
- API keys masked in responses (e.g. `sk-****qsK0`)
- SQLite access scoped — no raw SQL via API
- Audit log: all admin actions logged with timestamp and admin ID

### Desktop app security (Tauri)

- CSP restricts frontend to Tauri IPC only
- No dynamic code execution, no remote script loading
- IPC allowlist — frontend can only call registered Rust commands
- Auto-update with Ed25519 signature verification

---

## Project Structure

```
amanclaw/
├── rust/
│   ├── crates/
│   │   ├── amanclaw-api/          # NEW: REST management API (axum)
│   │   └── ... (existing crates)
│   └── plugins/
├── desktop/                        # NEW: Tauri app
│   ├── src-tauri/
│   │   ├── Cargo.toml             # Depends on amanclaw-core, amanclaw-api
│   │   ├── src/
│   │   │   ├── main.rs            # Tauri entry point
│   │   │   ├── commands.rs        # IPC commands (Svelte <-> Rust)
│   │   │   ├── tray.rs            # System tray setup
│   │   │   ├── notifications.rs   # Native notification manager
│   │   │   └── state.rs           # App state (local/remote mode)
│   │   ├── tauri.conf.json
│   │   └── icons/
│   ├── src/                        # Svelte frontend
│   │   ├── lib/
│   │   │   ├── components/        # Reusable UI components
│   │   │   ├── pages/             # Dashboard, Communities, Skills, etc.
│   │   │   ├── stores/            # Svelte stores (state management)
│   │   │   └── api.ts             # API client (local IPC or remote REST)
│   │   ├── app.html
│   │   ├── app.css                # Tailwind base
│   │   └── routes/                # SvelteKit routes
│   ├── package.json
│   ├── svelte.config.js
│   ├── tailwind.config.js
│   └── vite.config.ts
└── docs/plans/
```

## Build Outputs

- AmanClaw.dmg (macOS ~15MB)
- AmanClaw.msi (Windows ~15MB)
- AmanClaw.AppImage (Linux ~15MB)

Auto-update via Tauri updater checking GitHub Releases.

---

## Tech Stack

| Component | Technology |
|-----------|-----------|
| Desktop framework | Tauri 2 |
| Backend language | Rust |
| Frontend framework | Svelte 5 |
| CSS | Tailwind CSS 4 |
| REST API | Axum |
| Database | SQLite (shared with bot) |
| System tray | Tauri tray plugin |
| Notifications | Tauri notification plugin |
| Credential storage | OS keychain via keyring crate |
| Auto-update | Tauri updater (Ed25519) |

## Architecture Decisions

1. **Tauri over Electron** — shares Rust ecosystem with AmanClaw, tiny binary, secure by default.
2. **Svelte over React** — smallest bundle, fastest rendering, simplest syntax for admin CRUD. Reusable for Phase 2 web dashboard.
3. **Hybrid launcher (local + remote)** — one-click for local users, remote management for cloud. REST API built once, used by all future clients.
4. **Axum for REST API** — already used by amanclaw-mcp, shares tokio runtime with bot engine.
5. **OS keychain for secrets** — native credential storage, no plaintext config files for sensitive data.
6. **System tray** — bot runs in background, solat notifications, minimal desktop footprint.
