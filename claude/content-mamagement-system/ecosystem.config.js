// PM2 Ecosystem Configuration
// Run with: pm2 start ecosystem.config.js

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
      env_production: {
        NODE_ENV: 'production',
        PORT: 3000,
      },
      error_file: '/var/log/qiyas/app-error.log',
      out_file: '/var/log/qiyas/app-out.log',
      merge_logs: true,
      log_date_format: 'YYYY-MM-DD HH:mm:ss Z',
    },
    {
      name: 'qiyas-socket',
      script: 'dist/index.js',
      cwd: '/var/www/qiyas/apps/socket-server',
      instances: 1,
      exec_mode: 'fork',
      autorestart: true,
      watch: false,
      max_memory_restart: '512M',
      env: {
        NODE_ENV: 'production',
        SOCKET_PORT: 3001,
      },
      env_production: {
        NODE_ENV: 'production',
        SOCKET_PORT: 3001,
      },
      error_file: '/var/log/qiyas/socket-error.log',
      out_file: '/var/log/qiyas/socket-out.log',
      merge_logs: true,
      log_date_format: 'YYYY-MM-DD HH:mm:ss Z',
    },
  ],

  deploy: {
    production: {
      user: 'deploy',
      host: ['your-server-ip'],
      ref: 'origin/main',
      repo: 'git@github.com:your-username/qiyas.git',
      path: '/var/www/qiyas',
      'pre-deploy-local': '',
      'post-deploy': 'pnpm install && pnpm build && pm2 reload ecosystem.config.js --env production',
      'pre-setup': '',
      env: {
        NODE_ENV: 'production',
      },
    },
    staging: {
      user: 'deploy',
      host: ['your-staging-server-ip'],
      ref: 'origin/develop',
      repo: 'git@github.com:your-username/qiyas.git',
      path: '/var/www/qiyas-staging',
      'post-deploy': 'pnpm install && pnpm build && pm2 reload ecosystem.config.js --env staging',
      env: {
        NODE_ENV: 'staging',
      },
    },
  },
}
