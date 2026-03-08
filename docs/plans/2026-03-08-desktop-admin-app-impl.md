# AmanClaw Desktop Admin App Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a cross-platform desktop admin app (Tauri 2 + Svelte + Tailwind) with REST management API for managing AmanClaw bot instances.

**Architecture:** New `amanclaw-api` crate provides REST endpoints via Axum. Tauri desktop app embeds amanclaw-core (local mode) or calls REST API (remote mode). Svelte frontend with Apple-style clean minimal UI. System tray with native notifications.

**Tech Stack:** Rust (Tauri 2, Axum, tokio), Svelte 5, Tailwind CSS 4, SQLite (shared with bot), OS keychain (keyring crate).

---

## Task 1: REST API Crate — Scaffold and Bot Status

**Files:**
- Create: `rust/crates/amanclaw-api/Cargo.toml`
- Create: `rust/crates/amanclaw-api/src/lib.rs`
- Create: `rust/crates/amanclaw-api/src/state.rs`
- Create: `rust/crates/amanclaw-api/src/routes/mod.rs`
- Create: `rust/crates/amanclaw-api/src/routes/bot.rs`
- Create: `rust/crates/amanclaw-api/src/auth.rs`
- Modify: `rust/Cargo.toml` (add workspace member)

**Step 1: Create Cargo.toml**

```toml
# rust/crates/amanclaw-api/Cargo.toml
[package]
name = "amanclaw-api"
version = "0.1.0"
edition = "2024"

[dependencies]
amanclaw-traits = { path = "../amanclaw-traits" }
amanclaw-memory = { path = "../amanclaw-memory" }
amanclaw-core = { path = "../amanclaw-core" }
axum = { version = "0.8", features = ["json"] }
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tower-http = { version = "0.6", features = ["cors", "trace"] }
tracing = "0.1"
rand = "0.8"
chrono = "0.4"

[dev-dependencies]
tokio = { version = "1", features = ["full", "test-util"] }
```

**Step 2: Create state.rs — shared API state**

```rust
// rust/crates/amanclaw-api/src/state.rs
use amanclaw_core::registry::PluginRegistry;
use amanclaw_memory::sqlite::SqliteMemory;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct ApiState {
    pub registry: Arc<PluginRegistry>,
    pub memory: Arc<SqliteMemory>,
    pub api_token: String,
    pub bot_status: Arc<RwLock<BotStatus>>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct BotStatus {
    pub running: bool,
    pub started_at: Option<String>,
    pub uptime_seconds: u64,
    pub communities_count: u64,
    pub users_count: u64,
    pub skills_count: usize,
}

impl BotStatus {
    pub fn new() -> Self {
        Self {
            running: false,
            started_at: None,
            uptime_seconds: 0,
            communities_count: 0,
            users_count: 0,
            skills_count: 0,
        }
    }
}
```

**Step 3: Create auth.rs — Bearer token middleware**

```rust
// rust/crates/amanclaw-api/src/auth.rs
use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::Response,
};

pub async fn require_auth(
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let token = request
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));

    let expected = request
        .extensions()
        .get::<String>()
        .map(|s| s.as_str());

    match (token, expected) {
        (Some(t), Some(e)) if t == e => Ok(next.run(request).await),
        _ => Err(StatusCode::UNAUTHORIZED),
    }
}
```

**Step 4: Create routes/bot.rs — bot status endpoint**

```rust
// rust/crates/amanclaw-api/src/routes/bot.rs
use crate::state::ApiState;
use axum::{extract::State, Json};

pub async fn get_status(
    State(state): State<ApiState>,
) -> Json<serde_json::Value> {
    let status = state.bot_status.read().await;
    Json(serde_json::json!({
        "running": status.running,
        "started_at": status.started_at,
        "uptime_seconds": status.uptime_seconds,
        "communities_count": status.communities_count,
        "users_count": status.users_count,
        "skills_count": status.skills_count,
    }))
}
```

**Step 5: Create routes/mod.rs**

```rust
// rust/crates/amanclaw-api/src/routes/mod.rs
pub mod bot;
```

**Step 6: Create lib.rs — router assembly**

```rust
// rust/crates/amanclaw-api/src/lib.rs
pub mod auth;
pub mod routes;
pub mod state;

use axum::{routing::get, Router};
use state::ApiState;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

pub fn api_router(state: ApiState) -> Router {
    Router::new()
        .route("/api/status", get(routes::bot::get_status))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

pub async fn run_api_server(state: ApiState, port: u16) -> anyhow::Result<()> {
    let app = api_router(state);
    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{}", port)).await?;
    tracing::info!("Management API listening on http://127.0.0.1:{}", port);
    axum::serve(listener, app).await?;
    Ok(())
}
```

**Step 7: Add to workspace**

Add `"crates/amanclaw-api"` to `rust/Cargo.toml` workspace members.

**Step 8: Build and verify**

Run: `cd rust && cargo build -p amanclaw-api`

**Step 9: Commit**

```bash
git commit -m "feat: add amanclaw-api crate with bot status endpoint"
```

---

## Task 2: REST API — Community & Skills Endpoints

**Files:**
- Create: `rust/crates/amanclaw-api/src/routes/communities.rs`
- Create: `rust/crates/amanclaw-api/src/routes/skills.rs`
- Modify: `rust/crates/amanclaw-api/src/routes/mod.rs`
- Modify: `rust/crates/amanclaw-api/src/lib.rs`

**Step 1: Create communities.rs**

