# AmanClaw: Surpass OpenClaw Strategy

**Date:** 2026-03-09
**Strategy:** Unfair Advantage + 10x Developer Experience
**Approach:** 60% foundation / 40% quick wins (Phase 1), then flip to 40/60

## Context

AmanClaw is a Rust-based modular AI assistant for Malaysian Muslim communities. OpenClaw is a TypeScript-based AI agent framework with 247K+ GitHub stars and 2,857+ marketplace skills.

Rather than competing head-on with OpenClaw's ecosystem size, AmanClaw will exploit structural advantages that OpenClaw cannot replicate (Rust performance, WASM security, Islamic domain, WhatsApp support, offline-first) while delivering a 10x better developer experience.

## Current Gaps (Project Maturity Assessment)

| Area | Maturity | Key Gap |
|------|----------|---------|
| Tests | 60% | 87 unit tests, no E2E, no CI automation |
| CI/CD | 20% | Only desktop release workflow |
| Documentation | 75% | Great README, missing CONTRIBUTING/CHANGELOG |
| Examples | 80% | Good plugins, missing deployment examples |
| Publishing | 0% | Not on crates.io |
| Docker | 90% | Well-hardened, missing .dockerignore/healthcheck |
| Benchmarks | 0% | None |
| Error Handling | 70% | anyhow used, no custom error types |
| Logging | 85% | Structured tracing, no observability integration |
| License | 90% | MIT, missing NOTICE |
| Changelog | 0% | None |
| Plugin SDK Docs | 90% | Comprehensive, missing versioning/debugging |

---

## Section 1: Foundation Layer

### 1.1 CI/CD Pipeline

Add GitHub Actions workflow (`.github/workflows/ci.yml`):

```yaml
on: [push, pull_request]
jobs:
  test:
    strategy:
      matrix:
        target: [x86_64-unknown-linux-gnu, aarch64-unknown-linux-gnu]
    steps:
      - cargo fmt --check
      - cargo clippy -- -D warnings
      - cargo test --workspace
      - cargo audit
```

- Build matrix: x86_64 + aarch64 (proves Raspberry Pi works)
- Docker image auto-publish to GitHub Container Registry on release tags
- Extend existing desktop release workflow

### 1.2 Test Coverage

- Integration tests: IncomingMessage → Pipeline → OutgoingMessage
- Plugin SDK tests: WASM plugin load → execute → result
- Channel adapter mock tests
- Goal: every PR must pass tests before merge

### 1.3 Contributor Infrastructure

New files:

- `CONTRIBUTING.md` — code style, PR process, how to add a skill
- `CHANGELOG.md` — semantic versioning from 0.1.0
- `SECURITY.md` — responsible disclosure policy
- `.github/ISSUE_TEMPLATE/bug_report.md`
- `.github/ISSUE_TEMPLATE/feature_request.md`
- `.github/ISSUE_TEMPLATE/new_skill_proposal.md`
- `.dockerignore`
- Healthcheck in docker-compose.yml

### 1.4 Publish Core SDK

Publish to crates.io:
- `amanclaw-traits` — core trait definitions
- `amanclaw-plugin-sdk` — plugin development SDK

Add metadata to workspace Cargo.toml:
- `repository` URL
- `homepage` URL
- `documentation` URL

External developers can `cargo add amanclaw-plugin-sdk` and build skills without cloning the repo.

---

## Section 2: 10x Developer Experience

### 2.1 One-Command Setup

```bash
# Install
curl -sSf https://amanclaw.dev/install.sh | sh

# Initialize project
amanclaw init          # interactive wizard → config.yaml + .env

# Start development (no API key needed)
amanclaw dev           # mock LLM mode

# Docker one-liner
docker run -e TELEGRAM_BOT_TOKEN=xxx ghcr.io/amanclaw/amanclaw
```

`amanclaw init` wizard prompts:
1. Which channels? (Telegram/Discord/WhatsApp/Slack)
2. Bot tokens for selected channels
3. LLM backend (OpenAI/Ollama/local)
4. Enable Islamic skills? (y/n)
5. Generates `config.yaml` + `.env`

