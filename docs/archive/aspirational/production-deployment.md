# VexFS Production Deployment Guide

This guide covers enterprise-grade production deployment of VexFS with high availability, security, and performance optimization.

## Architecture Overview

### Production Architecture Components
```
┌─────────────────────────────────────────────────────────────┐
│                    Load Balancer                            │
│                  (HAProxy/NGINX)                           │
└─────────────────────┬───────────────────────────────────────┘
                      │
    ┌─────────────────┼─────────────────┐
    │                 │                 │
┌───▼───┐        ┌───▼───┐        ┌───▼───┐
│VexFS  │        │VexFS  │        │VexFS  │
│Node 1 │        │Node 2 │        │Node 3 │
│(Master)│       │(Replica)│      │(Replica)│
└───┬───┘        └───┬───┘        └───┬───┘
    │                │                │
    └────────────────┼────────────────┘
                     │
    ┌────────────────▼────────────────┐
    │         Shared Storage          │
    │    (SAN/NFS/Distributed FS)     │
    └─────────────────────────────────┘
```

### Key Components
- **Load Balancer**: Traffic distribution and health checking
- **VexFS Cluster**: 3+ nodes for high availability
- **Shared Storage**: Persistent data storage backend
- **Monitoring**: Comprehensive observability stack
- **Backup**: Automated backup and recovery system

## Pre-Deployment Planning

### 1. Capacity Planning

#### Hardware Requirements (Per Node)
```yaml
Minimum Production:
  CPU: 8 cores (Intel Xeon or AMD EPYC)
  Memory: 32GB RAM
  Storage: 1TB NVMe SSD
  Network: 10Gbps Ethernet

Recommended Production:
  CPU: 16+ cores with AVX2 support
  Memory: 64GB+ RAM
  Storage: 2TB+ NVMe SSD (RAID 1)
  Network: 25Gbps+ Ethernet

High-Performance:
  CPU: 32+ cores with AVX-512
  Memory: 128GB+ RAM
  Storage: 4TB+ NVMe SSD (RAID 10)
  Network: 100Gbps Ethernet
```

#### Storage Planning
```bash
# Calculate storage requirements
Data Volume: [Expected data size] × 1.5 (growth factor)
Index Volume: Data Volume × 0.3 (vector indexes)
Log Volume: 100GB minimum
Backup Volume: (Data + Index) × 3 (retention)
Total: Data + Index + Log + Backup + 20% overhead
```

### 2. Network Planning

#### Port Requirements
```yaml
VexFS Services:
  - 8080/tcp: REST API
  - 8081/tcp: WebSocket API
  - 9090/tcp: Metrics endpoint
  - 9091/tcp: Health check endpoint

Cluster Communication:
  - 7000/tcp: Inter-node communication
  - 7001/tcp: Cluster management
  - 7002/tcp: Data replication

Monitoring:
  - 9100/tcp: Node exporter
  - 9187/tcp: VexFS exporter
```

#### Network Security
```bash
# Firewall rules (iptables example)
# Allow VexFS API traffic
iptables -A INPUT -p tcp --dport 8080:8081 -j ACCEPT

# Allow cluster communication (internal network only)
iptables -A INPUT -s 10.0.0.0/8 -p tcp --dport 7000:7002 -j ACCEPT

# Allow monitoring (monitoring network only)
iptables -A INPUT -s 10.1.0.0/16 -p tcp --dport 9090:9100 -j ACCEPT
```

## Deployment Procedures

### 1. Infrastructure Setup

