pub mod pipeline;
pub mod router;
pub mod registry;

use amanclaw_traits::config::AppConfig;
use anyhow::Result;

pub struct Engine {
    #[allow(dead_code)]
    config: AppConfig,
}

impl Engine {
    pub async fn new(config: AppConfig) -> Result<Self> {
        tracing::info!("Engine initializing...");
        Ok(Self { config })
    }

    pub async fn run(&self) -> Result<()> {
        tracing::info!("Engine running (no channels configured yet)");
        tokio::signal::ctrl_c().await?;
        Ok(())
    }

    pub async fn shutdown(&self) -> Result<()> {
        tracing::info!("Engine shutting down...");
        Ok(())
    }
}
