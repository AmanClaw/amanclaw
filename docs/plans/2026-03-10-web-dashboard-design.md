# Web Management Dashboard — Design

**Date:** 2026-03-10
**Status:** Approved

## Goal

Embedded web dashboard for managing AmanClaw, served directly from the Rust binary. No extra deployment — accessible at `http://host:8443/admin` from any device.

## Decisions

| Question | Answer |
| --- | --- |
| Hosting | Embedded in AmanClaw binary (Axum serves static files) |
| Frontend | Svelte 5 + Tailwind CSS |
| Pages | Full suite: Dashboard, Users, Skills, Channels, Communities, Content, Logs, Settings |
| Auth | Simple password (`ADMIN_PASSWORD` env var), JWT cookie |
| Mobile | Fully responsive |

## Architecture

```
Browser (any device)
    │
    │  http://pi-ip:8443/admin
    │
    ▼
┌─────────────────────────────────┐
│  AmanClaw Binary                │
│                                 │
│  Axum Server (:8443)            │
│  ├── /admin/*  → static files   │  ← Svelte SPA (embedded via include_dir!)
│  ├── /api/*    → REST endpoints │  ← existing amanclaw-api routes
│  ├── /ws       → WebSocket      │  ← live logs streaming
│  └── /metrics  → Prometheus     │
│                                 │
│  Engine, Channels, Skills...    │
└─────────────────────────────────┘
```

- Build time: Svelte app builds to `dist/`, embedded into Rust binary via `include_dir!` macro
- Runtime: Axum serves static files at `/admin/*`, falls back to `index.html` for SPA routing
- No extra deployment — dashboard ships with every AmanClaw build

## Pages

| Page | Features | API Endpoints |
| --- | --- | --- |
| Dashboard | Bot status, message count, active users, channel health, quick actions | `GET /api/status` |
| Users | User list, search/filter, approve/block, per-user stats, filter by platform | `GET /api/users`, `POST .../approve`, `POST .../block` |
| Skills | Skill list with toggles, health status, usage stats, API key config | `GET /api/skills`, `POST .../toggle`, `GET/PUT .../config` |
| Channels | Channel status (TG/WA/Discord/Slack), connection health, throughput | `GET /api/status` (extended) |
| Communities | Community CRUD, assign skills, zone/language config | `GET/POST/PUT/DELETE /api/communities` |
| Content | Edit doa collection, zakat rates, khutbah cache | `GET/PUT /api/content/*` |
| Logs | Live stream via WebSocket, filter by level/skill/channel, search | `/ws` WebSocket |
| Settings | LLM config, bot config, admin password change, rate limits | `GET/PUT /api/config` |

## Auth Flow

1. User opens `/admin` → redirected to login page
2. Enters password (set via `ADMIN_PASSWORD` env var)
3. Backend validates, returns JWT cookie (httponly, 24h expiry)
4. All `/api/*` requests include the cookie
5. If `ADMIN_PASSWORD` not set → dashboard disabled with message

Security:
- JWT secret auto-generated on first boot, saved to `data/.jwt_secret`
- Rate limit on login: 5 attempts per minute
- Cookie: httponly + SameSite=Strict
- API still supports Bearer token auth for programmatic access

## Frontend Structure

```
dashboard/
├── src/
│   ├── lib/
│   │   ├── components/
│   │   │   ├── Sidebar.svelte
│   │   │   ├── MobileNav.svelte
│   │   │   ├── StatusBadge.svelte
│   │   │   ├── UserCard.svelte
│   │   │   ├── SkillToggle.svelte
│   │   │   ├── LogEntry.svelte
│   │   │   └── StatCard.svelte
│   │   ├── pages/
│   │   │   ├── Dashboard.svelte
│   │   │   ├── Users.svelte
│   │   │   ├── Skills.svelte
│   │   │   ├── Channels.svelte
│   │   │   ├── Communities.svelte
│   │   │   ├── Content.svelte
│   │   │   ├── Logs.svelte
│   │   │   ├── Settings.svelte
│   │   │   └── Login.svelte
│   │   ├── stores/
│   │   │   ├── auth.ts
│   │   │   └── api.ts
│   │   └── app.css
│   ├── App.svelte
│   └── main.ts
├── package.json
├── svelte.config.js
├── tailwind.config.js
├── vite.config.ts
└── dist/
```

## Build Pipeline

1. `cd dashboard && npm run build` → outputs `dist/`
2. Rust binary uses `include_dir!("../dashboard/dist")` to embed at compile time
3. Docker multi-stage: frontend builds first, then Rust compiles with embedded assets

## Design Style

- Clean, minimal admin UI
- Dark/light mode via Tailwind `dark:` classes
- Mobile-first responsive with Tailwind breakpoints
- Sidebar nav on desktop, bottom tab bar on mobile
