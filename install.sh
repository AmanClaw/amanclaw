#!/bin/bash
set -euo pipefail

# AmanClaw — Unified Interactive Installer
# Usage: bash install.sh
#
# Handles fresh install and updates on Debian/Ubuntu (amd64 + arm64).
# Supports channels: Telegram, WhatsApp Web, Discord, Slack.

VERSION="1.0.0"
INSTALL_DIR="${AMANCLAW_DIR:-$HOME/amanclaw}"
DOCKER_DIR="$INSTALL_DIR/docker"
WA_BRIDGE_DIR="$INSTALL_DIR/wa-bridge"
DOCKER_IMAGE="amanclaw:latest"

# --- Colors ---
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m'

info()  { echo -e "${CYAN}[INFO]${NC} $*"; }
ok()    { echo -e "${GREEN}  ✓${NC} $*"; }
warn()  { echo -e "${YELLOW}[WARN]${NC} $*"; }
err()   { echo -e "${RED}[ERROR]${NC} $*"; }
die()   { err "$*"; exit 1; }
ask()   { read -rp "$(echo -e "${BOLD}$1${NC}")" "$2"; }
ask_default() {
    local prompt="$1" var="$2" default="$3"
    read -rp "$(echo -e "${BOLD}${prompt} [${default}]: ${NC}")" "$var"
    if [ -z "${!var}" ]; then eval "$var='$default'"; fi
}
confirm() {
    local prompt="${1:-Continue?}"
    read -rp "$(echo -e "${BOLD}${prompt} (y/N): ${NC}")" yn
    [[ "$yn" =~ ^[Yy] ]]
}

banner() {
    echo ""
    echo -e "${CYAN}╔══════════════════════════════════════════╗${NC}"
    echo -e "${CYAN}║${NC}   ${BOLD}AmanClaw — Interactive Setup Wizard${NC}    ${CYAN}║${NC}"
    echo -e "${CYAN}║${NC}   v${VERSION}                                ${CYAN}║${NC}"
    echo -e "${CYAN}╚══════════════════════════════════════════╝${NC}"
    echo ""
}

# ── Phase 1: Mode Detection ──────────────────────────────────────────────────

MODE="fresh"
if [ -f "$INSTALL_DIR/config.yaml" ]; then
    MODE="update"
fi

banner

if [ "$MODE" = "update" ]; then
    info "Existing installation detected at $INSTALL_DIR"
    echo ""
    echo "  Mode: UPDATE — will preserve config, update image, offer new channels"
    echo ""
    if ! confirm "Proceed with update?"; then
        echo "Aborted."
        exit 0
    fi
else
    info "No existing installation found — starting fresh install"
    echo ""
fi

# ── Phase 2: System Checks ───────────────────────────────────────────────────

echo ""
echo -e "${BOLD}=== System Checks ===${NC}"

# OS check
if [ ! -f /etc/os-release ]; then
    die "Cannot detect OS. This installer supports Debian/Ubuntu Linux."
fi
# shellcheck source=/dev/null
. /etc/os-release
if [[ "$ID" != "debian" && "$ID" != "ubuntu" && "${ID_LIKE:-}" != *"debian"* ]]; then
    die "Unsupported OS: $PRETTY_NAME. This installer requires Debian or Ubuntu."
fi
ok "OS: $PRETTY_NAME"

# Architecture
ARCH="$(dpkg --print-architecture 2>/dev/null || uname -m)"
case "$ARCH" in
    amd64|x86_64) ARCH="amd64" ;;
    arm64|aarch64) ARCH="arm64" ;;
    *) die "Unsupported architecture: $ARCH (need amd64 or arm64)" ;;
esac
ok "Architecture: $ARCH"

# Memory
MEM_MB=$(free -m 2>/dev/null | awk '/^Mem:/{print $2}' || echo "0")
if [ "$MEM_MB" -gt 0 ] && [ "$MEM_MB" -lt 512 ]; then
    warn "Low memory: ${MEM_MB}MB (recommended: 512MB+)"
else
    ok "Memory: ${MEM_MB}MB"
fi

