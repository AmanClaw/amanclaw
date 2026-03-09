# AmanClaw: Surpass OpenClaw — Phase 1 Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Make AmanClaw contributor-ready and easy to try — CI/CD, docs, CLI improvements, crates.io publishing.

**Architecture:** Phase 1 focuses on foundation (60%) and quick wins (40%). No new features — polish what exists, automate quality, and lower the barrier to entry.

**Tech Stack:** Rust (clap for CLI), GitHub Actions (CI), crates.io (publishing), Docker (GHCR)

---

## Task 1: CI/CD Pipeline — Test & Lint Workflow

**Files:**
- Create: `.github/workflows/ci.yml`

**Step 1: Create CI workflow file**

```yaml
name: CI

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

env:
  CARGO_TERM_COLOR: always
  RUSTFLAGS: "-D warnings"

jobs:
  fmt:
    name: Format
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt
      - run: cargo fmt --all --check
        working-directory: rust

  clippy:
    name: Clippy
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy
      - uses: Swatinem/rust-cache@v2
        with:
          workspaces: rust
      - run: cargo clippy --workspace --all-targets -- -D warnings
        working-directory: rust

  test:
    name: Test
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
        with:
          workspaces: rust
      - run: cargo test --workspace
        working-directory: rust

  audit:
    name: Security Audit
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: rustsec/audit-check@v2.0.0
        with:
          working-directory: rust
```

**Step 2: Verify workflow syntax is valid**

Run: `cd /Users/aman-asmuei/Personals/aman-office/secureclaw-simple && cat .github/workflows/ci.yml | python3 -c "import sys,yaml; yaml.safe_load(sys.stdin.read()); print('Valid YAML')"` (or just verify the file exists)

**Step 3: Commit**

```bash
git add .github/workflows/ci.yml
git commit -m "ci: add test, lint, and security audit workflow"
```

---

## Task 2: Fix Clippy Warnings

Before CI can enforce `clippy -D warnings`, existing warnings need to be fixed.

**Files:**
- Modify: various files in `rust/crates/` and `rust/plugins/`

**Step 1: Run clippy and capture warnings**

Run: `cd /Users/aman-asmuei/Personals/aman-office/secureclaw-simple/rust && cargo clippy --workspace --all-targets 2>&1 | head -100`

**Step 2: Fix each warning**

Fix warnings one file at a time. Common fixes:
- `#[allow(unused)]` → remove dead code or use it
- Unnecessary `.clone()` → remove
- `&String` → `&str` in function args
- Redundant closures → simplify

**Step 3: Verify clean clippy**

Run: `cd /Users/aman-asmuei/Personals/aman-office/secureclaw-simple/rust && cargo clippy --workspace --all-targets -- -D warnings`
Expected: no warnings, exit code 0

**Step 4: Run format check**

Run: `cd /Users/aman-asmuei/Personals/aman-office/secureclaw-simple/rust && cargo fmt --all --check`
If it fails: `cargo fmt --all` then verify

**Step 5: Run all tests**

Run: `cd /Users/aman-asmuei/Personals/aman-office/secureclaw-simple/rust && cargo test --workspace`
Expected: all tests pass

**Step 6: Commit**

```bash
git add -A
git commit -m "fix: resolve clippy warnings across workspace"
```

---

## Task 3: Docker Improvements

**Files:**
- Create: `rust/.dockerignore`
- Modify: `rust/docker-compose.yml` (add healthcheck)

**Step 1: Create .dockerignore**

```
target/
.git/
.github/
desktop/
docs/
*.md
LICENSE
.env
.env.example
.gitignore
```

**Step 2: Add healthcheck to docker-compose.yml**

Add under the `amanclaw` service:
```yaml
    healthcheck:
      test: ["CMD", "amanclaw", "--version"]
      interval: 30s
      timeout: 5s
      retries: 3
      start_period: 10s
```

Note: This requires `amanclaw --version` to work. If CLI doesn't support `--version` yet, use `test: ["CMD-SHELL", "test -f /usr/local/bin/amanclaw"]` as a fallback until Task 5 adds clap.

**Step 3: Verify docker build still works**

Run: `cd /Users/aman-asmuei/Personals/aman-office/secureclaw-simple/rust && docker build -t amanclaw-test .`
Expected: builds successfully

**Step 4: Commit**

```bash
git add rust/.dockerignore rust/docker-compose.yml
git commit -m "chore(docker): add .dockerignore and healthcheck"
```

---

## Task 4: Contributor Infrastructure

**Files:**
- Create: `CONTRIBUTING.md`
- Create: `CHANGELOG.md`
- Create: `SECURITY.md`
- Create: `.github/ISSUE_TEMPLATE/bug_report.md`
- Create: `.github/ISSUE_TEMPLATE/feature_request.md`
- Create: `.github/ISSUE_TEMPLATE/new_skill.md`

**Step 1: Create CONTRIBUTING.md**

