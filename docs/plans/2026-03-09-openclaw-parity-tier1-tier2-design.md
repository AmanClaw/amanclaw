# OpenClaw Parity — Tier 1 + Tier 2 Improvements Design

**Date:** 2026-03-09
**Status:** Approved
**Approach:** Bottom-Up Foundation (Approach A)
**Motivation:** Deep comparison with OpenClaw (247K+ stars) revealed 7 high-impact features AmanClaw lacks. This design implements Tier 1 (infrastructure) and Tier 2 (capabilities) while preserving AmanClaw's Rust performance, WASM sandboxing, and security advantages.

---

## Implementation Order

```
1. FTS5 Hybrid Search → 2. SOUL.md Files → 3. Cron/Scheduler → 4. Webhooks
→ 5. WebSocket Gateway → 6. Sub-Agent Spawning → 7. Skill Marketplace
```

Each step is independently testable and delivers standalone value.

| # | Feature | Effort | New Crate? |
|---|---------|--------|-----------|
| 1 | FTS5 Hybrid Search | Small | No |
| 2 | SOUL.md Agent Files | Small | No |
| 3 | Cron/Scheduled Tasks | Medium | No |
| 4 | Webhook Triggers | Medium | No |
| 5 | WebSocket Gateway | Large | Yes (`amanclaw-gateway`) |
| 6 | Sub-Agent Spawning | Medium | No |
| 7 | Skill Marketplace | Large | Yes (`amanclaw-registry`) |

---

## Section 1: FTS5 Hybrid Search

### Problem

Current `SqliteVectorStore::search()` uses `LIKE %query%` as text fallback — no ranking, no relevance scoring. OpenClaw uses BM25 + cosine similarity hybrid search.

### Design

#### FTS5 as external content table (no data duplication)

```sql
CREATE VIRTUAL TABLE IF NOT EXISTS vector_documents_fts
USING fts5(content, tokenize='unicode61 remove_diacritics 2', content='vector_documents', content_rowid='rowid');
```

- `content='vector_documents'` — FTS5 reads from existing table, zero storage overhead
- `tokenize='unicode61 remove_diacritics 2'` — handles Arabic diacritics (tashkeel) so "الرحمن" matches "الرَّحْمَنِ". Critical for Quran/Hadith search
- External content table means upsert code needs zero changes

#### Sync triggers (automatic FTS index maintenance)

```sql
CREATE TRIGGER IF NOT EXISTS vector_documents_ai AFTER INSERT ON vector_documents BEGIN
    INSERT INTO vector_documents_fts(rowid, content) VALUES (new.rowid, new.content);
END;
CREATE TRIGGER IF NOT EXISTS vector_documents_ad AFTER DELETE ON vector_documents BEGIN
    INSERT INTO vector_documents_fts(vector_documents_fts, rowid, content) VALUES('delete', old.rowid, old.content);
END;
CREATE TRIGGER IF NOT EXISTS vector_documents_au AFTER UPDATE ON vector_documents BEGIN
    INSERT INTO vector_documents_fts(vector_documents_fts, rowid, content) VALUES('delete', old.rowid, old.content);
    INSERT INTO vector_documents_fts(rowid, content) VALUES (new.rowid, new.content);
END;
```

#### Hybrid search with Reciprocal Rank Fusion (RRF)

When `search_by_embedding` is called, perform both cosine similarity AND FTS5 BM25 search, then combine with RRF:

```
rrf_score(doc) = 1/(k + vector_rank) + 1/(k + fts_rank)
```

Where `k=60` (standard from Cormack 2009 paper). RRF is rank-based, not score-based — avoids normalizing cosine similarity (0-1) against BM25 (unbounded).

```rust
fn hybrid_rrf(
    vector_ranked: &[(String, f64)],
    fts_ranked: &[(String, f64)],
    k: f64,
) -> Vec<(String, f64)> {
    let mut scores: HashMap<String, f64> = HashMap::new();
    for (rank, (id, _)) in vector_ranked.iter().enumerate() {
        *scores.entry(id.clone()).or_default() += 1.0 / (k + rank as f64 + 1.0);
    }
    for (rank, (id, _)) in fts_ranked.iter().enumerate() {
        *scores.entry(id.clone()).or_default() += 1.0 / (k + rank as f64 + 1.0);
    }
    let mut merged: Vec<_> = scores.into_iter().collect();
    merged.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    merged
}
```

#### FTS5-only text search (replaces LIKE fallback)

