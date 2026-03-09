#!/bin/bash
set -euo pipefail

# Deploy AmanClaw to Raspberry Pi
# Strategy: Cross-compile on Mac via docker buildx, transfer image to Pi
RASPI_HOST="aman@192.168.1.116"
RASPI_PASS="p@ssw0rd@m@n"
DEPLOY_DIR="/home/aman/amanclaw-docker"
IMAGE_TAR="/tmp/amanclaw-arm64.tar"

SSH="sshpass -p '$RASPI_PASS' ssh -o StrictHostKeyChecking=no $RASPI_HOST"
SCP="sshpass -p '$RASPI_PASS' scp -o StrictHostKeyChecking=no"

# --- Step 1: Cross-compile Docker image for ARM64 ---
echo "==> Building Docker image for linux/arm64 (cross-compile on Mac)..."
docker buildx build \
    --platform linux/arm64 \
    --builder multiarch \
    -t amanclaw:latest \
    --output type=docker,dest="$IMAGE_TAR" \
    -f rust/Dockerfile \
    rust/

echo "==> Image built: $(du -h "$IMAGE_TAR" | cut -f1)"

# --- Step 2: Transfer image to Pi ---
echo "==> Transferring image to Pi..."
eval $SCP "$IMAGE_TAR" "$RASPI_HOST:/tmp/amanclaw-arm64.tar"

# --- Step 3: Load image and deploy ---
echo "==> Loading image and deploying on Pi..."
eval $SSH "
docker load < /tmp/amanclaw-arm64.tar
rm -f /tmp/amanclaw-arm64.tar

# Create docker-compose
mkdir -p $DEPLOY_DIR
cat > $DEPLOY_DIR/docker-compose.yml << 'COMPOSEFILE'
services:
  amanclaw:
    image: amanclaw:latest
    container_name: amanclaw
    restart: unless-stopped
    env_file: /home/aman/amanclaw/.env
    network_mode: host
    volumes:
      - /home/aman/amanclaw/config.yaml:/home/amanclaw/config.yaml:ro
      - /home/aman/amanclaw/data:/home/amanclaw/data
      - /home/aman/amanclaw/plugins:/home/amanclaw/plugins:ro
      - /home/aman/amanclaw/memory.db:/home/amanclaw/memory.db
    security_opt:
      - no-new-privileges:true
    cap_drop:
      - ALL
    tmpfs:
      - /tmp:noexec,nosuid,size=50M
    mem_limit: 512m
    cpus: \"1.0\"
COMPOSEFILE

cd $DEPLOY_DIR && docker compose up -d
"

# --- Step 4: Verify ---
echo "==> Checking status..."
sleep 3
eval $SSH "docker ps --filter name=amanclaw --format 'table {{.Names}}\t{{.Image}}\t{{.Status}}'"

# Cleanup local tar
rm -f "$IMAGE_TAR"

echo ""
echo "==> Deployment complete!"
echo "    View logs: ssh $RASPI_HOST 'docker logs -f amanclaw'"
echo "    WA bridge: ssh $RASPI_HOST 'sudo journalctl -u wa-bridge -f'"
