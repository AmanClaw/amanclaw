pub mod pipeline;
pub mod router;
pub mod registry;

use amanclaw_traits::config::AppConfig;
use amanclaw_traits::channel::Channel;
use amanclaw_memory::sqlite::SqliteMemory;
use amanclaw_security::auth::Auth;
use amanclaw_security::rate_limiter::RateLimiter;
use amanclaw_llm::client::LlmClient;
use amanclaw_channel_telegram::TelegramChannel;
use amanclaw_channel_discord::DiscordChannel;
use amanclaw_channel_whatsapp::WhatsAppChannel;
use amanclaw_channel_whatsapp_web::WhatsAppWebChannel;
use amanclaw_channel_slack::SlackChannel;
use amanclaw_mcp::handler::McpHandler;
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
        let wasm_skills = amanclaw_wasm_runtime::runtime::load_all_plugins(plugin_dir);
        for skill in wasm_skills {
            registry.register(skill);
        }

        // Connect to external MCP servers and register their tools
        if !config.mcp_servers.is_empty() {
            let mcp_skills = amanclaw_mcp::bridge::connect_all(&config.mcp_servers).await;
            for skill in mcp_skills {
                registry.register(skill);
            }
        }

        // Load script plugins (Python, JavaScript, etc.)
        if !config.script_plugins.is_empty() {
            let script_configs: std::collections::HashMap<String, amanclaw_script_runtime::ScriptPluginConfig> =
                config.script_plugins.iter().map(|(k, v)| {
                    (k.clone(), amanclaw_script_runtime::ScriptPluginConfig {
                        command: v.command.clone(),
                        args: v.args.clone(),
                        env: v.env.clone(),
                    })
                }).collect();
            let script_skills = amanclaw_script_runtime::load_script_plugins(&script_configs).await;
            for skill in script_skills {
                registry.register(skill);
            }
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

        if let Some(mut slack) = SlackChannel::from_env() {
            slack.start(tx.clone()).await?;
            channels.push(Arc::new(slack));
            tracing::info!("Slack channel started");
        }

        // Start MCP server if configured
        if let Ok(port_str) = std::env::var("MCP_HTTP_PORT") {
            if let Ok(port) = port_str.parse::<u16>() {
                let mut mcp_handler = McpHandler::new("amanclaw", env!("CARGO_PKG_VERSION"));
                // Register all skills with MCP
                for (_name, skill) in registry.iter_skills() {
                    mcp_handler.register_skill(skill.clone());
                }
                let mcp_handler = Arc::new(mcp_handler);
                tokio::spawn(async move {
                    if let Err(e) = amanclaw_mcp::http::run_http(mcp_handler, port).await {
                        tracing::error!(error = %e, "MCP HTTP server error");
                    }
                });
                tracing::info!(port, "MCP HTTP server started");
            }
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
