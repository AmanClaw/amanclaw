# Phase 4: Specialized Products — CommunityBot

## Goal

Enable non-developers to deploy a pre-configured AmanClaw community bot to any PaaS in under 5 minutes. CommunityBot is the first "product" — a distribution of AmanClaw with a specific persona, config, and deploy templates.

## Architecture

No new Rust code for the product itself. CommunityBot = same `amanclaw` binary + pre-configured files. New Rust code only for the `amanclaw product` CLI command that scaffolds product directories.

```
products/communitybot/
├── config.yaml              # Pre-configured for community use
├── .env.example             # Only secrets a non-dev needs
├── souls/community.md       # Bot persona
├── Dockerfile               # Slim product Dockerfile
├── docker-compose.yml       # Simplified compose
├── fly.toml                 # Fly.io 1-click deploy
├── railway.json             # Railway 1-click deploy
├── render.yaml              # Render blueprint
├── README.md                # Non-developer friendly guide
└── index.html               # Landing page with deploy buttons
```

## Components

### 1. Product Directory (CommunityBot)

Pre-configured `config.yaml` with:
- Ollama as default LLM (free, local)
- All built-in skills enabled
- Learning engine on
- Bilingual BM + English

Simplified `.env.example` with only 3 required vars:
- `TELEGRAM_BOT_TOKEN` (or other channel token)
- `LLM_API_KEY` (optional if using Ollama)
- `LLM_BASE_URL` (defaults to Ollama localhost)

### 2. Deploy Templates

Three PaaS targets:
- **Fly.io** — `fly launch`, free tier, global edge
- **Railway** — GitHub connect, auto-deploy, free tier
- **Render** — Blueprint spec, free tier with auto-sleep

All use `ghcr.io/amanclaw/amanclaw:latest` Docker image.

### 3. Landing Page

Self-contained `index.html` — dark theme, no build tools. Shows:
- What CommunityBot does (3 features)
- Deploy buttons (Fly / Railway / Render / Docker)
- Quick setup steps
- "Powered by AmanClaw" footer

### 4. CLI Product Command

```bash
amanclaw product new communitybot     # scaffolds products/communitybot/
amanclaw product list                 # lists available product templates
```

Product templates are embedded in the binary. The scaffold command generates all files for a product distribution.

## Exit Criteria

A non-developer can deploy CommunityBot to Fly.io by:
1. `git clone` the repo
2. `cd products/communitybot`
3. Set `TELEGRAM_BOT_TOKEN` in `.env`
4. `fly launch`