#### System Preparation
```bash
# Update system
sudo apt update && sudo apt upgrade -y

# Install required packages
sudo apt install -y \
  curl wget gnupg2 software-properties-common \
  build-essential linux-headers-$(uname -r) \
  fuse3 libfuse3-dev pkg-config

# Configure system limits
sudo tee /etc/security/limits.d/vexfs.conf << EOF
vexfs soft nofile 65536
vexfs hard nofile 65536
vexfs soft nproc 32768
vexfs hard nproc 32768
EOF

# Configure kernel parameters
sudo tee /etc/sysctl.d/99-vexfs.conf << EOF
# Network optimization
net.core.rmem_max = 134217728
net.core.wmem_max = 134217728
net.ipv4.tcp_rmem = 4096 87380 134217728
net.ipv4.tcp_wmem = 4096 65536 134217728

# File system optimization
fs.file-max = 2097152
vm.swappiness = 1
vm.dirty_ratio = 15
vm.dirty_background_ratio = 5
EOF

sudo sysctl -p /etc/sysctl.d/99-vexfs.conf
```

#### Storage Configuration
```bash
# Format dedicated storage (example for /dev/nvme0n1)
sudo parted /dev/nvme0n1 mklabel gpt
sudo parted /dev/nvme0n1 mkpart primary 0% 100%

# Create filesystem
sudo mkfs.ext4 -F /dev/nvme0n1p1

# Mount storage
sudo mkdir -p /var/lib/vexfs
echo "/dev/nvme0n1p1 /var/lib/vexfs ext4 defaults,noatime 0 2" | sudo tee -a /etc/fstab
sudo mount /var/lib/vexfs

# Set permissions
sudo chown -R vexfs:vexfs /var/lib/vexfs
sudo chmod 755 /var/lib/vexfs
```

### 2. VexFS Installation

#### Package Installation
```bash
# Add VexFS repository
curl -fsSL https://packages.vexfs.io/gpg | sudo apt-key add -
echo "deb https://packages.vexfs.io/ubuntu $(lsb_release -cs) main" | sudo tee /etc/apt/sources.list.d/vexfs.list

# Install VexFS packages
sudo apt update
sudo apt install -y vexfs-server vexfs-tools vexfs-monitoring
```

#### Configuration
```bash
# Create production configuration
sudo tee /etc/vexfs/vexfs.conf << EOF
[general]
node_id = "vexfs-node-01"
cluster_name = "vexfs-production"
data_dir = "/var/lib/vexfs/data"
log_dir = "/var/log/vexfs"
log_level = "info"

[cluster]
enabled = true
bind_address = "0.0.0.0:7000"
advertise_address = "10.0.1.10:7000"
seed_nodes = [
  "10.0.1.10:7000",
  "10.0.1.11:7000",
  "10.0.1.12:7000"
]
replication_factor = 3

[storage]
backend = "local"
cache_size = "16GB"
compression = "lz4"
encryption = true
encryption_key_file = "/etc/vexfs/encryption.key"

[performance]
memory_pool_size = "8GB"
enable_simd = true
max_concurrent_operations = 1000
io_threads = 16

[api]
rest_enabled = true
rest_bind = "0.0.0.0:8080"
websocket_enabled = true
websocket_bind = "0.0.0.0:8081"
tls_enabled = true
tls_cert_file = "/etc/vexfs/ssl/server.crt"
tls_key_file = "/etc/vexfs/ssl/server.key"

[monitoring]
metrics_enabled = true
metrics_bind = "0.0.0.0:9090"
health_check_bind = "0.0.0.0:9091"
prometheus_enabled = true
EOF
```

#### SSL Certificate Setup
```bash
# Generate SSL certificates (or use existing CA)
sudo mkdir -p /etc/vexfs/ssl

# Self-signed certificate (for testing)
sudo openssl req -x509 -nodes -days 365 -newkey rsa:2048 \
  -keyout /etc/vexfs/ssl/server.key \
  -out /etc/vexfs/ssl/server.crt \
  -subj "/C=US/ST=State/L=City/O=Organization/CN=vexfs.example.com"

# Set permissions
sudo chown -R vexfs:vexfs /etc/vexfs/ssl
sudo chmod 600 /etc/vexfs/ssl/server.key
sudo chmod 644 /etc/vexfs/ssl/server.crt
```

#### Encryption Key Setup
```bash
# Generate encryption key
sudo openssl rand -base64 32 > /etc/vexfs/encryption.key
sudo chown vexfs:vexfs /etc/vexfs/encryption.key
sudo chmod 600 /etc/vexfs/encryption.key
```

