FROM rust:1.85-slim AS builder

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY raft_core ./raft_core
COPY raft_kv   ./raft_kv

RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates bash iproute2 iputils-ping procps iptables curl \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/raft-server /usr/local/bin/
COPY --from=builder /app/target/release/raft-client /usr/local/bin/
COPY --from=builder /app/target/release/raft-test-client /usr/local/bin/

ENTRYPOINT ["raft-server"]
