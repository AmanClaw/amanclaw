use crate::state::ApiState;
use amanclaw_traits::channel_config::{ChannelStatusInfo, WhatsAppWebConfig};
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};

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

/// PUT /api/channels/whatsapp-web/config — update WhatsApp Web config
pub async fn update_whatsapp_web(
    State(state): State<ApiState>,
    Json(config): Json<WhatsAppWebConfig>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    {
        let mut channels = state.channels_config.write().await;
        channels.whatsapp_web = Some(config.clone());
    }

    if let Some(ref path) = state.config_path
        && let Err(e) = persist_channels_config(&state, path).await
    {
        tracing::error!(error = %e, "Failed to persist channel config");
    }

    Ok(Json(serde_json::json!({"status": "saved"})))
}

/// POST /api/channels/:id/start — start a channel
pub async fn start_channel(
    State(state): State<ApiState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let mgr = state
        .channel_manager
        .as_ref()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
    let config = state.channels_config.read().await;

    match id.as_str() {
        "whatsapp-web" => {
            let wa_config = config
                .whatsapp_web
                .as_ref()
                .ok_or(StatusCode::BAD_REQUEST)?;
            match mgr.start_whatsapp_web(wa_config).await {
                Ok(()) => Ok(Json(serde_json::json!({"status": "started"}))),
                Err(e) => {
                    tracing::error!(error = %e, "Failed to start whatsapp-web");
                    Ok(Json(
                        serde_json::json!({"status": "error", "error": e.to_string()}),
                    ))
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
    let mgr = state
        .channel_manager
        .as_ref()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
    mgr.stop_channel(&id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::json!({"status": "stopped"})))
}

/// GET /api/channels/whatsapp-web/qr — proxy WAHA QR code
pub async fn get_whatsapp_qr(
    State(state): State<ApiState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let config = state.channels_config.read().await;
    let wa_config = config
        .whatsapp_web
        .as_ref()
        .ok_or(StatusCode::BAD_REQUEST)?;

    let url = format!("{}/api/{}/auth/qr", wa_config.waha_url, wa_config.session);
    let client = reqwest::Client::new();
    let mut req = client.get(&url);
    if let Some(ref key) = wa_config.waha_api_key {
        req = req.header("X-Api-Key", key);
    }

    match req.send().await {
        Ok(resp) if resp.status().is_success() => match resp.json::<serde_json::Value>().await {
            Ok(body) => Ok(Json(body)),
            Err(_) => Ok(Json(
                serde_json::json!({"error": "Failed to parse WAHA QR response"}),
            )),
        },
        Ok(resp) => {
            let status = resp.status().as_u16();
            Ok(Json(
                serde_json::json!({"error": format!("WAHA returned {}", status)}),
            ))
        }
        Err(e) => Ok(Json(
            serde_json::json!({"error": format!("Cannot reach WAHA: {}", e)}),
        )),
    }
}

/// GET /api/channels/whatsapp-web/session — proxy WAHA session status
pub async fn get_whatsapp_session(
    State(state): State<ApiState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let config = state.channels_config.read().await;
    let wa_config = config
        .whatsapp_web
        .as_ref()
        .ok_or(StatusCode::BAD_REQUEST)?;

    let url = format!("{}/api/sessions/{}", wa_config.waha_url, wa_config.session);
    let client = reqwest::Client::new();
    let mut req = client.get(&url);
    if let Some(ref key) = wa_config.waha_api_key {
        req = req.header("X-Api-Key", key);
    }

    match req.send().await {
        Ok(resp) if resp.status().is_success() => match resp.json::<serde_json::Value>().await {
            Ok(body) => Ok(Json(body)),
            Err(_) => Ok(Json(serde_json::json!({"status": "unknown"}))),
        },
        Ok(_) => Ok(Json(serde_json::json!({"status": "disconnected"}))),
        Err(e) => Ok(Json(
            serde_json::json!({"status": "error", "error": e.to_string()}),
        )),
    }
}

/// Persist the current channels config to config.yaml.
async fn persist_channels_config(state: &ApiState, path: &std::path::Path) -> anyhow::Result<()> {
    let config = state.channels_config.read().await;
    let content = tokio::fs::read_to_string(path).await.unwrap_or_default();
    let mut yaml: serde_yaml::Value =
        serde_yaml::from_str(&content).unwrap_or(serde_yaml::Value::Mapping(Default::default()));

    if let serde_yaml::Value::Mapping(ref mut map) = yaml {
        let channels_val = serde_yaml::to_value(&*config)?;
        map.insert(serde_yaml::Value::String("channels".into()), channels_val);
    }

    let new_content = serde_yaml::to_string(&yaml)?;
    tokio::fs::write(path, new_content).await?;
    tracing::info!("Channel config persisted to {:?}", path);
    Ok(())
}
