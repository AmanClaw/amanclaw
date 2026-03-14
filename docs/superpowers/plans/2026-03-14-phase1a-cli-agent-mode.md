# Plan 1A: CLI Agent Mode — Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add three CLI commands (`amanclaw ask`, `amanclaw chat`, `amanclaw agent`) that let developers interact with AmanClaw from the terminal without needing a chat app.

**Architecture:** The Engine uses an actor model — `Engine::start()` returns an `EngineHandle` that communicates via channels. For CLI mode, we add a new `EngineCommand::Ask` variant that returns the response via a `oneshot` channel (instead of routing through a chat adapter). A `CliRunner` wraps the engine startup and provides simple `ask()`/`chat()` methods. The CLI commands (`ask`, `chat`, `agent`) are added to the existing clap enum in `cli.rs`.

**Tech Stack:** Rust, clap (CLI), tokio (async), amanclaw-core Engine (actor model)

---

## File Structure

| File | Action | Responsibility |
|------|--------|---------------|
| `rust/crates/amanclaw-core/src/handle.rs` | MODIFY | Add `EngineCommand::Ask` with oneshot response channel |
| `rust/crates/amanclaw-core/src/lib.rs` | MODIFY | Handle `Ask` command in actor loop — process through pipeline, return response |
| `rust/crates/amanclaw-cli/src/cli.rs` | MODIFY | Add `Ask`, `Chat`, `Agent` to Command enum |
| `rust/crates/amanclaw-cli/src/runner.rs` | CREATE | `CliRunner`: starts engine, sends queries, receives responses |
| `rust/crates/amanclaw-cli/src/render.rs` | CREATE | Terminal output: print response, thinking indicator, prompt |
| `rust/crates/amanclaw-cli/src/lib.rs` | CREATE | Library root exporting runner and render modules |
| `rust/crates/amanclaw-cli/src/main.rs` | MODIFY | Add `cmd_ask()`, `cmd_chat()`, `cmd_agent()` handlers |
| `rust/crates/amanclaw-cli/Cargo.toml` | MODIFY | Add `whoami` dependency |
| `rust/crates/amanclaw-core/tests/integration.rs` | MODIFY | Add test for `EngineCommand::Ask` |

---

## Chunk 1: Engine Ask Command

### Task 1: Add EngineCommand::Ask to handle.rs

Add a new command variant that processes a message and returns the response via oneshot.

**Files:**
- Modify: `rust/crates/amanclaw-core/src/handle.rs`

- [ ] **Step 1: Add Ask variant to EngineCommand**

In `rust/crates/amanclaw-core/src/handle.rs`, add to the `EngineCommand` enum:

```rust
/// Process a message and return the response (for CLI / headless use).
Ask(IncomingMessage, oneshot::Sender<Option<amanclaw_traits::message::OutgoingMessage>>),
```

- [ ] **Step 2: Add `ask()` method to EngineHandle**

In `rust/crates/amanclaw-core/src/handle.rs`, add to `impl EngineHandle`:

```rust
/// Send a message and wait for the response.
/// Unlike `send_message`, this returns the pipeline result instead of
/// routing it through a channel adapter.
pub async fn ask(&self, msg: IncomingMessage) -> anyhow::Result<Option<amanclaw_traits::message::OutgoingMessage>> {
    let (tx, rx) = oneshot::channel();
    self.cmd_tx
        .send(EngineCommand::Ask(msg, tx))
        .await
        .map_err(|_| anyhow::anyhow!("engine actor stopped"))?;
    rx.await
        .map_err(|_| anyhow::anyhow!("engine actor dropped response"))
}
```

- [ ] **Step 3: Commit**

```bash
git add rust/crates/amanclaw-core/src/handle.rs
git commit -m "feat(core): add EngineCommand::Ask for headless query-response"
```

---

### Task 2: Handle Ask command in Engine actor loop

**Files:**
- Modify: `rust/crates/amanclaw-core/src/lib.rs`

- [ ] **Step 1: Add Ask handler in run_actor**

In `rust/crates/amanclaw-core/src/lib.rs`, inside the `run_actor` method's `match cmd` block (around line 486), add after the `ProcessMessage` arm:

```rust
EngineCommand::Ask(msg, reply) => {
    messages_processed += 1;
    let _ = status_tx.send(EngineStatus::Running {
        started_at,
        messages_processed,
    });
    let pipeline = self.pipeline.clone();
    let registry = self.registry.clone();
    let agent_router = self.agent_router.clone();
    let semaphore = semaphore.clone();
    join_set.spawn(async move {
        let _permit = semaphore.acquire_owned().await.unwrap();
        let profile = agent_router.resolve(&msg);
        let result = pipeline.process(msg, &registry, &profile).await;
        let _ = reply.send(result.unwrap_or(None));
    });
}
```

- [ ] **Step 2: Run existing tests to ensure nothing breaks**

Run: `cd rust && cargo test --package amanclaw-core -- --nocapture`
Expected: All existing tests PASS

- [ ] **Step 3: Commit**

```bash
git add rust/crates/amanclaw-core/src/lib.rs
git commit -m "feat(core): handle Ask command in engine actor loop"
```

---

### Task 3: Integration test for Engine Ask

**Files:**
- Modify: `rust/crates/amanclaw-core/tests/integration.rs`

- [ ] **Step 1: Write the test**

Add to `rust/crates/amanclaw-core/tests/integration.rs`:

```rust
#[tokio::test]
async fn test_engine_ask_returns_response() {
    let config = test_config();
    let result = Engine::start(config).await.unwrap();
    let handle = result.handle.clone();

    let msg = IncomingMessage {
        user_id: "test-user".into(),
        chat_id: "cli-test".into(),
        platform: "api".into(),
        text: "hello".into(),
        username: Some("tester".into()),
        first_name: None,
        is_group: false,
        image_data: None,
        reply_to: None,
        topic_id: None,
        channel_context: None,
    };

    let response = handle.ask(msg).await.unwrap();
    assert!(response.is_some(), "Ask should return a response");
    let response = response.unwrap();
    assert!(!response.text.is_empty(), "Response text should not be empty");

    handle.shutdown().await.unwrap();
}
```

- [ ] **Step 2: Run the test**

Run: `cd rust && cargo test --package amanclaw-core test_engine_ask_returns_response -- --nocapture`
Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add rust/crates/amanclaw-core/tests/integration.rs
git commit -m "test(core): add integration test for EngineCommand::Ask"
```

---

## Chunk 2: CliRunner and Renderer

### Task 4: Create CliRunner

**Files:**
- Create: `rust/crates/amanclaw-cli/src/runner.rs`
- Create: `rust/crates/amanclaw-cli/src/lib.rs`
- Modify: `rust/crates/amanclaw-cli/Cargo.toml`

- [ ] **Step 1: Add whoami dependency**

In `rust/crates/amanclaw-cli/Cargo.toml`, add:
```toml
whoami = "1"
```

- [ ] **Step 2: Create lib.rs**

Create `rust/crates/amanclaw-cli/src/lib.rs`:
```rust
pub mod render;
pub mod runner;
```

- [ ] **Step 3: Add lib target to Cargo.toml**

In `rust/crates/amanclaw-cli/Cargo.toml`, add:
```toml
[lib]
name = "amanclaw_cli"
path = "src/lib.rs"
```

- [ ] **Step 4: Create runner.rs**

Create `rust/crates/amanclaw-cli/src/runner.rs`:

```rust
use amanclaw_core::Engine;
use amanclaw_core::handle::EngineHandle;
use amanclaw_traits::config::AppConfig;
use amanclaw_traits::message::IncomingMessage;
use anyhow::{Context, Result};
use std::path::PathBuf;

pub struct CliRunner {
    handle: EngineHandle,
    user_id: String,
    _join: tokio::task::JoinHandle<Result<()>>,
}

impl CliRunner {
    /// Create a new CLI runner from a config file path.
    pub async fn from_config(config_path: PathBuf) -> Result<Self> {
        let config_str = std::fs::read_to_string(&config_path)
            .with_context(|| format!("Failed to read {}", config_path.display()))?;
        let config: AppConfig = serde_yaml::from_str(&config_str)
            .with_context(|| "Failed to parse config file")?;

        let result = Engine::start(config).await?;
        let user_id = whoami::username();

        Ok(Self {
            handle: result.handle,
            user_id,
            _join: result.join,
        })
    }

