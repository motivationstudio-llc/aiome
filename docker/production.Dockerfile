# ==========================================
# Aiome Production Dockerfile Template
# Features: Rootless, Read-only compatible, Hardened
# ==========================================

# --- Build Stage ---
FROM rust:1.80-slim-bookworm AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    git \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy workspace files
COPY . .

# Build target
ARG BIN_NAME=command-center
RUN cargo build --release --bin ${BIN_NAME}

# --- Runtime Stage ---
FROM debian:bookworm-slim

ARG BIN_NAME=command-center
ENV BIN_NAME=${BIN_NAME}

# Labels for security visibility
LABEL org.opencontainers.image.authors="motivationstudio,LLC" \
    security.rootless="true" \
    security.readonly="true" \
    security.no-docker-socket="true"

# 1. Create a non-privileged service user
RUN groupadd -g 10001 aiome && \
    useradd -u 10001 -g aiome -m -s /bin/false aiome

# 2. Hardening: Install only necessary CA certs and runtime libraries
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# 3. Copy binary from builder
COPY --from=builder /app/target/release/${BIN_NAME} /app/aiome-app
RUN chown root:root /app/aiome-app && chmod 555 /app/aiome-app

# 4. Prepare a writable data directory
RUN mkdir -p /app/data && chown aiome:aiome /app/data && chmod 700 /app/data

# 5. Security: Set the unprivileged user
USER aiome

# 6. Environment Sanity
ENV RUST_LOG=info \
    AIOME_DATA_DIR=/app/data \
    TMPDIR=/tmp

# Standard ports (Command Center uses 8080, Key Proxy uses 9999 - mapped via compose)
EXPOSE 8080 9999

# Execution
ENTRYPOINT ["/app/aiome-app"]