```sql
SELECT vd.id, vd.content, vd.metadata, bm25(vector_documents_fts) as rank
FROM vector_documents vd
JOIN vector_documents_fts fts ON vd.rowid = fts.rowid
WHERE vd.collection = ? AND vector_documents_fts MATCH ?
ORDER BY rank
LIMIT ?
```

### Changes

| File | Change |
|------|--------|
| `amanclaw-memory/src/schema.rs` | Add FTS5 virtual table + 3 sync triggers to `INIT_SQL` |
| `amanclaw-memory/src/vector.rs` | Replace `LIKE` with FTS5 BM25 in `search()`, add `hybrid_rrf()` to `search_by_embedding()` |
| No trait changes | Existing `VectorStore` signatures stay the same |
| No other crates affected | Triggers handle FTS sync automatically |

---

## Section 2: SOUL.md Agent Personality Files

### Problem

Agent personas are inline `system_prompt` strings in `config.yaml`. Long prompts are unwieldy, not version-controlled, and can't be shared or inherited.

### Design

#### Structured SOUL.md with frontmatter

```markdown
---
version: 1
extends: default.md
language: rojak
tags: [islamic, community, knowledge]
variables:
  mazhab: shafi'i
  region: malaysia
---

# UstazBot

You are UstazBot, an Islamic knowledge expert for Malaysian Muslim communities.

## Personality
- Warm, respectful, uses Islamic greetings
- Answers in Rojak (BM/EN mix) by default
- Cites Quran ayat and Hadith with references

## Constraints
- Never issue fatwa — recommend consulting qualified ustaz
- For fiqh differences, present all 4 mazhab with {{mazhab}} emphasis
- Decline non-Islamic topics, redirect to default agent
```

#### Inheritance system

Agents extend a base soul. Sections in child files replace same-named sections in parent. New sections are appended.

```
souls/
├── default.md          # Base personality
├── ustazbot.md         # extends: default.md
├── halalbot.md         # extends: default.md
└── solatbot.md         # extends: ustazbot.md
```

Resolution chain: `solatbot.md` → `ustazbot.md` → `default.md`. Max depth: 5 (cycle protection).

#### Variable interpolation

`{{mazhab}}` → replaced from frontmatter `variables:` block. Runtime variables `{{datetime}}`, `{{user_name}}`, `{{platform}}` injected by engine.

#### SoulLoader implementation

```rust
pub struct SoulLoader;

#[derive(Debug, Deserialize, Default)]
struct SoulFrontmatter {
    version: u32,
    extends: Option<String>,
    language: Option<String>,
    tags: Vec<String>,
    variables: HashMap<String, String>,
}

pub struct ResolvedSoul {
    pub prompt: String,
    pub variables: HashMap<String, String>,
    pub tags: Vec<String>,
}

impl SoulLoader {
    pub fn load(soul_dir: &Path, filename: &str) -> Result<ResolvedSoul> {
        // Walk inheritance chain (max depth 5)
        // Parse frontmatter (YAML between --- delimiters)
        // Merge sections: base first, child overrides
        // Interpolate {{variables}}
        // Return resolved prompt string
    }
}
```

#### Hot-reload on save

```rust
pub fn reload_agent_soul(&self, agent_id: &str) -> Result<()> {
    // Re-read soul file, re-resolve inheritance, update profile.system_prompt
}
```

#### Desktop App — Agents page

- Sidebar agent list with Markdown soul editor (CodeMirror)
- Frontmatter as structured form fields (extends, tags, variables)
- "Preview Resolved Prompt" button shows final prompt after inheritance + interpolation
- Duplicate button for creating variants

#### Config

```yaml
skills:
  soul_dir: "./souls"

agents:
  ustazbot:
    soul_file: "ustazbot.md"
    allowed_skills: [solat, qiblat, hijri, doa, quran]
    memory_namespace: ustaz
```

### Changes

| File | Change |
|------|--------|
| `amanclaw-traits/src/agent.rs` | Add `soul_file: Option<String>` to `AgentProfile` |
| `amanclaw-traits/src/config.rs` | Add `soul_dir: Option<String>` to `SkillsConfig` |
| `amanclaw-core/src/soul.rs` | **New** — `SoulLoader` with frontmatter parsing, inheritance, variable interpolation |
| `amanclaw-core/src/lib.rs` | Load souls during `Engine::new()`, add `reload_agent_soul()` |
| `amanclaw-core/src/router.rs` | Add `get_mut()` for hot-reload |
| `amanclaw-api/src/routes/agents.rs` | **New** — REST endpoints for soul CRUD |
| Desktop `src/lib/pages/Agents.svelte` | **New** — Agent management page with soul editor |
| `souls/default.md` | Base soul file |
| `souls/ustazbot.md` | UstazBot soul |

