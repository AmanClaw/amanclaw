# Unified Interactive Installer — Design

**Date:** 2026-03-10
**Status:** Approved

## Goal

Single `install.sh` bash script that handles fresh install, updates, and full channel configuration for AmanClaw on any Debian/Ubuntu Linux system (including Raspberry Pi).

## Decisions

| Question | Answer |
|----------|--------|
| Target platform | Any Debian/Ubuntu Linux (amd64 + arm64) |
| Install + update? | Yes, detects existing install |
| Script form | Single `install.sh` bash script |
| Channels | All four: Telegram, WhatsApp Web, Discord, Slack |
| Docker | Install automatically if missing |
| LLM setup | Ask for URL + API key, test connection |

## Script Phases

### Phase 1: Mode Detection
- If `$INSTALL_DIR/config.yaml` exists → **UPDATE** mode
- Otherwise → **FRESH INSTALL** mode
- Update mode preserves existing .env, config.yaml, data/
- Default `INSTALL_DIR`: `$HOME/amanclaw`

### Phase 2: System Checks
- Debian/Ubuntu via `/etc/os-release`
- Architecture: amd64 or arm64
- Memory warning if < 512MB free
- Disk warning if < 2GB free

### Phase 3: Docker
- Check `docker --version` and `docker compose version`
- If missing → install via `get.docker.com`
- Add current user to docker group if needed

### Phase 4: Pull Image
- Fresh: pull from registry (or cross-compile and transfer)
- Update: pull latest, keep old image as fallback

### Phase 5: LLM Configuration
- Ask for: API base URL, API key, model name
- Provide sensible defaults (localhost:8001, Qwen3)
- Test connection with a quick API call

### Phase 6: Channel Selection
- Multi-select menu: Telegram, WhatsApp Web, Discord, Slack
- User enters numbers separated by space

### Phase 7: Channel Credential Collection
Per selected channel:

**Telegram:** bot token (validated via getMe API), admin user ID

**WhatsApp Web:** admin phone number, then:
- Install Node.js 18+ if missing
- Install Chromium if missing
- Install wa-bridge (bridge.js + dependencies)
- Create systemd service
- Start bridge, display QR code for scanning
- Wait for successful connection

**Discord:** bot token, allowed channel IDs, admin user ID

**Slack:** bot token, app token, allowed channel IDs

### Phase 8: File Generation
Generates these files from collected inputs:
1. `$INSTALL_DIR/.env` — all secrets
2. `$INSTALL_DIR/config.yaml` — non-secret config
3. `$INSTALL_DIR/docker/docker-compose.yml` — container definition
4. `$INSTALL_DIR/wa-bridge/` — bridge files (if WhatsApp selected)

### Phase 9: Start Services
1. Start Docker container
2. Start wa-bridge systemd service (if WhatsApp)
3. Run health checks on all services
4. Print status summary and useful commands

## Update Mode Behavior
- Preserves existing .env and config.yaml
- Re-pulls Docker image
- Asks "Add more channels?" — doesn't re-prompt existing ones
- Restarts services after update

## Error Handling
- Each step validates before proceeding
- Clear error messages with suggested fixes
- Partial state preserved so re-run picks up where it left off
- Non-zero exit code on failure

## Generated File Locations
```
~/amanclaw/
├── .env                    # secrets
├── config.yaml             # configuration
├── data/                   # persistent data (memory.db, etc.)
├── plugins/                # Python plugins
├── docker/
│   └── docker-compose.yml
└── wa-bridge/              # (if WhatsApp enabled)
    ├── bridge.js
    ├── package.json
    └── .wa-session/
```
