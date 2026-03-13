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
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Create shared state early so the log layer can write to it
    let shared_state = Arc::new(RwLock::new(AppState::new()));

    // Initialize logging with custom layer that captures to AppState
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("amanclaw=info"));
    let log_layer = logs::AppLogLayer::new(shared_state.clone());

    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer())
        .with(log_layer)
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_shell::init())
        .manage(shared_state)
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
                            match amanclaw_core::Engine::start(cfg.clone()).await {
                                Ok(result) => {
                                    let engine_handle = result.handle.clone();
                                    let auth = result.auth.clone();
                                    let pool = result.pool.clone();
                                    let registry = result.registry.clone();
                                    let channel_manager = result.channel_manager.clone();
                                    let channels_config = result.channels_config.clone();

                                    let state_clone = state.clone();
                                    let join_handle = tokio::spawn(async move {
                                        match result.join.await {
                                            Ok(Ok(())) => {
                                                tracing::info!("Engine run loop exited (no active channels)");
                                            }
                                            Ok(Err(e)) => {
                                                let mut st = state_clone.write().await;
                                                st.engine_status = state::EngineStatus::Error(e.to_string());
                                            }
                                            Err(e) => {
                                                let mut st = state_clone.write().await;
                                                st.engine_status = state::EngineStatus::Error(format!("Engine task panicked: {}", e));
                                            }
                                        }
                                    });

                                    let mut st = state.write().await;
                                    st.engine_status = state::EngineStatus::Running;
                                    st.config = Some(cfg);
                                    st.started_at = Some(std::time::Instant::now());
                                    st.engine_handle = Some(state::EngineHandle {
                                        engine_handle,
                                        join_handle,
                                        auth,
                                        pool,
                                        registry,
                                        subagent_manager: None,
                                        channel_manager: Some(channel_manager),
                                        channels_config: Some(channels_config),
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
            commands::get_logs,
            commands::approve_user,
            commands::block_user,
            commands::unblock_user,
            commands::add_user,
            commands::make_admin,
            commands::remove_admin,
            commands::get_user_detail,
            commands::get_user_history,
            commands::get_user_stats,
            commands::get_mcp_servers,
            commands::save_mcp_server,
            commands::delete_mcp_server,
            commands::disable_skill,
            commands::enable_skill,
            commands::get_disabled_skills,
            // Agents
            commands::list_agents,
            commands::save_agent,
            commands::delete_agent,
            commands::load_soul_file,
            commands::save_soul_file,
            commands::preview_soul,
            commands::get_routing_rules,
            commands::save_routing_rules,
            // Cron Jobs
            commands::list_cron_jobs,
            commands::save_cron_job,
            commands::delete_cron_job,
            commands::get_cron_history,
            // Webhooks
            commands::list_webhook_endpoints,
            commands::save_webhook_endpoint,
            commands::delete_webhook_endpoint,
            commands::get_webhook_history,
            // Gateway
            commands::get_gateway_config,
            commands::save_gateway_config,
            commands::get_gateway_status,
            // Sub-Agents
            commands::get_subagent_config,
            commands::save_subagent_config,
            commands::list_subagents,
            commands::cancel_subagent,
            commands::cancel_all_subagents,
            // Marketplace / Registry
            commands::registry_list_installed,
            commands::registry_install_from_path,
            commands::registry_uninstall,
            commands::registry_search_installed,
            commands::marketplace_browse,
            // Knowledge Bases
            commands::get_embedding_config,
            commands::save_embedding_config,
            commands::get_vector_config,
            commands::save_vector_config,
            commands::list_knowledge_bases,
            commands::save_knowledge_base,
            commands::delete_knowledge_base,
            // Communities CRUD
            commands::create_community,
            commands::update_community,
            commands::delete_community,
            // Channels
            commands::list_channels,
            commands::get_channel_status,
            commands::save_whatsapp_web_config,
            commands::start_channel,
            commands::stop_channel,
            commands::get_whatsapp_qr,
            commands::get_whatsapp_session,
            // Content
            commands::get_doa_collection,
            commands::search_doa,
            commands::get_zakat_rates,
            commands::get_latest_khutbah,
        ])
        .run(tauri::generate_context!())
        .expect("error running AmanClaw Desktop");
}
