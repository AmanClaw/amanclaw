# Embedded Engine Desktop App Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Embed the amanclaw-core Engine directly into the Tauri desktop app so users get a single binary that runs the bot with GUI configuration.

**Architecture:** Add amanclaw-core/traits/api as Cargo dependencies to the Tauri app. Engine lifecycle (start/stop/restart) managed via IPC commands. Config stored in OS app data dir. First-run wizard for initial LLM setup. Channels enabled via GUI toggles in Settings.

**Tech Stack:** Tauri 2, Svelte 5, Tailwind CSS 4, amanclaw-core, amanclaw-traits, amanclaw-api, serde_yaml, dotenvy

---

### Task 1: Add Rust Dependencies

**Files:**
- Modify: `desktop/src-tauri/Cargo.toml`

**Step 1: Add amanclaw crate dependencies**

Edit `desktop/src-tauri/Cargo.toml` to add after the existing `[dependencies]`:

```toml
[dependencies]
tauri = { version = "2", features = ["tray-icon", "image-png"] }
tauri-plugin-notification = "2"
tauri-plugin-shell = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
serde_yaml = "0.9"
tokio = { version = "1", features = ["full"] }
reqwest = { version = "0.12", features = ["json"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
dotenvy = "0.15"
anyhow = "1"

# AmanClaw engine
amanclaw-core = { path = "../../rust/crates/amanclaw-core" }
amanclaw-traits = { path = "../../rust/crates/amanclaw-traits" }
amanclaw-api = { path = "../../rust/crates/amanclaw-api" }
```

**Step 2: Verify it compiles**

Run: `cd desktop && cargo check -p amanclaw-desktop 2>&1 | tail -5`
Expected: no errors (warnings OK)

**Step 3: Commit**

```bash
git add desktop/src-tauri/Cargo.toml
git commit -m "feat(desktop): add amanclaw-core engine dependencies"
```

---

### Task 2: Expand AppState for Engine Lifecycle

**Files:**
- Modify: `desktop/src-tauri/src/state.rs`

**Step 1: Rewrite state.rs with engine holder**

Replace the entire contents of `desktop/src-tauri/src/state.rs` with:

```rust
use amanclaw_traits::config::AppConfig;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::mpsc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AppMode {
    Local,
    Remote { url: String, token: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EngineStatus {
    Stopped,
    Starting,
    Running,
    Error(String),
}

/// Holds a sender that, when dropped, signals the engine to stop.
pub struct EngineHandle {
    /// Dropping this aborts the engine task.
    pub abort_handle: tokio::task::AbortHandle,
    /// Auth for user management.
    pub auth: Arc<std::sync::Mutex<amanclaw_security::auth::Auth>>,
    /// SQLite pool for queries.
    pub pool: sqlx::SqlitePool,
    /// Plugin registry for skill listing.
    pub registry: Arc<amanclaw_core::registry::PluginRegistry>,
}

pub struct AppState {
    pub mode: AppMode,
    pub engine_status: EngineStatus,
    pub engine_handle: Option<EngineHandle>,
    pub config: Option<AppConfig>,
    pub started_at: Option<std::time::Instant>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            mode: AppMode::Local,
            engine_status: EngineStatus::Stopped,
            engine_handle: None,
            config: None,
            started_at: None,
        }
    }
}
```

**Step 2: Verify it compiles**

Run: `cd desktop && cargo check -p amanclaw-desktop 2>&1 | tail -5`
Expected: no errors

**Step 3: Commit**

```bash
git add desktop/src-tauri/src/state.rs
git commit -m "feat(desktop): expand AppState with engine handle and status"
```

---

### Task 3: Config File Management Commands

**Files:**
- Create: `desktop/src-tauri/src/config.rs`
- Modify: `desktop/src-tauri/src/lib.rs` (add `mod config;`)

**Step 1: Create config.rs for file I/O**

Create `desktop/src-tauri/src/config.rs`:

```rust
use amanclaw_traits::config::AppConfig;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

/// Get the app data directory, creating it if needed.
pub fn data_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

pub fn config_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(data_dir(app)?.join("config.yaml"))
}

pub fn secrets_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(data_dir(app)?.join("secrets.env"))
}

pub fn db_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(data_dir(app)?.join("memory.db"))
}

pub fn plugins_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = data_dir(app)?.join("plugins");
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

pub fn load_config(app: &AppHandle) -> Result<AppConfig, String> {
    let path = config_path(app)?;
    let content = fs::read_to_string(&path).map_err(|e| format!("Cannot read config: {}", e))?;
    serde_yaml::from_str(&content).map_err(|e| format!("Invalid config: {}", e))
}

pub fn save_config(app: &AppHandle, config: &AppConfig) -> Result<(), String> {
    let path = config_path(app)?;
    let yaml = serde_yaml::to_string(config).map_err(|e| e.to_string())?;
    fs::write(&path, yaml).map_err(|e| e.to_string())
}

pub fn load_secrets(app: &AppHandle) -> HashMap<String, String> {
    let path = match secrets_path(app) {
        Ok(p) => p,
        Err(_) => return HashMap::new(),
    };
    let content = match fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return HashMap::new(),
    };
    content
        .lines()
        .filter(|l| !l.starts_with('#') && l.contains('='))
        .filter_map(|l| {
            let mut parts = l.splitn(2, '=');
            let key = parts.next()?.trim().to_string();
            let val = parts.next()?.trim().to_string();
            Some((key, val))
        })
        .collect()
}

pub fn save_secrets(app: &AppHandle, secrets: &HashMap<String, String>) -> Result<(), String> {
    let path = secrets_path(app)?;
    let content: String = secrets
        .iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(&path, content).map_err(|e| e.to_string())
}

pub fn has_config(app: &AppHandle) -> bool {
    config_path(app).map(|p| p.exists()).unwrap_or(false)
}
```

**Step 2: Add mod declaration in lib.rs**

In `desktop/src-tauri/src/lib.rs`, add `mod config;` after the existing mod declarations:

```rust
mod commands;
mod config;
mod logs;
mod notifications;
mod state;
mod tray;
```

**Step 3: Verify it compiles**

