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
