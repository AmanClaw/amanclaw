use crate::config;
use crate::state::{AppMode, AppState, EngineHandle, EngineStatus};
use amanclaw_traits::config::{AppConfig, LlmConfig, McpServerConfig};
use std::collections::HashMap;
use std::sync::Arc;
use sqlx::Row;
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
            wasm_memory_limit_mb: 64,
            wasm_fuel_limit: 1_000_000_000,
        },
        security: Default::default(),
        skills: Default::default(),
        mcp_servers: HashMap::new(),
        script_plugins: HashMap::new(),
        agents: HashMap::new(),
        routing: Default::default(),
        embeddings: None,
        vector: None,
        knowledge_bases: HashMap::new(),
        cron: Default::default(),
        webhooks: Default::default(),
        gateway: Default::default(),
        subagents: Default::default(),
        registry: Default::default(),
        channels: Default::default(),
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

    // Initialize and start engine actor
    let result = amanclaw_core::Engine::start(cfg.clone())
        .await
        .map_err(|e| format!("Engine init failed: {}", e))?;

    // Grab handles from the start result
    let engine_handle = result.handle.clone();
    let auth = result.auth.clone();
    let pool = result.pool.clone();
    let registry = result.registry.clone();
    let channel_manager = result.channel_manager.clone();
    let channels_config = result.channels_config.clone();

    // Spawn a wrapper task that monitors the engine actor
    let state_clone = state.inner().clone();
    let join_handle = tokio::spawn(async move {
        match result.join.await {
            Ok(Ok(())) => {
                tracing::info!("Engine run loop exited (no active channels)");
            }
            Ok(Err(e)) => {
                let mut st = state_clone.write().await;
                st.engine_status = EngineStatus::Error(e.to_string());
                tracing::error!(error = %e, "Engine error");
            }
            Err(e) => {
                let mut st = state_clone.write().await;
                st.engine_status = EngineStatus::Error(format!("Engine task panicked: {}", e));
                tracing::error!(error = %e, "Engine task panicked");
            }
        }
    });

    // Store handle
    {
        let mut st = state.write().await;
        st.engine_status = EngineStatus::Running;
        st.config = Some(cfg);
        st.started_at = Some(std::time::Instant::now());
        st.engine_handle = Some(EngineHandle {
            engine_handle,
            join_handle,
            auth,
            pool,
            registry,
            subagent_manager: None,
            channel_manager: Some(channel_manager),
            channels_config: Some(channels_config),
        });
    }

    Ok(())
}

