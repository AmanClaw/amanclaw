pub mod pipeline;
pub mod router;
pub mod registry;

use amanclaw_traits::config::AppConfig;
use amanclaw_memory::sqlite::SqliteMemory;
use amanclaw_security::auth::Auth;
use amanclaw_security::rate_limiter::RateLimiter;
use amanclaw_llm::client::LlmClient;
use amanclaw_wasm_runtime::loader::PluginLoader;
use crate::pipeline::Pipeline;
use crate::registry::PluginRegistry;
use anyhow::Result;
use tokio::sync::mpsc;
use std::path::Path;

pub struct Engine {
    #[allow(dead_code)]
    config: AppConfig,
    pipeline: Pipeline,
    #[allow(dead_code)]
    registry: PluginRegistry,
    rx: mpsc::Receiver<amanclaw_traits::message::IncomingMessage>,
    tx: mpsc::Sender<amanclaw_traits::message::IncomingMessage>,
}

impl Engine {
    pub async fn new(config: AppConfig) -> Result<Self> {
        // Initialize subsystems
        let db_path = std::env::var("MEMORY_DB_PATH").unwrap_or_else(|_| "memory.db".into());
        let memory = SqliteMemory::new(&db_path).await?;
        let auth = Auth::new(config.admin_users.clone());
        let rate_limiter = RateLimiter::new(config.rate_limit_per_minute);
        let llm = LlmClient::new(config.llm.clone());

        // Load WASM plugins
        let registry = PluginRegistry::new();
        let plugin_dir = Path::new(&config.plugins.dir);
        if let Ok(loader) = PluginLoader::new(plugin_dir) {
            let plugins = loader.discover()?;
            tracing::info!(count = plugins.len(), "Discovered WASM plugins");
            // TODO: instantiate each .wasm and call metadata() to register
        }

        let pipeline = Pipeline::with_services(auth, rate_limiter, memory, llm);
        let (tx, rx) = mpsc::channel(256);

        tracing::info!("Engine initialized");

        Ok(Self { config, pipeline, registry, rx, tx })
    }

    /// Get a sender for channels to push messages into the engine.
    pub fn sender(&self) -> mpsc::Sender<amanclaw_traits::message::IncomingMessage> {
        self.tx.clone()
    }

    pub async fn run(mut self) -> Result<()> {
        tracing::info!("Engine running");
        while let Some(msg) = self.rx.recv().await {
            match self.pipeline.process(msg).await {
                Ok(Some(response)) => {
                    tracing::info!(chat_id = %response.chat_id, "Sending response");
                    // TODO: route response back to correct channel
                }
                Ok(None) => {}
                Err(e) => tracing::error!(error = %e, "Pipeline error"),
            }
        }
        Ok(())
    }

    pub async fn shutdown(&self) -> Result<()> {
        tracing::info!("Engine shutdown complete");
        Ok(())
    }
}