```rust
// rust/crates/amanclaw-api/src/routes/communities.rs
use crate::state::ApiState;
use axum::{extract::{Path, State}, http::StatusCode, Json};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct CreateCommunity {
    pub name: String,
    pub zone: String,
    pub language: String,
    pub platform: String,
    pub platform_group_id: String,
}

#[derive(Deserialize)]
pub struct UpdateCommunity {
    pub name: Option<String>,
    pub zone: Option<String>,
    pub language: Option<String>,
}

#[derive(Deserialize)]
pub struct UpdateSkills {
    pub enabled_skills: Vec<String>,
}

pub async fn list_communities(
    State(state): State<ApiState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let communities = state.memory.get_all_communities().await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::json!({ "communities": communities })))
}

pub async fn get_community(
    State(state): State<ApiState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let community = state.memory.get_community(&id).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    match community {
        Some(c) => Ok(Json(serde_json::json!(c))),
        None => Err(StatusCode::NOT_FOUND),
    }
}

pub async fn create_community(
    State(state): State<ApiState>,
    Json(body): Json<CreateCommunity>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let community = state.memory
        .create_community(&body.name, &body.zone, &body.language, &body.platform, &body.platform_group_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::json!(community)))
}

pub async fn update_community_skills(
    State(state): State<ApiState>,
    Path(id): Path<String>,
    Json(body): Json<UpdateSkills>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    state.memory.update_community_skills(&id, &body.enabled_skills).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::json!({ "ok": true })))
}
```

**Step 2: Create skills.rs**

```rust
// rust/crates/amanclaw-api/src/routes/skills.rs
use crate::state::ApiState;
use axum::{extract::State, Json};

pub async fn list_skills(
    State(state): State<ApiState>,
) -> Json<serde_json::Value> {
    let tools = state.registry.get_tool_definitions();
    let skills: Vec<serde_json::Value> = tools.iter().map(|t| {
        serde_json::json!({
            "name": t.name,
            "description": t.description,
            "parameters": t.parameters_schema,
        })
    }).collect();
    Json(serde_json::json!({ "skills": skills, "count": skills.len() }))
}
```

**Step 3: Update routes/mod.rs**

```rust
pub mod bot;
pub mod communities;
pub mod skills;
```

**Step 4: Update lib.rs — add routes**

Add to `api_router`:
```rust
use axum::routing::{get, post, put, delete};

Router::new()
    .route("/api/status", get(routes::bot::get_status))
    .route("/api/communities", get(routes::communities::list_communities))
    .route("/api/communities", post(routes::communities::create_community))
    .route("/api/communities/{id}", get(routes::communities::get_community))
    .route("/api/communities/{id}/skills", put(routes::communities::update_community_skills))
    .route("/api/skills", get(routes::skills::list_skills))
    // ...
```

**Step 5: Build, commit**

```bash
git commit -m "feat: add community and skills REST API endpoints"
```

---

## Task 3: REST API — Users Endpoint

**Files:**
- Create: `rust/crates/amanclaw-api/src/routes/users.rs`
- Modify: `rust/crates/amanclaw-api/src/routes/mod.rs`
- Modify: `rust/crates/amanclaw-api/src/lib.rs`
- Modify: `rust/crates/amanclaw-api/src/state.rs`

**Step 1: Add auth to ApiState**

Add to `state.rs`:
```rust
use amanclaw_security::auth::Auth;
use std::sync::Mutex;

pub struct ApiState {
    // ... existing fields
    pub auth: Arc<Mutex<Auth>>,
}
```

**Step 2: Create users.rs**

```rust
// rust/crates/amanclaw-api/src/routes/users.rs
use crate::state::ApiState;
use axum::{extract::{Path, State}, http::StatusCode, Json};

pub async fn list_users(
    State(state): State<ApiState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let auth = state.auth.lock().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let users: Vec<serde_json::Value> = auth.list_users().iter().map(|(id, platform, user_state)| {
        serde_json::json!({
            "user_id": id,
            "platform": platform,
            "state": format!("{:?}", user_state),
        })
    }).collect();
    Ok(Json(serde_json::json!({ "users": users, "count": users.len() })))
}

pub async fn approve_user(
    State(state): State<ApiState>,
    Path((platform, user_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let mut auth = state.auth.lock().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    auth.approve_user(&user_id, &platform);
    Ok(Json(serde_json::json!({ "ok": true, "user_id": user_id, "state": "Approved" })))
}

pub async fn block_user(
    State(state): State<ApiState>,
    Path((platform, user_id)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let mut auth = state.auth.lock().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    auth.block_user(&user_id, &platform);
    Ok(Json(serde_json::json!({ "ok": true, "user_id": user_id, "state": "Blocked" })))
}
```

**Step 3: Add routes, build, commit**

```bash
git commit -m "feat: add users REST API endpoints with approve/block"
```

---

## Task 4: Wire REST API into Engine

**Files:**
- Modify: `rust/crates/amanclaw-core/src/lib.rs`
- Modify: `rust/crates/amanclaw-core/Cargo.toml`

**Step 1: Add amanclaw-api dependency to amanclaw-core**

```toml
amanclaw-api = { path = "../amanclaw-api" }
```

**Step 2: Start API server in Engine::new()**

Add after MCP HTTP server setup in `lib.rs`:

```rust
// Start management API if configured
if let Ok(port_str) = std::env::var("API_PORT") {
    if let Ok(port) = port_str.parse::<u16>() {
        let api_token = std::env::var("API_TOKEN")
            .unwrap_or_else(|_| generate_random_token());
        let api_state = amanclaw_api::state::ApiState {
            registry: registry.clone(),
            memory: Arc::new(memory.clone()),
            api_token,
            bot_status: Arc::new(tokio::sync::RwLock::new(
                amanclaw_api::state::BotStatus::new()
            )),
            auth: auth_arc.clone(),
        };
        tokio::spawn(async move {
            if let Err(e) = amanclaw_api::run_api_server(api_state, port).await {
                tracing::error!("Management API error: {}", e);
            }
        });
        tracing::info!("Management API started on port {}", port);
    }
}
```

**Step 3: Build full workspace, commit**

```bash
git commit -m "feat: wire REST management API into engine startup"
```

---

## Task 5: Scaffold Tauri Desktop App

**Files:**
- Create: `desktop/` directory (entire Tauri + Svelte project)

**Step 1: Install prerequisites**

```bash
# Install Tauri CLI
cargo install tauri-cli --version "^2"

# Create Tauri + Svelte project
cd /path/to/amanclaw
cargo tauri init --app-name "AmanClaw" --window-title "AmanClaw" --dist-dir "../build" --dev-url "http://localhost:5173" --before-dev-command "npm run dev" --before-build-command "npm run build" --ci
```

**Step 2: Initialize Svelte + Tailwind frontend**

```bash
cd desktop
npm create svelte@latest . -- --template skeleton --types typescript
npm install
npm install -D tailwindcss @tailwindcss/vite
```

**Step 3: Configure tailwind in vite.config.ts**

```typescript
// desktop/vite.config.ts
import { sveltekit } from '@sveltejs/kit';
import tailwindcss from '@tailwindcss/vite';
import { defineConfig } from 'vite';

export default defineConfig({
    plugins: [tailwindcss(), sveltekit()],
    clearScreen: false,
    server: {
        port: 5173,
        strictPort: true,
    },
});
```

**Step 4: Add Tailwind to app.css**

```css
/* desktop/src/app.css */
@import "tailwindcss";
```

**Step 5: Configure Tauri Cargo.toml**

```toml
# desktop/src-tauri/Cargo.toml
[package]
name = "amanclaw-desktop"
version = "0.1.0"
edition = "2024"

[dependencies]
tauri = { version = "2", features = ["tray-icon", "image-png"] }
tauri-plugin-notification = "2"
tauri-plugin-shell = "2"
amanclaw-core = { path = "../../rust/crates/amanclaw-core" }
amanclaw-api = { path = "../../rust/crates/amanclaw-api" }
amanclaw-traits = { path = "../../rust/crates/amanclaw-traits" }
amanclaw-memory = { path = "../../rust/crates/amanclaw-memory" }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["full"] }
reqwest = { version = "0.12", features = ["json"] }
tracing = "0.1"

[build-dependencies]
tauri-build = { version = "2", features = [] }
```

**Step 6: Create Tauri main.rs**

```rust
// desktop/src-tauri/src/main.rs
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod state;
mod tray;

use state::AppState;
use std::sync::Arc;
use tokio::sync::RwLock;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_shell::init())
        .manage(Arc::new(RwLock::new(AppState::new())))
        .setup(|app| {
            tray::setup_tray(app)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_status,
            commands::get_communities,
            commands::get_skills,
            commands::get_users,
            commands::get_mode,
            commands::set_mode,
        ])
        .run(tauri::generate_context!())
        .expect("error running AmanClaw Desktop");
}
```

**Step 7: Create state.rs**

```rust
// desktop/src-tauri/src/state.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AppMode {
    Local,
    Remote { url: String, token: String },
}

#[derive(Debug)]
pub struct AppState {
    pub mode: AppMode,
    pub bot_running: bool,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            mode: AppMode::Local,
            bot_running: false,
        }
    }
}
```

**Step 8: Create commands.rs — Tauri IPC commands**

```rust
// desktop/src-tauri/src/commands.rs
use crate::state::{AppMode, AppState};
use std::sync::Arc;
use tauri::State;
use tokio::sync::RwLock;

#[tauri::command]
pub async fn get_status(
    state: State<'_, Arc<RwLock<AppState>>>,
) -> Result<serde_json::Value, String> {
    let app = state.read().await;
    Ok(serde_json::json!({
        "bot_running": app.bot_running,
        "mode": format!("{:?}", app.mode),
    }))
}

#[tauri::command]
pub async fn get_mode(
    state: State<'_, Arc<RwLock<AppState>>>,
) -> Result<String, String> {
    let app = state.read().await;
    Ok(match &app.mode {
        AppMode::Local => "local".to_string(),
        AppMode::Remote { url, .. } => format!("remote:{}", url),
    })
}

#[tauri::command]
pub async fn set_mode(
    state: State<'_, Arc<RwLock<AppState>>>,
    mode: String,
    url: Option<String>,
    token: Option<String>,
) -> Result<(), String> {
    let mut app = state.write().await;
    app.mode = match mode.as_str() {
        "local" => AppMode::Local,
        "remote" => AppMode::Remote {
            url: url.unwrap_or_default(),
            token: token.unwrap_or_default(),
        },
        _ => return Err("Invalid mode".into()),
    };
    Ok(())
}

#[tauri::command]
pub async fn get_communities() -> Result<serde_json::Value, String> {
    // TODO: wire to local engine or remote API
    Ok(serde_json::json!({ "communities": [] }))
}

#[tauri::command]
pub async fn get_skills() -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({ "skills": [] }))
}

#[tauri::command]
pub async fn get_users() -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({ "users": [] }))
}
```

