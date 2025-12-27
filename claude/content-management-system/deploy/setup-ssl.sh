#!/bin/bash

# ===========================================
# Qiyas CMS - SSL Certificate Setup Script
# ===========================================
#
# This script sets up Let's Encrypt SSL certificates
# Run as root: sudo ./setup-ssl.sh yourdomain.com email@example.com
#
# Prerequisites:
# - Domain pointing to this server
# - Nginx installed and running
# - Port 80 accessible from internet

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

# Check arguments
if [ $# -lt 2 ]; then
    echo "Usage: $0 <domain> <email>"
    echo "Example: $0 example.com admin@example.com"
    exit 1
fi

DOMAIN=$1
EMAIL=$2

# Check if running as root
if [ "$EUID" -ne 0 ]; then
    error "Please run as root: sudo $0 $DOMAIN $EMAIL"
fi

log "Starting SSL setup for ${DOMAIN}..."

# =====================
# 1. Install Certbot
# =====================
log "Installing Certbot..."

if command -v certbot &> /dev/null; then
    log "Certbot already installed"
else
    apt update
    apt install -y certbot python3-certbot-nginx
fi

# =====================
# 2. Create ACME directory
# =====================
log "Creating ACME challenge directory..."
mkdir -p /var/www/certbot
chown -R www-data:www-data /var/www/certbot

# =====================
# 3. Create temporary nginx config for ACME challenge
# =====================
log "Creating temporary nginx config..."

TEMP_CONF="/etc/nginx/sites-available/qiyas-temp.conf"
cat > "$TEMP_CONF" << EOF
server {
    listen 80;
    listen [::]:80;
    server_name ${DOMAIN} www.${DOMAIN};

    location /.well-known/acme-challenge/ {
        root /var/www/certbot;
        allow all;
    }

    location / {
        return 200 'OK';
        add_header Content-Type text/plain;
    }
}
EOF

# Enable temporary config
ln -sf "$TEMP_CONF" /etc/nginx/sites-enabled/qiyas-temp.conf

# Remove any existing qiyas config temporarily
if [ -f /etc/nginx/sites-enabled/qiyas.conf ]; then
    rm /etc/nginx/sites-enabled/qiyas.conf
fi

# Test and reload nginx
nginx -t || error "Nginx configuration test failed"
systemctl reload nginx

# =====================
# 4. Obtain SSL Certificate
# =====================
log "Obtaining SSL certificate..."

certbot certonly \
    --webroot \
    --webroot-path=/var/www/certbot \
    -d "$DOMAIN" \
    -d "www.$DOMAIN" \
    --email "$EMAIL" \
    --agree-tos \
    --non-interactive \
    --force-renewal

# Check if certificate was obtained
if [ ! -f "/etc/letsencrypt/live/${DOMAIN}/fullchain.pem" ]; then
    error "Failed to obtain SSL certificate"
fi

log "SSL certificate obtained successfully!"

# =====================
# 5. Configure Main Nginx
# =====================
log "Configuring production nginx..."

# Copy the main config
NGINX_CONF="/etc/nginx/sites-available/qiyas.conf"

# Replace YOUR_DOMAIN placeholder
if [ -f "/var/www/qiyas/deploy/nginx/qiyas.conf" ]; then
    sed "s/YOUR_DOMAIN/${DOMAIN}/g" /var/www/qiyas/deploy/nginx/qiyas.conf > "$NGINX_CONF"
else
    warn "Nginx config template not found, using default..."
    # Create a basic config
    cat > "$NGINX_CONF" << 'NGINX'
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
    server_name DOMAIN_PLACEHOLDER www.DOMAIN_PLACEHOLDER;
    return 301 https://$host$request_uri;
}

server {
    listen 443 ssl http2;
    server_name DOMAIN_PLACEHOLDER www.DOMAIN_PLACEHOLDER;

    ssl_certificate /etc/letsencrypt/live/DOMAIN_PLACEHOLDER/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/DOMAIN_PLACEHOLDER/privkey.pem;
    ssl_protocols TLSv1.2 TLSv1.3;
    ssl_prefer_server_ciphers off;

    add_header Strict-Transport-Security "max-age=63072000" always;

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
    sed -i "s/DOMAIN_PLACEHOLDER/${DOMAIN}/g" "$NGINX_CONF"
fi

# Remove temporary config
rm -f /etc/nginx/sites-enabled/qiyas-temp.conf
rm -f "$TEMP_CONF"

# Enable main config
ln -sf "$NGINX_CONF" /etc/nginx/sites-enabled/qiyas.conf

# Test and reload
nginx -t || error "Nginx configuration test failed"
systemctl reload nginx

# =====================
# 6. Setup Auto-Renewal
# =====================
log "Setting up auto-renewal..."

# Create renewal hook to reload nginx
mkdir -p /etc/letsencrypt/renewal-hooks/deploy
cat > /etc/letsencrypt/renewal-hooks/deploy/reload-nginx.sh << 'EOF'
#!/bin/bash
systemctl reload nginx
EOF
chmod +x /etc/letsencrypt/renewal-hooks/deploy/reload-nginx.sh

# Test renewal
certbot renew --dry-run

# =====================
# 7. Verify
# =====================
log "Verifying setup..."

sleep 2

HTTP_CODE=$(curl -s -o /dev/null -w "%{http_code}" "https://${DOMAIN}" || echo "000")

if [ "$HTTP_CODE" = "200" ] || [ "$HTTP_CODE" = "301" ] || [ "$HTTP_CODE" = "302" ]; then
    echo ""
    echo -e "${GREEN}========================================${NC}"
    echo -e "${GREEN}   SSL Setup Complete!${NC}"
    echo -e "${GREEN}========================================${NC}"
    echo ""
    echo -e "Domain: ${BLUE}https://${DOMAIN}${NC}"
    echo -e "Certificate expires: $(openssl x509 -dates -noout -in /etc/letsencrypt/live/${DOMAIN}/cert.pem | grep notAfter | cut -d= -f2)"
    echo ""
    echo "Auto-renewal is enabled. Certificates will renew automatically."
    echo ""
else
    warn "Could not verify HTTPS. HTTP status: ${HTTP_CODE}"
    warn "Please check your application is running and try accessing https://${DOMAIN}"
fi

log "Done!"
