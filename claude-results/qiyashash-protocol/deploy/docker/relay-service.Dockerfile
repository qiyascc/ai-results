# QiyasHash Relay Service Dockerfile
FROM rust:1.75-bookworm AS builder

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
COPY services/ services/

RUN cargo build --release -p relay-service

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

RUN useradd -r -u 1001 -g root qiyashash
COPY --from=builder /app/target/release/relay-service /usr/local/bin/
RUN mkdir -p /data /certs && chown -R qiyashash:root /data /certs

USER qiyashash

# QUIC port
EXPOSE 4433

HEALTHCHECK --interval=30s --timeout=10s --start-period=10s --retries=3 \
    CMD curl -f http://localhost:4433/health || exit 1

ENTRYPOINT ["relay-service"]
CMD ["--storage-path", "/data"]
