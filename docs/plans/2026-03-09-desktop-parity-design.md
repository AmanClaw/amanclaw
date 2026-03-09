# Desktop App Parity — Full Feature Design

**Date:** 2026-03-09
**Status:** Approved
**Scope:** Bridge all gaps between desktop app (Tauri 2 + Svelte 5) and core engine features

---

## Overview

The desktop app currently covers basic lifecycle, skills, users, and MCP servers. This design adds UI for all 7 new core engine features plus finishes 2 existing stubs, totaling 9 new/updated pages and ~46 new IPC commands.

**Approach:** Local-first. All UIs built against direct Rust access via `engine_handle`. Remote mode support deferred to a later phase. Clean abstractions (commands return serializable types) make remote mode a drop-in later.

---

## Navigation (Updated Sidebar)

Dashboard → Communities → Agents → Skills → Marketplace → Cron Jobs → Webhooks → Gateway → Sub-Agents → Knowledge Bases → Content → Users → MCP Servers → Logs → Settings

---

## Page 1: Agents (SOUL.md)

**Purpose:** Manage agent profiles and SOUL.md personality files.

**Layout:** Split view — agent list left, editor right.

**Agent List Panel:**
- Cards: agent ID, name, soul_file, allowed skills count, memory namespace
- "Add Agent" button
- Default agent always present (non-deletable)
- Routing rule count badge per agent

**Agent Editor Panel:**
- Profile fields: id, name, memory_namespace, allowed_skills (multi-select)
- SOUL.md editor: code textarea with markdown preview toggle
  - soul_file set → loads from soul_dir
  - No soul_file → inline system_prompt textarea
  - "Save as SOUL.md" to extract to file
- Frontmatter helper: visual fields for version, language, tags, extends, variables → auto-generates YAML
- Preview: rendered prompt after inheritance resolution + variable interpolation

**Routing Rules Section (bottom):**
- Table: match criteria (platform, topic_id, channel_id, group_id) → agent
- Add/edit/delete rules
- Default agent fallback indicator

**Backend commands:**
- `list_agents() → Vec<AgentProfile>`
- `save_agent(profile)`
- `delete_agent(id)`
- `load_soul_file(filename) → String`
- `save_soul_file(filename, content)`
- `preview_soul(filename) → ResolvedSoul`
- `get_routing_rules() → Vec<RoutingRule>`
- `save_routing_rules(rules)`

---

## Page 2: Cron Jobs

**Purpose:** Create, edit, and monitor scheduled jobs.

**Layout:** Two tabs — "Jobs" and "History".

**Jobs Tab:**
- Table: name, schedule (human-readable + cron expression), type, targets, enabled toggle, last run status
- "Add Job" form:
  - Name, schedule (with presets: every hour, daily 6am, weekly Friday, custom), timezone override
  - Type radio: direct_message, skill_invocation, agent_prompt
  - Type-specific fields:
    - direct_message: template textarea
    - skill_invocation: skill dropdown + input JSON editor
    - agent_prompt: agent dropdown + prompt textarea
  - Targets: list of platform + chat_id + optional topic_id
  - Agent override dropdown, enabled toggle
- "Next run" column with calculated fire time
- Inline edit and delete

**History Tab:**
- Table from `cron_history`: job name, status, output preview, duration_ms, executed_at
- Filter by job, status, date range
- Auto-refresh 10 seconds
- Expand row for full output

**Backend commands:**
- `list_cron_jobs() → HashMap<String, CronJobConfig>`
- `save_cron_job(id, job)`
- `delete_cron_job(id)`
- `get_cron_history(filters) → Vec<CronHistoryEntry>`

---

## Page 3: Webhooks

**Purpose:** Configure inbound webhook endpoints with auth, transforms, and history.

**Layout:** Two tabs — "Endpoints" and "History".

**Endpoints Tab:**
- Table: name, path (/hooks/{id}), auth type badge, transform type, targets count, enabled toggle
- "Add Webhook" form:
  - Name, path (auto-generated or custom)
  - Auth type dropdown:
    - none: no extra fields
    - hmac_sha256: secret input
    - bearer: token input
    - header_match: header name + value
  - Transform type dropdown:
    - raw_json: no extra fields
    - json_path: message_path + title_path inputs
    - template: Handlebars textarea
    - agent_prompt: prompt_template + agent dropdown
    - skill_invocation: skill dropdown + input_template
  - Targets: platform + chat_id rows
  - Rate limit (optional), enabled toggle