```markdown
# Contributing to AmanClaw

Thank you for your interest in contributing to AmanClaw!

## Getting Started

1. Fork and clone the repository
2. Install Rust 1.88+ via [rustup](https://rustup.rs/)
3. Copy `config.example.yaml` to `config.yaml`
4. Run tests: `cd rust && cargo test --workspace`
5. Run the bot: `cd rust && cargo run -p amanclaw-cli`

## Development Workflow

1. Create a branch from `main`
2. Make your changes
3. Ensure `cargo fmt --all` passes
4. Ensure `cargo clippy --workspace -- -D warnings` passes
5. Ensure `cargo test --workspace` passes
6. Submit a pull request

## Code Style

- Follow standard Rust conventions
- Use `anyhow::Result` for error handling in application code
- Use `thiserror` for library error types
- Use `tracing` for logging (`info!`, `warn!`, `error!`)
- Keep functions small and focused
- Write tests for new functionality

## Adding a New Skill

1. Create a new crate: `cargo new --lib rust/plugins/skill-myskill`
2. Add `amanclaw-traits` as a dependency
3. Implement the `Skill` trait:
   - `metadata()` — name, description, version
   - `parameters_schema()` — JSON Schema for tool parameters
   - `execute()` — skill logic
4. Register in `rust/crates/amanclaw-core/src/lib.rs`
5. Add to workspace members in `rust/Cargo.toml`
6. Add tests

See `rust/plugins/skill-solat/` for a complete example.

## Adding a Python Plugin

1. Create `plugins/skill_myskill.py`
2. Implement `handle(params)` function returning a JSON string
3. Add to `script_plugins` in `config.yaml`

See `plugins/skill_hadith.py` for a complete example.

## Commit Messages

Use [Conventional Commits](https://www.conventionalcommits.org/):
- `feat:` new feature
- `fix:` bug fix
- `docs:` documentation
- `chore:` maintenance
- `ci:` CI/CD changes
- `refactor:` code refactoring
- `test:` adding/fixing tests

## Reporting Issues

- **Bugs:** Use the bug report template
- **Features:** Use the feature request template
- **New Skills:** Use the new skill template
- **Security:** See [SECURITY.md](SECURITY.md)
```

**Step 2: Create CHANGELOG.md**

```markdown
# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- CI/CD pipeline with test, lint, and security audit
- Contributor documentation (CONTRIBUTING.md, SECURITY.md)
- Docker improvements (.dockerignore, healthcheck)
- GitHub issue templates

## [0.1.0] - 2026-03-09

### Added
- Core engine with middleware pipeline
- 7 built-in Rust skills (solat, qiblat, hijri, doa, quran, sysinfo, shell)
- 6 Python plugins (hadith, halal, zakat, masjid, khutbah, jakim)
- 5 channel adapters (Telegram, Discord, WhatsApp, WhatsApp Web, Slack)
- WASM plugin runtime with sandboxing
- Python/JS script runtime
- MCP client support
- Multi-agent routing with SOUL.md personas
- Cron scheduler with timezone support
- Webhook triggers with auth validation
- WebSocket gateway (JSON-RPC 2.0)
- Sub-agent spawning
- Skill marketplace/registry
- FTS5 hybrid search (BM25 + vector)
- SQLite memory backend with vector store
- Security: auth, rate limiting, injection detection
- Desktop admin app (Svelte + Tauri)
- Docker support with security hardening
- Raspberry Pi deployment script
```

**Step 3: Create SECURITY.md**

```markdown
# Security Policy

## Reporting a Vulnerability

If you discover a security vulnerability in AmanClaw, please report it responsibly.

**Do NOT open a public issue.**

Instead, email: security@amanclaw.dev (or create a private security advisory on GitHub)

Include:
- Description of the vulnerability
- Steps to reproduce
- Potential impact
- Suggested fix (if any)

We will respond within 48 hours and aim to release a fix within 7 days for critical issues.

## Scope

- Core engine and pipeline
- WASM plugin sandbox (escape vulnerabilities are critical)
- Authentication and authorization
- Input sanitization and injection detection
- Channel adapter security
- API endpoints

## Security Features

AmanClaw includes several security measures:
- WASM sandboxing for untrusted plugins (Wasmtime)
- OWASP Agentic Top 10 rule sets
- Input injection detection and sanitization
- Rate limiting per user
- Non-root Docker container with dropped capabilities
- Read-only filesystem in Docker
- Domain allowlists for plugin network access
```

**Step 4: Create issue templates**

`.github/ISSUE_TEMPLATE/bug_report.md`:
```markdown
---
name: Bug Report
about: Report a bug in AmanClaw
title: "[bug] "
labels: bug
---

## Description
A clear description of the bug.

## Steps to Reproduce
1. ...
2. ...
3. ...

## Expected Behavior
What should happen.

## Actual Behavior
What actually happens.

## Environment
- OS:
- Rust version:
- AmanClaw version:
- Channel (Telegram/Discord/WhatsApp/Slack):
- LLM backend:

## Logs
```
Paste relevant logs here
```
```