`amanclaw dev` features:
- Built-in mock LLM that echoes tool calls (no API key needed)
- Prints messages to terminal as a fake chat interface
- Developers can test skills immediately

### 2.2 Skill Scaffolding

```bash
amanclaw skill new my-skill --lang rust
# → generates: plugins/skill-my-skill/
#   ├── Cargo.toml (with amanclaw-plugin-sdk dep)
#   ├── src/lib.rs (Skill trait impl template)
#   ├── tests/mod.rs (test harness)
#   └── amanclaw-skill.toml (manifest)

amanclaw skill new my-skill --lang python
# → generates: plugins/skill_my_skill.py (decorator template)

amanclaw skill test my-skill   # run in isolation with mock input
amanclaw skill package my-skill # build WASM + tarball for marketplace
```

### 2.3 Live Reload for Development

```bash
amanclaw dev --watch
```

- File watcher on `plugins/`, `souls/`, `config.yaml`
- Python/JS plugins: hot-reload on save
- SOUL.md changes: reload without restart
- WASM plugins: auto-rebuild + reload on source change
- Config changes: reload without restart

### 2.4 Interactive Playground

```bash
amanclaw playground  # opens http://localhost:3000
```

Local web UI features:
- Send test messages to any agent profile
- Full pipeline trace visualization (auth → context → LLM → tools → response)
- Memory inspector (history, facts, summaries)
- Skill toggle (enable/disable in real-time)
- SOUL.md live editor with preview

### 2.5 Error Messages That Teach

Every error includes: what went wrong, why, and how to fix it.

Before:
```
Error: connection refused
```

After:
```
✗ Cannot connect to LLM at localhost:8080

  Your LLM server is not running or the URL is wrong.

  Quick fixes:
    1. Start your LLM: ollama serve
    2. Use mock mode: amanclaw dev
    3. Check config: LLM_BASE_URL in .env

  Docs: https://amanclaw.dev/troubleshooting#llm-connection
```

Startup health diagnostics:
```
✓ Config loaded (config.yaml)
✓ Database ready (data/amanclaw.db)
✓ LLM connected (ollama:llama3)
✓ Skills loaded (7 built-in, 3 WASM, 2 Python)
✗ Telegram: TELEGRAM_BOT_TOKEN not set (skipped)
✓ WhatsApp: connected via WAHA
Ready! Listening on 2 channels with 12 skills.
```

---

## Section 3: Unfair Advantages

### 3.1 Raspberry Pi as First-Class Target

- Official ARM64 binary in every GitHub release (CI builds it)
- `amanclaw-pi` install script:
  ```bash
  curl -sSf https://amanclaw.dev/pi.sh | sh
  # Downloads ARM64 binary
  # Creates systemd service
  # Sets up SQLite + config
  # Enables auto-start on boot
  ```
- Published benchmarks in README:
  | Metric | Pi 4 (4GB) | Pi 5 (8GB) |
  |--------|-----------|-----------|
  | Startup time | <2s | <1s |
  | Memory usage | ~15MB idle | ~15MB idle |
  | Messages/sec | 50+ | 100+ |
  | SQLite + vector search | <10ms | <5ms |
- Marketing: "Run your community's AI assistant for $35 hardware + $0/month"
- OpenClaw needs Node.js + Postgres + Redis = impossible on Pi

### 3.2 WASM Plugin Security Model

Permission manifest (`amanclaw-skill.toml`):
```toml
[permissions]
network = ["api.sunnah.com", "api.quran.com"]
filesystem = false
max_memory = "32MB"
timeout = "10s"
```

On install:
```
Installing skill-hadith v1.2.0...

Permissions requested:
  ✓ Network: api.sunnah.com
  ✗ Filesystem: none
  ✗ Max memory: 32MB
  ✗ Timeout: 10s

Approve? [y/N]
```

Audit log: every plugin network call and resource usage recorded.

