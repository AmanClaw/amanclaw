# Desktop App Parity Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add 9 new/updated pages to the Tauri desktop app to achieve full parity with the core engine's features.

**Architecture:** Local-first approach. All new IPC commands access engine state directly via `engine_handle` and `config::` helpers. Svelte 5 pages follow the existing pattern: `$state` runes, `$derived`, `onMount` polling, `api.ts` wrappers. Each feature gets backend commands first, then frontend page.

**Tech Stack:** Rust (Tauri 2 commands), Svelte 5 + Tailwind CSS 4, SQLx (SQLite queries for history tables)

**Design doc:** `docs/plans/2026-03-09-desktop-parity-design.md`

---

## Task 1: Foundation — Update EngineHandle and State

**Files:**
- Modify: `desktop/src-tauri/src/state.rs`

**Step 1: Add new fields to EngineHandle**

Add `subagent_manager` and `scheduler_config` to `EngineHandle` so commands can access them. Also add an `engine_config` field so commands can read full config without reloading from disk.

In `desktop/src-tauri/src/state.rs`, update `EngineHandle`:

```rust
pub struct EngineHandle {
    pub abort_handle: tokio::task::AbortHandle,
    pub auth: Arc<std::sync::Mutex<amanclaw_security::auth::Auth>>,
    pub pool: sqlx::SqlitePool,
    pub registry: Arc<amanclaw_core::registry::PluginRegistry>,
    pub subagent_manager: Option<Arc<amanclaw_core::subagent::SubAgentManager>>,
}
```

**Step 2: Update EngineHandle construction in commands.rs**

In `desktop/src-tauri/src/commands.rs`, in the `start_engine` function where `EngineHandle` is created, add:

```rust
subagent_manager: None, // Will be populated when subagent commands are added
```

**Step 3: Run build**

Run: `cd desktop/src-tauri && cargo build`
Expected: Compiles.

**Step 4: Commit**

```bash
git add desktop/src-tauri/src/state.rs desktop/src-tauri/src/commands.rs
git commit -m "feat(desktop): extend EngineHandle with subagent_manager field"
```

---

## Task 2: Foundation — Update Sidebar Navigation

**Files:**
- Modify: `desktop/src/lib/components/Sidebar.svelte`

**Step 1: Add new pages to navigation**

Update the `pages` array in `Sidebar.svelte` to include all new pages in the designed order:

```typescript
const pages = [
    { id: 'dashboard', label: 'Dashboard', icon: '&#9632;' },
    { id: 'communities', label: 'Communities', icon: '&#127970;' },
    { id: 'agents', label: 'Agents', icon: '&#129302;' },
    { id: 'skills', label: 'Skills', icon: '&#9881;' },
    { id: 'marketplace', label: 'Marketplace', icon: '&#128722;' },
    { id: 'cron', label: 'Cron Jobs', icon: '&#9200;' },
    { id: 'webhooks', label: 'Webhooks', icon: '&#128268;' },
    { id: 'gateway', label: 'Gateway', icon: '&#9889;' },
    { id: 'subagents', label: 'Sub-Agents', icon: '&#129520;' },
    { id: 'knowledgebases', label: 'Knowledge Bases', icon: '&#128218;' },
    { id: 'content', label: 'Content', icon: '&#128214;' },
    { id: 'users', label: 'Users', icon: '&#128101;' },
    { id: 'mcpservers', label: 'MCP Servers', icon: '&#128640;' },
    { id: 'logs', label: 'Logs', icon: '&#128203;' },
];
```

**Step 2: Add page routing in +page.svelte**

In `desktop/src/routes/+page.svelte`, add imports and route cases for all new pages. Import each new page component and add `{:else if $currentPage === 'agents'}` blocks etc. Use placeholder `<p>Coming soon</p>` for pages not yet built.

**Step 3: Commit**

```bash
git add desktop/src/lib/components/Sidebar.svelte desktop/src/routes/+page.svelte
git commit -m "feat(desktop): add all new pages to sidebar navigation"
```

---

## Task 3: Agents — Backend Commands

**Files:**
- Modify: `desktop/src-tauri/src/commands.rs`
- Modify: `desktop/src-tauri/src/lib.rs` (register commands)

**Step 1: Add agent commands**

Add these commands to `commands.rs`:

```rust
#[tauri::command]
pub async fn list_agents(
    app: AppHandle,
) -> Result<serde_json::Value, String> {
    let cfg = config::load_config(&app)?;
    let agents: Vec<serde_json::Value> = cfg.agents.iter().map(|(id, profile)| {
        serde_json::json!({
            "id": id,
            "name": profile.name,
            "system_prompt": profile.system_prompt,
            "soul_file": profile.soul_file,
            "allowed_skills": profile.allowed_skills,
            "memory_namespace": profile.memory_namespace,
        })
    }).collect();
    Ok(serde_json::json!({ "agents": agents, "count": agents.len() }))
}

#[tauri::command]
pub async fn save_agent(
    app: AppHandle,
    id: String,
    name: String,
    system_prompt: String,
    soul_file: Option<String>,
    allowed_skills: Vec<String>,
    memory_namespace: String,
) -> Result<(), String> {
    let mut cfg = config::load_config(&app)?;
    let profile = amanclaw_traits::agent::AgentProfile {
        id: id.clone(),
        name,
        system_prompt,
        soul_file,
        allowed_skills,
        memory_namespace,
    };
    cfg.agents.insert(id, profile);
    config::save_config(&app, &cfg)
}

#[tauri::command]
pub async fn delete_agent(
    app: AppHandle,
    id: String,
) -> Result<(), String> {
    let mut cfg = config::load_config(&app)?;
    cfg.agents.remove(&id);
    config::save_config(&app, &cfg)
}

#[tauri::command]
pub async fn load_soul_file(
    app: AppHandle,
    filename: String,
) -> Result<String, String> {
    let cfg = config::load_config(&app)?;
    let soul_dir = std::path::Path::new(&cfg.skills.soul_dir);
    let path = soul_dir.join(&filename);
    std::fs::read_to_string(&path).map_err(|e| format!("Failed to read {}: {}", filename, e))
}

#[tauri::command]
pub async fn save_soul_file(
    app: AppHandle,
    filename: String,
    content: String,
) -> Result<(), String> {
    let cfg = config::load_config(&app)?;
    let soul_dir = std::path::Path::new(&cfg.skills.soul_dir);
    std::fs::create_dir_all(soul_dir).map_err(|e| e.to_string())?;
    let path = soul_dir.join(&filename);
    std::fs::write(&path, &content).map_err(|e| format!("Failed to write {}: {}", filename, e))
}

#[tauri::command]
pub async fn preview_soul(
    app: AppHandle,
    filename: String,
) -> Result<serde_json::Value, String> {
    let cfg = config::load_config(&app)?;
    let soul_dir = std::path::Path::new(&cfg.skills.soul_dir);
    match amanclaw_core::soul::SoulLoader::load(soul_dir, &filename) {
        Ok(resolved) => Ok(serde_json::json!({
            "prompt": resolved.prompt,
            "variables": resolved.variables,
            "tags": resolved.tags,
        })),
        Err(e) => Err(format!("Failed to resolve soul: {}", e)),
    }
}

#[tauri::command]
pub async fn get_routing_rules(
    app: AppHandle,
) -> Result<serde_json::Value, String> {
    let cfg = config::load_config(&app)?;
    let rules: Vec<serde_json::Value> = cfg.routing.rules.iter().map(|r| {
        serde_json::json!({
            "match": {
                "platform": r.match_criteria.platform,
                "topic_id": r.match_criteria.topic_id,
                "channel_id": r.match_criteria.channel_id,
                "group_id": r.match_criteria.group_id,
            },
            "agent": r.agent,
        })
    }).collect();
    Ok(serde_json::json!({
        "rules": rules,
        "default_agent": cfg.routing.default_agent,
    }))
}

#[tauri::command]
pub async fn save_routing_rules(
    app: AppHandle,
    default_agent: String,
    rules: Vec<serde_json::Value>,
) -> Result<(), String> {
    let mut cfg = config::load_config(&app)?;
    cfg.routing.default_agent = default_agent;
    cfg.routing.rules = rules.iter().map(|r| {
        amanclaw_traits::config::RoutingRule {
            match_criteria: amanclaw_traits::config::RoutingMatch {
                platform: r["match"]["platform"].as_str().map(String::from),
                topic_id: r["match"]["topic_id"].as_str().map(String::from),
                channel_id: r["match"]["channel_id"].as_str().map(String::from),
                group_id: r["match"]["group_id"].as_str().map(String::from),
            },
            agent: r["agent"].as_str().unwrap_or("default").to_string(),
        }
    }).collect();
    config::save_config(&app, &cfg)
}
```

**Step 2: Register commands in lib.rs**

Add all new commands to the `invoke_handler` list in `lib.rs`.

**Step 3: Run build**

Run: `cd desktop/src-tauri && cargo build`
Expected: Compiles.

**Step 4: Commit**

```bash
git add desktop/src-tauri/src/commands.rs desktop/src-tauri/src/lib.rs
git commit -m "feat(desktop): add agent management IPC commands"
```

---

## Task 4: Agents — Frontend Page

**Files:**
- Create: `desktop/src/lib/pages/Agents.svelte`
- Modify: `desktop/src/lib/api.ts`
- Modify: `desktop/src/routes/+page.svelte`

**Step 1: Add API functions**

In `api.ts`, add:

```typescript
// Agents
listAgents: () => invoke('list_agents'),
saveAgent: (params: {
    id: string; name: string; systemPrompt: string;
    soulFile?: string; allowedSkills: string[]; memoryNamespace: string;
}) => invoke('save_agent', params),
deleteAgent: (id: string) => invoke('delete_agent', { id }),
loadSoulFile: (filename: string) => invoke('load_soul_file', { filename }) as Promise<string>,
saveSoulFile: (filename: string, content: string) => invoke('save_soul_file', { filename, content }),
previewSoul: (filename: string) => invoke('preview_soul', { filename }),
getRoutingRules: () => invoke('get_routing_rules'),
saveRoutingRules: (defaultAgent: string, rules: any[]) =>
    invoke('save_routing_rules', { defaultAgent, rules }),
```

**Step 2: Create Agents.svelte**

Create `desktop/src/lib/pages/Agents.svelte` with:
- Split view: agent list (left 1/3) + editor (right 2/3)
- Agent list with cards, "Add Agent" button
- Editor with profile fields, SOUL.md textarea with preview toggle
- Routing rules table at bottom with add/edit/delete
- Follow existing Skills.svelte patterns for state, polling, styling

**Step 3: Wire up routing in +page.svelte**

Import `Agents` and add the route case.

**Step 4: Run dev server and verify**

Run: `cd desktop && npm run dev` (in one terminal) + `cd desktop/src-tauri && cargo tauri dev` (if needed)
Expected: Agents page renders, can add/edit/delete agents.

**Step 5: Commit**

```bash
git add desktop/src/lib/pages/Agents.svelte desktop/src/lib/api.ts desktop/src/routes/+page.svelte
git commit -m "feat(desktop): add Agents page with SOUL.md editor and routing rules"
```

---

## Task 5: Cron Jobs — Backend Commands

**Files:**
- Modify: `desktop/src-tauri/src/commands.rs`
- Modify: `desktop/src-tauri/src/lib.rs`

**Step 1: Add cron commands**