**Step 9: Create tray.rs — system tray**

```rust
// desktop/src-tauri/src/tray.rs
use tauri::{
    image::Image,
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    App,
};

pub fn setup_tray(app: &App) -> Result<(), Box<dyn std::error::Error>> {
    let quit = MenuItem::with_id(app, "quit", "Quit AmanClaw", true, None::<&str>)?;
    let open = MenuItem::with_id(app, "open", "Open Dashboard", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &quit])?;

    TrayIconBuilder::new()
        .menu(&menu)
        .tooltip("AmanClaw - Bot Running")
        .on_menu_event(|app, event| {
            match event.id().as_ref() {
                "quit" => app.exit(0),
                "open" => {
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
                _ => {}
            }
        })
        .build(app)?;

    Ok(())
}
```

**Step 10: Build and verify**

```bash
cd desktop && npm install && cd src-tauri && cargo build
```

**Step 11: Commit**

```bash
git commit -m "feat: scaffold Tauri desktop app with Svelte + Tailwind"
```

---

## Task 6: Svelte Frontend — Layout Shell & Dashboard

**Files:**
- Create: `desktop/src/lib/components/Sidebar.svelte`
- Create: `desktop/src/lib/components/StatusBadge.svelte`
- Create: `desktop/src/lib/stores/app.ts`
- Create: `desktop/src/lib/api.ts`
- Create: `desktop/src/routes/+layout.svelte`
- Create: `desktop/src/routes/+page.svelte`

**Step 1: Create API client**

```typescript
// desktop/src/lib/api.ts
import { invoke } from '@tauri-apps/api/core';

export const api = {
    getStatus: () => invoke('get_status'),
    getCommunities: () => invoke('get_communities'),
    getSkills: () => invoke('get_skills'),
    getUsers: () => invoke('get_users'),
    getMode: () => invoke('get_mode'),
    setMode: (mode: string, url?: string, token?: string) =>
        invoke('set_mode', { mode, url, token }),
};
```

**Step 2: Create app store**

```typescript
// desktop/src/lib/stores/app.ts
import { writable } from 'svelte/store';

export const botStatus = writable({
    running: false,
    mode: 'local',
    communities: 0,
    users: 0,
    skills: 0,
});

export const currentPage = writable('dashboard');
```

**Step 3: Create Sidebar component**

```svelte
<!-- desktop/src/lib/components/Sidebar.svelte -->
<script lang="ts">
    import { currentPage } from '$lib/stores/app';

    const pages = [
        { id: 'dashboard', label: 'Dashboard', icon: '⊞' },
        { id: 'communities', label: 'Communities', icon: '⊡' },
        { id: 'skills', label: 'Skills', icon: '⚡' },
        { id: 'users', label: 'Users', icon: '⊙' },
        { id: 'content', label: 'Content', icon: '☰' },
        { id: 'logs', label: 'Logs', icon: '▤' },
    ];

    const bottomPages = [
        { id: 'settings', label: 'Settings', icon: '⚙' },
    ];
</script>

<aside class="w-56 h-screen bg-gray-50/80 backdrop-blur-xl border-r border-gray-200 flex flex-col justify-between p-3">
    <div>
        <div class="px-3 py-4 mb-2">
            <h1 class="text-sm font-semibold text-gray-900 tracking-tight">AmanClaw</h1>
        </div>
        <nav class="space-y-0.5">
            {#each pages as page}
                <button
                    class="w-full flex items-center gap-2.5 px-3 py-1.5 rounded-md text-[13px] transition-colors
                        {$currentPage === page.id
                            ? 'bg-gray-200/80 text-gray-900 font-medium'
                            : 'text-gray-600 hover:bg-gray-100 hover:text-gray-900'}"
                    on:click={() => currentPage.set(page.id)}
                >
                    <span class="text-base leading-none">{page.icon}</span>
                    {page.label}
                </button>
            {/each}
        </nav>
    </div>

    <div>
        <div class="border-t border-gray-200 pt-2 mb-2">
            {#each bottomPages as page}
                <button
                    class="w-full flex items-center gap-2.5 px-3 py-1.5 rounded-md text-[13px] text-gray-600 hover:bg-gray-100 hover:text-gray-900 transition-colors"
                    on:click={() => currentPage.set(page.id)}
                >
                    <span class="text-base leading-none">{page.icon}</span>
                    {page.label}
                </button>
            {/each}
        </div>
        <div class="mx-2 p-2.5 bg-white rounded-lg border border-gray-200 shadow-sm">
            <div class="flex items-center gap-2">
                <span class="w-2 h-2 rounded-full bg-green-500"></span>
                <span class="text-[11px] font-medium text-gray-700">Bot Running</span>
            </div>
        </div>
    </div>
</aside>
```

**Step 4: Create layout**

```svelte
<!-- desktop/src/routes/+layout.svelte -->
<script lang="ts">
    import '../app.css';
    import Sidebar from '$lib/components/Sidebar.svelte';
</script>

<div class="flex h-screen bg-white select-none">
    <Sidebar />
    <main class="flex-1 overflow-y-auto">
        <slot />
    </main>
</div>
```

**Step 5: Create dashboard page**

