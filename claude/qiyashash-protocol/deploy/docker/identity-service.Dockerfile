# QiyasHash Identity Service Dockerfile
# Multi-stage build for minimal image size

# Build stage
FROM rust:1.75-bookworm AS builder

WORKDIR /app

# Copy manifests
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
COPY services/ services/

# Build release binary
RUN cargo build --release -p identity-service

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -r -u 1001 -g root qiyashash

# Copy binary from builder
COPY --from=builder /app/target/release/identity-service /usr/local/bin/

# Create data directory
RUN mkdir -p /data && chown qiyashash:root /data

# Switch to non-root user
USER qiyashash

# Expose port
EXPOSE 8081

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8081/api/v1/health || exit 1

# Run service
ENTRYPOINT ["identity-service"]
CMD ["--host", "0.0.0.0", "--port", "8081", "--storage-path", "/data"]