# Disk
DISK_AVAIL=$(df -BM "$HOME" 2>/dev/null | awk 'NR==2{gsub("M",""); print $4}' || echo "0")
if [ "$DISK_AVAIL" -gt 0 ] && [ "$DISK_AVAIL" -lt 2048 ]; then
    warn "Low disk space: ${DISK_AVAIL}MB available (recommended: 2GB+)"
else
    ok "Disk: ${DISK_AVAIL}MB available"
fi

# ── Phase 3: Docker ──────────────────────────────────────────────────────────

echo ""
echo -e "${BOLD}=== Docker ===${NC}"

NEED_DOCKER=false
if command -v docker &>/dev/null; then
    ok "Docker: $(docker --version | head -1)"
else
    NEED_DOCKER=true
fi

# Check docker compose (v2 plugin)
if docker compose version &>/dev/null 2>&1; then
    ok "Docker Compose: $(docker compose version --short 2>/dev/null)"
elif [ "$NEED_DOCKER" = false ]; then
    NEED_DOCKER=true
fi

DOCKER_CMD="docker"
DOCKER_COMPOSE_CMD="docker compose"

if [ "$NEED_DOCKER" = true ]; then
    info "Docker not found or incomplete. Installing via get.docker.com..."
    if ! confirm "Install Docker?"; then
        die "Docker is required. Install it manually and re-run this script."
    fi
    curl -fsSL https://get.docker.com | sh
    ok "Docker installed"

    # Add user to docker group
    if ! groups | grep -q docker; then
        sudo usermod -aG docker "$USER"
        warn "Added $USER to docker group. You may need to log out and back in."
        warn "For now, the script will use sudo for docker commands."
        DOCKER_CMD="sudo docker"
        DOCKER_COMPOSE_CMD="sudo docker compose"
    fi
else
    # Check if current user can run docker
    if ! docker ps &>/dev/null 2>&1; then
        warn "Cannot connect to Docker daemon. Using sudo."
        DOCKER_CMD="sudo docker"
        DOCKER_COMPOSE_CMD="sudo docker compose"
    fi
fi

# ── Phase 4: Docker Image ────────────────────────────────────────────────────

echo ""
echo -e "${BOLD}=== AmanClaw Docker Image ===${NC}"

if $DOCKER_CMD image inspect "$DOCKER_IMAGE" &>/dev/null 2>&1; then
    ok "Image found: $DOCKER_IMAGE"
    if [ "$MODE" = "update" ]; then
        info "To update the image, transfer a new one before running this script:"
        echo "    On build machine: docker buildx build --platform linux/$ARCH -t amanclaw:latest --output type=docker,dest=/tmp/amanclaw.tar -f rust/Dockerfile rust/"
        echo "    Transfer:         scp /tmp/amanclaw.tar $(whoami)@$(hostname):/tmp/"
        echo "    Load:             docker load < /tmp/amanclaw.tar"
        echo ""
    fi
else
    warn "Docker image '$DOCKER_IMAGE' not found locally."
    echo ""
    echo "  You need to build and transfer the image first:"
    echo ""
    echo "  Option A — Build on another machine (recommended for Pi):"
    echo "    docker buildx build --platform linux/$ARCH -t amanclaw:latest \\"
    echo "      --output type=docker,dest=/tmp/amanclaw.tar -f rust/Dockerfile rust/"
    LOCAL_IP=$(hostname -I 2>/dev/null | awk '{print $1}' || echo "<this-machine>")
    echo "    scp /tmp/amanclaw.tar $(whoami)@${LOCAL_IP}:/tmp/"
    echo "    docker load < /tmp/amanclaw.tar"
    echo ""
    echo "  Option B — Build locally (slow on Pi, needs ~4GB RAM):"
    echo "    docker build -t amanclaw:latest -f rust/Dockerfile rust/"
    echo ""
    echo "  After loading the image, re-run this script."
    die "Docker image not available"
fi

# ── Phase 5: LLM Configuration ───────────────────────────────────────────────

# Collected config
declare -A ENV_VARS=()
LLM_BASE_URL=""
LLM_API_KEY=""
LLM_MODEL=""