Run: `cd desktop && cargo check -p amanclaw-desktop 2>&1 | tail -5`

**Step 4: Commit**

```bash
git add desktop/src-tauri/src/config.rs desktop/src-tauri/src/lib.rs
git commit -m "feat(desktop): add config file management module"
```

---

### Task 4: Engine Lifecycle IPC Commands

**Files:**
- Modify: `desktop/src-tauri/src/commands.rs`
- Modify: `desktop/src-tauri/src/lib.rs` (register new commands)

**Step 1: Rewrite commands.rs with engine lifecycle**

Replace the entire contents of `desktop/src-tauri/src/commands.rs`:

```rust
use crate::config;
use crate::state::{AppMode, AppState, EngineHandle, EngineStatus};
use amanclaw_traits::config::{AppConfig, LlmConfig};
use std::collections::HashMap;
use std::sync::Arc;
use tauri::{AppHandle, State};
use tokio::sync::RwLock;

type SharedState = Arc<RwLock<AppState>>;

// --- First-run & config ---

#[tauri::command]
pub async fn check_first_run(app: AppHandle) -> Result<bool, String> {
    Ok(!config::has_config(&app))
}

#[tauri::command]
pub async fn get_config(app: AppHandle) -> Result<serde_json::Value, String> {
    if !config::has_config(&app) {
        return Ok(serde_json::json!(null));
    }
    let cfg = config::load_config(&app)?;
    let secrets = config::load_secrets(&app);
    Ok(serde_json::json!({
        "llm": {
            "base_url": cfg.llm.base_url,
            "model": cfg.llm.model,
            "max_tokens": cfg.llm.max_tokens,
            "temperature": cfg.llm.temperature,
            "api_key": cfg.llm.api_key.unwrap_or_default(),
        },
        "rate_limit_per_minute": cfg.rate_limit_per_minute,
        "channels": {
            "telegram": secrets.get("TELEGRAM_BOT_TOKEN").cloned().unwrap_or_default(),
            "discord": secrets.get("DISCORD_BOT_TOKEN").cloned().unwrap_or_default(),
            "slack_bot": secrets.get("SLACK_BOT_TOKEN").cloned().unwrap_or_default(),
            "slack_app": secrets.get("SLACK_APP_TOKEN").cloned().unwrap_or_default(),
        },
    }))
}

#[tauri::command]
pub async fn save_config(
    app: AppHandle,
    state: State<'_, SharedState>,
    llm_base_url: String,
    llm_model: String,
    llm_api_key: String,
    max_tokens: Option<u32>,
    temperature: Option<f32>,
    rate_limit: Option<u32>,
    telegram_token: Option<String>,
    discord_token: Option<String>,
    slack_bot_token: Option<String>,
    slack_app_token: Option<String>,
) -> Result<(), String> {
    let plugins_dir = config::plugins_dir(&app)?;

    let cfg = AppConfig {
        llm: LlmConfig {
            base_url: llm_base_url,
            model: llm_model,
            max_tokens: max_tokens.unwrap_or(4096),
            temperature: temperature.unwrap_or(0.7),
            api_key: if llm_api_key.is_empty() { None } else { Some(llm_api_key) },
            native_tool_calling: None,
        },
        admin_users: HashMap::new(),
        rate_limit_per_minute: rate_limit.unwrap_or(20),
        plugins: amanclaw_traits::config::PluginConfig {
            dir: plugins_dir.to_string_lossy().to_string(),
            hot_reload: false,
        },
        security: Default::default(),
        skills: Default::default(),
        mcp_servers: HashMap::new(),
        script_plugins: HashMap::new(),
    };

    config::save_config(&app, &cfg)?;

    // Save channel tokens as secrets
    let mut secrets = HashMap::new();
    if let Some(t) = telegram_token.filter(|s| !s.is_empty()) {
        secrets.insert("TELEGRAM_BOT_TOKEN".to_string(), t);
    }
    if let Some(t) = discord_token.filter(|s| !s.is_empty()) {
        secrets.insert("DISCORD_BOT_TOKEN".to_string(), t);
    }
    if let Some(t) = slack_bot_token.filter(|s| !s.is_empty()) {
        secrets.insert("SLACK_BOT_TOKEN".to_string(), t);
    }
    if let Some(t) = slack_app_token.filter(|s| !s.is_empty()) {
        secrets.insert("SLACK_APP_TOKEN".to_string(), t);
    }
    config::save_secrets(&app, &secrets)?;

    // Update in-memory config
    let mut st = state.write().await;
    st.config = Some(cfg);

    Ok(())
}

// --- Engine lifecycle ---

#[tauri::command]
pub async fn start_engine(
    app: AppHandle,
    state: State<'_, SharedState>,
) -> Result<(), String> {
    {
        let st = state.read().await;
        if matches!(st.engine_status, EngineStatus::Running | EngineStatus::Starting) {
            return Err("Engine already running".into());
        }
    }

    // Mark starting
    {
        let mut st = state.write().await;
        st.engine_status = EngineStatus::Starting;
    }

    // Load config
    let cfg = config::load_config(&app)?;
    let db_path = config::db_path(&app)?;
    let secrets = config::load_secrets(&app);

    // Set env vars for channel adapters (they read from env)
    for (k, v) in &secrets {
        std::env::set_var(k, v);
    }
    std::env::set_var("MEMORY_DB_PATH", db_path.to_string_lossy().as_ref());

    // Initialize engine
    let engine = amanclaw_core::Engine::new(cfg.clone())
        .await
        .map_err(|e| format!("Engine init failed: {}", e))?;

    // Grab handles before moving engine into the task
    let auth = engine.auth().clone();
    let pool = engine.pool().clone();
    let registry = engine.registry().clone();

    // Spawn engine in background
    let state_clone = state.inner().clone();
    let join_handle = tokio::spawn(async move {
        if let Err(e) = engine.run().await {
            let mut st = state_clone.write().await;
            st.engine_status = EngineStatus::Error(e.to_string());
            tracing::error!(error = %e, "Engine error");
        } else {
            let mut st = state_clone.write().await;
            st.engine_status = EngineStatus::Stopped;
        }
    });

    // Store handle
    {
        let mut st = state.write().await;
        st.engine_status = EngineStatus::Running;
        st.config = Some(cfg);
        st.started_at = Some(std::time::Instant::now());
        st.engine_handle = Some(EngineHandle {
            abort_handle: join_handle.abort_handle(),
            auth,
            pool,
            registry,
        });
    }

    Ok(())
}

#[tauri::command]
pub async fn stop_engine(
    state: State<'_, SharedState>,
) -> Result<(), String> {
    let mut st = state.write().await;
    if let Some(handle) = st.engine_handle.take() {
        handle.abort_handle.abort();
        st.engine_status = EngineStatus::Stopped;
        st.started_at = None;
        Ok(())
    } else {
        Err("Engine not running".into())
    }
}

#[tauri::command]
pub async fn restart_engine(
    app: AppHandle,
    state: State<'_, SharedState>,
) -> Result<(), String> {
    // Stop if running
    {
        let mut st = state.write().await;
        if let Some(handle) = st.engine_handle.take() {
            handle.abort_handle.abort();
            st.engine_status = EngineStatus::Stopped;
            st.started_at = None;
        }
    }
    // Small delay for cleanup
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    // Start again
    start_engine(app, state).await
}

// --- Status ---

#[tauri::command]
pub async fn get_status(
    state: State<'_, SharedState>,
) -> Result<serde_json::Value, String> {
    let st = state.read().await;
    let uptime_secs = st.started_at.map(|t| t.elapsed().as_secs()).unwrap_or(0);

    let (status_str, error_msg) = match &st.engine_status {
        EngineStatus::Stopped => ("stopped", None),
        EngineStatus::Starting => ("starting", None),
        EngineStatus::Running => ("running", None),
        EngineStatus::Error(e) => ("error", Some(e.clone())),
    };

    Ok(serde_json::json!({
        "engine_status": status_str,
        "bot_running": matches!(st.engine_status, EngineStatus::Running),
        "mode": match &st.mode {
            AppMode::Local => "local",
            AppMode::Remote { .. } => "remote",
        },
        "uptime_seconds": uptime_secs,
        "error": error_msg,
    }))
}

// --- Mode ---

#[tauri::command]
pub async fn get_mode(
    state: State<'_, SharedState>,
) -> Result<String, String> {
    let app = state.read().await;
    Ok(match &app.mode {
        AppMode::Local => "local".to_string(),
        AppMode::Remote { url, .. } => format!("remote:{}", url),
    })
}

#[tauri::command]
pub async fn set_mode(
    state: State<'_, SharedState>,
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

// --- Data (local engine or remote API) ---

#[tauri::command]
pub async fn get_communities(
    state: State<'_, SharedState>,
) -> Result<serde_json::Value, String> {
    let st = state.read().await;
    match &st.mode {
        AppMode::Remote { url, token } => {
            let client = reqwest::Client::new();
            let resp = client.get(format!("{}/api/communities", url))
                .bearer_auth(token)
                .send().await.map_err(|e| e.to_string())?;
            resp.json().await.map_err(|e| e.to_string())
        }
        AppMode::Local => {
            // TODO: query local engine pool when CommunityRepo is wired
            Ok(serde_json::json!({ "communities": [], "count": 0 }))
        }
    }
}

#[tauri::command]
pub async fn get_skills(
    state: State<'_, SharedState>,
) -> Result<serde_json::Value, String> {
    let st = state.read().await;
    match &st.mode {
        AppMode::Remote { url, token } => {
            let client = reqwest::Client::new();
            let resp = client.get(format!("{}/api/skills", url))
                .bearer_auth(token)
                .send().await.map_err(|e| e.to_string())?;
            resp.json().await.map_err(|e| e.to_string())
        }
        AppMode::Local => {
            if let Some(handle) = &st.engine_handle {
                let skills: Vec<serde_json::Value> = handle.registry.iter_skills()
                    .map(|(name, skill)| {
                        let meta = skill.metadata();
                        serde_json::json!({
                            "name": name,
                            "description": meta.description,
                            "version": meta.version,
                        })
                    })
                    .collect();
                let count = skills.len();
                Ok(serde_json::json!({ "skills": skills, "count": count }))
            } else {
                Ok(serde_json::json!({ "skills": [], "count": 0 }))
            }
        }
    }
}

#[tauri::command]
pub async fn get_users(
    state: State<'_, SharedState>,
) -> Result<serde_json::Value, String> {
    let st = state.read().await;
    match &st.mode {
        AppMode::Remote { url, token } => {
            let client = reqwest::Client::new();
            let resp = client.get(format!("{}/api/users", url))
                .bearer_auth(token)
                .send().await.map_err(|e| e.to_string())?;
            resp.json().await.map_err(|e| e.to_string())
        }
        AppMode::Local => {
            if let Some(handle) = &st.engine_handle {
                let auth = handle.auth.lock().unwrap();
                let users = auth.list_users();
                let user_list: Vec<serde_json::Value> = users.iter()
                    .map(|(id, platform, status)| {
                        serde_json::json!({
                            "user_id": id,
                            "platform": platform,
                            "status": format!("{:?}", status),
                        })
                    })
                    .collect();
                let count = user_list.len();
                Ok(serde_json::json!({ "users": user_list, "count": count }))
            } else {
                Ok(serde_json::json!({ "users": [], "count": 0 }))
            }
        }
    }
}

// --- Data dir ---

#[tauri::command]
pub async fn get_data_dir(app: AppHandle) -> Result<String, String> {
    let dir = config::data_dir(&app)?;
    Ok(dir.to_string_lossy().to_string())
}
```

