pub mod pipeline;
pub mod router;
pub mod registry;
pub mod context_engine;
pub mod soul;
pub mod scheduler;
pub mod webhooks;
pub mod subagent;
pub mod skills;

use amanclaw_traits::config::AppConfig;
use amanclaw_traits::context::ContextEngine;
use amanclaw_traits::memory::MemoryBackend;
use amanclaw_traits::vector::VectorStore;
use amanclaw_traits::channel::Channel;
use amanclaw_memory::sqlite::SqliteMemory;
use amanclaw_memory::vector::SqliteVectorStore;
use amanclaw_security::auth::Auth;
use amanclaw_security::rate_limiter::RateLimiter;
use amanclaw_llm::client::LlmClient;
use amanclaw_llm::embeddings::EmbeddingClient;
use crate::context_engine::StandardContextEngine;
use amanclaw_channel_telegram::TelegramChannel;
use amanclaw_channel_discord::DiscordChannel;
use amanclaw_channel_whatsapp::WhatsAppChannel;
use amanclaw_channel_whatsapp_web::WhatsAppWebChannel;
use amanclaw_channel_slack::SlackChannel;
use amanclaw_mcp::handler::McpHandler;
use crate::pipeline::Pipeline;
use crate::registry::PluginRegistry;
use crate::router::AgentRouter;
use anyhow::Result;
use tokio::sync::mpsc;
use std::path::Path;
use std::sync::{Arc, Mutex};
use sqlx::SqlitePool;

pub struct Engine {
    #[allow(dead_code)]
    config: AppConfig,
    pipeline: Pipeline,
    registry: Arc<PluginRegistry>,
    channels: Vec<Arc<dyn Channel>>,
    rx: mpsc::Receiver<amanclaw_traits::message::IncomingMessage>,
    tx: Option<mpsc::Sender<amanclaw_traits::message::IncomingMessage>>,
    auth: Arc<Mutex<Auth>>,
    pool: SqlitePool,
    agent_router: AgentRouter,
    sched_rx: mpsc::Receiver<crate::scheduler::SchedulerEvent>,
}

impl Engine {
    pub async fn new(mut config: AppConfig) -> Result<Self> {
        // Initialize subsystems
        let db_path = std::env::var("MEMORY_DB_PATH").unwrap_or_else(|_| "memory.db".into());
        let memory = SqliteMemory::new(&db_path).await?;
        let auth = Auth::new(config.admin_users.clone());
        let rate_limiter = RateLimiter::new(config.rate_limit_per_minute);
        let llm = LlmClient::new(config.llm.clone());

        // Register built-in skills (skip disabled ones)
        let disabled = &config.skills.disabled;
        let mut registry = PluginRegistry::new();
        let builtins: Vec<Arc<dyn amanclaw_traits::skill::Skill>> = vec![
            Arc::new(amanclaw_skill_sysinfo::SysInfoSkill),
            Arc::new(amanclaw_skill_shell::ShellSkill),
            Arc::new(amanclaw_skill_solat::SolatSkill),
            Arc::new(amanclaw_skill_qiblat::QiblatSkill),
            Arc::new(amanclaw_skill_hijri::HijriSkill),
            Arc::new(amanclaw_skill_doa::DoaSkill),
            Arc::new(amanclaw_skill_quran::QuranSkill),
        ];
        for skill in builtins {
            let name = skill.metadata().name;
            if disabled.iter().any(|d| d == &name) {
                tracing::info!(skill = %name, "Skill disabled by config");
                continue;
            }
            registry.register(skill);
        }

        // Register sub-agent skill if enabled
        if config.subagents.enabled {
            let subagent_mgr = Arc::new(subagent::SubAgentManager::new(config.subagents.clone()));
            let subagent_skill = Arc::new(skills::subagent_skill::SubAgentSkill::new(subagent_mgr));
            registry.register(subagent_skill);
        }

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

        // Load SOUL.md files for agents that have them configured
        let soul_dir = std::path::Path::new(&config.skills.soul_dir);
        for (_id, profile) in config.agents.iter_mut() {
            if let Some(ref filename) = profile.soul_file {
                match crate::soul::SoulLoader::load(soul_dir, filename) {
                    Ok(resolved) => {
                        profile.system_prompt = resolved.prompt;
                        tracing::info!(agent = %profile.id, file = %filename, "Loaded SOUL.md");
                    }
                    Err(e) => {
                        tracing::warn!(agent = %profile.id, error = %e, "Failed to load SOUL.md, using inline prompt");
                    }
                }
            }
        }

        // Build agent router from config
        let agent_router = AgentRouter::new(
            config.agents.clone(),
            config.routing.rules.clone(),
            config.routing.default_agent.clone(),
        );

        let registry = Arc::new(registry);
        let auth_arc = Arc::new(Mutex::new(auth));
        let pool = memory.pool().clone();
        let memory_arc: Arc<dyn MemoryBackend> = Arc::new(memory);
        let llm_arc = Arc::new(llm);

        // Optional: create vector store (always available since we use SQLite)
        let vector_store: Option<Arc<dyn VectorStore>> = Some(Arc::new(
            SqliteVectorStore::new(pool.clone())
        ));

        // Optional: create embedding client if configured
        let embedding_client = config.embeddings.as_ref().map(|ec| {
            Arc::new(EmbeddingClient::new(
                ec.base_url.clone(),
                ec.model.clone(),
                ec.api_key.clone(),
            ))
        });

        // Index knowledge bases if configured
        if let (Some(vs), Some(ec)) = (&vector_store, &embedding_client) {
            for (name, kb_config) in &config.knowledge_bases {
                let source_path = std::path::Path::new(&kb_config.source);
                if source_path.exists() {
                    tracing::info!(name, collection = %kb_config.collection, "Loading knowledge base");
                    match std::fs::read_to_string(source_path) {
                        Ok(content) => {
                            match serde_json::from_str::<Vec<amanclaw_traits::vector::Document>>(&content) {
                                Ok(docs) => {
                                    let texts: Vec<&str> = docs.iter().map(|d| d.content.as_str()).collect();
                                    let mut offset = 0;
                                    for chunk in texts.chunks(32) {
                                        match ec.embed(chunk).await {
                                            Ok(embeddings) => {
                                                let chunk_docs: Vec<_> = docs[offset..offset + chunk.len()].to_vec();
                                                if let Err(e) = vs.upsert_with_embeddings(
                                                    &kb_config.collection, &chunk_docs, &embeddings,
                                                ).await {
                                                    tracing::error!(name, error = %e, "Failed to index knowledge base chunk");
                                                }
                                            }
                                            Err(e) => {
                                                tracing::error!(name, error = %e, "Failed to generate embeddings");
                                            }
                                        }
                                        offset += chunk.len();
                                    }
                                    tracing::info!(name, docs = docs.len(), "Knowledge base indexed");
                                }
                                Err(e) => {
                                    tracing::error!(name, error = %e, "Failed to parse knowledge base JSON");
                                }
                            }
                        }
                        Err(e) => {
                            tracing::error!(name, error = %e, "Failed to read knowledge base file");
                        }
                    }
                } else {
                    tracing::warn!(name, path = %kb_config.source, "Knowledge base file not found");
                }
            }
        }

        let context_engine: Arc<dyn ContextEngine> = Arc::new(
            StandardContextEngine::new(
                memory_arc.clone(),
                llm_arc.clone(),
                registry.clone(),
                amanclaw_llm::prompts::SYSTEM_PROMPT_BASE.to_string(),
                vector_store,
                embedding_client.clone(),
            )
        );
        let emitter: Arc<dyn amanclaw_traits::event::EventEmitter> = Arc::new(amanclaw_traits::event::NoopEmitter);
        let pipeline = Pipeline::with_services(auth_arc.clone(), rate_limiter, context_engine, memory_arc, llm_arc, emitter);
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

        // Initialize scheduler
        let (sched_tx, sched_rx) = mpsc::channel(64);
        let mut scheduler = crate::scheduler::Scheduler::new(sched_tx);
        scheduler.start_jobs(&config.cron.jobs, &config.cron.timezone);

        tracing::info!(skills = registry.skill_count(), "Engine initialized");

        Ok(Self { config, pipeline, registry, channels, rx, tx: Some(tx), auth: auth_arc, pool, agent_router, sched_rx })
    }