if [ "$MODE" = "fresh" ]; then
    echo ""
    echo -e "${BOLD}=== LLM Backend ===${NC}"
    echo "AmanClaw needs an OpenAI-compatible API endpoint."
    echo "(Works with: vLLM, Ollama, LM Studio, LocalAI, etc.)"
    echo ""

    ask_default "API base URL" LLM_BASE_URL "http://localhost:8001/v1"
    ask_default "API key (or press Enter for none)" LLM_API_KEY ""
    ask_default "Model name" LLM_MODEL "Qwen/Qwen3-VL-30B-A3B-Instruct"

    # Test connection
    echo ""
    info "Testing LLM connection..."
    HTTP_CODE=$(curl -s -o /dev/null -w "%{http_code}" \
        -H "Authorization: Bearer ${LLM_API_KEY:-none}" \
        "${LLM_BASE_URL}/models" 2>/dev/null || echo "000")

    if [ "$HTTP_CODE" = "200" ]; then
        ok "LLM connected (HTTP 200)"
    elif [ "$HTTP_CODE" = "000" ]; then
        warn "Could not reach LLM at $LLM_BASE_URL (connection refused)"
        warn "You can configure this later in $INSTALL_DIR/.env"
    else
        warn "LLM returned HTTP $HTTP_CODE — may still work, continuing"
    fi

    [ -n "$LLM_API_KEY" ] && ENV_VARS[LLM_API_KEY]="$LLM_API_KEY"
else
    info "LLM config preserved from existing installation"
fi

# ── Phase 6 & 7: Channel Setup ───────────────────────────────────────────────

echo ""
echo -e "${BOLD}=== Channel Setup ===${NC}"
echo ""

# Track which channels to enable
ENABLE_TELEGRAM=false
ENABLE_WHATSAPP=false
ENABLE_DISCORD=false
ENABLE_SLACK=false

# Admin users
ADMIN_TELEGRAM_IDS=""
ADMIN_WHATSAPP_NUMBERS=""

# In update mode, detect existing channels
EXISTING_CHANNELS=""
if [ "$MODE" = "update" ] && [ -f "$INSTALL_DIR/.env" ]; then
    grep -q "TELEGRAM_BOT_TOKEN=." "$INSTALL_DIR/.env" 2>/dev/null && EXISTING_CHANNELS+=" telegram"
    grep -q "WAHA_API_URL=." "$INSTALL_DIR/.env" 2>/dev/null && EXISTING_CHANNELS+=" whatsapp"
    grep -q "DISCORD_BOT_TOKEN=." "$INSTALL_DIR/.env" 2>/dev/null && EXISTING_CHANNELS+=" discord"
    grep -q "SLACK_BOT_TOKEN=." "$INSTALL_DIR/.env" 2>/dev/null && EXISTING_CHANNELS+=" slack"

    if [ -n "$EXISTING_CHANNELS" ]; then
        info "Currently enabled channels:$EXISTING_CHANNELS"
        echo ""
    fi
fi

echo "Which channels do you want to enable?"
echo "  [1] Telegram"
echo "  [2] WhatsApp Web (via wa-bridge)"
echo "  [3] Discord"
echo "  [4] Slack"
echo ""
ask "Enter numbers separated by space (e.g. \"1 2\"): " CHANNEL_CHOICES

for choice in $CHANNEL_CHOICES; do
    case "$choice" in
        1) ENABLE_TELEGRAM=true ;;
        2) ENABLE_WHATSAPP=true ;;
        3) ENABLE_DISCORD=true ;;
        4) ENABLE_SLACK=true ;;
        *) warn "Unknown choice: $choice (skipping)" ;;
    esac
done