**Step 2: Register new commands in lib.rs**

Replace the `invoke_handler` block in `desktop/src-tauri/src/lib.rs`:

```rust
        .invoke_handler(tauri::generate_handler![
            commands::check_first_run,
            commands::get_config,
            commands::save_config,
            commands::start_engine,
            commands::stop_engine,
            commands::restart_engine,
            commands::get_status,
            commands::get_mode,
            commands::set_mode,
            commands::get_communities,
            commands::get_skills,
            commands::get_users,
            commands::get_data_dir,
        ])
```

**Step 3: Verify it compiles**

Run: `cd desktop && cargo check -p amanclaw-desktop 2>&1 | tail -10`
Expected: no errors (warnings OK). Fix any type mismatches between Engine API and our commands.

**Step 4: Commit**

```bash
git add desktop/src-tauri/src/commands.rs desktop/src-tauri/src/lib.rs
git commit -m "feat(desktop): add engine lifecycle and config IPC commands"
```

---

### Task 5: Auto-Start Engine on Launch

**Files:**
- Modify: `desktop/src-tauri/src/lib.rs`

**Step 1: Add auto-start logic in setup hook**

Replace the entire `desktop/src-tauri/src/lib.rs`:

```rust
mod commands;
mod config;
mod logs;
mod notifications;
mod state;
mod tray;

use state::AppState;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing_subscriber::EnvFilter;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize logging
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("amanclaw=info"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_shell::init())
        .manage(Arc::new(RwLock::new(AppState::new())))
        .setup(|app| {
            tray::setup_tray(app)?;

            // Auto-start engine if config exists
            let app_handle = app.handle().clone();
            let state: Arc<RwLock<AppState>> = app.state::<Arc<RwLock<AppState>>>().inner().clone();
            if config::has_config(&app_handle) {
                tauri::async_runtime::spawn(async move {
                    tracing::info!("Auto-starting engine from saved config...");
                    // Load secrets into env
                    let secrets = config::load_secrets(&app_handle);
                    for (k, v) in &secrets {
                        std::env::set_var(k, v);
                    }
                    if let Ok(db_path) = config::db_path(&app_handle) {
                        std::env::set_var("MEMORY_DB_PATH", db_path.to_string_lossy().as_ref());
                    }

                    match config::load_config(&app_handle) {
                        Ok(cfg) => {
                            match amanclaw_core::Engine::new(cfg.clone()).await {
                                Ok(engine) => {
                                    let auth = engine.auth().clone();
                                    let pool = engine.pool().clone();
                                    let registry = engine.registry().clone();

                                    let state_clone = state.clone();
                                    let join_handle = tokio::spawn(async move {
                                        if let Err(e) = engine.run().await {
                                            let mut st = state_clone.write().await;
                                            st.engine_status = state::EngineStatus::Error(e.to_string());
                                        } else {
                                            let mut st = state_clone.write().await;
                                            st.engine_status = state::EngineStatus::Stopped;
                                        }
                                    });

                                    let mut st = state.write().await;
                                    st.engine_status = state::EngineStatus::Running;
                                    st.config = Some(cfg);
                                    st.started_at = Some(std::time::Instant::now());
                                    st.engine_handle = Some(state::EngineHandle {
                                        abort_handle: join_handle.abort_handle(),
                                        auth,
                                        pool,
                                        registry,
                                    });
                                    tracing::info!("Engine auto-started successfully");
                                }
                                Err(e) => {
                                    tracing::error!(error = %e, "Engine auto-start failed");
                                    let mut st = state.write().await;
                                    st.engine_status = state::EngineStatus::Error(e.to_string());
                                }
                            }
                        }
                        Err(e) => {
                            tracing::error!(error = %e, "Failed to load config");
                        }
                    }
                });
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::check_first_run,
            commands::get_config,
            commands::save_config,
            commands::start_engine,
            commands::stop_engine,
            commands::restart_engine,
            commands::get_status,
            commands::get_mode,
            commands::set_mode,
            commands::get_communities,
            commands::get_skills,
            commands::get_users,
            commands::get_data_dir,
        ])
        .run(tauri::generate_context!())
        .expect("error running AmanClaw Desktop");
}
```

