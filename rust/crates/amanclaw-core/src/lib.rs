pub mod channel_manager;
pub mod context_engine;
pub mod diagnostics;
pub mod error;
pub mod handle;
pub mod learning;
pub mod middleware;
pub mod pipeline;
pub mod registry;
pub mod router;
pub mod scheduler;
pub mod skills;
pub mod soul;
pub mod subagent;
pub mod token_budget;
pub mod webhooks;

use crate::channel_manager::ChannelManager;
use crate::context_engine::StandardContextEngine;
use crate::handle::{EngineCommand, EngineHandle, EngineStatus};
use crate::pipeline::Pipeline;
use crate::registry::PluginRegistry;
use crate::router::AgentRouter;
use amanclaw_channel_discord::DiscordChannel;
use amanclaw_channel_slack::SlackChannel;
use amanclaw_channel_telegram::TelegramChannel;
use amanclaw_channel_whatsapp::WhatsAppChannel;
use amanclaw_channel_whatsapp_web::WhatsAppWebChannel;
use amanclaw_llm::client::LlmClient;
use amanclaw_llm::embeddings::EmbeddingClient;
use amanclaw_mcp::handler::McpHandler;
use amanclaw_memory::sqlite::SqliteMemory;
use amanclaw_memory::vector::SqliteVectorStore;
use amanclaw_security::auth::Auth;
use amanclaw_security::rate_limiter::RateLimiter;
use amanclaw_traits::channel::Channel;
use amanclaw_traits::channel_config::ChannelsConfig;
use amanclaw_traits::config::AppConfig;
use amanclaw_traits::context::ContextEngine;
use amanclaw_traits::memory::MemoryBackend;
use amanclaw_traits::message::IncomingMessage;
use amanclaw_traits::vector::VectorStore;
use anyhow::Result;
use sqlx::SqlitePool;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use tokio::sync::{Semaphore, mpsc, watch};

/// Result returned by [`Engine::start`] containing handles to the running engine.
pub struct EngineStartResult {
    /// Cheap, cloneable handle for sending commands to the engine actor.
    pub handle: EngineHandle,
    /// Join handle for the actor task. Await this to block until the engine exits.
    pub join: tokio::task::JoinHandle<Result<()>>,
    /// Shared auth instance for management API.
    pub auth: Arc<RwLock<Auth>>,
    /// SQLite connection pool.
    pub pool: SqlitePool,
    /// Plugin registry with all loaded skills.
    pub registry: Arc<PluginRegistry>,
    /// Channel manager for dynamic channel lifecycle.
    pub channel_manager: Arc<ChannelManager>,
    /// Shared channels config (from config.yaml + env vars).
    pub channels_config: Arc<RwLock<ChannelsConfig>>,
}

pub struct Engine {
    #[allow(dead_code)]
    config: AppConfig,
    pipeline: Arc<Pipeline>,
    registry: Arc<PluginRegistry>,
    channels: Vec<Arc<dyn Channel>>,
    agent_router: Arc<AgentRouter>,
}

impl Engine {
    /// Initialize and start the engine actor. Returns handles for interacting with
    /// the running engine.
    pub async fn start(mut config: AppConfig) -> Result<EngineStartResult> {
        // Initialize subsystems
        let db_path = std::env::var("MEMORY_DB_PATH").unwrap_or_else(|_| "memory.db".into());
        let memory = SqliteMemory::new(&db_path).await?;
        let auth = Auth::with_pool(config.admin_users.clone(), memory.pool().clone()).await;
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

        // Load WASM plugins with configured resource limits
        let plugin_dir = Path::new(&config.plugins.dir);
        let wasm_sandbox = amanclaw_wasm_runtime::runtime::sandbox_from_limits(
            config.plugins.wasm_memory_limit_mb,
            config.plugins.wasm_fuel_limit,
        );
        let wasm_skills =
            amanclaw_wasm_runtime::runtime::load_all_plugins(plugin_dir, wasm_sandbox.clone());
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
            let script_configs: std::collections::HashMap<
                String,
                amanclaw_script_runtime::ScriptPluginConfig,
            > = config
                .script_plugins
                .iter()
                .map(|(k, v)| {
                    (
                        k.clone(),
                        amanclaw_script_runtime::ScriptPluginConfig {
                            command: v.command.clone(),
                            args: v.args.clone(),
                            env: v.env.clone(),
                        },
                    )
                })
                .collect();
            let script_skills = amanclaw_script_runtime::load_script_plugins(&script_configs).await;
            for skill in script_skills {
                registry.register(skill);
            }
        }