### 3. Service Configuration

#### Systemd Service
```bash
# VexFS service is installed with package, verify configuration
sudo systemctl cat vexfs

# Enable and start service
sudo systemctl enable vexfs
sudo systemctl start vexfs

# Check status
sudo systemctl status vexfs
```

#### Service Monitoring
```bash
# Configure service monitoring
sudo tee /etc/systemd/system/vexfs.service.d/monitoring.conf << EOF
[Service]
ExecStartPost=/bin/sleep 10
ExecStartPost=/usr/local/bin/vexfs-health-check
Restart=always
RestartSec=10
EOF

sudo systemctl daemon-reload
sudo systemctl restart vexfs
```

### 4. Load Balancer Configuration

#### HAProxy Configuration
```bash
# Install HAProxy
sudo apt install -y haproxy

# Configure HAProxy
sudo tee /etc/haproxy/haproxy.cfg << EOF
global
    daemon
    maxconn 4096
    log stdout local0

defaults
    mode http
    timeout connect 5000ms
    timeout client 50000ms
    timeout server 50000ms
    option httplog

frontend vexfs_api
    bind *:80
    bind *:443 ssl crt /etc/ssl/certs/vexfs.pem
    redirect scheme https if !{ ssl_fc }
    default_backend vexfs_nodes

backend vexfs_nodes
    balance roundrobin
    option httpchk GET /health
    server vexfs-node-01 10.0.1.10:8080 check
    server vexfs-node-02 10.0.1.11:8080 check
    server vexfs-node-03 10.0.1.12:8080 check

frontend vexfs_websocket
    bind *:8081
    default_backend vexfs_websocket_nodes

backend vexfs_websocket_nodes
    balance source
    server vexfs-node-01 10.0.1.10:8081 check
    server vexfs-node-02 10.0.1.11:8081 check
    server vexfs-node-03 10.0.1.12:8081 check
EOF

# Start HAProxy
sudo systemctl enable haproxy
sudo systemctl start haproxy
```

## Cluster Setup

### 1. Multi-Node Deployment

#### Node 1 (Master) Setup
```bash
# Initialize cluster
sudo vexctl cluster init --node-id vexfs-node-01

# Check cluster status
sudo vexctl cluster status
```

#### Additional Nodes Setup
```bash
# Join cluster (run on nodes 2 and 3)
sudo vexctl cluster join --seed-node 10.0.1.10:7000 --node-id vexfs-node-02

# Verify cluster membership
sudo vexctl cluster members
```

### 2. Data Replication Setup
```bash
# Configure replication
sudo vexctl replication configure \
  --replication-factor 3 \
  --consistency-level quorum

# Verify replication status
sudo vexctl replication status
```

## Security Hardening

### 1. Access Control
```bash
# Create service account
sudo useradd -r -s /bin/false vexfs-service

# Configure RBAC
sudo tee /etc/vexfs/rbac.yaml << EOF
roles:
  - name: admin
    permissions: ["*"]
  - name: read-only
    permissions: ["read", "search"]
  - name: api-user
    permissions: ["read", "write", "search"]

users:
  - name: admin
    roles: ["admin"]
    auth_method: "certificate"
  - name: api-service
    roles: ["api-user"]
    auth_method: "token"
EOF
```

### 2. Network Security
```bash
# Configure firewall
sudo ufw enable
sudo ufw allow from 10.0.0.0/8 to any port 7000:7002
sudo ufw allow from 10.1.0.0/16 to any port 9090:9100
sudo ufw allow 80,443,8080,8081
```

### 3. Audit Logging
```bash
# Enable audit logging
sudo tee -a /etc/vexfs/vexfs.conf << EOF

[audit]
enabled = true
log_file = "/var/log/vexfs/audit.log"
log_level = "info"
include_data = false
retention_days = 90
EOF
```

## Monitoring Setup

