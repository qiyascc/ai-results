# QiyasHash Deployment Guide

This guide covers deploying QiyasHash services in production environments.

## Table of Contents

1. [Prerequisites](#prerequisites)
2. [Docker Deployment](#docker-deployment)
3. [Kubernetes Deployment](#kubernetes-deployment)
4. [Manual Deployment](#manual-deployment)
5. [Configuration](#configuration)
6. [Security Hardening](#security-hardening)
7. [Monitoring](#monitoring)
8. [Backup & Recovery](#backup--recovery)

---

## Prerequisites

### System Requirements

| Component | Minimum | Recommended |
|-----------|---------|-------------|
| CPU | 2 cores | 4+ cores |
| RAM | 4 GB | 8+ GB |
| Storage | 20 GB SSD | 100+ GB SSD |
| Network | 100 Mbps | 1 Gbps |

### Software Requirements

- Docker 24.0+ or Podman 4.0+
- Docker Compose 2.20+ (for compose deployments)
- OpenSSL 3.0+ (for TLS certificates)
- Rust 1.75+ (for building from source)

---

## Docker Deployment

### Quick Start

```bash
# Clone repository
git clone https://github.com/qiyascc/qiyashashchat.git
cd qiyashashchat

# Build images
docker compose build

# Start services
docker compose up -d

# Check status
docker compose ps
```

### Docker Compose Configuration

```yaml
# docker-compose.yml
version: '3.8'

services:
  identity-service:
    build:
      context: .
      dockerfile: deploy/docker/identity-service.Dockerfile
    ports:
      - "8081:8081"
    volumes:
      - identity-data:/data
    environment:
      - RUST_LOG=info
      - STORAGE_PATH=/data
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8081/api/v1/health"]
      interval: 30s
      timeout: 10s
      retries: 3

  encryption-service:
    build:
      context: .
      dockerfile: deploy/docker/encryption-service.Dockerfile
    ports:
      - "8082:8082"
    volumes:
      - encryption-data:/data
    environment:
      - RUST_LOG=info
      - STORAGE_PATH=/data
    restart: unless-stopped

  dht-peer-service:
    build:
      context: .
      dockerfile: deploy/docker/dht-peer-service.Dockerfile
    ports:
      - "4001:4001"
      - "4001:4001/udp"
    volumes:
      - dht-data:/data
    environment:
      - RUST_LOG=info
      - BOOTSTRAP_NODES=
    restart: unless-stopped

  relay-service:
    build:
      context: .
      dockerfile: deploy/docker/relay-service.Dockerfile
    ports:
      - "4433:4433"
    volumes:
      - relay-data:/data
      - ./certs:/certs:ro
    environment:
      - RUST_LOG=info
      - TLS_CERT=/certs/relay.crt
      - TLS_KEY=/certs/relay.key
    restart: unless-stopped

  web-client:
    build:
      context: ./clients/web
      dockerfile: Dockerfile
    ports:
      - "3000:80"
    restart: unless-stopped

volumes:
  identity-data:
  encryption-data:
  dht-data:
  relay-data:
```

### Building Docker Images

```dockerfile
# deploy/docker/identity-service.Dockerfile
FROM rust:1.75-bookworm AS builder

WORKDIR /app
COPY . .
RUN cargo build --release -p identity-service

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/identity-service /usr/local/bin/

EXPOSE 8081
CMD ["identity-service", "--host", "0.0.0.0", "--port", "8081"]
```

---

## Kubernetes Deployment

### Helm Chart

```bash
# Add helm repo (when available)
helm repo add qiyashash https://charts.qiyashash.dev

# Install
helm install qiyashash qiyashash/qiyashash-stack \
  --namespace qiyashash \
  --create-namespace \
  --values values.yaml
```

### Manual Kubernetes Deployment

```yaml
# deploy/k8s/identity-service.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: identity-service
  namespace: qiyashash
spec:
  replicas: 3
  selector:
    matchLabels:
      app: identity-service
  template:
    metadata:
      labels:
        app: identity-service
    spec:
      containers:
        - name: identity-service
          image: ghcr.io/qiyascc/qiyashash-identity:latest
          ports:
            - containerPort: 8081
          env:
            - name: RUST_LOG
              value: "info"
            - name: STORAGE_PATH
              value: "/data"
          volumeMounts:
            - name: data
              mountPath: /data
          resources:
            requests:
              memory: "256Mi"
              cpu: "100m"
            limits:
              memory: "512Mi"
              cpu: "500m"
          livenessProbe:
            httpGet:
              path: /api/v1/health
              port: 8081
            initialDelaySeconds: 10
            periodSeconds: 30
          readinessProbe:
            httpGet:
              path: /api/v1/health
              port: 8081
            initialDelaySeconds: 5
            periodSeconds: 10
      volumes:
        - name: data
          persistentVolumeClaim:
            claimName: identity-data

---
apiVersion: v1
kind: Service
metadata:
  name: identity-service
  namespace: qiyashash
spec:
  selector:
    app: identity-service
  ports:
    - port: 8081
      targetPort: 8081
  type: ClusterIP
```

---

## Manual Deployment

### Building from Source

```bash
# Install dependencies (Ubuntu/Debian)
sudo apt-get update
sudo apt-get install -y build-essential pkg-config libssl-dev librocksdb-dev

# Build all services
cargo build --release

# Install binaries
sudo install -m 755 target/release/identity-service /usr/local/bin/
sudo install -m 755 target/release/encryption-service /usr/local/bin/
sudo install -m 755 target/release/dht-peer-service /usr/local/bin/
sudo install -m 755 target/release/relay-service /usr/local/bin/
```

### Systemd Service Files

```ini
# /etc/systemd/system/qiyashash-identity.service
[Unit]
Description=QiyasHash Identity Service
After=network.target

[Service]
Type=simple
User=qiyashash
Group=qiyashash
ExecStart=/usr/local/bin/identity-service --host 0.0.0.0 --port 8081 --storage-path /var/lib/qiyashash/identity
Restart=always
RestartSec=5
Environment=RUST_LOG=info

# Security hardening
NoNewPrivileges=true
ProtectSystem=strict
ProtectHome=true
PrivateTmp=true
ReadWritePaths=/var/lib/qiyashash/identity

[Install]
WantedBy=multi-user.target
```

```bash
# Enable and start services
sudo systemctl daemon-reload
sudo systemctl enable qiyashash-identity
sudo systemctl start qiyashash-identity
sudo systemctl status qiyashash-identity
```

---

## Configuration

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `RUST_LOG` | Log level (trace, debug, info, warn, error) | info |
| `STORAGE_PATH` | Data storage path | ./data |
| `TLS_CERT` | TLS certificate path | - |
| `TLS_KEY` | TLS private key path | - |
| `BOOTSTRAP_NODES` | DHT bootstrap nodes (comma-separated) | - |
| `RELAY_NODES` | Relay nodes (comma-separated) | - |

### Configuration File

```toml
# /etc/qiyashash/config.toml

[server]
host = "0.0.0.0"
port = 8081

[storage]
path = "/var/lib/qiyashash/identity"
max_size_gb = 10

[tls]
cert_path = "/etc/qiyashash/certs/server.crt"
key_path = "/etc/qiyashash/certs/server.key"

[logging]
level = "info"
format = "json"

[metrics]
enabled = true
port = 9090
```

---

## Security Hardening

### TLS Configuration

```bash
# Generate TLS certificates
openssl req -x509 -newkey rsa:4096 -keyout server.key -out server.crt \
  -days 365 -nodes -subj "/CN=qiyashash.local"

# For production, use Let's Encrypt
certbot certonly --standalone -d relay.yourdomain.com
```

### Firewall Rules

```bash
# UFW (Ubuntu)
sudo ufw allow 8081/tcp  # Identity service
sudo ufw allow 8082/tcp  # Encryption service
sudo ufw allow 4001/tcp  # DHT
sudo ufw allow 4001/udp  # DHT
sudo ufw allow 4433/tcp  # Relay (QUIC)

# iptables
iptables -A INPUT -p tcp --dport 8081 -j ACCEPT
iptables -A INPUT -p tcp --dport 4001 -j ACCEPT
iptables -A INPUT -p udp --dport 4001 -j ACCEPT
```

### SELinux Policy (RHEL/CentOS)

```bash
# Create custom SELinux module
cat > qiyashash.te << 'EOF'
module qiyashash 1.0;

require {
    type qiyashash_t;
    type qiyashash_port_t;
}

allow qiyashash_t qiyashash_port_t:tcp_socket { name_bind name_connect };
EOF

checkmodule -M -m -o qiyashash.mod qiyashash.te
semodule_package -o qiyashash.pp -m qiyashash.mod
semodule -i qiyashash.pp
```

---

## Monitoring

### Prometheus Metrics

```yaml
# prometheus.yml
scrape_configs:
  - job_name: 'qiyashash-identity'
    static_configs:
      - targets: ['localhost:9090']
    metrics_path: /metrics
```

### Grafana Dashboard

Import the dashboard from `deploy/grafana/qiyashash-dashboard.json`.

### Health Checks

```bash
# Check all services
curl http://localhost:8081/api/v1/health
curl http://localhost:8082/api/v1/health

# Check DHT peers
curl http://localhost:4001/api/v1/peers
```

---

## Backup & Recovery

### Backup Script

```bash
#!/bin/bash
# backup.sh

BACKUP_DIR="/var/backups/qiyashash"
DATE=$(date +%Y%m%d_%H%M%S)

# Stop services
systemctl stop qiyashash-*

# Backup data
tar -czf "${BACKUP_DIR}/qiyashash_${DATE}.tar.gz" /var/lib/qiyashash/

# Restart services
systemctl start qiyashash-identity
systemctl start qiyashash-encryption

echo "Backup completed: qiyashash_${DATE}.tar.gz"
```

### Recovery

```bash
#!/bin/bash
# restore.sh

BACKUP_FILE=$1

if [ -z "$BACKUP_FILE" ]; then
    echo "Usage: restore.sh <backup_file>"
    exit 1
fi

# Stop services
systemctl stop qiyashash-*

# Restore data
tar -xzf "$BACKUP_FILE" -C /

# Start services
systemctl start qiyashash-identity
systemctl start qiyashash-encryption

echo "Restore completed from: $BACKUP_FILE"
```

---

## Troubleshooting

### Common Issues

1. **Service won't start**
   - Check logs: `journalctl -u qiyashash-identity -f`
   - Verify permissions on data directory
   - Check port availability

2. **DHT not connecting**
   - Verify firewall allows UDP on port 4001
   - Check bootstrap nodes are reachable
   - Enable debug logging: `RUST_LOG=debug`

3. **TLS errors**
   - Verify certificate chain
   - Check key permissions (should be 600)
   - Ensure hostname matches certificate CN/SAN

### Getting Help

- GitHub Issues: https://github.com/qiyascc/qiyashashchat/issues
- Documentation: https://docs.qiyashash.dev