**Step 2: Verify it compiles**

Run: `cd desktop && cargo check -p amanclaw-desktop 2>&1 | tail -10`

**Step 3: Commit**

```bash
git add desktop/src-tauri/src/lib.rs
git commit -m "feat(desktop): auto-start engine on launch if config exists"
```

---

### Task 6: Update Frontend API Client

**Files:**
- Modify: `desktop/src/lib/api.ts`
- Modify: `desktop/src/lib/stores/app.ts`

**Step 1: Expand api.ts with new commands**

Replace `desktop/src/lib/api.ts`:

```typescript
import { invoke } from '@tauri-apps/api/core';

export const api = {
	// First-run
	checkFirstRun: () => invoke('check_first_run') as Promise<boolean>,

	// Config
	getConfig: () => invoke('get_config'),
	saveConfig: (params: {
		llm_base_url: string;
		llm_model: string;
		llm_api_key: string;
		max_tokens?: number;
		temperature?: number;
		rate_limit?: number;
		telegram_token?: string;
		discord_token?: string;
		slack_bot_token?: string;
		slack_app_token?: string;
	}) => invoke('save_config', params),

	// Engine lifecycle
	startEngine: () => invoke('start_engine'),
	stopEngine: () => invoke('stop_engine'),
	restartEngine: () => invoke('restart_engine'),

	// Status & data
	getStatus: () => invoke('get_status'),
	getCommunities: () => invoke('get_communities'),
	getSkills: () => invoke('get_skills'),
	getUsers: () => invoke('get_users'),
	getMode: () => invoke('get_mode'),
	setMode: (mode: string, url?: string, token?: string) =>
		invoke('set_mode', { mode, url, token }),
	getDataDir: () => invoke('get_data_dir') as Promise<string>,
};
```

**Step 2: Update stores/app.ts**

Replace `desktop/src/lib/stores/app.ts`:

```typescript
import { writable } from 'svelte/store';

export const botStatus = writable({
	engine_status: 'stopped' as 'stopped' | 'starting' | 'running' | 'error',
	bot_running: false,
	mode: 'local',
	uptime_seconds: 0,
	error: null as string | null,
	communities: 0,
	users: 0,
	skills: 0,
});

export const currentPage = writable('dashboard');
export const isFirstRun = writable(false);
```

**Step 3: Commit**

```bash
git add desktop/src/lib/api.ts desktop/src/lib/stores/app.ts
git commit -m "feat(desktop): update frontend API client with engine lifecycle commands"
```