```rust
#[tauri::command]
pub async fn list_cron_jobs(
    app: AppHandle,
) -> Result<serde_json::Value, String> {
    let cfg = config::load_config(&app)?;
    let jobs: Vec<serde_json::Value> = cfg.cron.jobs.iter().map(|(id, job)| {
        serde_json::json!({
            "id": id,
            "name": job.name,
            "schedule": job.schedule,
            "timezone": job.timezone,
            "type": job.job_type,
            "skill": job.skill,
            "input": job.input,
            "prompt": job.prompt,
            "template": job.template,
            "targets": job.targets.iter().map(|t| serde_json::json!({
                "platform": t.platform,
                "chat_id": t.chat_id,
                "topic_id": t.topic_id,
            })).collect::<Vec<_>>(),
            "agent": job.agent,
            "enabled": job.enabled,
        })
    }).collect();
    Ok(serde_json::json!({
        "jobs": jobs,
        "count": jobs.len(),
        "timezone": cfg.cron.timezone,
    }))
}

#[tauri::command]
pub async fn save_cron_job(
    app: AppHandle,
    id: String,
    job: serde_json::Value,
) -> Result<(), String> {
    let mut cfg = config::load_config(&app)?;
    let cron_job: amanclaw_traits::config::CronJobConfig =
        serde_json::from_value(job).map_err(|e| format!("Invalid job config: {}", e))?;
    cfg.cron.jobs.insert(id, cron_job);
    config::save_config(&app, &cfg)
}

#[tauri::command]
pub async fn delete_cron_job(
    app: AppHandle,
    id: String,
) -> Result<(), String> {
    let mut cfg = config::load_config(&app)?;
    cfg.cron.jobs.remove(&id);
    config::save_config(&app, &cfg)
}

#[tauri::command]
pub async fn get_cron_history(
    state: State<'_, SharedState>,
) -> Result<serde_json::Value, String> {
    let st = state.read().await;
    if let Some(handle) = &st.engine_handle {
        let rows = sqlx::query_as::<_, (i64, String, String, Option<String>, Option<i64>, String)>(
            "SELECT id, job_id, status, output, duration_ms, executed_at FROM cron_history ORDER BY executed_at DESC LIMIT 100"
        )
        .fetch_all(&handle.pool)
        .await
        .map_err(|e| e.to_string())?;

        let entries: Vec<serde_json::Value> = rows.iter().map(|r| {
            serde_json::json!({
                "id": r.0, "job_id": r.1, "status": r.2,
                "output": r.3, "duration_ms": r.4, "executed_at": r.5,
            })
        }).collect();
        Ok(serde_json::json!({ "entries": entries, "count": entries.len() }))
    } else {
        Ok(serde_json::json!({ "entries": [], "count": 0 }))
    }
}
```

**Step 2: Register commands in lib.rs**

**Step 3: Run build**

Run: `cd desktop/src-tauri && cargo build`
Expected: Compiles.

**Step 4: Commit**

```bash
git add desktop/src-tauri/src/commands.rs desktop/src-tauri/src/lib.rs
git commit -m "feat(desktop): add cron job management IPC commands"
```

---

## Task 6: Cron Jobs — Frontend Page

**Files:**
- Create: `desktop/src/lib/pages/CronJobs.svelte`
- Modify: `desktop/src/lib/api.ts`
- Modify: `desktop/src/routes/+page.svelte`

**Step 1: Add API functions**

```typescript
// Cron Jobs
listCronJobs: () => invoke('list_cron_jobs'),
saveCronJob: (id: string, job: any) => invoke('save_cron_job', { id, job }),
deleteCronJob: (id: string) => invoke('delete_cron_job', { id }),
getCronHistory: () => invoke('get_cron_history'),
```

**Step 2: Create CronJobs.svelte**

Two tabs: "Jobs" (table with add/edit/delete, enabled toggle) and "History" (read-only table from cron_history). Follow Skills.svelte tab pattern. Add form with schedule input, type radio (direct_message/skill_invocation/agent_prompt), type-specific fields, targets list.

**Step 3: Wire routing, test, commit**

```bash
git add desktop/src/lib/pages/CronJobs.svelte desktop/src/lib/api.ts desktop/src/routes/+page.svelte
git commit -m "feat(desktop): add Cron Jobs page with scheduling and history"
```

---

## Task 7: Webhooks — Backend Commands

**Files:**
- Modify: `desktop/src-tauri/src/commands.rs`
- Modify: `desktop/src-tauri/src/lib.rs`

**Step 1: Add webhook commands**

```rust
#[tauri::command]
pub async fn list_webhook_endpoints(
    app: AppHandle,
) -> Result<serde_json::Value, String> {
    let cfg = config::load_config(&app)?;
    let endpoints: Vec<serde_json::Value> = cfg.webhooks.endpoints.iter().map(|(id, ep)| {
        serde_json::json!({
            "id": id,
            "name": ep.name,
            "path": ep.path,
            "auth": { "type": ep.auth.auth_type },
            "transform": { "type": ep.transform.transform_type },
            "targets": ep.targets.iter().map(|t| serde_json::json!({
                "platform": t.platform, "chat_id": t.chat_id, "topic_id": t.topic_id,
            })).collect::<Vec<_>>(),
            "agent": ep.agent,
            "rate_limit": ep.rate_limit,
            "enabled": ep.enabled,
        })
    }).collect();
    Ok(serde_json::json!({
        "endpoints": endpoints,
        "count": endpoints.len(),
        "base_path": cfg.webhooks.base_path,
    }))
}

#[tauri::command]
pub async fn save_webhook_endpoint(
    app: AppHandle,
    id: String,
    endpoint: serde_json::Value,
) -> Result<(), String> {
    let mut cfg = config::load_config(&app)?;
    let ep: amanclaw_traits::config::WebhookEndpointConfig =
        serde_json::from_value(endpoint).map_err(|e| format!("Invalid webhook config: {}", e))?;
    cfg.webhooks.endpoints.insert(id, ep);
    config::save_config(&app, &cfg)
}

#[tauri::command]
pub async fn delete_webhook_endpoint(
    app: AppHandle,
    id: String,
) -> Result<(), String> {
    let mut cfg = config::load_config(&app)?;
    cfg.webhooks.endpoints.remove(&id);
    config::save_config(&app, &cfg)
}

#[tauri::command]
pub async fn get_webhook_history(
    state: State<'_, SharedState>,
) -> Result<serde_json::Value, String> {
    let st = state.read().await;
    if let Some(handle) = &st.engine_handle {
        let rows = sqlx::query_as::<_, (i64, String, String, Option<String>, Option<String>, Option<String>, Option<i64>, String)>(
            "SELECT id, webhook_id, status, source_ip, payload_preview, error, duration_ms, received_at FROM webhook_history ORDER BY received_at DESC LIMIT 100"
        )
        .fetch_all(&handle.pool)
        .await
        .map_err(|e| e.to_string())?;

        let entries: Vec<serde_json::Value> = rows.iter().map(|r| {
            serde_json::json!({
                "id": r.0, "webhook_id": r.1, "status": r.2, "source_ip": r.3,
                "payload_preview": r.4, "error": r.5, "duration_ms": r.6, "received_at": r.7,
            })
        }).collect();
        Ok(serde_json::json!({ "entries": entries, "count": entries.len() }))
    } else {
        Ok(serde_json::json!({ "entries": [], "count": 0 }))
    }
}
```

