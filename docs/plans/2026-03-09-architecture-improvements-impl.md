# Architecture Improvements Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Incrementally refactor AmanClaw engine for production safety, performance, and scalability — from desktop to cloud.

**Architecture:** 5-phase incremental refactor. Each phase is independently deployable. No breaking changes to public trait interfaces. All changes are internal implementations.

**Tech Stack:** Rust, Tokio, Axum, SQLx, Wasmtime, dashmap, moka, metrics/metrics-exporter-prometheus, thiserror

---

## Phase 1 — Critical Fixes

### Task 1: Auth Mutex → RwLock

**Files:**
- Modify: `rust/crates/amanclaw-core/src/lib.rs:36,46,185`
- Modify: `rust/crates/amanclaw-core/src/pipeline.rs:14,20,57,108,112,143,317,331`
- Modify: `rust/crates/amanclaw-api/src/state.rs:14`
- Modify: `desktop/src-tauri/src/state.rs:24`

**Step 1: Update Auth import and type in pipeline.rs**

Replace `std::sync::Mutex` with `tokio::sync::RwLock` for Auth throughout pipeline:

```rust
// pipeline.rs line 14: change import
use std::sync::Arc;
// Remove: use std::sync::Mutex;
// (Mutex still used for RateLimiter temporarily)

// pipeline.rs: Pipeline enum — auth field type changes
use tokio::sync::RwLock;

pub enum Pipeline {
    Full {
        auth: Arc<RwLock<Auth>>,        // was Mutex
        rate_limiter: Mutex<RateLimiter>,
        // ... rest unchanged
    },
    Stub,
}
```

**Step 2: Update all auth.lock().unwrap() calls to auth.read().await / auth.write().await**

```rust
// pipeline.rs line 108: read path
let state = auth.read().await.get_user_state(user_id, platform);

// pipeline.rs line 112: write path
auth.write().await.register_user(user_id, platform);

// pipeline.rs line 317: read path (handle_command approve)
auth.write().await.approve_user(target, &msg.platform);

// pipeline.rs line 331: read path (handle_command block)
auth.write().await.block_user(target, &msg.platform);

// pipeline.rs line 332: read path (handle_command users)
let users = auth.read().await.list_users();
```

Note: `handle_command` signature changes to take `&RwLock<Auth>` instead of `&Mutex<Auth>`, and becomes async for the lock calls.

**Step 3: Update Engine struct in lib.rs**

```rust
// lib.rs line 36: change import
use tokio::sync::RwLock;
// Remove std::sync::Mutex import (no longer needed for auth)

// lib.rs line 46: change field type
auth: Arc<RwLock<Auth>>,

// lib.rs line 185: change construction
let auth_arc = Arc::new(RwLock::new(auth));
```

**Step 4: Update ApiState in amanclaw-api**

```rust
// amanclaw-api/src/state.rs line 14: change type
pub auth: Arc<tokio::sync::RwLock<Auth>>,
```

Update all route handlers that use `state.auth.lock().unwrap()` to `state.auth.read().await` or `state.auth.write().await`.

**Step 5: Update desktop state**

```rust
// desktop/src-tauri/src/state.rs line 24: change type
pub auth: Arc<tokio::sync::RwLock<amanclaw_security::auth::Auth>>,
```

Update all desktop IPC commands that use `auth.lock().unwrap()` to `auth.read().await` / `auth.write().await`.

**Step 6: Run tests**

Run: `cd rust && cargo test -p amanclaw-core`
Expected: All existing tests pass.

**Step 7: Commit**

```
feat(core): replace Auth Mutex with RwLock for concurrent reads
```

---

### Task 2: Rate Limiter — Lock-Free with DashMap

**Files:**
- Modify: `rust/crates/amanclaw-security/Cargo.toml`
- Modify: `rust/crates/amanclaw-security/src/rate_limiter.rs`
- Modify: `rust/crates/amanclaw-core/src/pipeline.rs:21,49`

**Step 1: Add dashmap dependency**

```toml
# amanclaw-security/Cargo.toml — add under [dependencies]
dashmap = "6"
```

**Step 2: Rewrite RateLimiter to be lock-free**

```rust
// rate_limiter.rs — full rewrite
use dashmap::DashMap;
use std::time::Instant;

pub struct RateLimiter {
    limit_per_minute: u32,
    windows: DashMap<String, Vec<Instant>>,
}

impl RateLimiter {
    pub fn new(limit_per_minute: u32) -> Self {
        Self {
            limit_per_minute,
            windows: DashMap::new(),
        }
    }

    /// Check if user is within rate limit. Thread-safe, no external lock needed.
    pub fn check(&self, user_id: &str) -> bool {
        let now = Instant::now();
        let cutoff = now - std::time::Duration::from_secs(60);

        let mut entry = self.windows.entry(user_id.to_string()).or_default();
        let timestamps = entry.value_mut();
        timestamps.retain(|t| *t > cutoff);

        if timestamps.len() >= self.limit_per_minute as usize {
            return false;
        }

        timestamps.push(now);
        true
    }
}
```

**Step 3: Remove Mutex wrapper in pipeline.rs**

```rust
// pipeline.rs: Pipeline enum — rate_limiter no longer needs Mutex
pub enum Pipeline {
    Full {
        auth: Arc<RwLock<Auth>>,
        rate_limiter: RateLimiter,  // was Mutex<RateLimiter>
        // ... rest unchanged
    },
    Stub,
}

// pipeline.rs: with_services — remove Mutex::new wrapper
Self::Full {
    auth,
    rate_limiter,  // direct, no Mutex::new()
    // ...
}

// pipeline.rs line 143: remove .lock().unwrap()
if !rate_limiter.check(user_id) {
```

**Step 4: Run tests**

Run: `cd rust && cargo test -p amanclaw-security -p amanclaw-core`
Expected: All tests pass.

**Step 5: Commit**

```
feat(security): lock-free rate limiter with DashMap
```

---

### Task 3: Configurable Tool Rounds

**Files:**
- Modify: `rust/crates/amanclaw-traits/src/agent.rs:5-24`
- Modify: `rust/crates/amanclaw-core/src/pipeline.rs:16,54,190`

**Step 1: Add max_tool_rounds to ContextConfig**

```rust
// amanclaw-traits/src/agent.rs — add field to ContextConfig struct
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextConfig {
    // ... existing fields ...
    #[serde(default = "default_max_tool_rounds")]
    pub max_tool_rounds: usize,
}

fn default_max_tool_rounds() -> usize { 5 }

// Update Default impl to include:
// max_tool_rounds: 5,
```