        // Load registry-installed skills if enabled
        if config.registry.enabled {
            let reg_pool = SqlitePool::connect(&format!(
                "sqlite:{}",
                std::env::var("MEMORY_DB_PATH").unwrap_or_else(|_| "memory.db".into())
            ))
            .await?;
            if let Ok(skill_registry) = amanclaw_registry::local::SkillRegistry::new(
                reg_pool,
                config.registry.skills_dir.clone(),
            )
            .await
                && let Ok(installed) = skill_registry.list_installed().await
            {
                for skill_info in &installed {
                    let skill_dir = std::path::Path::new(&skill_info.install_dir);
                    match skill_info.skill_type.as_str() {
                        "wasm" => {
                            if let Some(entry) = &skill_info.entry {
                                let wasm_path = skill_dir.join(entry);
                                let wasm_skills = amanclaw_wasm_runtime::runtime::load_all_plugins(
                                    &wasm_path,
                                    wasm_sandbox.clone(),
                                );
                                for skill in wasm_skills {
                                    registry.register(skill);
                                }
                            }
                        }
                        "script" => {
                            tracing::info!(
                                name = %skill_info.name,
                                "Registry skill (script) found — requires script runtime config"
                            );
                        }
                        other => {
                            tracing::warn!(
                                name = %skill_info.name,
                                skill_type = other,
                                "Unknown registry skill type"
                            );
                        }
                    }
                }
                tracing::info!(count = installed.len(), "Registry skills loaded");
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

        // Initialize Reactive Learning Engine knowledge store
        let knowledge_store = Arc::new(amanclaw_memory::knowledge_store::KnowledgeStore::new(
            memory.pool().clone(),
        ));
        knowledge_store.init().await?;
        tracing::info!("Reactive Learning Engine initialized");

        let registry = Arc::new(registry);
        let auth_arc = Arc::new(RwLock::new(auth));
        let pool = memory.pool().clone();
        let memory_arc: Arc<dyn MemoryBackend> =
            Arc::new(amanclaw_memory::cached::CachedMemory::new(
                Arc::new(memory),
                1000, // max entries
                300,  // TTL 5 minutes
            ));
        let llm_arc = Arc::new(llm);

        // Optional: create vector store (always available since we use SQLite)
        let vector_store: Option<Arc<dyn VectorStore>> =
            Some(Arc::new(SqliteVectorStore::new(pool.clone())));

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
                            match serde_json::from_str::<Vec<amanclaw_traits::vector::Document>>(
                                &content,
                            ) {
                                Ok(docs) => {
                                    let texts: Vec<&str> =
                                        docs.iter().map(|d| d.content.as_str()).collect();
                                    let mut offset = 0;
                                    for chunk in texts.chunks(32) {
                                        match ec.embed(chunk).await {
                                            Ok(embeddings) => {
                                                let chunk_docs: Vec<_> =
                                                    docs[offset..offset + chunk.len()].to_vec();
                                                if let Err(e) = vs
                                                    .upsert_with_embeddings(
                                                        &kb_config.collection,
                                                        &chunk_docs,
                                                        &embeddings,
                                                    )
                                                    .await
                                                {
                                                    tracing::error!(name, error = %e, "Failed to index knowledge base chunk");
                                                }
                                            }
                                            Err(e) => {
                                                tracing::error!(name, error = %e, "Failed to generate embeddings");
                                            }
                                        }
                                        offset += chunk.len();
                                    }
                                    tracing::info!(
                                        name,
                                        docs = docs.len(),
                                        "Knowledge base indexed"
                                    );
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

        let context_engine: Arc<dyn ContextEngine> = Arc::new(StandardContextEngine::new(
            memory_arc.clone(),
            llm_arc.clone(),
            registry.clone(),
            amanclaw_llm::prompts::SYSTEM_PROMPT_BASE.to_string(),
            vector_store,
            embedding_client.clone(),
        ));
        let emitter: Arc<dyn amanclaw_traits::event::EventEmitter> =
            Arc::new(amanclaw_traits::event::NoopEmitter);
        let pipeline = Pipeline::with_services(
            auth_arc.clone(),
            rate_limiter,
            context_engine,
            memory_arc,
            llm_arc,
            emitter,
            Some(knowledge_store),
        );

        // Build channels config from config.yaml (will be merged with env in Task 5)
        let channels_config = Arc::new(RwLock::new(config.channels.clone()));

        // Create message channel for adapters
        let (msg_tx, msg_rx) = mpsc::channel::<IncomingMessage>(256);

        // Create ChannelManager for dynamic channel lifecycle
        let channel_manager = Arc::new(ChannelManager::new(msg_tx.clone()));

        // Start channel adapters
        let mut channels: Vec<Arc<dyn Channel>> = Vec::new();

        if let Ok(token) = std::env::var("TELEGRAM_BOT_TOKEN") {
            let mut telegram = TelegramChannel::new(token);
            telegram.start(msg_tx.clone()).await?;
            let ch: Arc<dyn Channel> = Arc::new(telegram);
            channel_manager
                .register_running("telegram", ch.clone())
                .await;
            channels.push(ch);
            tracing::info!("Telegram channel started");
        }

        if let Ok(token) = std::env::var("DISCORD_BOT_TOKEN") {
            let mut discord = DiscordChannel::new(token);
            discord.start(msg_tx.clone()).await?;
            let ch: Arc<dyn Channel> = Arc::new(discord);
            channel_manager
                .register_running("discord", ch.clone())
                .await;
            channels.push(ch);
            tracing::info!("Discord channel started");
        }

        if let Some(mut whatsapp) = WhatsAppChannel::from_env() {
            whatsapp.start(msg_tx.clone()).await?;
            let ch: Arc<dyn Channel> = Arc::new(whatsapp);
            channel_manager
                .register_running("whatsapp-cloud", ch.clone())
                .await;
            channels.push(ch);
            tracing::info!("WhatsApp channel started");
        }

        if let Some(mut whatsapp_web) = WhatsAppWebChannel::from_env() {
            whatsapp_web.start(msg_tx.clone()).await?;
            let ch: Arc<dyn Channel> = Arc::new(whatsapp_web);
            channel_manager
                .register_running("whatsapp-web", ch.clone())
                .await;
            channels.push(ch);
            tracing::info!("WhatsApp Web (WAHA) channel started");
        }

        if let Some(mut slack) = SlackChannel::from_env() {
            slack.start(msg_tx.clone()).await?;
            let ch: Arc<dyn Channel> = Arc::new(slack);
            channel_manager.register_running("slack", ch.clone()).await;
            channels.push(ch);
            tracing::info!("Slack channel started");
        }

        // Drop msg_tx so the channel closes when all adapter senders are dropped
        drop(msg_tx);

        // Start MCP server if configured
        if let Ok(port_str) = std::env::var("MCP_HTTP_PORT")
            && let Ok(port) = port_str.parse::<u16>()
        {
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

        // Initialize scheduler
        let (sched_tx, sched_rx) = mpsc::channel(64);
        let mut scheduler = crate::scheduler::Scheduler::new(sched_tx);
        scheduler.start_jobs(&config.cron.jobs, &config.cron.timezone);

        tracing::info!(skills = registry.skill_count(), "Engine initialized");

        // Create command channel and status watch for the EngineHandle
        let (cmd_tx, cmd_rx) = mpsc::channel::<EngineCommand>(512);
        let (status_tx, status_rx) = watch::channel(EngineStatus::Starting);
        let handle = EngineHandle::new(cmd_tx, status_rx);

        // Build engine and spawn actor
        let engine = Engine {
            config,
            pipeline: Arc::new(pipeline),
            registry: registry.clone(),
            channels,
            agent_router: Arc::new(agent_router),
        };

        let join =
            tokio::spawn(
                async move { engine.run_actor(cmd_rx, msg_rx, status_tx, sched_rx).await },
            );

        Ok(EngineStartResult {
            handle,
            join,
            auth: auth_arc,
            pool,
            registry,
            channel_manager,
            channels_config,
        })
    }

    /// Actor loop: receives commands and messages, processes them concurrently.
    async fn run_actor(
        self,
        mut cmd_rx: mpsc::Receiver<EngineCommand>,
        mut msg_rx: mpsc::Receiver<IncomingMessage>,
        status_tx: watch::Sender<EngineStatus>,
        mut sched_rx: mpsc::Receiver<crate::scheduler::SchedulerEvent>,
    ) -> Result<()> {
        let semaphore = Arc::new(Semaphore::new(32));
        let mut join_set = tokio::task::JoinSet::<()>::new();
        let mut messages_processed: u64 = 0;
        let started_at = Instant::now();

        let _ = status_tx.send(EngineStatus::Running {
            started_at,
            messages_processed: 0,
        });

        tracing::info!("Engine actor running");

        loop {
            tokio::select! {
                Some(cmd) = cmd_rx.recv() => {
                    match cmd {
                        EngineCommand::ProcessMessage(msg) => {
                            messages_processed += 1;
                            let _ = status_tx.send(EngineStatus::Running {
                                started_at,
                                messages_processed,
                            });
                            self.spawn_process_message(msg, &semaphore, &mut join_set);
                        }
                        EngineCommand::SchedulerEvent(event) => {
                            self.handle_scheduler_event(event, &semaphore, &mut join_set);
                        }
                        EngineCommand::GetStatus(reply) => {
                            let _ = reply.send(EngineStatus::Running {
                                started_at,
                                messages_processed,
                            });
                        }
                        EngineCommand::GetSkills(reply) => {
                            let _ = reply.send(self.registry.list_skill_metadata());
                        }
                        EngineCommand::Shutdown(reply) => {
                            tracing::info!("Engine shutdown requested");
                            let _ = status_tx.send(EngineStatus::Stopped);
                            // Wait for in-flight tasks (with timeout)
                            let deadline = tokio::time::sleep(std::time::Duration::from_secs(30));
                            tokio::pin!(deadline);
                            loop {
                                tokio::select! {
                                    result = join_set.join_next() => {
                                        if result.is_none() { break; } // all done
                                    }
                                    _ = &mut deadline => {
                                        tracing::warn!("Shutdown timeout, {} tasks still running", join_set.len());
                                        break;
                                    }
                                }
                            }
                            let _ = reply.send(());
                            return Ok(());
                        }
                    }
                }
                Some(msg) = msg_rx.recv() => {
                    messages_processed += 1;
                    let _ = status_tx.send(EngineStatus::Running {
                        started_at,
                        messages_processed,
                    });
                    self.spawn_process_message(msg, &semaphore, &mut join_set);
                }
                Some(event) = sched_rx.recv() => {
                    self.handle_scheduler_event(event, &semaphore, &mut join_set);
                }
                else => break,
            }
        }

        tracing::info!("Engine actor stopped");
        Ok(())
    }

    /// Spawn a task to process an incoming message with concurrency control.
    fn spawn_process_message(
        &self,
        msg: IncomingMessage,
        semaphore: &Arc<Semaphore>,
        join_set: &mut tokio::task::JoinSet<()>,
    ) {
        let semaphore = semaphore.clone();
        let pipeline = self.pipeline.clone();
        let registry = self.registry.clone();
        let agent_router = self.agent_router.clone();
        let channels = self.channels.clone();

        join_set.spawn(async move {
            let _permit = semaphore.acquire_owned().await.unwrap();
            let platform = msg.platform.clone();
            let profile = agent_router.resolve(&msg);
            tracing::debug!(agent = %profile.id, "Routed to agent");
            match pipeline.process(msg, &registry, &profile).await {
                Ok(Some(response)) => {
                    Self::send_to_channel(&channels, &platform, response).await;
                }
                Ok(None) => {}
                Err(e) => tracing::error!(error = %e, "Pipeline error"),
            }
        });
    }

    /// Handle a scheduler event with concurrency control.
    fn handle_scheduler_event(
        &self,
        event: crate::scheduler::SchedulerEvent,
        semaphore: &Arc<Semaphore>,
        join_set: &mut tokio::task::JoinSet<()>,
    ) {
        match event {
            crate::scheduler::SchedulerEvent::SendMessage(response) => {
                let channels = self.channels.clone();
                join_set.spawn(async move {
                    let platform = response.platform.clone().unwrap_or_default();
                    Self::send_to_channel(&channels, &platform, response).await;
                });
            }
            crate::scheduler::SchedulerEvent::InjectMessage(msg) => {
                let semaphore = semaphore.clone();
                let pipeline = self.pipeline.clone();
                let registry = self.registry.clone();
                let agent_router = self.agent_router.clone();
                let channels = self.channels.clone();

                join_set.spawn(async move {
                    let _permit = semaphore.acquire_owned().await.unwrap();
                    let platform = msg.platform.clone();
                    let profile = agent_router.resolve(&msg);
                    match pipeline.process(msg, &registry, &profile).await {
                        Ok(Some(response)) => {
                            Self::send_to_channel(&channels, &platform, response).await;
                        }
                        Ok(None) => {}
                        Err(e) => tracing::error!(error = %e, "Cron pipeline error"),
                    }
                });
            }
        }
    }

    /// Send a response to the appropriate channel adapter.
    async fn send_to_channel(
        channels: &[Arc<dyn Channel>],
        platform: &str,
        response: amanclaw_traits::message::OutgoingMessage,
    ) {
        tracing::info!(chat_id = %response.chat_id, "Sending response");
        for ch in channels {
            if ch.platform() == platform {
                if let Err(e) = ch.send_message(response.clone()).await {
                    tracing::error!(error = %e, "Failed to send response");
                }
                break;
            }
        }
    }
}