Security comparison:
| Feature | OpenClaw | AmanClaw |
|---------|----------|----------|
| Plugin isolation | None (full trust JS) | WASM sandbox (Wasmtime) |
| Network restrictions | None | Domain allowlist |
| Memory limits | None | Configurable per-plugin |
| Execution timeout | None | Enforced per-plugin |
| Filesystem access | Full | Denied by default |
| Permission approval | None | Interactive on install |

### 3.3 WhatsApp-Native Features

OpenClaw has zero WhatsApp support. AmanClaw has two adapters.

Build WhatsApp-specific capabilities:
- **Button/list messages** for skill selection (not just plain text)
- **Voice note handling**: receive voice → transcribe → process → reply
- **Group management**: auto-welcome with community setup wizard
- **Broadcast lists** for scheduled reminders (prayer times, Quran verse)
- **Media handling**: image → describe/OCR → process → reply
- **Status updates**: post to WhatsApp Status channel

Market size: WhatsApp is #1 messaging app in SEA, Middle East, Africa — 2B+ users globally. OpenClaw ignores this entirely.

### 3.4 Islamic AI Platform (Global)

Expand beyond Malaysia:

**Prayer time calculation methods:**
- JAKIM (Malaysia) — current
- MWL (Muslim World League)
- ISNA (Islamic Society of North America)
- Egyptian General Authority
- University of Islamic Sciences, Karachi
- Umm al-Qura University, Makkah

**Multi-language support:**
- Malay + English (current)
- Arabic, Urdu, Turkish, Indonesian, Bangla (top Muslim-majority languages)
- Language detection + auto-response in user's language

**Mazhab-aware responses:**
- SOUL.md variable: `{{mazhab}}` → Hanafi/Shafi'i/Maliki/Hanbali
- Fiqh skills adjust rulings based on community's mazhab setting
- Example: wudu steps differ slightly between mazhabs

**Pre-built SOUL.md personas:**
- `souls/ustaz.md` — Islamic knowledge expert
- `souls/halal-advisor.md` — food/product guidance
- `souls/quran-companion.md` — Quran study assistant
- `souls/community-admin.md` — group management bot

**Curated Islamic skill pack:**
```bash
amanclaw skill install-pack islamic
# Installs: solat, qiblat, hijri, doa, quran, hadith, halal, zakat, masjid, khutbah, jakim
```

No other AI framework offers this. Blue ocean.

### 3.5 Offline-First Architecture

- SQLite + sqlite-vec = works without internet (except LLM calls)
- Local LLM backends: Ollama, llama.cpp, vLLM — fully air-gapped
- Pre-downloaded data packs:
  ```bash
  amanclaw data install quran    # Full Quran with translations (~50MB)
  amanclaw data install hadith   # Bukhari + Muslim collections (~100MB)
  amanclaw data install halal    # JAKIM halal database (~20MB)
  ```
- Use case: mosque in rural area with intermittent internet, Pi running local Llama model
- OpenClaw's cloud-first architecture cannot do this

---

## Section 4: Ecosystem & Marketplace

### 4.1 GitHub-Powered Marketplace

No custom infrastructure needed (solo-dev friendly):

- Central index repo: `amanclaw/skill-index`
  ```json
  {
    "skills": [
      {
        "name": "skill-solat",
        "version": "1.0.0",
        "author": "amanclaw",
        "repo": "amanclaw/skill-solat",
        "tier": "official",
        "lang": "rust",
        "tags": ["islamic", "prayer", "malaysia"]
      }
    ]
  }
  ```
- Each skill is its own GitHub repo with `amanclaw-skill.toml` manifest
- CLI commands:
  ```bash
  amanclaw skill search "prayer"
  amanclaw skill install amanclaw/skill-solat
  amanclaw skill update --all
  ```
- Submission: PR to index repo → CI validates manifest + builds → auto-merge

### 4.2 Starter Skill Templates

GitHub template repos:
- `amanclaw/skill-template-rust` — click "Use this template" → ready
- `amanclaw/skill-template-python` — same for Python plugins