```svelte
<!-- desktop/src/routes/+page.svelte -->
<script lang="ts">
    import { onMount } from 'svelte';
    import { api } from '$lib/api';
    import { botStatus } from '$lib/stores/app';

    onMount(async () => {
        const status = await api.getStatus();
        botStatus.set(status as any);
    });
</script>

<div class="p-8 max-w-4xl">
    <div class="mb-8">
        <h2 class="text-xl font-semibold text-gray-900 tracking-tight">Dashboard</h2>
        <p class="text-sm text-gray-500 mt-1">Overview of your AmanClaw instance</p>
    </div>

    <!-- Stats cards -->
    <div class="grid grid-cols-3 gap-4 mb-8">
        <div class="bg-gray-50 rounded-xl border border-gray-200 p-5">
            <p class="text-[11px] font-medium text-gray-500 uppercase tracking-wider">Communities</p>
            <p class="text-2xl font-semibold text-gray-900 mt-1">{$botStatus.communities}</p>
        </div>
        <div class="bg-gray-50 rounded-xl border border-gray-200 p-5">
            <p class="text-[11px] font-medium text-gray-500 uppercase tracking-wider">Active Skills</p>
            <p class="text-2xl font-semibold text-gray-900 mt-1">{$botStatus.skills}</p>
        </div>
        <div class="bg-gray-50 rounded-xl border border-gray-200 p-5">
            <p class="text-[11px] font-medium text-gray-500 uppercase tracking-wider">Users</p>
            <p class="text-2xl font-semibold text-gray-900 mt-1">{$botStatus.users}</p>
        </div>
    </div>

    <!-- Status -->
    <div class="bg-gray-50 rounded-xl border border-gray-200 p-5">
        <div class="flex items-center justify-between">
            <div class="flex items-center gap-3">
                <span class="w-3 h-3 rounded-full {$botStatus.running ? 'bg-green-500' : 'bg-red-500'}"></span>
                <div>
                    <p class="text-sm font-medium text-gray-900">
                        {$botStatus.running ? 'Bot Running' : 'Bot Stopped'}
                    </p>
                    <p class="text-xs text-gray-500">Local Mode</p>
                </div>
            </div>
            <button class="px-3 py-1.5 text-xs font-medium rounded-md border border-gray-300 text-gray-700 hover:bg-gray-100 transition-colors">
                {$botStatus.running ? 'Stop' : 'Start'}
            </button>
        </div>
    </div>
</div>
```

**Step 6: Verify frontend**

```bash
cd desktop && npm run dev
```

**Step 7: Commit**

```bash
git commit -m "feat: add Svelte frontend with sidebar layout and dashboard"
```

---

## Task 7: Svelte Frontend — Communities Page

**Files:**
- Create: `desktop/src/lib/pages/Communities.svelte`
- Modify: `desktop/src/routes/+page.svelte` (add page routing)

**Step 1: Create Communities page**

```svelte
<!-- desktop/src/lib/pages/Communities.svelte -->
<script lang="ts">
    import { onMount } from 'svelte';
    import { api } from '$lib/api';

    let communities: any[] = [];
    let loading = true;

    onMount(async () => {
        const data = await api.getCommunities() as any;
        communities = data.communities || [];
        loading = false;
    });
</script>

<div class="p-8 max-w-4xl">
    <div class="flex items-center justify-between mb-8">
        <div>
            <h2 class="text-xl font-semibold text-gray-900 tracking-tight">Communities</h2>
            <p class="text-sm text-gray-500 mt-1">Manage your connected groups</p>
        </div>
        <button class="px-3 py-1.5 text-xs font-medium rounded-md bg-gray-900 text-white hover:bg-gray-800 transition-colors">
            Add Community
        </button>
    </div>

    {#if loading}
        <p class="text-sm text-gray-500">Loading...</p>
    {:else if communities.length === 0}
        <div class="text-center py-16 bg-gray-50 rounded-xl border border-gray-200">
            <p class="text-sm text-gray-500">No communities yet</p>
            <p class="text-xs text-gray-400 mt-1">Add your first community to get started</p>
        </div>
    {:else}
        <div class="space-y-2">
            {#each communities as community}
                <div class="flex items-center justify-between p-4 bg-gray-50 rounded-xl border border-gray-200 hover:border-gray-300 transition-colors">
                    <div>
                        <p class="text-sm font-medium text-gray-900">{community.name}</p>
                        <p class="text-xs text-gray-500 mt-0.5">
                            {community.platform} · {community.zone} · {community.language}
                        </p>
                    </div>
                    <div class="flex items-center gap-2">
                        <span class="text-xs text-gray-400">{community.enabled_skills?.length || 0} skills</span>
                        <button class="text-xs text-gray-500 hover:text-gray-900">Edit</button>
                    </div>
                </div>
            {/each}
        </div>
    {/if}
</div>
```

**Step 2: Add page routing to +page.svelte**

Update the main page to conditionally render based on `currentPage` store:

```svelte
<!-- desktop/src/routes/+page.svelte -->
<script lang="ts">
    import { currentPage } from '$lib/stores/app';
    import Communities from '$lib/pages/Communities.svelte';
    // ... import other pages
</script>

{#if $currentPage === 'communities'}
    <Communities />
{:else if $currentPage === 'skills'}
    <!-- Skills page placeholder -->
{:else}
    <!-- Dashboard content -->
{/if}
```

**Step 3: Commit**

```bash
git commit -m "feat: add Communities page with list and add button"
```

---

## Task 8: Svelte Frontend — Skills & Users Pages

**Files:**
- Create: `desktop/src/lib/pages/Skills.svelte`
- Create: `desktop/src/lib/pages/Users.svelte`
- Modify: `desktop/src/routes/+page.svelte`

**Step 1: Create Skills page**

