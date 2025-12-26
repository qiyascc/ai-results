# QiyasHash DHT Peer Service Dockerfile
FROM rust:1.75-bookworm AS builder

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
COPY services/ services/

RUN cargo build --release -p dht-peer-service

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

RUN useradd -r -u 1001 -g root qiyashash
COPY --from=builder /app/target/release/dht-peer-service /usr/local/bin/
RUN mkdir -p /data && chown qiyashash:root /data

USER qiyashash

# DHT TCP and UDP ports
EXPOSE 4001 4001/udp

HEALTHCHECK --interval=30s --timeout=10s --start-period=10s --retries=3 \
    CMD curl -f http://localhost:4001/api/v1/health || exit 1

ENTRYPOINT ["dht-peer-service"]
CMD ["--storage-path", "/data"]