# --- Telegram ---
if [ "$ENABLE_TELEGRAM" = true ]; then
    echo ""
    echo -e "${BOLD}--- Telegram ---${NC}"

    if [[ "$EXISTING_CHANNELS" == *"telegram"* ]] && [ "$MODE" = "update" ]; then
        ok "Telegram already configured (preserving existing token)"
    else
        ask "Bot token (from @BotFather): " TG_TOKEN
        if [ -z "$TG_TOKEN" ]; then
            warn "No token provided — skipping Telegram"
            ENABLE_TELEGRAM=false
        else
            # Validate token
            info "Validating token..."
            TG_RESULT=$(curl -s "https://api.telegram.org/bot${TG_TOKEN}/getMe" 2>/dev/null || echo "{}")
            if echo "$TG_RESULT" | grep -q '"ok":true'; then
                BOT_NAME=$(echo "$TG_RESULT" | grep -o '"username":"[^"]*"' | cut -d'"' -f4)
                ok "Telegram bot: @${BOT_NAME}"
            else
                warn "Token validation failed — saving anyway (check token later)"
            fi
            ENV_VARS[TELEGRAM_BOT_TOKEN]="$TG_TOKEN"
        fi

        ask_default "Admin user ID (use /myid to find, or skip)" ADMIN_TG_ID ""
        [ -n "$ADMIN_TG_ID" ] && ADMIN_TELEGRAM_IDS="$ADMIN_TG_ID"
    fi
fi

# --- Discord ---
if [ "$ENABLE_DISCORD" = true ]; then
    echo ""
    echo -e "${BOLD}--- Discord ---${NC}"

    if [[ "$EXISTING_CHANNELS" == *"discord"* ]] && [ "$MODE" = "update" ]; then
        ok "Discord already configured (preserving existing token)"
    else
        ask "Bot token (from Discord Developer Portal): " DISCORD_TOKEN
        if [ -z "$DISCORD_TOKEN" ]; then
            warn "No token provided — skipping Discord"
            ENABLE_DISCORD=false
        else
            ENV_VARS[DISCORD_BOT_TOKEN]="$DISCORD_TOKEN"
            ok "Discord token saved"
        fi
    fi
fi

# --- Slack ---
if [ "$ENABLE_SLACK" = true ]; then
    echo ""
    echo -e "${BOLD}--- Slack ---${NC}"

    if [[ "$EXISTING_CHANNELS" == *"slack"* ]] && [ "$MODE" = "update" ]; then
        ok "Slack already configured (preserving existing tokens)"
    else
        ask "Bot token (xoxb-...): " SLACK_BOT_TOKEN_VAL
        ask "App token (xapp-...): " SLACK_APP_TOKEN_VAL
        if [ -z "$SLACK_BOT_TOKEN_VAL" ] || [ -z "$SLACK_APP_TOKEN_VAL" ]; then
            warn "Both tokens required — skipping Slack"
            ENABLE_SLACK=false
        else
            ENV_VARS[SLACK_BOT_TOKEN]="$SLACK_BOT_TOKEN_VAL"
            ENV_VARS[SLACK_APP_TOKEN]="$SLACK_APP_TOKEN_VAL"
            ok "Slack tokens saved"
        fi
    fi
fi

