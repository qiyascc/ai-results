#!/bin/bash

# Qiyas CMS - VDS Deployment Script
# Usage: ./deploy.sh [environment]
# Example: ./deploy.sh production

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Configuration
ENVIRONMENT=${1:-production}
APP_DIR="/var/www/qiyas"
BACKUP_DIR="/var/backups/qiyas"
REPO_URL="git@github.com:your-username/qiyas.git"
BRANCH="main"

log() {
    echo -e "${GREEN}[$(date +'%Y-%m-%d %H:%M:%S')]${NC} $1"
}

warn() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

error() {
    echo -e "${RED}[ERROR]${NC} $1"
    exit 1
}

# Check if running as root
if [ "$EUID" -ne 0 ]; then
    error "Please run as root or with sudo"
fi

log "Starting deployment for environment: $ENVIRONMENT"

# Create directories
mkdir -p $APP_DIR
mkdir -p $BACKUP_DIR

# Backup current deployment
if [ -d "$APP_DIR/.next" ]; then
    log "Creating backup..."
    BACKUP_NAME="backup_$(date +'%Y%m%d_%H%M%S').tar.gz"
    tar -czf "$BACKUP_DIR/$BACKUP_NAME" -C "$APP_DIR" . 2>/dev/null || true
    
    # Keep only last 5 backups
    ls -t $BACKUP_DIR/backup_*.tar.gz 2>/dev/null | tail -n +6 | xargs -r rm
    log "Backup created: $BACKUP_NAME"
fi

# Navigate to app directory
cd $APP_DIR

# Pull latest code
if [ -d ".git" ]; then
    log "Pulling latest changes..."
    git fetch origin
    git reset --hard origin/$BRANCH
else
    log "Cloning repository..."
    git clone --branch $BRANCH $REPO_URL .
fi

# Install pnpm if not installed
if ! command -v pnpm &> /dev/null; then
    log "Installing pnpm..."
    npm install -g pnpm
fi

# Install dependencies
log "Installing dependencies..."
pnpm install --frozen-lockfile

# Generate Prisma client
log "Generating Prisma client..."
pnpm prisma generate

# Run database migrations
log "Running database migrations..."
pnpm prisma migrate deploy

# Build application
log "Building application..."
pnpm build

# Build socket server
log "Building socket server..."
cd apps/socket-server
pnpm install --frozen-lockfile
pnpm build
cd $APP_DIR

# Restart services with PM2
log "Restarting services..."

# Check if PM2 is installed
if ! command -v pm2 &> /dev/null; then
    log "Installing PM2..."
    npm install -g pm2
fi

# Start/restart Next.js app
if pm2 list | grep -q "qiyas-app"; then
    pm2 restart qiyas-app
else
    pm2 start node --name "qiyas-app" -- server.js
fi

# Start/restart Socket server
if pm2 list | grep -q "qiyas-socket"; then
    pm2 restart qiyas-socket
else
    pm2 start apps/socket-server/dist/index.js --name "qiyas-socket"
fi

# Save PM2 configuration
pm2 save

# Reload nginx
log "Reloading nginx..."
nginx -t && systemctl reload nginx

# Health check
log "Running health check..."
sleep 5

HTTP_STATUS=$(curl -s -o /dev/null -w "%{http_code}" http://localhost:3000/api/v1/health)

if [ "$HTTP_STATUS" == "200" ]; then
    log "✅ Deployment successful! Application is healthy."
else
    warn "Health check returned status: $HTTP_STATUS"
    warn "Please check the application logs: pm2 logs qiyas-app"
fi

# Show status
pm2 status

log "Deployment completed!"
