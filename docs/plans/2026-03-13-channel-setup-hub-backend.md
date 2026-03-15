# Channel Setup Hub (Backend) Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add channel configuration API with WAHA QR code proxy, config persistence to config.yaml, and ChannelManager for hot-reload — enabling the dashboard/desktop UI to configure and manage all channels without editing files.

**Architecture:** New `ChannelsConfig` struct in amanclaw-traits, a `ChannelManager` in amanclaw-core that wraps channel lifecycle, and API route handlers in amanclaw-api that proxy WAHA endpoints and persist config changes. Env vars remain as fallback.

**Tech Stack:** Rust, Axum (API), reqwest (WAHA proxy), serde_yaml (config persistence), sqlx (existing)

---

## File Structure

| File | Responsibility |
|------|---------------|
| `rust/crates/amanclaw-traits/src/channel_config.rs` | ChannelsConfig, per-channel config structs, ChannelStatusInfo |
| `rust/crates/amanclaw-traits/src/config.rs` | Add `channels` field to AppConfig |
| `rust/crates/amanclaw-core/src/channel_manager.rs` | ChannelManager: lifecycle, start/stop, status tracking |
| `rust/crates/amanclaw-api/src/routes/channels.rs` | API handlers: list, get, update, start, stop, test, QR proxy |
| `rust/crates/amanclaw-api/src/state.rs` | Add ChannelManager + config_path to ApiState |
| `rust/crates/amanclaw-api/src/lib.rs` | Register channel routes |
| `rust/crates/amanclaw-core/src/lib.rs` | Use ChannelManager in Engine::start |

---

## Chunk 1: Channel Config Types

### Task 1: Add ChannelsConfig to amanclaw-traits

**Files:**
- Create: `rust/crates/amanclaw-traits/src/channel_config.rs`
- Modify: `rust/crates/amanclaw-traits/src/lib.rs`
- Modify: `rust/crates/amanclaw-traits/src/config.rs`

- [ ] **Step 1: Write the config types with tests**

Create `rust/crates/amanclaw-traits/src/channel_config.rs`:

```rust
use serde::{Deserialize, Serialize};

/// Per-channel configuration variants.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ChannelConfig {
    Telegram(TelegramConfig),
    Discord(DiscordConfig),
    Slack(SlackConfig),
    WhatsappCloud(WhatsAppCloudConfig),
    WhatsappWeb(WhatsAppWebConfig),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    pub token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    pub token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlackConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    pub bot_token: String,
    #[serde(default)]
    pub app_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhatsAppCloudConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    pub access_token: String,
    pub phone_number_id: String,
    #[serde(default = "default_verify_token")]
    pub verify_token: String,
    #[serde(default = "default_whatsapp_port")]
    pub webhook_port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhatsAppWebConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    pub waha_url: String,
    #[serde(default)]
    pub waha_api_key: Option<String>,
    #[serde(default = "default_session")]
    pub session: String,
    #[serde(default = "default_waha_port")]
    pub webhook_port: u16,
}

/// All channels config — the `channels:` section in config.yaml
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChannelsConfig {
    #[serde(default)]
    pub telegram: Option<TelegramConfig>,
    #[serde(default)]
    pub discord: Option<DiscordConfig>,
    #[serde(default)]
    pub slack: Option<SlackConfig>,
    #[serde(default)]
    pub whatsapp_cloud: Option<WhatsAppCloudConfig>,
    #[serde(default)]
    pub whatsapp_web: Option<WhatsAppWebConfig>,
}

/// Status info returned by the API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelStatusInfo {
    pub id: String,
    pub platform: String,
    pub configured: bool,
    pub enabled: bool,
    pub running: bool,
    pub error: Option<String>,
}

fn default_true() -> bool { true }
fn default_verify_token() -> String { "amanclaw_verify".into() }
fn default_whatsapp_port() -> u16 { 8080 }
fn default_waha_port() -> u16 { 8081 }
fn default_session() -> String { "default".into() }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_channels_config_deserialize_empty() {
        let yaml = "{}";
        let config: ChannelsConfig = serde_yaml::from_str(yaml).unwrap();
        assert!(config.telegram.is_none());
        assert!(config.whatsapp_web.is_none());
    }

    #[test]
    fn test_channels_config_deserialize_with_channels() {
        let yaml = r#"
telegram:
  token: "bot123:ABC"
whatsapp_web:
  waha_url: "http://localhost:3000"
  waha_api_key: "secret"
"#;
        let config: ChannelsConfig = serde_yaml::from_str(yaml).unwrap();
        assert!(config.telegram.is_some());
        assert_eq!(config.telegram.unwrap().token, "bot123:ABC");
        let wa = config.whatsapp_web.unwrap();
        assert_eq!(wa.waha_url, "http://localhost:3000");
        assert_eq!(wa.session, "default"); // default value
        assert_eq!(wa.webhook_port, 8081); // default value
    }

    #[test]
    fn test_channel_status_info() {
        let status = ChannelStatusInfo {
            id: "telegram".into(),
            platform: "telegram".into(),
            configured: true,
            enabled: true,
            running: true,
            error: None,
        };
        let json = serde_json::to_string(&status).unwrap();
        assert!(json.contains("telegram"));
    }
}
```