**Step 2: Remove hardcoded constant and pass from profile**

```rust
// pipeline.rs: remove line 16
// const MAX_TOOL_ROUNDS: usize = 5;  // DELETE THIS LINE

// pipeline.rs line 190: pass profile's max_tool_rounds
let response = Self::tool_calling_loop(
    llm, registry, &mut messages, &tools,
    user_id, platform,
    profile.context.max_tool_rounds,  // NEW PARAM
).await?;

// pipeline.rs: update tool_calling_loop signature
async fn tool_calling_loop(
    llm: &LlmClient,
    registry: &PluginRegistry,
    messages: &mut Vec<serde_json::Value>,
    tools: &[amanclaw_traits::skill::ToolDefinition],
    user_id: &str,
    platform: &str,
    max_rounds: usize,  // NEW PARAM
) -> Result<String> {
    for round in 0..max_rounds {
        // ... rest unchanged
    }
    // ...
}
```

**Step 3: Run tests**

Run: `cd rust && cargo test -p amanclaw-traits -p amanclaw-core`
Expected: All tests pass.

**Step 4: Commit**

```
feat(traits): configurable max_tool_rounds per agent profile
```

---

### Task 4: Token Budget Tracking in Context Engine

**Files:**
- Create: `rust/crates/amanclaw-core/src/token_budget.rs`
- Modify: `rust/crates/amanclaw-core/src/lib.rs:1` (add module)
- Modify: `rust/crates/amanclaw-core/src/context_engine.rs:37-133`
- Modify: `rust/crates/amanclaw-traits/src/agent.rs` (add max_context_tokens to ContextConfig)

**Step 1: Add max_context_tokens to ContextConfig**

```rust
// amanclaw-traits/src/agent.rs — add to ContextConfig
#[serde(default = "default_max_context_tokens")]
pub max_context_tokens: usize,

fn default_max_context_tokens() -> usize { 8000 }

// Update Default impl:
// max_context_tokens: 8000,
```

**Step 2: Create token_budget.rs**

```rust
// rust/crates/amanclaw-core/src/token_budget.rs

/// Simple token estimation. 1 token ≈ 4 characters (conservative).
/// No external tokenizer dependency.
pub fn estimate_tokens(text: &str) -> usize {
    // Average English: ~4 chars per token. Conservative for safety.
    (text.len() + 3) / 4
}

/// Manages a token budget for context building.
/// Priority (highest to lowest): system_prompt > user_message > recent_history > facts > rag > older_history
pub struct TokenBudget {
    max_tokens: usize,
    used: usize,
}

impl TokenBudget {
    pub fn new(max_tokens: usize) -> Self {
        Self { max_tokens, used: 0 }
    }

    /// Reserve tokens for text. Returns true if fits, false if over budget.
    pub fn reserve(&mut self, text: &str) -> bool {
        let cost = estimate_tokens(text);
        if self.used + cost <= self.max_tokens {
            self.used += cost;
            true
        } else {
            false
        }
    }

    /// How many tokens remain.
    pub fn remaining(&self) -> usize {
        self.max_tokens.saturating_sub(self.used)
    }

    /// Force-reserve (for system prompt + user message which must always be included).
    pub fn force_reserve(&mut self, text: &str) {
        self.used += estimate_tokens(text);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_estimate_tokens() {
        assert_eq!(estimate_tokens("hello"), 2); // 5 chars → ~2 tokens
        assert_eq!(estimate_tokens(""), 0);
        assert!(estimate_tokens("a]") <= 1);
    }

    #[test]
    fn test_budget_reserve() {
        let mut budget = TokenBudget::new(100);
        assert!(budget.reserve("short text"));
        assert_eq!(budget.remaining(), 100 - estimate_tokens("short text"));
    }

    #[test]
    fn test_budget_overflow() {
        let mut budget = TokenBudget::new(5);
        // "a]" * 100 should exceed budget
        let long = "a".repeat(100);
        assert!(!budget.reserve(&long));
    }
}
```

**Step 3: Add module to lib.rs**

```rust
// amanclaw-core/src/lib.rs line 1: add
pub mod token_budget;
```

**Step 4: Integrate into StandardContextEngine::build_context()**

```rust
// context_engine.rs — modify build_context() to use TokenBudget
use crate::token_budget::TokenBudget;

async fn build_context(&self, request: ContextRequest) -> Result<ContextResult> {
    let profile = &request.agent_profile;
    let ns = &request.namespace;
    let user_id = &request.user_id;
    let mut budget = TokenBudget::new(profile.context.max_context_tokens);

    // 1. Build system prompt (always included — force reserve)
    let now = chrono::Local::now().format("%Y-%m-%d %H:%M %A").to_string();
    let base = if profile.system_prompt.is_empty() {
        self.base_system_prompt.clone()
    } else {
        profile.system_prompt.clone()
    };
    let mut system = base.replace("{datetime}", &now);
    budget.force_reserve(&system);

    // 2. Prepend summary if available AND fits budget
    if let Ok(Some(summary)) = self.memory.get_summary(ns, user_id).await {
        let section = format!("\n\n## Previous conversation summary\n{}", summary);
        if budget.reserve(&section) {
            system.push_str(&section);
        }
    }

    // 3. Append known facts if they fit
    if let Ok(facts) = self.memory.get_facts(user_id).await {
        if !facts.is_empty() {
            let mut facts_section = "\n\n## Known facts about this user".to_string();
            for (k, v) in &facts {
                let line = format!("\n- {}: {}", k, v);
                if budget.reserve(&line) {
                    facts_section.push_str(&line);
                } else {
                    break; // stop adding facts if budget exceeded
                }
            }
            system.push_str(&facts_section);
        }
    }

    // 4. RAG retrieval if enabled — only add results that fit
    if profile.context.rag_enabled && !profile.context.rag_collections.is_empty() {
        if let Some(ref vs) = self.vector_store {
            let mut all_results = Vec::new();
            // ... (existing RAG search code unchanged) ...

            if !all_results.is_empty() {
                all_results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
                all_results.truncate(profile.context.rag_top_k);
                let mut rag_section = "\n\n## Relevant knowledge".to_string();
                for doc in &all_results {
                    let line = format!("\n- {}", doc.content);
                    if budget.reserve(&line) {
                        rag_section.push_str(&line);
                    } else {
                        break;
                    }
                }
                system.push_str(&rag_section);
            }
        }
    }

    // 5. Build message array
    let mut messages = vec![serde_json::json!({"role": "system", "content": system})];

    // 6. Reserve user message (always included)
    budget.force_reserve(&request.user_message);

    // 7. Add history — from most recent, stop when budget exceeded
    let history = self.memory.get_history(ns, user_id, profile.context.history_limit).await?;
    let mut history_messages = Vec::new();
    for m in history.iter().rev() {
        if budget.reserve(&m.content) {
            history_messages.push(serde_json::json!({"role": m.role, "content": m.content}));
        } else {
            break;
        }
    }
    history_messages.reverse(); // restore chronological order
    messages.extend(history_messages);

    // 8. Add user message (multimodal if image)
    // ... (existing image handling code unchanged) ...

    // 9. Filter tools by agent profile
    let tools = self.registry.get_filtered_tool_definitions(&profile.allowed_skills);

    Ok(ContextResult { messages, tools })
}
```

