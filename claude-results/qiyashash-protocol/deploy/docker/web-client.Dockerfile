# QiyasHash Web Client Dockerfile
# Multi-stage build for minimal image size

# Build stage
FROM node:20-alpine AS builder

WORKDIR /app

# Copy package files
COPY package.json package-lock.json* ./

# Install dependencies
RUN npm ci

# Copy source
COPY . .

# Build
RUN npm run build

# Runtime stage
FROM nginx:alpine

# Copy nginx config
COPY deploy/nginx.conf /etc/nginx/conf.d/default.conf

# Copy built assets
COPY --from=builder /app/dist /usr/share/nginx/html

# Add security headers
RUN echo 'add_header X-Frame-Options "DENY";' >> /etc/nginx/conf.d/security.conf && \
    echo 'add_header X-Content-Type-Options "nosniff";' >> /etc/nginx/conf.d/security.conf && \
    echo 'add_header X-XSS-Protection "1; mode=block";' >> /etc/nginx/conf.d/security.conf && \
    echo 'add_header Content-Security-Policy "default-src '\''self'\''; script-src '\''self'\'' '\''unsafe-inline'\''; style-src '\''self'\'' '\''unsafe-inline'\'';";' >> /etc/nginx/conf.d/security.conf

# Expose port
EXPOSE 80

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD wget --no-verbose --tries=1 --spider http://localhost/ || exit 1

CMD ["nginx", "-g", "daemon off;"]
