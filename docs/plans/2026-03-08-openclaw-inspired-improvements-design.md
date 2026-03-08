# OpenClaw-Inspired Improvements Design

**Date:** 2026-03-08
**Status:** Approved
**Motivation:** Deep comparison with OpenClaw (247K+ stars, TypeScript) revealed three key areas where AmanClaw can close the feature gap while preserving its Rust/security advantages.

---

## Overview

Three interconnected improvements, designed as a phased rollout:

1. **Multi-Agent Routing** — Route messages to isolated agent profiles with their own persona, skills, and memory
2. **Pluggable Memory Backends** — Trait-based storage with vector search for semantic retrieval (Quran, Hadith)
3. **Context Engine Plugin System** — Swappable context strategies with compaction lifecycle hooks

Each section builds on the previous: Router produces AgentProfile → Memory provides trait-based storage → Context Engine orchestrates both into a clean pipeline.

---

## Section 1: Multi-Agent Routing

### Problem

AmanClaw has one global pipeline — every message goes through the same LLM with the same system prompt and skill set. OpenClaw routes different channels/topics to isolated agents.

### Design

**New struct: `AgentProfile`** (in `amanclaw-traits`)

```rust
pub struct AgentProfile {
    pub id: String,                        // "ustazbot", "halalbot", "default"
    pub name: String,                      // Display name
    pub system_prompt: String,             // Custom persona prompt
    pub allowed_skills: Vec<String>,       // Subset of registry (empty = all)
    pub llm_override: Option<LlmConfig>,   // Optional different model
    pub memory_namespace: String,          // Isolates conversation history
    pub context: ContextConfig,            // Per-agent context settings
}

pub struct ContextConfig {
    pub history_limit: i64,            // Default: 20
    pub summarize_threshold: i64,      // Default: 40
    pub summarize_keep_recent: i64,    // Default: 10
    pub rag_enabled: bool,             // Default: false
    pub rag_collections: Vec<String>,  // Knowledge base collections
    pub rag_top_k: usize,             // Default: 3
}
```

**Routing rules** in `config.yaml`:

```yaml
agents:
  default:
    name: "AmanClaw"
    system_prompt: "You are AmanClaw, a helpful assistant..."
    allowed_skills: []  # all
    context:
      history_limit: 20
      summarize_threshold: 40
      rag_enabled: false

  ustazbot:
    name: "UstazBot"
    system_prompt: "You are UstazBot, an Islamic knowledge expert..."
    allowed_skills: [solat, qiblat, hijri, doa, quran, hadith]
    context:
      history_limit: 30
      summarize_threshold: 60
      rag_enabled: true
      rag_collections: [quran_ayat, hadith_texts, tafsir]
      rag_top_k: 5
    llm_override:
      model: "claude-3-haiku"

routing:
  rules:
    - match: { platform: telegram, topic_id: "123" }
      agent: ustazbot
    - match: { platform: discord, channel_id: "456" }
      agent: ustazbot
    - match: { platform: telegram, group_id: "789" }
      agent: halalbot
  default_agent: default
```

### How It Works

1. `IncomingMessage` gets new optional fields: `topic_id`, `channel_context`
2. Router inspects message metadata → matches against rules → selects `AgentProfile`
3. Pipeline receives the profile alongside the message
4. Pipeline uses profile's system prompt, filters registry to allowed_skills, uses profile's memory namespace
5. If no rule matches → falls back to `default` agent

### Pipeline Signature Change

```rust
// Before:
pub async fn process(&self, msg: IncomingMessage, registry: &PluginRegistry) -> ...

// After:
pub async fn process(&self, msg: IncomingMessage, registry: &PluginRegistry, profile: &AgentProfile) -> ...
```

The registry stays global (one registry, all skills loaded once). The pipeline **filters** `get_tool_definitions()` based on `profile.allowed_skills` before calling the LLM. Memory operations are namespaced: `{profile.memory_namespace}:{user_id}`.

### New Message Fields

```rust
pub struct IncomingMessage {
    // ... existing fields ...
    pub topic_id: Option<String>,       // Telegram topics, Discord threads
    pub channel_context: Option<String>, // Additional routing context
}
```

### Enables Phase 4

Specialized bots (UstazBot, HalalBot, SolatBot) run as agent profiles within a single AmanClaw instance — no separate binaries needed.

---

## Section 2: Pluggable Memory Backends

### Problem

AmanClaw is hardcoded to `SqliteMemory`. OpenClaw supports vector stores and knowledge graphs. For Islamic knowledge (Quran, Hadith, tafsir), semantic search is far more useful than exact-match SQL queries.

### Memory Backend Trait

New trait in `amanclaw-traits`:

```rust
#[async_trait]
pub trait MemoryBackend: Send + Sync {
    // Conversation history
    async fn save_exchange(&self, ns: &str, user_id: &str, platform: &str, user_msg: &str, assistant_msg: &str) -> Result<()>;
    async fn get_history(&self, ns: &str, user_id: &str, limit: i64) -> Result<Vec<HistoryMessage>>;
    async fn clear_history(&self, ns: &str, user_id: &str) -> Result<()>;
    async fn get_message_count(&self, ns: &str, user_id: &str) -> Result<i64>;

    // Facts
    async fn save_fact(&self, user_id: &str, key: &str, value: &str) -> Result<()>;
    async fn get_facts(&self, user_id: &str) -> Result<HashMap<String, String>>;
    async fn delete_fact(&self, user_id: &str, key: &str) -> Result<bool>;

    // Summarization
    async fn get_summary(&self, ns: &str, user_id: &str) -> Result<Option<String>>;
    async fn save_summary_and_prune(&self, ns: &str, user_id: &str, summary: &str, keep_recent: i64) -> Result<()>;
    async fn needs_summarization(&self, ns: &str, user_id: &str, threshold: i64) -> Result<bool>;
}
```

The `ns` (namespace) parameter comes from `AgentProfile.memory_namespace`, tying into Section 1.

### Vector Store Trait

```rust
#[async_trait]
pub trait VectorStore: Send + Sync {
    async fn upsert(&self, collection: &str, docs: &[Document]) -> Result<()>;
    async fn search(&self, collection: &str, query: &str, limit: usize) -> Result<Vec<SearchResult>>;
    async fn delete(&self, collection: &str, ids: &[String]) -> Result<()>;
}

pub struct Document {
    pub id: String,
    pub content: String,
    pub metadata: HashMap<String, String>,
}

pub struct SearchResult {
    pub id: String,
    pub content: String,
    pub score: f64,
    pub metadata: HashMap<String, String>,
}
```

### Implementations

| Backend | Crate | Purpose |
|---|---|---|
| `SqliteMemory` | `amanclaw-memory` | Default, refactored to implement `MemoryBackend` |
| `SqliteVectorStore` | `amanclaw-memory` | Uses `sqlite-vec` extension — zero external deps |
| `QdrantVectorStore` | new `amanclaw-vector-qdrant` | Optional, for production-scale vector search |

### Why sqlite-vec as Default

- No external service needed (same SQLite file)
- Works on Raspberry Pi
- Good enough for 100K-1M documents (Quran + Hadith corpus fits easily)
- Qdrant optional for scale

### Embedding Generation

New `EmbeddingClient` in `amanclaw-llm`:

```rust
pub struct EmbeddingClient {
    client: Client,
    base_url: String,
    model: String,
}

impl EmbeddingClient {
    pub async fn embed(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> { ... }
}
```

Calls any OpenAI-compatible `/v1/embeddings` endpoint.

### Config

```yaml
memory:
  backend: sqlite
  db_path: "./memory.db"

vector:
  backend: sqlite-vec     # or "qdrant"
  # qdrant_url: "http://localhost:6333"

embeddings:
  base_url: "http://localhost:8001/v1"
  model: "BAAI/bge-m3"   # Multilingual: BM/EN/Arabic

knowledge_bases:
  quran:
    collection: "quran_ayat"
    source: "./data/quran.json"
  hadith:
    collection: "hadith_texts"
    source: "./data/hadith.json"
```

---

## Section 3: Context Engine Plugin System

### Problem

AmanClaw's context building is hardcoded in `pipeline.rs`: fetch 20 messages, prepend summary, append facts. OpenClaw's latest release added a "context-engine plugin slot" with compaction lifecycle hooks — making context strategy swappable.

### Context Engine Trait

```rust
#[async_trait]
pub trait ContextEngine: Send + Sync {
    /// Build the full message context for an LLM call.
    async fn build_context(&self, request: ContextRequest) -> Result<ContextResult>;

    /// Called after a successful exchange — opportunity to update state.
    async fn on_exchange_complete(&self, exchange: ExchangeEvent) -> Result<()>;

    /// Called when compaction/summarization is triggered.
    async fn on_compaction(&self, event: CompactionEvent) -> Result<CompactionResult>;
}

pub struct ContextRequest {
    pub user_id: String,
    pub platform: String,
    pub namespace: String,
    pub user_message: String,
    pub image_data: Option<Vec<u8>>,
    pub agent_profile: AgentProfile,
}

pub struct ContextResult {
    pub messages: Vec<serde_json::Value>,
    pub tools: Vec<ToolDefinition>,
}

pub struct ExchangeEvent {
    pub user_id: String,
    pub namespace: String,
    pub user_message: String,
    pub assistant_response: String,
}

pub struct CompactionEvent {
    pub user_id: String,
    pub namespace: String,
    pub message_count: i64,
    pub threshold: i64,
}

pub struct CompactionResult {
    pub should_compact: bool,
    pub summary: Option<String>,
    pub keep_recent: i64,
}
```

### Default Implementation: StandardContextEngine

Lives in `amanclaw-core`. Replicates current behavior exactly, plus RAG:

```rust
pub struct StandardContextEngine {
    memory: Arc<dyn MemoryBackend>,
    vector_store: Option<Arc<dyn VectorStore>>,
    embedding_client: Option<Arc<EmbeddingClient>>,
    llm: Arc<LlmClient>,
}
```

**`build_context` steps:**
1. Fetch history from `MemoryBackend` (limit from `agent_profile.context.history_limit`)
2. Prepend summary if available
3. Append user facts
4. RAG retrieval from `VectorStore` if `rag_enabled` in profile
5. Inject agent-specific system prompt from `AgentProfile`
6. Build multimodal message if image present
7. Filter tools by `agent_profile.allowed_skills`
8. Return `ContextResult`

**`on_exchange_complete` steps:**
1. Save exchange to `MemoryBackend` (namespaced)
2. Check if compaction needed
3. If yes, call `on_compaction`

**`on_compaction` steps:**
1. Fetch all history
2. Call LLM to summarize
3. Return `CompactionResult` with summary and prune instructions

### Pipeline Simplification

```rust
// Before: 150 lines of inline context building + summarization in process_full()
// After:

async fn process_full(context_engine: &dyn ContextEngine, ...) -> Result<Option<OutgoingMessage>> {
    // 1. Auth check (unchanged)
    // 2. Rate limit (unchanged)
    // 3. Sanitize (unchanged)

    // 4. Build context — ONE call replaces 40 lines
    let ctx = context_engine.build_context(ContextRequest { ... }).await?;

    // 5. Tool calling loop (unchanged, uses ctx.tools)
    let response = Self::tool_calling_loop(llm, registry, &mut ctx.messages, &ctx.tools, ...).await?;

    // 6. Post-exchange hook — ONE call replaces 25 lines
    context_engine.on_exchange_complete(ExchangeEvent { ... }).await?;

    Ok(Some(OutgoingMessage { ... }))
}
```

### Future Context Engines

| Engine | Use case |
|---|---|
| `StandardContextEngine` | Default — history + facts + RAG + summarization |
| `SlidingWindowEngine` | Simple token-count window, no summarization |
| `GraphContextEngine` | Knowledge graph traversal (Phase 3+) |
| `HybridEngine` | Combines multiple strategies with priority |

---

## System Flow (All Three Sections Combined)

```
IncomingMessage
    ↓
[Router] ─── matches rules ──→ AgentProfile (Section 1)
    ↓
[Pipeline]
    ├─ Auth / Rate Limit / Sanitize (unchanged)
    ├─ ContextEngine.build_context(profile) (Section 3)
    │     ├─ MemoryBackend.get_history(namespace) (Section 2)
    │     ├─ VectorStore.search(rag_collections) (Section 2)
    │     └─ Returns filtered tools + full message context
    ├─ Tool Calling Loop (unchanged)
    └─ ContextEngine.on_exchange_complete() (Section 3)
           ├─ MemoryBackend.save_exchange(namespace)
           └─ Compaction if threshold exceeded
```

## Implementation Phases

### Phase A: Memory Trait Extraction (Foundation)
- Extract `MemoryBackend` trait from `SqliteMemory`
- Add `ns` (namespace) parameter to all methods
- Refactor `SqliteMemory` to implement new trait
- Update pipeline to use `Arc<dyn MemoryBackend>`
- Zero behavior change — pure refactor

### Phase B: Multi-Agent Routing
- Add `AgentProfile`, `ContextConfig` to traits
- Add `topic_id`, `channel_context` to `IncomingMessage`
- Implement Router with config-driven rules
- Update pipeline signature to accept `AgentProfile`
- Filter tools by `allowed_skills`
- Namespace memory by `memory_namespace`

### Phase C: Context Engine
- Add `ContextEngine` trait to traits
- Implement `StandardContextEngine` in core
- Refactor pipeline to delegate to context engine
- Pipeline shrinks from ~150 to ~50 lines

### Phase D: Vector Store + RAG
- Add `VectorStore` trait, `EmbeddingClient`
- Implement `SqliteVectorStore` with `sqlite-vec`
- Add RAG retrieval to `StandardContextEngine.build_context()`
- Add knowledge base loading at startup
- Optional: `QdrantVectorStore` crate

## Crate Changes Summary

| Crate | Changes |
|---|---|
| `amanclaw-traits` | Add `MemoryBackend`, `VectorStore`, `ContextEngine` traits, `AgentProfile`, `ContextConfig`, new message fields |
| `amanclaw-memory` | Refactor `SqliteMemory` → implements `MemoryBackend`, add `SqliteVectorStore` |
| `amanclaw-core` | Add Router, `StandardContextEngine`, update Pipeline, update Engine |
| `amanclaw-llm` | Add `EmbeddingClient` |
| `amanclaw-traits/config` | Add `agents`, `routing`, `memory`, `vector`, `embeddings`, `knowledge_bases` config sections |
| Channel plugins | Pass through `topic_id` / `channel_context` from platform messages |
| new `amanclaw-vector-qdrant` | Optional Qdrant backend (Phase D) |