**Step 2: Register, build, commit**

```bash
git add desktop/src-tauri/src/commands.rs desktop/src-tauri/src/lib.rs
git commit -m "feat(desktop): add webhook management IPC commands"
```

---

## Task 8: Webhooks — Frontend Page

**Files:**
- Create: `desktop/src/lib/pages/Webhooks.svelte`
- Modify: `desktop/src/lib/api.ts`
- Modify: `desktop/src/routes/+page.svelte`

**Step 1: Add API functions**

```typescript
// Webhooks
listWebhookEndpoints: () => invoke('list_webhook_endpoints'),
saveWebhookEndpoint: (id: string, endpoint: any) => invoke('save_webhook_endpoint', { id, endpoint }),
deleteWebhookEndpoint: (id: string) => invoke('delete_webhook_endpoint', { id }),
getWebhookHistory: () => invoke('get_webhook_history'),
```

**Step 2: Create Webhooks.svelte**

Two tabs: "Endpoints" and "History". Same pattern as CronJobs. Auth type dropdown (none/hmac_sha256/bearer/header_match) with conditional fields. Transform type dropdown with conditional fields. Targets editor. Copyable webhook URL.

**Step 3: Wire routing, test, commit**

```bash
git add desktop/src/lib/pages/Webhooks.svelte desktop/src/lib/api.ts desktop/src/routes/+page.svelte
git commit -m "feat(desktop): add Webhooks page with endpoint config and history"
```

---

## Task 9: Gateway — Backend Commands

**Files:**
- Modify: `desktop/src-tauri/src/commands.rs`
- Modify: `desktop/src-tauri/src/lib.rs`

**Step 1: Add gateway commands**

```rust
#[tauri::command]
pub async fn get_gateway_config(
    app: AppHandle,
) -> Result<serde_json::Value, String> {
    let cfg = config::load_config(&app)?;
    Ok(serde_json::json!({
        "enabled": cfg.gateway.enabled,
        "heartbeat_interval_secs": cfg.gateway.heartbeat_interval_secs,
        "max_connections": cfg.gateway.max_connections,
        "stale_session_timeout_secs": cfg.gateway.stale_session_timeout_secs,
    }))
}

#[tauri::command]
pub async fn save_gateway_config(
    app: AppHandle,
    enabled: bool,
    heartbeat_interval_secs: u64,
    max_connections: usize,
    stale_session_timeout_secs: u64,
) -> Result<(), String> {
    let mut cfg = config::load_config(&app)?;
    cfg.gateway.enabled = enabled;
    cfg.gateway.heartbeat_interval_secs = heartbeat_interval_secs;
    cfg.gateway.max_connections = max_connections;
    cfg.gateway.stale_session_timeout_secs = stale_session_timeout_secs;
    config::save_config(&app, &cfg)
}

#[tauri::command]
pub async fn get_gateway_status(
    app: AppHandle,
) -> Result<serde_json::Value, String> {
    let cfg = config::load_config(&app)?;
    Ok(serde_json::json!({
        "enabled": cfg.gateway.enabled,
        "connection_count": 0, // TODO: query from gateway session manager when available
    }))
}
```

**Step 2: Register, build, commit**

```bash
git add desktop/src-tauri/src/commands.rs desktop/src-tauri/src/lib.rs
git commit -m "feat(desktop): add gateway configuration IPC commands"
```

---

## Task 10: Gateway — Frontend Page

**Files:**
- Create: `desktop/src/lib/pages/Gateway.svelte`
- Modify: `desktop/src/lib/api.ts`
- Modify: `desktop/src/routes/+page.svelte`

**Step 1: Add API functions**

```typescript
// Gateway
getGatewayConfig: () => invoke('get_gateway_config'),
saveGatewayConfig: (params: {
    enabled: boolean; heartbeatIntervalSecs: number;
    maxConnections: number; staleSessionTimeoutSecs: number;
}) => invoke('save_gateway_config', params),
getGatewayStatus: () => invoke('get_gateway_status'),
```

**Step 2: Create Gateway.svelte**

Top panel: config form (enabled toggle, heartbeat, max connections, stale timeout). Bottom panel: live events feed. Use browser-native `WebSocket` to connect to `ws://127.0.0.1:{apiPort}/ws` when enabled. Subscribe to `*`, display events in a scrolling list with color-coded topics. Pause/resume/clear buttons.

Note: The WebSocket connection is made directly from the Svelte component using the browser WebSocket API (Tauri webview supports this). No IPC command needed for the live feed.

**Step 3: Wire routing, test, commit**

```bash
git add desktop/src/lib/pages/Gateway.svelte desktop/src/lib/api.ts desktop/src/routes/+page.svelte
git commit -m "feat(desktop): add Gateway page with live WebSocket event feed"
```

---

## Task 11: Sub-Agents — Backend Commands

**Files:**
- Modify: `desktop/src-tauri/src/commands.rs`
- Modify: `desktop/src-tauri/src/lib.rs`

**Step 1: Add sub-agent commands**