- [ ] **Step 2: Add module to lib.rs**

In `rust/crates/amanclaw-traits/src/lib.rs`, add `pub mod channel_config;`

- [ ] **Step 3: Add channels field to AppConfig**

In `rust/crates/amanclaw-traits/src/config.rs`, add to AppConfig struct:

```rust
    #[serde(default)]
    pub channels: crate::channel_config::ChannelsConfig,
```

- [ ] **Step 4: Run tests**

Run: `cd rust && cargo test -p amanclaw-traits channel_config 2>&1 | tail -10`
Expected: All 3 tests PASS.

- [ ] **Step 5: Commit**

```bash
git add rust/crates/amanclaw-traits/src/channel_config.rs rust/crates/amanclaw-traits/src/lib.rs rust/crates/amanclaw-traits/src/config.rs
git commit -m "feat(channels): add ChannelsConfig types and AppConfig integration"
```

---

## Chunk 2: ChannelManager

### Task 2: Create ChannelManager for channel lifecycle

**Files:**
- Create: `rust/crates/amanclaw-core/src/channel_manager.rs`
- Modify: `rust/crates/amanclaw-core/src/lib.rs`

The ChannelManager holds running channels, tracks their status, and supports dynamic start/stop.

- [ ] **Step 1: Write ChannelManager**

Create `rust/crates/amanclaw-core/src/channel_manager.rs`:

```rust
use amanclaw_traits::channel::Channel;
use amanclaw_traits::channel_config::{
    ChannelStatusInfo, ChannelsConfig, WhatsAppWebConfig,
};
use amanclaw_traits::message::IncomingMessage;
use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc};

/// Manages channel lifecycle: start, stop, status tracking, hot-reload.
pub struct ChannelManager {
    channels: RwLock<HashMap<String, ChannelEntry>>,
    msg_tx: mpsc::Sender<IncomingMessage>,
}

struct ChannelEntry {
    channel: Arc<dyn Channel>,
    running: bool,
    error: Option<String>,
}

impl ChannelManager {
    pub fn new(msg_tx: mpsc::Sender<IncomingMessage>) -> Self {
        Self {
            channels: RwLock::new(HashMap::new()),
            msg_tx,
        }
    }

    /// Register a channel that was started externally (for backwards compat with env var init).
    pub async fn register_running(&self, id: &str, channel: Arc<dyn Channel>) {
        let mut channels = self.channels.write().await;
        channels.insert(id.to_string(), ChannelEntry {
            channel,
            running: true,
            error: None,
        });
    }

    /// Get status of all known channels.
    pub async fn get_all_status(&self, config: &ChannelsConfig) -> Vec<ChannelStatusInfo> {
        let channels = self.channels.read().await;

        let all_ids = vec![
            ("telegram", config.telegram.as_ref().map(|c| c.enabled).unwrap_or(false), config.telegram.is_some()),
            ("discord", config.discord.as_ref().map(|c| c.enabled).unwrap_or(false), config.discord.is_some()),
            ("slack", config.slack.as_ref().map(|c| c.enabled).unwrap_or(false), config.slack.is_some()),
            ("whatsapp-cloud", config.whatsapp_cloud.as_ref().map(|c| c.enabled).unwrap_or(false), config.whatsapp_cloud.is_some()),
            ("whatsapp-web", config.whatsapp_web.as_ref().map(|c| c.enabled).unwrap_or(false), config.whatsapp_web.is_some()),
        ];

        all_ids.iter().map(|(id, enabled, configured)| {
            let entry = channels.get(*id);
            ChannelStatusInfo {
                id: id.to_string(),
                platform: id.to_string(),
                configured: *configured,
                enabled: *enabled,
                running: entry.map(|e| e.running).unwrap_or(false),
                error: entry.and_then(|e| e.error.clone()),
            }
        }).collect()
    }

    /// Get status of a single channel.
    pub async fn get_status(&self, id: &str, config: &ChannelsConfig) -> Option<ChannelStatusInfo> {
        self.get_all_status(config).await.into_iter().find(|s| s.id == id)
    }

    /// Get a reference to a running channel for sending messages.
    pub async fn get_channel(&self, platform: &str) -> Option<Arc<dyn Channel>> {
        let channels = self.channels.read().await;
        channels.get(platform).filter(|e| e.running).map(|e| e.channel.clone())
    }

    /// Get all running channels (for Engine to use in send_to_channel).
    pub async fn get_running_channels(&self) -> Vec<Arc<dyn Channel>> {
        let channels = self.channels.read().await;
        channels.values().filter(|e| e.running).map(|e| e.channel.clone()).collect()
    }

    /// Start a WhatsApp Web channel from config.
    pub async fn start_whatsapp_web(&self, config: &WhatsAppWebConfig) -> Result<()> {
        // Stop existing if running
        self.stop_channel("whatsapp-web").await.ok();

        let mut channel = amanclaw_channel_whatsapp_web::WhatsAppWebChannel::new(
            config.waha_url.clone(),
            config.waha_api_key.clone(),
            config.session.clone(),
            config.webhook_port,
        );

        match channel.start(self.msg_tx.clone()).await {
            Ok(()) => {
                let mut channels = self.channels.write().await;
                channels.insert("whatsapp-web".to_string(), ChannelEntry {
                    channel: Arc::new(channel),
                    running: true,
                    error: None,
                });
                tracing::info!("WhatsApp Web channel started via ChannelManager");
                Ok(())
            }
            Err(e) => {
                let mut channels = self.channels.write().await;
                channels.insert("whatsapp-web".to_string(), ChannelEntry {
                    channel: Arc::new(amanclaw_channel_whatsapp_web::WhatsAppWebChannel::new(
                        config.waha_url.clone(), config.waha_api_key.clone(),
                        config.session.clone(), config.webhook_port,
                    )),
                    running: false,
                    error: Some(e.to_string()),
                });
                Err(e)
            }
        }
    }

    /// Stop a channel by ID.
    pub async fn stop_channel(&self, id: &str) -> Result<()> {
        let mut channels = self.channels.write().await;
        if let Some(entry) = channels.get_mut(id) {
            entry.running = false;
            tracing::info!(channel = id, "Channel stopped");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_channel_manager_empty() {
        let (tx, _rx) = mpsc::channel(1);
        let mgr = ChannelManager::new(tx);
        let config = ChannelsConfig::default();
        let statuses = mgr.get_all_status(&config).await;
        assert_eq!(statuses.len(), 5);
        assert!(statuses.iter().all(|s| !s.running));
        assert!(statuses.iter().all(|s| !s.configured));
    }

    #[tokio::test]
    async fn test_get_running_channels_empty() {
        let (tx, _rx) = mpsc::channel(1);
        let mgr = ChannelManager::new(tx);
        let running = mgr.get_running_channels().await;
        assert!(running.is_empty());
    }
}
```