---

## Section 3: Cron/Scheduled Tasks

### Problem

AmanClaw is purely reactive. OpenClaw has first-class cron jobs for proactive actions. For Islamic communities: daily prayer reminders, Quran verse of the day, Jumuah alerts, Ramadan notifications.

### Design

#### Three job types

```rust
pub enum CronJobType {
    /// Static/templated message (no LLM, zero cost)
    DirectMessage { template: String },
    /// Execute a skill and send output
    SkillInvocation { skill: String, input: String },
    /// Inject prompt into pipeline, agent generates response
    AgentPrompt { prompt: String },
}
```

- `DirectMessage` — zero latency, no LLM cost (5x daily prayer reminders)
- `SkillInvocation` — moderate, one skill call, predictable output
- `AgentPrompt` — full LLM pipeline, flexible (daily content generation)

#### CronJob struct

```rust
pub struct CronJob {
    pub id: String,
    pub name: String,
    pub schedule: String,             // cron: "0 5 30 * * *"
    pub timezone: Option<String>,     // "Asia/Kuala_Lumpur"
    pub job_type: CronJobType,
    pub targets: Vec<CronTarget>,     // platform + chat_id + topic_id
    pub enabled: bool,
    pub agent: Option<String>,
    pub metadata: HashMap<String, String>,
}

pub struct CronTarget {
    pub platform: String,
    pub chat_id: String,
    pub topic_id: Option<String>,
}
```

#### Scheduler implementation

Uses `cron` crate for expression parsing + `tokio::time::sleep` for waiting. Each job runs in its own tokio task.

```rust
pub struct Scheduler {
    jobs: Arc<RwLock<HashMap<String, CronJob>>>,
    handles: HashMap<String, JoinHandle<()>>,
    tx: mpsc::Sender<SchedulerEvent>,
}

pub enum SchedulerEvent {
    SendMessage(OutgoingMessage),      // Direct send to channel
    InjectMessage(IncomingMessage),    // Process through pipeline
}
```

#### Pipeline integration

Cron messages set `is_cron: true` and skip auth/rate-limit/injection checks. They're trusted internal sources.

#### Engine integration

```rust
// In Engine::run() — multiplex chat messages and scheduler events:
loop {
    tokio::select! {
        Some(msg) = self.rx.recv() => { /* chat messages */ }
        Some(event) = sched_rx.recv() => { /* cron + webhook events */ }
        else => break,
    }
}
```

#### Runtime management

Add/remove/toggle jobs without restart via API or desktop.

#### Execution history

```sql
CREATE TABLE IF NOT EXISTS cron_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    job_id TEXT NOT NULL,
    status TEXT NOT NULL,
    output TEXT,
    duration_ms INTEGER,
    executed_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
```

#### Desktop App — Scheduler page

List of jobs with status (last run, next run), toggle switches, execution history table.

#### Config

```yaml
cron:
  timezone: "Asia/Kuala_Lumpur"
  jobs:
    subuh_reminder:
      name: "Subuh Prayer Reminder"
      schedule: "0 5 30 * * *"
      type: skill_invocation
      skill: solat
      input: '{"zone": "WLY01"}'
      targets:
        - platform: telegram
          chat_id: "-1001234567890"
      enabled: true

    quran_daily:
      name: "Quran Verse of the Day"
      schedule: "0 8 0 * * *"
      type: agent_prompt
      prompt: "Share today's Quran verse of the day from Juz Amma."
      targets:
        - platform: telegram
          chat_id: "-1001234567890"
      agent: ustazbot
      enabled: true

    jumuah_reminder:
      name: "Jumuah Reminder"
      schedule: "0 11 30 * * FRI"
      type: direct_message
      template: "🕌 Jumaat Mubarak! Jangan lupa solat Jumaat."
      targets:
        - platform: telegram
          chat_id: "-1001234567890"
      enabled: true
```

### Changes