# --- WhatsApp Web ---
if [ "$ENABLE_WHATSAPP" = true ]; then
    echo ""
    echo -e "${BOLD}--- WhatsApp Web ---${NC}"

    if [[ "$EXISTING_CHANNELS" == *"whatsapp"* ]] && [ "$MODE" = "update" ]; then
        ok "WhatsApp Web already configured"
        if confirm "Re-install wa-bridge?"; then
            : # Fall through to install
        else
            ENABLE_WHATSAPP="existing"
        fi
    fi

    if [ "$ENABLE_WHATSAPP" = true ]; then
        ask_default "Admin phone number (without +, e.g. 60123456789)" WA_ADMIN_PHONE ""
        [ -n "$WA_ADMIN_PHONE" ] && ADMIN_WHATSAPP_NUMBERS="$WA_ADMIN_PHONE"

        # Check Node.js
        info "Checking Node.js..."
        if command -v node &>/dev/null; then
            NODE_VER=$(node -v | sed 's/v//' | cut -d. -f1)
            if [ "$NODE_VER" -ge 18 ]; then
                ok "Node.js $(node -v)"
            else
                die "Node.js 18+ required (found $(node -v)). Install: curl -fsSL https://deb.nodesource.com/setup_20.x | sudo -E bash - && sudo apt-get install -y nodejs"
            fi
        else
            info "Node.js not found. Installing Node.js 20..."
            curl -fsSL https://deb.nodesource.com/setup_20.x | sudo -E bash -
            sudo apt-get install -y nodejs
            ok "Node.js $(node -v) installed"
        fi

        # Check Chromium
        info "Checking Chromium..."
        CHROMIUM_BIN=""
        for bin in chromium chromium-browser google-chrome; do
            if command -v "$bin" &>/dev/null; then
                CHROMIUM_BIN="$(command -v "$bin")"
                break
            fi
        done
        if [ -z "$CHROMIUM_BIN" ]; then
            info "Installing Chromium..."
            sudo apt-get update -qq && sudo apt-get install -y -qq chromium || sudo apt-get install -y -qq chromium-browser
            CHROMIUM_BIN="$(command -v chromium 2>/dev/null || command -v chromium-browser 2>/dev/null)"
        fi
        ok "Chromium: $CHROMIUM_BIN"

        # Install wa-bridge
        info "Installing wa-bridge..."
        mkdir -p "$WA_BRIDGE_DIR"

        # Write bridge.js
        cat > "$WA_BRIDGE_DIR/bridge.js" << 'BRIDGEJS'
const { Client, LocalAuth } = require("whatsapp-web.js");
const express = require("express");
const qrcode = require("qrcode-terminal");

const AMANCLAW_WEBHOOK = process.env.WEBHOOK_URL || "http://127.0.0.1:8081/webhook";
const PORT = process.env.BRIDGE_PORT || 3000;

const app = express();
app.use(express.json());

let waClient = null;
let isReady = false;

const client = new Client({
  authStrategy: new LocalAuth({ dataPath: "./.wa-session" }),
  puppeteer: {
    executablePath: process.env.CHROMIUM_PATH || "/usr/bin/chromium",
    headless: true,
    args: [
      "--no-sandbox",
      "--disable-setuid-sandbox",
      "--disable-dev-shm-usage",
      "--disable-gpu",
      "--no-first-run",
      "--single-process",
      "--disable-extensions",
    ],
  },
});

client.on("qr", (qr) => {
  console.log("\n=== Scan this QR code with WhatsApp ===\n");
  qrcode.generate(qr, { small: true });
  console.log("\nWaiting for scan...\n");
});

client.on("ready", () => {
  console.log("[wa-bridge] WhatsApp client ready!");
  isReady = true;
  waClient = client;
});

client.on("authenticated", () => {
  console.log("[wa-bridge] Authenticated successfully");
});

client.on("auth_failure", (msg) => {
  console.error("[wa-bridge] Auth failure:", msg);
});

client.on("disconnected", (reason) => {
  console.warn("[wa-bridge] Disconnected:", reason);
  isReady = false;
  setTimeout(() => client.initialize(), 5000);
});

client.on("message", async (msg) => {
  const chat = await msg.getChat();
  const contact = await msg.getContact();

  const payload = {
    event: "message",
    session: "default",
    payload: {
      id: msg.id._serialized,
      from: msg.from,
      to: msg.to,
      body: msg.body || "",
      type: msg.type,
      fromMe: msg.fromMe,
      hasMedia: msg.hasMedia,
      chatId: chat.id._serialized,
      _data: {
        notifyName: contact.pushname || contact.name || null,
      },
    },
  };

  try {
    const res = await fetch(AMANCLAW_WEBHOOK, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(payload),
    });
    if (!res.ok) console.error("[wa-bridge] Webhook error:", res.status);
  } catch (err) {
    console.error("[wa-bridge] Failed to forward message:", err.message);
  }
});

// WAHA-compatible: POST /api/sendText
app.post("/api/sendText", async (req, res) => {
  if (!isReady) return res.status(503).json({ error: "WhatsApp not connected" });

  const { chatId, text } = req.body;
  if (!chatId || !text) return res.status(400).json({ error: "chatId and text required" });

  try {
    const result = await waClient.sendMessage(chatId, text);
    res.json({ id: result.id._serialized, status: "sent" });
  } catch (err) {
    console.error("[wa-bridge] Send error:", err.message);
    res.status(500).json({ error: err.message });
  }
});