- [ ] **Step 2: Add module to lib.rs**

In `rust/crates/amanclaw-core/src/lib.rs`, add `pub mod channel_manager;`

- [ ] **Step 3: Verify compilation**

Run: `cd rust && cargo check -p amanclaw-core 2>&1 | tail -10`
Expected: Compiles. (Note: `amanclaw-channel-whatsapp-web` is already a dependency.)

- [ ] **Step 4: Run tests**

Run: `cd rust && cargo test -p amanclaw-core channel_manager 2>&1 | tail -10`
Expected: Both tests PASS.

- [ ] **Step 5: Commit**

```bash
git add rust/crates/amanclaw-core/src/channel_manager.rs rust/crates/amanclaw-core/src/lib.rs
git commit -m "feat(channels): add ChannelManager for channel lifecycle management"
```

---

## Chunk 3: API Endpoints

### Task 3: Add channel API routes

**Files:**
- Create: `rust/crates/amanclaw-api/src/routes/channels.rs`
- Modify: `rust/crates/amanclaw-api/src/routes/mod.rs`
- Modify: `rust/crates/amanclaw-api/src/state.rs`
- Modify: `rust/crates/amanclaw-api/src/lib.rs`
- Modify: `rust/crates/amanclaw-api/Cargo.toml`

- [ ] **Step 1: Update ApiState**

In `rust/crates/amanclaw-api/src/state.rs`, add:

```rust
use amanclaw_core::channel_manager::ChannelManager;
use amanclaw_traits::channel_config::ChannelsConfig;
use std::path::PathBuf;

// Add to ApiState struct:
pub channel_manager: Option<Arc<ChannelManager>>,
pub channels_config: Arc<RwLock<ChannelsConfig>>,
pub config_path: Option<PathBuf>,
```

- [ ] **Step 2: Add reqwest to amanclaw-api Cargo.toml**

Add to `[dependencies]`:
```toml
reqwest = { version = "0.12", features = ["json"] }
serde_yaml = { workspace = true }
```

- [ ] **Step 3: Create channel route handlers**

Create `rust/crates/amanclaw-api/src/routes/channels.rs`:

```rust
use crate::state::ApiState;
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use amanclaw_traits::channel_config::{ChannelStatusInfo, WhatsAppWebConfig};

/// GET /api/channels — list all channels with status
pub async fn list_channels(State(state): State<ApiState>) -> Json<Vec<ChannelStatusInfo>> {
    let config = state.channels_config.read().await;
    if let Some(ref mgr) = state.channel_manager {
        Json(mgr.get_all_status(&config).await)
    } else {
        Json(vec![])
    }
}

/// GET /api/channels/:id — get single channel status
pub async fn get_channel(
    State(state): State<ApiState>,
    Path(id): Path<String>,
) -> Result<Json<ChannelStatusInfo>, StatusCode> {
    let config = state.channels_config.read().await;
    if let Some(ref mgr) = state.channel_manager {
        mgr.get_status(&id, &config)
            .await
            .map(Json)
            .ok_or(StatusCode::NOT_FOUND)
    } else {
        Err(StatusCode::SERVICE_UNAVAILABLE)
    }
}

/// PUT /api/channels/whatsapp-web — update WhatsApp Web config
pub async fn update_whatsapp_web(
    State(state): State<ApiState>,
    Json(config): Json<WhatsAppWebConfig>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Update in-memory config
    {
        let mut channels = state.channels_config.write().await;
        channels.whatsapp_web = Some(config.clone());
    }

    // Persist to config.yaml if path available
    if let Some(ref path) = state.config_path {
        if let Err(e) = persist_channels_config(&state, path).await {
            tracing::error!(error = %e, "Failed to persist channel config");
        }
    }

    Ok(Json(serde_json::json!({"status": "saved"})))
}

/// POST /api/channels/:id/start — start a channel
pub async fn start_channel(
    State(state): State<ApiState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let mgr = state.channel_manager.as_ref().ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
    let config = state.channels_config.read().await;

    match id.as_str() {
        "whatsapp-web" => {
            let wa_config = config.whatsapp_web.as_ref().ok_or(StatusCode::BAD_REQUEST)?;
            match mgr.start_whatsapp_web(wa_config).await {
                Ok(()) => Ok(Json(serde_json::json!({"status": "started"}))),
                Err(e) => {
                    tracing::error!(error = %e, "Failed to start whatsapp-web");
                    Ok(Json(serde_json::json!({"status": "error", "error": e.to_string()})))
                }
            }
        }
        _ => Err(StatusCode::NOT_IMPLEMENTED),
    }
}

/// POST /api/channels/:id/stop — stop a channel
pub async fn stop_channel(
    State(state): State<ApiState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let mgr = state.channel_manager.as_ref().ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
    mgr.stop_channel(&id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::json!({"status": "stopped"})))
}

/// GET /api/channels/whatsapp-web/qr — proxy WAHA QR code
pub async fn get_whatsapp_qr(State(state): State<ApiState>) -> Result<Json<serde_json::Value>, StatusCode> {
    let config = state.channels_config.read().await;
    let wa_config = config.whatsapp_web.as_ref().ok_or(StatusCode::BAD_REQUEST)?;

    let url = format!("{}/api/{}/auth/qr", wa_config.waha_url, wa_config.session);
    let client = reqwest::Client::new();
    let mut req = client.get(&url);
    if let Some(ref key) = wa_config.waha_api_key {
        req = req.header("X-Api-Key", key);
    }

    match req.send().await {
        Ok(resp) if resp.status().is_success() => {
            // WAHA returns QR as image or JSON depending on version
            match resp.json::<serde_json::Value>().await {
                Ok(body) => Ok(Json(body)),
                Err(_) => Ok(Json(serde_json::json!({"error": "Failed to parse WAHA QR response"}))),
            }
        }
        Ok(resp) => {
            let status = resp.status().as_u16();
            Ok(Json(serde_json::json!({"error": format!("WAHA returned {}", status)})))
        }
        Err(e) => {
            Ok(Json(serde_json::json!({"error": format!("Cannot reach WAHA: {}", e)})))
        }
    }
}

/// GET /api/channels/whatsapp-web/session — proxy WAHA session status
pub async fn get_whatsapp_session(State(state): State<ApiState>) -> Result<Json<serde_json::Value>, StatusCode> {
    let config = state.channels_config.read().await;
    let wa_config = config.whatsapp_web.as_ref().ok_or(StatusCode::BAD_REQUEST)?;

    let url = format!("{}/api/sessions/{}", wa_config.waha_url, wa_config.session);
    let client = reqwest::Client::new();
    let mut req = client.get(&url);
    if let Some(ref key) = wa_config.waha_api_key {
        req = req.header("X-Api-Key", key);
    }

    match req.send().await {
        Ok(resp) if resp.status().is_success() => {
            match resp.json::<serde_json::Value>().await {
                Ok(body) => Ok(Json(body)),
                Err(_) => Ok(Json(serde_json::json!({"status": "unknown"}))),
            }
        }
        Ok(_) => Ok(Json(serde_json::json!({"status": "disconnected"}))),
        Err(e) => Ok(Json(serde_json::json!({"status": "error", "error": e.to_string()}))),
    }
}

/// Persist the current channels config to config.yaml.
async fn persist_channels_config(state: &ApiState, path: &std::path::Path) -> anyhow::Result<()> {
    let config = state.channels_config.read().await;
    // Read existing config, update channels section, write back
    let content = tokio::fs::read_to_string(path).await.unwrap_or_default();
    let mut yaml: serde_yaml::Value = serde_yaml::from_str(&content).unwrap_or(serde_yaml::Value::Mapping(Default::default()));

    if let serde_yaml::Value::Mapping(ref mut map) = yaml {
        let channels_val = serde_yaml::to_value(&*config)?;
        map.insert(serde_yaml::Value::String("channels".into()), channels_val);
    }

    let new_content = serde_yaml::to_string(&yaml)?;
    tokio::fs::write(path, new_content).await?;
    tracing::info!("Channel config persisted to {:?}", path);
    Ok(())
}
```

