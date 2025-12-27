#!/bin/bash

# ===========================================
# Qiyas CMS - VDS Deployment Script
# ===========================================
#
# Usage: ./deploy.sh [domain] [email]
# Example: ./deploy.sh qiyas.cc admin@qiyas.cc
#
# This script:
# 1. Installs all dependencies (Node.js, pnpm, PM2, PostgreSQL, Redis, Nginx)
# 2. Clones/updates the repository
# 3. Builds the application
# 4. Sets up the database
# 5. Configures Nginx
# 6. Sets up SSL with Let's Encrypt
# 7. Starts the application with PM2

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log() { echo -e "${GREEN}[$(date +'%H:%M:%S')]${NC} $1"; }
warn() { echo -e "${YELLOW}[WARNING]${NC} $1"; }
error() { echo -e "${RED}[ERROR]${NC} $1"; exit 1; }
info() { echo -e "${BLUE}[INFO]${NC} $1"; }

# Configuration
DOMAIN=${1:-""}
EMAIL=${2:-""}
APP_DIR="/var/www/qiyas"
BACKUP_DIR="/var/backups/qiyas"
REPO_URL="https://github.com/YOUR_USERNAME/qiyas.git"
BRANCH="main"
NODE_VERSION="20"

# Check if running as root
if [ "$EUID" -ne 0 ]; then
    error "Please run as root: sudo $0 $DOMAIN $EMAIL"
fi

echo ""
echo "=============================================="
echo "   Qiyas CMS - Deployment Script"
echo "=============================================="
echo ""

if [ -z "$DOMAIN" ]; then
    warn "No domain specified. SSL setup will be skipped."
    info "To enable SSL later, run: ./deploy/setup-ssl.sh your-domain.com email@example.com"
fi

# =====================
# 1. System Update
# =====================
log "Updating system packages..."
apt update && apt upgrade -y

# =====================
# 2. Install Dependencies
# =====================
log "Installing required packages..."
apt install -y curl wget git build-essential nginx

# Install Node.js 20
if ! command -v node &> /dev/null || [[ $(node -v) != v${NODE_VERSION}* ]]; then
    log "Installing Node.js ${NODE_VERSION}..."
    curl -fsSL https://deb.nodesource.com/setup_${NODE_VERSION}.x | bash -
    apt install -y nodejs
fi

# Install pnpm
if ! command -v pnpm &> /dev/null; then
    log "Installing pnpm..."
    npm install -g pnpm
fi

# Install PM2
if ! command -v pm2 &> /dev/null; then
    log "Installing PM2..."
    npm install -g pm2
fi

# =====================
# 3. Install PostgreSQL
# =====================
if ! command -v psql &> /dev/null; then
    log "Installing PostgreSQL..."
    apt install -y postgresql postgresql-contrib
    systemctl enable postgresql
    systemctl start postgresql
fi

# Create database and user
log "Setting up database..."
DB_PASSWORD=$(openssl rand -base64 32 | tr -dc 'a-zA-Z0-9' | head -c 32)

sudo -u postgres psql -tc "SELECT 1 FROM pg_roles WHERE rolname='qiyas'" | grep -q 1 || \
    sudo -u postgres psql -c "CREATE USER qiyas WITH PASSWORD '${DB_PASSWORD}';"

sudo -u postgres psql -tc "SELECT 1 FROM pg_database WHERE datname='qiyas'" | grep -q 1 || \
    sudo -u postgres createdb -O qiyas qiyas

# =====================
# 4. Install Redis
# =====================
if ! command -v redis-cli &> /dev/null; then
    log "Installing Redis..."
    apt install -y redis-server
    
    # Configure Redis
    sed -i 's/^supervised no/supervised systemd/' /etc/redis/redis.conf
    sed -i 's/^# maxmemory .*/maxmemory 256mb/' /etc/redis/redis.conf
    sed -i 's/^# maxmemory-policy .*/maxmemory-policy allkeys-lru/' /etc/redis/redis.conf
    
    systemctl enable redis-server
    systemctl restart redis-server
fi