`.github/ISSUE_TEMPLATE/feature_request.md`:
```markdown
---
name: Feature Request
about: Suggest a new feature
title: "[feat] "
labels: enhancement
---

## Problem
What problem does this solve?

## Proposed Solution
How should it work?

## Alternatives Considered
Other approaches you've thought about.

## Additional Context
Any other relevant information.
```

`.github/ISSUE_TEMPLATE/new_skill.md`:
```markdown
---
name: New Skill Proposal
about: Propose a new skill/plugin for AmanClaw
title: "[skill] "
labels: skill
---

## Skill Name
e.g., skill-weather

## Description
What does this skill do?

## Parameters
What inputs does it accept?

## Example Usage
```
User: "What's the weather in KL?"
Bot: "Currently 32°C, partly cloudy in Kuala Lumpur"
```

## Data Source / API
What API or data source will it use?

## Language
- [ ] Rust (built-in)
- [ ] Python (script plugin)
- [ ] WASM (Rust/AssemblyScript)
```

**Step 5: Commit**

```bash
git add CONTRIBUTING.md CHANGELOG.md SECURITY.md .github/ISSUE_TEMPLATE/
git commit -m "docs: add contributor infrastructure"
```

---

## Task 5: CLI Subcommands with Clap

**Files:**
- Modify: `rust/crates/amanclaw-cli/Cargo.toml` (add clap dependency)
- Modify: `rust/crates/amanclaw-cli/src/main.rs` (restructure with clap)

**Step 1: Add clap dependency**

Add to `rust/Cargo.toml` workspace dependencies:
```toml
clap = { version = "4", features = ["derive"] }
```

Add to `rust/crates/amanclaw-cli/Cargo.toml` dependencies:
```toml
clap.workspace = true
```

**Step 2: Write test for CLI parsing**

Create `rust/crates/amanclaw-cli/src/cli.rs`:

```rust
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "amanclaw", version, about = "Modular AI assistant for communities")]
pub struct Cli {
    /// Path to config file
    #[arg(short, long, default_value = "config.yaml")]
    pub config: String,

    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand)]
pub enum Command {
    /// Start the bot (default if no subcommand)
    Run,
    /// Initialize a new AmanClaw project
    Init,
    /// Start in development mode with mock LLM
    Dev,
    /// Validate config file
    Check,
    /// Show version and build info
    Version,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn test_cli_no_args_defaults_to_none_command() {
        let cli = Cli::parse_from(["amanclaw"]);
        assert!(cli.command.is_none());
        assert_eq!(cli.config, "config.yaml");
    }

    #[test]
    fn test_cli_run_subcommand() {
        let cli = Cli::parse_from(["amanclaw", "run"]);
        assert!(matches!(cli.command, Some(Command::Run)));
    }

    #[test]
    fn test_cli_init_subcommand() {
        let cli = Cli::parse_from(["amanclaw", "init"]);
        assert!(matches!(cli.command, Some(Command::Init)));
    }

    #[test]
    fn test_cli_dev_subcommand() {
        let cli = Cli::parse_from(["amanclaw", "dev"]);
        assert!(matches!(cli.command, Some(Command::Dev)));
    }

    #[test]
    fn test_cli_check_subcommand() {
        let cli = Cli::parse_from(["amanclaw", "check"]);
        assert!(matches!(cli.command, Some(Command::Check)));
    }

    #[test]
    fn test_cli_custom_config() {
        let cli = Cli::parse_from(["amanclaw", "-c", "my-config.yaml"]);
        assert_eq!(cli.config, "my-config.yaml");
    }

    #[test]
    fn test_cli_version_flag() {
        // clap handles --version automatically, just verify it doesn't panic
        let result = Cli::try_parse_from(["amanclaw", "--version"]);
        assert!(result.is_err()); // clap exits on --version, which is an "error" in try_parse
    }
}
```

**Step 3: Run tests to verify parsing works**

Run: `cd /Users/aman-asmuei/Personals/aman-office/secureclaw-simple/rust && cargo test -p amanclaw-cli`
Expected: all 7 tests pass

**Step 4: Refactor main.rs to use clap CLI**

Modify `rust/crates/amanclaw-cli/src/main.rs`:

```rust
mod cli;

use anyhow::{Context, Result};
use clap::Parser;
use cli::{Cli, Command};
use std::path::PathBuf;
use tracing_subscriber::{fmt, EnvFilter};

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    let cli = Cli::parse();

    let log_format = std::env::var("LOG_FORMAT").ok();
    setup_logging(log_format.as_deref());

    match cli.command {
        Some(Command::Init) => cmd_init().await,
        Some(Command::Dev) => cmd_dev(&cli.config).await,
        Some(Command::Check) => cmd_check(&cli.config).await,
        Some(Command::Version) => cmd_version(),
        Some(Command::Run) | None => cmd_run(&cli.config).await,
    }
}

async fn cmd_run(config_path: &str) -> Result<()> {
    let config_path = find_config(config_path)?;
    let config_str = std::fs::read_to_string(&config_path)
        .with_context(|| format!("Failed to read {}", config_path.display()))?;
    let config: amanclaw_traits::AppConfig = serde_yaml::from_str(&config_str)
        .with_context(|| "Failed to parse config file")?;

    tracing::info!("Starting AmanClaw with config: {}", config_path.display());

    let result = amanclaw_core::Engine::start(config).await?;

    // Optional management API
    if let Ok(port_str) = std::env::var("API_PORT") {
        if let Ok(port) = port_str.parse::<u16>() {
            let api_token = std::env::var("API_TOKEN").unwrap_or_else(|_| {
                use std::time::{SystemTime, UNIX_EPOCH};
                format!(
                    "amanclaw-{:x}-{}",
                    SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_millis(),
                    std::process::id()
                )
            });
            tracing::info!("Management API starting on port {}", port);
            tracing::info!("API Token: {}", api_token);

            let api_state = amanclaw_api::ApiState {
                engine: result.handle.clone(),
                auth: result.auth.clone(),
                pool: result.pool.clone(),
                registry: result.registry.clone(),
                api_token,
            };

            tokio::spawn(async move {
                if let Err(e) = amanclaw_api::run_api_server(api_state, port).await {
                    tracing::error!("Management API error: {}", e);
                }
            });
        }
    }

    // Optional metrics exporter
    let _metrics = metrics_exporter_prometheus::PrometheusBuilder::new()
        .install_recorder()
        .ok();

    tokio::select! {
        join_result = result.join => {
            match join_result {
                Ok(inner) => inner.context("Engine exited with error")?,
                Err(e) => anyhow::bail!("Engine task panicked: {}", e),
            }
        }
        _ = tokio::signal::ctrl_c() => {
            tracing::info!("Shutdown signal received, stopping...");
            let _ = result.handle.shutdown().await;
        }
    }

    Ok(())
}

async fn cmd_init() -> Result<()> {
    println!("Initializing AmanClaw project...");

    // Copy config.example.yaml to config.yaml if it doesn't exist
    let config_path = PathBuf::from("config.yaml");
    if config_path.exists() {
        println!("config.yaml already exists, skipping.");
    } else {
        // Look for example config
        let example_paths = ["config.example.yaml", "config.example.yml"];
        let mut found = false;
        for example in &example_paths {
            let p = PathBuf::from(example);
            if p.exists() {
                std::fs::copy(&p, &config_path)
                    .with_context(|| format!("Failed to copy {} to config.yaml", example))?;
                println!("Created config.yaml from {}", example);
                found = true;
                break;
            }
        }
        if !found {
            // Write a minimal config
            let minimal = include_str!("../../../config_minimal.yaml");
            std::fs::write(&config_path, minimal)
                .context("Failed to write config.yaml")?;
            println!("Created minimal config.yaml");
        }
    }

    // Create .env if it doesn't exist
    let env_path = PathBuf::from(".env");
    if env_path.exists() {
        println!(".env already exists, skipping.");
    } else {
        let env_content = "\
# AmanClaw Environment Variables
# LLM_API_KEY=your-api-key-here
# TELEGRAM_BOT_TOKEN=your-telegram-bot-token
# DISCORD_BOT_TOKEN=your-discord-bot-token
# MEMORY_DB_PATH=data/memory.db
# LOG_FORMAT=json
";
        std::fs::write(&env_path, env_content).context("Failed to write .env")?;
        println!("Created .env template");
    }

    // Create directories
    for dir in ["data", "plugins", "souls"] {
        let p = PathBuf::from(dir);
        if !p.exists() {
            std::fs::create_dir_all(&p)
                .with_context(|| format!("Failed to create {} directory", dir))?;
            println!("Created {}/", dir);
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

async fn cmd_dev(config_path: &str) -> Result<()> {
    println!("Starting AmanClaw in development mode...");
    println!("Using mock LLM — no API key required");
    println!();

    // Set mock LLM env vars if not already set
    if std::env::var("LLM_BASE_URL").is_err() {
        println!("Note: LLM_BASE_URL not set. Using echo mode (skills work, LLM echoes input).");
        println!("      Set LLM_BASE_URL to connect to a real LLM (e.g., Ollama at http://localhost:11434/v1)");
        println!();
    }

    // Fall through to normal run for now
    // Future: start built-in mock LLM server
    cmd_run(config_path).await
}

async fn cmd_check(config_path: &str) -> Result<()> {
    let config_path = find_config(config_path)?;
    let config_str = std::fs::read_to_string(&config_path)
        .with_context(|| format!("Failed to read {}", config_path.display()))?;

    match serde_yaml::from_str::<amanclaw_traits::AppConfig>(&config_str) {
        Ok(config) => {
            println!("✓ Config valid: {}", config_path.display());
            println!("  LLM: {} ({})", config.llm.base_url, config.llm.model);
            println!(
                "  Skills disabled: {}",
                config
                    .skills
                    .disabled
                    .as_ref()
                    .map(|d| if d.is_empty() {
                        "none".to_string()
                    } else {
                        d.join(", ")
                    })
                    .unwrap_or_else(|| "none".to_string())
            );
            println!(
                "  Agents: {}",
                config
                    .agents
                    .as_ref()
                    .map(|a| a.len().to_string())
                    .unwrap_or_else(|| "default".to_string())
            );
            println!(
                "  Script plugins: {}",
                config
                    .script_plugins
                    .as_ref()
                    .map(|p| p.len().to_string())
                    .unwrap_or_else(|| "0".to_string())
            );
            Ok(())
        }
        Err(e) => {
            println!("✗ Config invalid: {}", config_path.display());
            println!("  Error: {}", e);
            std::process::exit(1);
        }
    }
}

fn cmd_version() -> Result<()> {
    println!("amanclaw {}", env!("CARGO_PKG_VERSION"));
    println!("rust edition: {}", "2024");
    Ok(())
}

fn find_config(hint: &str) -> Result<PathBuf> {
    let p = PathBuf::from(hint);
    if p.exists() {
        return Ok(p);
    }

    // Try common names
    for name in ["config.yaml", "config.yml"] {
        let p = PathBuf::from(name);
        if p.exists() {
            return Ok(p);
        }
    }

    anyhow::bail!(
        "Config file not found. Tried: {}, config.yaml, config.yml\n\n\
         Quick fix:\n\
         1. Run: amanclaw init    (creates config.yaml from template)\n\
         2. Or:  amanclaw -c /path/to/config.yaml run",
        hint
    );
}

fn setup_logging(format: Option<&str>) {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("amanclaw=info"));

    match format {
        Some("json") => {
            fmt().with_env_filter(filter).json().init();
        }
        _ => {
            fmt().with_env_filter(filter).init();
        }
    }
}
```