- Copyable full URL per endpoint
- Inline edit and delete

**History Tab:**
- Table from `webhook_history`: name, status, source_ip, payload preview, error, duration_ms, received_at
- Filter by endpoint, status, date range
- Expand for full payload

**Backend commands:**
- `list_webhooks() → HashMap<String, WebhookEndpointConfig>`
- `save_webhook(id, endpoint)`
- `delete_webhook(id)`
- `get_webhook_history(filters) → Vec<WebhookHistoryEntry>`

---

## Page 4: Gateway

**Purpose:** WebSocket gateway configuration + live event stream.

**Layout:** Two panels — "Configuration" (top) and "Live Events" (bottom, expandable).

**Configuration Panel:**
- Form: enabled toggle, heartbeat interval, max connections, stale session timeout
- Save with restart reminder when toggling enabled
- Status: active/inactive, current connection count

**Live Events Panel:**
- Connects to `ws://127.0.0.1:{API_PORT}/ws` when enabled
- Authenticates via JSON-RPC `gateway.auth`
- Subscribes to `*` by default, topic filter input (e.g. `message.*`, `security.*`)
- Event stream (similar to Logs):
  - Scrolling list, newest bottom, max 200 entries
  - Each: timestamp, topic, collapsed JSON data
  - Color-coded: message.* = blue, security.* = red, engine.* = green
  - Click to expand full JSON
- Pause/resume and clear buttons

**Backend commands:**
- `get_gateway_config() → GatewayConfig`
- `save_gateway_config(config)`
- `get_gateway_status() → { enabled, connection_count }`
- Live events: Svelte connects directly via WebSocket (no IPC command)

---

## Page 5: Sub-Agents

**Purpose:** Monitor active sub-agents, cancel tasks.

**Layout:** Collapsible config panel (top) + active sub-agents table (main).

**Configuration Panel:**
- Form: enabled toggle, max_per_session, max_global, max_depth, default_timeout_secs
- Save button

**Active Sub-Agents Panel:**
- Table (5s auto-refresh): ID (truncated), agent profile, prompt (truncated), parent session, depth, status badge
- Status badges: Running (blue pulse), Completed (green), Failed (red), Cancelled (gray)
- Actions: Cancel (running only), expand for full prompt/result
- Filters: status dropdown, session ID search
- Summary bar: "3 running / 12 completed / 1 failed — 3 of 20 global slots used"
- "Cancel All" per session (with confirmation)

**Backend commands:**
- `get_subagent_config() → SubAgentConfig`
- `save_subagent_config(config)`
- `list_subagents(session_filter?) → Vec<SubAgent>`
- `cancel_subagent(id) → bool`
- `cancel_all_subagents(session) → usize`

---

## Page 6: Marketplace

**Purpose:** Unified skill discovery and installation via amanclaw-registry. Replaces hardcoded MCP marketplace tab in Skills.svelte.

**Layout:** Three tabs — "Browse", "Installed", "Publish" (future stub).

**Browse Tab:**
- Search bar with real-time filtering
- Skill card grid (from remote index) or "No remote registry configured" empty state
- Cards: name, version, description, tags, author
- "Install" button → download, verify checksum, install
- Refresh index button, tag filter chips

**Installed Tab:**
- Table from SkillRegistry: name, version, type, description, installed_at
- Actions: Uninstall (confirmation), Open directory
- Search within installed
- "Install from folder" → file picker for local amanclaw-skill.toml package

**Publish Tab (stub):**
- Info text about future publishing
- amanclaw-skill.toml manifest format docs
- Example manifest preview

**Backend commands:**
- `registry_list_installed() → Vec<InstalledSkill>`
- `registry_install_from_path(path) → InstalledSkill`
- `registry_uninstall(name) → bool`
- `registry_search_installed(query) → Vec<InstalledSkill>`
- `registry_refresh_remote() → usize`
- `registry_search_remote(query) → Vec<RemoteSkillEntry>`
- `registry_install_remote(name)`

---

## Page 7: Knowledge Bases

**Purpose:** Configure RAG — embedding model, vector store, knowledge base files.

**Layout:** "Embedding Configuration" (top) + "Knowledge Bases" table (main).