| File | Change |
|------|--------|
| `amanclaw-traits/src/message.rs` | Add `is_cron: bool`, ensure `topic_id` on `OutgoingMessage` |
| `amanclaw-traits/src/config.rs` | Add `CronConfig`, `CronJobConfig` structs |
| `amanclaw-core/src/scheduler.rs` | **New** — `Scheduler`, `CronJob`, `CronJobType`, `SchedulerEvent` |
| `amanclaw-core/src/lib.rs` | Init scheduler, `tokio::select!` in `run()` |
| `amanclaw-core/src/pipeline.rs` | Skip auth/rate-limit for `is_cron` messages |
| `amanclaw-memory/src/schema.rs` | Add `cron_history` table |
| `amanclaw-api/src/routes/cron.rs` | **New** — REST endpoints for cron CRUD |
| Desktop `src/lib/pages/Scheduler.svelte` | **New** — Scheduler management page |
| `amanclaw-core/Cargo.toml` | Add `cron = "0.13"`, `chrono-tz = "0.10"` |

### Dependencies

```toml
cron = "0.13"
chrono-tz = "0.10"
```

---

## Section 4: Webhook Triggers

### Problem

External systems (JAKIM API, mosque donation platforms, GitHub, payment gateways) have no way to trigger agent actions. OpenClaw supports webhooks as first-class event sources.

### Design

#### Webhook endpoint definition

```rust
pub struct WebhookEndpoint {
    pub id: String,
    pub name: String,
    pub path: String,                 // "/webhooks/jakim-update"
    pub auth: WebhookAuth,
    pub transform: WebhookTransform,
    pub targets: Vec<CronTarget>,     // Reuse from cron
    pub agent: Option<String>,
    pub enabled: bool,
    pub rate_limit: Option<u32>,
    pub metadata: HashMap<String, String>,
}
```

#### Four auth methods

```rust
pub enum WebhookAuth {
    None,
    HmacSha256 { secret: String, header: String },   // GitHub, Stripe
    BearerToken { token: String },
    HeaderMatch { header: String, value: String },
}
```

#### Five transform types

```rust
pub enum WebhookTransform {
    RawJson,                                          // Forward raw payload
    JsonPath { message_path: String, title_path: Option<String> },
    Template { template: String },                    // Handlebars
    SkillInvocation { skill: String, input_template: String },
    AgentPrompt { prompt_template: String },           // Full LLM pipeline
}
```

#### Unified event channel with cron

Both cron and webhooks produce `SchedulerEvent` — same channel, same handling in `Engine::run()`.

#### Route separation

Webhook receivers at `/hooks/{webhook_id}` — NO auth middleware (webhooks validate themselves). Management endpoints at `/api/webhooks/*` — require admin auth.

#### Execution history

```sql
CREATE TABLE IF NOT EXISTS webhook_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    webhook_id TEXT NOT NULL,
    status TEXT NOT NULL,
    source_ip TEXT,
    payload_preview TEXT,
    error TEXT,
    duration_ms INTEGER,
    received_at DATETIME DEFAULT CURRENT_TIMESTAMP
);
```

#### Config

```yaml
webhooks:
  base_path: "/hooks"
  default_secret: "${WEBHOOK_SECRET}"
  endpoints:
    jakim_update:
      name: "JAKIM Prayer Time Update"
      path: "/jakim"
      auth:
        type: hmac_sha256
        secret: "${JAKIM_WEBHOOK_SECRET}"
        header: "X-Signature"
      transform:
        type: template
        template: "🕌 JAKIM Update: Prayer times for zone {{zone}} updated."
      targets:
        - platform: telegram
          chat_id: "-1001234567890"
      enabled: true

    donation_alert:
      name: "Mosque Donation Alert"
      path: "/donations"
      auth:
        type: bearer_token
        token: "${DONATION_API_TOKEN}"
      transform:
        type: agent_prompt
        prompt_template: "A donation of RM{{amount}} received from {{donor_name}}. Generate thank you and du'a."
      targets:
        - platform: telegram
          chat_id: "-1001234567890"
      agent: ustazbot
      rate_limit: 30
      enabled: true
```

### Changes

| File | Change |
|------|--------|
| `amanclaw-traits/src/message.rs` | Add `is_webhook: bool` to `IncomingMessage` |
| `amanclaw-traits/src/config.rs` | Add `WebhookConfig`, `WebhookEndpointConfig`, `WebhookAuth`, `WebhookTransform` |
| `amanclaw-core/src/webhooks.rs` | **New** — `WebhookRouter`, auth validation, payload transform, HMAC verification |
| `amanclaw-core/src/pipeline.rs` | Skip auth/rate-limit for `is_webhook` messages |
| `amanclaw-api/src/routes/webhooks.rs` | **New** — Receiver endpoint + management CRUD |
| `amanclaw-api/src/lib.rs` | Mount webhook routes |
| `amanclaw-memory/src/schema.rs` | Add `webhook_history` table |
| Desktop `src/lib/pages/Webhooks.svelte` | **New** — Webhook management page |

