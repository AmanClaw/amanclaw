use crate::state::ApiState;
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use serde::Deserialize;
use std::collections::HashMap;

/// GET /api/mcp-servers — list configured MCP servers
pub async fn list_mcp_servers(
    State(state): State<ApiState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let Some(ref path) = state.config_path else {
        return Ok(Json(serde_json::json!({ "servers": {} })));
    };

    let content = tokio::fs::read_to_string(path)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let config: amanclaw_traits::config::AppConfig =
        serde_yaml::from_str(&content).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let servers: serde_json::Map<String, serde_json::Value> = config
        .mcp_servers
        .iter()
        .map(|(name, sc)| {
            let transport = if sc.url.is_some() { "http" } else { "stdio" };
            (
                name.clone(),
                serde_json::json!({
                    "command": sc.command,
                    "args": sc.args,
                    "env": sc.env,
                    "url": sc.url,
                    "transport": transport,
                }),
            )
        })
        .collect();

    Ok(Json(serde_json::json!({ "servers": servers })))
}

#[derive(Deserialize)]
pub struct SaveMcpServer {
    pub command: Option<String>,
    pub args: Option<Vec<String>>,
    pub env: Option<HashMap<String, String>>,
    pub url: Option<String>,
}

/// PUT /api/mcp-servers/:name — create or update an MCP server
pub async fn save_mcp_server(
    State(state): State<ApiState>,
    Path(name): Path<String>,
    Json(body): Json<SaveMcpServer>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let path = state
        .config_path
        .as_ref()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;

    let content = tokio::fs::read_to_string(path)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let mut yaml: serde_yaml::Value =
        serde_yaml::from_str(&content).unwrap_or(serde_yaml::Value::Mapping(Default::default()));

    if let serde_yaml::Value::Mapping(ref mut map) = yaml {
        let mcp_key = serde_yaml::Value::String("mcp_servers".into());
        let mcp_map = map
            .entry(mcp_key)
            .or_insert_with(|| serde_yaml::Value::Mapping(Default::default()));

        if let serde_yaml::Value::Mapping(servers) = mcp_map {
            let entry = amanclaw_traits::config::McpServerConfig {
                command: body.command,
                args: body.args.unwrap_or_default(),
                env: body.env.unwrap_or_default(),
                url: body.url,
            };
            let val = serde_yaml::to_value(&entry).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            servers.insert(serde_yaml::Value::String(name.clone()), val);
        }
    }

    let new_content =
        serde_yaml::to_string(&yaml).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    tokio::fs::write(path, new_content)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    tracing::info!(name = %name, "MCP server saved");
    Ok(Json(serde_json::json!({"status": "saved"})))
}

/// DELETE /api/mcp-servers/:name — remove an MCP server
pub async fn delete_mcp_server(
    State(state): State<ApiState>,
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let path = state
        .config_path
        .as_ref()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?;

    let content = tokio::fs::read_to_string(path)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let mut yaml: serde_yaml::Value =
        serde_yaml::from_str(&content).unwrap_or(serde_yaml::Value::Mapping(Default::default()));

    if let serde_yaml::Value::Mapping(ref mut map) = yaml {
        let mcp_key = serde_yaml::Value::String("mcp_servers".into());
        if let Some(serde_yaml::Value::Mapping(servers)) = map.get_mut(&mcp_key) {
            servers.remove(&serde_yaml::Value::String(name.clone()));
        }
    }

    let new_content =
        serde_yaml::to_string(&yaml).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    tokio::fs::write(path, new_content)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    tracing::info!(name = %name, "MCP server deleted");
    Ok(Json(serde_json::json!({"status": "deleted"})))
}
