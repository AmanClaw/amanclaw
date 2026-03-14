# AmanClaw: Sovereign Islamic AI Personal Agent

**Date:** 2026-03-14
**Approach:** Sovereign General Agent + Islamic Core (Open Source First)
**Vision:** The world's first Islamic AI personal agent with sovereign infrastructure
**Supersedes:** All prior phase plans (2026-03-08 Islamic Community Platform, 2026-03-09 Surpass OpenClaw). This is the unified strategic roadmap going forward.

---

## Positioning & Brand

**Tagline:** *"Your AI, your rules. Built on principles you trust."*

AmanClaw is a personal AI agent that runs anywhere — your laptop, Raspberry Pi, or your own cloud. It handles everyday tasks: coding, research, automation, writing, data analysis — plus world-class Islamic AI capabilities that no one else offers.

"Sovereign" means three things:
1. **Data sovereignty** — All data is stored locally (SQLite) by default. In cloud mode, users choose the region. Data never moves without explicit consent.
2. **Model sovereignty** — Use any model you want (local Ollama, self-hosted vLLM, or cloud APIs). AmanClaw never forces a specific provider. When using "bring your own API key" in cloud mode, users accept that data flows to their chosen provider — this is clearly disclosed.
3. **Infrastructure sovereignty** — The full stack is open source and self-hostable. No feature requires AmanClaw Cloud. Cloud sells convenience, not capability.

**Launch narrative:**
> "AmanClaw is a personal AI agent built for people who care about sovereignty and values. One binary. Runs offline. 100+ skills from coding to Islamic finance. Your data never leaves your control."

No comparisons. No "better than X." Just: here's what we built, and here's why it matters.

---

## Architecture

Four layers, building on the existing foundation:

```
┌─────────────────────────────────────────────────┐
│  Layer 4: SOVEREIGN CLOUD (AmanClaw Cloud)      │
│  Managed hosting · Model registry · Marketplace │
│  OIC data residency · Usage analytics           │
├─────────────────────────────────────────────────┤
│  Layer 3: ISLAMIC SOVEREIGN CORE                │
│  Islamic model fine-tuning · Fatwa engine        │
│  Shariah finance · Halal supply chain · Waqf     │
│  Quran tafsir · Scholar consensus engine         │
├─────────────────────────────────────────────────┤
│  Layer 2: GENERAL AGENT PLATFORM                │
│  MCP protocol · Skill marketplace · CLI agent    │
│  Multi-agent orchestration · Memory/RAG          │
│  Webhooks · Scheduler · API gateway              │
├─────────────────────────────────────────────────┤
│  Layer 1: FOUNDATION (mostly done ✓)            │
│  Rust engine · WASM plugins · 5 chat channels    │
│  SQLite · Plugin SDK · Desktop app · Dashboard   │
└─────────────────────────────────────────────────┘
```

**Key architectural decisions:**
- MCP (Model Context Protocol) support for thousands of integrations without building them
- Skill marketplace with community contributions, quality tiers, and review process
- Multi-agent orchestration for complex task delegation
- Offline-first remains core — cloud is optional, never required

---

## Layer 2: General Agent Platform

### MCP Protocol Support (High Priority)

Expand existing `amanclaw-mcp` crate to full MCP client + server:
- **Client:** Connect to external MCP servers (GitHub, Slack, databases, file systems, browsers)
- **Server:** Expose AmanClaw as an MCP server so other tools can use it

### General Skills (100+ target)

| Category | Skills | Priority |
|----------|--------|----------|
| **Developer** | Code generation, git ops, PR review, debugging, test writing, documentation | Phase 1 |
| **Research** | Web search, summarization, fact-checking, PDF analysis, data extraction | Phase 1 |
| **Productivity** | Task management, calendar, email drafts, meeting notes, reminders | Phase 1 |
| **Data** | CSV/JSON analysis, visualization, SQL queries, spreadsheet ops | Phase 2 |
| **Media** | Image generation (via local Stable Diffusion), OCR, audio transcription | Phase 2 |
| **System** | File management, process monitoring, network diagnostics, backups | Phase 2 |
| **Finance** | Expense tracking, invoicing, budgeting, crypto monitoring | Phase 3 |
| **Communication** | Email sending, social media posting, translation | Phase 3 |

