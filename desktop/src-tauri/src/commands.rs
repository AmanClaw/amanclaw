use crate::state::{AppMode, AppState, EngineStatus};
use std::sync::Arc;
use tauri::State;
use tokio::sync::RwLock;

#[tauri::command]
pub async fn get_status(
    state: State<'_, Arc<RwLock<AppState>>>,
) -> Result<serde_json::Value, String> {
    let app = state.read().await;
    Ok(serde_json::json!({
        "engine_running": app.engine_status == EngineStatus::Running,
        "mode": match &app.mode {
            AppMode::Local => "local",
            AppMode::Remote { .. } => "remote",
        },
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
pub async fn get_communities(
    state: State<'_, Arc<RwLock<AppState>>>,
) -> Result<serde_json::Value, String> {
    let app = state.read().await;
    match &app.mode {
        AppMode::Remote { url, token } => {
            let client = reqwest::Client::new();
            let resp = client.get(format!("{}/api/communities", url))
                .bearer_auth(token)
                .send().await
                .map_err(|e| e.to_string())?;
            let data: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
            Ok(data)
        }
        AppMode::Local => Ok(serde_json::json!({ "communities": [], "count": 0 })),
    }
}

#[tauri::command]
pub async fn get_skills(
    state: State<'_, Arc<RwLock<AppState>>>,
) -> Result<serde_json::Value, String> {
    let app = state.read().await;
    match &app.mode {
        AppMode::Remote { url, token } => {
            let client = reqwest::Client::new();
            let resp = client.get(format!("{}/api/skills", url))
                .bearer_auth(token)
                .send().await
                .map_err(|e| e.to_string())?;
            let data: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
            Ok(data)
        }
        AppMode::Local => Ok(serde_json::json!({ "skills": [], "count": 0 })),
    }
}

#[tauri::command]
pub async fn get_users(
    state: State<'_, Arc<RwLock<AppState>>>,
) -> Result<serde_json::Value, String> {
    let app = state.read().await;
    match &app.mode {
        AppMode::Remote { url, token } => {
            let client = reqwest::Client::new();
            let resp = client.get(format!("{}/api/users", url))
                .bearer_auth(token)
                .send().await
                .map_err(|e| e.to_string())?;
            let data: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
            Ok(data)
        }
        AppMode::Local => Ok(serde_json::json!({ "users": [], "count": 0 })),
    }
}
