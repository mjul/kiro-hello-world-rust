# Deployment Guide

This guide covers deploying the SSO Web App to various environments.

## Local Development

### Quick Start
```powershell
# Windows PowerShell
.\scripts\init_db.ps1
$env:RUST_LOG='debug'; cargo run
```

```bash
# Linux/macOS
./scripts/init_db.sh
RUST_LOG=debug cargo run
```

## Production Deployment

### 1. Environment Setup

Create a production `.env` file:

```env
# Production Configuration
DATABASE_URL=sqlite:/app/data/sso_app.db
MICROSOFT_CLIENT_ID=your_production_ms_client_id
MICROSOFT_CLIENT_SECRET=your_production_ms_client_secret
GITHUB_CLIENT_ID=your_production_gh_client_id
GITHUB_CLIENT_SECRET=your_production_gh_client_secret
SESSION_SECRET=your_secure_64_char_random_string_here
BASE_URL=https://your-domain.com
RUST_LOG=info
```

### 2. Build for Production

```bash
# Build optimized binary
cargo build --release

# Binary location
# Windows: .\target\release\sso-web-app.exe
# Linux/macOS: ./target/release/sso-web-app
```

### 3. Docker Deployment

#### Dockerfile
```dockerfile
FROM rust:1.75 as builder

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY templates ./templates
COPY migrations ./migrations

RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && \
    apt-get install -y ca-certificates && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/sso-web-app ./
COPY --from=builder /app/templates ./templates
COPY --from=builder /app/migrations ./migrations

RUN mkdir -p /app/data

EXPOSE 3000

CMD ["./sso-web-app"]
```

#### Docker Compose
```yaml
version: '3.8'

services:
  sso-web-app:
    build: .
    ports:
      - "3000:3000"
    environment:
      - DATABASE_URL=sqlite:/app/data/sso_app.db
      - MICROSOFT_CLIENT_ID=${MICROSOFT_CLIENT_ID}
      - MICROSOFT_CLIENT_SECRET=${MICROSOFT_CLIENT_SECRET}
      - GITHUB_CLIENT_ID=${GITHUB_CLIENT_ID}
      - GITHUB_CLIENT_SECRET=${GITHUB_CLIENT_SECRET}
      - SESSION_SECRET=${SESSION_SECRET}
      - BASE_URL=${BASE_URL}
      - RUST_LOG=info
    volumes:
      - ./data:/app/data
    restart: unless-stopped
```

#### Build and Run
```bash
# Build image
docker build -t sso-web-app .

# Run with environment file
docker run -d \
  --name sso-web-app \
  -p 3000:3000 \
  --env-file .env \
  -v $(pwd)/data:/app/data \
  sso-web-app

# Or use docker-compose
docker-compose up -d
```

### 4. Reverse Proxy Setup

#### Nginx Configuration
```nginx
server {
    listen 80;
    server_name your-domain.com;
    return 301 https://$server_name$request_uri;
}

server {
    listen 443 ssl http2;
    server_name your-domain.com;

    ssl_certificate /path/to/your/certificate.crt;
    ssl_certificate_key /path/to/your/private.key;

    # Security headers
    add_header X-Frame-Options DENY;
    add_header X-Content-Type-Options nosniff;
    add_header X-XSS-Protection "1; mode=block";
    add_header Strict-Transport-Security "max-age=31536000; includeSubDomains";

    location / {
        proxy_pass http://127.0.0.1:3000;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        
        # Timeouts
        proxy_connect_timeout 60s;
        proxy_send_timeout 60s;
        proxy_read_timeout 60s;
    }
}
```

#### Caddy Configuration
```caddyfile
your-domain.com {
    reverse_proxy localhost:3000
    
    header {
        X-Frame-Options DENY
        X-Content-Type-Options nosniff
        X-XSS-Protection "1; mode=block"
        Strict-Transport-Security "max-age=31536000; includeSubDomains"
    }
}
```

### 5. Systemd Service (Linux)

Create `/etc/systemd/system/sso-web-app.service`:

```ini
[Unit]
Description=SSO Web App
After=network.target

[Service]
Type=simple
User=sso-app
Group=sso-app
WorkingDirectory=/opt/sso-web-app
ExecStart=/opt/sso-web-app/sso-web-app
EnvironmentFile=/opt/sso-web-app/.env
Restart=always
RestartSec=10

# Security settings
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/opt/sso-web-app/data

[Install]
WantedBy=multi-user.target
```

Enable and start:
```bash
sudo systemctl daemon-reload
sudo systemctl enable sso-web-app
sudo systemctl start sso-web-app
sudo systemctl status sso-web-app
```

### 6. Windows Service

