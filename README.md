# AmanClaw

A personal AI assistant on Telegram. Python, SQLite, OpenAI-compatible LLM backend.

## Quick Start

```bash
# 1. Clone and set up
chmod +x setup.sh && ./setup.sh

# 2. Add your secrets to .env
#    TELEGRAM_BOT_TOKEN=...
#    LLM_API_KEY=...

# 3. Edit config.yaml (LLM endpoint, admin user ID)

# 4. Run
source .venv/bin/activate
python -m amanclaw
```

## Features

- Telegram bot with user registration/approval flow
- OpenAI-compatible LLM (vLLM, Ollama, LM Studio, etc.)
- Fully async — non-blocking aiohttp LLM calls
- 15 skills: shell, files, web search, documents, system info, reminders, schedules, fact memory
- Vision support (send photos for analysis)
- Auto-summarization of long conversations
- Recurring scheduled tasks (cron-like)
- Webhook mode for production (optional, polling by default)
- Structured logging with JSON mode for Docker
- Rotating log files with configurable level
- Daily auto-pruning of old messages and delivered reminders
- Error reporting to admin via Telegram
- Graceful shutdown with resource cleanup
- Rate limiting and prompt injection detection

## Project Structure

```
amanclaw/
  bot.py            Main Telegram bot (polling + webhook)
  llm.py            Async LLM client (aiohttp, native + fallback tool calling)
  memory.py         SQLite conversation/facts/reminders/schedules
  security.py       Auth, rate limiter, input sanitizer
  skills/
    shell.py        Whitelisted shell commands
    files.py        Workspace file operations
    web_search.py   Brave Search API integration
    documents.py    PDF/text document extraction
    system_info.py  CPU/memory/disk status
    remember.py     Save/recall user facts
    reminder.py     One-time timed reminders
    scheduled.py    Recurring scheduled tasks
```

## Configuration

**Secrets** go in `.env` (never committed):
```
TELEGRAM_BOT_TOKEN=your-token
LLM_API_KEY=your-key
BRAVE_API_KEY=your-key          # optional, for web search
```

**Settings** go in `config.yaml`:
```yaml
llm:
  base_url: "http://localhost:8001/v1"
  model: "Qwen/Qwen3-VL-30B-A3B-Instruct"

admin_users:
  telegram: [YOUR_USER_ID]
```

**Environment overrides** (all optional):
| Variable | Purpose |
|----------|---------|
| `LLM_BASE_URL` | Override LLM endpoint from config |
| `LLM_API_KEY` | LLM API key |
| `TELEGRAM_BOT_TOKEN` | Telegram bot token |
| `BRAVE_API_KEY` | Enables web search skill |
| `MEMORY_DB_PATH` | Override SQLite database path |
| `LOG_LEVEL` | `DEBUG`, `INFO`, `WARNING`, `ERROR` |
| `LOG_FILE` | Path for rotating log file |
| `LOG_FORMAT` | `text` or `json` (for Docker) |
| `WEBHOOK_SECRET` | Secret token for webhook mode |

## Deployment

### Docker (recommended)

```bash
# Build and run
docker compose up -d

# View logs (JSON structured)
docker compose logs -f

# Stop
docker compose down
```

The container runs as non-root with dropped capabilities, read-only filesystem, and memory limits. Data persists in `./data/`.

For JSON logging in Docker, set `LOG_FORMAT=json` in `.env`.

### Webhook mode

For production, use webhook instead of polling:

```yaml
# config.yaml
webhook:
  enabled: true
  url: "https://your-domain.com"
  listen: "0.0.0.0"
  port: 8443
```

Set `WEBHOOK_SECRET` in `.env`. Requires HTTPS — use with a reverse proxy (nginx, caddy).

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

## Security

- Secrets in `.env` (chmod 600), never in config or code
- User allowlist with admin approval flow
- Per-user sliding window rate limiting
- Prompt injection detection (flags, doesn't block)
- Skill output sandboxing (marked as external data to LLM)
- Shell commands: whitelist-only, no pipes/chains/redirects
- File ops: confined to workspace directory
- Docker: non-root, `cap_drop: ALL`, read-only fs, resource limits, `no-new-privileges`
- Systemd: 12+ hardening directives
- Graceful shutdown: aiohttp session + SQLite connections closed cleanly
- Daily auto-pruning prevents unbounded DB growth
