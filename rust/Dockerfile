# Stage 1: Build Svelte dashboard
FROM node:20-slim AS dashboard-builder
WORKDIR /dashboard
COPY dashboard/package*.json ./
RUN npm ci
COPY dashboard/ .
RUN npm run build

# Stage 2: Build Rust binary (with embedded dashboard)
FROM rust:1.88-slim AS builder
WORKDIR /app
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*
COPY rust/ ./rust/
COPY --from=dashboard-builder /dashboard/dist ./dashboard/dist
WORKDIR /app/rust
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
COPY --from=builder /app/rust/target/release/amanclaw /usr/local/bin/amanclaw
COPY rust/sdks/python/ /tmp/amanclaw-sdk/
RUN pip3 install --break-system-packages /tmp/amanclaw-sdk/ && rm -rf /tmp/amanclaw-sdk/
RUN mkdir -p plugins data .npm .cache
RUN chown -R amanclaw:amanclaw /home/amanclaw
USER amanclaw
EXPOSE 8443
CMD ["amanclaw"]
