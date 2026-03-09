#!/bin/bash
set -euo pipefail

# WhatsApp Web Bridge — Setup Script
# Run this on the target machine (e.g. Raspberry Pi) to install the bridge.

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
INSTALL_DIR="${WA_BRIDGE_DIR:-$HOME/wa-bridge}"

echo "=== WhatsApp Web Bridge Setup ==="
echo ""

# --- Check prerequisites ---
echo "[1/5] Checking prerequisites..."

if ! command -v node &>/dev/null; then
    echo "  ERROR: Node.js not found. Install Node.js 18+ first:"
    echo "    curl -fsSL https://deb.nodesource.com/setup_18.x | sudo -E bash -"
    echo "    sudo apt-get install -y nodejs"
    exit 1
fi

NODE_VERSION=$(node -v | sed 's/v//' | cut -d. -f1)
if [ "$NODE_VERSION" -lt 18 ]; then
    echo "  ERROR: Node.js 18+ required (found v$(node -v))"
    exit 1
fi
echo "  Node.js $(node -v) OK"

# Check for Chromium
CHROMIUM_BIN=""
for bin in chromium chromium-browser google-chrome; do
    if command -v "$bin" &>/dev/null; then
        CHROMIUM_BIN="$(command -v "$bin")"
        break
    fi
done

if [ -z "$CHROMIUM_BIN" ]; then
    echo "  Chromium not found. Installing..."
    sudo apt-get update -qq && sudo apt-get install -y -qq chromium
    CHROMIUM_BIN="$(command -v chromium || command -v chromium-browser)"
fi
echo "  Chromium: $CHROMIUM_BIN"

# --- Install bridge ---
echo ""
echo "[2/5] Installing wa-bridge to $INSTALL_DIR..."
mkdir -p "$INSTALL_DIR"
cp "$SCRIPT_DIR/bridge.js" "$INSTALL_DIR/"
cp "$SCRIPT_DIR/package.json" "$INSTALL_DIR/"

cd "$INSTALL_DIR"
npm install --production 2>&1 | tail -3
echo "  Dependencies installed"

# --- Configure systemd service ---
echo ""
echo "[3/5] Setting up systemd service..."

cat > /tmp/wa-bridge.service << EOF
[Unit]
Description=WhatsApp Web Bridge for AmanClaw
After=network.target

[Service]
Type=simple
User=$(whoami)
WorkingDirectory=$INSTALL_DIR
ExecStart=$(command -v node) bridge.js
Environment=WEBHOOK_URL=http://127.0.0.1:8081/webhook
Environment=BRIDGE_PORT=3000
Environment=CHROMIUM_PATH=$CHROMIUM_BIN
Restart=on-failure
RestartSec=10

[Install]
WantedBy=multi-user.target
EOF

sudo cp /tmp/wa-bridge.service /etc/systemd/system/wa-bridge.service
sudo systemctl daemon-reload
sudo systemctl enable wa-bridge
echo "  Service installed and enabled"

# --- Add env vars to AmanClaw .env ---
echo ""
echo "[4/5] Configuring AmanClaw .env..."
AMANCLAW_ENV="$HOME/amanclaw/.env"
if [ -f "$AMANCLAW_ENV" ]; then
    if ! grep -q "WAHA_API_URL" "$AMANCLAW_ENV"; then
        echo "" >> "$AMANCLAW_ENV"
        echo "# WhatsApp Web Bridge" >> "$AMANCLAW_ENV"
        echo "WAHA_API_URL=http://127.0.0.1:3000" >> "$AMANCLAW_ENV"
        echo "WAHA_SESSION=default" >> "$AMANCLAW_ENV"
        echo "WAHA_WEBHOOK_PORT=8081" >> "$AMANCLAW_ENV"
        echo "  Added WAHA env vars to $AMANCLAW_ENV"
    else
        echo "  WAHA env vars already present"
    fi
else
    echo "  WARNING: $AMANCLAW_ENV not found — add these manually:"
    echo "    WAHA_API_URL=http://127.0.0.1:3000"
    echo "    WAHA_SESSION=default"
    echo "    WAHA_WEBHOOK_PORT=8081"
fi

# --- Start and show QR ---
echo ""
echo "[5/5] Starting wa-bridge..."
sudo systemctl start wa-bridge
sleep 15

echo ""
echo "=== Setup Complete ==="
echo ""
echo "The bridge is running. Check for the QR code:"
echo "  sudo journalctl -u wa-bridge -f"
echo ""
echo "Scan the QR code with WhatsApp:"
echo "  WhatsApp → Settings → Linked Devices → Link a Device"
echo ""
echo "After scanning, restart AmanClaw to connect:"
echo "  cd ~/amanclaw-docker && docker compose restart"
echo ""
echo "Commands:"
echo "  Status:  sudo systemctl status wa-bridge"
echo "  Logs:    sudo journalctl -u wa-bridge -f"
echo "  Restart: sudo systemctl restart wa-bridge"
echo "  Health:  curl http://localhost:3000/health"
