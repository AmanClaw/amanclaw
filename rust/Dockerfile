FROM rust:1.85-slim AS builder
WORKDIR /app
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*
COPY . .
RUN cargo build --release -p amanclaw-cli

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y \
    ca-certificates \
    python3 \
    python3-pip \
    python3-venv \
    && rm -rf /var/lib/apt/lists/*
RUN useradd --system --create-home amanclaw
WORKDIR /home/amanclaw
COPY --from=builder /app/target/release/amanclaw /usr/local/bin/amanclaw
RUN mkdir -p plugins data
RUN chown -R amanclaw:amanclaw /home/amanclaw
USER amanclaw
EXPOSE 8443
CMD ["amanclaw"]