- [ ] **Step 4: Register routes in mod.rs**

In `rust/crates/amanclaw-api/src/routes/mod.rs`, add `pub mod channels;`

- [ ] **Step 5: Wire routes in lib.rs**

In `rust/crates/amanclaw-api/src/lib.rs`, add to the `authed` router:

```rust
.route("/api/channels", get(routes::channels::list_channels))
.route("/api/channels/{id}", get(routes::channels::get_channel))
.route("/api/channels/whatsapp-web/config", put(routes::channels::update_whatsapp_web))
.route("/api/channels/whatsapp-web/qr", get(routes::channels::get_whatsapp_qr))
.route("/api/channels/whatsapp-web/session", get(routes::channels::get_whatsapp_session))
.route("/api/channels/{id}/start", post(routes::channels::start_channel))
.route("/api/channels/{id}/stop", post(routes::channels::stop_channel))
```

- [ ] **Step 6: Verify compilation**

Run: `cd rust && cargo check -p amanclaw-api 2>&1 | tail -10`
Expected: Compiles. (Will need to update ApiState initialization sites — handle in Task 4.)

- [ ] **Step 7: Commit**

```bash
git add rust/crates/amanclaw-api/src/routes/channels.rs rust/crates/amanclaw-api/src/routes/mod.rs rust/crates/amanclaw-api/src/state.rs rust/crates/amanclaw-api/src/lib.rs rust/crates/amanclaw-api/Cargo.toml
git commit -m "feat(channels): add channel management API endpoints with WAHA QR proxy"
```

---

## Chunk 4: Engine Integration

### Task 4: Wire ChannelManager into Engine::start

**Files:**
- Modify: `rust/crates/amanclaw-core/src/lib.rs`
- Modify: `rust/crates/amanclaw-cli/src/main.rs` (or wherever ApiState is constructed)

This task connects everything: Engine creates ChannelManager, registers env-var-started channels with it, passes it to ApiState.

- [ ] **Step 1: Read the CLI main.rs to find ApiState construction**

Read `rust/crates/amanclaw-cli/src/main.rs` to understand where ApiState is built and passed to the API server.

- [ ] **Step 2: Update Engine to create and expose ChannelManager**

In `rust/crates/amanclaw-core/src/lib.rs`:

1. Add ChannelManager to EngineStartResult:
```rust
pub channel_manager: Arc<ChannelManager>,
pub channels_config: Arc<RwLock<ChannelsConfig>>,
```

2. In Engine::start, after channels are started:
- Create `ChannelManager::new(msg_tx.clone())` BEFORE dropping msg_tx
- After each channel starts, call `channel_manager.register_running(platform, channel.clone())`
- Build ChannelsConfig from env vars for backwards compat
- Return channel_manager in EngineStartResult

3. In the actor's `send_to_channel`, use ChannelManager instead of the static channels vec.

- [ ] **Step 3: Update ApiState construction in CLI**

Pass `channel_manager` and `channels_config` from EngineStartResult into ApiState.

Initialize new ApiState fields:
```rust
channel_manager: Some(engine_result.channel_manager),
channels_config: engine_result.channels_config,
config_path: Some(PathBuf::from(config_path)),
```

- [ ] **Step 4: Verify full compilation**

Run: `cd rust && cargo check 2>&1 | tail -15`
Expected: Full workspace compiles.

- [ ] **Step 5: Run all tests**

Run: `cd rust && cargo test 2>&1 | tail -20`
Expected: All tests pass.

- [ ] **Step 6: Commit**

```bash
git add rust/crates/amanclaw-core/src/lib.rs rust/crates/amanclaw-cli/src/main.rs
git commit -m "feat(channels): wire ChannelManager into engine and API state"
```

---

## Chunk 5: Config Fallback (Env Vars)

### Task 5: Build ChannelsConfig from environment variables

**Files:**
- Modify: `rust/crates/amanclaw-traits/src/channel_config.rs`

