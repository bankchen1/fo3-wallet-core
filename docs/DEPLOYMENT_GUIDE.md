# FO3 Wallet Core Production Deployment Guide

**Version:** Phase 3 (Production Ready)  
**Target Environment:** Production  
**Infrastructure:** Docker Compose with TLS  
**Monitoring:** Prometheus + Grafana + Jaeger  

## 🎯 Overview

This guide provides step-by-step instructions for deploying FO3 Wallet Core to production environments with enterprise-grade security, monitoring, and high availability.

## 📋 Prerequisites

### System Requirements

**Minimum Production Requirements:**
- **CPU:** 4 cores (8 recommended)
- **RAM:** 8GB (16GB recommended)
- **Storage:** 100GB SSD (500GB recommended)
- **Network:** 1Gbps connection
- **OS:** Ubuntu 20.04 LTS or CentOS 8

**Software Requirements:**
- Docker 20.10+
- Docker Compose 2.0+
- SSL certificates (Let's Encrypt or commercial)
- Domain name with DNS configuration

### Security Prerequisites

1. **SSL/TLS Certificates:**
   ```bash
   # Create certificate directory
   sudo mkdir -p /opt/fo3/certs
   
   # Copy your certificates
   sudo cp server.crt /opt/fo3/certs/
   sudo cp server.key /opt/fo3/certs/
   sudo cp ca.crt /opt/fo3/certs/
   
   # Set proper permissions
   sudo chmod 600 /opt/fo3/certs/server.key
   sudo chmod 644 /opt/fo3/certs/server.crt
   sudo chmod 644 /opt/fo3/certs/ca.crt
   ```

2. **Firewall Configuration:**
   ```bash
   # Configure UFW firewall
   sudo ufw enable
   sudo ufw allow 22/tcp    # SSH
   sudo ufw allow 80/tcp    # HTTP (redirect to HTTPS)
   sudo ufw allow 443/tcp   # HTTPS
   sudo ufw allow 50051/tcp # gRPC (if direct access needed)
   ```

3. **Create Data Directories:**
   ```bash
   # Create persistent data directories
   sudo mkdir -p /opt/fo3/data/{postgres,redis,prometheus,grafana,jaeger,kyc_documents}
   sudo mkdir -p /opt/fo3/logs
   sudo mkdir -p /opt/fo3/backups
   
   # Set ownership
   sudo chown -R 1000:1000 /opt/fo3/data
   sudo chown -R 1000:1000 /opt/fo3/logs
   ```

## 🚀 Deployment Steps

### Step 1: Clone Repository

```bash
# Clone the repository
git clone https://github.com/bankchen1/fo3-wallet-core.git
cd fo3-wallet-core

# Checkout production branch
git checkout main
```

### Step 2: Configure Environment

```bash
# Copy production environment template
cp .env.production.example .env.production

# Edit production configuration
nano .env.production
```

**Critical Configuration Items:**
```bash
# Security (MUST CHANGE)
JWT_SECRET=your_secure_64_character_jwt_secret_for_production_use_only
ENCRYPTION_KEY=your_secure_32_byte_encryption_key_for_production_use
KYC_ENCRYPTION_KEY=your_base64_encoded_32_byte_key_for_kyc_documents

# Database (MUST CHANGE)
POSTGRES_PASSWORD=your_secure_postgres_password_for_production
REDIS_PASSWORD=your_secure_redis_password_for_production

# Monitoring (MUST CHANGE)
GRAFANA_PASSWORD=your_secure_grafana_admin_password

# Blockchain RPC URLs (REQUIRED)
ETHEREUM_RPC_URL=https://mainnet.infura.io/v3/YOUR_INFURA_PROJECT_ID
ETHEREUM_API_KEY=YOUR_INFURA_API_KEY
SOLANA_RPC_URL=https://api.mainnet-beta.solana.com

# TLS Configuration
ENABLE_TLS=true
TLS_CERT_PATH=/app/certs/server.crt
TLS_KEY_PATH=/app/certs/server.key
```

### Step 3: Configure Nginx

```bash
# Create nginx configuration
mkdir -p nginx
cat > nginx/nginx.prod.conf << 'EOF'
events {
    worker_connections 1024;
}

http {
    upstream fo3_grpc {
        server fo3-wallet-api:50051;
    }
    
    upstream fo3_websocket {
        server fo3-wallet-api:8080;
    }
    
    # HTTP to HTTPS redirect
    server {
        listen 80;
        server_name api.fo3wallet.com;
        return 301 https://$server_name$request_uri;
    }
    
    # HTTPS gRPC proxy
    server {
        listen 443 ssl http2;
        server_name api.fo3wallet.com;
        
        ssl_certificate /etc/nginx/ssl/server.crt;
        ssl_certificate_key /etc/nginx/ssl/server.key;
        ssl_protocols TLSv1.2 TLSv1.3;
        ssl_ciphers HIGH:!aNULL:!MD5;
        
        location / {
            grpc_pass grpc://fo3_grpc;
            grpc_set_header Host $host;
            grpc_set_header X-Real-IP $remote_addr;
        }
    }
    
    # WebSocket proxy
    server {
        listen 8080 ssl;
        server_name ws.fo3wallet.com;
        
        ssl_certificate /etc/nginx/ssl/server.crt;
        ssl_certificate_key /etc/nginx/ssl/server.key;
        
        location / {
            proxy_pass http://fo3_websocket;
            proxy_http_version 1.1;
            proxy_set_header Upgrade $http_upgrade;
            proxy_set_header Connection "upgrade";
            proxy_set_header Host $host;
        }
    }
}
EOF
```

### Step 4: Configure Monitoring

```bash
# Create Prometheus configuration
mkdir -p prometheus
cat > prometheus/prometheus.prod.yml << 'EOF'
global:
  scrape_interval: 15s
  evaluation_interval: 15s

rule_files:
  - "alert_rules.yml"

alerting:
  alertmanagers:
    - static_configs:
        - targets: []

scrape_configs:
  - job_name: 'fo3-wallet-api'
    static_configs:
      - targets: ['fo3-wallet-api:9090']
    scrape_interval: 5s
    metrics_path: /metrics
    
  - job_name: 'postgres'
    static_configs:
      - targets: ['postgres:5432']
    
  - job_name: 'redis'
    static_configs:
      - targets: ['redis:6379']
EOF

# Create Grafana datasource configuration
mkdir -p grafana/datasources
cat > grafana/datasources/prometheus.yml << 'EOF'
apiVersion: 1

datasources:
  - name: Prometheus
    type: prometheus
    access: proxy
    url: http://prometheus:9090
    isDefault: true
EOF
```

### Step 5: Deploy Services

```bash
# Build and start production services
docker-compose -f docker-compose.production.yml up -d

# Wait for services to start
sleep 30

# Check service health
docker-compose -f docker-compose.production.yml ps
```

### Step 6: Verify Deployment

```bash
# Test gRPC health check
grpc_health_probe -addr=localhost:50051 -tls -tls-server-name=api.fo3wallet.com

# Test authentication
grpcurl -insecure \
  -d '{"email": "admin@fo3wallet.com", "password": "admin123"}' \
  api.fo3wallet.com:50051 fo3.wallet.v1.AuthService/Login

# Check service logs
docker-compose -f docker-compose.production.yml logs fo3-wallet-api
```

## 🔧 Configuration Management

### Environment Variables

**Security Configuration:**
```bash
# Generate secure secrets
JWT_SECRET=$(openssl rand -base64 64)
ENCRYPTION_KEY=$(openssl rand -base64 32)
KYC_ENCRYPTION_KEY=$(openssl rand -base64 32)

# Database passwords
POSTGRES_PASSWORD=$(openssl rand -base64 32)
REDIS_PASSWORD=$(openssl rand -base64 32)
GRAFANA_PASSWORD=$(openssl rand -base64 16)
```

**Blockchain Configuration:**
```bash
# Ethereum (Required)
ETHEREUM_RPC_URL=https://mainnet.infura.io/v3/YOUR_PROJECT_ID
ETHEREUM_API_KEY=YOUR_INFURA_API_KEY

# Solana (Required)
SOLANA_RPC_URL=https://api.mainnet-beta.solana.com

# Optional networks
POLYGON_RPC_URL=https://polygon-mainnet.infura.io/v3/YOUR_PROJECT_ID
BSC_RPC_URL=https://bsc-dataseed1.binance.org/
ARBITRUM_RPC_URL=https://arbitrum-mainnet.infura.io/v3/YOUR_PROJECT_ID
```

### SSL Certificate Management

**Using Let's Encrypt:**
```bash
# Install certbot
sudo apt-get install certbot

# Generate certificates
sudo certbot certonly --standalone \
  -d api.fo3wallet.com \
  -d ws.fo3wallet.com \
  --email admin@fo3wallet.com \
  --agree-tos

# Copy certificates
sudo cp /etc/letsencrypt/live/api.fo3wallet.com/fullchain.pem /opt/fo3/certs/server.crt
sudo cp /etc/letsencrypt/live/api.fo3wallet.com/privkey.pem /opt/fo3/certs/server.key

# Set up auto-renewal
sudo crontab -e
# Add: 0 12 * * * /usr/bin/certbot renew --quiet
```

## 📊 Monitoring and Alerting

### Grafana Dashboards

Access Grafana at `https://monitoring.fo3wallet.com:3000`

**Default Dashboards:**
- FO3 Wallet Core Overview
- gRPC Service Metrics
- Database Performance
- System Resources
- Error Rates and Latency

### Prometheus Alerts

```yaml
# prometheus/alert_rules.yml
groups:
  - name: fo3_wallet_alerts
    rules:
      - alert: HighErrorRate
        expr: rate(grpc_server_handled_total{grpc_code!="OK"}[5m]) > 0.1
        for: 2m
        labels:
          severity: warning
        annotations:
          summary: "High error rate detected"
          
      - alert: DatabaseDown
        expr: up{job="postgres"} == 0
        for: 1m
        labels:
          severity: critical
        annotations:
          summary: "PostgreSQL database is down"
          
      - alert: HighMemoryUsage
        expr: (node_memory_MemTotal_bytes - node_memory_MemAvailable_bytes) / node_memory_MemTotal_bytes > 0.9
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "High memory usage detected"
```

### Log Management

```bash
# Configure log rotation
sudo cat > /etc/logrotate.d/fo3-wallet << 'EOF'
/opt/fo3/logs/*.log {
    daily
    missingok
    rotate 30
    compress
    delaycompress
    notifempty
    create 644 root root
    postrotate
        docker-compose -f /opt/fo3/fo3-wallet-core/docker-compose.production.yml restart fo3-wallet-api
    endscript
}
EOF
```

## 🔄 Backup and Recovery

### Database Backup

```bash
# Create backup script
cat > /opt/fo3/scripts/backup.sh << 'EOF'
#!/bin/bash
BACKUP_DIR="/opt/fo3/backups"
DATE=$(date +%Y%m%d_%H%M%S)

# PostgreSQL backup
docker exec fo3-postgres-prod pg_dump -U fo3_user fo3_wallet > "$BACKUP_DIR/postgres_$DATE.sql"

# Compress backup
gzip "$BACKUP_DIR/postgres_$DATE.sql"

# Remove backups older than 30 days
find "$BACKUP_DIR" -name "postgres_*.sql.gz" -mtime +30 -delete

echo "Backup completed: postgres_$DATE.sql.gz"
EOF

chmod +x /opt/fo3/scripts/backup.sh

# Schedule daily backups
sudo crontab -e
# Add: 0 2 * * * /opt/fo3/scripts/backup.sh
```

### Disaster Recovery

```bash
# Restore from backup
gunzip /opt/fo3/backups/postgres_20240115_020000.sql.gz
docker exec -i fo3-postgres-prod psql -U fo3_user fo3_wallet < /opt/fo3/backups/postgres_20240115_020000.sql
```

## 🔒 Security Hardening

### System Security

```bash
# Update system packages
sudo apt-get update && sudo apt-get upgrade -y

# Install fail2ban
sudo apt-get install fail2ban -y

# Configure SSH security
sudo sed -i 's/#PermitRootLogin yes/PermitRootLogin no/' /etc/ssh/sshd_config
sudo sed -i 's/#PasswordAuthentication yes/PasswordAuthentication no/' /etc/ssh/sshd_config
sudo systemctl restart ssh
```

### Application Security

```bash
# Set secure file permissions
sudo chmod 600 .env.production
sudo chmod 600 /opt/fo3/certs/server.key
sudo chown root:root /opt/fo3/certs/*

# Configure Docker security
echo '{"log-driver": "json-file", "log-opts": {"max-size": "10m", "max-file": "3"}}' | sudo tee /etc/docker/daemon.json
sudo systemctl restart docker
```

## 🚨 Troubleshooting

### Common Issues

**Service Won't Start:**
```bash
# Check logs
docker-compose -f docker-compose.production.yml logs fo3-wallet-api

# Check disk space
df -h

# Check memory usage
free -h
```

**Database Connection Issues:**
```bash
# Test database connectivity
docker exec fo3-postgres-prod psql -U fo3_user -d fo3_wallet -c "SELECT 1;"

# Check database logs
docker-compose -f docker-compose.production.yml logs postgres
```

**SSL Certificate Issues:**
```bash
# Verify certificate
openssl x509 -in /opt/fo3/certs/server.crt -text -noout

# Test SSL connection
openssl s_client -connect api.fo3wallet.com:443
```

### Performance Tuning

```bash
# Optimize PostgreSQL
echo "shared_preload_libraries = 'pg_stat_statements'" >> /opt/fo3/data/postgres/postgresql.conf
echo "max_connections = 200" >> /opt/fo3/data/postgres/postgresql.conf
echo "shared_buffers = 256MB" >> /opt/fo3/data/postgres/postgresql.conf

# Optimize Redis
echo "maxmemory 1gb" >> /opt/fo3/data/redis/redis.conf
echo "maxmemory-policy allkeys-lru" >> /opt/fo3/data/redis/redis.conf
```

## 📚 Additional Resources

- [API Reference](./API_REFERENCE.md)
- [Mobile Integration Guide](./MOBILE_INTEGRATION.md)
- [Development Setup](./DEVELOPMENT_SETUP.md)
- [Phase 2D Completion Report](./PHASE2D_COMPLETION_REPORT.md)

## 🆘 Support

For deployment support:
- GitHub Issues: [fo3-wallet-core/issues](https://github.com/bankchen1/fo3-wallet-core/issues)
- DevOps Team: devops@fo3wallet.com
- Emergency: +1-555-FO3-HELP
