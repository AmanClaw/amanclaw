mod cli;
mod dev_watcher;
mod scaffold;

use anyhow::{Context, Result};
use clap::Parser;
use cli::{Cli, Command, SkillAction};
use std::path::PathBuf;
use std::sync::Arc;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    let cli = Cli::parse();

    let log_format = std::env::var("LOG_FORMAT").ok();
    setup_logging(log_format.as_deref());

    match cli.command {
        Some(Command::Init) => cmd_init().await,
        Some(Command::Dev { watch }) => cmd_dev(&cli.config, watch).await,
        Some(Command::Check) => cmd_check(&cli.config),
        Some(Command::Skill { action }) => cmd_skill(action),
        Some(Command::Run) | None => cmd_run(&cli.config).await,
    }
}

async fn cmd_run(config_path: &str) -> Result<()> {
    let config_path = find_config(config_path)?;
    let config_str = std::fs::read_to_string(&config_path)
        .with_context(|| format!("Failed to read {}", config_path.display()))?;
    let config: amanclaw_traits::config::AppConfig =
        serde_yaml::from_str(&config_str).with_context(|| "Failed to parse config file")?;

    tracing::info!(model = %config.llm.model, base_url = %config.llm.base_url, "Config loaded");
    tracing::info!("Starting AmanClaw with config: {}", config_path.display());

    let diag = amanclaw_core::diagnostics::run_startup_diagnostics(&config);
    amanclaw_core::diagnostics::print_diagnostics(&diag);

    let metrics_handle = metrics_exporter_prometheus::PrometheusBuilder::new()
        .install_recorder()
        .ok();

    let result = amanclaw_core::Engine::start(config).await?;

    if let Ok(port_str) = std::env::var("API_PORT") {
        if let Ok(port) = port_str.parse::<u16>() {
            let api_token = std::env::var("API_TOKEN").unwrap_or_else(|_| {
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
                metrics_handle: metrics_handle.clone(),
            };
            tokio::spawn(async move {
                if let Err(e) = amanclaw_api::run_api_server(api_state, port).await {
                    tracing::error!("Management API error: {}", e);
                }
            });
            tracing::info!(port, "Management API started");
        }
    }

    tokio::select! {
        join_result = result.join => {
            match join_result {
                Ok(inner) => inner.context("Engine exited with error")?,
                Err(e) => anyhow::bail!("Engine task panicked: {e}"),
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

async fn cmd_init() -> Result<()> {
    println!("Initializing AmanClaw project...");

    let config_path = PathBuf::from("config.yaml");
    if config_path.exists() {
        println!("  config.yaml already exists, skipping.");
    } else {
        let example = PathBuf::from("config.example.yaml");
        if example.exists() {
            std::fs::copy(&example, &config_path)
                .context("Failed to copy config.example.yaml")?;
            println!("  Created config.yaml from config.example.yaml");
        } else {
            let minimal = include_str!("../../../config_minimal.yaml");
            std::fs::write(&config_path, minimal)
                .context("Failed to write config.yaml")?;
            println!("  Created minimal config.yaml");
        }
    }

    let env_path = PathBuf::from(".env");
    if env_path.exists() {
        println!("  .env already exists, skipping.");
    } else {
        let env_content = "# AmanClaw Environment Variables\n\
            # LLM_API_KEY=your-api-key-here\n\
            # TELEGRAM_BOT_TOKEN=your-telegram-bot-token\n\
            # DISCORD_BOT_TOKEN=your-discord-bot-token\n\
            # MEMORY_DB_PATH=data/memory.db\n\
            # LOG_FORMAT=json\n";
        std::fs::write(&env_path, env_content).context("Failed to write .env")?;
        println!("  Created .env template");
    }

    for dir in ["data", "plugins", "souls"] {
        let p = PathBuf::from(dir);
        if !p.exists() {
            std::fs::create_dir_all(&p)
                .with_context(|| format!("Failed to create {dir} directory"))?;
            println!("  Created {dir}/");
        }
    }

    println!();
    println!("AmanClaw project initialized!");
    println!();
    println!("Next steps:");
    println!("  1. Edit config.yaml with your LLM settings");
    println!("  2. Set bot tokens in .env");
    println!("  3. Run: amanclaw dev    (mock LLM, no API key needed)");
    println!("  4. Run: amanclaw run    (production mode)");

    Ok(())
}

async fn cmd_dev(config_path: &str, watch: bool) -> Result<()> {
    println!("Starting AmanClaw in development mode...");
    println!("Using mock LLM — no API key required");
    println!();

    if std::env::var("LLM_BASE_URL").is_err() {
        println!("Note: LLM_BASE_URL not set. Using echo mode.");
        println!("      Set LLM_BASE_URL to connect to a real LLM (e.g., Ollama at http://localhost:11434/v1)");
        println!();
    }

    // Keep _watcher alive for the duration of cmd_run by binding at this scope
    let _watcher_guard = if watch {
        let watcher = dev_watcher::DevWatcher::new(config_path)
            .context("Failed to start file watcher")?;
        tracing::info!("Watch mode enabled — monitoring plugins/, souls/, and config for changes");

        let (guard, mut rx) = watcher.into_parts();
        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                match event {
                    dev_watcher::DevEvent::PluginChanged(path) => {
                        tracing::info!(path = %path, "Plugin changed — reload triggered");
                    }
                    dev_watcher::DevEvent::SoulChanged(path) => {
                        tracing::info!(path = %path, "Soul changed — reload triggered");
                    }
                    dev_watcher::DevEvent::ConfigChanged => {
                        tracing::info!("Config changed — restart recommended");
                    }
                }
            }
        });
        Some(guard)
    } else {
        None
    };

    cmd_run(config_path).await
}

fn cmd_check(config_path: &str) -> Result<()> {
    let config_path = find_config(config_path)?;
    let config_str = std::fs::read_to_string(&config_path)
        .with_context(|| format!("Failed to read {}", config_path.display()))?;

    match serde_yaml::from_str::<amanclaw_traits::config::AppConfig>(&config_str) {
        Ok(config) => {
            println!("Config valid: {}", config_path.display());
            println!("  LLM: {} ({})", config.llm.base_url, config.llm.model);
            let disabled = if config.skills.disabled.is_empty() {
                "none".to_string()
            } else {
                config.skills.disabled.join(", ")
            };
            println!("  Skills disabled: {disabled}");
            let agents = if config.agents.is_empty() {
                "default".to_string()
            } else {
                config.agents.len().to_string()
            };
            println!("  Agents: {agents}");
            let scripts = config.script_plugins.len().to_string();
            println!("  Script plugins: {scripts}");
            Ok(())
        }
        Err(e) => {
            println!("Config invalid: {}", config_path.display());
            println!("  Error: {e}");
            std::process::exit(1);
        }
    }
}

fn cmd_skill(action: SkillAction) -> Result<()> {
    match action {
        SkillAction::New { name, lang, output } => {
            let project_dir =
                scaffold::scaffold_skill(&name, &lang, output.as_deref())?;
            println!("Created {} skill '{name}' at {}", lang, project_dir.display());
            Ok(())
        }
        SkillAction::Test { name } => {
            let skill_dir = PathBuf::from(format!("skill-{name}"));
            if !skill_dir.exists() {
                anyhow::bail!(
                    "Skill directory '{}' not found. Run from the parent directory.",
                    skill_dir.display()
                );
            }
            println!("Running tests for skill '{name}'...");
            let status = std::process::Command::new("cargo")
                .arg("test")
                .current_dir(&skill_dir)
                .status()
                .context("Failed to run cargo test")?;
            if !status.success() {
                anyhow::bail!("Tests failed for skill '{name}'");
            }
            Ok(())
        }
    }
}

fn find_config(hint: &str) -> Result<PathBuf> {
    let p = PathBuf::from(hint);
    if p.exists() {
        return Ok(p);
    }

    for name in ["config.yaml", "config.yml"] {
        let p = PathBuf::from(name);
        if p.exists() {
            return Ok(p);
        }
    }

    anyhow::bail!(
        "Config file not found. Tried: {hint}, config.yaml, config.yml\n\n\
         Quick fix:\n\
         1. Run: amanclaw init    (creates config.yaml from template)\n\
         2. Or:  amanclaw -c /path/to/config.yaml run"
    );
}

fn setup_logging(format: Option<&str>) {
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("amanclaw=info"));

    match format {
        Some("json") => {
            tracing_subscriber::fmt()
                .with_env_filter(filter)
                .json()
                .init();
        }
        _ => {
            tracing_subscriber::fmt().with_env_filter(filter).init();
        }
    }
}
