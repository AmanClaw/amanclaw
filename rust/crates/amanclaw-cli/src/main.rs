use amanclaw_core::Engine;
use anyhow::{Context, Result};
use std::path::PathBuf;
use std::sync::Arc;
use tracing_subscriber::EnvFilter;

fn setup_logging(log_format: Option<&str>) {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("amanclaw=info"));

    match log_format {
        Some("json") => {
            tracing_subscriber::fmt()
                .with_env_filter(filter)
                .json()
                .init();
        }
        _ => {
            tracing_subscriber::fmt()
                .with_env_filter(filter)
                .init();
        }
    }
}

fn find_config() -> Result<PathBuf> {
    let candidates = ["config.yaml", "config.yml"];
    for name in &candidates {
        let path = PathBuf::from(name);
        if path.exists() {
            return Ok(path);
        }
    }
    anyhow::bail!("No config.yaml found. Copy config.example.yaml to config.yaml and edit it.")
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load .env if present
    dotenvy::dotenv().ok();

    let log_format = std::env::var("LOG_FORMAT").ok();
    setup_logging(log_format.as_deref());

    tracing::info!("AmanClaw starting...");

    // Load config
    let config_path = find_config()?;
    let config_str = std::fs::read_to_string(&config_path)
        .with_context(|| format!("Failed to read {}", config_path.display()))?;
    let config: amanclaw_traits::config::AppConfig = serde_yaml::from_str(&config_str)
        .with_context(|| "Failed to parse config.yaml")?;

    tracing::info!(model = %config.llm.model, base_url = %config.llm.base_url, "Config loaded");

    // Build and start engine actor
    let result = Engine::start(config).await?;

    // Start management API if configured
    if let Ok(port_str) = std::env::var("API_PORT") {
        if let Ok(port) = port_str.parse::<u16>() {
            let api_token = std::env::var("API_TOKEN")
                .unwrap_or_else(|_| {
                    let token = format!(
                        "amanclaw-{:x}-{}",
                        std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_millis(),
                        std::process::id()
                    );
                    tracing::info!(token = %token, "Generated API token (set API_TOKEN to override)");
                    token
                });
            let api_state = amanclaw_api::state::ApiState {
                registry: result.registry.clone(),
                pool: result.pool.clone(),
                api_token,
                bot_status: Arc::new(tokio::sync::RwLock::new(
                    amanclaw_api::state::BotStatus::new(),
                )),
                auth: result.auth.clone(),
                webhook_router: None,
                gateway: None,
            };
            tokio::spawn(async move {
                if let Err(e) = amanclaw_api::run_api_server(api_state, port).await {
                    tracing::error!("Management API error: {}", e);
                }
            });
            tracing::info!(port, "Management API started");
        }
    }

    // Graceful shutdown on Ctrl+C
    tokio::select! {
        join_result = result.join => {
            match join_result {
                Ok(inner) => inner.context("Engine exited with error")?,
                Err(e) => anyhow::bail!("Engine task panicked: {}", e),
            }
        }
        _ = tokio::signal::ctrl_c() => {
            tracing::info!("Shutdown signal received");
            let _ = result.handle.shutdown().await;
        }
    }

    tracing::info!("AmanClaw stopped.");
    Ok(())
}