Most implemented as Python/JS plugins via existing script runtime.

### CLI Agent Mode

```bash
amanclaw ask "refactor this function to use async"
amanclaw agent --task "set up CI for this repo"
amanclaw chat  # interactive REPL
```

Opens AmanClaw to the developer audience beyond chat apps.

### Multi-Agent Orchestration

Agents delegate subtasks through the existing pipeline:
```
User: "Research halal restaurants in KL and create a comparison spreadsheet"
  → Research Agent: searches web, collects data
  → Data Agent: structures into spreadsheet
  → Coordinator: combines and returns result
```

Each agent is a pipeline instance with its own context.

### Skill Marketplace

Evolve existing plugin system:
- `amanclaw install skill-name` from CLI
- Web-based catalog (extending existing dashboard MCP Servers page)
- Quality tiers: Community → Verified → Official
- Ratings, download counts, compatibility tags

---

## Layer 3: Islamic Sovereign Core

### Islamic Knowledge Engine

- **Quran Engine** — Full text + tafsir (Ibn Kathir, Al-Jalalayn, Al-Tabari) + cross-references + thematic search. User asks a life question, engine finds relevant ayat with scholarly context
- **Hadith Engine** — Kutub al-Sittah (6 major books) with chain of narration (isnad) grading, topic classification, and cross-referencing between collections. Data sources: Sunnah.com API (open, well-structured), HadithAPI.com, and local SQLite cache for offline use. Isnad grading uses existing scholarly classifications (sahih/hasan/da'if) from these sources — AmanClaw does not independently grade narrators
- **Fiqh Resolver** — Answers from multiple madhhab (Shafi'i, Hanafi, Maliki, Hanbali) with sources. Never single-opinion answers on khilafiyyah matters
- **Fatwa Aggregator** — Index fatwas from recognized bodies (JAKIM, MUI, Dar al-Ifta, ISNA) with source attribution. AmanClaw never issues fatwas — it cites scholarly authorities

### Islamic Finance Module

- **Shariah screening** — Check stocks/investments against Islamic finance criteria (debt ratio, revenue sources, purification calculations)
- **Zakat calculator** — Expand existing to cover business zakat, gold/silver, agriculture, livestock with nisab tracking
- **Islamic mortgage (Murabaha) calculator** — Compare Islamic financing products
- **Waqf management** — Track endowment contributions and distributions for organizations (deferred to Phase 4/5 — institutional feature, not core)

### Community Intelligence

- **Prayer time engine** — Already excellent. Add: automatic zone detection via GPS, Ramadan/Eid notifications, Qiyam al-Layl times
- **Mosque network** — Live prayer times from connected mosques, event announcements, community notices
- **Islamic calendar integration** — Hijri date-aware scheduling. "Remind me on 15 Ramadan" just works
- **Khutbah assistant** — Help khatib prepare sermons with topic research, Quran/Hadith references, multilingual drafts

### Ethical AI Guardrails

- **Content filtering** — Three-layer approach: (1) system prompt guardrails set Islamic-aware tone, (2) post-processing filter checks LLM output against a blocklist of sensitive topics, (3) for Islamic rulings specifically, RAG retrieval ensures responses are grounded in scholarly sources rather than LLM hallucination. Responses respect Islamic values without being preachy. Helpful, not judgmental
- **Scholarly attribution** — Every Islamic ruling cites its source. Never presents opinion as fact
- **Madhab awareness** — Knows user's preferred school of thought, presents accordingly
- **Sensitivity handling** — Graceful on controversial topics, redirects to qualified scholars when appropriate
- **Transparency** — Clear labeling: "This is from [source]" vs "This is AI-generated analysis"

### Sovereign Model Strategy

1. **Now:** Use any OpenAI-compatible model (Ollama, vLLM, etc.) — already works
2. **Next:** Fine-tune open models (Qwen, Llama) on Islamic corpus. Create `amanclaw-islamic-7b` and `amanclaw-islamic-70b`
3. **Later:** Partner with Islamic universities (IIUM, Al-Azhar, Madinah University) for scholarly validation
4. **Long-term:** Self-hosted model registry where communities share vetted fine-tuned models

**Key principle:** The model is a tool, not a scholar. AmanClaw always defers to human scholarly authority on matters of deen.

---

## Layer 4: AmanClaw Cloud

### Managed Hosting

- One-click deploy — sign up, connect WhatsApp/Telegram, start using
- Regional data centers — start Malaysia (MDEC-compliant), expand to Indonesia, Saudi Arabia, Turkey, UAE
- Data residency guarantee — user chooses where data lives
- Bring your own model — connect your own API key or use hosted models

### Pricing

| Tier | Target | Price | Includes |
|------|--------|-------|----------|
| **Free** | Individuals | $0 | 1 channel, 100 msgs/day, community skills |
| **Personal** | Power users | ~$5/mo | 3 channels, unlimited msgs, all skills |
| **Community** | Mosques/orgs | ~$15/mo | Unlimited channels, multi-admin, custom persona (tone/personality config via soul files) |
| **Enterprise** | Govt/corp | Custom | Dedicated infra, SLA, compliance, audit logs |

### Marketplace Revenue

- 70/30 split (developer gets 70%)
- Official Islamic skills always free — this is amanah, not a product

### Open Core Model

| Open Source (always free) | Cloud-only |
|---------------------------|------------|
| Engine, pipeline, all crates | Managed hosting |
| All Islamic skills | Auto-scaling |
| Plugin SDK + WASM runtime | Usage analytics dashboard |
| CLI agent | Team collaboration |
| Desktop app | Compliance certifications |
| Self-hosting everything | Premium support SLA |

**Rule:** The core agent never gets paywalled. Cloud sells convenience, not capability.

---

## Roadmap

### Phase 1: General Agent Parity (Months 1-3)

- Full MCP client/server implementation
- CLI agent mode (`amanclaw ask`, `amanclaw agent`, `amanclaw chat`)
- 30+ general skills (developer, research, productivity)
- Skill marketplace v1 with `amanclaw install`
- CI/CD pipeline, publish to crates.io
- Comprehensive test suite + benchmarks
- Launch on GitHub with contributor infrastructure

### Phase 2: Islamic Sovereign Core (Months 3-6)

- Quran engine with tafsir + thematic search
- Hadith engine with isnad grading + cross-referencing
- Fiqh resolver with multi-madhab support
- Islamic finance module (Shariah screening, zakat expansion, murabaha)
- Ethical AI guardrails + scholarly attribution
- Hijri calendar-aware scheduling
- Multi-agent orchestration (basic coordinator + worker pattern)
- Begin university outreach (IIUM, Al-Azhar) — relationship-building for Phase 4 validation

### Phase 3: Cloud & Community (Months 6-9)

- AmanClaw Cloud launch (Malaysia region)
- One-click deploy for WhatsApp/Telegram
- Marketplace with community submissions
- Web-based agent interface
- Freemium pricing live

### Phase 4: Sovereign Infrastructure (Months 9-12)

- Experimental fine-tune of `amanclaw-islamic-7b` (initial release, not production-grade — requires scholar review)
- Self-hosted model registry
- OIC cloud partnerships (Malaysia, Indonesia, Saudi)
- Government compliance certifications
- Formalize university partnerships (outreach started in Phase 2)
- Waqf management module
- Advanced multi-agent orchestration (parallel agents, complex workflows)

### Phase 5: Ecosystem (Year 2+)

- Specialized bots (UstazBot, HalalBot, FinanceBot)
- Mobile app
- Enterprise features (audit logs, SSO, RBAC)
- Open marketplace with revenue sharing
- Developer conference / community events

---

## Strengths

| Strength | Why it matters |
|----------|---------------|
| **Rust + WASM** | 100x lighter than JS alternatives. Runs on a Raspberry Pi. Security sandbox for plugins |
| **Offline-first** | Works without internet. No cloud dependency. True sovereignty |
| **Islamic AI** | 2B+ Muslim market with zero credible AI agents built for them |
| **Open source** | Trust through transparency. Community drives growth |
| **5 chat channels** | Meet users where they are. WhatsApp alone covers 2B+ users |
| **Bilingual** | BM + English native. Expandable to Arabic, Urdu, Turkish, Indonesian |