**Step 5: Run tests**

Run: `cd rust && cargo test -p amanclaw-core`
Expected: All tests pass including existing context_engine tests.

**Step 6: Commit**

```
feat(core): add token budget tracking to context engine
```

---

### Task 5: Bounded Channels + Backpressure in Adapters

**Files:**
- Modify: `rust/crates/amanclaw-traits/src/channel.rs:8-21`
- Modify: `rust/plugins/channel-telegram/src/lib.rs`
- Modify: `rust/plugins/channel-discord/src/lib.rs`
- Modify: `rust/plugins/channel-whatsapp/src/lib.rs`
- Modify: `rust/plugins/channel-whatsapp-web/src/lib.rs`
- Modify: `rust/plugins/channel-slack/src/lib.rs`

**Step 1: Update each adapter's message dispatch to use try_send**

Each adapter currently calls `tx.send(msg).await` when receiving a platform message. Change to `tx.try_send(msg)` with backpressure handling.

The pattern is the same for all adapters. Example for Telegram:

```rust
// In the message handler where tx.send(msg).await is called:
match tx.try_send(msg) {
    Ok(()) => {} // sent successfully
    Err(tokio::sync::mpsc::error::TrySendError::Full(_)) => {
        tracing::warn!("Engine buffer full, dropping message (backpressure)");
        // Reply to user with backpressure message
        if let Err(e) = bot.send_message(
            chat_id,
            "I'm processing too many messages right now. Please try again in a moment."
        ).send().await {
            tracing::error!(error = %e, "Failed to send backpressure reply");
        }
    }
    Err(tokio::sync::mpsc::error::TrySendError::Closed(_)) => {
        tracing::error!("Engine channel closed");
    }
}
```

Apply this pattern to each adapter's message dispatch point. The exact location differs per adapter — find where `tx.send(incoming_msg).await` is called and replace it.

**Step 2: Run tests**

Run: `cd rust && cargo test`
Expected: All tests pass.

**Step 3: Commit**

```
feat(adapters): add backpressure handling with try_send
```

---

## Phase 2 — Engine Actor Model

### Task 6: Define EngineCommand and EngineStatus Types

**Files:**
- Create: `rust/crates/amanclaw-core/src/handle.rs`
- Modify: `rust/crates/amanclaw-core/src/lib.rs:1` (add module)

**Step 1: Create handle.rs with types**

```rust
// rust/crates/amanclaw-core/src/handle.rs

use amanclaw_traits::message::IncomingMessage;
use amanclaw_traits::skill::SkillMetadata;
use crate::scheduler::SchedulerEvent;
use tokio::sync::{mpsc, oneshot, watch};
use std::time::Instant;

/// Commands sent to the engine actor.
pub enum EngineCommand {
    ProcessMessage(IncomingMessage),
    SchedulerEvent(SchedulerEvent),
    GetStatus(oneshot::Sender<EngineStatus>),
    GetSkills(oneshot::Sender<Vec<SkillMetadata>>),
    Shutdown(oneshot::Sender<()>),
}

/// Engine status, broadcast via watch channel.
#[derive(Debug, Clone)]
pub enum EngineStatus {
    Stopped,
    Starting,
    Running { started_at: Instant, messages_processed: u64 },
    Error(String),
}

impl EngineStatus {
    pub fn is_running(&self) -> bool {
        matches!(self, Self::Running { .. })
    }
}

/// Cheap, cloneable handle to the engine actor.
#[derive(Clone)]
pub struct EngineHandle {
    cmd_tx: mpsc::Sender<EngineCommand>,
    status_rx: watch::Receiver<EngineStatus>,
}

impl EngineHandle {
    pub fn new(
        cmd_tx: mpsc::Sender<EngineCommand>,
        status_rx: watch::Receiver<EngineStatus>,
    ) -> Self {
        Self { cmd_tx, status_rx }
    }

    /// Send a message into the engine for processing.
    pub async fn send_message(&self, msg: IncomingMessage) -> anyhow::Result<()> {
        self.cmd_tx.send(EngineCommand::ProcessMessage(msg)).await
            .map_err(|_| anyhow::anyhow!("engine actor stopped"))
    }

    /// Get current engine status (non-blocking).
    pub fn status(&self) -> EngineStatus {
        self.status_rx.borrow().clone()
    }

    /// Request skill list from actor.
    pub async fn skills(&self) -> anyhow::Result<Vec<SkillMetadata>> {
        let (tx, rx) = oneshot::channel();
        self.cmd_tx.send(EngineCommand::GetSkills(tx)).await
            .map_err(|_| anyhow::anyhow!("engine actor stopped"))?;
        rx.await.map_err(|_| anyhow::anyhow!("engine actor dropped response"))
    }

    /// Request graceful shutdown.
    pub async fn shutdown(&self) -> anyhow::Result<()> {
        let (tx, rx) = oneshot::channel();
        self.cmd_tx.send(EngineCommand::Shutdown(tx)).await
            .map_err(|_| anyhow::anyhow!("engine already stopped"))?;
        rx.await.map_err(|_| anyhow::anyhow!("engine dropped shutdown response"))
    }

    /// Get a sender for adapters to push messages.
    pub fn sender(&self) -> mpsc::Sender<EngineCommand> {
        self.cmd_tx.clone()
    }
}
```

**Step 2: Add module**

```rust
// amanclaw-core/src/lib.rs — add at top
pub mod handle;
```

**Step 3: Run tests**

Run: `cd rust && cargo test -p amanclaw-core`
Expected: Compiles and tests pass (no behavior change yet).

**Step 4: Commit**

```
feat(core): add EngineHandle and EngineCommand types
```

---

### Task 7: Refactor Engine into Actor Pattern