    /// Get a sender for channels to push messages into the engine.
    pub fn sender(&self) -> mpsc::Sender<amanclaw_traits::message::IncomingMessage> {
        self.tx.as_ref().expect("sender() called after run()").clone()
    }

    /// Get the shared auth instance for use by the management API.
    pub fn auth(&self) -> &Arc<Mutex<Auth>> {
        &self.auth
    }

    /// Get the SQLite pool for use by the management API.
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    /// Get the plugin registry.
    pub fn registry(&self) -> &Arc<PluginRegistry> {
        &self.registry
    }

    pub async fn run(mut self) -> Result<()> {
        // Drop our sender so the channel closes when all external senders are dropped
        drop(self.tx.take());
        tracing::info!("Engine running");

        loop {
            tokio::select! {
                Some(msg) = self.rx.recv() => {
                    let platform = msg.platform.clone();
                    let profile = self.agent_router.resolve(&msg);
                    tracing::debug!(agent = %profile.id, "Routed to agent");
                    match self.pipeline.process(msg, &self.registry, &profile).await {
                        Ok(Some(response)) => {
                            self.send_to_channel(&platform, response).await;
                        }
                        Ok(None) => {}
                        Err(e) => tracing::error!(error = %e, "Pipeline error"),
                    }
                }
                Some(event) = self.sched_rx.recv() => {
                    match event {
                        crate::scheduler::SchedulerEvent::SendMessage(response) => {
                            let platform = response.platform.clone().unwrap_or_default();
                            self.send_to_channel(&platform, response).await;
                        }
                        crate::scheduler::SchedulerEvent::InjectMessage(msg) => {
                            let platform = msg.platform.clone();
                            let profile = self.agent_router.resolve(&msg);
                            match self.pipeline.process(msg, &self.registry, &profile).await {
                                Ok(Some(response)) => {
                                    self.send_to_channel(&platform, response).await;
                                }
                                Ok(None) => {}
                                Err(e) => tracing::error!(error = %e, "Cron pipeline error"),
                            }
                        }
                    }
                }
                else => break,
            }
        }
        Ok(())
    }

    async fn send_to_channel(&self, platform: &str, response: amanclaw_traits::message::OutgoingMessage) {
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

    pub async fn shutdown(&self) -> Result<()> {
        tracing::info!("Engine shutdown complete");
        Ok(())
    }
}