# =====================
# 5. Setup Application Directory
# =====================
log "Setting up application directory..."
mkdir -p $APP_DIR
mkdir -p $BACKUP_DIR
mkdir -p /var/log/qiyas

# Backup existing deployment
if [ -d "$APP_DIR/.next" ]; then
    log "Creating backup..."
    BACKUP_NAME="backup_$(date +'%Y%m%d_%H%M%S').tar.gz"
    tar -czf "$BACKUP_DIR/$BACKUP_NAME" -C "$APP_DIR" . 2>/dev/null || true
    ls -t $BACKUP_DIR/backup_*.tar.gz 2>/dev/null | tail -n +6 | xargs -r rm
fi

# Clone or update repository
cd $APP_DIR
if [ -d ".git" ]; then
    log "Updating repository..."
    git fetch origin
    git reset --hard origin/$BRANCH
else
    log "Cloning repository..."
    # If you have a private repo, you'll need to set up SSH keys
    # For now, we'll assume files are already present or use a public repo
    if [ ! -f "package.json" ]; then
        warn "Repository not found. Please clone manually or copy files to $APP_DIR"
        info "Expected: git clone $REPO_URL $APP_DIR"
    fi
fi

# =====================
# 6. Configure Environment
# =====================
log "Configuring environment..."
JWT_SECRET=$(openssl rand -base64 64 | tr -dc 'a-zA-Z0-9' | head -c 64)

if [ ! -f ".env" ]; then
    cat > .env << EOF
# Database
DATABASE_URL="postgresql://qiyas:${DB_PASSWORD}@localhost:5432/qiyas"

# Redis
REDIS_URL="redis://localhost:6379"

# Authentication
JWT_SECRET="${JWT_SECRET}"
JWT_EXPIRES_IN="7d"
JWT_REFRESH_EXPIRES_IN="30d"

# Application
NODE_ENV="production"
NEXT_PUBLIC_APP_URL="https://${DOMAIN:-localhost}"
NEXT_PUBLIC_SOCKET_URL="https://${DOMAIN:-localhost}"
CORS_ORIGINS="https://${DOMAIN:-localhost}"

# Socket Server
SOCKET_PORT=3001

# File Upload
MAX_FILE_SIZE=10485760
UPLOAD_DIR="./public/uploads"
EOF
    chmod 600 .env
    log "Environment file created"
else
    log "Environment file already exists"
fi

# =====================
# 7. Install Dependencies & Build
# =====================
log "Installing dependencies..."
pnpm install --frozen-lockfile

log "Generating Prisma client..."
pnpm prisma generate

log "Running database migrations..."
pnpm prisma migrate deploy

log "Seeding database..."
pnpm prisma db seed || warn "Seed may have already run"

log "Building application..."
pnpm build

# Build socket server
log "Building socket server..."
cd apps/socket-server
pnpm install --frozen-lockfile
pnpm build
cd $APP_DIR

# Create uploads directory
mkdir -p public/uploads
chmod 755 public/uploads

# =====================
# 8. Configure PM2
# =====================
log "Configuring PM2..."

cat > ecosystem.config.cjs << 'EOF'
module.exports = {
  apps: [
    {
      name: 'qiyas-app',
      script: 'node',
      args: 'server.js',
      cwd: '/var/www/qiyas',
      instances: 'max',
      exec_mode: 'cluster',
      autorestart: true,
      watch: false,
      max_memory_restart: '1G',
      env: {
        NODE_ENV: 'production',
        PORT: 3000,
      },
      error_file: '/var/log/qiyas/app-error.log',
      out_file: '/var/log/qiyas/app-out.log',
      merge_logs: true,
      log_date_format: 'YYYY-MM-DD HH:mm:ss',
    },
    {
      name: 'qiyas-socket',
      script: 'dist/index.js',
      cwd: '/var/www/qiyas/apps/socket-server',
      instances: 1,
      autorestart: true,
      watch: false,
      max_memory_restart: '512M',
      env: {
        NODE_ENV: 'production',
        SOCKET_PORT: 3001,
      },
      error_file: '/var/log/qiyas/socket-error.log',
      out_file: '/var/log/qiyas/socket-out.log',
      merge_logs: true,
      log_date_format: 'YYYY-MM-DD HH:mm:ss',
    },
  ],
}
EOF

