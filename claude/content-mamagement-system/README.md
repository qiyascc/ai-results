# Qiyas CMS

Enterprise-grade, multi-language Content Management System built with Next.js 15, optimized for minimal operational costs on VDS deployment.

## 🚀 Features

- **Multi-Language Support**: AZ, EN, TR with full i18n
- **Role-Based Access Control (RBAC)**: Granular permissions system
- **Real-time Features**: WebSocket-powered chat and notifications
- **Blog System**: Full-featured blog with categories, tags, and SEO
- **Media Library**: Image upload and management
- **Admin Dashboard**: Complete content management interface
- **AI Chat Integration**: Built-in AI chat functionality
- **Authentication**: Secure JWT-based authentication
- **Dark/Light Theme**: Full theme support

## 📊 Cost Analysis

| Service | Managed Service | Self-Hosted (Hetzner VDS) |
|---------|-----------------|---------------------------|
| Database | $50-100/month | Included |
| Redis | $15-30/month | Included |
| Hosting | $20-50/month | €7-12/month |
| Total | $85-180/month | **~€10/month** |

## 🛠 Tech Stack

- **Framework**: Next.js 15 (App Router)
- **Language**: TypeScript
- **Styling**: Tailwind CSS + shadcn/ui
- **Database**: PostgreSQL + Prisma ORM
- **Cache**: Redis
- **Real-time**: Socket.io
- **State**: Zustand + TanStack Query
- **Auth**: JWT (jose)
- **i18n**: next-intl
- **Validation**: Zod

## 📁 Project Structure

```
qiyas-project/
├── apps/
│   └── socket-server/     # Standalone Socket.io server
├── prisma/
│   ├── schema.prisma      # Database schema
│   └── seed.ts            # Database seeding
├── src/
│   ├── app/               # Next.js App Router
│   │   ├── [locale]/      # Localized pages
│   │   └── api/           # API routes
│   ├── components/        # React components
│   │   ├── ui/            # shadcn/ui components
│   │   ├── layouts/       # Layout components
│   │   ├── chat/          # Chat components
│   │   └── providers/     # Context providers
│   ├── hooks/             # Custom React hooks
│   ├── lib/               # Utilities
│   │   ├── auth/          # Authentication utilities
│   │   ├── api/           # API helpers
│   │   ├── i18n/          # Internationalization
│   │   └── db.ts          # Database client
│   ├── locales/           # Translation files
│   │   ├── az/
│   │   ├── en/
│   │   └── tr/
│   ├── schemas/           # Zod validation schemas
│   └── stores/            # Zustand stores
├── deploy/                # Deployment configs
├── docker-compose.yml     # Docker configuration
└── ecosystem.config.js    # PM2 configuration
```

## 🚀 Getting Started

### Prerequisites

- Node.js 20+
- pnpm 8+
- PostgreSQL 16+
- Redis 7+

### Development Setup

1. **Clone and install dependencies**
```bash
git clone https://github.com/your-username/qiyas.git
cd qiyas
pnpm install
```

2. **Start development databases**
```bash
docker-compose -f docker-compose.dev.yml up -d
```

3. **Configure environment**
```bash
cp .env.example .env
# Edit .env with your settings
```

4. **Setup database**
```bash
pnpm prisma generate
pnpm prisma migrate dev
pnpm prisma db seed
```

5. **Start development servers**
```bash
# Terminal 1: Next.js
pnpm dev

# Terminal 2: Socket server
cd apps/socket-server && pnpm dev
```

6. **Access the application**
- App: http://localhost:3000
- Admin: http://localhost:3000/az/admin
- API Health: http://localhost:3000/api/v1/health
- Adminer: http://localhost:8080
- Redis Commander: http://localhost:8081

### Default Credentials

- **Email**: admin@qiyas.cc
- **Password**: Admin123!

## 🚢 Deployment

### VDS Deployment (Recommended)

1. **Server Setup**
```bash
# Install dependencies on VDS
apt update && apt upgrade -y
apt install -y nginx certbot python3-certbot-nginx

# Install Node.js
curl -fsSL https://deb.nodesource.com/setup_20.x | bash -
apt install -y nodejs

# Install pnpm and PM2
npm install -g pnpm pm2
```

2. **Database Setup**
```bash
# Install PostgreSQL
apt install -y postgresql postgresql-contrib

# Create database
sudo -u postgres createuser qiyas
sudo -u postgres createdb qiyas -O qiyas

# Install Redis
apt install -y redis-server
systemctl enable redis-server
```

3. **Deploy Application**
```bash
# Clone repository
git clone https://github.com/your-username/qiyas.git /var/www/qiyas
cd /var/www/qiyas

# Configure environment
cp .env.example .env
nano .env

# Install and build
pnpm install
pnpm prisma generate
pnpm prisma migrate deploy
pnpm prisma db seed
pnpm build

# Start with PM2
pm2 start ecosystem.config.js
pm2 save
pm2 startup
```

4. **Configure Nginx**
```bash
cp deploy/nginx.conf /etc/nginx/sites-available/qiyas
ln -s /etc/nginx/sites-available/qiyas /etc/nginx/sites-enabled/
nginx -t && systemctl reload nginx
```

5. **SSL Certificate**
```bash
certbot --nginx -d your-domain.com
```

### Docker Deployment

```bash
# Build and start all services
docker-compose up -d --build

# Run migrations
docker-compose exec app pnpm prisma migrate deploy
docker-compose exec app pnpm prisma db seed
```

## 📚 API Documentation

### Authentication

```bash
# Login
POST /api/v1/auth/login
Body: { "email": "admin@qiyas.cc", "password": "Admin123!" }

# Get current user
GET /api/v1/auth/me
Header: Authorization: Bearer <token>

# Logout
POST /api/v1/auth/logout
```

### Posts

```bash
# List posts
GET /api/v1/posts?page=1&limit=10&search=keyword

# Create post
POST /api/v1/posts
Body: { "title": "...", "content": "...", "categoryId": "...", "status": "DRAFT" }

# Update post
PUT /api/v1/posts/:id

# Delete post
DELETE /api/v1/posts/:id
```

### Media

```bash
# Upload file
POST /api/v1/media
Body: FormData with "file" field

# List media
GET /api/v1/media?page=1&limit=24&type=image
```

## 🔐 Security

- JWT tokens with secure httpOnly cookies
- CSRF protection
- Rate limiting on API routes
- Input validation with Zod
- SQL injection protection via Prisma
- XSS protection headers

## 📈 Performance

- Standalone Next.js build for minimal footprint
- Redis caching for sessions and data
- Image optimization
- Gzip compression
- Static file caching
- Database connection pooling

## 🔧 Environment Variables

```env
# Database
DATABASE_URL="postgresql://user:password@localhost:5432/qiyas"

# Redis
REDIS_URL="redis://localhost:6379"

# JWT
JWT_SECRET="your-super-secret-key"
JWT_EXPIRES_IN="7d"

# App
NEXT_PUBLIC_APP_URL="https://your-domain.com"
NEXT_PUBLIC_SOCKET_URL="https://your-domain.com"
```

## 📝 License

MIT License - See [LICENSE](LICENSE) for details.

## 🤝 Contributing

1. Fork the repository
2. Create feature branch (`git checkout -b feature/amazing`)
3. Commit changes (`git commit -m 'Add amazing feature'`)
4. Push to branch (`git push origin feature/amazing`)
5. Open a Pull Request

## 📞 Support

- Documentation: [docs.qiyas.cc](https://docs.qiyas.cc)
- Issues: [GitHub Issues](https://github.com/your-username/qiyas/issues)
- Email: support@qiyas.cc