Use [NSSM](https://nssm.cc/) to create a Windows service:

```cmd
# Install NSSM
# Download from https://nssm.cc/download

# Create service
nssm install "SSO Web App" "C:\path\to\sso-web-app.exe"
nssm set "SSO Web App" AppDirectory "C:\path\to\app"
nssm set "SSO Web App" AppEnvironmentExtra "RUST_LOG=info"

# Start service
nssm start "SSO Web App"
```

## Cloud Deployment

### AWS (EC2 + ALB)

1. **Launch EC2 instance** (t3.micro for small workloads)
2. **Install Docker** and deploy using Docker Compose
3. **Set up Application Load Balancer** with SSL certificate
4. **Configure security groups** (allow 80, 443, and 3000 from ALB)

### Azure (Container Instances)

```bash
# Create resource group
az group create --name sso-web-app-rg --location eastus

# Deploy container
az container create \
  --resource-group sso-web-app-rg \
  --name sso-web-app \
  --image your-registry/sso-web-app:latest \
  --dns-name-label sso-web-app-unique \
  --ports 3000 \
  --environment-variables \
    DATABASE_URL=sqlite:/app/data/sso_app.db \
    BASE_URL=https://sso-web-app-unique.eastus.azurecontainer.io \
    RUST_LOG=info \
  --secure-environment-variables \
    MICROSOFT_CLIENT_ID=your_client_id \
    MICROSOFT_CLIENT_SECRET=your_client_secret \
    GITHUB_CLIENT_ID=your_github_id \
    GITHUB_CLIENT_SECRET=your_github_secret \
    SESSION_SECRET=your_session_secret
```

### Google Cloud (Cloud Run)

```bash
# Build and push to Container Registry
docker build -t gcr.io/your-project/sso-web-app .
docker push gcr.io/your-project/sso-web-app

# Deploy to Cloud Run
gcloud run deploy sso-web-app \
  --image gcr.io/your-project/sso-web-app \
  --platform managed \
  --region us-central1 \
  --allow-unauthenticated \
  --set-env-vars DATABASE_URL=sqlite:/app/data/sso_app.db,RUST_LOG=info \
  --set-env-vars BASE_URL=https://your-service-url
```

## Monitoring and Logging

### Application Logs

The application uses structured logging. Configure log levels:

```env
# Production logging
RUST_LOG=info

# Debug logging
RUST_LOG=debug

# Module-specific logging
RUST_LOG=sso_web_app=info,tower_http=debug
```

### Health Check Endpoint

Add to your load balancer/monitoring:
- **URL**: `GET /login`
- **Expected**: HTTP 200 with HTML content
- **Timeout**: 30 seconds

### Monitoring Checklist

- [ ] Application logs are being collected
- [ ] Database file has proper backup strategy
- [ ] SSL certificate auto-renewal is configured
- [ ] Resource usage is monitored (CPU, memory, disk)
- [ ] OAuth2 credentials are securely stored
- [ ] Session secret is rotated periodically

## Security Considerations

### Production Security Checklist

- [ ] Use HTTPS in production (`BASE_URL` starts with `https://`)
- [ ] Generate strong session secret (64+ characters)
- [ ] Store secrets in environment variables, not in code
- [ ] Update OAuth2 redirect URIs to production URLs
- [ ] Enable security headers in reverse proxy
- [ ] Regular security updates for dependencies
- [ ] Database file permissions are restricted
- [ ] Application runs as non-root user

### OAuth2 Security

1. **Microsoft Azure AD**:
   - Use "Accounts in this organizational directory only" for internal apps
   - Enable "ID tokens" in Authentication settings
   - Add production redirect URI

2. **GitHub OAuth**:
   - Use organization-owned OAuth apps for team projects
   - Regularly rotate client secrets
   - Monitor OAuth app usage in GitHub settings

## Backup and Recovery

### Database Backup

```bash
# Simple backup
cp sso_app.db sso_app_backup_$(date +%Y%m%d_%H%M%S).db

# Automated backup script
#!/bin/bash
BACKUP_DIR="/backups/sso-web-app"
mkdir -p $BACKUP_DIR
sqlite3 sso_app.db ".backup $BACKUP_DIR/sso_app_$(date +%Y%m%d_%H%M%S).db"
find $BACKUP_DIR -name "*.db" -mtime +7 -delete
```

### Recovery

```bash
# Restore from backup
cp sso_app_backup_20240115_120000.db sso_app.db
# Restart application
```

## Troubleshooting

### Common Production Issues

1. **Application won't start**:
   - Check environment variables are set
   - Verify database directory permissions
   - Check port availability

2. **OAuth2 redirect errors**:
   - Verify redirect URIs in OAuth2 app settings
   - Check BASE_URL matches actual domain
   - Ensure HTTPS is used in production

3. **Session issues**:
   - Verify SESSION_SECRET is consistent
   - Check cookie domain settings
   - Ensure secure cookies for HTTPS

4. **Database errors**:
   - Check disk space
   - Verify write permissions
   - Check SQLite file corruption

### Log Analysis

```bash
# View application logs
journalctl -u sso-web-app -f

# Docker logs
docker logs -f sso-web-app

# Search for errors
grep -i error /var/log/sso-web-app.log
```