# Start/restart with PM2
pm2 delete all 2>/dev/null || true
pm2 start ecosystem.config.cjs
pm2 save

# Setup PM2 startup
pm2 startup systemd -u root --hp /root
systemctl enable pm2-root

# =====================
# 9. Configure Nginx
# =====================
log "Configuring Nginx..."

# Remove default site
rm -f /etc/nginx/sites-enabled/default

# Copy nginx config
if [ -n "$DOMAIN" ]; then
    sed "s/YOUR_DOMAIN/${DOMAIN}/g" deploy/nginx/qiyas.conf > /etc/nginx/sites-available/qiyas.conf
else
    # Create a simple HTTP-only config for development
    cat > /etc/nginx/sites-available/qiyas.conf << 'NGINX'
upstream nextjs_backend {
    server 127.0.0.1:3000;
    keepalive 64;
}

upstream socket_backend {
    server 127.0.0.1:3001;
    keepalive 64;
}

server {
    listen 80;
    server_name _;
    client_max_body_size 50M;

    location /api {
        proxy_pass http://nextjs_backend;
        proxy_http_version 1.1;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }

    location /socket.io {
        proxy_pass http://socket_backend;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;
    }

    location / {
        proxy_pass http://nextjs_backend;
        proxy_http_version 1.1;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection 'upgrade';
    }
}
NGINX
fi

ln -sf /etc/nginx/sites-available/qiyas.conf /etc/nginx/sites-enabled/

nginx -t || error "Nginx configuration test failed"
systemctl enable nginx
systemctl restart nginx

# =====================
# 10. Setup SSL (if domain provided)
# =====================
if [ -n "$DOMAIN" ] && [ -n "$EMAIL" ]; then
    log "Setting up SSL..."
    chmod +x deploy/setup-ssl.sh
    ./deploy/setup-ssl.sh "$DOMAIN" "$EMAIL"
fi

# =====================
# 11. Configure Firewall
# =====================
log "Configuring firewall..."
if command -v ufw &> /dev/null; then
    ufw allow 22/tcp
    ufw allow 80/tcp
    ufw allow 443/tcp
    ufw --force enable
fi

# =====================
# 12. Health Check
# =====================
log "Running health check..."
sleep 5

HEALTH_URL="http://localhost:3000/api/v1/health"
HTTP_STATUS=$(curl -s -o /dev/null -w "%{http_code}" "$HEALTH_URL" || echo "000")

echo ""
echo "=============================================="
if [ "$HTTP_STATUS" = "200" ]; then
    echo -e "${GREEN}   Deployment Successful!${NC}"
else
    echo -e "${YELLOW}   Deployment Complete (Health check: ${HTTP_STATUS})${NC}"
fi
echo "=============================================="
echo ""
echo "Application URL:"
if [ -n "$DOMAIN" ]; then
    echo -e "  ${BLUE}https://${DOMAIN}${NC}"
else
    echo -e "  ${BLUE}http://YOUR_SERVER_IP${NC}"
fi
echo ""
echo "Admin Panel:"
if [ -n "$DOMAIN" ]; then
    echo -e "  ${BLUE}https://${DOMAIN}/az/admin${NC}"
else
    echo -e "  ${BLUE}http://YOUR_SERVER_IP/az/admin${NC}"
fi
echo ""
echo "Default Credentials:"
echo "  Email: admin@qiyas.cc"
echo "  Password: Admin123!"
echo ""
echo "Useful Commands:"
echo "  pm2 status              - Check app status"
echo "  pm2 logs qiyas-app      - View app logs"
echo "  pm2 logs qiyas-socket   - View socket logs"
echo "  pm2 restart all         - Restart all apps"
echo ""
if [ -z "$DOMAIN" ]; then
    echo -e "${YELLOW}Note: SSL is not configured. Run the following to enable HTTPS:${NC}"
    echo "  ./deploy/setup-ssl.sh your-domain.com your-email@example.com"
    echo ""
fi

log "Done!"
