# Deployment Guide

Deploy VexFS v1.0 in testing environments with comprehensive reliability, security, and performance features.

## 🎯 Production Architecture

### Recommended Production Setup

```
┌─────────────────────────────────────────────────────────────┐
│                    Load Balancer (HAProxy/Nginx)           │
├─────────────────────────────────────────────────────────────┤
│  VexFS Cluster                                             │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐        │
│  │   VexFS     │  │   VexFS     │  │   VexFS     │        │
│  │  Instance 1 │  │  Instance 2 │  │  Instance 3 │        │
│  │             │  │             │  │             │        │
│  └─────────────┘  └─────────────┘  └─────────────┘        │
├─────────────────────────────────────────────────────────────┤
│                    Shared Storage (NFS/GlusterFS)          │
├─────────────────────────────────────────────────────────────┤
│                    Monitoring & Logging                    │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐        │
│  │ Prometheus  │  │   Grafana   │  │    ELK      │        │
│  │             │  │             │  │   Stack     │        │
│  └─────────────┘  └─────────────┘  └─────────────┘        │
└─────────────────────────────────────────────────────────────┘
```

### Hardware Requirements

#### Minimum Production Requirements

| Component | Specification |
|-----------|---------------|
| **CPU** | 8 cores, 3.0GHz+ |
| **Memory** | 32GB RAM |
| **Storage** | 1TB NVMe SSD |
| **Network** | 1Gbps |
| **OS** | Ubuntu 20.04+ LTS |

#### Recommended High-Performance Setup

| Component | Specification |
|-----------|---------------|
| **CPU** | 16+ cores, 3.5GHz+ (Intel Xeon/AMD EPYC) |
| **Memory** | 64GB+ RAM |
| **Storage** | 2TB+ NVMe SSD (RAID 1) |
| **Network** | 10Gbps |
| **OS** | Ubuntu 22.04 LTS |

## 🚀 Deployment Methods

### Method 1: Docker Production Deployment

#### Production Docker Compose

```yaml
# docker-compose.prod.yml
version: '3.8'

services:
  vexfs-server:
    image: vexfs/vexfs:1.0.0
    restart: unless-stopped
    ports:
      - "8000:8000"
    volumes:
      - vexfs_data:/data
      - vexfs_config:/etc/vexfs
      - vexfs_logs:/var/log/vexfs
    environment:
      - RUST_LOG=info
      - VEXFS_DATA_DIR=/data
      - VEXFS_CONFIG_FILE=/etc/vexfs/config.toml
      - VEXFS_CACHE_SIZE=16GB
      - VEXFS_WORKER_THREADS=16
      - VEXFS_MAX_CONNECTIONS=2000
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8000/api/v1/version"]
      interval: 30s
      timeout: 10s
      retries: 3
      start_period: 40s
    deploy:
      resources:
        limits:
          memory: 32G
          cpus: '16'
        reservations:
          memory: 16G
          cpus: '8'

  nginx:
    image: nginx:alpine
    restart: unless-stopped
    ports:
      - "80:80"
      - "443:443"
    volumes:
      - ./nginx.conf:/etc/nginx/nginx.conf
      - ./ssl:/etc/nginx/ssl
      - nginx_logs:/var/log/nginx
    depends_on:
      - vexfs-server

volumes:
  vexfs_data:
    driver: local
    driver_opts:
      type: none
      o: bind
      device: /opt/vexfs/data
  vexfs_config:
    driver: local
    driver_opts:
      type: none
      o: bind
      device: /opt/vexfs/config
  vexfs_logs:
    driver: local
    driver_opts:
      type: none
      o: bind
      device: /opt/vexfs/logs
  nginx_logs:
```

#### Production Configuration

```toml
# /opt/vexfs/config/config.toml
[server]
bind_address = "0.0.0.0"
port = 8000
data_directory = "/data"
max_connections = 2000
request_timeout = 30

[vector]
default_dimension = 384
cache_size = "16GB"
index_type = "hnsw"
hnsw_m = 16
hnsw_ef_construction = 200
hnsw_ef_search = 100

[performance]
worker_threads = 16
batch_size = 2000
async_writes = true
memory_map = true
preload_index = true
compression = "zstd"
compression_level = 3

[security]
enable_tls = true
cert_file = "/etc/vexfs/ssl/cert.pem"
key_file = "/etc/vexfs/ssl/key.pem"
require_auth = true
auth_token_file = "/etc/vexfs/auth_tokens"

[logging]
level = "info"
file = "/var/log/vexfs/vexfs.log"
max_size = "100MB"
max_files = 10
json_format = true

[monitoring]
enable_metrics = true
metrics_port = 9091
health_check_interval = 30

[backup]
enable_auto_backup = true
backup_interval = "24h"
backup_directory = "/data/backups"
retention_days = 30
```

