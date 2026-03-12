mod cli;
mod dev_watcher;
mod playground;
mod product_scaffold;
mod scaffold;
mod skill_installer;
mod skill_publisher;

use anyhow::{Context, Result};
use clap::Parser;
use cli::{Cli, Command, ProductAction, SkillAction};
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
        Some(Command::Skill { action }) => cmd_skill(action).await,
        Some(Command::Playground { port }) => playground::run_playground(port).await,
        Some(Command::Product { action }) => cmd_product(action),
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
                channel_manager: Some(result.channel_manager.clone()),
                channels_config: result.channels_config.clone(),
                config_path: Some(config_path.clone()),
                webhook_router: None,
                gateway: None,
                metrics_handle: metrics_handle.clone(),
                admin_password: std::env::var("ADMIN_PASSWORD").ok(),
                jwt_secret: std::env::var("JWT_SECRET").unwrap_or_else(|_| {
                    use rand::Rng;
                    let secret: String = rand::rng()
                        .sample_iter(&rand::distr::Alphanumeric)
                        .take(64)
                        .map(char::from)
                        .collect();
                    secret
                }),
                started_at: std::time::Instant::now(),
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
            std::fs::copy(&example, &config_path).context("Failed to copy config.example.yaml")?;
            println!("  Created config.yaml from config.example.yaml");
        } else {
            let minimal = include_str!("../../../config_minimal.yaml");
            std::fs::write(&config_path, minimal).context("Failed to write config.yaml")?;
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
        println!(
            "      Set LLM_BASE_URL to connect to a real LLM (e.g., Ollama at http://localhost:11434/v1)"
        );
        println!();
    }

    // Keep _watcher alive for the duration of cmd_run by binding at this scope
    let _watcher_guard = if watch {
        let watcher =
            dev_watcher::DevWatcher::new(config_path).context("Failed to start file watcher")?;
        tracing::info!("Watch mode enabled — monitoring plugins/, souls/, and config for changes");

        let (guard, mut rx) = watcher.into_parts();
        tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                match event {
                    dev_watcher::DevEvent::Plugin(path) => {
                        tracing::info!(path = %path, "Plugin changed — reload triggered");
                    }
                    dev_watcher::DevEvent::Soul(path) => {
                        tracing::info!(path = %path, "Soul changed — reload triggered");
                    }
                    dev_watcher::DevEvent::Config => {
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

async fn cmd_skill(action: SkillAction) -> Result<()> {
    match action {
        SkillAction::New { name, lang, output } => {
            let project_dir = scaffold::scaffold_skill(&name, &lang, output.as_deref())?;
            println!(
                "Created {} skill '{name}' at {}",
                lang,
                project_dir.display()
            );
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
        SkillAction::Search { query } => {
            let client = amanclaw_skill_index::IndexClient::new();
            let index = client.fetch_index().await?;
            let results = index.search(&query);
            if results.is_empty() {
                println!("No skills found matching '{query}'.");
            } else {
                println!("Found {} skill(s) matching '{query}':\n", results.len());
                for s in &results {
                    println!(
                        "{} {} v{} — {}",
                        s.tier.badge(),
                        s.name,
                        s.version,
                        s.description
                    );
                    println!(
                        "  repo: {} | lang: {} | tags: {}",
                        s.repo,
                        s.lang,
                        s.tags.join(", ")
                    );
                    println!();
                }
            }
            Ok(())
        }
        SkillAction::Packs => {
            let client = amanclaw_skill_index::IndexClient::new();
            let index = client.fetch_index().await?;
            let names = index.pack_names();
            if names.is_empty() {
                println!("No skill packs available.");
            } else {
                println!("Available skill packs:\n");
                for name in &names {
                    let count = index.packs.get(*name).map(|v| v.len()).unwrap_or(0);
                    println!("  {name} ({count} skills)");
                }
                println!("\nInstall a pack: amanclaw skill install-pack <name>");
            }
            Ok(())
        }
        SkillAction::Install { name, plugins_dir } => {
            let client = amanclaw_skill_index::IndexClient::new();
            let index = client.fetch_index().await?;
            let entry = index.find(&name);
            let (repo, lang) = if let Some(e) = entry {
                (e.repo.clone(), e.lang.clone())
            } else {
                let repo = skill_installer::resolve_repo(&name);
                (repo, "rust".into())
            };
            println!("Installing {name} from {repo}...");
            skill_installer::install_skill(&repo, &name, &lang, std::path::Path::new(&plugins_dir))
                .await?;
            Ok(())
        }
        SkillAction::InstallPack { pack, plugins_dir } => {
            let client = amanclaw_skill_index::IndexClient::new();
            let index = client.fetch_index().await?;
            let skill_names = index.pack_skills(&pack).ok_or_else(|| {
                anyhow::anyhow!(
                    "Pack '{pack}' not found. Use 'amanclaw skill packs' to see available packs."
                )
            })?;
            println!("Installing pack '{pack}' ({} skills)...\n", skill_names.len());
            let dir = std::path::Path::new(&plugins_dir);
            for skill_name in skill_names {
                println!("Installing {skill_name}...");
                let entry = index.find(skill_name);
                let (repo, lang) = if let Some(e) = entry {
                    (e.repo.clone(), e.lang.clone())
                } else {
                    let repo = skill_installer::resolve_repo(skill_name);
                    (repo, "rust".into())
                };
                if let Err(e) =
                    skill_installer::install_skill(&repo, skill_name, &lang, dir).await
                {
                    eprintln!("  Warning: Failed to install {skill_name}: {e}");
                }
            }
            println!("\nPack '{pack}' installation complete.");
            Ok(())
        }
        SkillAction::Publish { path } => {
            let dir = std::path::Path::new(&path);
            let result = skill_publisher::validate_skill(dir)?;
            println!("Skill: {} v{}", result.name, result.version);
            println!("Language: {}", result.language);
            println!("Description: {}", result.description);
            println!();
            if result.warnings.is_empty() {
                println!("Validation: PASSED (eligible for 'verified' tier)");
            } else {
                println!("Warnings:");
                for w in &result.warnings {
                    println!("  - {w}");
                }
                println!("\nValidation: PASSED with warnings (eligible for 'community' tier)");
            }
            println!("\nTo publish:");
            println!("  1. Push your skill to GitHub");
            println!("  2. Create a release with .wasm or .py artifacts");
            println!("  3. Submit a PR to https://github.com/AmanClaw/skill-index");
            println!("     adding your skill entry to index.json");
            Ok(())
        }
    }
}

fn cmd_product(action: ProductAction) -> Result<()> {
    match action {
        ProductAction::New { template, output } => {
            let dir = product_scaffold::scaffold_product(&template, output.as_deref())?;
            println!("Created product '{template}' at {}", dir.display());
            println!();
            println!("Next steps:");
            println!("  1. cd {}", dir.display());
            println!("  2. cp .env.example .env");
            println!("  3. Edit .env with your bot token and LLM settings");
            println!("  4. docker compose up -d");
            Ok(())
        }
        ProductAction::List => {
            let templates = product_scaffold::list_templates();
            println!("Available product templates:\n");
            for (name, desc) in &templates {
                println!("  {name} — {desc}");
            }
            println!("\nCreate one: amanclaw product new <template>");
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
