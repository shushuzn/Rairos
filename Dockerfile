# ==============================================================================
# Rairos — Rust Research OS
# Multi-stage build for minimal runtime image
# ==============================================================================

# ─── Stage 1: Builder ────────────────────────────────────────────────────────
FROM rust:1.87-slim AS builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    pkg-config \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Cache cargo registry and build dependencies
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY audit.toml ./

# Pre-build dependency crates (faster incremental builds)
RUN mkdir -p crates && \
    cargo fetch --locked 2>/dev/null || true

# Copy source
COPY . .

# Build all binary targets
RUN cargo build --release --bin rairos-cli --bin rairos-web --bin rairos-mcp

# ==============================================================================
# Stage 2: Runtime — rairos-cli
# ==============================================================================
FROM debian:bookworm-slim AS rairos-cli

RUN apt-get update && apt-get install -y --no-install-recommends \
    libssl3 \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Data directory
RUN mkdir -p /data /home/rairos/.rairos && \
    useradd -m -u 1000 rairos && \
    chown -R rairos:rairos /app /data /home/rairos

COPY --from=builder /app/target/release/rairos-cli /usr/local/bin/rairos

USER rairos
ENV RAIROS_DATA_DIR=/data
ENV RAIROS_HOME_DIR=/home/rairos/.rairos

ENTRYPOINT ["rairos"]
CMD ["--help"]

# ==============================================================================
# Stage 3: Runtime — rairos-web
# ==============================================================================
FROM debian:bookworm-slim AS rairos-web

RUN apt-get update && apt-get install -y --no-install-recommends \
    libssl3 \
    ca-certificates \
    curl \
    tesseract-ocr \
    tesseract-ocr-eng \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

RUN mkdir -p /data /home/rairos/.rairos && \
    useradd -m -u 1000 rairos && \
    chown -R rairos:rairos /app /data /home/rairos

EXPOSE 8501 11434

COPY --from=builder /app/target/release/rairos-web /usr/local/bin/rairos-web
COPY --from=builder /app/target/release/rairos-mcp /usr/local/bin/rairos-mcp

USER rairos
ENV RAIROS_DATA_DIR=/data
ENV RAIROS_HOME_DIR=/home/rairos/.rairos

ENTRYPOINT ["rairos-web"]
CMD ["--host", "0.0.0.0", "--port", "8501"]