// GET /api/sessions
app.get("/api/sessions", (req, res) => {
  res.json([{ name: "default", status: isReady ? "WORKING" : "STARTING" }]);
});

// Health
app.get("/health", (req, res) => {
  res.json({ status: isReady ? "connected" : "disconnected", uptime: process.uptime() });
});

app.listen(PORT, () => {
  console.log("[wa-bridge] API server on port " + PORT);
  console.log("[wa-bridge] Webhook target: " + AMANCLAW_WEBHOOK);
});

client.initialize();
console.log("[wa-bridge] Initializing WhatsApp Web client...");
BRIDGEJS

        # Write package.json
        cat > "$WA_BRIDGE_DIR/package.json" << 'PACKAGEJSON'
{
  "name": "wa-bridge",
  "version": "1.0.0",
  "description": "WhatsApp Web bridge for AmanClaw (WAHA-compatible API)",
  "main": "bridge.js",
  "scripts": {
    "start": "node bridge.js"
  },
  "dependencies": {
    "whatsapp-web.js": "^1.26.1-alpha.3",
    "qrcode-terminal": "^0.12.0",
    "express": "^4.21.0"
  }
}
PACKAGEJSON

        # npm install
        cd "$WA_BRIDGE_DIR"
        npm install --production 2>&1 | tail -5
        cd - >/dev/null
        ok "wa-bridge dependencies installed"

        # Create systemd service
        cat > /tmp/wa-bridge.service << SERVICEEOF
[Unit]
Description=WhatsApp Web Bridge for AmanClaw
After=network.target

[Service]
Type=simple
User=$(whoami)
WorkingDirectory=$WA_BRIDGE_DIR
ExecStart=$(command -v node) bridge.js
Environment=WEBHOOK_URL=http://127.0.0.1:8081/webhook
Environment=BRIDGE_PORT=3000
Environment=CHROMIUM_PATH=$CHROMIUM_BIN
Restart=on-failure
RestartSec=10