    /// One-shot: send a query, return the response text.
    pub async fn ask(&self, query: &str) -> Result<String> {
        let msg = self.build_message(query);
        let response = self.handle.ask(msg).await?;
        match response {
            Some(r) => Ok(r.text),
            None => Ok("(no response)".into()),
        }
    }

    fn build_message(&self, text: &str) -> IncomingMessage {
        IncomingMessage {
            user_id: self.user_id.clone(),
            chat_id: format!("cli-{}", self.user_id),
            platform: "cli".into(),
            text: text.to_string(),
            username: Some(self.user_id.clone()),
            first_name: None,
            is_group: false,
            image_data: None,
            reply_to: None,
            topic_id: None,
            channel_context: None,
        }
    }

    /// Shutdown the engine gracefully.
    pub async fn shutdown(&self) -> Result<()> {
        self.handle.shutdown().await
    }
}
```

- [ ] **Step 5: Verify it compiles**

Run: `cd rust && cargo check --package amanclaw-cli`
Expected: Compiles with no errors

- [ ] **Step 6: Commit**

```bash
git add rust/crates/amanclaw-cli/src/runner.rs rust/crates/amanclaw-cli/src/lib.rs rust/crates/amanclaw-cli/Cargo.toml
git commit -m "feat(cli): add CliRunner for headless engine interaction"
```

---

### Task 5: Create terminal renderer

**Files:**
- Create: `rust/crates/amanclaw-cli/src/render.rs`

- [ ] **Step 1: Create render.rs**

```rust
use std::io::Write;

/// Print an LLM response to stdout.
pub fn print_response(text: &str) {
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    writeln!(out, "{text}").ok();
}

/// Print a thinking indicator to stderr.
pub fn print_thinking() {
    eprint!("Thinking...");
}

/// Clear the thinking indicator.
pub fn clear_thinking() {
    eprint!("\r            \r");
}

/// Print an error to stderr.
pub fn print_error(err: &anyhow::Error) {
    eprintln!("Error: {err:#}");
}