**Files:**
- Modify: `rust/crates/amanclaw-core/src/lib.rs`

**Step 1: Refactor Engine::new() to return EngineHandle**

The key change: `Engine::new()` returns `(EngineHandle, JoinHandle<Result<()>>)` instead of `Engine`. The engine runs itself on a background task.

```rust
// amanclaw-core/src/lib.rs — restructured

use crate::handle::{EngineCommand, EngineHandle, EngineStatus};
use tokio::sync::{mpsc, watch, Semaphore};

pub struct Engine {
    config: AppConfig,
    pipeline: Arc<Pipeline>,  // now Arc for sharing across tasks
    registry: Arc<PluginRegistry>,
    channels: Vec<Arc<dyn Channel>>,
    auth: Arc<RwLock<Auth>>,
    pool: SqlitePool,
    agent_router: Arc<AgentRouter>,  // now Arc
}

impl Engine {
    /// Create engine and start it as a background actor.
    /// Returns a handle for sending commands and a JoinHandle for the actor task.
    pub async fn start(config: AppConfig) -> Result<(EngineHandle, tokio::task::JoinHandle<Result<()>>)> {
        // ... existing Engine::new() initialization code ...
        // (all the skill registration, adapter startup, etc.)

        let (cmd_tx, cmd_rx) = mpsc::channel::<EngineCommand>(512);
        let (status_tx, status_rx) = watch::channel(EngineStatus::Starting);
        let handle = EngineHandle::new(cmd_tx.clone(), status_rx);

        // Adapters send messages via cmd_tx
        // (modify adapter startup to use cmd_tx wrapped to send EngineCommand::ProcessMessage)
        let adapter_tx = cmd_tx.clone();
        // ... start adapters with a wrapper that converts IncomingMessage → EngineCommand::ProcessMessage ...

        let engine = Engine { config, pipeline, registry, channels, auth, pool, agent_router };
        let max_concurrent = 32; // configurable later

        let join = tokio::spawn(async move {
            engine.run_actor(cmd_rx, status_tx, sched_rx, max_concurrent).await
        });

        Ok((handle, join))
    }

    async fn run_actor(
        self,
        mut cmd_rx: mpsc::Receiver<EngineCommand>,
        status_tx: watch::Sender<EngineStatus>,
        mut sched_rx: mpsc::Receiver<crate::scheduler::SchedulerEvent>,
        max_concurrent: usize,
    ) -> Result<()> {
        let semaphore = Arc::new(Semaphore::new(max_concurrent));
        let mut messages_processed: u64 = 0;
        let started_at = std::time::Instant::now();

        let _ = status_tx.send(EngineStatus::Running { started_at, messages_processed: 0 });
        tracing::info!("Engine actor running");

        loop {
            tokio::select! {
                Some(cmd) = cmd_rx.recv() => {
                    match cmd {
                        EngineCommand::ProcessMessage(msg) => {
                            messages_processed += 1;
                            let _ = status_tx.send(EngineStatus::Running { started_at, messages_processed });

                            let permit = semaphore.clone().acquire_owned().await.unwrap();
                            let pipeline = self.pipeline.clone();
                            let registry = self.registry.clone();
                            let router = self.agent_router.clone();
                            let channels = self.channels.clone();

                            tokio::spawn(async move {
                                let _permit = permit;
                                let platform = msg.platform.clone();
                                let profile = router.resolve(&msg);
                                match pipeline.process(msg, &registry, &profile).await {
                                    Ok(Some(response)) => {
                                        Self::send_to_channel_static(&channels, &platform, response).await;
                                    }
                                    Ok(None) => {}
                                    Err(e) => tracing::error!(error = %e, "Pipeline error"),
                                }
                            });
                        }
                        EngineCommand::GetStatus(reply) => {
                            let _ = reply.send(EngineStatus::Running { started_at, messages_processed });
                        }
                        EngineCommand::GetSkills(reply) => {
                            let skills = self.registry.list_skill_metadata();
                            let _ = reply.send(skills);
                        }
                        EngineCommand::Shutdown(reply) => {
                            tracing::info!("Engine shutdown requested");
                            let _ = status_tx.send(EngineStatus::Stopped);
                            let _ = reply.send(());
                            break;
                        }
                        EngineCommand::SchedulerEvent(event) => {
                            // Handle same as current sched_rx arm
                            match event {
                                crate::scheduler::SchedulerEvent::SendMessage(response) => {
                                    let platform = response.platform.clone().unwrap_or_default();
                                    Self::send_to_channel_static(&self.channels, &platform, response).await;
                                }
                                crate::scheduler::SchedulerEvent::InjectMessage(msg) => {
                                    let platform = msg.platform.clone();
                                    let profile = self.agent_router.resolve(&msg);
                                    match self.pipeline.process(msg, &self.registry, &profile).await {
                                        Ok(Some(response)) => {
                                            Self::send_to_channel_static(&self.channels, &platform, response).await;
                                        }
                                        Ok(None) => {}
                                        Err(e) => tracing::error!(error = %e, "Cron pipeline error"),
                                    }
                                }
                            }
                        }
                    }
                }
                Some(event) = sched_rx.recv() => {
                    // Forward scheduler events as commands
                    // (handled above when received as EngineCommand::SchedulerEvent)
                    match event {
                        crate::scheduler::SchedulerEvent::SendMessage(response) => {
                            let platform = response.platform.clone().unwrap_or_default();
                            Self::send_to_channel_static(&self.channels, &platform, response).await;
                        }
                        crate::scheduler::SchedulerEvent::InjectMessage(msg) => {
                            let platform = msg.platform.clone();
                            let profile = self.agent_router.resolve(&msg);
                            match self.pipeline.process(msg, &self.registry, &profile).await {
                                Ok(Some(response)) => {
                                    Self::send_to_channel_static(&self.channels, &platform, response).await;
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

    async fn send_to_channel_static(channels: &[Arc<dyn Channel>], platform: &str, response: amanclaw_traits::message::OutgoingMessage) {
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
```

**Step 2: Add list_skill_metadata to PluginRegistry**

```rust
// registry.rs — add method
pub fn list_skill_metadata(&self) -> Vec<SkillMetadata> {
    self.skills.values().map(|s| s.metadata()).collect()
}
```

**Step 3: Update CLI to use new Engine::start()**

```rust
// amanclaw-cli/src/main.rs — update
let (handle, engine_task) = Engine::start(config).await?;
engine_task.await??;
```

**Step 4: Run tests**