```rust
#[tauri::command]
pub async fn get_subagent_config(
    app: AppHandle,
) -> Result<serde_json::Value, String> {
    let cfg = config::load_config(&app)?;
    Ok(serde_json::json!({
        "enabled": cfg.subagents.enabled,
        "max_per_session": cfg.subagents.max_per_session,
        "max_global": cfg.subagents.max_global,
        "max_depth": cfg.subagents.max_depth,
        "default_timeout_secs": cfg.subagents.default_timeout_secs,
    }))
}

#[tauri::command]
pub async fn save_subagent_config(
    app: AppHandle,
    enabled: bool,
    max_per_session: usize,
    max_global: usize,
    max_depth: usize,
    default_timeout_secs: u64,
) -> Result<(), String> {
    let mut cfg = config::load_config(&app)?;
    cfg.subagents.enabled = enabled;
    cfg.subagents.max_per_session = max_per_session;
    cfg.subagents.max_global = max_global;
    cfg.subagents.max_depth = max_depth;
    cfg.subagents.default_timeout_secs = default_timeout_secs;
    config::save_config(&app, &cfg)
}

#[tauri::command]
pub async fn list_subagents(
    state: State<'_, SharedState>,
    session_filter: Option<String>,
) -> Result<serde_json::Value, String> {
    let st = state.read().await;
    if let Some(handle) = &st.engine_handle {
        if let Some(mgr) = &handle.subagent_manager {
            let agents = if let Some(session) = session_filter {
                mgr.list(&session).await
            } else {
                // List all — collect_results needs a session, so we return empty for now
                // In practice the UI would filter by session
                vec![]
            };
            let list: Vec<serde_json::Value> = agents.iter().map(|a| {
                serde_json::json!({
                    "id": a.id,
                    "agent_id": a.agent_id,
                    "prompt": a.prompt,
                    "parent_session": a.parent_session,
                    "depth": a.depth,
                    "status": format!("{:?}", a.status),
                })
            }).collect();
            return Ok(serde_json::json!({ "subagents": list, "count": list.len() }));
        }
    }
    Ok(serde_json::json!({ "subagents": [], "count": 0 }))
}

#[tauri::command]
pub async fn cancel_subagent(
    state: State<'_, SharedState>,
    id: String,
) -> Result<bool, String> {
    let st = state.read().await;
    if let Some(handle) = &st.engine_handle {
        if let Some(mgr) = &handle.subagent_manager {
            return Ok(mgr.cancel(&id).await);
        }
    }
    Ok(false)
}

#[tauri::command]
pub async fn cancel_all_subagents(
    state: State<'_, SharedState>,
    session: String,
) -> Result<usize, String> {
    let st = state.read().await;
    if let Some(handle) = &st.engine_handle {
        if let Some(mgr) = &handle.subagent_manager {
            return Ok(mgr.cancel_all(&session).await);
        }
    }
    Ok(0)
}
```

**Step 2: Register, build, commit**

```bash
git add desktop/src-tauri/src/commands.rs desktop/src-tauri/src/lib.rs
git commit -m "feat(desktop): add sub-agent monitoring IPC commands"
```

---

## Task 12: Sub-Agents — Frontend Page

**Files:**
- Create: `desktop/src/lib/pages/SubAgents.svelte`
- Modify: `desktop/src/lib/api.ts`
- Modify: `desktop/src/routes/+page.svelte`

**Step 1: Add API functions**

```typescript
// Sub-Agents
getSubagentConfig: () => invoke('get_subagent_config'),
saveSubagentConfig: (params: {
    enabled: boolean; maxPerSession: number; maxGlobal: number;
    maxDepth: number; defaultTimeoutSecs: number;
}) => invoke('save_subagent_config', params),
listSubagents: (sessionFilter?: string) => invoke('list_subagents', { sessionFilter }),
cancelSubagent: (id: string) => invoke('cancel_subagent', { id }) as Promise<boolean>,
cancelAllSubagents: (session: string) => invoke('cancel_all_subagents', { session }) as Promise<number>,
```

**Step 2: Create SubAgents.svelte**

Collapsible config panel (top) with enabled toggle and limit fields. Main table with 5-second polling showing sub-agents with status badges (Running=blue pulse, Completed=green, Failed=red, Cancelled=gray). Cancel button per row. Summary bar at top. Session filter input.

**Step 3: Wire routing, test, commit**

```bash
git add desktop/src/lib/pages/SubAgents.svelte desktop/src/lib/api.ts desktop/src/routes/+page.svelte
git commit -m "feat(desktop): add Sub-Agents monitoring page"
```

---

## Task 13: Marketplace — Backend Commands

**Files:**
- Modify: `desktop/src-tauri/src/commands.rs`
- Modify: `desktop/src-tauri/src/lib.rs`

**Step 1: Add registry commands**

```rust
#[tauri::command]
pub async fn registry_list_installed(
    state: State<'_, SharedState>,
    app: AppHandle,
) -> Result<serde_json::Value, String> {
    let st = state.read().await;
    if let Some(handle) = &st.engine_handle {
        let cfg = config::load_config(&app)?;
        let registry = amanclaw_registry::local::SkillRegistry::new(
            handle.pool.clone(), cfg.registry.skills_dir.clone()
        ).await.map_err(|e| e.to_string())?;

        let installed = registry.list_installed().await.map_err(|e| e.to_string())?;
        let list: Vec<serde_json::Value> = installed.iter().map(|s| {
            serde_json::json!({
                "name": s.name, "version": s.version, "skill_type": s.skill_type,
                "description": s.description, "entry": s.entry,
                "install_dir": s.install_dir, "installed_at": s.installed_at,
            })
        }).collect();
        Ok(serde_json::json!({ "skills": list, "count": list.len() }))
    } else {
        Ok(serde_json::json!({ "skills": [], "count": 0 }))
    }
}

#[tauri::command]
pub async fn registry_install_from_path(
    state: State<'_, SharedState>,
    app: AppHandle,
    path: String,
) -> Result<serde_json::Value, String> {
    let st = state.read().await;
    if let Some(handle) = &st.engine_handle {
        let cfg = config::load_config(&app)?;
        let registry = amanclaw_registry::local::SkillRegistry::new(
            handle.pool.clone(), cfg.registry.skills_dir.clone()
        ).await.map_err(|e| e.to_string())?;

        let installed = registry.install_from_path(std::path::Path::new(&path))
            .await.map_err(|e| e.to_string())?;
        Ok(serde_json::json!({
            "name": installed.name, "version": installed.version,
        }))
    } else {
        Err("Engine not running".into())
    }
}

#[tauri::command]
pub async fn registry_uninstall(
    state: State<'_, SharedState>,
    app: AppHandle,
    name: String,
) -> Result<bool, String> {
    let st = state.read().await;
    if let Some(handle) = &st.engine_handle {
        let cfg = config::load_config(&app)?;
        let registry = amanclaw_registry::local::SkillRegistry::new(
            handle.pool.clone(), cfg.registry.skills_dir.clone()
        ).await.map_err(|e| e.to_string())?;
        registry.uninstall(&name).await.map_err(|e| e.to_string())
    } else {
        Err("Engine not running".into())
    }
}

#[tauri::command]
pub async fn registry_search_installed(
    state: State<'_, SharedState>,
    app: AppHandle,
    query: String,
) -> Result<serde_json::Value, String> {
    let st = state.read().await;
    if let Some(handle) = &st.engine_handle {
        let cfg = config::load_config(&app)?;
        let registry = amanclaw_registry::local::SkillRegistry::new(
            handle.pool.clone(), cfg.registry.skills_dir.clone()
        ).await.map_err(|e| e.to_string())?;

        let results = registry.search_installed(&query).await.map_err(|e| e.to_string())?;
        let list: Vec<serde_json::Value> = results.iter().map(|s| {
            serde_json::json!({
                "name": s.name, "version": s.version, "skill_type": s.skill_type,
                "description": s.description, "installed_at": s.installed_at,
            })
        }).collect();
        Ok(serde_json::json!({ "skills": list, "count": list.len() }))
    } else {
        Ok(serde_json::json!({ "skills": [], "count": 0 }))
    }
}
```

