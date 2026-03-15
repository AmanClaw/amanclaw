# Phase 3: AmanClaw Cloud MVP — Design Spec

**Date:** 2026-03-15
**Scope:** Managed hosting beta with multi-tenant architecture, web chat widget, K3s deployment on Hostinger Malaysia
**Approach:** Shared binary with per-tenant SQLite isolation, invite-only beta, no billing
**Part of:** Phase 3 (Cloud & Community) from the [sovereign AI agent design](2026-03-14-sovereign-islamic-ai-agent-design.md)

---

## Overview

AmanClaw Cloud is a managed hosting service where users sign up, get their own AmanClaw instance, connect chat channels, and start using it — without touching a terminal. The MVP is an invite-only beta deployed on K3s (Hostinger Malaysia VPS) with a shared-process multi-tenant architecture.

**What it delivers:**
- Sign up with invite code → get a managed AmanClaw instance
- Connect Telegram/WhatsApp/Discord via dashboard
- Chat via web widget (no chat app needed)
- All 31 skills available (Islamic knowledge, general tools, finance)
- Data stays in Malaysia (Hostinger Malaysia region)

---

## Multi-Tenant Architecture

### Tenant Model

Each tenant gets an isolated directory with its own data:

```
cloud/
├── tenants/
│   ├── tenant-abc123/
│   │   ├── config.yaml       # Tenant-specific config (LLM, channels, skills)
│   │   ├── memory.db         # Conversation memory (SQLite)
│   │   ├── islamic.db        # Islamic knowledge (SQLite)
│   │   ├── plugins/          # Tenant plugins
│   │   └── souls/            # Tenant personas
│   ├── tenant-def456/
│   │   └── ...
│   └── ...
├── cloud.db                   # Cloud management database
└── cloud.yaml                 # Cloud server config
```

### Cloud Database

Separate SQLite database for cloud management (not tenant data):

```sql
tenants (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  slug TEXT UNIQUE NOT NULL,
  owner_email TEXT NOT NULL,
  status TEXT DEFAULT 'active',   -- active, suspended, deleted
  plan TEXT DEFAULT 'beta',       -- beta, free, personal, community
  created_at TEXT,
  last_active TEXT
)

invites (
  code TEXT PRIMARY KEY,
  email TEXT,
  used_by TEXT,
  created_at TEXT,
  expires_at TEXT
)

cloud_users (
  id TEXT PRIMARY KEY,
  email TEXT UNIQUE NOT NULL,
  password_hash TEXT NOT NULL,
  tenant_id TEXT REFERENCES tenants(id),
  role TEXT DEFAULT 'owner',      -- owner, admin, member
  created_at TEXT
)
```

### Engine Lifecycle

- **Lazy start:** Tenant engine starts on first request, not on sign-up
- **Idle shutdown:** Background task stops engines inactive for 30+ minutes
- **State tracking:** `HashMap<String, TenantState>` maps slug → engine handle

```rust
struct TenantState {
    tenant: Tenant,
    engine: Option<EngineHandle>,
    last_active: Instant,
}
```

---

## Cloud Server

A new binary `amanclaw-cloud` wraps the existing engine with multi-tenant routing.

### URL Routing

```
cloud.amanclaw.my
├── /                          → Static landing/sign-up page
├── /api/cloud/*               → Cloud management API
├── /t/{slug}/api/*             → Tenant API (proxied to tenant engine)
├── /t/{slug}/admin/*           → Tenant dashboard (existing Svelte app)
├── /t/{slug}/ws                → Tenant WebSocket
├── /t/{slug}/chat              → Web chat widget
├── /t/{slug}/hooks/*           → Tenant webhooks
└── /t/{slug}/mcp/*             → Tenant MCP endpoints
```

### Cloud Management API

```
POST /api/cloud/signup            → Create account (requires invite code)
POST /api/cloud/login             → Get cloud JWT
GET  /api/cloud/tenant            → Get my tenant info
PUT  /api/cloud/tenant            → Update tenant settings
GET  /api/cloud/tenant/status     → Engine status (running/stopped/error)
POST /api/cloud/tenant/start      → Force-start engine
POST /api/cloud/tenant/stop       → Force-stop engine
```

### Tenant Request Flow

1. Request arrives at `/t/{slug}/api/skills`
2. Cloud router extracts `slug` from path
3. Looks up `TenantState` by slug
4. If engine is None → start engine (load tenant config, create Engine instance)
5. Proxy request to tenant engine API
6. Update `last_active` timestamp

### Invite-Only Beta

- No billing — all beta users get free access
- Invite codes generated via CLI: `amanclaw-cloud invite create --email user@example.com`
- Sign-up requires valid, unexpired invite code
- Beta plan limits: 1 channel, 500 msgs/day, all skills

---

## Web Chat Widget

Lightweight browser-based chat at `/t/{slug}/chat`.

### Technical Implementation

Connects to tenant WebSocket gateway (already exists):

