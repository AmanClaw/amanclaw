#!/bin/bash
set -euo pipefail

# Deploy AmanClaw to Raspberry Pi via Docker
RASPI_HOST="aman@192.168.1.116"
RASPI_PASS="p@ssw0rd@m@n"
DEPLOY_DIR="/home/aman/amanclaw-docker"
SSH="sshpass -p '$RASPI_PASS' ssh -o StrictHostKeyChecking=no $RASPI_HOST"
SCP="sshpass -p '$RASPI_PASS' scp -o StrictHostKeyChecking=no"

echo "==> Preparing deployment package..."
TMPDIR=$(mktemp -d)
trap "rm -rf $TMPDIR" EXIT

# Copy Rust source (exclude target dir and git)
rsync -a --exclude='target' --exclude='.git' --exclude='node_modules' \
    rust/ "$TMPDIR/rust-src/"

# Copy Python plugins
mkdir -p "$TMPDIR/rust-src/python-plugins"
cp plugins/*.py "$TMPDIR/rust-src/python-plugins/" 2>/dev/null || true

# Copy Python SDK
cp -r rust/sdks/python/amanclaw_sdk "$TMPDIR/rust-src/python-plugins/" 2>/dev/null || true

echo "==> Creating deployment docker-compose on Pi..."
eval $SSH "mkdir -p $DEPLOY_DIR"

# Transfer Rust source for building
echo "==> Uploading source to Pi (this may take a moment)..."
eval $SCP -r "$TMPDIR/rust-src" "$RASPI_HOST:$DEPLOY_DIR/rust-src"

# Create Dockerfile on Pi
eval $SSH "cat > $DEPLOY_DIR/Dockerfile" << 'DOCKERFILE'
FROM rust:1.85-slim AS builder
WORKDIR /app
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*
COPY rust-src/ .
RUN cargo build --release -p amanclaw-cli

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y \
    ca-certificates \
    python3 \
    && rm -rf /var/lib/apt/lists/*
RUN useradd --system --create-home amanclaw
WORKDIR /home/amanclaw
COPY --from=builder /app/target/release/amanclaw /usr/local/bin/amanclaw
COPY rust-src/python-plugins/ /home/amanclaw/plugins/
RUN mkdir -p data && chown -R amanclaw:amanclaw /home/amanclaw
USER amanclaw
EXPOSE 8443
CMD ["amanclaw"]
DOCKERFILE

# Create docker-compose.yml on Pi
eval $SSH "cat > $DEPLOY_DIR/docker-compose.yml" << 'COMPOSEFILE'
services:
  amanclaw:
    build: .
    container_name: amanclaw
    restart: unless-stopped
    env_file: /home/aman/amanclaw/.env
    network_mode: host
    volumes:
      - /home/aman/amanclaw/config.yaml:/home/amanclaw/config.yaml:ro
      - /home/aman/amanclaw/data:/home/amanclaw/data
    security_opt:
      - no-new-privileges:true
    cap_drop:
      - ALL
    read_only: true
    tmpfs:
      - /tmp:noexec,nosuid,size=50M
    mem_limit: 512m
    cpus: "1.0"
COMPOSEFILE

echo "==> Stopping existing amanclaw service..."
eval $SSH "sudo systemctl stop amanclaw 2>/dev/null || true"
eval $SSH "cd $DEPLOY_DIR && docker compose down 2>/dev/null || true"

echo "==> Building Docker image on Pi (this will take a while on first build)..."
eval $SSH "cd $DEPLOY_DIR && docker compose build"

echo "==> Starting container..."
eval $SSH "cd $DEPLOY_DIR && docker compose up -d"

echo "==> Checking status..."
sleep 3
eval $SSH "docker ps --filter name=amanclaw"

echo ""
echo "==> Deployment complete!"
echo "    View logs: ssh $RASPI_HOST 'docker logs -f amanclaw'"
