use amanclaw_core::Engine;
use anyhow::{Context, Result};
use std::path::PathBuf;
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

    // Build and run engine
    let engine = Engine::new(config).await?;

    // Graceful shutdown on Ctrl+C
    tokio::select! {
        result = engine.run() => {
            result.context("Engine exited with error")?;
        }
        _ = tokio::signal::ctrl_c() => {
            tracing::info!("Shutdown signal received");
        }
    }

    tracing::info!("AmanClaw stopped.");
    Ok(())
}
