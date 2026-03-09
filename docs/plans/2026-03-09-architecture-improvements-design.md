# Architecture Improvements Design

**Date:** 2026-03-09
**Status:** Approved

## Overview

Incremental refactor of the AmanClaw engine for production safety, high performance, and scalability from desktop to cloud. No breaking changes to public trait interfaces (Skill, Channel, MemoryBackend, ContextEngine, VectorStore). All changes are internal implementations.

## Approach

Phased incremental refactor. Each phase is independently deployable. No big-bang rewrite.

## Phase 1 — Critical Fixes

Small, isolated changes that make the system production-safe.

### 1.1 Bounded Channels + Backpressure

**Problem:** Adapters have no backpressure handling if the engine's mpsc buffer fills.

**Change:** Use `try_send` in each adapter. When the engine buffer is full, reply to the user with "I'm busy, try again shortly" instead of blocking or dropping messages.

### 1.2 Token Budget Tracking in Context Engine

**Problem:** `context_engine.rs` appends system prompt + summary + facts + RAG + history + user message without counting tokens. Can exceed LLM context window.

**Change:** Add a `TokenBudget` struct that tracks estimated token usage. Priority order: system prompt > user message > recent history > facts > RAG > older history. Truncate from lowest priority when budget exceeded. Use simple word-count estimation (1 token ≈ 0.75 words) — no external tokenizer dependency.

### 1.3 Auth Mutex → RwLock

**Problem:** `Arc<Mutex<Auth>>` in pipeline — every message locks exclusively even for `get_user_state` (read-only).

**Change:** `Arc<RwLock<Auth>>` — reads use `read()`, writes (register, approve, block) use `write()`.

### 1.4 Rate Limiter Mutex → Sharded

**Problem:** `Mutex<RateLimiter>` — global lock for every message's rate check.

**Change:** Use `dashmap::DashMap<String, AtomicU32>` for per-user counters. Lock-free rate limiting.

### 1.5 Configurable Tool Rounds

**Problem:** Hardcoded `MAX_TOOL_ROUNDS: usize = 5`.

**Change:** Move to `AgentProfile.context.max_tool_rounds` (default 5).

## Phase 2 — Engine Actor Model

### 2.1 Engine as Actor

**Problem:** Engine is a struct that owns everything. Desktop wraps it in `Arc<Mutex<Option<Engine>>>`. Every IPC call locks the entire engine.

**Change:** Split into `EngineHandle` (cheap, cloneable) and `EngineActor` (internal):

```
EngineHandle
  - cmd_tx: mpsc::Sender<EngineCommand>
  - status: watch::Receiver<EngineStatus>
  - Methods: start(), stop(), status(), send_message()

EngineActor
  - Owns pipeline, registry, channels, scheduler
  - Receives commands via mpsc::Receiver<EngineCommand>
  - Broadcasts status via watch::Sender<EngineStatus>
  - Runs in its own tokio::spawn
```

Commands:

```rust
enum EngineCommand {
    ProcessMessage(IncomingMessage),
    SchedulerEvent(SchedulerEvent),
    GetStatus(oneshot::Sender<EngineStatus>),
    GetSkills(oneshot::Sender<Vec<SkillMetadata>>),
    Shutdown(oneshot::Sender<()>),
}
```

### 2.2 Desktop Integration

Replace `Arc<Mutex<Option<Engine>>>` with `EngineHandle`. All IPC commands call `handle.method()` — no locks.

### 2.3 Concurrent Message Processing

Actor spawns each message as a separate task. Use `tokio::sync::Semaphore` to limit concurrency (configurable, default 32).

```rust
EngineCommand::ProcessMessage(msg) => {
    let permit = semaphore.clone().acquire_owned().await;
    let pipeline = self.pipeline.clone();
    let registry = self.registry.clone();
    let router = self.agent_router.clone();
    let channels = self.channels.clone();
    tokio::spawn(async move {
        let _permit = permit;
        let profile = router.resolve(&msg);
        if let Ok(Some(response)) = pipeline.process(msg, &registry, &profile).await {
            send_to_channel(&channels, &platform, response).await;
        }
    });
}
```

## Phase 3 — Pipeline Middleware

### 3.1 Middleware Chain

**Problem:** `process_full()` is a 200-line monolithic function.

**Change:** Lightweight middleware pattern:

```rust
#[async_trait]
trait PipelineMiddleware: Send + Sync {
    async fn process(
        &self,
        ctx: PipelineContext,
        next: &dyn PipelineNext,
    ) -> Result<Option<OutgoingMessage>>;
}

struct PipelineContext {
    msg: IncomingMessage,
    profile: AgentProfile,
    is_internal: bool,
    extensions: Extensions,
}
```