Run: `cd rust && cargo test -p amanclaw-core`
Expected: Tests need updating to use new API. Update integration tests to use `Engine::start()`.

**Step 5: Commit**

```
feat(core): refactor Engine to actor model with EngineHandle
```

---

### Task 8: Update Desktop to Use EngineHandle

**Files:**
- Modify: `desktop/src-tauri/src/state.rs`
- Modify: `desktop/src-tauri/src/lib.rs`
- Modify: desktop IPC command files

**Step 1: Simplify desktop EngineHandle**

```rust
// desktop/src-tauri/src/state.rs — replace EngineHandle
pub struct DesktopEngineState {
    pub handle: amanclaw_core::handle::EngineHandle,
    pub join: tokio::task::JoinHandle<anyhow::Result<()>>,
    pub pool: sqlx::SqlitePool,      // kept for direct DB queries (communities, etc.)
    pub auth: Arc<tokio::sync::RwLock<Auth>>,  // kept for user management routes
    pub registry: Arc<PluginRegistry>,         // kept for skill listing
}

pub struct AppState {
    pub mode: AppMode,
    pub engine: Option<DesktopEngineState>,
    pub config: Option<AppConfig>,
    pub started_at: Option<std::time::Instant>,
    pub logs: VecDeque<LogEntry>,
}
```

**Step 2: Update engine startup in lib.rs**

```rust
// desktop/src-tauri/src/lib.rs — update auto-start
let (handle, join) = amanclaw_core::Engine::start(config.clone()).await?;
// Store DesktopEngineState with handle, join, pool, auth, registry
```

**Step 3: Update IPC commands to use handle.status()**

Replace `engine_status` field with `handle.status()` calls. No more manual status tracking.

**Step 4: Run desktop build**

Run: `cd desktop && cargo tauri build --debug`
Expected: Compiles successfully.

**Step 5: Commit**

```
feat(desktop): use EngineHandle actor pattern
```

---

## Phase 3 — Pipeline Middleware

### Task 9: Define Middleware Trait and Context

**Files:**
- Create: `rust/crates/amanclaw-core/src/middleware.rs`
- Modify: `rust/crates/amanclaw-core/src/lib.rs:1`

**Step 1: Create middleware.rs**

```rust
// rust/crates/amanclaw-core/src/middleware.rs

use amanclaw_traits::agent::AgentProfile;
use amanclaw_traits::message::{IncomingMessage, OutgoingMessage};
use anyhow::Result;
use std::any::{Any, TypeId};
use std::collections::HashMap;

/// Type-safe extension map for middleware to share data.
#[derive(Default)]
pub struct Extensions {
    map: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
}

impl Extensions {
    pub fn insert<T: Send + Sync + 'static>(&mut self, val: T) {
        self.map.insert(TypeId::of::<T>(), Box::new(val));
    }

    pub fn get<T: Send + Sync + 'static>(&self) -> Option<&T> {
        self.map.get(&TypeId::of::<T>()).and_then(|b| b.downcast_ref())
    }
}

/// Context passed through the middleware chain.
pub struct PipelineContext {
    pub msg: IncomingMessage,
    pub profile: AgentProfile,
    pub is_internal: bool,
    pub extensions: Extensions,
}

impl PipelineContext {
    pub fn new(msg: IncomingMessage, profile: AgentProfile) -> Self {
        let is_internal = msg.is_cron || msg.is_webhook || msg.is_subagent;
        Self { msg, profile, is_internal, extensions: Extensions::default() }
    }
}

/// Middleware trait — process a message and optionally call next.
#[async_trait::async_trait]
pub trait PipelineMiddleware: Send + Sync {
    async fn process(
        &self,
        ctx: PipelineContext,
        next: &MiddlewareChain,
    ) -> Result<Option<OutgoingMessage>>;
}

/// Chain of middleware. Calls each in order.
pub struct MiddlewareChain {
    middlewares: Vec<Box<dyn PipelineMiddleware>>,
}

impl MiddlewareChain {
    pub fn new(middlewares: Vec<Box<dyn PipelineMiddleware>>) -> Self {
        Self { middlewares }
    }

    pub async fn execute(&self, ctx: PipelineContext) -> Result<Option<OutgoingMessage>> {
        self.execute_from(0, ctx).await
    }

    pub async fn execute_from(&self, index: usize, ctx: PipelineContext) -> Result<Option<OutgoingMessage>> {
        if index >= self.middlewares.len() {
            return Ok(None); // end of chain
        }
        let rest = MiddlewareChain {
            middlewares: self.middlewares[index + 1..].iter().collect(), // won't compile — see step 2
        };
        // See step 2 for actual implementation using indices
        self.middlewares[index].process(ctx, &rest).await
    }
}
```

Note: The chain needs to be implemented with index passing rather than slicing, since `Box<dyn>` can't be easily sliced. Use an index-based approach:

```rust
pub struct MiddlewareChain {
    middlewares: Arc<Vec<Box<dyn PipelineMiddleware>>>,
    start_index: usize,
}

impl MiddlewareChain {
    pub fn new(middlewares: Vec<Box<dyn PipelineMiddleware>>) -> Self {
        Self { middlewares: Arc::new(middlewares), start_index: 0 }
    }

    pub async fn execute(&self, ctx: PipelineContext) -> Result<Option<OutgoingMessage>> {
        if self.start_index >= self.middlewares.len() {
            return Ok(None);
        }
        let next = MiddlewareChain {
            middlewares: self.middlewares.clone(),
            start_index: self.start_index + 1,
        };
        self.middlewares[self.start_index].process(ctx, &next).await
    }
}
```

**Step 2: Run tests**

Run: `cd rust && cargo test -p amanclaw-core`
Expected: Compiles. No behavior change yet.

**Step 3: Commit**

```
feat(core): add PipelineMiddleware trait and MiddlewareChain
```

---

### Task 10: Implement Individual Middleware Components

**Files:**
- Create: `rust/crates/amanclaw-core/src/middleware/auth.rs`
- Create: `rust/crates/amanclaw-core/src/middleware/command.rs`
- Create: `rust/crates/amanclaw-core/src/middleware/rate_limit.rs`
- Create: `rust/crates/amanclaw-core/src/middleware/sanitize.rs`
- Create: `rust/crates/amanclaw-core/src/middleware/context.rs`
- Create: `rust/crates/amanclaw-core/src/middleware/tool_calling.rs`
- Create: `rust/crates/amanclaw-core/src/middleware/persist.rs`

Convert `middleware.rs` to `middleware/mod.rs` and add submodules.

**Step 1: Convert to module directory**

