pub mod pipeline;
pub mod router;
pub mod registry;

use amanclaw_traits::config::AppConfig;
use amanclaw_traits::channel::Channel;
use amanclaw_memory::sqlite::SqliteMemory;
use amanclaw_security::auth::Auth;
use amanclaw_security::rate_limiter::RateLimiter;
use amanclaw_llm::client::LlmClient;
use amanclaw_wasm_runtime::loader::PluginLoader;
use amanclaw_channel_telegram::TelegramChannel;
use amanclaw_channel_discord::DiscordChannel;
use amanclaw_channel_whatsapp::WhatsAppChannel;
use amanclaw_channel_whatsapp_web::WhatsAppWebChannel;
use crate::pipeline::Pipeline;
use crate::registry::PluginRegistry;
use anyhow::Result;
use tokio::sync::mpsc;
use std::path::Path;
use std::sync::Arc;

pub struct Engine {
    #[allow(dead_code)]
    config: AppConfig,
    pipeline: Pipeline,
    registry: Arc<PluginRegistry>,
    channels: Vec<Arc<dyn Channel>>,
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

        // Register built-in skills
        let mut registry = PluginRegistry::new();
        registry.register(Arc::new(amanclaw_skill_sysinfo::SysInfoSkill));
        registry.register(Arc::new(amanclaw_skill_websearch::WebSearchSkill));
        registry.register(Arc::new(amanclaw_skill_shell::ShellSkill));

        // Load WASM plugins
        let plugin_dir = Path::new(&config.plugins.dir);
        if let Ok(loader) = PluginLoader::new(plugin_dir) {
            let plugins = loader.discover()?;
            tracing::info!(count = plugins.len(), "Discovered WASM plugins");
        }

        let registry = Arc::new(registry);
        let pipeline = Pipeline::with_services(auth, rate_limiter, memory, llm);
        let (tx, rx) = mpsc::channel(256);

        // Start channel adapters
        let mut channels: Vec<Arc<dyn Channel>> = Vec::new();

        if let Ok(token) = std::env::var("TELEGRAM_BOT_TOKEN") {
            let mut telegram = TelegramChannel::new(token);
            telegram.start(tx.clone()).await?;
            channels.push(Arc::new(telegram));
            tracing::info!("Telegram channel started");
        }

        if let Ok(token) = std::env::var("DISCORD_BOT_TOKEN") {
            let mut discord = DiscordChannel::new(token);
            discord.start(tx.clone()).await?;
            channels.push(Arc::new(discord));
            tracing::info!("Discord channel started");
        }

        if let Some(mut whatsapp) = WhatsAppChannel::from_env() {
            whatsapp.start(tx.clone()).await?;
            channels.push(Arc::new(whatsapp));
            tracing::info!("WhatsApp channel started");
        }

        if let Some(mut whatsapp_web) = WhatsAppWebChannel::from_env() {
            whatsapp_web.start(tx.clone()).await?;
            channels.push(Arc::new(whatsapp_web));
            tracing::info!("WhatsApp Web (WAHA) channel started");
        }

        tracing::info!(skills = registry.skill_count(), "Engine initialized");

        Ok(Self { config, pipeline, registry, channels, rx, tx })
    }

    /// Get a sender for channels to push messages into the engine.
    pub fn sender(&self) -> mpsc::Sender<amanclaw_traits::message::IncomingMessage> {
        self.tx.clone()
    }

    pub async fn run(mut self) -> Result<()> {
        // Drop our sender so the channel closes when all external senders are dropped
        drop(self.tx);
        tracing::info!("Engine running");
        while let Some(msg) = self.rx.recv().await {
            let platform = msg.platform.clone();
            match self.pipeline.process(msg, &self.registry).await {
                Ok(Some(response)) => {
                    tracing::info!(chat_id = %response.chat_id, "Sending response");
                    for ch in &self.channels {
                        if ch.platform() == platform {
                            if let Err(e) = ch.send_message(response.clone()).await {
                                tracing::error!(error = %e, "Failed to send response");
                            }
                            break;
                        }
                    }
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