```svelte
<!-- desktop/src/lib/pages/Skills.svelte -->
<script lang="ts">
    import { onMount } from 'svelte';
    import { api } from '$lib/api';

    let skills: any[] = [];

    onMount(async () => {
        const data = await api.getSkills() as any;
        skills = data.skills || [];
    });
</script>

<div class="p-8 max-w-4xl">
    <div class="mb-8">
        <h2 class="text-xl font-semibold text-gray-900 tracking-tight">Skills</h2>
        <p class="text-sm text-gray-500 mt-1">Manage bot capabilities</p>
    </div>

    <div class="space-y-2">
        {#each skills as skill}
            <div class="flex items-center justify-between p-4 bg-gray-50 rounded-xl border border-gray-200">
                <div>
                    <p class="text-sm font-medium text-gray-900">{skill.name}</p>
                    <p class="text-xs text-gray-500 mt-0.5">{skill.description}</p>
                </div>
                <label class="relative inline-flex items-center cursor-pointer">
                    <input type="checkbox" checked class="sr-only peer">
                    <div class="w-9 h-5 bg-gray-300 peer-checked:bg-gray-900 rounded-full transition-colors
                        after:content-[''] after:absolute after:top-[2px] after:start-[2px]
                        after:bg-white after:rounded-full after:h-4 after:w-4 after:transition-all
                        peer-checked:after:translate-x-full"></div>
                </label>
            </div>
        {/each}
    </div>
</div>
```

**Step 2: Create Users page**

```svelte
<!-- desktop/src/lib/pages/Users.svelte -->
<script lang="ts">
    import { onMount } from 'svelte';
    import { api } from '$lib/api';

    let users: any[] = [];

    onMount(async () => {
        const data = await api.getUsers() as any;
        users = data.users || [];
    });
</script>

<div class="p-8 max-w-4xl">
    <div class="mb-8">
        <h2 class="text-xl font-semibold text-gray-900 tracking-tight">Users</h2>
        <p class="text-sm text-gray-500 mt-1">Manage bot users and permissions</p>
    </div>

    {#if users.length === 0}
        <div class="text-center py-16 bg-gray-50 rounded-xl border border-gray-200">
            <p class="text-sm text-gray-500">No users registered yet</p>
        </div>
    {:else}
        <div class="bg-white rounded-xl border border-gray-200 overflow-hidden">
            <table class="w-full text-sm">
                <thead>
                    <tr class="border-b border-gray-100">
                        <th class="text-left px-4 py-3 text-[11px] font-medium text-gray-500 uppercase tracking-wider">User</th>
                        <th class="text-left px-4 py-3 text-[11px] font-medium text-gray-500 uppercase tracking-wider">Platform</th>
                        <th class="text-left px-4 py-3 text-[11px] font-medium text-gray-500 uppercase tracking-wider">Status</th>
                        <th class="text-right px-4 py-3 text-[11px] font-medium text-gray-500 uppercase tracking-wider">Actions</th>
                    </tr>
                </thead>
                <tbody>
                    {#each users as user}
                        <tr class="border-b border-gray-50 hover:bg-gray-50 transition-colors">
                            <td class="px-4 py-3 text-gray-900">{user.user_id}</td>
                            <td class="px-4 py-3 text-gray-500">{user.platform}</td>
                            <td class="px-4 py-3">
                                <span class="inline-flex px-2 py-0.5 text-[11px] font-medium rounded-full
                                    {user.state === 'Admin' ? 'bg-purple-100 text-purple-700' :
                                     user.state === 'Approved' ? 'bg-green-100 text-green-700' :
                                     user.state === 'Pending' ? 'bg-yellow-100 text-yellow-700' :
                                     user.state === 'Blocked' ? 'bg-red-100 text-red-700' :
                                     'bg-gray-100 text-gray-700'}">
                                    {user.state}
                                </span>
                            </td>
                            <td class="px-4 py-3 text-right">
                                {#if user.state === 'Pending'}
                                    <button class="text-xs text-green-600 hover:text-green-800 mr-2">Approve</button>
                                {/if}
                                {#if user.state !== 'Blocked' && user.state !== 'Admin'}
                                    <button class="text-xs text-red-600 hover:text-red-800">Block</button>
                                {/if}
                            </td>
                        </tr>
                    {/each}
                </tbody>
            </table>
        </div>
    {/if}
</div>
```

**Step 3: Wire pages into routing, commit**

```bash
git commit -m "feat: add Skills and Users pages"
```

---

## Task 9: Svelte Frontend — Settings Page

**Files:**
- Create: `desktop/src/lib/pages/Settings.svelte`

**Step 1: Create Settings page with mode switch**

