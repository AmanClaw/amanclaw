# Stage 1: Build Svelte dashboard
FROM node:20-slim AS dashboard-builder
WORKDIR /dashboard
COPY apps/dashboard/package*.json ./
RUN npm ci
COPY apps/dashboard/ .
RUN npm run build

# Stage 2: Build Rust binary (with embedded dashboard)
FROM rust:1.88-slim AS builder
WORKDIR /app
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*
COPY Cargo.toml Cargo.lock ./
COPY crates/ ./crates/
COPY apps/cli/ ./apps/cli/
COPY apps/cloud/ ./apps/cloud/
COPY skills/ ./skills/
COPY channels/ ./channels/
COPY wit/ ./wit/
COPY --from=dashboard-builder /dashboard/dist ./apps/dashboard/dist
RUN cargo build --release -p amanclaw-cli

# Stage 3: Runtime
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    python3 \
    python3-pip \
    python3-venv \
    && curl -fsSL https://deb.nodesource.com/setup_20.x | bash - \
    && apt-get install -y nodejs \
    && rm -rf /var/lib/apt/lists/*
# Install uvx (for uvx-based MCP servers)
RUN pip3 install --break-system-packages uv
RUN useradd --system --create-home amanclaw
WORKDIR /home/amanclaw
COPY --from=builder /app/target/release/amanclaw /usr/local/bin/amanclaw
COPY sdks/python/ /tmp/amanclaw-sdk/
RUN pip3 install --break-system-packages /tmp/amanclaw-sdk/ && rm -rf /tmp/amanclaw-sdk/
RUN mkdir -p plugins data .npm .cache
RUN chown -R amanclaw:amanclaw /home/amanclaw
USER amanclaw
EXPOSE 8443
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
  CMD curl -sf http://localhost:8443/health || exit 1
CMD ["amanclaw"]