Each template includes:
- CI workflow (test + build + publish release)
- Test harness with mock SkillInput
- README with badges
- `amanclaw-skill.toml` manifest
- LICENSE (MIT default)

Goal: developer who's never seen AmanClaw can publish a skill in 30 minutes.

### 4.3 Skill Packs (Curated Bundles)

```bash
amanclaw skill install-pack islamic
amanclaw skill install-pack community-management
amanclaw skill install-pack productivity
amanclaw skill install-pack developer-tools
amanclaw skill install-pack education
```

Packs are just lists in the index repo — zero infrastructure:
```json
{
  "packs": {
    "islamic": ["skill-solat", "skill-quran", "skill-hadith", "skill-halal", ...],
    "community-management": ["skill-welcome", "skill-moderation", "skill-polls", ...],
    "productivity": ["skill-reminders", "skill-todo", "skill-calendar", ...]
  }
}
```

30 well-curated skills in 5 packs feels more complete than 2,857 random plugins.

### 4.4 Skill Quality Tiers

Three tiers:
- **community** — anyone can publish, no review
- **verified** — passes automated checks:
  - Tests exist and pass
  - Permissions declared in manifest
  - No unsafe code in WASM
  - Documentation present
- **official** — maintained by AmanClaw team, guaranteed compatibility

Badge in search results:
```
[official] skill-solat v1.0.0 — Malaysian prayer times by JAKIM zone
[verified] skill-weather v0.3.0 — Weather forecast (OpenWeatherMap)
[community] skill-joke v0.1.0 — Random jokes
```

### 4.5 OpenClaw Plugin Compatibility Bridge

```bash
amanclaw skill import-openclaw <npm-package>
```

- Wraps OpenClaw TypeScript skills as Python subprocess plugins
- Auto-generates `amanclaw-skill.toml` manifest from OpenClaw metadata
- Not all will work, but popular utility skills can be ported
- Experimental — marked as such, community can improve wrappers
- Neutralizes OpenClaw's biggest advantage

---

## Section 5: Commercial Path

### 5.1 AmanClaw Cloud — Managed Hosting

| Tier | Price | Agents | Channels | Messages/mo | Features |
|------|-------|--------|----------|-------------|----------|
| Free | $0 | 1 | 1 | 500 | Community skills |
| Pro | $9 | 5 | All | 10K | SOUL.md editor, cron jobs |
| Community | $29 | Unlimited | All | Unlimited | Custom domain, webhooks |
| Mosque | $49 | Unlimited | All | Unlimited | Multi-community, analytics, white-label |

No infrastructure for users — paste bot tokens and go.

### 5.2 One-Click Deploy Templates

Before Cloud is ready:
- DigitalOcean 1-Click App ($5/mo droplet)
- Railway template (free tier works)
- Fly.io config (global edge, free tier)
- Raspberry Pi image (download, flash, boot, done)

Each pre-configured with SQLite, systemd, auto-updates.
Users start self-hosted → graduate to Cloud.

### 5.3 Specialized Bot Products

Pre-packaged vertical bots (AmanClaw + specific SOUL.md + skill pack + config):

- **SolatBot** — prayer time reminders, zero config
- **UstazBot** — Islamic Q&A with Quran + Hadith RAG
- **MasjidBot** — mosque management (announcements, donations, RSVP)
- **HalalBot** — product scan → halal check → reply

Each deployable standalone. Sell as products or bundle with Cloud tier.

### 5.4 Analytics Dashboard (Cloud-Only)

- Per-community metrics: messages, active users, popular skills, peak hours
- Insights: "Your community asks about halal 3x more than solat — consider HalalBot"
- Retention: "User X hasn't interacted in 7 days — send check-in?"
- Exportable reports for mosque committees

### 5.5 Open Core Model

MIT open source forever:
- Core engine, all skills, all channels, CLI, SDK, marketplace

Cloud-only premium:
- Analytics dashboard
- Multi-community management console
- Auto-scaling + managed backups
- Uptime SLA
- Priority marketplace listing