**Embedding Configuration:**
- Form: base_url, model, api_key (optional), vector backend dropdown (sqlite-vec / qdrant), qdrant_url (conditional)
- Save button, status indicator

**Knowledge Bases:**
- Table: name, collection, source path, document count, indexed status badge
- "Add KB" form: name, collection (auto-suggested), source file picker
- Actions: Re-index (with progress), Delete (confirmation + drop collection), View (doc count, samples)
- Status badges: Indexed (green), Not indexed (yellow), Error (red)

**Backend commands:**
- `get_embedding_config() → Option<EmbeddingConfig>`
- `save_embedding_config(config)`
- `get_vector_config() → Option<VectorConfig>`
- `save_vector_config(config)`
- `list_knowledge_bases() → HashMap<String, KnowledgeBaseConfig>`
- `save_knowledge_base(name, config)`
- `delete_knowledge_base(name)`
- `reindex_knowledge_base(name) → Result<usize>`

---

## Page 8: Communities (Updated)

**Purpose:** Full CRUD for community management (currently read-only stub).

**Changes:**
- Table: name, platform, zone, language, enabled skills count, created_at
- "Add Community" form: name, platform dropdown, platform_group_id, JAKIM zone dropdown (state-grouped), language radio (BM/English/Rojak), enabled skills multi-select
- Edit inline, delete with confirmation
- Notification settings: expandable toggles per community (solat, daily doa, khutbah)

**Backend commands:**
- `list_communities() → Vec<Community>` (upgrade existing)
- `create_community(data) → Community`
- `update_community(id, data)`
- `delete_community(id)`
- `update_community_notifications(id, notifications)`

---

## Page 9: Content (Updated)

**Purpose:** Read-only viewers for Islamic content data (currently empty tabs).

**Tabs:**
- **Doa** — Browse by category (harian, pagi, petang, musafir, makan, tidur), search by keyword. Data from skill-doa collection.
- **Zakat** — JAKIM rates display, calculator preview. Read-only reference.
- **Khutbah** — "Fetch latest" button, cached display. Read-only.

**Backend commands:**
- `get_doa_collection() → Vec<Doa>`
- `search_doa(query) → Vec<Doa>`
- `get_zakat_rates() → ZakatRates`
- `get_latest_khutbah() → Option<Khutbah>`

---

## Settings Page Updates

New summary sections appended to existing Settings page:

- **Agent Routing** — Default agent dropdown, "N routing rules" link to Agents
- **Cron** — Global timezone dropdown, "N jobs configured" link
- **Webhooks** — Base path display, default secret, "N endpoints" link
- **Gateway** — Quick enable/disable toggle, connection count
- **Sub-Agents** — Quick enable/disable, running/max summary
- **Registry** — Enable/disable, skills_dir, remote_url, "N installed" link
- **Embeddings** — Status text, link to Knowledge Bases

Summary views with links — no duplicate configuration forms. Zero new commands (reuses others).

---

## Skills.svelte Changes

- Remove "Marketplace" tab (replaced by dedicated Marketplace page)
- Keep "Installed" tab only — registered skills with enable/disable toggles

---

## Totals

| Metric | Count |
|--------|-------|
| New pages | 7 (Agents, Cron, Webhooks, Gateway, Sub-Agents, Marketplace, Knowledge Bases) |
| Updated pages | 2 (Communities, Content) |
| Updated pages (minor) | 2 (Skills, Settings) |
| New IPC commands | ~46 |
| WebSocket connections | 1 (Gateway live events) |
| New Tauri backend code | commands.rs extensions + new helper modules |
| New Svelte components | 9 page components + shared form components |

---

## Implementation Order (Suggested)

Batch by dependency and value:

1. **Foundation** — Shared form components, sidebar update, config save helpers
2. **Communities + Content** — Finish existing stubs (lowest risk, familiar patterns)
3. **Agents** — SOUL.md editor + routing rules (high value, used by cron/webhooks)
4. **Cron Jobs + Webhooks** — Similar patterns, share target/form components
5. **Gateway** — Config + WebSocket live events
6. **Sub-Agents** — Monitoring table
7. **Marketplace** — Registry integration
8. **Knowledge Bases** — RAG config + indexing
9. **Settings** — Summary sections (depends on all other pages existing)