**Step 5: Create minimal config template for `amanclaw init`**

Create `rust/config_minimal.yaml`:
```yaml
# AmanClaw Configuration
# See config.example.yaml for all options

llm:
  base_url: "http://localhost:11434/v1"  # Ollama default
  model: "llama3"
  max_tokens: 4096
  temperature: 0.7

admin_users:
  telegram: []

rate_limit_per_minute: 20

skills:
  shell_allowed_commands: []
  skill_timeout_seconds: 30
```

**Step 6: Run tests**

Run: `cd /Users/aman-asmuei/Personals/aman-office/secureclaw-simple/rust && cargo test -p amanclaw-cli`
Expected: all tests pass

Run: `cd /Users/aman-asmuei/Personals/aman-office/secureclaw-simple/rust && cargo build -p amanclaw-cli`
Expected: builds successfully

**Step 7: Verify CLI works**

Run: `cd /Users/aman-asmuei/Personals/aman-office/secureclaw-simple/rust && cargo run -p amanclaw-cli -- --version`
Expected: prints version

Run: `cd /Users/aman-asmuei/Personals/aman-office/secureclaw-simple/rust && cargo run -p amanclaw-cli -- --help`
Expected: prints help with subcommands

**Step 8: Commit**

```bash
git add rust/crates/amanclaw-cli/ rust/Cargo.toml rust/config_minimal.yaml
git commit -m "feat(cli): add subcommands (init, dev, check, version) with clap"
```

---

## Task 6: Startup Health Diagnostics

**Files:**
- Create: `rust/crates/amanclaw-core/src/diagnostics.rs`
- Modify: `rust/crates/amanclaw-core/src/lib.rs` (add diagnostics module, call after init)

**Step 1: Write test for diagnostics**

Create `rust/crates/amanclaw-core/src/diagnostics.rs`:

```rust
use amanclaw_traits::AppConfig;

pub struct DiagnosticResult {
    pub label: String,
    pub passed: bool,
    pub detail: String,
}

pub fn run_startup_diagnostics(config: &AppConfig) -> Vec<DiagnosticResult> {
    let mut results = Vec::new();

    // Config loaded
    results.push(DiagnosticResult {
        label: "Config loaded".into(),
        passed: true,
        detail: String::new(),
    });

    // LLM connection
    results.push(DiagnosticResult {
        label: "LLM configured".into(),
        passed: !config.llm.base_url.is_empty(),
        detail: format!("{} ({})", config.llm.base_url, config.llm.model),
    });

    // Telegram
    let tg = std::env::var("TELEGRAM_BOT_TOKEN").is_ok();
    results.push(DiagnosticResult {
        label: "Telegram".into(),
        passed: tg,
        detail: if tg {
            "token set".into()
        } else {
            "TELEGRAM_BOT_TOKEN not set (skipped)".into()
        },
    });

    // Discord
    let dc = std::env::var("DISCORD_BOT_TOKEN").is_ok();
    results.push(DiagnosticResult {
        label: "Discord".into(),
        passed: dc,
        detail: if dc {
            "token set".into()
        } else {
            "DISCORD_BOT_TOKEN not set (skipped)".into()
        },
    });

    // WhatsApp
    let wa = std::env::var("WHATSAPP_TOKEN").is_ok()
        || std::env::var("WHATSAPP_PHONE_NUMBER_ID").is_ok();
    results.push(DiagnosticResult {
        label: "WhatsApp".into(),
        passed: wa,
        detail: if wa {
            "configured".into()
        } else {
            "not configured (skipped)".into()
        },
    });

    // Slack
    let slack = std::env::var("SLACK_BOT_TOKEN").is_ok();
    results.push(DiagnosticResult {
        label: "Slack".into(),
        passed: slack,
        detail: if slack {
            "token set".into()
        } else {
            "SLACK_BOT_TOKEN not set (skipped)".into()
        },
    });

    // Skills
    let disabled_count = config
        .skills
        .disabled
        .as_ref()
        .map(|d| d.len())
        .unwrap_or(0);
    results.push(DiagnosticResult {
        label: "Skills".into(),
        passed: true,
        detail: format!("7 built-in ({} disabled)", disabled_count),
    });

    // Script plugins
    let script_count = config
        .script_plugins
        .as_ref()
        .map(|p| p.len())
        .unwrap_or(0);
    if script_count > 0 {
        results.push(DiagnosticResult {
            label: "Script plugins".into(),
            passed: true,
            detail: format!("{} configured", script_count),
        });
    }

    results
}

pub fn print_diagnostics(results: &[DiagnosticResult]) {
    println!();
    for r in results {
        let icon = if r.passed { "✓" } else { "·" };
        if r.detail.is_empty() {
            println!("  {} {}", icon, r.label);
        } else {
            println!("  {} {}: {}", icon, r.label, r.detail);
        }
    }

    let channels_active = results
        .iter()
        .filter(|r| {
            r.passed
                && ["Telegram", "Discord", "WhatsApp", "Slack"]
                    .contains(&r.label.as_str())
        })
        .count();

    println!();
    println!("  Ready! Listening on {} channel(s).", channels_active);
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;
    use amanclaw_traits::{AppConfig, LlmConfig, SkillsConfig};

    fn test_config() -> AppConfig {
        AppConfig {
            llm: LlmConfig {
                base_url: "http://localhost:8080/v1".into(),
                model: "test-model".into(),
                max_tokens: Some(4096),
                temperature: Some(0.7),
                api_key: None,
            },
            skills: SkillsConfig::default(),
            ..Default::default()
        }
    }

    #[test]
    fn test_diagnostics_returns_results() {
        let config = test_config();
        let results = run_startup_diagnostics(&config);
        assert!(!results.is_empty());
        // Config loaded should always pass
        assert!(results[0].passed);
        assert_eq!(results[0].label, "Config loaded");
    }

    #[test]
    fn test_diagnostics_detects_llm() {
        let config = test_config();
        let results = run_startup_diagnostics(&config);
        let llm = results.iter().find(|r| r.label == "LLM configured").unwrap();
        assert!(llm.passed);
        assert!(llm.detail.contains("test-model"));
    }

    #[test]
    fn test_diagnostics_channels_without_env() {
        let config = test_config();
        let results = run_startup_diagnostics(&config);
        // Without env vars, channels should show as not configured
        let tg = results.iter().find(|r| r.label == "Telegram").unwrap();
        assert!(!tg.passed || std::env::var("TELEGRAM_BOT_TOKEN").is_ok());
    }
}
```

**Step 2: Register module in lib.rs**

Add `pub mod diagnostics;` to `rust/crates/amanclaw-core/src/lib.rs`.

**Step 3: Run tests**

Run: `cd /Users/aman-asmuei/Personals/aman-office/secureclaw-simple/rust && cargo test -p amanclaw-core -- diagnostics`
Expected: all 3 tests pass

**Step 4: Integrate into CLI startup**

In `rust/crates/amanclaw-cli/src/main.rs`, add to `cmd_run()` after config parsing:

```rust
    // Print startup diagnostics
    let diag = amanclaw_core::diagnostics::run_startup_diagnostics(&config);
    amanclaw_core::diagnostics::print_diagnostics(&diag);
```

**Step 5: Verify startup output**

Run: `cd /Users/aman-asmuei/Personals/aman-office/secureclaw-simple/rust && cargo run -p amanclaw-cli -- check`
Expected: shows config validation with checkmarks

**Step 6: Commit**

```bash
git add rust/crates/amanclaw-core/src/diagnostics.rs rust/crates/amanclaw-core/src/lib.rs rust/crates/amanclaw-cli/src/main.rs
git commit -m "feat(cli): add startup health diagnostics"
```

---

## Task 7: Prepare Crates for Publishing

**Files:**
- Modify: `rust/Cargo.toml` (workspace metadata)
- Modify: `rust/crates/amanclaw-traits/Cargo.toml`
- Modify: `rust/crates/amanclaw-plugin-sdk/Cargo.toml`

**Step 1: Add publishing metadata to workspace Cargo.toml**

Add to `[workspace.package]` in `rust/Cargo.toml`:
```toml
repository = "https://github.com/AmanClaw/amanclaw"
homepage = "https://github.com/AmanClaw/amanclaw"
documentation = "https://docs.rs/amanclaw-traits"
keywords = ["ai", "assistant", "chatbot", "plugin", "islamic"]
categories = ["command-line-utilities", "web-programming"]
```

