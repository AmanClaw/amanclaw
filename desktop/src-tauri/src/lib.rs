mod commands;
mod config;
mod logs;
mod notifications;
mod state;
mod tray;

use state::AppState;
use std::sync::Arc;
use tauri::Manager;
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
                    if let Ok(db_path) = config::db_path(&app_handle) {
                        commands::apply_env_vars_public(&secrets, &db_path.to_string_lossy());
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
