# QiyasHash Encryption Service Dockerfile
FROM rust:1.75-bookworm AS builder

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
COPY services/ services/

RUN cargo build --release -p encryption-service

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

RUN useradd -r -u 1001 -g root qiyashash
COPY --from=builder /app/target/release/encryption-service /usr/local/bin/
RUN mkdir -p /data && chown qiyashash:root /data

USER qiyashash
EXPOSE 8082

HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8082/api/v1/health || exit 1

ENTRYPOINT ["encryption-service"]
CMD ["--host", "0.0.0.0", "--port", "8082", "--storage-path", "/data"]
