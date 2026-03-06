#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")" && pwd)"

# Kill any stale instances
pkill -f "python.*amanclaw" 2>/dev/null || true
pkill -f "node.*bridge/whatsapp" 2>/dev/null || true
sleep 1

cleanup() {
    echo ""
    echo "Shutting down..."
    kill "$BRIDGE_PID" 2>/dev/null || true
    kill "$BOT_PID" 2>/dev/null || true
    wait 2>/dev/null
    echo "Done."
}
trap cleanup EXIT INT TERM

# --- WhatsApp Bridge ---
if grep -q 'enabled: true' "$ROOT/config.yaml" 2>/dev/null; then
    echo "Starting WhatsApp bridge..."
    cd "$ROOT/bridge/whatsapp"
    if [ ! -d node_modules ]; then
        echo "Installing bridge dependencies..."
        npm install --silent
    fi
    node index.js &
    BRIDGE_PID=$!
    cd "$ROOT"

    # Wait for QR code / bridge ready
    echo ""
    echo "=== Waiting for WhatsApp bridge (port 3001)... ==="
    echo "=== Scan the QR code above with WhatsApp ==="
    echo ""
    sleep 3
else
    echo "WhatsApp not enabled in config.yaml, skipping bridge."
    BRIDGE_PID=""
fi

# --- Python dependencies ---
cd "$ROOT"
if [ ! -d .venv ]; then
    echo "Creating virtual environment..."
    python3 -m venv .venv
fi
source .venv/bin/activate
pip install -q -e ".[pdf]" 2>/dev/null

# --- Python Bot ---
echo "Starting AmanClaw bot..."
python3 -m amanclaw &
BOT_PID=$!

echo "Bot started (PID: $BOT_PID)"
[ -n "${BRIDGE_PID:-}" ] && echo "Bridge started (PID: $BRIDGE_PID)"
echo "Press Ctrl+C to stop everything."

wait