### 1. Prometheus Configuration
```yaml
# /etc/prometheus/prometheus.yml
global:
  scrape_interval: 15s

scrape_configs:
  - job_name: 'vexfs'
    static_configs:
      - targets: ['10.0.1.10:9090', '10.0.1.11:9090', '10.0.1.12:9090']
    scrape_interval: 5s
    metrics_path: /metrics

  - job_name: 'vexfs-health'
    static_configs:
      - targets: ['10.0.1.10:9091', '10.0.1.11:9091', '10.0.1.12:9091']
    scrape_interval: 10s
    metrics_path: /health
```

### 2. Grafana Dashboards
```bash
# Import VexFS dashboard
curl -X POST \
  http://grafana:3000/api/dashboards/db \
  -H "Content-Type: application/json" \
  -d @/etc/vexfs/grafana-dashboard.json
```

## Backup Configuration

### 1. Automated Backup
```bash
# Create backup script
sudo tee /usr/local/bin/vexfs-backup.sh << 'EOF'
#!/bin/bash
BACKUP_DIR="/backup/vexfs"
DATE=$(date +%Y%m%d_%H%M%S)

# Create backup
vexctl backup create \
  --output "$BACKUP_DIR/vexfs_backup_$DATE.tar.gz" \
  --compress \
  --verify

# Cleanup old backups (keep 30 days)
find "$BACKUP_DIR" -name "vexfs_backup_*.tar.gz" -mtime +30 -delete
EOF

sudo chmod +x /usr/local/bin/vexfs-backup.sh

# Schedule backup
echo "0 2 * * * /usr/local/bin/vexfs-backup.sh" | sudo crontab -u vexfs -
```

## Validation and Testing

### 1. Deployment Validation
```bash
# Health check
curl -f http://localhost:9091/health

# API test
curl -X GET http://localhost:8080/api/v1/status

# Cluster status
sudo vexctl cluster status

# Performance test
sudo vexctl benchmark run --duration 60s
```

### 2. Failover Testing
```bash
# Test node failure
sudo systemctl stop vexfs  # on one node

# Verify cluster health
sudo vexctl cluster status

# Test load balancer failover
curl -f http://load-balancer/health
```

## Maintenance Procedures

### 1. Rolling Updates
```bash
# Update one node at a time
sudo systemctl stop vexfs
sudo apt update && sudo apt upgrade vexfs-server
sudo systemctl start vexfs

# Verify cluster health after each node
sudo vexctl cluster status
```

### 2. Performance Monitoring
```bash
# Monitor performance metrics
sudo vexctl metrics show

# Generate performance report
sudo vexctl performance report --output /tmp/performance-report.html
```

## Troubleshooting

### Common Issues

#### Cluster Split-Brain
```bash
# Check cluster status
sudo vexctl cluster status

# Force cluster recovery (if needed)
sudo vexctl cluster recover --force-quorum
```

#### Performance Degradation
```bash
# Check resource usage
htop
iostat -x 1

# Analyze slow queries
sudo vexctl query analyze --slow-threshold 1000ms

# Check cache hit rates
sudo vexctl cache stats
```

#### Storage Issues
```bash
# Check disk space
df -h /var/lib/vexfs

# Check disk I/O
iostat -x 1

# Verify data integrity
sudo vexctl fsck --verify-checksums
```

## Support and Escalation

### Support Contacts
- **Level 1**: Operations team (24/7)
- **Level 2**: VexFS engineering team
- **Level 3**: Vendor support (if applicable)

### Escalation Procedures
1. **Critical Issues**: Immediate escalation to Level 2
2. **Performance Issues**: Collect diagnostics, escalate to Level 2
3. **Data Corruption**: Stop operations, escalate to Level 3

### Diagnostic Collection
```bash
# Collect diagnostic bundle
sudo vexctl diagnostics collect --output /tmp/vexfs-diagnostics.tar.gz

# Include system information
sudo vexctl diagnostics system-info >> /tmp/system-info.txt
```

This production deployment guide provides a comprehensive foundation for enterprise VexFS deployments with high availability, security, and operational excellence.