## 🔒 Security Configuration

### SSL/TLS Setup

```bash
# Generate production certificates
sudo mkdir -p /etc/vexfs/ssl

# Option 1: Let's Encrypt (recommended)
sudo apt install certbot
sudo certbot certonly --standalone -d vexfs.yourdomain.com

# Copy certificates
sudo cp /etc/letsencrypt/live/vexfs.yourdomain.com/fullchain.pem /etc/vexfs/ssl/cert.pem
sudo cp /etc/letsencrypt/live/vexfs.yourdomain.com/privkey.pem /etc/vexfs/ssl/key.pem

# Set permissions
sudo chown vexfs:vexfs /etc/vexfs/ssl/*
sudo chmod 600 /etc/vexfs/ssl/key.pem
sudo chmod 644 /etc/vexfs/ssl/cert.pem
```

### Authentication Setup

```bash
# Create API tokens
sudo tee /etc/vexfs/auth_tokens << EOF
# Format: token_name:hashed_token:permissions
admin:$(echo -n "admin_secret_token" | sha256sum | cut -d' ' -f1):read,write,admin
readonly:$(echo -n "readonly_token" | sha256sum | cut -d' ' -f1):read
service:$(echo -n "service_token" | sha256sum | cut -d' ' -f1):read,write
EOF

sudo chown vexfs:vexfs /etc/vexfs/auth_tokens
sudo chmod 600 /etc/vexfs/auth_tokens
```

## 📊 Monitoring and Observability

### Prometheus Configuration

```yaml
# prometheus.yml
global:
  scrape_interval: 15s
  evaluation_interval: 15s

scrape_configs:
  - job_name: 'vexfs'
    static_configs:
      - targets: ['localhost:9091']
    scrape_interval: 5s
    metrics_path: /metrics

  - job_name: 'node'
    static_configs:
      - targets: ['localhost:9100']
```

## 🔄 Backup and Recovery

### Automated Backup Script

```bash
#!/bin/bash
# /opt/vexfs/scripts/backup.sh

set -e

BACKUP_DIR="/opt/vexfs/backups"
DATA_DIR="/opt/vexfs/data"
RETENTION_DAYS=30
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
BACKUP_NAME="vexfs_backup_${TIMESTAMP}"

echo "Starting VexFS backup: ${BACKUP_NAME}"

# Create backup directory
mkdir -p "${BACKUP_DIR}/${BACKUP_NAME}"

# Create data backup
echo "Backing up data directory..."
tar -czf "${BACKUP_DIR}/${BACKUP_NAME}/data.tar.gz" -C "${DATA_DIR}" .

# Backup configuration
echo "Backing up configuration..."
cp -r /etc/vexfs "${BACKUP_DIR}/${BACKUP_NAME}/config"

# Cleanup old backups
echo "Cleaning up old backups..."
find "${BACKUP_DIR}" -name "vexfs_backup_*" -type d -mtime +${RETENTION_DAYS} -exec rm -rf {} \;

echo "Backup completed: ${BACKUP_NAME}"
```

## 🚀 Performance Optimization

### System Tuning

```bash
#!/bin/bash
# /opt/vexfs/scripts/tune-system.sh

echo "Tuning system for VexFS production..."

# Kernel parameters
sudo tee -a /etc/sysctl.conf << EOF
# VexFS optimizations
vm.swappiness = 1
vm.dirty_ratio = 15
vm.dirty_background_ratio = 5
net.core.somaxconn = 65535
fs.file-max = 2097152
EOF

# Apply changes
sudo sysctl -p

echo "System tuning complete. Reboot recommended."
```

## 🎯 Next Steps

After deployment:

1. **[Security Configuration](security.md)** - Implement comprehensive security
2. **[Monitoring Setup](monitoring.md)** - Set up comprehensive monitoring
3. **[Backup Strategy](backup.md)** - Configure automated backups
4. **[Performance Tuning](../user-guide/performance.md)** - Optimize for your workload

**Ready for testing!** VexFS v1.0 is now deployed with comprehensive reliability and performance features. 🚀