Move `middleware.rs` → `middleware/mod.rs`. Add submodule declarations.

**Step 2: Extract AuthMiddleware from pipeline.rs lines 106-133**

```rust
// middleware/auth.rs
pub struct AuthMiddleware {
    auth: Arc<RwLock<Auth>>,
}

#[async_trait]
impl PipelineMiddleware for AuthMiddleware {
    async fn process(&self, mut ctx: PipelineContext, next: &MiddlewareChain) -> Result<Option<OutgoingMessage>> {
        if ctx.is_internal {
            return next.execute(ctx).await;
        }
        // ... extract auth check logic from pipeline.rs lines 106-133 ...
        // On Admin/Approved → call next.execute(ctx).await
        // On Blocked → return Ok(None)
        // On New/Pending → return Ok(Some(registration message))
    }
}
```

**Step 3: Extract each middleware similarly**

Each middleware extracts one section from `process_full()`:
- `CommandMiddleware` — lines 136-139 (handle_command)
- `RateLimitMiddleware` — lines 142-155
- `SanitizeMiddleware` — lines 158-170
- `ContextMiddleware` — lines 176-187 (calls context_engine.build_context)
- `ToolCallingMiddleware` — lines 189-190 (calls tool_calling_loop)
- `PersistMiddleware` — lines 192-208 (save exchange + auto-summarize)

**Step 4: Wire up in Pipeline**

```rust
// pipeline.rs — rebuild Pipeline to use MiddlewareChain
impl Pipeline {
    pub fn with_services(/* same args */) -> Self {
        let chain = MiddlewareChain::new(vec![
            Box::new(AuthMiddleware::new(auth.clone())),
            Box::new(CommandMiddleware::new(auth.clone(), memory.clone())),
            Box::new(RateLimitMiddleware::new(rate_limiter)),
            Box::new(SanitizeMiddleware::new(emitter.clone())),
            Box::new(ContextMiddleware::new(context_engine.clone())),
            Box::new(ToolCallingMiddleware::new(llm.clone())),
            Box::new(PersistMiddleware::new(context_engine.clone(), memory.clone(), llm.clone())),
        ]);
        Self::Full { chain }
    }

    pub async fn process(&self, msg: IncomingMessage, registry: &PluginRegistry, profile: &AgentProfile) -> Result<Option<OutgoingMessage>> {
        match self {
            Self::Stub => self.process_stub(msg).await,
            Self::Full { chain } => {
                let ctx = PipelineContext::new(msg, profile.clone());
                // Store registry in extensions so ToolCallingMiddleware can access it
                ctx.extensions.insert(registry.clone());
                chain.execute(ctx).await
            }
        }
    }
}
```

**Step 5: Run all tests**

Run: `cd rust && cargo test -p amanclaw-core`
Expected: All existing tests pass with same behavior.

**Step 6: Commit**

```
feat(core): refactor pipeline into middleware chain
```

---

## Phase 4 — Database Abstraction + Caching

### Task 11: Add Feature Flags to amanclaw-memory

**Files:**
- Modify: `rust/crates/amanclaw-memory/Cargo.toml`
- Modify: `rust/crates/amanclaw-memory/src/lib.rs`

**Step 1: Add feature flags**

```toml
# amanclaw-memory/Cargo.toml
[features]
default = ["sqlite"]
sqlite = ["sqlx/sqlite"]
postgres = ["sqlx/postgres"]
```

**Step 2: Gate sqlite.rs behind feature**

```rust
// amanclaw-memory/src/lib.rs
#[cfg(feature = "sqlite")]
pub mod sqlite;

#[cfg(feature = "postgres")]
pub mod postgres;

pub mod vector;
pub mod community;
```

**Step 3: Run tests**

Run: `cd rust && cargo test -p amanclaw-memory`
Expected: Tests pass (sqlite feature enabled by default).

**Step 4: Commit**

```
feat(memory): add feature flags for sqlite/postgres backends
```

---

### Task 12: Add CachedMemory Wrapper

**Files:**
- Modify: `rust/crates/amanclaw-memory/Cargo.toml`
- Create: `rust/crates/amanclaw-memory/src/cached.rs`
- Modify: `rust/crates/amanclaw-memory/src/lib.rs`
- Modify: `rust/crates/amanclaw-core/src/lib.rs` (wrap memory in cache)

**Step 1: Add moka dependency**

```toml
# amanclaw-memory/Cargo.toml — add
moka = { version = "0.12", features = ["future"] }
```

**Step 2: Create cached.rs**