### Dependencies

```toml
handlebars = "6"
hmac = "0.12"
sha2 = "0.10"
hex = "0.4"
```

---

## Section 5: WebSocket Gateway

### Problem

AmanClaw's engine is closed — external clients (desktop app, web dashboard, monitoring tools) have no real-time connection. The REST API is request-response only. OpenClaw's Gateway is the nervous system that ties everything together.

### Design

#### Gateway as WebSocket layer on existing Axum server

No separate port or process. REST at `/api/*`, webhooks at `/hooks/*`, WebSocket at `/ws`. All on one Axum server.

#### Protocol: JSON-RPC 2.0 over WebSocket

Same protocol as MCP server — consistent across the project.

```rust
// Client → Gateway (request)
pub struct GatewayRequest {
    pub jsonrpc: String,
    pub id: Option<serde_json::Value>,
    pub method: String,
    pub params: Option<serde_json::Value>,
}

// Gateway → Client (push event)
pub struct GatewayEvent {
    pub jsonrpc: String,
    pub method: String,               // "event.message.received"
    pub params: serde_json::Value,
}
```

#### Methods

| Category | Methods |
|----------|---------|
| Session | `gateway.auth`, `gateway.ping`, `gateway.info` |
| Subscriptions | `subscribe`, `unsubscribe` |
| Engine | `engine.status`, `engine.start`, `engine.stop`, `engine.restart` |
| Messages | `message.send`, `message.history` |
| Agents | `agent.list`, `agent.get`, `agent.reload`, `agent.spawn` |
| Scheduler | `cron.list`, `cron.toggle`, `cron.run`, `webhook.list` |
| Skills | `skill.list`, `skill.invoke` |

#### Topic-based pub/sub

Clients subscribe to event topics using glob patterns:

```json
{"method": "subscribe", "params": {"topics": ["message.*", "engine.*", "cron.fired"]}}
```

Event topics: `engine.started`, `engine.stopped`, `message.received`, `message.sent`, `agent.routed`, `agent.tool_call`, `cron.fired`, `webhook.received`, `security.rate_limited`, `security.injection`, etc.

#### Session Manager

```rust
pub struct SessionManager {
    sessions: Arc<RwLock<HashMap<String, Session>>>,
}

pub struct Session {
    pub id: String,
    pub client_type: ClientType,       // Desktop, WebDashboard, RemoteAgent, SubAgent
    pub authenticated: bool,
    pub subscriptions: HashSet<String>,
    pub connected_at: DateTime<Utc>,
    pub last_ping: DateTime<Utc>,
    pub tx: mpsc::Sender<String>,
}
```

#### EventEmitter trait (decouples engine from gateway)

```rust
pub trait EventEmitter: Send + Sync {
    fn emit(&self, topic: &str, data: serde_json::Value);
}

pub struct NoopEmitter;               // CLI mode — zero overhead
pub struct GatewayEmitter { ... }     // Desktop/server mode — broadcasts to subscribers
```

Pipeline emits events at key points (message received, tool call, response sent). Events are fire-and-forget via `tokio::spawn` — never block the pipeline.

#### Desktop App integration

Replace REST polling with WebSocket:

```typescript
class GatewayClient {
    async connect(url: string, token: string) { ... }
    async call(method: string, params?: any): Promise<any> { ... }
    on(topic: string, callback: Function) { ... }
}
```

Exponential backoff reconnection (1s → 2s → 4s → ... → 30s cap).

#### Config

```yaml
gateway:
  enabled: true
  heartbeat_interval_secs: 30
  max_connections: 50
  max_message_size_kb: 512
  stale_session_timeout_secs: 60
```

### Changes