---

### Task 7: First-Run Wizard Page

**Files:**
- Create: `desktop/src/lib/pages/Wizard.svelte`

**Step 1: Create the wizard component**

Create `desktop/src/lib/pages/Wizard.svelte`:

```svelte
<script lang="ts">
	import { api } from '$lib/api';
	import { currentPage, isFirstRun } from '$lib/stores/app';

	let step = $state(1);
	let baseUrl = $state('http://localhost:11434/v1');
	let model = $state('qwen3:8b');
	let apiKey = $state('');
	let error = $state('');
	let saving = $state(false);

	async function finish() {
		if (!baseUrl.trim() || !model.trim()) {
			error = 'Base URL and Model are required';
			return;
		}
		saving = true;
		error = '';
		try {
			await api.saveConfig({
				llm_base_url: baseUrl,
				llm_model: model,
				llm_api_key: apiKey,
			});
			await api.startEngine();
			isFirstRun.set(false);
			currentPage.set('dashboard');
		} catch (e: any) {
			error = e?.toString() || 'Failed to start';
		} finally {
			saving = false;
		}
	}
</script>

<div class="flex items-center justify-center h-full">
	<div class="w-full max-w-md p-8">
		<div class="text-center mb-8">
			<h1 class="text-2xl font-semibold text-gray-900">Welcome to AmanClaw</h1>
			<p class="text-sm text-gray-500 mt-2">Let's get your bot running in 2 steps</p>
		</div>

		{#if step === 1}
			<div class="space-y-4">
				<h2 class="text-sm font-medium text-gray-900">Step 1: LLM Configuration</h2>
				<p class="text-xs text-gray-500">Connect to any OpenAI-compatible API (Ollama, vLLM, OpenAI, etc.)</p>

				<div>
					<label for="base-url" class="block text-xs font-medium text-gray-700 mb-1">Base URL</label>
					<input id="base-url" type="text" bind:value={baseUrl}
						placeholder="http://localhost:11434/v1"
						class="w-full px-3 py-2 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-gray-900">
				</div>

				<div>
					<label for="model-name" class="block text-xs font-medium text-gray-700 mb-1">Model</label>
					<input id="model-name" type="text" bind:value={model}
						placeholder="qwen3:8b"
						class="w-full px-3 py-2 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-gray-900">
				</div>

				<div>
					<label for="api-key" class="block text-xs font-medium text-gray-700 mb-1">API Key <span class="text-gray-400">(optional for local)</span></label>
					<input id="api-key" type="password" bind:value={apiKey}
						placeholder="sk-..."
						class="w-full px-3 py-2 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-gray-900">
				</div>

				<button onclick={() => { step = 2 }}
					class="w-full mt-2 px-4 py-2.5 text-sm font-medium rounded-lg bg-gray-900 text-white hover:bg-gray-800 transition-colors">
					Next
				</button>
			</div>
		{:else}
			<div class="space-y-4">
				<h2 class="text-sm font-medium text-gray-900">Step 2: Ready to Launch</h2>

				<div class="bg-gray-50 rounded-lg border border-gray-200 p-4 space-y-2">
					<div class="flex justify-between text-xs">
						<span class="text-gray-500">LLM URL</span>
						<span class="text-gray-900 font-mono">{baseUrl}</span>
					</div>
					<div class="flex justify-between text-xs">
						<span class="text-gray-500">Model</span>
						<span class="text-gray-900 font-mono">{model}</span>
					</div>
					<div class="flex justify-between text-xs">
						<span class="text-gray-500">API Key</span>
						<span class="text-gray-900">{apiKey ? '••••••••' : 'Not set'}</span>
					</div>
				</div>

				<p class="text-xs text-gray-500">You can configure channels (Telegram, Discord, etc.) later in Settings.</p>

				{#if error}
					<p class="text-xs text-red-600 bg-red-50 p-2 rounded">{error}</p>
				{/if}

				<div class="flex gap-2">
					<button onclick={() => { step = 1 }}
						class="px-4 py-2.5 text-sm font-medium rounded-lg border border-gray-300 text-gray-700 hover:bg-gray-50 transition-colors">
						Back
					</button>
					<button onclick={finish} disabled={saving}
						class="flex-1 px-4 py-2.5 text-sm font-medium rounded-lg bg-gray-900 text-white hover:bg-gray-800 transition-colors disabled:opacity-50">
						{saving ? 'Starting...' : 'Start AmanClaw'}
					</button>
				</div>
			</div>
		{/if}
	</div>
</div>
```

**Step 2: Commit**

```bash
git add desktop/src/lib/pages/Wizard.svelte
git commit -m "feat(desktop): add first-run wizard page"
```

---

### Task 8: Wire Wizard into Page Router

**Files:**
- Modify: `desktop/src/routes/+page.svelte`
- Modify: `desktop/src/routes/+layout.svelte`

**Step 1: Update +page.svelte with first-run check and wizard**

Replace `desktop/src/routes/+page.svelte`:

```svelte
<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api';
	import { botStatus, currentPage, isFirstRun } from '$lib/stores/app';
	import Communities from '$lib/pages/Communities.svelte';
	import Skills from '$lib/pages/Skills.svelte';
	import Users from '$lib/pages/Users.svelte';
	import Settings from '$lib/pages/Settings.svelte';
	import Logs from '$lib/pages/Logs.svelte';
	import Content from '$lib/pages/Content.svelte';
	import Wizard from '$lib/pages/Wizard.svelte';

	let loaded = $state(false);

	onMount(async () => {
		try {
			const firstRun = await api.checkFirstRun();
			isFirstRun.set(firstRun);
			if (!firstRun) {
				const status = await api.getStatus();
				botStatus.set({ ...$botStatus, ...(status as any) });
			}
		} catch (e) {
			// Not connected yet
		}
		loaded = true;
	});

	async function handleStart() {
		try {
			await api.startEngine();
			const status = await api.getStatus();
			botStatus.set({ ...$botStatus, ...(status as any) });
		} catch (e: any) {
			botStatus.set({ ...$botStatus, engine_status: 'error', error: e?.toString() });
		}
	}

	async function handleStop() {
		try {
			await api.stopEngine();
			const status = await api.getStatus();
			botStatus.set({ ...$botStatus, ...(status as any) });
		} catch (e) {
			// ignore
		}
	}

	async function handleRestart() {
		try {
			botStatus.set({ ...$botStatus, engine_status: 'starting' });
			await api.restartEngine();
			const status = await api.getStatus();
			botStatus.set({ ...$botStatus, ...(status as any) });
		} catch (e: any) {
			botStatus.set({ ...$botStatus, engine_status: 'error', error: e?.toString() });
		}
	}
</script>

{#if !loaded}
	<div class="flex items-center justify-center h-full">
		<p class="text-sm text-gray-400">Loading...</p>
	</div>
{:else if $isFirstRun}
	<Wizard />
{:else if $currentPage === 'communities'}
	<Communities />
{:else if $currentPage === 'skills'}
	<Skills />
{:else if $currentPage === 'users'}
	<Users />
{:else if $currentPage === 'settings'}
	<Settings />
{:else if $currentPage === 'logs'}
	<Logs />
{:else if $currentPage === 'content'}
	<Content />
{:else}
	<!-- Dashboard -->
	<div class="p-8 max-w-4xl">
		<div class="mb-8">
			<h2 class="text-xl font-semibold text-gray-900 tracking-tight">Dashboard</h2>
			<p class="text-sm text-gray-500 mt-1">Overview of your AmanClaw instance</p>
		</div>

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

		<!-- Engine Control -->
		<div class="bg-gray-50 rounded-xl border border-gray-200 p-5 mb-4">
			<div class="flex items-center justify-between">
				<div class="flex items-center gap-3">
					<span class="w-3 h-3 rounded-full {
						$botStatus.engine_status === 'running' ? 'bg-green-500' :
						$botStatus.engine_status === 'starting' ? 'bg-yellow-500 animate-pulse' :
						$botStatus.engine_status === 'error' ? 'bg-red-500' :
						'bg-gray-400'
					}"></span>
					<div>
						<p class="text-sm font-medium text-gray-900">
							{$botStatus.engine_status === 'running' ? 'Engine Running' :
							 $botStatus.engine_status === 'starting' ? 'Engine Starting...' :
							 $botStatus.engine_status === 'error' ? 'Engine Error' :
							 'Engine Stopped'}
						</p>
						<p class="text-xs text-gray-500">
							{$botStatus.mode === 'local' ? 'Local Mode' : 'Remote Mode'}
							{#if $botStatus.uptime_seconds > 0}
								 · Uptime: {Math.floor($botStatus.uptime_seconds / 60)}m
							{/if}
						</p>
					</div>
				</div>
				<div class="flex gap-2">
					{#if $botStatus.engine_status === 'running'}
						<button onclick={handleRestart}
							class="px-3 py-1.5 text-xs font-medium rounded-md border border-gray-300 text-gray-700 hover:bg-gray-100 transition-colors">
							Restart
						</button>
						<button onclick={handleStop}
							class="px-3 py-1.5 text-xs font-medium rounded-md border border-red-300 text-red-700 hover:bg-red-50 transition-colors">
							Stop
						</button>
					{:else if $botStatus.engine_status !== 'starting'}
						<button onclick={handleStart}
							class="px-3 py-1.5 text-xs font-medium rounded-md bg-gray-900 text-white hover:bg-gray-800 transition-colors">
							Start
						</button>
					{/if}
				</div>
			</div>

			{#if $botStatus.error}
				<div class="mt-3 p-2 bg-red-50 rounded text-xs text-red-700">
					{$botStatus.error}
				</div>
			{/if}
		</div>
	</div>
{/if}
```

**Step 2: Update +layout.svelte to hide sidebar during wizard**

Read `desktop/src/routes/+layout.svelte` first. If it currently looks like:

```svelte
<script>
  import Sidebar from '$lib/components/Sidebar.svelte';
  import '../app.css';
</script>

<div class="flex h-screen">
  <Sidebar />
  <main class="flex-1 overflow-y-auto">
    <slot />
  </main>
</div>
```

Replace it with:

```svelte
<script>
	import Sidebar from '$lib/components/Sidebar.svelte';
	import { isFirstRun } from '$lib/stores/app';
	import '../app.css';
</script>

{#if $isFirstRun}
	<main class="h-screen overflow-y-auto bg-white">
		<slot />
	</main>
{:else}
	<div class="flex h-screen">
		<Sidebar />
		<main class="flex-1 overflow-y-auto">
			<slot />
		</main>
	</div>
{/if}
```

**Step 3: Commit**

```bash
git add desktop/src/routes/+page.svelte desktop/src/routes/+layout.svelte
git commit -m "feat(desktop): wire wizard into page router with first-run detection"
```

---

### Task 9: Expand Settings Page

**Files:**
- Modify: `desktop/src/lib/pages/Settings.svelte`

**Step 1: Replace Settings.svelte with full config UI**

Replace `desktop/src/lib/pages/Settings.svelte`:

```svelte
<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api';

	let mode = $state('local');
	let remoteUrl = $state('');
	let remoteToken = $state('');

	// LLM
	let llmBaseUrl = $state('');
	let llmModel = $state('');
	let llmApiKey = $state('');
	let maxTokens = $state(4096);
	let temperature = $state(0.7);

	// Channels
	let telegramToken = $state('');
	let discordToken = $state('');
	let slackBotToken = $state('');
	let slackAppToken = $state('');

	// Engine
	let rateLimit = $state(20);

	let saved = $state(false);
	let saving = $state(false);
	let dataDir = $state('');

	onMount(async () => {
		try {
			const m = await api.getMode() as string;
			if (m.startsWith('remote:')) {
				mode = 'remote';
				remoteUrl = m.replace('remote:', '');
			}

			const cfg = await api.getConfig() as any;
			if (cfg) {
				llmBaseUrl = cfg.llm?.base_url || '';
				llmModel = cfg.llm?.model || '';
				llmApiKey = cfg.llm?.api_key || '';
				maxTokens = cfg.llm?.max_tokens || 4096;
				temperature = cfg.llm?.temperature || 0.7;
				rateLimit = cfg.rate_limit_per_minute || 20;
				telegramToken = cfg.channels?.telegram || '';
				discordToken = cfg.channels?.discord || '';
				slackBotToken = cfg.channels?.slack_bot || '';
				slackAppToken = cfg.channels?.slack_app || '';
			}

			dataDir = await api.getDataDir();
		} catch (e) {
			// Not connected
		}
	});

	async function saveAll() {
		saving = true;
		try {
			await api.setMode(mode, remoteUrl, remoteToken);
			await api.saveConfig({
				llm_base_url: llmBaseUrl,
				llm_model: llmModel,
				llm_api_key: llmApiKey,
				max_tokens: maxTokens,
				temperature: temperature,
				rate_limit: rateLimit,
				telegram_token: telegramToken || undefined,
				discord_token: discordToken || undefined,
				slack_bot_token: slackBotToken || undefined,
				slack_app_token: slackAppToken || undefined,
			});
			saved = true;
			setTimeout(() => saved = false, 2000);
		} catch (e) {
			// Handle error
		} finally {
			saving = false;
		}
	}
</script>

<div class="p-8 max-w-2xl">
	<div class="mb-8">
		<h2 class="text-xl font-semibold text-gray-900 tracking-tight">Settings</h2>
		<p class="text-sm text-gray-500 mt-1">Configure your AmanClaw instance</p>
	</div>

	<!-- LLM Config -->
	<section class="mb-8">
		<h3 class="text-sm font-medium text-gray-900 mb-3">LLM Configuration</h3>
		<div class="space-y-3">
			<div>
				<label for="s-base-url" class="block text-xs font-medium text-gray-700 mb-1">Base URL</label>
				<input id="s-base-url" type="text" bind:value={llmBaseUrl} placeholder="http://localhost:11434/v1"
					class="w-full px-3 py-2 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-gray-900 focus:border-transparent">
			</div>
			<div class="grid grid-cols-2 gap-3">
				<div>
					<label for="s-model" class="block text-xs font-medium text-gray-700 mb-1">Model</label>
					<input id="s-model" type="text" bind:value={llmModel} placeholder="qwen3:8b"
						class="w-full px-3 py-2 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-gray-900 focus:border-transparent">
				</div>
				<div>
					<label for="s-api-key" class="block text-xs font-medium text-gray-700 mb-1">API Key</label>
					<input id="s-api-key" type="password" bind:value={llmApiKey} placeholder="Optional"
						class="w-full px-3 py-2 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-gray-900 focus:border-transparent">
				</div>
			</div>
			<div class="grid grid-cols-2 gap-3">
				<div>
					<label for="s-max-tokens" class="block text-xs font-medium text-gray-700 mb-1">Max Tokens</label>
					<input id="s-max-tokens" type="number" bind:value={maxTokens}
						class="w-full px-3 py-2 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-gray-900 focus:border-transparent">
				</div>
				<div>
					<label for="s-temperature" class="block text-xs font-medium text-gray-700 mb-1">Temperature</label>
					<input id="s-temperature" type="number" step="0.1" min="0" max="2" bind:value={temperature}
						class="w-full px-3 py-2 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-gray-900 focus:border-transparent">
				</div>
			</div>
		</div>
	</section>

	<!-- Channels -->
	<section class="mb-8 border-t border-gray-200 pt-6">
		<h3 class="text-sm font-medium text-gray-900 mb-3">Channel Tokens</h3>
		<p class="text-xs text-gray-500 mb-3">Leave empty to disable a channel. Restart engine after changes.</p>
		<div class="space-y-3">
			<div>
				<label for="s-telegram" class="block text-xs font-medium text-gray-700 mb-1">Telegram Bot Token</label>
				<input id="s-telegram" type="password" bind:value={telegramToken} placeholder="123456:ABC-DEF..."
					class="w-full px-3 py-2 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-gray-900 focus:border-transparent">
			</div>
			<div>
				<label for="s-discord" class="block text-xs font-medium text-gray-700 mb-1">Discord Bot Token</label>
				<input id="s-discord" type="password" bind:value={discordToken} placeholder="MTIz..."
					class="w-full px-3 py-2 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-gray-900 focus:border-transparent">
			</div>
			<div class="grid grid-cols-2 gap-3">
				<div>
					<label for="s-slack-bot" class="block text-xs font-medium text-gray-700 mb-1">Slack Bot Token</label>
					<input id="s-slack-bot" type="password" bind:value={slackBotToken} placeholder="xoxb-..."
						class="w-full px-3 py-2 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-gray-900 focus:border-transparent">
				</div>
				<div>
					<label for="s-slack-app" class="block text-xs font-medium text-gray-700 mb-1">Slack App Token</label>
					<input id="s-slack-app" type="password" bind:value={slackAppToken} placeholder="xapp-..."
						class="w-full px-3 py-2 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-gray-900 focus:border-transparent">
				</div>
			</div>
		</div>
	</section>

	<!-- Engine -->
	<section class="mb-8 border-t border-gray-200 pt-6">
		<h3 class="text-sm font-medium text-gray-900 mb-3">Engine</h3>
		<div>
			<label for="s-rate-limit" class="block text-xs font-medium text-gray-700 mb-1">Rate Limit (per minute per user)</label>
			<input id="s-rate-limit" type="number" bind:value={rateLimit} min="1" max="100"
				class="w-40 px-3 py-2 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-gray-900 focus:border-transparent">
		</div>
	</section>

	<!-- Connection Mode -->
	<section class="mb-8 border-t border-gray-200 pt-6">
		<h3 class="text-sm font-medium text-gray-900 mb-3">Connection Mode</h3>
		<div class="space-y-2">
			<label class="flex items-center gap-3 p-3 rounded-lg border border-gray-200 cursor-pointer hover:bg-gray-50 transition-colors
				{mode === 'local' ? 'border-gray-900 bg-gray-50' : ''}">
				<input type="radio" bind:group={mode} value="local" class="accent-gray-900">
				<div>
					<p class="text-sm font-medium text-gray-900">Local Mode</p>
					<p class="text-xs text-gray-500">Bot engine runs in this app</p>
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
					<label for="s-remote-url" class="block text-xs font-medium text-gray-700 mb-1">Server URL</label>
					<input id="s-remote-url" type="text" bind:value={remoteUrl} placeholder="https://your-server.com"
						class="w-full px-3 py-2 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-gray-900 focus:border-transparent">
				</div>
				<div>
					<label for="s-remote-token" class="block text-xs font-medium text-gray-700 mb-1">API Token</label>
					<input id="s-remote-token" type="password" bind:value={remoteToken} placeholder="Bearer token"
						class="w-full px-3 py-2 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-gray-900 focus:border-transparent">
				</div>
			</div>
		{/if}
	</section>

	<!-- Save -->
	<div class="flex items-center gap-3">
		<button onclick={saveAll} disabled={saving}
			class="px-4 py-2 text-xs font-medium rounded-md bg-gray-900 text-white hover:bg-gray-800 transition-colors disabled:opacity-50">
			{saving ? 'Saving...' : saved ? 'Saved!' : 'Save Settings'}
		</button>
		<p class="text-xs text-gray-400">Restart engine after changing LLM or channel settings</p>
	</div>

	<!-- Data Dir -->
	<section class="mt-8 border-t border-gray-200 pt-6">
		<h3 class="text-sm font-medium text-gray-900 mb-2">Data</h3>
		<p class="text-xs text-gray-500">Config, database, and plugins stored at:</p>
		<p class="text-xs text-gray-700 font-mono mt-1 bg-gray-50 p-2 rounded">{dataDir}</p>
	</section>

	<div class="border-t border-gray-200 pt-6 mt-6">
		<p class="text-xs text-gray-500">AmanClaw Desktop v0.1.0 · Built in Malaysia</p>
	</div>
</div>
```