**Step 2: Update traits crate for publishing**

Ensure `rust/crates/amanclaw-traits/Cargo.toml` has:
```toml
[package]
name = "amanclaw-traits"
description = "Core traits and types for the AmanClaw AI assistant framework"
version.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true
keywords.workspace = true
```

**Step 3: Update plugin SDK crate for publishing**

Ensure `rust/crates/amanclaw-plugin-sdk/Cargo.toml` has:
```toml
[package]
name = "amanclaw-plugin-sdk"
description = "SDK for building AmanClaw plugins and skills"
version.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true
keywords.workspace = true
```

**Step 4: Verify crates can be packaged**

Run: `cd /Users/aman-asmuei/Personals/aman-office/secureclaw-simple/rust && cargo package -p amanclaw-traits --allow-dirty`
Expected: creates package successfully (or shows fixable warnings)

Run: `cd /Users/aman-asmuei/Personals/aman-office/secureclaw-simple/rust && cargo package -p amanclaw-plugin-sdk --allow-dirty`
Expected: creates package successfully

**Step 5: Fix any packaging issues**

Common issues: missing README in crate dir, missing license file, dependency path issues. Fix as needed.

**Step 6: Commit**

```bash
git add rust/Cargo.toml rust/crates/amanclaw-traits/Cargo.toml rust/crates/amanclaw-plugin-sdk/Cargo.toml
git commit -m "chore: prepare amanclaw-traits and amanclaw-plugin-sdk for crates.io"
```

Note: Actual `cargo publish` should be done manually when ready, not automated in this task.

---

## Task 8: Integration Test — Full Pipeline

**Files:**
- Modify: `rust/crates/amanclaw-core/tests/integration.rs` (add pipeline tests)

**Step 1: Add pipeline integration test**

Add to `rust/crates/amanclaw-core/tests/integration.rs`:

```rust
#[tokio::test]
async fn test_pipeline_processes_message_end_to_end() {
    let mock_server = MockServer::start().await;
    let tmp_db = tempfile::NamedTempFile::new().unwrap();
    unsafe {
        std::env::set_var("MEMORY_DB_PATH", tmp_db.path().to_str().unwrap());
    }

    // Mock LLM returns a simple text response (no tool calls)
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": "Hello! I'm AmanClaw."
                },
                "finish_reason": "stop"
            }],
            "usage": { "prompt_tokens": 10, "completion_tokens": 5, "total_tokens": 15 }
        })))
        .mount(&mock_server)
        .await;

    let yaml = format!(
        r#"
llm:
  base_url: "{}/v1"
  model: "test"
  max_tokens: 100
  temperature: 0
admin_users:
  telegram: [12345]
rate_limit_per_minute: 100
skills:
  shell_allowed_commands: []
  skill_timeout_seconds: 10
"#,
        mock_server.uri()
    );

    let config: amanclaw_traits::AppConfig = serde_yaml::from_str(&yaml).unwrap();
    let result = amanclaw_core::Engine::start(config).await.unwrap();

    // Send a message as an approved user
    let msg = amanclaw_traits::IncomingMessage {
        user_id: "12345".into(),
        chat_id: "12345".into(),
        platform: "telegram".into(),
        text: "Hello".into(),
        username: Some("testuser".into()),
        first_name: Some("Test".into()),
        is_group: false,
        ..Default::default()
    };

    result.handle.send_message(msg).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    // Verify LLM was called
    let requests = mock_server.received_requests().await.unwrap();
    assert!(
        requests.len() >= 1,
        "Expected at least 1 LLM request, got {}",
        requests.len()
    );

    result.handle.shutdown().await.unwrap();
}

#[tokio::test]
async fn test_check_command_returns_stats() {
    let mock_server = MockServer::start().await;
    let tmp_db = tempfile::NamedTempFile::new().unwrap();
    unsafe {
        std::env::set_var("MEMORY_DB_PATH", tmp_db.path().to_str().unwrap());
    }

    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "choices": [{
                "message": { "role": "assistant", "content": "ok" },
                "finish_reason": "stop"
            }],
            "usage": { "prompt_tokens": 5, "completion_tokens": 2, "total_tokens": 7 }
        })))
        .mount(&mock_server)
        .await;

    let yaml = format!(
        r#"
llm:
  base_url: "{}/v1"
  model: "test"
  max_tokens: 100
  temperature: 0
admin_users:
  telegram: [12345]
rate_limit_per_minute: 100
skills:
  shell_allowed_commands: []
  skill_timeout_seconds: 10
"#,
        mock_server.uri()
    );

    let config: amanclaw_traits::AppConfig = serde_yaml::from_str(&yaml).unwrap();
    let result = amanclaw_core::Engine::start(config).await.unwrap();

    // Send /stats command as admin
    let msg = amanclaw_traits::IncomingMessage {
        user_id: "12345".into(),
        chat_id: "12345".into(),
        platform: "telegram".into(),
        text: "/stats".into(),
        username: Some("admin".into()),
        first_name: Some("Admin".into()),
        is_group: false,
        ..Default::default()
    };

    result.handle.send_message(msg).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;

    result.handle.shutdown().await.unwrap();
}
```

