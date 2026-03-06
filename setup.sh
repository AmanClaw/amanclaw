#!/bin/bash
# AmanClaw — Quick Setup
# Run: chmod +x setup.sh && ./setup.sh

set -e

echo "=== AmanClaw Setup ==="
echo ""

# Check Python
if ! command -v python3 &> /dev/null; then
    echo "Error: Python 3.11+ is required. Install it first."
    exit 1
fi

PY_VERSION=$(python3 -c 'import sys; print(f"{sys.version_info.major}.{sys.version_info.minor}")')
PY_MAJOR=$(echo "$PY_VERSION" | cut -d. -f1)
PY_MINOR=$(echo "$PY_VERSION" | cut -d. -f2)
if [ "$PY_MAJOR" -lt 3 ] || { [ "$PY_MAJOR" -eq 3 ] && [ "$PY_MINOR" -lt 11 ]; }; then
    echo "Error: Python 3.11+ required (found $PY_VERSION)"
    exit 1
fi

echo "1. Creating virtual environment..."
python3 -m venv .venv
source .venv/bin/activate

echo "2. Installing dependencies..."
pip install --quiet --upgrade pip
pip install --quiet -e .

echo "3. Creating workspace directory..."
mkdir -p ~/amanclaw-workspace

echo "4. Setting up config files..."
if [ ! -f config.yaml ]; then
    cp config.example.yaml config.yaml
    echo "   Created config.yaml from template — edit it with your settings."
else
    echo "   config.yaml already exists, skipping."
fi

if [ ! -f .env ]; then
    cp .env.example .env
    echo "   Created .env from template — add your secrets there."
else
    echo "   .env already exists, skipping."
fi

# Set secure permissions on secret files
chmod 600 .env
[ -f config.yaml ] && chmod 600 config.yaml

echo ""
echo "=== Setup Complete ==="
echo ""
echo "Next steps:"
echo ""
echo "  1. Add your secrets to .env:"
echo "     TELEGRAM_BOT_TOKEN=your-token-here"
echo "     LLM_API_KEY=your-key-here"
echo ""
echo "  2. Edit config.yaml with your LLM endpoint and admin user ID."
echo ""
echo "  3. Run the bot:"
echo "     source .venv/bin/activate"
echo "     python -m amanclaw"
echo ""
echo "  4. Send /myid to your bot on Telegram to get your user ID,"
echo "     then add it to config.yaml under admin_users.telegram."
echo ""