```svelte
<!-- desktop/src/lib/pages/Settings.svelte -->
<script lang="ts">
    import { onMount } from 'svelte';
    import { api } from '$lib/api';

    let mode = 'local';
    let remoteUrl = '';
    let remoteToken = '';
    let saved = false;

    onMount(async () => {
        const m = await api.getMode() as string;
        if (m.startsWith('remote:')) {
            mode = 'remote';
            remoteUrl = m.replace('remote:', '');
        }
    });

    async function saveMode() {
        await api.setMode(mode, remoteUrl, remoteToken);
        saved = true;
        setTimeout(() => saved = false, 2000);
    }
</script>

<div class="p-8 max-w-2xl">
    <div class="mb-8">
        <h2 class="text-xl font-semibold text-gray-900 tracking-tight">Settings</h2>
        <p class="text-sm text-gray-500 mt-1">Configure your AmanClaw instance</p>
    </div>

    <!-- Connection Mode -->
    <div class="mb-8">
        <h3 class="text-sm font-medium text-gray-900 mb-3">Connection Mode</h3>
        <div class="space-y-2">
            <label class="flex items-center gap-3 p-3 rounded-lg border border-gray-200 cursor-pointer hover:bg-gray-50 transition-colors
                {mode === 'local' ? 'border-gray-900 bg-gray-50' : ''}">
                <input type="radio" bind:group={mode} value="local" class="accent-gray-900">
                <div>
                    <p class="text-sm font-medium text-gray-900">Local Mode</p>
                    <p class="text-xs text-gray-500">Bot runs on this machine</p>
                </div>
            </label>
            <label class="flex items-center gap-3 p-3 rounded-lg border border-gray-200 cursor-pointer hover:bg-gray-50 transition-colors
                {mode === 'remote' ? 'border-gray-900 bg-gray-50' : ''}">
                <input type="radio" bind:group={mode} value="remote" class="accent-gray-900">
                <div>
                    <p class="text-sm font-medium text-gray-900">Remote Mode</p>
                    <p class="text-xs text-gray-500">Connect to a remote AmanClaw server</p>
                </div>
            </label>
        </div>

        {#if mode === 'remote'}
            <div class="mt-4 space-y-3">
                <div>
                    <label class="block text-xs font-medium text-gray-700 mb-1">Server URL</label>
                    <input type="text" bind:value={remoteUrl} placeholder="https://your-server.com"
                        class="w-full px-3 py-2 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-gray-900 focus:border-transparent">
                </div>
                <div>
                    <label class="block text-xs font-medium text-gray-700 mb-1">API Token</label>
                    <input type="password" bind:value={remoteToken} placeholder="Bearer token"
                        class="w-full px-3 py-2 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-gray-900 focus:border-transparent">
                </div>
            </div>
        {/if}

        <button on:click={saveMode}
            class="mt-4 px-4 py-2 text-xs font-medium rounded-md bg-gray-900 text-white hover:bg-gray-800 transition-colors">
            {saved ? 'Saved' : 'Save'}
        </button>
    </div>

    <!-- About -->
    <div class="border-t border-gray-200 pt-6">
        <h3 class="text-sm font-medium text-gray-900 mb-2">About</h3>
        <p class="text-xs text-gray-500">AmanClaw Desktop v0.1.0</p>
        <p class="text-xs text-gray-500">Built in Malaysia</p>
    </div>
</div>
```

**Step 2: Commit**

```bash
git commit -m "feat: add Settings page with local/remote mode switch"
```

---

## Task 10: System Tray Notifications & Solat Reminders

**Files:**
- Create: `desktop/src-tauri/src/notifications.rs`
- Modify: `desktop/src-tauri/src/main.rs`

**Step 1: Create notifications.rs**

```rust
// desktop/src-tauri/src/notifications.rs
use tauri::AppHandle;
use tauri_plugin_notification::NotificationExt;

pub fn notify_solat(app: &AppHandle, prayer: &str, time: &str, zone: &str) {
    let _ = app.notification()
        .builder()
        .title(&format!("Waktu {} - {}", prayer, zone))
        .body(&format!("Waktu {} telah masuk: {}", prayer, time))
        .show();
}

pub fn notify_user_pending(app: &AppHandle, user_id: &str, platform: &str) {
    let _ = app.notification()
        .builder()
        .title("New User Pending")
        .body(&format!("{} ({}) is waiting for approval", user_id, platform))
        .show();
}

pub fn notify_skill_error(app: &AppHandle, skill: &str, error: &str) {
    let _ = app.notification()
        .builder()
        .title(&format!("Skill Error: {}", skill))
        .body(error)
        .show();
}

pub fn notify_community_joined(app: &AppHandle, name: &str) {
    let _ = app.notification()
        .builder()
        .title("Community Joined")
        .body(&format!("{} has been onboarded", name))
        .show();
}
```

**Step 2: Wire into main.rs setup, commit**

```bash
git commit -m "feat: add native notifications for solat, users, and skill errors"
```

---

## Task 11: Logs Page with Live Streaming

**Files:**
- Create: `desktop/src/lib/pages/Logs.svelte`
- Create: `desktop/src-tauri/src/logs.rs`

**Step 1: Create log capture in Rust**

```rust
// desktop/src-tauri/src/logs.rs
use std::sync::Arc;
use tokio::sync::broadcast;
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: String,
    pub target: String,
    pub message: String,
}

pub struct LogBroadcaster {
    tx: broadcast::Sender<LogEntry>,
}

impl LogBroadcaster {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(1000);
        Self { tx }
    }

    pub fn send(&self, entry: LogEntry) {
        let _ = self.tx.send(entry);
    }

    pub fn subscribe(&self) -> broadcast::Receiver<LogEntry> {
        self.tx.subscribe()
    }
}
```

**Step 2: Create Logs Svelte page**