```rust
// rust/crates/amanclaw-memory/src/cached.rs

use amanclaw_traits::memory::{HistoryMessage, MemoryBackend};
use anyhow::Result;
use moka::future::Cache;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

/// Cache wrapper around any MemoryBackend.
/// Caches history, facts, and summaries with TTL-based expiration.
pub struct CachedMemory {
    inner: Arc<dyn MemoryBackend>,
    history_cache: Cache<String, Vec<HistoryMessage>>,   // key: "ns:user_id"
    facts_cache: Cache<String, HashMap<String, String>>,  // key: user_id
    summary_cache: Cache<String, Option<String>>,          // key: "ns:user_id"
}

impl CachedMemory {
    pub fn new(inner: Arc<dyn MemoryBackend>, max_entries: u64, ttl_seconds: u64) -> Self {
        let ttl = Duration::from_secs(ttl_seconds);
        Self {
            inner,
            history_cache: Cache::builder().max_capacity(max_entries).time_to_live(ttl).build(),
            facts_cache: Cache::builder().max_capacity(max_entries).time_to_live(ttl).build(),
            summary_cache: Cache::builder().max_capacity(max_entries).time_to_live(ttl).build(),
        }
    }

    fn cache_key(ns: &str, user_id: &str) -> String {
        format!("{}:{}", ns, user_id)
    }
}

#[async_trait::async_trait]
impl MemoryBackend for CachedMemory {
    async fn save_exchange(&self, ns: &str, user_id: &str, platform: &str, user_msg: &str, assistant_msg: &str) -> Result<()> {
        // Invalidate history cache on write
        self.history_cache.invalidate(&Self::cache_key(ns, user_id)).await;
        self.inner.save_exchange(ns, user_id, platform, user_msg, assistant_msg).await
    }

    async fn get_history(&self, ns: &str, user_id: &str, limit: i64) -> Result<Vec<HistoryMessage>> {
        let key = Self::cache_key(ns, user_id);
        if let Some(cached) = self.history_cache.get(&key).await {
            return Ok(cached);
        }
        let result = self.inner.get_history(ns, user_id, limit).await?;
        self.history_cache.insert(key, result.clone()).await;
        Ok(result)
    }

    async fn clear_history(&self, ns: &str, user_id: &str) -> Result<()> {
        self.history_cache.invalidate(&Self::cache_key(ns, user_id)).await;
        self.inner.clear_history(ns, user_id).await
    }

    async fn get_message_count(&self, ns: &str, user_id: &str) -> Result<i64> {
        self.inner.get_message_count(ns, user_id).await
    }

    async fn save_fact(&self, user_id: &str, key: &str, value: &str) -> Result<()> {
        self.facts_cache.invalidate(user_id).await;
        self.inner.save_fact(user_id, key, value).await
    }

    async fn get_facts(&self, user_id: &str) -> Result<HashMap<String, String>> {
        if let Some(cached) = self.facts_cache.get(user_id).await {
            return Ok(cached);
        }
        let result = self.inner.get_facts(user_id).await?;
        self.facts_cache.insert(user_id.to_string(), result.clone()).await;
        Ok(result)
    }

    async fn delete_fact(&self, user_id: &str, key: &str) -> Result<bool> {
        self.facts_cache.invalidate(user_id).await;
        self.inner.delete_fact(user_id, key).await
    }

    async fn get_summary(&self, ns: &str, user_id: &str) -> Result<Option<String>> {
        let key = Self::cache_key(ns, user_id);
        if let Some(cached) = self.summary_cache.get(&key).await {
            return Ok(cached);
        }
        let result = self.inner.get_summary(ns, user_id).await?;
        self.summary_cache.insert(key, result.clone()).await;
        Ok(result)
    }

    async fn save_summary_and_prune(&self, ns: &str, user_id: &str, summary: &str, keep_recent: i64) -> Result<()> {
        let key = Self::cache_key(ns, user_id);
        self.summary_cache.invalidate(&key).await;
        self.history_cache.invalidate(&key).await;
        self.inner.save_summary_and_prune(ns, user_id, summary, keep_recent).await
    }

    async fn needs_summarization(&self, ns: &str, user_id: &str, threshold: i64) -> Result<bool> {
        self.inner.needs_summarization(ns, user_id, threshold).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    struct CountingMemory {
        call_count: Mutex<u32>,
    }

    impl CountingMemory {
        fn new() -> Self { Self { call_count: Mutex::new(0) } }
        fn calls(&self) -> u32 { *self.call_count.lock().unwrap() }
    }

    #[async_trait::async_trait]
    impl MemoryBackend for CountingMemory {
        async fn save_exchange(&self, _: &str, _: &str, _: &str, _: &str, _: &str) -> Result<()> { Ok(()) }
        async fn get_history(&self, _: &str, _: &str, _: i64) -> Result<Vec<HistoryMessage>> {
            *self.call_count.lock().unwrap() += 1;
            Ok(vec![HistoryMessage { role: "user".into(), content: "hi".into() }])
        }
        async fn clear_history(&self, _: &str, _: &str) -> Result<()> { Ok(()) }
        async fn get_message_count(&self, _: &str, _: &str) -> Result<i64> { Ok(0) }
        async fn save_fact(&self, _: &str, _: &str, _: &str) -> Result<()> { Ok(()) }
        async fn get_facts(&self, _: &str) -> Result<HashMap<String, String>> { Ok(HashMap::new()) }
        async fn delete_fact(&self, _: &str, _: &str) -> Result<bool> { Ok(true) }
        async fn get_summary(&self, _: &str, _: &str) -> Result<Option<String>> { Ok(None) }
        async fn save_summary_and_prune(&self, _: &str, _: &str, _: &str, _: i64) -> Result<()> { Ok(()) }
        async fn needs_summarization(&self, _: &str, _: &str, _: i64) -> Result<bool> { Ok(false) }
    }

    #[tokio::test]
    async fn test_cache_prevents_repeated_db_calls() {
        let inner = Arc::new(CountingMemory::new());
        let cached = CachedMemory::new(inner.clone(), 100, 300);

        // First call hits DB
        let _ = cached.get_history("ns", "u1", 20).await;
        assert_eq!(inner.calls(), 1);

        // Second call hits cache
        let _ = cached.get_history("ns", "u1", 20).await;
        assert_eq!(inner.calls(), 1); // still 1

        // After write, cache invalidated
        cached.save_exchange("ns", "u1", "tg", "msg", "reply").await.unwrap();
        let _ = cached.get_history("ns", "u1", 20).await;
        assert_eq!(inner.calls(), 2); // cache miss
    }
}
```

**Step 3: Add module**

```rust
// amanclaw-memory/src/lib.rs — add
pub mod cached;
```

**Step 4: Wrap memory in Engine::new()**

```rust
// amanclaw-core/src/lib.rs — in Engine::start(), after creating SqliteMemory:
let memory = SqliteMemory::new(&db_path).await?;
let memory_arc: Arc<dyn MemoryBackend> = Arc::new(
    amanclaw_memory::cached::CachedMemory::new(
        Arc::new(memory),
        1000, // max entries
        300,  // TTL seconds (5 min)
    )
);
```

**Step 5: Run tests**

Run: `cd rust && cargo test -p amanclaw-memory -p amanclaw-core`
Expected: All tests pass.

**Step 6: Commit**

```
feat(memory): add CachedMemory LRU wrapper with moka
```

---

## Phase 5 — Observability + WASM Hardening

### Task 13: Add Prometheus Metrics

**Files:**
- Modify: `rust/crates/amanclaw-core/Cargo.toml`
- Modify: `rust/crates/amanclaw-api/Cargo.toml`
- Create: `rust/crates/amanclaw-core/src/middleware/metrics.rs`
- Modify: `rust/crates/amanclaw-api/src/lib.rs` (add /metrics endpoint)

**Step 1: Add dependencies**

```toml
# amanclaw-core/Cargo.toml
metrics = "0.24"

# amanclaw-api/Cargo.toml
metrics = "0.24"
metrics-exporter-prometheus = "0.16"
```

**Step 2: Create MetricsMiddleware**