Add a `from_env()` method to ChannelsConfig that reads environment variables and builds the config. This ensures backwards compatibility — existing deployments using env vars continue working without config.yaml changes.

- [ ] **Step 1: Add from_env to ChannelsConfig**

```rust
impl ChannelsConfig {
    /// Build channels config from environment variables (backwards compatibility).
    /// Only populates channels whose env vars are set.
    pub fn from_env() -> Self {
        let telegram = std::env::var("TELEGRAM_BOT_TOKEN").ok().map(|token| {
            TelegramConfig { enabled: true, token }
        });

        let discord = std::env::var("DISCORD_BOT_TOKEN").ok().map(|token| {
            DiscordConfig { enabled: true, token }
        });

        let slack = std::env::var("SLACK_BOT_TOKEN").ok().map(|bot_token| {
            SlackConfig { enabled: true, bot_token, app_token: std::env::var("SLACK_APP_TOKEN").ok() }
        });

        let whatsapp_cloud = std::env::var("WHATSAPP_ACCESS_TOKEN").ok().map(|access_token| {
            WhatsAppCloudConfig {
                enabled: true,
                access_token,
                phone_number_id: std::env::var("WHATSAPP_PHONE_NUMBER_ID").unwrap_or_default(),
                verify_token: std::env::var("WHATSAPP_VERIFY_TOKEN").unwrap_or_else(|_| "amanclaw_verify".into()),
                webhook_port: std::env::var("WHATSAPP_WEBHOOK_PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(8080),
            }
        });

        let whatsapp_web = std::env::var("WAHA_API_URL").ok().map(|waha_url| {
            WhatsAppWebConfig {
                enabled: true,
                waha_url,
                waha_api_key: std::env::var("WAHA_API_KEY").ok(),
                session: std::env::var("WAHA_SESSION").unwrap_or_else(|_| "default".into()),
                webhook_port: std::env::var("WAHA_WEBHOOK_PORT").ok().and_then(|p| p.parse().ok()).unwrap_or(8081),
            }
        });

        Self { telegram, discord, slack, whatsapp_cloud, whatsapp_web }
    }

    /// Merge: prefer yaml config, fill gaps from env vars.
    pub fn merge_with_env(self) -> Self {
        let env = Self::from_env();
        Self {
            telegram: self.telegram.or(env.telegram),
            discord: self.discord.or(env.discord),
            slack: self.slack.or(env.slack),
            whatsapp_cloud: self.whatsapp_cloud.or(env.whatsapp_cloud),
            whatsapp_web: self.whatsapp_web.or(env.whatsapp_web),
        }
    }
}
```

- [ ] **Step 2: Add test**

```rust
#[test]
fn test_merge_with_env_prefers_yaml() {
    let yaml_config = ChannelsConfig {
        telegram: Some(TelegramConfig { enabled: true, token: "yaml_token".into() }),
        ..Default::default()
    };
    // merge_with_env will try env vars for missing channels, but telegram should keep yaml value
    let merged = yaml_config.merge_with_env();
    assert_eq!(merged.telegram.unwrap().token, "yaml_token");
}
```

- [ ] **Step 3: Run tests**

Run: `cd rust && cargo test -p amanclaw-traits channel_config 2>&1 | tail -10`
Expected: All tests PASS.

- [ ] **Step 4: Commit**

```bash
git add rust/crates/amanclaw-traits/src/channel_config.rs
git commit -m "feat(channels): add env var fallback with merge_with_env"
```

---

## Summary

| Chunk | Task | What it delivers |
|-------|------|-----------------|
| 1 | Task 1 | ChannelsConfig types + AppConfig integration |
| 2 | Task 2 | ChannelManager with lifecycle management |
| 3 | Task 3 | API endpoints: list, get, update, start, stop, QR proxy, session proxy |
| 4 | Task 4 | Engine wiring: ChannelManager in Engine::start + ApiState |
| 5 | Task 5 | Env var fallback for backwards compatibility |

After all tasks: the API is ready for the dashboard/desktop UI to consume. The UI plan (Part 2) will be a separate spec.
