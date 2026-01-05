# Server Deployment Guide

This guide covers deploying Talos in production environments, including database setup, configuration, Docker deployment, and security best practices.

## Table of Contents

- [Database Setup](#database-setup)
- [Configuration Reference](#configuration-reference)
- [Environment Variables](#environment-variables)
- [Running the Server](#running-the-server)
- [Docker Deployment](#docker-deployment)
- [Reverse Proxy Setup](#reverse-proxy-setup)
- [TLS/HTTPS Configuration](#tlshttps-configuration)
- [Production Checklist](#production-checklist)
- [Health Monitoring](#health-monitoring)

---

## Database Setup

Talos supports SQLite and PostgreSQL. Choose based on your needs:

| Feature | SQLite | PostgreSQL |
|---------|--------|------------|
| Setup complexity | Simple | Moderate |
| Concurrent writes | Limited | Excellent |
| Replication | No | Yes |
| Backups | File copy | pg_dump, streaming |
| Recommended for | Dev, small deployments | Production |

### SQLite Setup

SQLite is the default and requires no additional setup:

```toml
# config.toml
[database]
db_type = "sqlite"
sqlite_url = "sqlite://talos.db"
```

For production, use an absolute path:

```toml
[database]
db_type = "sqlite"
sqlite_url = "sqlite:///var/lib/talos/talos.db"
```

### PostgreSQL Setup

1. **Create the database:**

```bash
createdb talos
```

2. **Configure Talos:**

```toml
# config.toml
[database]
db_type = "postgres"
postgres_url = "postgres://user:password@localhost:5432/talos"
```

3. **Run migrations:**

```bash
export DATABASE_URL="postgres://user:password@localhost:5432/talos"
sqlx migrate run
```

### Running Migrations

Migrations are required before first run:

```bash
# Set your database URL
export DATABASE_URL="sqlite://talos.db"
# or
export DATABASE_URL="postgres://user:password@localhost:5432/talos"

# Run migrations
sqlx migrate run
```

---

## Configuration Reference

Talos uses a `config.toml` file for configuration. Here's a complete reference:

```toml
# =============================================================================
# TALOS CONFIGURATION FILE
# =============================================================================

# -----------------------------------------------------------------------------
# Server Settings
# -----------------------------------------------------------------------------
[server]
host = "0.0.0.0"              # Bind address (0.0.0.0 for all interfaces)
port = 8080                    # Port to listen on
heartbeat_interval = 300       # Expected heartbeat interval (seconds)

# -----------------------------------------------------------------------------
# Database Settings
# -----------------------------------------------------------------------------
[database]
db_type = "sqlite"             # "sqlite" or "postgres"
sqlite_url = "sqlite://talos.db"
postgres_url = "postgres://user:pass@localhost:5432/talos"

# -----------------------------------------------------------------------------
# License Key Generation
# -----------------------------------------------------------------------------
[license]
key_prefix = "LIC"             # Prefix for generated keys (e.g., "LIC-XXXX")
key_segments = 4               # Number of segments after prefix
key_segment_length = 4         # Characters per segment

# -----------------------------------------------------------------------------
# JWT Authentication (requires jwt-auth feature)
# -----------------------------------------------------------------------------
[auth]
enabled = false                # Enable/disable JWT auth
jwt_secret = ""                # Use env var TALOS_JWT_SECRET instead
jwt_issuer = "talos"           # Token issuer claim
jwt_audience = "talos-api"     # Token audience claim
token_expiration_secs = 86400  # Token lifetime (24 hours)

# -----------------------------------------------------------------------------
# Rate Limiting (requires rate-limiting feature)
# -----------------------------------------------------------------------------
[rate_limit]
enabled = true
validate_per_minute = 100      # /validate endpoint
heartbeat_per_minute = 60      # /heartbeat endpoint
bind_per_minute = 10           # /bind endpoint

# -----------------------------------------------------------------------------
# Background Jobs (requires background-jobs feature)
# -----------------------------------------------------------------------------
[jobs]
enabled = true
grace_period_check = "0 */15 * * * *"    # Every 15 minutes
expiration_check = "0 0 * * * *"         # Every hour
stale_device_check = "0 0 0 * * *"       # Daily at midnight
stale_device_days = 90                    # Days before device considered stale

# -----------------------------------------------------------------------------
# Admin API Security
# -----------------------------------------------------------------------------
[admin]
ip_whitelist = []              # Empty = allow all, or ["127.0.0.1", "10.0.0.0/8"]
audit_logging = false          # Log all admin actions

# -----------------------------------------------------------------------------
# Tier Configuration (optional)
# -----------------------------------------------------------------------------
[tiers.free]
features = []
bandwidth_gb = 0

[tiers.pro]
features = ["export", "api_access", "priority_support"]
bandwidth_gb = 100

[tiers.enterprise]
features = ["export", "api_access", "priority_support", "sso", "audit_logs"]
bandwidth_gb = 0  # Unlimited

# -----------------------------------------------------------------------------
# Logging
# -----------------------------------------------------------------------------
[logging]
level = "info"                 # trace, debug, info, warn, error
format = "json"                # "json" or "pretty"
```

---

## Environment Variables

All configuration options can be overridden via environment variables:

| Variable | Description | Example |
|----------|-------------|---------|
| `TALOS_SERVER_HOST` | Server bind address | `0.0.0.0` |
| `TALOS_SERVER_PORT` | Server port | `8080` |
| `TALOS_DATABASE_TYPE` | Database type | `sqlite` or `postgres` |
| `TALOS_DATABASE_URL` | Database connection URL | `postgres://...` |
| `TALOS_JWT_SECRET` | JWT signing secret (required if auth enabled) | `your-secret-key` |
| `TALOS_JWT_ISSUER` | JWT issuer claim | `talos` |
| `TALOS_JWT_AUDIENCE` | JWT audience claim | `talos-api` |
| `TALOS_LOG_LEVEL` | Log level | `info` |
| `DATABASE_URL` | Used by SQLx for migrations | Same as `TALOS_DATABASE_URL` |

**Example `.env` file:**

```bash
TALOS_SERVER_HOST=0.0.0.0
TALOS_SERVER_PORT=8080
TALOS_DATABASE_TYPE=postgres
TALOS_DATABASE_URL=postgres://talos:secretpassword@localhost:5432/talos
TALOS_JWT_SECRET=your-super-secret-jwt-key-at-least-32-chars
TALOS_LOG_LEVEL=info
DATABASE_URL=postgres://talos:secretpassword@localhost:5432/talos
```

---

## Running the Server

### Development

```bash
cargo run --bin talos_server
```

### Production Build

```bash
# Build optimized release binary
cargo build --release --features "admin-api,jwt-auth,rate-limiting,background-jobs"

# Run
./target/release/talos_server
```

### With systemd

Create `/etc/systemd/system/talos.service`:

```ini
[Unit]
Description=Talos License Server
After=network.target postgresql.service

[Service]
Type=simple
User=talos
Group=talos
WorkingDirectory=/opt/talos
ExecStart=/opt/talos/talos_server
Restart=always
RestartSec=5
Environment=RUST_LOG=info
EnvironmentFile=/opt/talos/.env

[Install]
WantedBy=multi-user.target
```

Enable and start:

```bash
sudo systemctl daemon-reload
sudo systemctl enable talos
sudo systemctl start talos
```

---

## Docker Deployment

### Dockerfile

```dockerfile
# Build stage
FROM rust:1.75-slim as builder

WORKDIR /app
COPY . .

RUN apt-get update && apt-get install -y pkg-config libssl-dev
RUN cargo build --release --features "admin-api,jwt-auth,rate-limiting,background-jobs,postgres"

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates libssl3 && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/target/release/talos_server /app/
COPY --from=builder /app/migrations /app/migrations

ENV RUST_LOG=info
EXPOSE 8080

HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
  CMD curl -f http://localhost:8080/health || exit 1

CMD ["./talos_server"]
```

### docker-compose.yml

```yaml
version: '3.8'

services:
  talos:
    build: .
    ports:
      - "8080:8080"
    environment:
      - TALOS_SERVER_HOST=0.0.0.0
      - TALOS_SERVER_PORT=8080
      - TALOS_DATABASE_TYPE=postgres
      - TALOS_DATABASE_URL=postgres://talos:secretpassword@db:5432/talos
      - TALOS_JWT_SECRET=${JWT_SECRET}
      - RUST_LOG=info
      - DATABASE_URL=postgres://talos:secretpassword@db:5432/talos
    depends_on:
      db:
        condition: service_healthy
    restart: unless-stopped

  db:
    image: postgres:15-alpine
    environment:
      - POSTGRES_USER=talos
      - POSTGRES_PASSWORD=secretpassword
      - POSTGRES_DB=talos
    volumes:
      - postgres_data:/var/lib/postgresql/data
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U talos"]
      interval: 5s
      timeout: 5s
      retries: 5
    restart: unless-stopped

volumes:
  postgres_data:
```

### Running with Docker Compose

```bash
# Start services
docker-compose up -d

# Run migrations
docker-compose exec talos sqlx migrate run

# View logs
docker-compose logs -f talos

# Stop services
docker-compose down
```

---

## Reverse Proxy Setup

### nginx

```nginx
upstream talos {
    server 127.0.0.1:8080;
    keepalive 32;
}

server {
    listen 443 ssl http2;
    server_name license.example.com;

    ssl_certificate /etc/letsencrypt/live/license.example.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/license.example.com/privkey.pem;

    # Security headers
    add_header X-Frame-Options DENY;
    add_header X-Content-Type-Options nosniff;
    add_header X-XSS-Protection "1; mode=block";

    location / {
        proxy_pass http://talos;
        proxy_http_version 1.1;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        proxy_set_header Connection "";

        # Timeouts
        proxy_connect_timeout 60s;
        proxy_send_timeout 60s;
        proxy_read_timeout 60s;
    }

    # Health check endpoint (no auth required)
    location /health {
        proxy_pass http://talos/health;
        proxy_http_version 1.1;
    }
}

# Redirect HTTP to HTTPS
server {
    listen 80;
    server_name license.example.com;
    return 301 https://$server_name$request_uri;
}
```

### Traefik

```yaml
# traefik.yml
http:
  routers:
    talos:
      rule: "Host(`license.example.com`)"
      service: talos
      tls:
        certResolver: letsencrypt

  services:
    talos:
      loadBalancer:
        servers:
          - url: "http://talos:8080"
        healthCheck:
          path: /health
          interval: 30s
```

---

## TLS/HTTPS Configuration

For production, always use HTTPS. Options:

### Option 1: Reverse Proxy (Recommended)

Use nginx or Traefik with Let's Encrypt (shown above).

### Option 2: Direct TLS in Talos

Coming soon - Talos will support direct TLS termination.

### Let's Encrypt with Certbot

```bash
# Install certbot
sudo apt install certbot python3-certbot-nginx

# Get certificate
sudo certbot --nginx -d license.example.com

# Auto-renewal (added automatically)
sudo certbot renew --dry-run
```

---

## Production Checklist

Before going to production, verify:

### Security

- [ ] JWT authentication enabled (`jwt-auth` feature)
- [ ] Strong JWT secret (32+ characters, random)
- [ ] Rate limiting enabled (`rate-limiting` feature)
- [ ] IP whitelisting configured for admin API (`[admin] ip_whitelist`)
- [ ] HTTPS/TLS configured
- [ ] Database credentials are secure
- [ ] No secrets in config files (use env vars)
- [ ] Firewall configured (only expose necessary ports)

### Reliability

- [ ] PostgreSQL for database (not SQLite)
- [ ] Database backups configured
- [ ] Health check monitoring set up
- [ ] Log aggregation configured
- [ ] Restart policy configured (systemd/Docker)

### Performance

- [ ] Connection pooling configured
- [ ] Rate limits tuned for expected load
- [ ] Database indexes verified (run migrations)
- [ ] Reverse proxy with keepalive connections

### Monitoring

- [ ] Health endpoint monitored (`/health`)
- [ ] Log level set appropriately (`info` for production)
- [ ] Alerting configured for failures

---

## Health Monitoring

### Health Endpoint

Talos provides a `/health` endpoint:

```bash
curl http://localhost:8080/health
```

Response:

```json
{
  "status": "healthy",
  "service": "talos",
  "version": "0.1.0",
  "database": {
    "connected": true,
    "db_type": "postgres"
  }
}
```

Status values:
- `healthy` - All systems operational
- `degraded` - Database connection issues

### Monitoring Integration

**Prometheus (via reverse proxy logs):**

```nginx
# nginx.conf - Log format for Prometheus
log_format prometheus '$remote_addr - $remote_user [$time_local] '
                      '"$request" $status $body_bytes_sent '
                      '"$http_referer" "$http_user_agent" '
                      '$request_time';
```

**Uptime monitoring:**

```bash
# Simple health check script
#!/bin/bash
response=$(curl -s -o /dev/null -w "%{http_code}" http://localhost:8080/health)
if [ "$response" != "200" ]; then
    echo "Talos health check failed: $response"
    # Send alert
fi
```

**Docker health check:**

```yaml
healthcheck:
  test: ["CMD", "curl", "-f", "http://localhost:8080/health"]
  interval: 30s
  timeout: 10s
  retries: 3
  start_period: 10s
```

---

## Next Steps

- **[Admin API Guide](admin-api.md)** - Manage licenses programmatically
- **[Advanced Topics](advanced.md)** - Background jobs, custom fingerprinting
- **[Troubleshooting](troubleshooting.md)** - Common issues and solutions