| File | Change |
|------|--------|
| `rust/crates/amanclaw-gateway/` | **New crate** — `SessionManager`, `GatewayHandler`, `GatewayEmitter` |
| `amanclaw-traits/src/event.rs` | **New** — `EventEmitter` trait, `NoopEmitter` |
| `amanclaw-traits/src/config.rs` | Add `GatewayConfig` |
| `amanclaw-core/src/pipeline.rs` | Add `emitter: Arc<dyn EventEmitter>`, emit events |
| `amanclaw-core/src/lib.rs` | Accept `EventEmitter`, pass to pipeline/scheduler |
| `amanclaw-api/src/lib.rs` | Mount `/ws` route |
| Desktop `src/lib/gateway.ts` | **New** — WS client with RPC, pub/sub, reconnection |
| Desktop pages | Replace REST polling with gateway events |
| `Cargo.toml` (workspace) | Add `amanclaw-gateway` member |

### Dependencies

```toml
# axum already has ws support
axum = { version = "0.8", features = ["ws"] }
uuid = { version = "1", features = ["v4"] }
futures = "0.3"
```

---

## Section 6: Sub-Agent Spawning

### Problem

AmanClaw processes every message in a single pipeline pass. Complex tasks can't be parallelized. OpenClaw solves this with sub-agents — isolated child agents that run concurrently.

### Design

#### Sub-agents as lightweight pipeline executions

Not separate OS processes. They run in tokio tasks with:
- Isolated conversation history (own memory namespace)
- Same or different agent profile
- Cancellation support via `oneshot` channel
- Timeout enforcement

#### Core types

```rust
pub struct SubAgent {
    pub id: String,
    pub parent_session: String,
    pub agent_profile: String,
    pub prompt: String,
    pub status: SubAgentStatus,        // Pending, Running, Completed, Failed, Cancelled
    pub result: Option<String>,
    pub spawned_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub cancel_tx: Option<oneshot::Sender<()>>,
}

pub struct SpawnRequest {
    pub task: String,
    pub agent: Option<String>,
    pub timeout_secs: Option<u64>,     // Default: 120
    pub max_tool_rounds: Option<usize>,
    pub context: Option<String>,       // Extra context from parent
    pub skills: Option<Vec<String>>,   // Restrict skills
    pub depth: usize,                  // Nesting depth tracker
}
```

#### SubAgentManager

```rust
pub struct SubAgentManager {
    agents: Arc<RwLock<HashMap<String, SubAgent>>>,
    // ...
}

impl SubAgentManager {
    pub async fn spawn(&self, parent_session: &str, request: SpawnRequest, parent_profile: &AgentProfile) -> Result<String>;
    pub async fn cancel(&self, sub_id: &str) -> Result<()>;
    pub async fn cancel_all(&self, parent_session: &str) -> Result<usize>;
    pub async fn get(&self, sub_id: &str) -> Option<SubAgentInfo>;
    pub async fn list(&self, parent_session: &str) -> Vec<SubAgentInfo>;
    pub async fn collect_results(&self, parent_session: &str) -> Vec<SubAgentResult>;
    pub async fn cleanup(&self, max_age: Duration);
}
```

#### LLM-driven spawning (sub-agents as tools)

Register `spawn_subagent`, `check_subagents`, `cancel_subagent` as built-in tools. The LLM decides when to parallelize:

```
User: "Plan my family's Hajj trip for 4 people"
LLM → spawn_subagent(task: "Research flights from KLIA to Jeddah")
LLM → spawn_subagent(task: "List visa requirements for Malaysian citizens")
LLM → spawn_subagent(task: "Estimate budget for 4 people")
LLM: "I'm researching in 3 parallel tracks..."
[Next round] → check_subagents() → synthesize results
```

#### Guardrails

```rust
pub struct SubAgentLimits {
    pub max_per_session: usize,        // Default: 5
    pub max_global: usize,             // Default: 20
    pub max_depth: usize,              // Default: 2 (prevents infinite recursion)
    pub default_timeout_secs: u64,     // Default: 120
    pub max_timeout_secs: u64,         // Default: 600
    pub max_tool_rounds: usize,        // Default: 10
    pub allow_nested: bool,            // Default: true
    pub retention_secs: u64,           // Default: 3600
}
```

#### Pipeline integration

Sub-agent messages set `is_subagent: true` → skip auth/rate-limit (like cron/webhooks). Memory namespace: `{parent_namespace}:{sub_id}` — never pollutes parent history.

#### Config

```yaml
subagents:
  enabled: true
  max_per_session: 5
  max_global: 20
  max_depth: 2
  default_timeout_secs: 120
  max_timeout_secs: 600
  allow_nested: true
```

### Changes

