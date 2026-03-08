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

/// Set environment variables from secrets. MUST be called before Engine::new().
/// SAFETY: This is unsafe in Rust 2024 edition because env vars are process-global.
/// We call this before spawning the engine, so no concurrent reads should occur.
fn apply_env_vars(secrets: &HashMap<String, String>, db_path: &str) {
    unsafe {
        for (k, v) in secrets {
            std::env::set_var(k, v);
        }
        std::env::set_var("MEMORY_DB_PATH", db_path);
    }
}

/// Public version for use by lib.rs auto-start.
pub fn apply_env_vars_public(secrets: &HashMap<String, String>, db_path: &str) {
    apply_env_vars(secrets, db_path);
}

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
    apply_env_vars(&secrets, &db_path.to_string_lossy());

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
            // engine.run() returns Ok when no channels are active (rx closed).
            // Keep status as Running — the engine is initialized and handles are valid.
            tracing::info!("Engine run loop exited (no active channels)");
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

// --- User actions ---

#[tauri::command]
pub async fn approve_user(
    app: AppHandle,
    state: State<'_, SharedState>,
    user_id: String,
    platform: String,
) -> Result<(), String> {
    let st = state.read().await;
    if let Some(handle) = &st.engine_handle {
        let mut auth = handle.auth.lock().unwrap();
        auth.approve_user(&user_id, &platform);
    } else {
        return Err("Engine not running".into());
    }

    // Persist to config.yaml so user survives restarts
    if let Ok(mut cfg) = config::load_config(&app) {
        let users = cfg.admin_users.entry(platform).or_insert_with(Vec::new);
        if !users.contains(&user_id) {
            users.push(user_id);
        }
        let _ = config::save_config(&app, &cfg);
    }

    Ok(())
}

#[tauri::command]
pub async fn block_user(
    app: AppHandle,
    state: State<'_, SharedState>,
    user_id: String,
    platform: String,
) -> Result<(), String> {
    let st = state.read().await;
    if let Some(handle) = &st.engine_handle {
        let mut auth = handle.auth.lock().unwrap();
        auth.block_user(&user_id, &platform);
    } else {
        return Err("Engine not running".into());
    }

    // Remove from admin_users in config so block persists across restarts
    if let Ok(mut cfg) = config::load_config(&app) {
        if let Some(users) = cfg.admin_users.get_mut(&platform) {
            users.retain(|id| id != &user_id);
        }
        let _ = config::save_config(&app, &cfg);
    }

    Ok(())
}

// --- Data dir ---

#[tauri::command]
pub async fn get_data_dir(app: AppHandle) -> Result<String, String> {
    let dir = config::data_dir(&app)?;
    Ok(dir.to_string_lossy().to_string())
}

// --- Logs ---

#[tauri::command]
pub async fn get_logs(
    state: State<'_, SharedState>,
) -> Result<serde_json::Value, String> {
    let st = state.read().await;
    let logs: Vec<serde_json::Value> = st.logs.iter().map(|e| {
        serde_json::json!({
            "timestamp": e.timestamp,
            "level": e.level,
            "target": e.target,
            "message": e.message,
        })
    }).collect();
    Ok(serde_json::json!(logs))
}