**Step 2: Commit**

```bash
git add desktop/src/lib/pages/Settings.svelte
git commit -m "feat(desktop): expand Settings with LLM, channels, engine config"
```

---

### Task 10: Update Sidebar Status Indicator

**Files:**
- Modify: `desktop/src/lib/components/Sidebar.svelte`

**Step 1: Make sidebar status reactive to engine state**

Replace `desktop/src/lib/components/Sidebar.svelte`:

```svelte
<script lang="ts">
	import { currentPage, botStatus } from '$lib/stores/app';

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

	const statusColor = $derived(
		$botStatus.engine_status === 'running' ? 'bg-green-500' :
		$botStatus.engine_status === 'starting' ? 'bg-yellow-500 animate-pulse' :
		$botStatus.engine_status === 'error' ? 'bg-red-500' :
		'bg-gray-400'
	);

	const statusText = $derived(
		$botStatus.engine_status === 'running' ? 'Engine Running' :
		$botStatus.engine_status === 'starting' ? 'Starting...' :
		$botStatus.engine_status === 'error' ? 'Engine Error' :
		'Engine Stopped'
	);
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
					onclick={() => currentPage.set(page.id)}
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
					onclick={() => currentPage.set(page.id)}
				>
					<span class="text-base leading-none">{page.icon}</span>
					{page.label}
				</button>
			{/each}
		</div>
		<div class="mx-2 p-2.5 bg-white rounded-lg border border-gray-200 shadow-sm">
			<div class="flex items-center gap-2">
				<span class="w-2 h-2 rounded-full {statusColor}"></span>
				<span class="text-[11px] font-medium text-gray-700">{statusText}</span>
			</div>
		</div>
	</div>
</aside>
```

**Step 2: Commit**

```bash
git add desktop/src/lib/components/Sidebar.svelte
git commit -m "feat(desktop): make sidebar status indicator reactive to engine state"
```

---

### Task 11: Update CSP for Tauri Config

**Files:**
- Modify: `desktop/src-tauri/tauri.conf.json`

**Step 1: Relax CSP to allow connections to LLM APIs**

The engine makes HTTP calls to LLM APIs from the Rust backend (not the webview), so no CSP change is actually needed. However, we should verify the config has the correct `identifier` for app data dir resolution.

Check that `desktop/src-tauri/tauri.conf.json` has a valid `identifier`. The current `my.amanclaw.desktop` is fine. No changes needed.

**Step 2: Commit (skip if no changes)**

No commit needed for this task.

---

### Task 12: Build and Verify

**Step 1: Verify Rust compilation**

Run: `cd desktop && cargo check -p amanclaw-desktop 2>&1 | tail -20`
Expected: no errors (warnings OK about unused functions)

Fix any compilation errors before proceeding. Common issues:
- Missing `use` imports in commands.rs
- Type mismatches between Engine API and our code
- The `iter_skills()` method might return different types — check and adapt
- The `Auth::list_users()` might have a different signature — check and adapt

**Step 2: Verify frontend builds**

Run: `cd desktop && npm run build 2>&1 | tail -10`
Expected: no errors

**Step 3: Run the app**

Run: `cd desktop && cargo tauri dev`
Expected: App launches. If no config.yaml exists in app data dir, wizard shows. If config exists, dashboard shows with engine status.

**Step 4: Commit any fixes**

```bash
git add -A desktop/
git commit -m "fix(desktop): resolve compilation issues for embedded engine"
```

---

### Task 13: Final Integration Test

**Step 1: Test first-run wizard**

1. Delete any existing config: find the app data dir (shown in Settings page) and delete `config.yaml`
2. Launch app → wizard should appear
3. Enter LLM URL (e.g., `http://localhost:11434/v1`), model (`qwen3:8b`)
4. Click Next → review → click "Start AmanClaw"
5. Should redirect to Dashboard with engine status "Running" (or "Error" if no LLM is available — that's OK)

**Step 2: Test Settings page**

1. Navigate to Settings
2. All fields should be populated from saved config
3. Add a Telegram token, click Save
4. Click Restart on Dashboard
5. Engine should restart with new config

**Step 3: Test Stop/Start**

1. On Dashboard, click Stop → status changes to "Stopped"
2. Click Start → status changes to "Running"

**Step 4: Commit**

```bash
git add -A
git commit -m "feat(desktop): embedded engine integration complete"
```