Community stays happy. Revenue funds development.

---

## Section 6: Phased Execution Roadmap

### Phase 1: Foundation + Quick Wins (Weeks 1-4)

*"Make it trustworthy and easy to try"*

| Week | Foundation (60%) | Quick Wins (40%) |
|------|-----------------|------------------|
| 1 | CI/CD pipeline (test, lint, audit) | `amanclaw init` wizard |
| 2 | CONTRIBUTING.md, CHANGELOG.md, issue templates | `amanclaw dev` with mock LLM |
| 3 | Integration tests for message pipeline | ARM64 binary in releases |
| 4 | Publish `amanclaw-traits` + `amanclaw-plugin-sdk` to crates.io | Startup health diagnostics |

**Exit criteria:** PRs run tests. New developer can `cargo install amanclaw-cli && amanclaw init && amanclaw dev` in 5 minutes.

### Phase 2: DX + Differentiators (Weeks 5-10)

*"Make it delightful to build on"*

| Week | DX | Unfair Advantage |
|------|-----|-----------------|
| 5-6 | `amanclaw skill new` scaffolding + test | Benchmarks published (Pi 4) |
| 7-8 | `amanclaw playground` local web UI | WhatsApp buttons/voice notes |
| 9-10 | Live reload (`--watch`) | Global prayer time methods |

**Exit criteria:** Create, test, run a skill without reading docs. Published Pi benchmarks.

### Phase 3: Ecosystem Launch (Weeks 11-16)

*"Make it grow beyond you"*

| Week | Ecosystem | Content |
|------|-----------|---------|
| 11-12 | `skill-index` repo + search/install CLI | Islamic skill pack |
| 13-14 | GitHub template repos (Rust + Python) | SOUL.md personas |
| 15-16 | Skill quality tiers | OpenClaw compat bridge (experimental) |

**Exit criteria:** External devs can discover, install, publish skills. 3-5 packs available.

### Phase 4: Specialized Products (Weeks 17-22)

*"Make it useful for non-developers"*

| Week | Product | Distribution |
|------|---------|-------------|
| 17-18 | SolatBot standalone | DigitalOcean 1-Click + Pi image |
| 19-20 | UstazBot with RAG | Railway + Fly.io templates |
| 21-22 | MasjidBot | Landing pages |

**Exit criteria:** Mosque admin with zero tech knowledge deploys SolatBot to WhatsApp.

### Phase 5: AmanClaw Cloud (Weeks 23-30)

*"Make it a business"*

| Week | Cloud | Business |
|------|-------|----------|
| 23-25 | Multi-tenant hosting | Free tier launch |
| 26-28 | Analytics dashboard | Pro + Community tiers |
| 29-30 | Auto-scaling + backups | Mosque tier |

**Exit criteria:** Paying customers. Revenue funds full-time development.

---

## Success Metrics

| Milestone | Metric | Target |
|-----------|--------|--------|
| Phase 1 complete | GitHub stars | 500+ |
| Phase 2 complete | Skills created by others | 10+ |
| Phase 3 complete | Community contributors | 20+ |
| Phase 4 complete | Active bot deployments | 100+ |
| Phase 5 complete | Monthly recurring revenue | $1,000+ |

---

## Why This Wins Against OpenClaw

1. **Performance moat** — Rust on Pi vs Node.js needing cloud infra
2. **Security moat** — WASM sandbox vs full-trust JS plugins
3. **Domain moat** — 11 Islamic skills vs zero, with global expansion
4. **Platform moat** — WhatsApp (2B users) vs Discord/Slack only
5. **Cost moat** — $35 Pi vs $50-200/mo cloud hosting
6. **DX parity** — one-command setup, scaffolding, playground matches or beats OpenClaw
7. **Ecosystem bridge** — OpenClaw plugin compat neutralizes their biggest advantage
8. **Commercial path** — open core with Cloud monetization funds sustainable growth

AmanClaw doesn't need to be OpenClaw. It needs to be the obvious choice for the use cases OpenClaw can't serve.