Middleware stack (ordered):
1. `AuthMiddleware` — check user state, register new users
2. `CommandMiddleware` — handle /clear, /approve, etc. (short-circuits)
3. `RateLimitMiddleware` — per-user rate check
4. `SanitizeMiddleware` — injection detection
5. `ContextMiddleware` — build LLM context (system prompt + history + RAG)
6. `ToolCallingMiddleware` — LLM loop with tool execution
7. `PersistMiddleware` — save exchange + auto-summarize

### 3.2 Scope

Internal to `pipeline.rs` only. Nothing outside the pipeline changes. PluginRegistry, AgentRouter, StandardContextEngine, all adapters — unchanged.

## Phase 4 — Database Abstraction + Caching

### 4.1 PostgresMemory

**Problem:** `MemoryBackend` trait exists but only SQLite implements it. Hard ceiling for multi-instance.

**Change:** Add `PostgresMemory` behind feature flag:

```toml
[features]
default = ["sqlite"]
sqlite = ["sqlx/sqlite"]
postgres = ["sqlx/postgres"]
```

Config-driven selection:

```yaml
memory:
  backend: sqlite          # or "postgres"
  sqlite_path: memory.db
  postgres_url: ""
```

Desktop/Pi → SQLite. Cloud → Postgres.

### 4.2 QdrantVectorStore

Add `QdrantVectorStore` behind feature flag. Same pattern as memory.

### 4.3 In-Memory Cache Layer

**Problem:** Every message hits SQLite for history, facts, and summary.

**Change:** `CachedMemory` wrapper using `moka` (async LRU cache):

```rust
struct CachedMemory {
    inner: Arc<dyn MemoryBackend>,
    history_cache: moka::future::Cache<(String, String), Vec<HistoryMessage>>,
    facts_cache: moka::future::Cache<String, HashMap<String, String>>,
    summary_cache: moka::future::Cache<(String, String), Option<String>>,
}
```

- Wraps any MemoryBackend (SQLite or Postgres)
- Cache invalidated on writes
- TTL: 5 minutes (configurable)
- Max entries: 1000 (configurable)

## Phase 5 — Observability + WASM Hardening

### 5.1 Prometheus Metrics

Add `metrics` + `metrics-exporter-prometheus`:

```
messages_processed_total (counter, by platform)
tool_calls_total (counter, by skill)
llm_calls_total (counter, by agent)
pipeline_duration_seconds (histogram, by agent)
llm_latency_seconds (histogram)
tool_execution_seconds (histogram, by skill)
active_sessions (gauge)
channel_buffer_usage (gauge, by platform)
rate_limit_hits_total (counter)
auth_rejections_total (counter)
```

Endpoint: `GET /metrics` on the API server.

Implementation: `MetricsMiddleware` in pipeline chain.

### 5.2 WASM Resource Limits

**Problem:** No memory limits on WASM plugins.

**Change:**

```rust
store.limiter(|_| WasmLimiter {
    max_memory: 64 * 1024 * 1024,  // 64MB
    max_table_elements: 10_000,
});
store.set_fuel(1_000_000)?;
```

Config: `plugins.wasm_memory_limit_mb`, `plugins.wasm_fuel_limit`.

### 5.3 Structured Error Types

**Problem:** `anyhow::Result` everywhere. Hard to match specific errors.

**Change:** `thiserror` enums for pipeline:

```rust
#[derive(thiserror::Error, Debug)]
enum PipelineError {
    #[error("user blocked")]
    UserBlocked,
    #[error("rate limited")]
    RateLimited,
    #[error("llm error: {0}")]
    LlmError(String),
    #[error("skill error: {skill}: {message}")]
    SkillError { skill: String, message: String },
    #[error("context budget exceeded")]
    ContextBudgetExceeded,
}
```

Keep `anyhow` at boundaries (Engine::new, adapters). Typed errors inside pipeline.

## Scalability Path

```
Phase 1-3 (Single Node):
  SQLite + actor engine + middleware pipeline
  Handles: ~1000 concurrent users

Phase 4 (Multi-Instance):
  Postgres + cache layer + Qdrant
  Handles: ~10,000 concurrent users

Future (Cloud Native):
  + Message queue (NATS/Redis Streams) between adapters and engine
  + Kubernetes auto-scaling
  + Multi-region
  Handles: ~100,000+ concurrent users
```

## Summary

| Phase | Focus | Risk | Scope |
|-------|-------|------|-------|
| 1 | Critical fixes | Low | pipeline.rs, context_engine.rs, adapters |
| 2 | Engine actor | Medium | lib.rs, desktop IPC |
| 3 | Pipeline middleware | Medium | pipeline.rs only |
| 4 | DB + cache | Low | amanclaw-memory, new CachedMemory |
| 5 | Observability + hardening | Low | wasm-runtime, api, new metrics |

No phase breaks public trait interfaces. Skills, adapters, and plugins never need updating.
