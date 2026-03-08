mod commands;
mod config;
mod logs;
mod notifications;
mod state;
mod tray;

use state::AppState;
use std::sync::Arc;
use tokio::sync::RwLock;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
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
