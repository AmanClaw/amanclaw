FROM rust:1.85-slim AS builder
WORKDIR /app
COPY . .
RUN cargo build --release -p amanclaw-cli

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
RUN useradd --system --create-home amanclaw
USER amanclaw
WORKDIR /home/amanclaw
COPY --from=builder /app/target/release/amanclaw /usr/local/bin/amanclaw
RUN mkdir -p plugins data
EXPOSE 8443
CMD ["amanclaw"]
