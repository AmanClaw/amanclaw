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
- Skills: shell commands, file ops, system info, reminders, fact memory
- Vision support (send photos for analysis)
- Auto-summarization of long conversations
- Rate limiting and injection detection

## Project Structure

```
amanclaw/
  bot.py            Main Telegram bot
  llm.py            LLM client (native + fallback tool calling)
  memory.py         SQLite conversation/facts/reminders
  security.py       Auth, rate limiter, input sanitizer
  skills/
    shell.py        Whitelisted shell commands
    files.py        Workspace file operations
    system_info.py  CPU/memory/disk status
    remember.py     Save/recall user facts
    reminder.py     Timed reminders
```

## Configuration

**Secrets** go in `.env` (never committed):
```
TELEGRAM_BOT_TOKEN=your-token
LLM_API_KEY=your-key
```

**Settings** go in `config.yaml`:
```yaml
llm:
  base_url: "http://localhost:8001/v1"
  model: "Qwen/Qwen3-VL-30B-A3B-Instruct"

admin_users:
  telegram: [YOUR_USER_ID]
```

## Deployment

### Docker (recommended)

```bash
# Build and run
docker compose up -d

# View logs
docker compose logs -f

# Stop
docker compose down
```

The container runs as non-root with dropped capabilities, read-only filesystem, and memory limits. Data persists in `./data/`.

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

- Secrets in `.env`, never in config or code
- User allowlist with admin approval flow
- Per-user rate limiting
- Prompt injection detection (flags, doesn't block)
- Skill output sandboxing (marked as external data to LLM)
- Shell commands: whitelist-only, no pipes/chains/redirects
- File ops: confined to workspace directory
- Docker: non-root, dropped caps, read-only fs, resource limits
- Systemd: full Linux hardening profile
