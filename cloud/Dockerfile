# Stage 1: Build dashboard (for embedded SPA)
FROM node:20-slim AS dashboard-builder
WORKDIR /app/dashboard
COPY dashboard/package*.json ./
RUN npm ci
COPY dashboard/ ./
RUN npm run build

# Stage 2: Build cloud binary
FROM rust:1.88-slim AS builder
RUN apt-get update && apt-get install -y pkg-config libssl-dev
WORKDIR /app
COPY rust/ rust/
COPY cloud/ cloud/
COPY --from=dashboard-builder /app/dashboard/dist rust/crates/amanclaw-api/dashboard-dist/
WORKDIR /app/cloud
RUN cargo build --release

# Stage 3: Runtime
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates curl python3 && rm -rf /var/lib/apt/lists/*
RUN useradd -m -s /bin/bash amanclaw
COPY --from=builder /app/cloud/target/release/amanclaw-cloud /usr/local/bin/
COPY plugins/ /app/plugins/
COPY rust/sdks/python/ /app/sdks/python/
USER amanclaw
WORKDIR /app
EXPOSE 8443
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 CMD curl -sf http://localhost:8443/api/cloud/tenant || exit 1
CMD ["amanclaw-cloud", "serve", "--port", "8443"]