[Install]
WantedBy=multi-user.target
SERVICEEOF

        sudo cp /tmp/wa-bridge.service /etc/systemd/system/wa-bridge.service
        sudo systemctl daemon-reload
        sudo systemctl enable wa-bridge
        ok "wa-bridge systemd service created"

        # Set WAHA env vars
        ENV_VARS[WAHA_API_URL]="http://127.0.0.1:3000"
        ENV_VARS[WAHA_SESSION]="default"
        ENV_VARS[WAHA_WEBHOOK_PORT]="8081"

        # Start and wait for QR
        info "Starting wa-bridge..."
        sudo systemctl start wa-bridge
        echo ""
        echo "  The QR code should appear in the wa-bridge logs."
        echo "  Open another terminal and run:"
        echo ""
        echo "    sudo journalctl -u wa-bridge -f"
        echo ""
        echo "  Scan the QR with WhatsApp → Settings → Linked Devices → Link a Device"
        echo ""
        read -rp "$(echo -e "${BOLD}Press Enter after scanning the QR code...${NC}")"

        # Verify connection
        info "Checking wa-bridge health..."
        sleep 3
        WA_HEALTH=$(curl -s http://127.0.0.1:3000/health 2>/dev/null || echo '{}')
        if echo "$WA_HEALTH" | grep -q '"connected"'; then
            ok "WhatsApp Web connected!"
        else
            warn "WhatsApp Web not connected yet. Check: sudo journalctl -u wa-bridge -f"
        fi
    fi
fi

# ── Phase 8: Generate Files ──────────────────────────────────────────────────

echo ""
echo -e "${BOLD}=== Generating Configuration ===${NC}"

mkdir -p "$INSTALL_DIR" "$INSTALL_DIR/data" "$INSTALL_DIR/plugins" "$DOCKER_DIR"

# --- .env ---
ENV_FILE="$INSTALL_DIR/.env"

if [ "$MODE" = "fresh" ]; then
    {
        echo "# AmanClaw — Environment Variables"
        echo "# Generated by install.sh on $(date -Iseconds)"
        echo ""

        # LLM
        [ -n "$LLM_API_KEY" ] && echo "LLM_API_KEY=$LLM_API_KEY"

        # Channel tokens
        for key in TELEGRAM_BOT_TOKEN DISCORD_BOT_TOKEN SLACK_BOT_TOKEN SLACK_APP_TOKEN \
                   WAHA_API_URL WAHA_SESSION WAHA_WEBHOOK_PORT; do
            [ -n "${ENV_VARS[$key]:-}" ] && echo "$key=${ENV_VARS[$key]}"
        done

        echo ""
        echo "# Islamic plugin API keys (optional)"
        echo "SUNNAH_API_KEY="
        echo "GOOGLE_PLACES_API_KEY="
    } > "$ENV_FILE"
    chmod 600 "$ENV_FILE"
    ok "Created $ENV_FILE"
else
    # Update mode: append new env vars to existing .env
    for key in TELEGRAM_BOT_TOKEN DISCORD_BOT_TOKEN SLACK_BOT_TOKEN SLACK_APP_TOKEN \
               WAHA_API_URL WAHA_SESSION WAHA_WEBHOOK_PORT; do
        if [ -n "${ENV_VARS[$key]:-}" ] && ! grep -q "^${key}=" "$ENV_FILE" 2>/dev/null; then
            echo "$key=${ENV_VARS[$key]}" >> "$ENV_FILE"
            ok "Added $key to .env"
        fi
    done
fi

# --- config.yaml ---
if [ "$MODE" = "fresh" ]; then
    CONFIG_FILE="$INSTALL_DIR/config.yaml"

    # Build admin_users sections
    TG_ADMIN_LINE="[]"
    [ -n "$ADMIN_TELEGRAM_IDS" ] && TG_ADMIN_LINE="[$ADMIN_TELEGRAM_IDS]"

    WA_ADMIN_LINE="[]"
    [ -n "$ADMIN_WHATSAPP_NUMBERS" ] && WA_ADMIN_LINE="[\"$ADMIN_WHATSAPP_NUMBERS\"]"

    cat > "$CONFIG_FILE" << CONFIGEOF
# AmanClaw — Configuration
# Generated by install.sh on $(date -Iseconds)

llm:
  base_url: "${LLM_BASE_URL}"
  model: "${LLM_MODEL}"
  max_tokens: 4096
  temperature: 0.7

admin_users:
  telegram: ${TG_ADMIN_LINE}
  whatsapp-web: ${WA_ADMIN_LINE}

rate_limit_per_minute: 20
memory_db: data/memory.db

skills:
  shell_allowed_commands:
    - ls
    - cat
    - head
    - tail
    - wc
    - grep
    - find
    - which
    - df
    - du
    - free
    - uptime
    - date
    - ps
    - whoami
    - hostname
    - pwd
    - tree
  shell_working_dir: "~"
  workspace_dir: "~/amanclaw-workspace"
  skill_timeout_seconds: 30

learning:
  enabled: true
  proactive_checkins: true
  checkin_day: 6
  checkin_hour: 10
  checkin_min_age_days: 14
  document_max_chars: 50000
CONFIGEOF

    # Add Discord config if enabled
    if [ "$ENABLE_DISCORD" = true ]; then
        cat >> "$CONFIG_FILE" << 'DISCORDEOF'

discord:
  enabled: true
  allowed_channels: []
  command_prefix: "!"
DISCORDEOF
    fi

    # Add Slack config if enabled
    if [ "$ENABLE_SLACK" = true ]; then
        cat >> "$CONFIG_FILE" << 'SLACKEOF'

slack:
  enabled: true
  socket_mode: true
  allowed_channels: []
SLACKEOF
    fi

    cat >> "$CONFIG_FILE" << 'SECURITYEOF'

security:
  injection_rules: "default"
  sanitize_output: true

script_plugins:
  hadith:
    command: "python3"
    args: ["plugins/skill_hadith.py"]
    env:
      SUNNAH_API_KEY: "${SUNNAH_API_KEY}"
  halal:
    command: "python3"
    args: ["plugins/skill_halal.py"]
    env: {}
  zakat:
    command: "python3"
    args: ["plugins/skill_zakat.py"]
    env: {}
  masjid:
    command: "python3"
    args: ["plugins/skill_masjid.py"]
    env:
      GOOGLE_PLACES_API_KEY: "${GOOGLE_PLACES_API_KEY}"
  khutbah:
    command: "python3"
    args: ["plugins/skill_khutbah.py"]
    env: {}
  jakim:
    command: "python3"
    args: ["plugins/skill_jakim.py"]
    env: {}
SECURITYEOF

    ok "Created $CONFIG_FILE"
fi

# --- docker-compose.yml ---
COMPOSE_FILE="$DOCKER_DIR/docker-compose.yml"
cat > "$COMPOSE_FILE" << COMPOSEEOF
# AmanClaw — Docker Compose
# Generated by install.sh on $(date -Iseconds)
services:
  amanclaw:
    image: ${DOCKER_IMAGE}
    container_name: amanclaw
    restart: unless-stopped
    env_file: ${INSTALL_DIR}/.env
    network_mode: host
    volumes:
      - ${INSTALL_DIR}/config.yaml:/home/amanclaw/config.yaml:ro
      - ${INSTALL_DIR}/data:/home/amanclaw/data
      - ${INSTALL_DIR}/plugins:/home/amanclaw/plugins:ro
    security_opt:
      - no-new-privileges:true
    cap_drop:
      - ALL
    tmpfs:
      - /tmp:noexec,nosuid,size=50M
    mem_limit: 512m
    cpus: "1.0"
COMPOSEEOF
ok "Created $COMPOSE_FILE"

# ── Phase 9: Start Services ──────────────────────────────────────────────────

echo ""
echo -e "${BOLD}=== Starting AmanClaw ===${NC}"

# Start Docker container
info "Starting Docker container..."
cd "$DOCKER_DIR"
$DOCKER_COMPOSE_CMD up -d
cd - >/dev/null
ok "AmanClaw container started"

# Health check
sleep 5
if $DOCKER_CMD ps --filter name=amanclaw --format '{{.Status}}' | grep -qi "up"; then
    ok "AmanClaw is running"
else
    warn "AmanClaw container may not have started correctly"
    warn "Check logs: $DOCKER_COMPOSE_CMD -f $COMPOSE_FILE logs -f"
fi

# ── Summary ──────────────────────────────────────────────────────────────────

echo ""
echo -e "${GREEN}╔══════════════════════════════════════════╗${NC}"
echo -e "${GREEN}║${NC}   ${BOLD}Setup Complete!${NC}                        ${GREEN}║${NC}"
echo -e "${GREEN}╚══════════════════════════════════════════╝${NC}"
echo ""
echo "  Install dir: $INSTALL_DIR"
echo ""
echo "  Enabled channels:"
[ "$ENABLE_TELEGRAM" = true ] && echo "    ✓ Telegram"
[[ "$ENABLE_WHATSAPP" == true || "$ENABLE_WHATSAPP" == "existing" ]] && echo "    ✓ WhatsApp Web"
[ "$ENABLE_DISCORD" = true ] && echo "    ✓ Discord"
[ "$ENABLE_SLACK" = true ] && echo "    ✓ Slack"
echo ""
echo -e "${BOLD}Commands:${NC}"
echo "  Status:     $DOCKER_COMPOSE_CMD -f $COMPOSE_FILE ps"
echo "  Logs:       $DOCKER_COMPOSE_CMD -f $COMPOSE_FILE logs -f"
echo "  Restart:    $DOCKER_COMPOSE_CMD -f $COMPOSE_FILE restart"
echo "  Update:     bash install.sh  (re-run this script)"
if [[ "$ENABLE_WHATSAPP" == true || "$ENABLE_WHATSAPP" == "existing" ]]; then
    echo "  WA status:  curl http://localhost:3000/health"
    echo "  WA logs:    sudo journalctl -u wa-bridge -f"
fi
echo ""