```
Browser → WebSocket /t/{slug}/ws → Cloud Router → Tenant Engine → Pipeline → Response
```

JSON-RPC protocol (already supported):
```json
{"jsonrpc": "2.0", "method": "chat", "params": {"text": "What time is Maghrib?"}, "id": 1}
```

### UI Scope (MVP)

- Single-user chat (tenant owner, authenticated via cloud JWT)
- Text messages with markdown rendering
- Typing indicator while waiting for response
- Auto-reconnect on disconnect
- Mobile-responsive, dark/light mode
- ~200 lines of Svelte

---

## Kubernetes Deployment

### Infrastructure

```
Hostinger VPS (Malaysia Region, 4GB+ RAM)
├── K3s (lightweight Kubernetes)
│   ├── Namespace: amanclaw-cloud
│   │   ├── Deployment: cloud-server (2 replicas)
│   │   ├── Service: cloud-svc (ClusterIP)
│   │   ├── Ingress: cloud.amanclaw.my (TLS via cert-manager)
│   │   ├── PVC: tenant-data (shared storage)
│   │   └── CronJob: backup (daily SQLite → S3-compatible)
│   └── Namespace: monitoring
│       ├── Prometheus (scrapes /metrics)
│       └── Grafana (dashboards)
├── Traefik (ingress controller, bundled with K3s)
└── cert-manager (Let's Encrypt TLS)
```

### Why K3s

- Lightweight certified Kubernetes — <512MB overhead
- Single binary, runs on any VPS
- Includes Traefik ingress out of the box
- Scales to multi-node by adding more VPS

### Deployment Files

```
deploy/
├── k3s/
│   ├── namespace.yaml
│   ├── deployment.yaml
│   ├── service.yaml
│   ├── ingress.yaml
│   ├── pvc.yaml
│   ├── configmap.yaml
│   ├── secret.yaml
│   ├── backup-cronjob.yaml
│   └── monitoring/
│       ├── prometheus.yaml
│       └── grafana.yaml
├── Dockerfile.cloud
└── scripts/
    ├── setup-k3s.sh
    ├── deploy.sh
    └── backup.sh
```

### Resource Estimates

| Tenants | RAM | CPU | Storage |
|---------|-----|-----|---------|
| 10 (beta) | ~1GB | 1 core | ~5GB |
| 50 | ~3GB | 2 cores | ~20GB |
| 100 | ~6GB | 4 cores | ~40GB |

With lazy engine loading (idle shutdown after 30 min), active engines << total tenants.

### Setup Flow (Operator)

```bash
# 1. Provision Hostinger VPS (Malaysia region, 4GB+ RAM)
# 2. Install K3s
ssh root@your-vps 'curl -sfL https://get.k3s.io | sh -'

# 3. Deploy AmanClaw Cloud
./deploy/scripts/deploy.sh

# 4. Point DNS: cloud.amanclaw.my → VPS IP

# 5. Generate invite codes
amanclaw-cloud invite create --email user@example.com
```

---

## Cloud CLI

The `amanclaw-cloud` binary commands:

```bash
# Server
amanclaw-cloud serve                          # Start cloud server
amanclaw-cloud serve --port 8443 --host 0.0.0.0

# Invites
amanclaw-cloud invite create --email user@example.com
amanclaw-cloud invite list
amanclaw-cloud invite revoke CODE

# Tenant management
amanclaw-cloud tenant list
amanclaw-cloud tenant info SLUG
amanclaw-cloud tenant suspend SLUG
amanclaw-cloud tenant delete SLUG

# Backup
amanclaw-cloud backup --output /backups/
amanclaw-cloud backup --s3 s3://bucket/backups/
```

---

## Sub-Projects

| Sub-project | Scope | Depends on |
|-------------|-------|------------|
| **3A: Cloud Crate + Tenant Management** | amanclaw-cloud crate, tenant model, cloud DB, tenant router, cloud management API, invite system | — |
| **3B: Web Chat Widget** | Svelte chat component, WebSocket integration | 3A |
| **3C: K8s Deployment** | K3s setup, Dockerfile.cloud, K8s manifests, backup, monitoring | 3A |
| **3D: Cloud CLI** | serve, invite, tenant, backup commands | 3A |

Execute: 3A first, then 3B + 3C + 3D in parallel.

---

## What This Delivers

```bash
# Operator
./deploy/scripts/setup-k3s.sh
./deploy/scripts/deploy.sh
amanclaw-cloud invite create --email user@example.com
# → invite code ABC123

# User
# 1. Goes to cloud.amanclaw.my
# 2. Signs up with invite code
# 3. Gets dashboard at cloud.amanclaw.my/t/my-bot/admin/
# 4. Connects Telegram bot token
# 5. Chats via cloud.amanclaw.my/t/my-bot/chat
# 6. Bot responds with all 31 skills, data in Malaysia
```

No terminal. No Docker. No config files. Just sign up and go.