**Step 2: Add amanclaw-registry dependency to desktop Cargo.toml**

In `desktop/src-tauri/Cargo.toml`, add:
```toml
amanclaw-registry = { path = "../../rust/crates/amanclaw-registry" }
```

**Step 3: Register, build, commit**

```bash
git add desktop/src-tauri/src/commands.rs desktop/src-tauri/src/lib.rs desktop/src-tauri/Cargo.toml
git commit -m "feat(desktop): add skill marketplace registry IPC commands"
```

---

## Task 14: Marketplace — Frontend Page

**Files:**
- Create: `desktop/src/lib/pages/Marketplace.svelte`
- Modify: `desktop/src/lib/api.ts`
- Modify: `desktop/src/routes/+page.svelte`

**Step 1: Add API functions**

```typescript
// Marketplace / Registry
registryListInstalled: () => invoke('registry_list_installed'),
registryInstallFromPath: (path: string) => invoke('registry_install_from_path', { path }),
registryUninstall: (name: string) => invoke('registry_uninstall', { name }) as Promise<boolean>,
registrySearchInstalled: (query: string) => invoke('registry_search_installed', { query }),
```

**Step 2: Create Marketplace.svelte**

Three tabs: "Browse" (remote index — show empty state for now since no remote configured), "Installed" (table from registry_list_installed with uninstall + search), "Publish" (stub with info text). "Install from folder" button using Tauri file dialog.

**Step 3: Wire routing, test, commit**

```bash
git add desktop/src/lib/pages/Marketplace.svelte desktop/src/lib/api.ts desktop/src/routes/+page.svelte
git commit -m "feat(desktop): add Marketplace page with registry install/search/uninstall"
```

---

## Task 15: Knowledge Bases — Backend Commands

**Files:**
- Modify: `desktop/src-tauri/src/commands.rs`
- Modify: `desktop/src-tauri/src/lib.rs`

**Step 1: Add knowledge base commands**

```rust
#[tauri::command]
pub async fn get_embedding_config(
    app: AppHandle,
) -> Result<serde_json::Value, String> {
    let cfg = config::load_config(&app)?;
    match &cfg.embeddings {
        Some(ec) => Ok(serde_json::json!({
            "configured": true,
            "base_url": ec.base_url,
            "model": ec.model,
            "api_key": ec.api_key.is_some(),
        })),
        None => Ok(serde_json::json!({ "configured": false })),
    }
}

#[tauri::command]
pub async fn save_embedding_config(
    app: AppHandle,
    base_url: String,
    model: String,
    api_key: Option<String>,
) -> Result<(), String> {
    let mut cfg = config::load_config(&app)?;
    cfg.embeddings = Some(amanclaw_traits::config::EmbeddingConfig {
        base_url, model, api_key,
    });
    config::save_config(&app, &cfg)
}

#[tauri::command]
pub async fn get_vector_config(
    app: AppHandle,
) -> Result<serde_json::Value, String> {
    let cfg = config::load_config(&app)?;
    match &cfg.vector {
        Some(vc) => Ok(serde_json::json!({
            "configured": true,
            "backend": vc.backend,
            "qdrant_url": vc.qdrant_url,
        })),
        None => Ok(serde_json::json!({ "configured": false, "backend": "sqlite-vec" })),
    }
}

#[tauri::command]
pub async fn save_vector_config(
    app: AppHandle,
    backend: String,
    qdrant_url: Option<String>,
) -> Result<(), String> {
    let mut cfg = config::load_config(&app)?;
    cfg.vector = Some(amanclaw_traits::config::VectorConfig {
        backend, qdrant_url,
    });
    config::save_config(&app, &cfg)
}

#[tauri::command]
pub async fn list_knowledge_bases(
    app: AppHandle,
) -> Result<serde_json::Value, String> {
    let cfg = config::load_config(&app)?;
    let kbs: Vec<serde_json::Value> = cfg.knowledge_bases.iter().map(|(name, kb)| {
        serde_json::json!({
            "name": name,
            "collection": kb.collection,
            "source": kb.source,
        })
    }).collect();
    Ok(serde_json::json!({ "knowledge_bases": kbs, "count": kbs.len() }))
}

#[tauri::command]
pub async fn save_knowledge_base(
    app: AppHandle,
    name: String,
    collection: String,
    source: String,
) -> Result<(), String> {
    let mut cfg = config::load_config(&app)?;
    cfg.knowledge_bases.insert(name, amanclaw_traits::config::KnowledgeBaseConfig {
        collection, source,
    });
    config::save_config(&app, &cfg)
}

#[tauri::command]
pub async fn delete_knowledge_base(
    app: AppHandle,
    name: String,
) -> Result<(), String> {
    let mut cfg = config::load_config(&app)?;
    cfg.knowledge_bases.remove(&name);
    config::save_config(&app, &cfg)
}
```

**Step 2: Register, build, commit**

```bash
git add desktop/src-tauri/src/commands.rs desktop/src-tauri/src/lib.rs
git commit -m "feat(desktop): add knowledge base and embedding config IPC commands"
```

---

## Task 16: Knowledge Bases — Frontend Page