```rust
// rust/crates/amanclaw-core/src/middleware/metrics.rs

use super::{PipelineContext, PipelineMiddleware, MiddlewareChain};
use amanclaw_traits::message::OutgoingMessage;
use anyhow::Result;
use metrics::{counter, histogram};

pub struct MetricsMiddleware;

#[async_trait::async_trait]
impl PipelineMiddleware for MetricsMiddleware {
    async fn process(&self, ctx: PipelineContext, next: &MiddlewareChain) -> Result<Option<OutgoingMessage>> {
        let platform = ctx.msg.platform.clone();
        let agent = ctx.profile.id.clone();
        let start = std::time::Instant::now();

        counter!("messages_processed_total", "platform" => platform.clone(), "agent" => agent.clone()).increment(1);

        let result = next.execute(ctx).await;

        let duration = start.elapsed().as_secs_f64();
        histogram!("pipeline_duration_seconds", "agent" => agent).record(duration);

        result
    }
}
```

**Step 3: Add /metrics endpoint**

```rust
// amanclaw-api/src/lib.rs — add route (no auth)
use metrics_exporter_prometheus::PrometheusHandle;

// In api_router():
let metrics_routes = Router::new()
    .route("/metrics", get(metrics_handler))
    .with_state(state.clone());

// Add to Router::new().merge(...)
.merge(metrics_routes)

async fn metrics_handler(State(state): State<ApiState>) -> String {
    state.metrics_handle.render()
}
```

**Step 4: Initialize metrics exporter at startup**

```rust
// In Engine::start() or CLI main:
let builder = metrics_exporter_prometheus::PrometheusBuilder::new();
let handle = builder.install_recorder().expect("failed to install metrics recorder");
// Pass handle to ApiState
```

**Step 5: Run tests**

Run: `cd rust && cargo test`
Expected: All tests pass.

**Step 6: Commit**

```
feat(api): add Prometheus metrics endpoint and pipeline metrics middleware
```

---

### Task 14: WASM Resource Limits

**Files:**
- Modify: `rust/crates/amanclaw-wasm-runtime/src/runtime.rs:68-126,207-250`
- Modify: `rust/crates/amanclaw-traits/src/config.rs` (add wasm limit fields to PluginConfig)

**Step 1: Add config fields**

```rust
// amanclaw-traits/src/config.rs — add to PluginConfig
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfig {
    #[serde(default = "default_plugin_dir")]
    pub dir: String,
    #[serde(default)]
    pub hot_reload: bool,
    #[serde(default = "default_wasm_memory_limit_mb")]
    pub wasm_memory_limit_mb: u64,
    #[serde(default = "default_wasm_fuel_limit")]
    pub wasm_fuel_limit: u64,
}

fn default_wasm_memory_limit_mb() -> u64 { 64 }
fn default_wasm_fuel_limit() -> u64 { 1_000_000 }
```

**Step 2: Add ResourceLimiter to WASM store**

```rust
// amanclaw-wasm-runtime/src/runtime.rs

use wasmtime::{ResourceLimiter, Store, StoreLimits, StoreLimitsBuilder};

// In load_wasm_skill and execute_wasm, when creating Store:
let limits = StoreLimitsBuilder::new()
    .memory_size(memory_limit_mb * 1024 * 1024)
    .table_elements(10_000)
    .build();

let mut store = Store::new(&engine, limits);
store.limiter(|s| s);

// If using fuel:
store.set_fuel(fuel_limit)?;
```

Note: Wasmtime 29's `StoreLimits` implements `ResourceLimiter`. The `Store` data type becomes `StoreLimits` instead of `()`.

**Step 3: Update SandboxConfig to carry limits**

```rust
// Pass memory_limit_mb and fuel_limit through SandboxConfig or as parameters to load_wasm_skill
pub struct SandboxConfig {
    pub memory_limit_mb: u64,
    pub fuel_limit: u64,
    // ... existing fields
}
```

**Step 4: Run tests**

Run: `cd rust && cargo test -p amanclaw-wasm-runtime`
Expected: Tests pass. WASM plugins now have memory and CPU limits.

**Step 5: Commit**

```
feat(wasm): add memory and fuel limits to WASM plugin sandbox
```

---

### Task 15: Structured Pipeline Errors

**Files:**
- Create: `rust/crates/amanclaw-core/src/error.rs`
- Modify: `rust/crates/amanclaw-core/src/lib.rs:1`
- Modify: `rust/crates/amanclaw-core/src/middleware/*.rs` (use typed errors)

**Step 1: Create error.rs**

```rust
// rust/crates/amanclaw-core/src/error.rs

#[derive(thiserror::Error, Debug)]
pub enum PipelineError {
    #[error("user blocked")]
    UserBlocked,

    #[error("user pending approval")]
    UserPending,

    #[error("rate limited")]
    RateLimited,

    #[error("injection detected")]
    InjectionDetected,

    #[error("llm error: {0}")]
    LlmError(String),

    #[error("skill error: {skill}: {message}")]
    SkillError { skill: String, message: String },

    #[error("context budget exceeded: {0} tokens needed, {1} available")]
    ContextBudgetExceeded(usize, usize),

    #[error("engine shutting down")]
    EngineShutdown,

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}
```

**Step 2: Add module**

```rust
// amanclaw-core/src/lib.rs — add
pub mod error;
```

**Step 3: Update middleware to return PipelineError where appropriate**

Middleware can return `PipelineError` variants. The outer pipeline catches them and converts to user-friendly messages or Ok(None) as appropriate.

**Step 4: Run tests**

Run: `cd rust && cargo test -p amanclaw-core`
Expected: All tests pass.

**Step 5: Commit**

```
feat(core): add structured PipelineError types
```

---

## Summary

| Task | Phase | Description | Dependencies |
|------|-------|-------------|--------------|
| 1 | 1 | Auth Mutex → RwLock | None |
| 2 | 1 | Rate limiter DashMap | None |
| 3 | 1 | Configurable tool rounds | None |
| 4 | 1 | Token budget tracking | None |
| 5 | 1 | Bounded channels + backpressure | None |
| 6 | 2 | EngineCommand + EngineHandle types | Tasks 1-2 |
| 7 | 2 | Engine actor refactor | Task 6 |
| 8 | 2 | Desktop EngineHandle integration | Task 7 |
| 9 | 3 | Middleware trait + chain | Task 7 |
| 10 | 3 | Extract middleware components | Task 9 |
| 11 | 4 | Memory feature flags | None |
| 12 | 4 | CachedMemory wrapper | Task 11 |
| 13 | 5 | Prometheus metrics | Task 10 |
| 14 | 5 | WASM resource limits | None |
| 15 | 5 | Structured pipeline errors | Task 10 |

**Parallel opportunities:**
- Tasks 1-5 (Phase 1) are all independent — can be done in parallel
- Tasks 11, 14 are independent of Phase 2/3 — can be done early
- Tasks 6-8 must be sequential
- Tasks 9-10 must be sequential
