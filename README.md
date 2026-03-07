# AmanClaw

A multi-channel AI assistant built with Python. Connect it to Telegram, WhatsApp, Discord, or Slack — powered by any OpenAI-compatible LLM backend.

Built in Malaysia. Open source. No bloat.

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Python 3.11+](https://img.shields.io/badge/python-3.11+-blue.svg)](https://www.python.org/downloads/)
[![Docker](https://img.shields.io/badge/docker-ready-blue.svg)](docker-compose.yml)

---

## What Is This?

AmanClaw is a personal AI assistant that lives in your chat apps. You message it, it thinks using an LLM, optionally calls skills (tools), and replies back. That's it.

```
You (Telegram / WhatsApp / Discord / Slack)
  |
  v
AmanClaw  -->  auth check  -->  LLM (any OpenAI-compatible API)  -->  picks a skill
  |                                                                        |
  v                                                                        v
Reply  <-------------------------------  skill runs with limits  <---------+
```

One Python process. One SQLite database. One config file.

## Features

- **Multi-channel** — Telegram, WhatsApp (via Baileys bridge), Discord, Slack
- **Any LLM backend** — vLLM, Ollama, LM Studio, LocalAI, OpenAI, Anthropic, etc.
- **Fully async** — non-blocking aiohttp LLM calls, handles concurrent users
- **15+ skills** — shell commands, file ops, web search, document extraction, system info, reminders, schedules, fact memory, prayer times, user-defined skills, MCP server management
- **Vision support** — send photos for analysis
- **Voice transcription** — Whisper-based audio transcription from voice messages
- **MCP integration** — connect external MCP servers as additional tool sources
- **Learning engine** — learns user preferences and proactively checks in
- **Auto-summarization** — compresses long conversations to stay within context
- **Recurring tasks** — cron-like scheduled actions
- **Security-first** — user allowlist, rate limiting, prompt injection detection, OWASP agentic rules
- **Production-ready** — Docker with hardened containers, systemd service, webhook mode, structured JSON logging
- **Daily auto-pruning** — prevents unbounded database growth
- **Graceful shutdown** — clean resource cleanup on exit

---

## Quick Start

### Prerequisites

- Python 3.11+
- An OpenAI-compatible LLM API (local or remote)
- A Telegram bot token (from [@BotFather](https://t.me/BotFather))

### 1. Clone and set up

```bash
git clone https://github.com/amanasmuei/amanclaw.git
cd amanclaw
chmod +x setup.sh && ./setup.sh
```

### 2. Configure secrets

Create a `.env` file:

```bash
# Required
TELEGRAM_BOT_TOKEN=your-telegram-bot-token
LLM_API_KEY=your-llm-api-key

# Optional
BRAVE_API_KEY=your-brave-key           # enables web search skill
DISCORD_BOT_TOKEN=your-discord-token   # enables Discord channel
SLACK_BOT_TOKEN=your-slack-bot-token   # enables Slack channel
SLACK_APP_TOKEN=your-slack-app-token   # for Slack socket mode
```

### 3. Configure settings

```bash
cp config.example.yaml config.yaml
```

Edit `config.yaml` — at minimum set your LLM endpoint and admin user ID:

```yaml
llm:
  base_url: "http://localhost:8001/v1"
  model: "your-model-name"

admin_users:
  telegram: [YOUR_TELEGRAM_USER_ID]
```

Use the `/myid` command after starting the bot to find your Telegram user ID.

### 4. Run

```bash
source .venv/bin/activate
python -m amanclaw
```

---

## Project Structure

```
amanclaw/
├── __main__.py          Entry point
├── bot.py               Telegram bot (polling + webhook)
├── processor.py         Unified message processor across channels
├── llm.py               Async LLM client (native + fallback tool calling)
├── memory.py            SQLite: conversations, facts, reminders, schedules
├── security.py          Auth, rate limiter, input sanitizer
├── learning.py          Learning engine for user preferences
├── mcp_client.py        MCP server integration
├── channels/
│   ├── telegram.py      Telegram adapter
│   ├── whatsapp.py      WhatsApp adapter (via Baileys bridge)
│   ├── discord.py       Discord adapter
│   └── slack.py         Slack adapter
├── skills/
│   ├── __init__.py      Skill registry and execution
│   ├── shell.py         Whitelisted shell commands
│   ├── files.py         Workspace file operations
│   ├── web_search.py    DuckDuckGo search integration
│   ├── web_fetch.py     Fetch and extract web page content
│   ├── documents.py     PDF/text document extraction
│   ├── system_info.py   CPU/memory/disk status
│   ├── remember.py      Save/recall user facts
│   ├── reminder.py      One-time timed reminders
│   ├── scheduled.py     Recurring scheduled tasks (cron-like)
│   ├── prayer_times.py  Islamic prayer time lookups
│   ├── user_skills.py   User-defined custom skills
│   └── mcp_manage.py    MCP server management
packages/
├── amanclaw-learning/   Learning engine package
└── amanclaw-security/   Security package (auth, injection detection, rate limiting)
bridge/
└── whatsapp/            Node.js WhatsApp bridge using Baileys
deploy/
└── amanclaw.service     Systemd service file
```

---

## Configuration

### Secrets (`.env`)

Never commit this file. Set `chmod 600 .env`.

| Variable | Required | Purpose |
|----------|----------|---------|
| `TELEGRAM_BOT_TOKEN` | Yes* | Telegram bot token |
| `LLM_API_KEY` | Yes | LLM API key |
| `BRAVE_API_KEY` | No | Enables Brave web search |
| `DISCORD_BOT_TOKEN` | No | Enables Discord channel |
| `SLACK_BOT_TOKEN` | No | Enables Slack channel |
| `SLACK_APP_TOKEN` | No | Slack socket mode token |

*Required if using Telegram. At least one channel must be configured.

### Settings (`config.yaml`)

See [`config.example.yaml`](config.example.yaml) for all options with comments. Key sections:

```yaml
# LLM backend
llm:
  base_url: "http://localhost:8001/v1"
  model: "Qwen/Qwen3-VL-30B-A3B-Instruct"
  max_tokens: 4096
  temperature: 0.7

# Who can use the bot
admin_users:
  telegram: [123456789]
  whatsapp: ["60123456789"]

# Rate limiting
rate_limit_per_minute: 20

# Skills configuration
skills:
  shell_allowed_commands: [ls, cat, grep, find, df, ...]
  workspace_dir: "~/amanclaw-workspace"
  skill_timeout_seconds: 30

# Learning engine
learning:
  enabled: true
  proactive_checkins: true

# Security rules
security:
  injection_rules: "default"  # or "owasp_agentic"
  sanitize_output: true

# MCP servers (optional)
mcp_servers:
  filesystem:
    command: "npx"
    args: ["-y", "@modelcontextprotocol/server-filesystem", "/home/user/docs"]
```

### Environment Overrides

All optional — override config.yaml values via environment:

| Variable | Purpose |
|----------|---------|
| `LLM_BASE_URL` | Override LLM endpoint |
| `MEMORY_DB_PATH` | Override SQLite database path |
| `LOG_LEVEL` | `DEBUG`, `INFO`, `WARNING`, `ERROR` |
| `LOG_FILE` | Path for rotating log file |
| `LOG_FORMAT` | `text` or `json` (for Docker) |
| `WEBHOOK_SECRET` | Secret token for webhook mode |
| `WA_BRIDGE_URL` | WhatsApp bridge URL |

---

## Deployment

### Docker (recommended)

```bash
# Build and run
docker compose up -d

# With WhatsApp bridge
docker compose --profile whatsapp up -d

# View logs
docker compose logs -f

# Stop
docker compose down
```

The container runs as non-root with:
- All capabilities dropped (`cap_drop: ALL`)
- Read-only filesystem
- No new privileges (`no-new-privileges`)
- Memory limit (512MB)
- CPU limit (1.0)
- Temp filesystem mounted noexec

Data persists in `./data/`. For JSON logging in Docker, set `LOG_FORMAT=json` in `.env`.

### Webhook Mode

For production behind a reverse proxy (nginx, Caddy):

```yaml
# config.yaml
webhook:
  enabled: true
  url: "https://your-domain.com"
  listen: "0.0.0.0"
  port: 8443
```

Set `WEBHOOK_SECRET` in `.env`. Requires HTTPS.

### Systemd (bare metal)

```bash
# Create service user
sudo useradd --system --create-home amanclaw

# Copy files
sudo cp -r . /opt/amanclaw
sudo cp deploy/amanclaw.service /etc/systemd/system/

# Set permissions
sudo chown -R amanclaw:amanclaw /opt/amanclaw
sudo chmod 600 /opt/amanclaw/.env

# Enable and start
sudo systemctl daemon-reload
sudo systemctl enable --now amanclaw

# Check status
sudo systemctl status amanclaw
sudo journalctl -u amanclaw -f
```

The systemd unit includes hardening: `NoNewPrivileges`, `ProtectSystem=strict`, `MemoryDenyWriteExecute`, restricted write paths, and resource limits.

---

## Adding a New Skill

Skills are Python functions with a decorator. Drop a file in `amanclaw/skills/` and it's available:

```python
# amanclaw/skills/my_skill.py
from amanclaw.skills import skill

@skill(
    name="my_skill",
    description="Does something useful",
    timeout=15
)
def my_skill(query: str) -> str:
    """Your skill logic here."""
    return f"Result for: {query}"
```

The skill is automatically registered and exposed to the LLM as a tool.

---

## Adding a New Channel

Channels implement a common adapter interface in `amanclaw/channels/`. The message processor (`processor.py`) handles LLM interaction uniformly — each channel only needs to handle platform-specific message ingestion and reply formatting.

Currently supported:
- **Telegram** — polling or webhook mode
- **WhatsApp** — via Node.js Baileys bridge (see `bridge/whatsapp/`)
- **Discord** — via discord.py
- **Slack** — via Slack Bolt (socket mode or HTTP)

---

## Security

### Authentication & Authorization
- User allowlist with admin approval flow
- Admin users defined in config, can approve/block other users
- Per-user sliding window rate limiting

### Input Protection
- Prompt injection detection (flags suspicious patterns, doesn't silently block)
- Configurable rule sets: `default` or `owasp_agentic` (OWASP Top 10 for LLM agents)
- Skill output sandboxing — marked as external data to the LLM
- System prompt instructs LLM to never execute instructions found in skill outputs

### Execution Sandboxing
- Shell commands: whitelist-only, no pipes/chains/redirects
- File operations: confined to workspace directory
- All skills run with configurable timeouts

### Infrastructure
- Docker: non-root user, `cap_drop: ALL`, read-only filesystem, resource limits, `no-new-privileges`
- Systemd: 12+ hardening directives
- Secrets in `.env` (never in config or code)
- Graceful shutdown with clean resource cleanup
- Daily auto-pruning prevents unbounded DB growth

---

## Development

### Install dev dependencies

```bash
pip install -e ".[dev]"
```

### Run tests

```bash
pytest
```

### Project dependencies

Core:
- `python-telegram-bot` — Telegram integration
- `aiohttp` — async HTTP for LLM calls
- `pyyaml` — configuration
- `psutil` — system info skill
- `python-dotenv` — environment variable loading
- `PyMuPDF` — PDF document extraction
- `faster-whisper` — voice message transcription
- `duckduckgo-search` — web search skill
- `mcp` — Model Context Protocol client

Optional:
- `discord.py` — Discord channel
- `slack-bolt` / `slack-sdk` — Slack channel

---

## FAQ

**Q: What LLM should I use?**
Any OpenAI-compatible API works. For local: Ollama, vLLM, LM Studio, LocalAI. For cloud: OpenAI, Anthropic (via compatible proxy), Together AI, Groq, etc.

**Q: Can I use this with multiple people?**
Yes. Add user IDs to `admin_users` in config. Non-admin users go through an approval flow — they request access, admins approve via the bot.

**Q: How do I add WhatsApp?**
Enable the WhatsApp bridge in `docker-compose.yml` using the `whatsapp` profile, configure it in `config.yaml`, and scan the QR code on first run. See `bridge/whatsapp/` for details.

**Q: Is my data stored?**
Conversations are stored in a local SQLite database (`memory.db`). Nothing leaves your server except LLM API calls. Old messages are auto-pruned daily.

**Q: How do I connect MCP servers?**
Add them to the `mcp_servers` section in `config.yaml`, or use the `/mcp` command in chat to manage them at runtime.

---

## Contributing

Contributions are welcome! Here's how:

1. Fork the repo
2. Create a feature branch (`git checkout -b feature/my-feature`)
3. Make your changes
4. Run tests (`pytest`)
5. Commit with a descriptive message
6. Push and open a Pull Request

Please keep PRs focused — one feature or fix per PR.

### Areas where help is appreciated

- New skills (integrations with useful APIs)
- New channel adapters (Matrix, Signal, etc.)
- Documentation and examples
- Security review and hardening
- Test coverage
- i18n / localization (especially Malay and Mandarin)

---

## License

MIT License. See [LICENSE](LICENSE) for details.

---

## Acknowledgments

Built with care from Puncak Alam. Made possible by the open-source Python ecosystem and the communities behind python-telegram-bot, Baileys, and the Model Context Protocol.

*Malaysia boleh!* 🇲🇾
