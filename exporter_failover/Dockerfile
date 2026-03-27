# Multi-stage build for the exporter service
FROM rust:1.85-bookworm AS builder

# Install cmake for rdkafka cmake-build feature
RUN apt-get update && apt-get install -y cmake build-essential libssl-dev pkg-config && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy manifests
COPY Cargo.toml Cargo.lock* ./

# Create a dummy main.rs to cache dependency compilation
RUN mkdir src && echo 'fn main() { println!("placeholder"); }' > src/main.rs
RUN cargo build --release 2>/dev/null || true

# Copy actual source
COPY src/ src/

# Touch main.rs to force rebuild with real source
RUN touch src/main.rs

# Build the actual binary
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/exporter_failover /usr/local/bin/exporter

# Health check
HEALTHCHECK --interval=10s --timeout=3s --retries=3 \
    CMD curl -f http://localhost:${HTTP_PORT:-9090}/health || exit 1

ENTRYPOINT ["/usr/local/bin/exporter"]
