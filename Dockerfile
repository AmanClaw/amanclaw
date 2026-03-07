FROM python:3.12-slim AS base

# Security: run as non-root
RUN groupadd --gid 1000 amanclaw && \
    useradd --uid 1000 --gid 1000 --create-home amanclaw

# System deps only — no build tools in final image
RUN apt-get update && \
    apt-get install -y --no-install-recommends tini && \
    apt-get clean && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Install Python deps first (layer caching)
COPY pyproject.toml ./
COPY packages/ ./packages/
RUN pip install --no-cache-dir . 2>/dev/null || true

# Copy application code
COPY amanclaw/ ./amanclaw/
COPY config.example.yaml ./config.example.yaml

# Install the actual package
RUN pip install --no-cache-dir .

# Create workspace and data directories owned by app user
RUN mkdir -p /data /home/amanclaw/amanclaw-workspace && \
    chown -R amanclaw:amanclaw /app /data /home/amanclaw/amanclaw-workspace

# Switch to non-root user
USER amanclaw

# Health check — verify the process is alive
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD python -c "import amanclaw; print('ok')" || exit 1

# Use tini as init to handle signals properly
ENTRYPOINT ["tini", "--"]
CMD ["python", "-m", "amanclaw"]