**Files:**
- Create: `desktop/src/lib/pages/KnowledgeBases.svelte`
- Modify: `desktop/src/lib/api.ts`
- Modify: `desktop/src/routes/+page.svelte`

**Step 1: Add API functions**

```typescript
// Knowledge Bases
getEmbeddingConfig: () => invoke('get_embedding_config'),
saveEmbeddingConfig: (params: { baseUrl: string; model: string; apiKey?: string }) =>
    invoke('save_embedding_config', params),
getVectorConfig: () => invoke('get_vector_config'),
saveVectorConfig: (params: { backend: string; qdrantUrl?: string }) =>
    invoke('save_vector_config', params),
listKnowledgeBases: () => invoke('list_knowledge_bases'),
saveKnowledgeBase: (name: string, collection: string, source: string) =>
    invoke('save_knowledge_base', { name, collection, source }),
deleteKnowledgeBase: (name: string) => invoke('delete_knowledge_base', { name }),
```

**Step 2: Create KnowledgeBases.svelte**

Top section: embedding config form (base_url, model, api_key, vector backend dropdown). Main section: knowledge bases table with add/delete. Status badges (configured/not configured).

**Step 3: Wire routing, test, commit**

```bash
git add desktop/src/lib/pages/KnowledgeBases.svelte desktop/src/lib/api.ts desktop/src/routes/+page.svelte
git commit -m "feat(desktop): add Knowledge Bases page with RAG configuration"
```

---

## Task 17: Communities — Backend Commands

**Files:**
- Modify: `desktop/src-tauri/src/commands.rs`
- Modify: `desktop/src-tauri/src/lib.rs`

**Step 1: Add community CRUD commands**

```rust
#[tauri::command]
pub async fn create_community(
    state: State<'_, SharedState>,
    name: String,
    platform: String,
    platform_group_id: String,
    zone: String,
    language: String,
    enabled_skills: Vec<String>,
) -> Result<serde_json::Value, String> {
    let st = state.read().await;
    if let Some(handle) = &st.engine_handle {
        let id = uuid::Uuid::new_v4().to_string();
        let skills_json = serde_json::to_string(&enabled_skills).unwrap_or_else(|_| "[]".into());
        sqlx::query(
            "INSERT INTO communities (id, name, platform, platform_group_id, zone, language, enabled_skills) VALUES (?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&id).bind(&name).bind(&platform).bind(&platform_group_id)
        .bind(&zone).bind(&language).bind(&skills_json)
        .execute(&handle.pool).await.map_err(|e| e.to_string())?;
        Ok(serde_json::json!({ "id": id }))
    } else {
        Err("Engine not running".into())
    }
}

#[tauri::command]
pub async fn update_community(
    state: State<'_, SharedState>,
    id: String,
    name: String,
    zone: String,
    language: String,
    enabled_skills: Vec<String>,
) -> Result<(), String> {
    let st = state.read().await;
    if let Some(handle) = &st.engine_handle {
        let skills_json = serde_json::to_string(&enabled_skills).unwrap_or_else(|_| "[]".into());
        sqlx::query(
            "UPDATE communities SET name = ?, zone = ?, language = ?, enabled_skills = ? WHERE id = ?"
        )
        .bind(&name).bind(&zone).bind(&language).bind(&skills_json).bind(&id)
        .execute(&handle.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    } else {
        Err("Engine not running".into())
    }
}

#[tauri::command]
pub async fn delete_community(
    state: State<'_, SharedState>,
    id: String,
) -> Result<(), String> {
    let st = state.read().await;
    if let Some(handle) = &st.engine_handle {
        sqlx::query("DELETE FROM communities WHERE id = ?")
            .bind(&id)
            .execute(&handle.pool).await.map_err(|e| e.to_string())?;
        Ok(())
    } else {
        Err("Engine not running".into())
    }
}
```

**Step 2: Add uuid dependency to desktop Cargo.toml if not present**

**Step 3: Register, build, commit**

```bash
git add desktop/src-tauri/src/commands.rs desktop/src-tauri/src/lib.rs desktop/src-tauri/Cargo.toml
git commit -m "feat(desktop): add community CRUD IPC commands"
```

---

## Task 18: Communities — Frontend Page Update

**Files:**
- Modify: `desktop/src/lib/pages/Communities.svelte`
- Modify: `desktop/src/lib/api.ts`

**Step 1: Add API functions**

```typescript
// Communities
createCommunity: (params: {
    name: string; platform: string; platformGroupId: string;
    zone: string; language: string; enabledSkills: string[];
}) => invoke('create_community', params),
updateCommunity: (params: {
    id: string; name: string; zone: string; language: string; enabledSkills: string[];
}) => invoke('update_community', params),
deleteCommunity: (id: string) => invoke('delete_community', { id }),
```

**Step 2: Update Communities.svelte**

Replace the read-only stub with full CRUD: table, "Add Community" form (name, platform dropdown, group ID, JAKIM zone dropdown with state grouping, language radio, skills multi-select), edit inline, delete with confirmation.

**Step 3: Commit**

```bash
git add desktop/src/lib/pages/Communities.svelte desktop/src/lib/api.ts
git commit -m "feat(desktop): upgrade Communities page to full CRUD"
```

---

## Task 19: Content — Backend Commands

**Files:**
- Modify: `desktop/src-tauri/src/commands.rs`
- Modify: `desktop/src-tauri/src/lib.rs`

**Step 1: Add content read-only commands**

```rust
#[tauri::command]
pub async fn get_doa_collection(
    category: Option<String>,
) -> Result<serde_json::Value, String> {
    let all = amanclaw_skill_doa::collection::get_all();
    let filtered: Vec<serde_json::Value> = all.iter()
        .filter(|d| {
            category.as_ref().map_or(true, |c| d.category == *c)
        })
        .map(|d| serde_json::json!({
            "category": d.category,
            "title_bm": d.title_bm,
            "title_en": d.title_en,
            "arabic": d.arabic,
            "transliteration": d.transliteration,
            "translation_bm": d.translation_bm,
            "translation_en": d.translation_en,
            "reference": d.reference,
        }))
        .collect();
    Ok(serde_json::json!({ "doas": filtered, "count": filtered.len() }))
}

#[tauri::command]
pub async fn search_doa(
    query: String,
) -> Result<serde_json::Value, String> {
    let results = amanclaw_skill_doa::collection::search(&query);
    let list: Vec<serde_json::Value> = results.iter().map(|d| {
        serde_json::json!({
            "category": d.category,
            "title_bm": d.title_bm,
            "title_en": d.title_en,
            "arabic": d.arabic,
            "transliteration": d.transliteration,
            "translation_bm": d.translation_bm,
            "translation_en": d.translation_en,
        })
    }).collect();
    Ok(serde_json::json!({ "doas": list, "count": list.len() }))
}
```