```svelte
<!-- desktop/src/lib/pages/Logs.svelte -->
<script lang="ts">
    let logs: any[] = [];
    let filter = '';

    $: filteredLogs = filter
        ? logs.filter(l => l.message.toLowerCase().includes(filter.toLowerCase())
            || l.level.toLowerCase().includes(filter.toLowerCase()))
        : logs;
</script>

<div class="p-8 max-w-5xl">
    <div class="flex items-center justify-between mb-6">
        <div>
            <h2 class="text-xl font-semibold text-gray-900 tracking-tight">Logs</h2>
            <p class="text-sm text-gray-500 mt-1">Live bot activity</p>
        </div>
        <input type="text" bind:value={filter} placeholder="Filter logs..."
            class="px-3 py-1.5 text-xs border border-gray-200 rounded-md w-48 focus:outline-none focus:ring-2 focus:ring-gray-900">
    </div>

    <div class="bg-gray-950 rounded-xl p-4 font-mono text-xs h-[calc(100vh-200px)] overflow-y-auto">
        {#each filteredLogs as log}
            <div class="py-0.5 flex gap-3">
                <span class="text-gray-600 shrink-0">{log.timestamp}</span>
                <span class="shrink-0 {
                    log.level === 'ERROR' ? 'text-red-400' :
                    log.level === 'WARN' ? 'text-yellow-400' :
                    log.level === 'INFO' ? 'text-blue-400' :
                    'text-gray-500'
                }">{log.level.padEnd(5)}</span>
                <span class="text-gray-300">{log.message}</span>
            </div>
        {/each}
        {#if filteredLogs.length === 0}
            <p class="text-gray-600">No logs yet. Start the bot to see activity.</p>
        {/if}
    </div>
</div>
```

**Step 3: Commit**

```bash
git commit -m "feat: add Logs page with filtering and live log stream"
```

---

## Task 12: Content Management Page

**Files:**
- Create: `desktop/src/lib/pages/Content.svelte`

**Step 1: Create Content page for doa/zakat management**

```svelte
<!-- desktop/src/lib/pages/Content.svelte -->
<script lang="ts">
    let activeTab = 'doa';
</script>

<div class="p-8 max-w-4xl">
    <div class="mb-6">
        <h2 class="text-xl font-semibold text-gray-900 tracking-tight">Content</h2>
        <p class="text-sm text-gray-500 mt-1">Manage Islamic content and data</p>
    </div>

    <!-- Tabs -->
    <div class="flex gap-1 mb-6 bg-gray-100 p-1 rounded-lg w-fit">
        {#each ['doa', 'zakat', 'khutbah'] as tab}
            <button
                class="px-3 py-1.5 text-xs font-medium rounded-md transition-colors
                    {activeTab === tab ? 'bg-white text-gray-900 shadow-sm' : 'text-gray-600 hover:text-gray-900'}"
                on:click={() => activeTab = tab}
            >
                {tab.charAt(0).toUpperCase() + tab.slice(1)}
            </button>
        {/each}
    </div>

    {#if activeTab === 'doa'}
        <div class="bg-gray-50 rounded-xl border border-gray-200 p-5">
            <div class="flex items-center justify-between mb-4">
                <p class="text-sm font-medium text-gray-900">Doa Collection</p>
                <button class="text-xs text-gray-500 hover:text-gray-900">Add Doa</button>
            </div>
            <p class="text-xs text-gray-500">20 doas across 9 categories. Edit via the collection manager.</p>
        </div>
    {:else if activeTab === 'zakat'}
        <div class="bg-gray-50 rounded-xl border border-gray-200 p-5">
            <p class="text-sm font-medium text-gray-900 mb-4">Zakat Fitrah Rates (2026)</p>
            <p class="text-xs text-gray-500">Update yearly rates per state from JAKIM.</p>
        </div>
    {:else if activeTab === 'khutbah'}
        <div class="bg-gray-50 rounded-xl border border-gray-200 p-5">
            <p class="text-sm font-medium text-gray-900 mb-4">Khutbah Cache</p>
            <p class="text-xs text-gray-500">Cached weekly khutbah from JAKIM portal.</p>
        </div>
    {/if}
</div>
```

**Step 2: Commit**

```bash
git commit -m "feat: add Content management page for doa, zakat, khutbah"
```

---

## Task 13: Full Build & Integration Test

**Step 1: Build REST API**

Run: `cd rust && cargo build -p amanclaw-api`

**Step 2: Build full workspace**

Run: `cd rust && cargo build`

**Step 3: Run all tests**

Run: `cd rust && cargo test --workspace`

**Step 4: Build Tauri desktop app**

Run: `cd desktop && cargo tauri build`

**Step 5: Test the app launches**

Run: Open the built `.dmg`/`.app` and verify:
- App window opens with sidebar
- Dashboard shows stats (all zeros initially)
- All pages navigate correctly
- System tray icon appears
- Settings page allows mode switching

**Step 6: Commit**

```bash
git commit -m "chore: verify full build of desktop app and REST API"
```

---

## Summary

| Task | Component | Description |
|------|-----------|-------------|
| 1 | amanclaw-api | Scaffold REST API crate with bot status endpoint |
| 2 | amanclaw-api | Community and skills endpoints |
| 3 | amanclaw-api | Users endpoint with approve/block |
| 4 | amanclaw-core | Wire REST API into engine startup |
| 5 | desktop | Scaffold Tauri + Svelte + Tailwind app |
| 6 | desktop/svelte | Layout shell and Dashboard page |
| 7 | desktop/svelte | Communities page |
| 8 | desktop/svelte | Skills and Users pages |
| 9 | desktop/svelte | Settings page with mode switch |
| 10 | desktop/tauri | System tray notifications |
| 11 | desktop/svelte | Logs page with live streaming |
| 12 | desktop/svelte | Content management page |
| 13 | build | Full build and integration test |

**Total: 13 tasks. REST API (Tasks 1-4) should be built first, then desktop app (Tasks 5-12), then full verification (Task 13).**