| File | Change |
|------|--------|
| `amanclaw-traits/src/message.rs` | Add `is_subagent: bool` to `IncomingMessage` |
| `amanclaw-traits/src/config.rs` | Add `SubAgentConfig` |
| `amanclaw-core/src/subagent.rs` | **New** — `SubAgentManager`, `SubAgent`, `SpawnRequest`, limits |
| `amanclaw-core/src/skills/subagent_skill.rs` | **New** — `SubAgentSkill` (spawn, check, cancel tools) |
| `amanclaw-core/src/lib.rs` | Init `SubAgentManager`, register `SubAgentSkill` |
| `amanclaw-core/src/pipeline.rs` | Skip auth/rate-limit for `is_subagent` |
| `amanclaw-gateway/src/handler.rs` | Add `agent.spawn`, `agent.subagents`, `agent.cancel` |
| `amanclaw-api/src/routes/agents.rs` | Sub-agent REST endpoints |

---

## Section 7: Skill Marketplace / Registry

### Problem

AmanClaw skills are compiled-in or manually configured. No discovery, installation, or sharing mechanism. OpenClaw has ClawHub with 2,857+ skills.

### Design

#### Three-layer system

1. **Skill Package Format** — `amanclaw-skill.toml` manifest + entrypoint + optional soul + data
2. **Local Registry** — SQLite tracking of installed skills with dependency resolution
3. **Remote Registry** — GitHub-based index (JSON file + release tarballs)

#### Skill manifest (`amanclaw-skill.toml`)

```toml
[skill]
name = "zakat-calculator"
version = "1.2.0"
description = "Calculate zakat for income, savings, gold, and agriculture"
author = "aman"
license = "MIT"
keywords = ["islamic", "finance", "zakat"]
categories = ["islamic", "finance"]
min_engine_version = "0.5.0"

[skill.runtime]
type = "python"                       # "wasm" | "python" | "javascript"
entrypoint = "skill_zakat.py"

[skill.permissions]
network = true
filesystem = false
shell = false

[skill.agent_preset]
soul_file = "souls/zakat-advisor.md"

[[skill.knowledge_bases]]
collection = "zakat-fiqh"
source = "data/zakat_rulings.json"

[[skill.dependencies]]
name = "hijri"
version = ">=1.0.0"
optional = true
```

#### Package structure

```
zakat-calculator/
├── amanclaw-skill.toml
├── skill_zakat.py
├── README.md
├── souls/zakat-advisor.md
├── data/zakat_rulings.json
└── tests/test_zakat.py
```

#### Local SkillRegistry

```rust
pub struct SkillRegistry {
    pool: SqlitePool,
    skills_dir: PathBuf,
    remote: Option<RemoteRegistry>,
}

impl SkillRegistry {
    pub async fn install_from_path(&self, path: &Path) -> Result<InstalledSkill>;
    pub async fn install(&self, name: &str, version: Option<&str>) -> Result<InstalledSkill>;
    pub async fn uninstall(&self, name: &str) -> Result<()>;
    pub async fn update(&self, name: &str, version: Option<&str>) -> Result<InstalledSkill>;
    pub async fn list_installed(&self) -> Result<Vec<InstalledSkill>>;
    pub async fn search_installed(&self, query: &str) -> Result<Vec<InstalledSkill>>;
    pub async fn check_updates(&self) -> Result<Vec<SkillUpdate>>;
}
```

#### Remote Registry (GitHub-based)

```rust
pub struct RemoteRegistry {
    index_url: String,                // GitHub raw URL to index.json
    cache_dir: PathBuf,
    http: reqwest::Client,
}
```

Registry index is a JSON file on GitHub. Packages are GitHub release `.tar.gz` assets. SHA-256 checksum verification on every download.

#### Schema

```sql
CREATE TABLE IF NOT EXISTS installed_skills (
    name TEXT PRIMARY KEY,
    version TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    author TEXT NOT NULL DEFAULT '',
    runtime_type TEXT NOT NULL,
    keywords TEXT NOT NULL DEFAULT '[]',
    categories TEXT NOT NULL DEFAULT '[]',
    entrypoint TEXT NOT NULL,
    permissions TEXT NOT NULL DEFAULT '{}',
    installed_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    source TEXT NOT NULL DEFAULT 'local',
    checksum TEXT
);

CREATE TABLE IF NOT EXISTS skill_dependencies (
    skill_name TEXT NOT NULL REFERENCES installed_skills(name),
    depends_on TEXT NOT NULL,
    version_req TEXT NOT NULL,
    optional INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (skill_name, depends_on)
);
```