Note: The `get_zakat_rates` and `get_latest_khutbah` commands depend on Python plugin data that may not be directly accessible from Rust. For now, add stubs returning placeholder data. These can be wired up later when the Python SDK provides Rust-callable interfaces.

```rust
#[tauri::command]
pub async fn get_zakat_rates() -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({
        "fitrah": { "rate": 7.00, "currency": "MYR", "year": 2026 },
        "note": "Rates from JAKIM — update via skill-zakat Python plugin",
    }))
}

#[tauri::command]
pub async fn get_latest_khutbah() -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({
        "available": false,
        "note": "Khutbah data available via skill-khutbah Python plugin",
    }))
}
```

**Step 2: Add amanclaw-skill-doa dependency**

In `desktop/src-tauri/Cargo.toml`:
```toml
amanclaw-skill-doa = { path = "../../rust/plugins/skill-doa" }
```

**Step 3: Register, build, commit**

```bash
git add desktop/src-tauri/src/commands.rs desktop/src-tauri/src/lib.rs desktop/src-tauri/Cargo.toml
git commit -m "feat(desktop): add content viewer IPC commands for doa, zakat, khutbah"
```

---

## Task 20: Content — Frontend Page Update

**Files:**
- Modify: `desktop/src/lib/pages/Content.svelte`
- Modify: `desktop/src/lib/api.ts`

**Step 1: Add API functions**

```typescript
// Content
getDoaCollection: (category?: string) => invoke('get_doa_collection', { category }),
searchDoa: (query: string) => invoke('search_doa', { query }),
getZakatRates: () => invoke('get_zakat_rates'),
getLatestKhutbah: () => invoke('get_latest_khutbah'),
```

**Step 2: Update Content.svelte**

Replace empty tabs with functional viewers:
- **Doa tab**: Category filter dropdown (harian, pagi, petang, musafir, makan, tidur), search input, card list showing arabic text, transliteration, translations.
- **Zakat tab**: Display rates from get_zakat_rates. Static reference view.
- **Khutbah tab**: "Fetch latest" button, display cached text or "not available" state.

**Step 3: Commit**

```bash
git add desktop/src/lib/pages/Content.svelte desktop/src/lib/api.ts
git commit -m "feat(desktop): add functional content viewers for doa, zakat, khutbah"
```

---

## Task 21: Skills Page — Remove Marketplace Tab

**Files:**
- Modify: `desktop/src/lib/pages/Skills.svelte`

**Step 1: Remove marketplace tab**

Remove the "Marketplace" tab and all related state/functions (catalog, installTarget, etc.). Keep only the "Installed" tab showing registered skills with enable/disable toggles. The tab switcher UI can be removed entirely since there's only one view now.

**Step 2: Commit**

```bash
git add desktop/src/lib/pages/Skills.svelte
git commit -m "refactor(desktop): remove marketplace tab from Skills page (moved to dedicated page)"
```

---

## Task 22: Settings Page — Add Summary Sections

**Files:**
- Modify: `desktop/src/lib/pages/Settings.svelte`

**Step 1: Add summary sections**

After the existing settings sections, add new sections that display summary info with links:

- **Agent Routing**: "Default agent: {name}. {N} routing rules configured." Button to navigate to Agents page.
- **Cron**: "Timezone: {tz}. {N} jobs configured." Link to Cron Jobs.
- **Webhooks**: "Base path: /hooks. {N} endpoints." Link to Webhooks.
- **Gateway**: Enable/disable toggle + "Active, {N} connections" or "Disabled". Link to Gateway.
- **Sub-Agents**: Enable/disable toggle + "{N} running / {max} max". Link to Sub-Agents.
- **Registry**: Enable/disable toggle + "{N} skills installed". Link to Marketplace.
- **Embeddings**: "Configured ({model})" or "Not configured". Link to Knowledge Bases.

Each section loads data via existing API functions (no new commands needed). Uses `currentPage.set('agents')` for navigation.

**Step 2: Commit**

```bash
git add desktop/src/lib/pages/Settings.svelte
git commit -m "feat(desktop): add feature summary sections to Settings page"
```

---

## Task 23: Final Build and Integration Test

**Files:**
- All modified files

**Step 1: Full Rust build**

Run: `cd desktop/src-tauri && cargo build`
Expected: Compiles with no errors.

**Step 2: Full frontend build**

Run: `cd desktop && npm run build`
Expected: Builds successfully.

**Step 3: Run the app**

Run: `cd desktop && cargo tauri dev`
Expected: App launches. Verify each new page renders. Navigate through all sidebar items.

**Step 4: Commit any fixes**

If any build fixes needed, commit them.

**Step 5: Final commit**

```bash
git add -A
git commit -m "chore(desktop): final build verification for desktop parity"
```

---

## Implementation Order Summary

| Batch | Tasks | Description |
|-------|-------|-------------|
| 1 | 1-2 | Foundation (state, sidebar, routing stubs) |
| 2 | 3-4 | Agents (backend + frontend) |
| 3 | 5-6 | Cron Jobs (backend + frontend) |
| 4 | 7-8 | Webhooks (backend + frontend) |
| 5 | 9-10 | Gateway (backend + frontend) |
| 6 | 11-12 | Sub-Agents (backend + frontend) |
| 7 | 13-14 | Marketplace (backend + frontend) |
| 8 | 15-16 | Knowledge Bases (backend + frontend) |
| 9 | 17-18 | Communities CRUD (backend + frontend) |
| 10 | 19-20 | Content viewers (backend + frontend) |
| 11 | 21-22 | Cleanup (Skills tab removal, Settings summaries) |
| 12 | 23 | Final build and integration test |