/// Print the interactive chat prompt.
pub fn print_prompt() {
    eprint!("you> ");
    std::io::stderr().flush().ok();
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cd rust && cargo check --package amanclaw-cli`
Expected: Compiles

- [ ] **Step 3: Commit**

```bash
git add rust/crates/amanclaw-cli/src/render.rs
git commit -m "feat(cli): add terminal output renderer"
```

---

## Chunk 3: CLI Commands

### Task 6: Add Ask, Chat, Agent to clap

**Files:**
- Modify: `rust/crates/amanclaw-cli/src/cli.rs`

- [ ] **Step 1: Add new command variants**

In `rust/crates/amanclaw-cli/src/cli.rs`, add to the `Command` enum:

```rust
/// Ask a one-shot question
Ask {
    /// The question to ask (multiple words joined)
    query: Vec<String>,
},
/// Start interactive chat session
Chat,
/// Run an autonomous agent for a task
Agent {
    /// Task description
    #[arg(short, long)]
    task: String,
    /// Maximum rounds of execution
    #[arg(long, default_value = "10")]
    max_rounds: usize,
},
```

- [ ] **Step 2: Add clap tests for new commands**

Add to the test module in `cli.rs`:

```rust
#[test]
fn test_cli_ask_command() {
    let cli = Cli::parse_from(["amanclaw", "ask", "what", "is", "solat"]);
    match cli.command {
        Some(Command::Ask { query }) => {
            assert_eq!(query, vec!["what", "is", "solat"]);
        }
        _ => panic!("expected Ask command"),
    }
}

#[test]
fn test_cli_chat_command() {
    let cli = Cli::parse_from(["amanclaw", "chat"]);
    assert!(matches!(cli.command, Some(Command::Chat)));
}

#[test]
fn test_cli_agent_command() {
    let cli = Cli::parse_from(["amanclaw", "agent", "--task", "find prayer times"]);
    match cli.command {
        Some(Command::Agent { task, max_rounds }) => {
            assert_eq!(task, "find prayer times");
            assert_eq!(max_rounds, 10);
        }
        _ => panic!("expected Agent command"),
    }
}

#[test]
fn test_cli_agent_custom_rounds() {
    let cli = Cli::parse_from(["amanclaw", "agent", "--task", "do something", "--max-rounds", "5"]);
    match cli.command {
        Some(Command::Agent { task, max_rounds }) => {
            assert_eq!(task, "do something");
            assert_eq!(max_rounds, 5);
        }
        _ => panic!("expected Agent command"),
    }
}
```

- [ ] **Step 3: Run clap tests**

Run: `cd rust && cargo test --package amanclaw-cli cli::tests -- --nocapture`
Expected: All tests PASS

- [ ] **Step 4: Commit**

```bash
git add rust/crates/amanclaw-cli/src/cli.rs
git commit -m "feat(cli): add Ask, Chat, Agent command definitions"
```

---

### Task 7: Implement cmd_ask()

**Files:**
- Modify: `rust/crates/amanclaw-cli/src/main.rs`

- [ ] **Step 1: Add the command handler**

In `main.rs`, add the function:

```rust
async fn cmd_ask(config_path: &str, query: Vec<String>) -> Result<()> {
    let query_text = if query.is_empty() {
        // Read from stdin (piped input)
        use std::io::Read;
        let mut input = String::new();
        std::io::stdin().read_to_string(&mut input)?;
        input.trim().to_string()
    } else {
        query.join(" ")
    };

    if query_text.is_empty() {
        anyhow::bail!("Usage: amanclaw ask <question>\n       echo 'question' | amanclaw ask");
    }

    let config_path = find_config(config_path)?;
    let runner = amanclaw_cli::runner::CliRunner::from_config(config_path).await?;

    amanclaw_cli::render::print_thinking();
    match runner.ask(&query_text).await {
        Ok(response) => {
            amanclaw_cli::render::clear_thinking();
            amanclaw_cli::render::print_response(&response);
        }
        Err(e) => {
            amanclaw_cli::render::clear_thinking();
            amanclaw_cli::render::print_error(&e);
        }
    }

    runner.shutdown().await.ok();
    Ok(())
}
```

- [ ] **Step 2: Wire up in main match**

Add to the match in `main()`:
```rust
Some(Command::Ask { query }) => cmd_ask(&cli.config, query).await,
```

- [ ] **Step 3: Verify it compiles**

Run: `cd rust && cargo check --package amanclaw-cli`
Expected: Compiles

- [ ] **Step 4: Commit**

```bash
git add rust/crates/amanclaw-cli/src/main.rs
git commit -m "feat(cli): implement 'amanclaw ask' one-shot command"
```

---

### Task 8: Implement cmd_chat()

**Files:**
- Modify: `rust/crates/amanclaw-cli/src/main.rs`

- [ ] **Step 1: Add the command handler**

```rust
async fn cmd_chat(config_path: &str) -> Result<()> {
    eprintln!("AmanClaw Chat (type 'exit' or Ctrl+C to quit)\n");

    let config_path = find_config(config_path)?;
    let runner = amanclaw_cli::runner::CliRunner::from_config(config_path).await?;

    loop {
        amanclaw_cli::render::print_prompt();

        let mut input = String::new();
        if std::io::stdin().read_line(&mut input)? == 0 {
            break; // EOF
        }

        let input = input.trim();
        if input.is_empty() {
            continue;
        }
        if input == "exit" || input == "quit" {
            break;
        }

        amanclaw_cli::render::print_thinking();
        match runner.ask(input).await {
            Ok(response) => {
                amanclaw_cli::render::clear_thinking();
                amanclaw_cli::render::print_response(&response);
                eprintln!(); // blank line between exchanges
            }
            Err(e) => {
                amanclaw_cli::render::clear_thinking();
                amanclaw_cli::render::print_error(&e);
            }
        }
    }

    eprintln!("Goodbye!");
    runner.shutdown().await.ok();
    Ok(())
}
```

- [ ] **Step 2: Wire up in main match**

```rust
Some(Command::Chat) => cmd_chat(&cli.config).await,
```

- [ ] **Step 3: Commit**

```bash
git add rust/crates/amanclaw-cli/src/main.rs
git commit -m "feat(cli): implement 'amanclaw chat' interactive REPL"
```

---

### Task 9: Implement cmd_agent()

**Files:**
- Modify: `rust/crates/amanclaw-cli/src/main.rs`

- [ ] **Step 1: Add the command handler**

```rust
async fn cmd_agent(config_path: &str, task: String, max_rounds: usize) -> Result<()> {
    eprintln!("AmanClaw Agent");
    eprintln!("Task: {task}");
    eprintln!("Max rounds: {max_rounds}\n");

    let config_path = find_config(config_path)?;
    let runner = amanclaw_cli::runner::CliRunner::from_config(config_path).await?;

    let prompt = format!(
        "You are an autonomous agent. Complete this task step by step. \
         Use available tools as needed. When done, start your response with TASK_COMPLETE \
         followed by a summary.\n\nTask: {task}"
    );

    for round in 1..=max_rounds {
        eprintln!("[Round {round}/{max_rounds}]");

        amanclaw_cli::render::print_thinking();
        match runner.ask(&prompt).await {
            Ok(response) => {
                amanclaw_cli::render::clear_thinking();
                amanclaw_cli::render::print_response(&response);

                if response.contains("TASK_COMPLETE") {
                    eprintln!("\nTask completed in {round} round(s).");
                    break;
                }
            }
            Err(e) => {
                amanclaw_cli::render::clear_thinking();
                amanclaw_cli::render::print_error(&e);
                break;
            }
        }
    }

    runner.shutdown().await.ok();
    Ok(())
}
```

- [ ] **Step 2: Wire up in main match**

```rust
Some(Command::Agent { task, max_rounds }) => cmd_agent(&cli.config, task, max_rounds).await,
```

- [ ] **Step 3: Commit**

```bash
git add rust/crates/amanclaw-cli/src/main.rs
git commit -m "feat(cli): implement 'amanclaw agent' autonomous task executor"
```

---

## Chunk 4: Polish

### Task 10: Update help text and README

**Files:**
- Modify: `rust/crates/amanclaw-cli/src/cli.rs` — update app about text
- Modify: `README.md` — add CLI section

- [ ] **Step 1: Update clap about text**

In `cli.rs`, change the `about` field:
```rust
about = "AmanClaw — AI personal agent\n\n  amanclaw ask \"question\"        One-shot query\n  amanclaw chat                  Interactive conversation\n  amanclaw agent --task \"do X\"   Autonomous task execution\n  amanclaw run                   Start bot server"
```

- [ ] **Step 2: Add CLI section to README**

Add after the "Highlights" section:

```markdown
## CLI Agent Mode

Use AmanClaw directly from your terminal:

\`\`\`bash
# One-shot question
amanclaw ask "What time is Maghrib in KL?"

# Interactive chat
amanclaw chat

# Autonomous agent
amanclaw agent --task "Find prayer times for today and calculate zakat on RM50,000"

# Piped input
echo "Translate this to BM" | amanclaw ask
\`\`\`
```

- [ ] **Step 3: Commit**

```bash
git add rust/crates/amanclaw-cli/src/cli.rs README.md
git commit -m "docs: add CLI agent mode to help text and README"
```

---

## Summary

| Task | Description | Steps |
|------|-------------|-------|
| 1 | EngineCommand::Ask in handle.rs | 3 |
| 2 | Handle Ask in actor loop | 3 |
| 3 | Integration test for Ask | 3 |
| 4 | CliRunner module | 6 |
| 5 | Terminal renderer | 3 |
| 6 | Clap command definitions + tests | 4 |
| 7 | `amanclaw ask` implementation | 4 |
| 8 | `amanclaw chat` implementation | 3 |
| 9 | `amanclaw agent` implementation | 3 |
| 10 | Help text and README | 3 |

**Total: 10 tasks, 35 steps**

After completing this plan:
```bash
amanclaw ask "your question"           # One-shot
amanclaw chat                          # Interactive REPL
amanclaw agent --task "complex task"   # Autonomous agent
echo "question" | amanclaw ask         # Piped input
```
