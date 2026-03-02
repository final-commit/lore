# Forge — Deployment Guide

## Docker (Recommended)

### Quick Start

```bash
docker run -d \
  --name forge \
  -p 3000:3000 \
  -v forge-data:/data \
  -e FORGE_JWT_SECRET="$(openssl rand -base64 32)" \
  ghcr.io/your-org/forge:latest
```

First user to register becomes admin.

### Docker Compose

```yaml
version: '3.8'
services:
  forge:
    image: ghcr.io/your-org/forge:latest
    ports:
      - "3000:3000"
    volumes:
      - forge-data:/data
      - ./my-docs-repo:/data/repo  # optional: mount existing git repo
    environment:
      FORGE_JWT_SECRET: "change-me-to-a-random-secret"
      FORGE_PORT: "3000"
      FORGE_HOST: "0.0.0.0"
      FORGE_LOG_LEVEL: "info"
      # Optional: connect to remote git
      # FORGE_GIT_REMOTE_URL: "https://github.com/yourorg/docs.git"
      # FORGE_GIT_REMOTE_TOKEN: "ghp_xxxx"
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "wget", "-qO-", "http://localhost:3000/health"]
      interval: 30s
      timeout: 5s
      retries: 3

volumes:
  forge-data:
```

## Configuration

All config via environment variables:

| Variable | Default | Description |
|----------|---------|-------------|
| `FORGE_HOST` | `0.0.0.0` | Bind address |
| `FORGE_PORT` | `3000` | Port |
| `FORGE_JWT_SECRET` | **required** | Secret for JWT signing (min 32 chars) |
| `FORGE_REPO_PATH` | `/data/repo` | Path to git repository |
| `FORGE_DB_PATH` | `/data/forge.db` | SQLite database path |
| `FORGE_SEARCH_INDEX_PATH` | `/data/search_index` | Tantivy search index |
| `FORGE_LOG_LEVEL` | `info` | Log level (trace/debug/info/warn/error) |
| `FORGE_CORS_ORIGINS` | `*` | Comma-separated allowed origins |
| `FORGE_MAX_UPLOAD_BYTES` | `10485760` | Max attachment upload size (10MB) |

## Reverse Proxy (nginx)

```nginx
server {
    listen 80;
    server_name docs.yourcompany.com;
    return 301 https://$host$request_uri;
}

server {
    listen 443 ssl;
    server_name docs.yourcompany.com;

    ssl_certificate /etc/letsencrypt/live/docs.yourcompany.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/docs.yourcompany.com/privkey.pem;

    # WebSocket support (Yjs collaborative editing)
    location /ws/ {
        proxy_pass http://localhost:3000;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;
        proxy_read_timeout 86400;
    }

    location / {
        proxy_pass http://localhost:3000;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }
}
```

## Resource Requirements

| Users | RAM | CPU | Storage |
|-------|-----|-----|---------|
| 1-10 | 64MB | 0.1 vCPU | Depends on docs |
| 10-50 | 128MB | 0.25 vCPU | Depends on docs |
| 50-200 | 256MB | 0.5 vCPU | Depends on docs |

Forge runs happily on a Raspberry Pi 4 (1GB) or a $5/mo VPS.

## Git Sync Setup

To sync with a GitHub/GitLab/Gitea remote:

1. **Create a repo** on GitHub/GitLab/Gitea
2. **Set env vars:**
   ```bash
   FORGE_GIT_REMOTE_URL=https://github.com/yourorg/docs.git
   FORGE_GIT_REMOTE_TOKEN=ghp_your_token
   ```
3. **Set up webhook** (optional, for auto-pull on push):
   - GitHub: Settings → Webhooks → Add webhook
   - URL: `https://docs.yourcompany.com/api/webhooks/git`
   - Secret: any random string (set `FORGE_WEBHOOK_SECRET` to match)
   - Events: Push events

## Backup

Forge stores all data in two places:
- **`/data/repo/`** — All documents (git repository, inherently versioned)
- **`/data/forge.db`** — Metadata (comments, users, collections, etc.)

```bash
# Backup
docker exec forge tar -czf /tmp/backup.tar.gz /data
docker cp forge:/tmp/backup.tar.gz ./forge-backup-$(date +%Y%m%d).tar.gz

# Or just back up the volume
docker run --rm -v forge-data:/data -v $(pwd):/backup alpine \
  tar -czf /backup/forge-backup.tar.gz /data
```

The git repo is its own backup — push to a remote and you have an off-site copy of all documents.

## Upgrade

```bash
docker pull ghcr.io/your-org/forge:latest
docker compose down && docker compose up -d
```

Migrations run automatically on startup.
