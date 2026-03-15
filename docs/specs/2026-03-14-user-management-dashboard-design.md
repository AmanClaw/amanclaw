# User Management Dashboard — Design Spec

## Problem

1. Auth state (approve/block/pending) is stored in-memory only — lost on core restart
2. No visibility into who's using the bot across channels
3. No admin UI — management is API-only

## Solution

Web dashboard (React + Tailwind + Vite) backed by new API endpoints, with SQLite persistence for user state. Tauri desktop wrapper planned for future phase.

## Architecture

```
React SPA (web/)
  │ REST API + Bearer token auth
  ▼
Axum API (amanclaw-api)
  │ new user mgmt routes
  ▼
SQLite (amanclaw-memory)
  │ new `users` table
  ▼
Auth (amanclaw-security)
  └─ SQLite-backed with write-through cache
```

## 1. SQLite User Persistence

New `users` table:

```sql
CREATE TABLE IF NOT EXISTS users (
    user_id TEXT NOT NULL,
    platform TEXT NOT NULL,
    state TEXT NOT NULL DEFAULT 'pending',
    username TEXT,
    first_name TEXT,
    first_seen DATETIME DEFAULT CURRENT_TIMESTAMP,
    last_seen DATETIME DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (user_id, platform)
);
```

- `Auth` loads from SQLite on startup, writes back on state changes
- `register_user` / `approve_user` / `block_user` persist to SQLite
- `first_seen` / `last_seen` tracked automatically
- `username` / `first_name` captured from `IncomingMessage` on first contact

## 2. API Endpoints

All routes require `Authorization: Bearer <API_TOKEN>`.

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/api/users` | List users, filterable: `?platform=&status=&search=` |
| `GET` | `/api/users/:platform/:user_id` | User detail: state, facts, message count, last seen |
| `GET` | `/api/users/:platform/:user_id/history` | Conversation history: `?limit=&offset=` |
| `PUT` | `/api/users/:platform/:user_id/approve` | Approve user |
| `PUT` | `/api/users/:platform/:user_id/block` | Block user |
| `PUT` | `/api/users/:platform/:user_id/unblock` | Reset to pending |
| `GET` | `/api/stats` | Dashboard stats: totals, per-platform, pending count |

User detail response:

```json
{
  "user_id": "123456789",
  "platform": "telegram",
  "state": "approved",
  "username": "aman",
  "first_name": "Aman",
  "first_seen": "2026-03-01T10:00:00Z",
  "last_seen": "2026-03-14T08:30:00Z",
  "message_count": 142,
  "facts": { "name": "Aman", "city": "KL", "zone": "WLY01" }
}
```

## 3. React Dashboard

**Tech:** React + Tailwind CSS + Vite, TypeScript.

**Pages:**

- **Login** — API token input, stored in localStorage
- **Dashboard** — Stats cards (total, pending, approved, blocked, per-platform)
- **User list** — Table with platform icon, username, status badge, last seen. Filters: platform, status, search. Inline approve/block actions.
- **User detail** — Profile card, facts table, paginated conversation history

**Layout:**

```
┌──────────────────────────────────────┐
│  Sidebar          │  Main Content    │
│  ─────────────    │                  │
│  Dashboard        │  [Page Content]  │
│  Users            │                  │
│                   │                  │
│  ── bottom ──     │                  │
│  Logout           │                  │
└──────────────────────────────────────┘
```

**Project structure:**

```
web/
├── src/
│   ├── components/    # Sidebar, StatsCard, UserTable, UserDetail
│   ├── pages/         # Login, Dashboard, Users, UserDetail
│   ├── api/           # Fetch wrapper with token
│   ├── App.tsx
│   └── main.tsx
├── index.html
├── vite.config.ts
├── tailwind.config.js
└── package.json
```

**Dev:** `npm run dev` with Vite proxy to Axum.
**Prod:** `npm run build` → static files served by Axum at `/dashboard`.

## 4. Auth Middleware Update

Message flow changes:

```
Message arrives
  → AuthMiddleware (reads from SQLite-backed Auth)
    → New user: INSERT into `users` table, state=pending, capture username/first_name
    → Known user: UPDATE `last_seen`
  → PersistMiddleware (unchanged)
```

`Auth` struct changes:
- Constructor takes `SqlitePool`
- Loads admin list from config + registered users from SQLite on startup
- In-memory `HashMap` as write-through cache
- Reads from memory, writes to memory + SQLite

## 5. Tauri Desktop (future)

Not built in this phase. Design accommodates it:
- `web/` is standalone SPA — Tauri wraps directly
- API URL configurable
- Add `src-tauri/` when ready
- No frontend code changes needed

## Files Changed

| File | Change |
|------|--------|
| `amanclaw-memory/src/schema.rs` | Add `users` table to `INIT_SQL` |
| `amanclaw-memory/src/sqlite.rs` | Add user CRUD methods |
| `amanclaw-security/src/auth.rs` | Rewrite: SQLite-backed with pool |
| `amanclaw-core/src/middleware/auth.rs` | Update to capture username/first_name, update last_seen |
| `amanclaw-core/src/lib.rs` | Pass pool to Auth constructor |
| `amanclaw-api/src/routes/users.rs` | Rewrite: 7 new endpoints |
| `amanclaw-api/src/routes/mod.rs` | Add stats route |
| `amanclaw-api/src/state.rs` | Add memory pool to ApiState |
| `web/` (new) | Full React dashboard |