#### CLI commands

```bash
amanclaw skill search "zakat"
amanclaw skill install zakat-calculator
amanclaw skill install zakat-calculator@1.2.0
amanclaw skill install ./path/to/local/skill
amanclaw skill list
amanclaw skill update --all
amanclaw skill uninstall zakat-calculator
amanclaw skill check-updates
amanclaw skill pack
amanclaw skill verify
```

#### Engine integration

`Engine::new()` auto-loads all installed registry skills (WASM, Python, JS) alongside built-in skills.

#### Permission enforcement

- WASM skills: Wasmtime WASI config (deny network/filesystem if not permitted)
- Script skills: Environment variables (`AMANCLAW_NO_NETWORK=1`)

#### Desktop App — Marketplace page

Browse/search with categories, install/uninstall with one click, update badges, skill detail view with permissions/dependencies/agent presets.

#### Config

```yaml
registry:
  enabled: true
  skills_dir: "./plugins/registry"
  remote_url: "https://raw.githubusercontent.com/amanclaw/registry/main/index.json"
  auto_update_check: true
  auto_update_interval_hours: 24
  allow_unverified: false
```

### Changes

| File | Change |
|------|--------|
| `rust/crates/amanclaw-registry/` | **New crate** — `SkillRegistry`, `RemoteRegistry`, `SkillSandbox`, manifest parsing |
| `amanclaw-traits/src/config.rs` | Add `RegistryConfig` |
| `amanclaw-core/src/lib.rs` | Load registry-installed skills in `Engine::new()` |
| `amanclaw-cli/src/main.rs` | Add `skill` subcommand |
| `amanclaw-api/src/routes/skills.rs` | Extend with registry management endpoints |
| `amanclaw-gateway/src/handler.rs` | Add `skill.*` methods |
| `amanclaw-memory/src/schema.rs` | Add `installed_skills`, `skill_dependencies` tables |
| Desktop `src/lib/pages/Skills.svelte` | **Rewrite** — Marketplace UI |
| `Cargo.toml` (workspace) | Add `amanclaw-registry` member |
| New repo: `amanclaw/registry` | GitHub repo hosting `index.json` |

### Dependencies

```toml
toml = "0.8"
semver = "1"
sha2 = "0.10"
flate2 = "1"
tar = "0.4"
```

---

## Full Crate Changes Summary

### New Crates

| Crate | Purpose |
|-------|---------|
| `amanclaw-gateway` | WebSocket gateway, session manager, event emitter |
| `amanclaw-registry` | Skill marketplace, local + remote registry |

### Modified Crates

| Crate | Changes |
|-------|---------|
| `amanclaw-traits` | Add `EventEmitter` trait, `soul_file` to `AgentProfile`, `is_cron`/`is_webhook`/`is_subagent` to `IncomingMessage`, config structs for cron/webhooks/gateway/subagents/registry |
| `amanclaw-memory` | FTS5 virtual table + triggers in schema, hybrid RRF search in vector store, cron/webhook history + installed skills tables |
| `amanclaw-core` | `SoulLoader`, `Scheduler`, `WebhookRouter`, `SubAgentManager`, `SubAgentSkill`, `EventEmitter` integration, `tokio::select!` in `run()` |
| `amanclaw-api` | Mount `/ws` and `/hooks/*` routes, agent/cron/webhook/skill management endpoints |
| `amanclaw-llm` | No changes |
| `amanclaw-cli` | Add `skill` subcommand |
| Channel crates | No changes |

### New Dependencies

```toml
# amanclaw-core
cron = "0.13"
chrono-tz = "0.10"

# amanclaw-api / amanclaw-core
handlebars = "6"
hmac = "0.12"
sha2 = "0.10"
hex = "0.4"

# amanclaw-gateway
uuid = { version = "1", features = ["v4"] }
futures = "0.3"

# amanclaw-registry
toml = "0.8"
semver = "1"
flate2 = "1"
tar = "0.4"
```

### Desktop App New Files

| File | Purpose |
|------|---------|
| `src/lib/gateway.ts` | WebSocket client with RPC, pub/sub, reconnection |
| `src/lib/pages/Agents.svelte` | Agent management with soul editor |
| `src/lib/pages/Scheduler.svelte` | Cron job management |
| `src/lib/pages/Webhooks.svelte` | Webhook management |
| `src/lib/pages/Skills.svelte` | Skill marketplace (rewrite) |