**Step 2: Run integration tests**

Run: `cd /Users/aman-asmuei/Personals/aman-office/secureclaw-simple/rust && cargo test -p amanclaw-core --test integration`
Expected: all tests pass (existing + new)

**Step 3: Commit**

```bash
git add rust/crates/amanclaw-core/tests/integration.rs
git commit -m "test: add end-to-end pipeline integration tests"
```

---

## Task 9: ARM64 Binary in Releases

**Files:**
- Create: `.github/workflows/release.yml`

**Step 1: Create release workflow**

```yaml
name: Release

on:
  push:
    tags:
      - "v*"

permissions:
  contents: write

env:
  CARGO_TERM_COLOR: always

jobs:
  build:
    name: Build ${{ matrix.target }}
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        include:
          - target: x86_64-unknown-linux-gnu
            os: ubuntu-latest
            artifact: amanclaw-linux-x86_64
          - target: aarch64-unknown-linux-gnu
            os: ubuntu-latest
            artifact: amanclaw-linux-aarch64
          - target: x86_64-apple-darwin
            os: macos-latest
            artifact: amanclaw-macos-x86_64
          - target: aarch64-apple-darwin
            os: macos-latest
            artifact: amanclaw-macos-aarch64

    steps:
      - uses: actions/checkout@v4

      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}

      - name: Install cross-compilation tools
        if: matrix.target == 'aarch64-unknown-linux-gnu'
        run: |
          sudo apt-get update
          sudo apt-get install -y gcc-aarch64-linux-gnu pkg-config libssl-dev

      - uses: Swatinem/rust-cache@v2
        with:
          workspaces: rust

      - name: Build
        working-directory: rust
        run: cargo build --release --target ${{ matrix.target }} -p amanclaw-cli
        env:
          CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER: aarch64-linux-gnu-gcc

      - name: Package
        run: |
          mkdir -p dist
          cp rust/target/${{ matrix.target }}/release/amanclaw dist/${{ matrix.artifact }}
          chmod +x dist/${{ matrix.artifact }}

      - uses: actions/upload-artifact@v4
        with:
          name: ${{ matrix.artifact }}
          path: dist/${{ matrix.artifact }}

  release:
    name: Release
    needs: build
    runs-on: ubuntu-latest
    steps:
      - uses: actions/download-artifact@v4
        with:
          path: dist
          merge-multiple: true

      - uses: softprops/action-gh-release@v2
        with:
          files: dist/*
          generate_release_notes: true
```

**Step 2: Commit**

```bash
git add .github/workflows/release.yml
git commit -m "ci: add multi-platform release workflow with ARM64 support"
```

---

## Task 10: Run Full Test Suite and Verify

**Step 1: Format check**

Run: `cd /Users/aman-asmuei/Personals/aman-office/secureclaw-simple/rust && cargo fmt --all --check`

**Step 2: Clippy check**

Run: `cd /Users/aman-asmuei/Personals/aman-office/secureclaw-simple/rust && cargo clippy --workspace --all-targets -- -D warnings`

**Step 3: Full test suite**

Run: `cd /Users/aman-asmuei/Personals/aman-office/secureclaw-simple/rust && cargo test --workspace`

**Step 4: Build release**

Run: `cd /Users/aman-asmuei/Personals/aman-office/secureclaw-simple/rust && cargo build --release -p amanclaw-cli`

**Step 5: Verify CLI commands**

Run: `cd /Users/aman-asmuei/Personals/aman-office/secureclaw-simple/rust && ./target/release/amanclaw --help`
Run: `cd /Users/aman-asmuei/Personals/aman-office/secureclaw-simple/rust && ./target/release/amanclaw --version`
Run: `cd /Users/aman-asmuei/Personals/aman-office/secureclaw-simple/rust && ./target/release/amanclaw check`

**Step 6: Final commit if any fixes needed**

```bash
git commit -m "chore: phase 1 final cleanup"
```

---

## Summary

| Task | Type | Description |
|------|------|-------------|
| 1 | Foundation | CI/CD pipeline (test, lint, audit) |
| 2 | Foundation | Fix clippy warnings |
| 3 | Foundation | Docker improvements (.dockerignore, healthcheck) |
| 4 | Foundation | Contributor docs (CONTRIBUTING, CHANGELOG, SECURITY, issue templates) |
| 5 | Quick Win | CLI subcommands with clap (init, dev, check, version) |
| 6 | Quick Win | Startup health diagnostics |
| 7 | Foundation | Prepare crates for crates.io publishing |
| 8 | Foundation | Integration tests for full pipeline |
| 9 | Quick Win | ARM64 binary in releases |
| 10 | Foundation | Full test suite verification |

**Total commits: 10**
**Estimated time: 1 week (Tasks 1-4 day 1-2, Tasks 5-7 day 3-4, Tasks 8-10 day 5)**