#[tauri::command]
pub async fn stop_engine(
    state: State<'_, SharedState>,
) -> Result<(), String> {
    let handle = {
        let mut st = state.write().await;
        st.engine_handle.take()
    };
    if let Some(handle) = handle {
        handle.engine_handle.shutdown().await.map_err(|e| e.to_string())?;
        let mut st = state.write().await;
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
        let handle = {
            let mut st = state.write().await;
            st.engine_handle.take()
        };
        if let Some(handle) = handle {
            let _ = handle.engine_handle.shutdown().await;
            let mut st = state.write().await;
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

    // Prefer real-time status from core handle when available
    let status: EngineStatus = if let Some(ref handle) = st.engine_handle {
        handle.engine_handle.status().into()
    } else {
        st.engine_status.clone()
    };

    let uptime_secs = st.started_at.map(|t| t.elapsed().as_secs()).unwrap_or(0);

    let (status_str, error_msg) = match &status {
        EngineStatus::Stopped => ("stopped", None),
        EngineStatus::Starting => ("starting", None),
        EngineStatus::Running => ("running", None),
        EngineStatus::Error(e) => ("error", Some(e.clone())),
    };

    Ok(serde_json::json!({
        "engine_status": status_str,
        "bot_running": matches!(status, EngineStatus::Running),
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
                        let source = if meta.version == "mcp" {
                            "mcp"
                        } else if name.contains("__") {
                            "mcp"
                        } else {
                            "builtin"
                        };
                        serde_json::json!({
                            "name": name,
                            "description": meta.description,
                            "version": meta.version,
                            "timeout_ms": meta.timeout_ms,
                            "source": source,
                            "parameters": skill.parameters_schema(),
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
    platform: Option<String>,
    status: Option<String>,
    search: Option<String>,
) -> Result<serde_json::Value, String> {
    let st = state.read().await;
    match &st.mode {
        AppMode::Remote { url, token } => {
            let client = reqwest::Client::new();
            let mut req_url = format!("{}/api/users", url);
            let mut params = Vec::new();
            if let Some(p) = &platform { params.push(format!("platform={p}")); }
            if let Some(s) = &status { params.push(format!("status={s}")); }
            if let Some(q) = &search { params.push(format!("search={q}")); }
            if !params.is_empty() { req_url.push_str(&format!("?{}", params.join("&"))); }
            let resp = client.get(&req_url)
                .bearer_auth(token)
                .send().await.map_err(|e| e.to_string())?;
            resp.json().await.map_err(|e| e.to_string())
        }
        AppMode::Local => {
            if let Some(handle) = &st.engine_handle {
                let pool = &handle.pool;
                let mut sql = "SELECT user_id, platform, state, username, first_name, first_seen, last_seen FROM users WHERE 1=1".to_string();
                let mut binds: Vec<String> = Vec::new();

                if let Some(p) = &platform {
                    sql.push_str(" AND platform = ?");
                    binds.push(p.clone());
                }
                if let Some(s) = &status {
                    sql.push_str(" AND state = ?");
                    binds.push(s.clone());
                }
                if let Some(q) = &search {
                    sql.push_str(" AND (user_id LIKE ? OR username LIKE ? OR first_name LIKE ?)");
                    let pattern = format!("%{q}%");
                    binds.push(pattern.clone());
                    binds.push(pattern.clone());
                    binds.push(pattern);
                }
                sql.push_str(" ORDER BY last_seen DESC");

                let mut query = sqlx::query(&sql);
                for b in &binds {
                    query = query.bind(b);
                }
                let rows = query.fetch_all(pool).await.map_err(|e| e.to_string())?;

                // Build set of admin user IDs for quick lookup
                let auth = handle.auth.read().await;
                let admin_map = auth.admin_users();
                let mut admin_set: std::collections::HashSet<(String, String)> = std::collections::HashSet::new();
                for (plat, ids) in admin_map {
                    for id in ids {
                        admin_set.insert((id.clone(), plat.clone()));
                    }
                }

                let mut user_list: Vec<serde_json::Value> = rows.iter()
                    .map(|r| {
                        let uid: String = r.get("user_id");
                        let plat: String = r.get("platform");
                        let db_state: String = r.get("state");
                        let is_admin = admin_set.contains(&(uid.clone(), plat.clone()));
                        let effective_state = if is_admin { "admin".to_string() } else { db_state };
                        serde_json::json!({
                            "user_id": uid,
                            "platform": plat,
                            "state": effective_state,
                            "username": r.get::<Option<String>, _>("username"),
                            "first_name": r.get::<Option<String>, _>("first_name"),
                            "first_seen": r.get::<Option<String>, _>("first_seen"),
                            "last_seen": r.get::<Option<String>, _>("last_seen"),
                        })
                    })
                    .collect();

                // Add admin users that aren't in the DB yet
                let db_keys: std::collections::HashSet<(String, String)> = rows.iter()
                    .map(|r| (r.get::<String, _>("user_id"), r.get::<String, _>("platform")))
                    .collect();
                let skip_admin = status.as_deref().is_some_and(|s| s != "admin");
                if !skip_admin {
                    for (plat, ids) in admin_map {
                        if platform.as_ref().is_some_and(|p| p != plat) { continue; }
                        for uid in ids {
                            if db_keys.contains(&(uid.clone(), plat.clone())) { continue; }
                            if let Some(q) = &search {
                                let q = q.to_lowercase();
                                if !uid.to_lowercase().contains(&q) { continue; }
                            }
                            user_list.push(serde_json::json!({
                                "user_id": uid,
                                "platform": plat,
                                "state": "admin",
                                "username": null,
                                "first_name": null,
                                "first_seen": null,
                                "last_seen": null,
                            }));
                        }
                    }
                }

                // If filtering by admin status, keep only admins
                if status.as_deref() == Some("admin") {
                    user_list.retain(|u| u["state"] == "admin");
                }

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
        let mut auth = handle.auth.write().await;
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
        let mut auth = handle.auth.write().await;
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

#[tauri::command]
pub async fn unblock_user(
    state: State<'_, SharedState>,
    user_id: String,
    platform: String,
) -> Result<(), String> {
    let st = state.read().await;
    match &st.mode {
        AppMode::Remote { url, token } => {
            let client = reqwest::Client::new();
            client.put(format!("{}/api/users/{}/{}/unblock", url, platform, user_id))
                .bearer_auth(token)
                .send().await.map_err(|e| e.to_string())?;
            Ok(())
        }
        AppMode::Local => {
            if let Some(handle) = &st.engine_handle {
                let mut auth = handle.auth.write().await;
                auth.unblock_user(&user_id, &platform);
                Ok(())
            } else {
                Err("Engine not running".into())
            }
        }
    }
}

#[tauri::command]
pub async fn add_user(
    state: State<'_, SharedState>,
    user_id: String,
    platform: String,
    username: Option<String>,
    first_name: Option<String>,
    status: Option<String>,
) -> Result<(), String> {
    let initial_state = status.as_deref().unwrap_or("approved");
    let st = state.read().await;
    match &st.mode {
        AppMode::Remote { url, token } => {
            // Remote mode: POST to API (if supported), otherwise error
            let client = reqwest::Client::new();
            client.post(format!("{}/api/users", url))
                .bearer_auth(token)
                .json(&serde_json::json!({
                    "user_id": user_id,
                    "platform": platform,
                    "username": username,
                    "first_name": first_name,
                    "state": initial_state,
                }))
                .send().await.map_err(|e| e.to_string())?;
            Ok(())
        }
        AppMode::Local => {
            if let Some(handle) = &st.engine_handle {
                let pool = &handle.pool;
                sqlx::query(
                    "INSERT INTO users (user_id, platform, state, username, first_name)
                     VALUES (?, ?, ?, ?, ?)
                     ON CONFLICT(user_id, platform) DO UPDATE SET
                       username = COALESCE(excluded.username, users.username),
                       first_name = COALESCE(excluded.first_name, users.first_name),
                       state = excluded.state,
                       last_seen = CURRENT_TIMESTAMP",
                )
                .bind(&user_id)
                .bind(&platform)
                .bind(initial_state)
                .bind(&username)
                .bind(&first_name)
                .execute(pool)
                .await
                .map_err(|e| e.to_string())?;

                // Also update the in-memory Auth cache
                let mut auth = handle.auth.write().await;
                auth.register_user(&user_id, &platform, username.as_deref(), first_name.as_deref());
                match initial_state {
                    "approved" => auth.approve_user(&user_id, &platform),
                    "blocked" => auth.block_user(&user_id, &platform),
                    _ => {}
                }
                Ok(())
            } else {
                Err("Engine not running".into())
            }
        }
    }
}

#[tauri::command]
pub async fn make_admin(
    app: AppHandle,
    state: State<'_, SharedState>,
    user_id: String,
    platform: String,
) -> Result<(), String> {
    let st = state.read().await;
    if let Some(handle) = &st.engine_handle {
        let mut auth = handle.auth.write().await;
        auth.make_admin(&user_id, &platform);
    }

    // Persist to config.yaml
    if let Ok(mut cfg) = config::load_config(&app) {
        let users = cfg.admin_users.entry(platform).or_default();
        if !users.contains(&user_id) {
            users.push(user_id);
        }
        let _ = config::save_config(&app, &cfg);
    }
    Ok(())
}

#[tauri::command]
pub async fn remove_admin(
    app: AppHandle,
    state: State<'_, SharedState>,
    user_id: String,
    platform: String,
) -> Result<(), String> {
    let st = state.read().await;
    if let Some(handle) = &st.engine_handle {
        let mut auth = handle.auth.write().await;
        auth.remove_admin(&user_id, &platform);
    }

    // Remove from config.yaml
    if let Ok(mut cfg) = config::load_config(&app) {
        if let Some(users) = cfg.admin_users.get_mut(&platform) {
            users.retain(|id| id != &user_id);
        }
        let _ = config::save_config(&app, &cfg);
    }
    Ok(())
}

#[tauri::command]
pub async fn get_user_detail(
    state: State<'_, SharedState>,
    user_id: String,
    platform: String,
) -> Result<serde_json::Value, String> {
    let st = state.read().await;
    match &st.mode {
        AppMode::Remote { url, token } => {
            let client = reqwest::Client::new();
            let resp = client.get(format!("{}/api/users/{}/{}", url, platform, user_id))
                .bearer_auth(token)
                .send().await.map_err(|e| e.to_string())?;
            resp.json().await.map_err(|e| e.to_string())
        }
        AppMode::Local => {
            if let Some(handle) = &st.engine_handle {
                let pool = &handle.pool;

                // Get user row
                let row = sqlx::query(
                    "SELECT user_id, platform, state, username, first_name, first_seen, last_seen FROM users WHERE user_id = ? AND platform = ?",
                )
                .bind(&user_id)
                .bind(&platform)
                .fetch_optional(pool)
                .await
                .map_err(|e| e.to_string())?;

                // Check admin status
                let auth = handle.auth.read().await;
                let is_admin = auth.admin_users()
                    .get(&platform)
                    .map(|ids| ids.iter().any(|id| id == &user_id))
                    .unwrap_or(false);

                // Admin-only user (not in DB) — return minimal info
                let Some(row) = row else {
                    if is_admin {
                        return Ok(serde_json::json!({
                            "user_id": user_id,
                            "platform": platform,
                            "state": "admin",
                            "username": null,
                            "first_name": null,
                            "first_seen": null,
                            "last_seen": null,
                            "facts": {},
                            "message_count": 0,
                        }));
                    }
                    return Err("User not found".into());
                };

                // Get facts
                let fact_rows = sqlx::query("SELECT key, value FROM facts WHERE user_id = ?")
                    .bind(&user_id)
                    .fetch_all(pool)
                    .await
                    .unwrap_or_default();
                let facts: serde_json::Map<String, serde_json::Value> = fact_rows.iter()
                    .map(|r| (r.get::<String, _>("key"), serde_json::Value::String(r.get::<String, _>("value"))))
                    .collect();

                // Get message count
                let msg_count: i64 = sqlx::query("SELECT COUNT(*) as c FROM messages WHERE user_id = ?")
                    .bind(&user_id)
                    .fetch_one(pool)
                    .await
                    .map(|r| r.get("c"))
                    .unwrap_or(0);

                let effective_state = if is_admin { "admin" } else { &row.get::<String, _>("state") };

                Ok(serde_json::json!({
                    "user_id": row.get::<String, _>("user_id"),
                    "platform": row.get::<String, _>("platform"),
                    "state": effective_state,
                    "username": row.get::<Option<String>, _>("username"),
                    "first_name": row.get::<Option<String>, _>("first_name"),
                    "first_seen": row.get::<Option<String>, _>("first_seen"),
                    "last_seen": row.get::<Option<String>, _>("last_seen"),
                    "facts": facts,
                    "message_count": msg_count,
                }))
            } else {
                Err("Engine not running".into())
            }
        }
    }
}

#[tauri::command]
pub async fn get_user_history(
    state: State<'_, SharedState>,
    user_id: String,
    platform: String,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<serde_json::Value, String> {
    let limit = limit.unwrap_or(20);
    let offset = offset.unwrap_or(0);
    let st = state.read().await;
    match &st.mode {
        AppMode::Remote { url, token } => {
            let client = reqwest::Client::new();
            let resp = client.get(format!(
                "{}/api/users/{}/{}/history?limit={}&offset={}",
                url, platform, user_id, limit, offset
            ))
                .bearer_auth(token)
                .send().await.map_err(|e| e.to_string())?;
            resp.json().await.map_err(|e| e.to_string())
        }
        AppMode::Local => {
            if let Some(handle) = &st.engine_handle {
                let pool = &handle.pool;

                let total: i64 = sqlx::query("SELECT COUNT(*) as c FROM messages WHERE user_id = ?")
                    .bind(&user_id)
                    .fetch_one(pool)
                    .await
                    .map(|r| r.get("c"))
                    .unwrap_or(0);

                let rows = sqlx::query(
                    "SELECT role, content FROM messages WHERE user_id = ? ORDER BY id DESC LIMIT ? OFFSET ?",
                )
                .bind(&user_id)
                .bind(limit)
                .bind(offset)
                .fetch_all(pool)
                .await
                .map_err(|e| e.to_string())?;

                let mut messages: Vec<serde_json::Value> = rows.iter()
                    .map(|r| serde_json::json!({
                        "role": r.get::<String, _>("role"),
                        "content": r.get::<String, _>("content"),
                    }))
                    .collect();
                messages.reverse();

                Ok(serde_json::json!({
                    "messages": messages,
                    "total": total,
                }))
            } else {
                Ok(serde_json::json!({ "messages": [], "total": 0 }))
            }
        }
    }
}

#[tauri::command]
pub async fn get_user_stats(
    state: State<'_, SharedState>,
) -> Result<serde_json::Value, String> {
    let st = state.read().await;
    match &st.mode {
        AppMode::Remote { url, token } => {
            let client = reqwest::Client::new();
            let resp = client.get(format!("{}/api/stats", url))
                .bearer_auth(token)
                .send().await.map_err(|e| e.to_string())?;
            resp.json().await.map_err(|e| e.to_string())
        }
        AppMode::Local => {
            if let Some(handle) = &st.engine_handle {
                let pool = &handle.pool;

                let total: i64 = sqlx::query("SELECT COUNT(*) as c FROM users")
                    .fetch_one(pool).await.map(|r| r.get("c")).unwrap_or(0);
                let pending: i64 = sqlx::query("SELECT COUNT(*) as c FROM users WHERE state = 'pending'")
                    .fetch_one(pool).await.map(|r| r.get("c")).unwrap_or(0);
                let approved: i64 = sqlx::query("SELECT COUNT(*) as c FROM users WHERE state = 'approved'")
                    .fetch_one(pool).await.map(|r| r.get("c")).unwrap_or(0);
                let blocked: i64 = sqlx::query("SELECT COUNT(*) as c FROM users WHERE state = 'blocked'")
                    .fetch_one(pool).await.map(|r| r.get("c")).unwrap_or(0);

                // Count admin users from config
                let auth = handle.auth.read().await;
                let admin_count: i64 = auth.admin_users().values()
                    .map(|ids| ids.len() as i64).sum();

                let platform_rows = sqlx::query("SELECT platform, COUNT(*) as c FROM users GROUP BY platform")
                    .fetch_all(pool).await.unwrap_or_default();
                let by_platform: serde_json::Map<String, serde_json::Value> = platform_rows.iter()
                    .map(|r| (r.get::<String, _>("platform"), serde_json::json!(r.get::<i64, _>("c"))))
                    .collect();

                Ok(serde_json::json!({
                    "total": total + admin_count,
                    "admin": admin_count,
                    "pending": pending,
                    "approved": approved,
                    "blocked": blocked,
                    "by_platform": by_platform,
                }))
            } else {
                Ok(serde_json::json!({ "total": 0, "admin": 0, "pending": 0, "approved": 0, "blocked": 0, "by_platform": {} }))
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

// --- MCP Servers ---

#[tauri::command]
pub async fn get_mcp_servers(
    app: AppHandle,
) -> Result<serde_json::Value, String> {
    if !config::has_config(&app) {
        return Ok(serde_json::json!({ "servers": {} }));
    }
    let cfg = config::load_config(&app)?;
    let servers: serde_json::Map<String, serde_json::Value> = cfg.mcp_servers.iter()
        .map(|(name, sc)| {
            let transport = if sc.url.is_some() { "http" } else { "stdio" };
            (name.clone(), serde_json::json!({
                "command": sc.command,
                "args": sc.args,
                "env": sc.env,
                "url": sc.url,
                "transport": transport,
            }))
        })
        .collect();
    Ok(serde_json::json!({ "servers": servers }))
}

#[tauri::command]
pub async fn save_mcp_server(
    app: AppHandle,
    state: State<'_, SharedState>,
    name: String,
    command: Option<String>,
    args: Option<Vec<String>>,
    env: Option<HashMap<String, String>>,
    url: Option<String>,
) -> Result<(), String> {
    let mut cfg = config::load_config(&app)?;
    cfg.mcp_servers.insert(name, McpServerConfig {
        command,
        args: args.unwrap_or_default(),
        env: env.unwrap_or_default(),
        url,
    });
    config::save_config(&app, &cfg)?;

    // Update in-memory config
    let mut st = state.write().await;
    st.config = Some(cfg);
    Ok(())
}

#[tauri::command]
pub async fn delete_mcp_server(
    app: AppHandle,
    state: State<'_, SharedState>,
    name: String,
) -> Result<(), String> {
    let mut cfg = config::load_config(&app)?;
    cfg.mcp_servers.remove(&name);
    config::save_config(&app, &cfg)?;

    let mut st = state.write().await;
    st.config = Some(cfg);
    Ok(())
}

#[tauri::command]
pub async fn disable_skill(
    app: AppHandle,
    state: State<'_, SharedState>,
    name: String,
) -> Result<(), String> {
    let mut cfg = config::load_config(&app)?;
    if !cfg.skills.disabled.contains(&name) {
        cfg.skills.disabled.push(name);
    }
    config::save_config(&app, &cfg)?;
    let mut st = state.write().await;
    st.config = Some(cfg);
    Ok(())
}

#[tauri::command]
pub async fn enable_skill(
    app: AppHandle,
    state: State<'_, SharedState>,
    name: String,
) -> Result<(), String> {
    let mut cfg = config::load_config(&app)?;
    cfg.skills.disabled.retain(|n| n != &name);
    config::save_config(&app, &cfg)?;
    let mut st = state.write().await;
    st.config = Some(cfg);
    Ok(())
}

#[tauri::command]
pub async fn get_disabled_skills(
    app: AppHandle,
) -> Result<Vec<String>, String> {
    if !config::has_config(&app) {
        return Ok(vec![]);
    }
    let cfg = config::load_config(&app)?;
    Ok(cfg.skills.disabled)
}

// --- Agents ---

#[tauri::command]
pub async fn list_agents(
    app: AppHandle,
) -> Result<serde_json::Value, String> {
    let cfg = config::load_config(&app)?;
    let agents: Vec<serde_json::Value> = cfg.agents.iter().map(|(id, profile)| {
        serde_json::json!({
            "id": id,
            "name": profile.name,
            "system_prompt": profile.system_prompt,
            "soul_file": profile.soul_file,
            "allowed_skills": profile.allowed_skills,
            "memory_namespace": profile.memory_namespace,
        })
    }).collect();
    Ok(serde_json::json!({ "agents": agents, "count": agents.len() }))
}

#[tauri::command]
pub async fn save_agent(
    app: AppHandle,
    id: String,
    name: String,
    system_prompt: String,
    soul_file: Option<String>,
    allowed_skills: Vec<String>,
    memory_namespace: String,
) -> Result<(), String> {
    let mut cfg = config::load_config(&app)?;
    let profile = amanclaw_traits::agent::AgentProfile {
        id: id.clone(),
        name,
        system_prompt,
        soul_file,
        allowed_skills,
        memory_namespace,
        llm_override: None,
        context: Default::default(),
    };
    cfg.agents.insert(id, profile);
    config::save_config(&app, &cfg)
}

#[tauri::command]
pub async fn delete_agent(
    app: AppHandle,
    id: String,
) -> Result<(), String> {
    let mut cfg = config::load_config(&app)?;
    cfg.agents.remove(&id);
    config::save_config(&app, &cfg)
}

#[tauri::command]
pub async fn load_soul_file(
    app: AppHandle,
    filename: String,
) -> Result<String, String> {
    let cfg = config::load_config(&app)?;
    let soul_dir = std::path::Path::new(&cfg.skills.soul_dir);
    let path = soul_dir.join(&filename);
    std::fs::read_to_string(&path).map_err(|e| format!("Failed to read {}: {}", filename, e))
}

#[tauri::command]
pub async fn save_soul_file(
    app: AppHandle,
    filename: String,
    content: String,
) -> Result<(), String> {
    let cfg = config::load_config(&app)?;
    let soul_dir = std::path::Path::new(&cfg.skills.soul_dir);
    std::fs::create_dir_all(soul_dir).map_err(|e| e.to_string())?;
    let path = soul_dir.join(&filename);
    std::fs::write(&path, &content).map_err(|e| format!("Failed to write {}: {}", filename, e))
}

#[tauri::command]
pub async fn preview_soul(
    app: AppHandle,
    filename: String,
) -> Result<serde_json::Value, String> {
    let cfg = config::load_config(&app)?;
    let soul_dir = std::path::Path::new(&cfg.skills.soul_dir);
    match amanclaw_core::soul::SoulLoader::load(soul_dir, &filename) {
        Ok(resolved) => Ok(serde_json::json!({
            "prompt": resolved.prompt,
            "variables": resolved.variables,
            "tags": resolved.tags,
        })),
        Err(e) => Err(format!("Failed to resolve soul: {}", e)),
    }
}

#[tauri::command]
pub async fn get_routing_rules(
    app: AppHandle,
) -> Result<serde_json::Value, String> {
    let cfg = config::load_config(&app)?;
    let rules: Vec<serde_json::Value> = cfg.routing.rules.iter().map(|r| {
        serde_json::json!({
            "match": {
                "platform": r.match_criteria.platform,
                "topic_id": r.match_criteria.topic_id,
                "channel_id": r.match_criteria.channel_id,
                "group_id": r.match_criteria.group_id,
            },
            "agent": r.agent,
        })
    }).collect();
    Ok(serde_json::json!({
        "rules": rules,
        "default_agent": cfg.routing.default_agent,
    }))
}

#[tauri::command]
pub async fn save_routing_rules(
    app: AppHandle,
    default_agent: String,
    rules: Vec<serde_json::Value>,
) -> Result<(), String> {
    let mut cfg = config::load_config(&app)?;
    cfg.routing.default_agent = default_agent;
    cfg.routing.rules = rules.iter().map(|r| {
        amanclaw_traits::config::RoutingRule {
            match_criteria: amanclaw_traits::config::RoutingMatch {
                platform: r["match"]["platform"].as_str().map(String::from),
                topic_id: r["match"]["topic_id"].as_str().map(String::from),
                channel_id: r["match"]["channel_id"].as_str().map(String::from),
                group_id: r["match"]["group_id"].as_str().map(String::from),
            },
            agent: r["agent"].as_str().unwrap_or("default").to_string(),
        }
    }).collect();
    config::save_config(&app, &cfg)
}

// --- Cron Jobs ---

#[tauri::command]
pub async fn list_cron_jobs(
    app: AppHandle,
) -> Result<serde_json::Value, String> {
    let cfg = config::load_config(&app)?;
    let jobs: Vec<serde_json::Value> = cfg.cron.jobs.iter().map(|(id, job)| {
        serde_json::json!({
            "id": id,
            "name": job.name,
            "schedule": job.schedule,
            "timezone": job.timezone,
            "type": job.job_type,
            "skill": job.skill,
            "input": job.input,
            "prompt": job.prompt,
            "template": job.template,
            "targets": job.targets.iter().map(|t| serde_json::json!({
                "platform": t.platform,
                "chat_id": t.chat_id,
                "topic_id": t.topic_id,
            })).collect::<Vec<_>>(),
            "agent": job.agent,
            "enabled": job.enabled,
        })
    }).collect();
    Ok(serde_json::json!({
        "jobs": jobs,
        "count": jobs.len(),
        "timezone": cfg.cron.timezone,
    }))
}

#[tauri::command]
pub async fn save_cron_job(
    app: AppHandle,
    id: String,
    job: serde_json::Value,
) -> Result<(), String> {
    let mut cfg = config::load_config(&app)?;
    let cron_job: amanclaw_traits::config::CronJobConfig =
        serde_json::from_value(job).map_err(|e| format!("Invalid job config: {}", e))?;
    cfg.cron.jobs.insert(id, cron_job);
    config::save_config(&app, &cfg)
}

#[tauri::command]
pub async fn delete_cron_job(
    app: AppHandle,
    id: String,
) -> Result<(), String> {
    let mut cfg = config::load_config(&app)?;
    cfg.cron.jobs.remove(&id);
    config::save_config(&app, &cfg)
}

#[tauri::command]
pub async fn get_cron_history(
    state: State<'_, SharedState>,
) -> Result<serde_json::Value, String> {
    let st = state.read().await;
    if let Some(handle) = &st.engine_handle {
        let rows = sqlx::query(
            "SELECT id, job_id, status, output, duration_ms, executed_at FROM cron_history ORDER BY executed_at DESC LIMIT 100"
        )
        .fetch_all(&handle.pool)
        .await
        .map_err(|e| e.to_string())?;

        let entries: Vec<serde_json::Value> = rows.iter().map(|r| {
            serde_json::json!({
                "id": r.get::<i64, _>("id"),
                "job_id": r.get::<String, _>("job_id"),
                "status": r.get::<String, _>("status"),
                "output": r.get::<Option<String>, _>("output"),
                "duration_ms": r.get::<Option<i64>, _>("duration_ms"),
                "executed_at": r.get::<String, _>("executed_at"),
            })
        }).collect();
        Ok(serde_json::json!({ "entries": entries, "count": entries.len() }))
    } else {
        Ok(serde_json::json!({ "entries": [], "count": 0 }))
    }
}

// --- Webhooks ---

#[tauri::command]
pub async fn list_webhook_endpoints(
    app: AppHandle,
) -> Result<serde_json::Value, String> {
    let cfg = config::load_config(&app)?;
    let endpoints: Vec<serde_json::Value> = cfg.webhooks.endpoints.iter().map(|(id, ep)| {
        serde_json::json!({
            "id": id,
            "name": ep.name,
            "path": ep.path,
            "auth": { "type": ep.auth.auth_type },
            "transform": { "type": ep.transform.transform_type },
            "targets": ep.targets.iter().map(|t| serde_json::json!({
                "platform": t.platform, "chat_id": t.chat_id, "topic_id": t.topic_id,
            })).collect::<Vec<_>>(),
            "agent": ep.agent,
            "rate_limit": ep.rate_limit,
            "enabled": ep.enabled,
        })
    }).collect();
    Ok(serde_json::json!({
        "endpoints": endpoints,
        "count": endpoints.len(),
        "base_path": cfg.webhooks.base_path,
    }))
}

#[tauri::command]
pub async fn save_webhook_endpoint(
    app: AppHandle,
    id: String,
    endpoint: serde_json::Value,
) -> Result<(), String> {
    let mut cfg = config::load_config(&app)?;
    let ep: amanclaw_traits::config::WebhookEndpointConfig =
        serde_json::from_value(endpoint).map_err(|e| format!("Invalid webhook config: {}", e))?;
    cfg.webhooks.endpoints.insert(id, ep);
    config::save_config(&app, &cfg)
}

#[tauri::command]
pub async fn delete_webhook_endpoint(
    app: AppHandle,
    id: String,
) -> Result<(), String> {
    let mut cfg = config::load_config(&app)?;
    cfg.webhooks.endpoints.remove(&id);
    config::save_config(&app, &cfg)
}

#[tauri::command]
pub async fn get_webhook_history(
    state: State<'_, SharedState>,
) -> Result<serde_json::Value, String> {
    let st = state.read().await;
    if let Some(handle) = &st.engine_handle {
        let rows = sqlx::query(
            "SELECT id, webhook_id, status, source_ip, payload_preview, error, duration_ms, received_at FROM webhook_history ORDER BY received_at DESC LIMIT 100"
        )
        .fetch_all(&handle.pool)
        .await
        .map_err(|e| e.to_string())?;

        let entries: Vec<serde_json::Value> = rows.iter().map(|r| {
            serde_json::json!({
                "id": r.get::<i64, _>("id"),
                "webhook_id": r.get::<String, _>("webhook_id"),
                "status": r.get::<String, _>("status"),
                "source_ip": r.get::<Option<String>, _>("source_ip"),
                "payload_preview": r.get::<Option<String>, _>("payload_preview"),
                "error": r.get::<Option<String>, _>("error"),
                "duration_ms": r.get::<Option<i64>, _>("duration_ms"),
                "received_at": r.get::<String, _>("received_at"),
            })
        }).collect();
        Ok(serde_json::json!({ "entries": entries, "count": entries.len() }))
    } else {
        Ok(serde_json::json!({ "entries": [], "count": 0 }))
    }
}

// --- Gateway ---

#[tauri::command]
pub async fn get_gateway_config(
    app: AppHandle,
) -> Result<serde_json::Value, String> {
    let cfg = config::load_config(&app)?;
    Ok(serde_json::json!({
        "enabled": cfg.gateway.enabled,
        "heartbeat_interval_secs": cfg.gateway.heartbeat_interval_secs,
        "max_connections": cfg.gateway.max_connections,
        "stale_session_timeout_secs": cfg.gateway.stale_session_timeout_secs,
    }))
}

#[tauri::command]
pub async fn save_gateway_config(
    app: AppHandle,
    enabled: bool,
    heartbeat_interval_secs: u64,
    max_connections: usize,
    stale_session_timeout_secs: u64,
) -> Result<(), String> {
    let mut cfg = config::load_config(&app)?;
    cfg.gateway.enabled = enabled;
    cfg.gateway.heartbeat_interval_secs = heartbeat_interval_secs;
    cfg.gateway.max_connections = max_connections;
    cfg.gateway.stale_session_timeout_secs = stale_session_timeout_secs;
    config::save_config(&app, &cfg)
}

#[tauri::command]
pub async fn get_gateway_status(
    app: AppHandle,
) -> Result<serde_json::Value, String> {
    let cfg = config::load_config(&app)?;
    Ok(serde_json::json!({
        "enabled": cfg.gateway.enabled,
        "connection_count": 0,
    }))
}

// --- Sub-Agents ---

#[tauri::command]
pub async fn get_subagent_config(
    app: AppHandle,
) -> Result<serde_json::Value, String> {
    let cfg = config::load_config(&app)?;
    Ok(serde_json::json!({
        "enabled": cfg.subagents.enabled,
        "max_per_session": cfg.subagents.max_per_session,
        "max_global": cfg.subagents.max_global,
        "max_depth": cfg.subagents.max_depth,
        "default_timeout_secs": cfg.subagents.default_timeout_secs,
    }))
}

#[tauri::command]
pub async fn save_subagent_config(
    app: AppHandle,
    enabled: bool,
    max_per_session: usize,
    max_global: usize,
    max_depth: usize,
    default_timeout_secs: u64,
) -> Result<(), String> {
    let mut cfg = config::load_config(&app)?;
    cfg.subagents.enabled = enabled;
    cfg.subagents.max_per_session = max_per_session;
    cfg.subagents.max_global = max_global;
    cfg.subagents.max_depth = max_depth;
    cfg.subagents.default_timeout_secs = default_timeout_secs;
    config::save_config(&app, &cfg)
}

#[tauri::command]
pub async fn list_subagents(
    state: State<'_, SharedState>,
    session_filter: Option<String>,
) -> Result<serde_json::Value, String> {
    let st = state.read().await;
    if let Some(handle) = &st.engine_handle {
        if let Some(mgr) = &handle.subagent_manager {
            let agents = if let Some(session) = session_filter {
                mgr.list(&session).await
            } else {
                vec![]
            };
            let list: Vec<serde_json::Value> = agents.iter().map(|a| {
                serde_json::json!({
                    "id": a.id,
                    "agent_id": a.agent_id,
                    "prompt": a.prompt,
                    "parent_session": a.parent_session,
                    "depth": a.depth,
                    "status": format!("{:?}", a.status),
                })
            }).collect();
            return Ok(serde_json::json!({ "subagents": list, "count": list.len() }));
        }
    }
    Ok(serde_json::json!({ "subagents": [], "count": 0 }))
}

#[tauri::command]
pub async fn cancel_subagent(
    state: State<'_, SharedState>,
    id: String,
) -> Result<bool, String> {
    let st = state.read().await;
    if let Some(handle) = &st.engine_handle {
        if let Some(mgr) = &handle.subagent_manager {
            return Ok(mgr.cancel(&id).await);
        }
    }
    Ok(false)
}

#[tauri::command]
pub async fn cancel_all_subagents(
    state: State<'_, SharedState>,
    session: String,
) -> Result<usize, String> {
    let st = state.read().await;
    if let Some(handle) = &st.engine_handle {
        if let Some(mgr) = &handle.subagent_manager {
            return Ok(mgr.cancel_all(&session).await);
        }
    }
    Ok(0)
}

// --- Marketplace / Registry ---

#[tauri::command]
pub async fn registry_list_installed(
    state: State<'_, SharedState>,
    app: AppHandle,
) -> Result<serde_json::Value, String> {
    let st = state.read().await;
    if let Some(handle) = &st.engine_handle {
        let cfg = config::load_config(&app)?;
        let registry = amanclaw_registry::local::SkillRegistry::new(
            handle.pool.clone(), cfg.registry.skills_dir.clone()
        ).await.map_err(|e| e.to_string())?;

        let installed = registry.list_installed().await.map_err(|e| e.to_string())?;
        let list: Vec<serde_json::Value> = installed.iter().map(|s| {
            serde_json::json!({
                "name": s.name, "version": s.version, "skill_type": s.skill_type,
                "description": s.description, "entry": s.entry,
                "install_dir": s.install_dir, "installed_at": s.installed_at,
            })
        }).collect();
        Ok(serde_json::json!({ "skills": list, "count": list.len() }))
    } else {
        Ok(serde_json::json!({ "skills": [], "count": 0 }))
    }
}

#[tauri::command]
pub async fn registry_install_from_path(
    state: State<'_, SharedState>,
    app: AppHandle,
    path: String,
) -> Result<serde_json::Value, String> {
    let st = state.read().await;
    if let Some(handle) = &st.engine_handle {
        let cfg = config::load_config(&app)?;
        let registry = amanclaw_registry::local::SkillRegistry::new(
            handle.pool.clone(), cfg.registry.skills_dir.clone()
        ).await.map_err(|e| e.to_string())?;

        let installed = registry.install_from_path(std::path::Path::new(&path))
            .await.map_err(|e| e.to_string())?;
        Ok(serde_json::json!({
            "name": installed.name, "version": installed.version,
        }))
    } else {
        Err("Engine not running".into())
    }
}

#[tauri::command]
pub async fn registry_uninstall(
    state: State<'_, SharedState>,
    app: AppHandle,
    name: String,
) -> Result<bool, String> {
    let st = state.read().await;
    if let Some(handle) = &st.engine_handle {
        let cfg = config::load_config(&app)?;
        let registry = amanclaw_registry::local::SkillRegistry::new(
            handle.pool.clone(), cfg.registry.skills_dir.clone()
        ).await.map_err(|e| e.to_string())?;
        registry.uninstall(&name).await.map_err(|e| e.to_string())
    } else {
        Err("Engine not running".into())
    }
}

#[tauri::command]
pub async fn registry_search_installed(
    state: State<'_, SharedState>,
    app: AppHandle,
    query: String,
) -> Result<serde_json::Value, String> {
    let st = state.read().await;
    if let Some(handle) = &st.engine_handle {
        let cfg = config::load_config(&app)?;
        let registry = amanclaw_registry::local::SkillRegistry::new(
            handle.pool.clone(), cfg.registry.skills_dir.clone()
        ).await.map_err(|e| e.to_string())?;

        let results = registry.search_installed(&query).await.map_err(|e| e.to_string())?;
        let list: Vec<serde_json::Value> = results.iter().map(|s| {
            serde_json::json!({
                "name": s.name, "version": s.version, "skill_type": s.skill_type,
                "description": s.description, "installed_at": s.installed_at,
            })
        }).collect();
        Ok(serde_json::json!({ "skills": list, "count": list.len() }))
    } else {
        Ok(serde_json::json!({ "skills": [], "count": 0 }))
    }
}

// --- Marketplace Browse (Skill Index) ---

/// Bundled seed index (compiled into the binary from docs/skill-index.json).
const SEED_INDEX_JSON: &str = include_str!("../../../../docs/skill-index.json");

#[tauri::command]
pub async fn marketplace_browse(
    query: Option<String>,
) -> Result<serde_json::Value, String> {
    use amanclaw_skill_index::IndexClient;

    // Try remote first, fall back to bundled seed
    let index = match IndexClient::new().fetch_index().await {
        Ok(idx) => idx,
        Err(_) => IndexClient::parse_index(SEED_INDEX_JSON).map_err(|e| e.to_string())?,
    };

    let skills: Vec<serde_json::Value> = match &query {
        Some(q) if !q.trim().is_empty() => index
            .search(q)
            .into_iter()
            .map(|s| serde_json::to_value(s).unwrap())
            .collect(),
        _ => index
            .skills
            .iter()
            .map(|s| serde_json::to_value(s).unwrap())
            .collect(),
    };

    let packs: serde_json::Value = serde_json::to_value(&index.packs).unwrap_or_default();

    Ok(serde_json::json!({
        "skills": skills,
        "packs": packs,
        "count": skills.len(),
    }))
}

// --- Knowledge Bases ---

#[tauri::command]
pub async fn get_embedding_config(
    app: AppHandle,
) -> Result<serde_json::Value, String> {
    let cfg = config::load_config(&app)?;
    match &cfg.embeddings {
        Some(ec) => Ok(serde_json::json!({
            "configured": true,
            "base_url": ec.base_url,
            "model": ec.model,
            "api_key": ec.api_key.is_some(),
        })),
        None => Ok(serde_json::json!({ "configured": false })),
    }
}

#[tauri::command]
pub async fn save_embedding_config(
    app: AppHandle,
    base_url: String,
    model: String,
    api_key: Option<String>,
) -> Result<(), String> {
    let mut cfg = config::load_config(&app)?;
    cfg.embeddings = Some(amanclaw_traits::config::EmbeddingConfig {
        base_url, model, api_key,
    });
    config::save_config(&app, &cfg)
}

#[tauri::command]
pub async fn get_vector_config(
    app: AppHandle,
) -> Result<serde_json::Value, String> {
    let cfg = config::load_config(&app)?;
    match &cfg.vector {
        Some(vc) => Ok(serde_json::json!({
            "configured": true,
            "backend": vc.backend,
            "qdrant_url": vc.qdrant_url,
        })),
        None => Ok(serde_json::json!({ "configured": false, "backend": "sqlite-vec" })),
    }
}

#[tauri::command]
pub async fn save_vector_config(
    app: AppHandle,
    backend: String,
    qdrant_url: Option<String>,
) -> Result<(), String> {
    let mut cfg = config::load_config(&app)?;
    cfg.vector = Some(amanclaw_traits::config::VectorConfig {
        backend, qdrant_url,
    });
    config::save_config(&app, &cfg)
}

#[tauri::command]
pub async fn list_knowledge_bases(
    app: AppHandle,
) -> Result<serde_json::Value, String> {
    let cfg = config::load_config(&app)?;
    let kbs: Vec<serde_json::Value> = cfg.knowledge_bases.iter().map(|(name, kb)| {
        serde_json::json!({
            "name": name,
            "collection": kb.collection,
            "source": kb.source,
        })
    }).collect();
    Ok(serde_json::json!({ "knowledge_bases": kbs, "count": kbs.len() }))
}

#[tauri::command]
pub async fn save_knowledge_base(
    app: AppHandle,
    name: String,
    collection: String,
    source: String,
) -> Result<(), String> {
    let mut cfg = config::load_config(&app)?;
    cfg.knowledge_bases.insert(name, amanclaw_traits::config::KnowledgeBaseConfig {
        collection, source,
    });
    config::save_config(&app, &cfg)
}

#[tauri::command]
pub async fn delete_knowledge_base(
    app: AppHandle,
    name: String,
) -> Result<(), String> {
    let mut cfg = config::load_config(&app)?;
    cfg.knowledge_bases.remove(&name);
    config::save_config(&app, &cfg)
}

// --- Communities CRUD ---

#[tauri::command]
pub async fn create_community(
    state: State<'_, SharedState>,
    name: String,
    platform: String,
    platform_group_id: String,
    zone: String,
    language: String,
    enabled_skills: Vec<String>,
) -> Result<serde_json::Value, String> {
    let st = state.read().await;
    if let Some(handle) = &st.engine_handle {
        let id = uuid::Uuid::new_v4().to_string();
        let skills_json = serde_json::to_string(&enabled_skills).unwrap_or_else(|_| "[]".into());
        sqlx::query(
            "INSERT INTO communities (id, name, platform, platform_group_id, zone, language, enabled_skills) VALUES (?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&id).bind(&name).bind(&platform).bind(&platform_group_id)
        .bind(&zone).bind(&language).bind(&skills_json)
        .execute(&handle.pool).await.map_err(|e| e.to_string())?;
        Ok(serde_json::json!({ "id": id }))
    } else {
        Err("Engine not running".into())
    }
}

#[tauri::command]
pub async fn update_community(
    state: State<'_, SharedState>,
    id: String,
    name: String,
    zone: String,
    language: String,
    enabled_skills: Vec<String>,
) -> Result<(), String> {
    let st = state.read().await;
    if let Some(handle) = &st.engine_handle {
        let skills_json = serde_json::to_string(&enabled_skills).unwrap_or_else(|_| "[]".into());
        sqlx::query(
            "UPDATE communities SET name = ?, zone = ?, language = ?, enabled_skills = ? WHERE id = ?"
        )
        .bind(&name).bind(&zone).bind(&language).bind(&skills_json).bind(&id)
        .execute(&handle.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    } else {
        Err("Engine not running".into())
    }
}

#[tauri::command]
pub async fn delete_community(
    state: State<'_, SharedState>,
    id: String,
) -> Result<(), String> {
    let st = state.read().await;
    if let Some(handle) = &st.engine_handle {
        sqlx::query("DELETE FROM communities WHERE id = ?")
            .bind(&id)
            .execute(&handle.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    } else {
        Err("Engine not running".into())
    }
}

// --- Content ---

#[tauri::command]
pub async fn get_doa_collection(
    category: Option<String>,
) -> Result<serde_json::Value, String> {
    let doas = if let Some(cat) = &category {
        amanclaw_skill_doa::collection::by_category(cat)
    } else {
        amanclaw_skill_doa::collection::ALL_DOA.iter().collect()
    };
    let filtered: Vec<serde_json::Value> = doas.iter().map(|d| serde_json::json!({
        "category": d.category,
        "title_ms": d.title_ms,
        "title_en": d.title_en,
        "arabic": d.arabic,
        "transliteration": d.transliteration,
        "translation_ms": d.translation_ms,
        "translation_en": d.translation_en,
        "source": d.source,
    })).collect();
    Ok(serde_json::json!({ "doas": filtered, "count": filtered.len() }))
}

#[tauri::command]
pub async fn search_doa(
    query: String,
) -> Result<serde_json::Value, String> {
    let results = amanclaw_skill_doa::collection::search_doa(&query);
    let list: Vec<serde_json::Value> = results.iter().map(|d| {
        serde_json::json!({
            "category": d.category,
            "title_ms": d.title_ms,
            "title_en": d.title_en,
            "arabic": d.arabic,
            "transliteration": d.transliteration,
            "translation_ms": d.translation_ms,
            "translation_en": d.translation_en,
        })
    }).collect();
    Ok(serde_json::json!({ "doas": list, "count": list.len() }))
}

#[tauri::command]
pub async fn get_zakat_rates() -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({
        "fitrah": { "rate": 7.00, "currency": "MYR", "year": 2026 },
        "note": "Rates from JAKIM — update via skill-zakat Python plugin",
    }))
}

#[tauri::command]
pub async fn get_latest_khutbah() -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({
        "available": false,
        "note": "Khutbah data available via skill-khutbah Python plugin",
    }))
}

// --- Channel Management ---

#[tauri::command]
pub async fn list_channels(
    state: State<'_, SharedState>,
) -> Result<serde_json::Value, String> {
    let st = state.read().await;
    if let Some(handle) = &st.engine_handle {
        if let (Some(mgr), Some(cfg_lock)) = (&handle.channel_manager, &handle.channels_config) {
            let cfg = cfg_lock.read().await;
            let statuses = mgr.get_all_status(&cfg).await;
            return Ok(serde_json::json!({ "channels": statuses }));
        }
    }
    Ok(serde_json::json!({ "channels": [] }))
}

#[tauri::command]
pub async fn get_channel_status(
    state: State<'_, SharedState>,
    id: String,
) -> Result<serde_json::Value, String> {
    let st = state.read().await;
    if let Some(handle) = &st.engine_handle {
        if let (Some(mgr), Some(cfg_lock)) = (&handle.channel_manager, &handle.channels_config) {
            let cfg = cfg_lock.read().await;
            if let Some(status) = mgr.get_status(&id, &cfg).await {
                return Ok(serde_json::to_value(status).map_err(|e| e.to_string())?);
            }
            return Err(format!("Channel '{}' not found", id));
        }
    }
    Err("Engine not running".into())
}

#[tauri::command]
pub async fn save_whatsapp_web_config(
    app: AppHandle,
    state: State<'_, SharedState>,
    waha_url: String,
    waha_api_key: Option<String>,
    session: Option<String>,
    webhook_port: Option<u16>,
) -> Result<(), String> {
    use amanclaw_traits::channel_config::WhatsAppWebConfig;

    let wa_config = WhatsAppWebConfig {
        enabled: true,
        waha_url,
        waha_api_key,
        session: session.unwrap_or_else(|| "default".into()),
        webhook_port: webhook_port.unwrap_or(8081),
    };

    // Update in-memory channels config if engine is running
    {
        let st = state.read().await;
        if let Some(handle) = &st.engine_handle {
            if let Some(cfg_lock) = &handle.channels_config {
                let mut cfg = cfg_lock.write().await;
                cfg.whatsapp_web = Some(wa_config.clone());
            }
        }
    }

    // Persist to secrets file for env-var based init
    let mut secrets = config::load_secrets(&app);
    secrets.insert("WAHA_API_URL".into(), wa_config.waha_url.clone());
    if let Some(ref key) = wa_config.waha_api_key {
        secrets.insert("WAHA_API_KEY".into(), key.clone());
    }
    secrets.insert("WAHA_SESSION".into(), wa_config.session.clone());
    secrets.insert("WAHA_WEBHOOK_PORT".into(), wa_config.webhook_port.to_string());
    config::save_secrets(&app, &secrets)?;

    Ok(())
}

#[tauri::command]
pub async fn start_channel(
    state: State<'_, SharedState>,
    id: String,
) -> Result<serde_json::Value, String> {
    let st = state.read().await;
    let handle = st.engine_handle.as_ref().ok_or("Engine not running")?;
    let mgr = handle.channel_manager.as_ref().ok_or("Channel manager not available")?;
    let cfg_lock = handle.channels_config.as_ref().ok_or("Channels config not available")?;
    let cfg = cfg_lock.read().await;

    match id.as_str() {
        "whatsapp-web" => {
            let wa_config = cfg.whatsapp_web.as_ref()
                .ok_or("WhatsApp Web not configured")?;
            match mgr.start_whatsapp_web(wa_config).await {
                Ok(()) => Ok(serde_json::json!({"status": "started"})),
                Err(e) => Ok(serde_json::json!({"status": "error", "error": e.to_string()})),
            }
        }
        _ => Err(format!("Starting channel '{}' is not yet supported", id)),
    }
}

#[tauri::command]
pub async fn stop_channel(
    state: State<'_, SharedState>,
    id: String,
) -> Result<serde_json::Value, String> {
    let st = state.read().await;
    let handle = st.engine_handle.as_ref().ok_or("Engine not running")?;
    let mgr = handle.channel_manager.as_ref().ok_or("Channel manager not available")?;
    mgr.stop_channel(&id).await.map_err(|e| e.to_string())?;
    Ok(serde_json::json!({"status": "stopped"}))
}

#[tauri::command]
pub async fn get_whatsapp_qr(
    state: State<'_, SharedState>,
) -> Result<serde_json::Value, String> {
    let st = state.read().await;
    let handle = st.engine_handle.as_ref().ok_or("Engine not running")?;
    let cfg_lock = handle.channels_config.as_ref().ok_or("Channels config not available")?;
    let cfg = cfg_lock.read().await;
    let wa_config = cfg.whatsapp_web.as_ref().ok_or("WhatsApp Web not configured")?;

    let url = format!("{}/api/{}/auth/qr", wa_config.waha_url, wa_config.session);
    let client = reqwest::Client::new();
    let mut req = client.get(&url);
    if let Some(ref key) = wa_config.waha_api_key {
        req = req.header("X-Api-Key", key);
    }

    match req.send().await {
        Ok(resp) if resp.status().is_success() => {
            match resp.json::<serde_json::Value>().await {
                Ok(body) => Ok(body),
                Err(_) => Ok(serde_json::json!({"error": "Failed to parse WAHA QR response"})),
            }
        }
        Ok(resp) => {
            let status = resp.status().as_u16();
            Ok(serde_json::json!({"error": format!("WAHA returned {}", status)}))
        }
        Err(e) => Ok(serde_json::json!({"error": format!("Cannot reach WAHA: {}", e)})),
    }
}

#[tauri::command]
pub async fn get_whatsapp_session(
    state: State<'_, SharedState>,
) -> Result<serde_json::Value, String> {
    let st = state.read().await;
    let handle = st.engine_handle.as_ref().ok_or("Engine not running")?;
    let cfg_lock = handle.channels_config.as_ref().ok_or("Channels config not available")?;
    let cfg = cfg_lock.read().await;
    let wa_config = cfg.whatsapp_web.as_ref().ok_or("WhatsApp Web not configured")?;

    let url = format!("{}/api/sessions/{}", wa_config.waha_url, wa_config.session);
    let client = reqwest::Client::new();
    let mut req = client.get(&url);
    if let Some(ref key) = wa_config.waha_api_key {
        req = req.header("X-Api-Key", key);
    }

    match req.send().await {
        Ok(resp) if resp.status().is_success() => {
            match resp.json::<serde_json::Value>().await {
                Ok(body) => Ok(body),
                Err(_) => Ok(serde_json::json!({"status": "unknown"})),
            }
        }
        Ok(_) => Ok(serde_json::json!({"status": "disconnected"})),
        Err(e) => Ok(serde_json::json!({"status": "error", "error": e.to_string()})),
    }
}
