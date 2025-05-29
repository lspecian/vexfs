# VexFS v1.0 Production Deployment Guide

This directory contains all the production deployment artifacts for VexFS v1.0, making it ready for enterprise-grade deployment across various environments.

## 📁 Directory Structure

```
deployment/
├── docker/                     # Docker containers and orchestration
│   ├── Dockerfile.production   # Production-hardened Docker image
│   └── docker-compose.production.yml  # Complete production stack
├── kubernetes/                 # Kubernetes deployment
│   └── helm/                   # Helm charts for K8s deployment
│       └── vexfs/              # VexFS Helm chart
├── packages/                   # Linux distribution packages
│   ├── debian/                 # Debian/Ubuntu package files
│   ├── rpm/                    # RPM package files
│   ├── build-deb.sh           # Debian package builder
│   └── build-rpm.sh           # RPM package builder
├── scripts/                    # Installation and management scripts
│   ├── install.sh             # Bare metal installation
│   ├── uninstall.sh           # Complete removal script
│   ├── backup.sh              # Automated backup script
│   └── restore.sh             # Backup restoration script
└── README.md                  # This file
```

## 🚀 Deployment Methods

### 1. Docker Deployment (Recommended for Development/Testing)

#### Quick Start with Docker Compose

```bash
# Clone the repository
git clone https://github.com/vexfs/vexfs.git
cd vexfs

# Start the production stack
cd deployment/docker
docker-compose -f docker-compose.production.yml up -d

# Check status
docker-compose -f docker-compose.production.yml ps

# View logs
docker-compose -f docker-compose.production.yml logs -f vexfs-server
```

#### Production Docker Features

- **Security Hardened**: Non-root user, distroless base image, minimal attack surface
- **Multi-stage Build**: Optimized image size and build caching
- **Health Checks**: Built-in health monitoring
- **Resource Limits**: CPU and memory constraints
- **Monitoring Stack**: Prometheus, Grafana, and log aggregation
- **TLS Termination**: Nginx reverse proxy with SSL support

#### Environment Variables

```bash
# Core Configuration
RUST_LOG=info
PORT=8000
VEXFS_DATA_DIR=/data
VEXFS_LOG_LEVEL=info

# Performance Tuning
VEXFS_MAX_CONNECTIONS=1000
VEXFS_REQUEST_TIMEOUT=30s
VEXFS_RATE_LIMIT_REQUESTS=100
VEXFS_RATE_LIMIT_WINDOW=60s

# Security
VEXFS_TLS_ENABLED=false
VEXFS_CORS_ENABLED=true

# Monitoring
VEXFS_METRICS_ENABLED=true
VEXFS_HEALTH_CHECK_ENABLED=true
```

### 2. Kubernetes Deployment (Recommended for Production)

#### Prerequisites

- Kubernetes cluster (1.20+)
- Helm 3.0+
- kubectl configured

#### Installation

```bash
# Add VexFS Helm repository (when published)
helm repo add vexfs https://charts.vexfs.org
helm repo update

# Or install from local chart
cd deployment/kubernetes/helm

# Install VexFS
helm install vexfs ./vexfs \
  --namespace vexfs \
  --create-namespace \
  --values values-production.yaml

# Check deployment
kubectl get pods -n vexfs
kubectl get services -n vexfs
```

#### Kubernetes Features

- **High Availability**: Multi-replica deployment with anti-affinity
- **Auto-scaling**: Horizontal Pod Autoscaler based on CPU/memory
- **Load Balancing**: Service mesh integration
- **Persistent Storage**: StatefulSet with PVC for data persistence
- **Security**: Pod Security Standards, Network Policies, RBAC
- **Monitoring**: ServiceMonitor for Prometheus integration
- **Ingress**: TLS termination and routing

#### Configuration

```yaml
# values-production.yaml
vexfs:
  replicaCount: 3
  
  resources:
    limits:
      cpu: 2000m
      memory: 2Gi
    requests:
      cpu: 500m
      memory: 512Mi
  
  autoscaling:
    enabled: true
    minReplicas: 3
    maxReplicas: 10
    targetCPUUtilizationPercentage: 70

  ingress:
    enabled: true
    className: "nginx"
    hosts:
      - host: vexfs.yourdomain.com
        paths:
          - path: /
            pathType: Prefix
    tls:
      - secretName: vexfs-tls
        hosts:
          - vexfs.yourdomain.com

persistence:
  enabled: true
  size: 100Gi
  storageClass: "fast-ssd"
```

### 3. Linux Package Installation

#### Debian/Ubuntu

```bash
# Download and install .deb package
cd deployment/packages
sudo ./build-deb.sh

# Install the package
sudo dpkg -i build/vexfs_1.0.0_amd64.deb
sudo apt-get install -f  # Fix any dependency issues

# Start the service
sudo systemctl start vexfs
sudo systemctl enable vexfs

# Check status
sudo systemctl status vexfs
```

#### Red Hat/CentOS/Fedora

```bash
# Build and install RPM package
cd deployment/packages
sudo ./build-rpm.sh

# Install the package
sudo rpm -ivh rpmbuild/RPMS/x86_64/vexfs-1.0.0-1.x86_64.rpm
# or
sudo dnf install rpmbuild/RPMS/x86_64/vexfs-1.0.0-1.x86_64.rpm

# Start the service
sudo systemctl start vexfs
sudo systemctl enable vexfs
```

#### Package Features

- **Systemd Integration**: Automatic service management
- **User Management**: Dedicated vexfs user and group
- **Security Hardening**: Proper file permissions and SELinux/AppArmor support
- **Log Rotation**: Automatic log management
- **Configuration Management**: Default configuration with environment overrides

### 4. Bare Metal Installation

#### Automated Installation

```bash
# Run the installation script
cd deployment/scripts
sudo ./install.sh

# The script will:
# - Check system requirements
# - Install dependencies
# - Create vexfs user and directories
# - Build and install VexFS binary
# - Configure systemd service
# - Set up log rotation
# - Start the service
```

#### Manual Installation Steps

1. **System Requirements**
   - Linux kernel 4.4+
   - 2GB+ RAM
   - 10GB+ disk space
   - systemd

2. **Dependencies**
   ```bash
   # Ubuntu/Debian
   sudo apt-get install curl wget ca-certificates openssl
   
   # CentOS/RHEL/Fedora
   sudo dnf install curl wget ca-certificates openssl
   ```

3. **Build VexFS**
   ```bash
   # Install Rust if not present
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   source ~/.cargo/env
   
   # Build VexFS
   cargo build --release --features server --bin vexfs_server
   sudo cp target/release/vexfs_server /usr/local/bin/
   ```

4. **Configuration**
   ```bash
   # Create directories and user
   sudo useradd -r -s /bin/false vexfs
   sudo mkdir -p /var/lib/vexfs /var/log/vexfs /etc/vexfs
   sudo chown vexfs:vexfs /var/lib/vexfs /var/log/vexfs
   
   # Create configuration
   sudo tee /etc/vexfs/vexfs.conf << EOF
   PORT=8000
   VEXFS_DATA_DIR=/var/lib/vexfs
   VEXFS_LOG_LEVEL=info
   EOF
   ```

## 🔧 Configuration Management

### Environment Variables

VexFS supports configuration through environment variables with the `VEXFS_` prefix:

| Variable | Default | Description |
|----------|---------|-------------|
| `VEXFS_DATA_DIR` | `/var/lib/vexfs` | Data storage directory |
| `VEXFS_LOG_LEVEL` | `info` | Logging level (error, warn, info, debug, trace) |
| `VEXFS_MAX_CONNECTIONS` | `1000` | Maximum concurrent connections |
| `VEXFS_REQUEST_TIMEOUT` | `30s` | Request timeout |
| `VEXFS_RATE_LIMIT_REQUESTS` | `100` | Rate limit requests per window |
| `VEXFS_RATE_LIMIT_WINDOW` | `60s` | Rate limit time window |
| `VEXFS_TLS_ENABLED` | `false` | Enable TLS encryption |
| `VEXFS_CORS_ENABLED` | `true` | Enable CORS headers |
| `VEXFS_METRICS_ENABLED` | `true` | Enable Prometheus metrics |
| `VEXFS_HEALTH_CHECK_ENABLED` | `true` | Enable health check endpoints |

### Configuration Files

- **Main Config**: `/etc/vexfs/vexfs.conf`
- **Systemd Service**: `/etc/systemd/system/vexfs.service`
- **Log Rotation**: `/etc/logrotate.d/vexfs`

### Security Configuration

#### TLS/SSL Setup

```bash
# Generate self-signed certificate (for testing)
sudo openssl req -x509 -nodes -days 365 -newkey rsa:2048 \
  -keyout /etc/vexfs/server.key \
  -out /etc/vexfs/server.crt

# Set proper permissions
sudo chown root:vexfs /etc/vexfs/server.*
sudo chmod 640 /etc/vexfs/server.*

# Enable TLS in configuration
echo "VEXFS_TLS_ENABLED=true" | sudo tee -a /etc/vexfs/vexfs.conf
echo "VEXFS_TLS_CERT=/etc/vexfs/server.crt" | sudo tee -a /etc/vexfs/vexfs.conf
echo "VEXFS_TLS_KEY=/etc/vexfs/server.key" | sudo tee -a /etc/vexfs/vexfs.conf
```

#### Firewall Configuration

```bash
# UFW (Ubuntu)
sudo ufw allow 8000/tcp
sudo ufw enable

# firewalld (CentOS/RHEL)
sudo firewall-cmd --permanent --add-port=8000/tcp
sudo firewall-cmd --reload

# iptables
sudo iptables -A INPUT -p tcp --dport 8000 -j ACCEPT
```

## 📊 Monitoring and Logging

### Health Checks

VexFS provides several health check endpoints:

- **Health**: `GET /health` - Basic health status
- **Readiness**: `GET /ready` - Service readiness check
- **Metrics**: `GET /metrics` - Prometheus metrics

### Prometheus Metrics

Available metrics include:

- `vexfs_requests_total` - Total requests processed
- `vexfs_response_time_seconds` - Average response time
- `vexfs_collections_total` - Number of collections
- `vexfs_documents_total` - Number of documents
- `vexfs_memory_usage_bytes` - Memory usage
- `vexfs_uptime_seconds` - Service uptime
- `vexfs_active_connections` - Active connections

### Log Management

#### Structured Logging

VexFS uses structured JSON logging:

```json
{
  "timestamp": "2024-05-29T10:30:00Z",
  "level": "INFO",
  "target": "vexfs_server",
  "message": "Request processed",
  "fields": {
    "method": "POST",
    "path": "/api/v1/collections",
    "status": 200,
    "duration_ms": 45
  }
}
```

#### Log Rotation

Automatic log rotation is configured:

```bash
# View log rotation config
cat /etc/logrotate.d/vexfs

# Manual log rotation
sudo logrotate -f /etc/logrotate.d/vexfs
```

#### Viewing Logs

```bash
# Systemd journal
sudo journalctl -u vexfs -f

# Log files
sudo tail -f /var/log/vexfs/vexfs.log

# Docker logs
docker logs -f vexfs-server

# Kubernetes logs
kubectl logs -f deployment/vexfs -n vexfs
```

## 💾 Backup and Recovery

### Automated Backups

```bash
# Run backup script
sudo ./deployment/scripts/backup.sh

# Schedule daily backups
echo "0 2 * * * root /path/to/deployment/scripts/backup.sh" | sudo tee -a /etc/crontab

# List available backups
sudo ./deployment/scripts/backup.sh --list
```

### Backup Types

- **Daily**: Created every day, retained for 30 days
- **Weekly**: Created on Sundays, retained for 12 weeks
- **Monthly**: Created on the 1st of each month, retained for 12 months

### Restore Process

```bash
# List available backups
sudo ./deployment/scripts/restore.sh --help

# Dry run restore
sudo ./deployment/scripts/restore.sh --dry-run backup_file.tar.gz

# Perform restore
sudo ./deployment/scripts/restore.sh /opt/vexfs/backups/daily/vexfs_daily_20240529_120000.tar.gz
```

## 🔒 Security Best Practices

### System Security

1. **User Isolation**: VexFS runs as dedicated non-root user
2. **File Permissions**: Strict file and directory permissions
3. **Systemd Security**: Comprehensive systemd security features
4. **Network Security**: Firewall configuration and network policies

### Application Security

1. **Rate Limiting**: Built-in request rate limiting
2. **Input Validation**: Comprehensive input validation
3. **CORS Configuration**: Configurable CORS policies
4. **TLS Encryption**: Optional TLS/SSL support

### Container Security

1. **Distroless Images**: Minimal attack surface
2. **Non-root Execution**: Container runs as non-root user
3. **Read-only Filesystem**: Immutable container filesystem
4. **Security Scanning**: Automated vulnerability scanning

## 🚨 Troubleshooting

### Common Issues

#### Service Won't Start

```bash
# Check service status
sudo systemctl status vexfs

# View detailed logs
sudo journalctl -u vexfs -n 50

# Check configuration
sudo vexfs_server --check-config
```

#### Port Already in Use

```bash
# Check what's using the port
sudo netstat -tlnp | grep :8000
sudo ss -tlnp | grep :8000

# Change port in configuration
sudo sed -i 's/PORT=8000/PORT=8001/' /etc/vexfs/vexfs.conf
sudo systemctl restart vexfs
```

#### Permission Denied

```bash
# Fix ownership
sudo chown -R vexfs:vexfs /var/lib/vexfs
sudo chown -R vexfs:vexfs /var/log/vexfs

# Fix permissions
sudo chmod 750 /var/lib/vexfs
sudo chmod 750 /var/log/vexfs
```

#### High Memory Usage

```bash
# Check memory usage
free -h
ps aux | grep vexfs_server

# Adjust memory limits in systemd
sudo systemctl edit vexfs
# Add:
# [Service]
# MemoryLimit=1G
```

### Performance Tuning

#### System Limits

```bash
# Increase file descriptor limits
echo "vexfs soft nofile 65536" | sudo tee -a /etc/security/limits.conf
echo "vexfs hard nofile 65536" | sudo tee -a /etc/security/limits.conf

# Increase process limits
echo "vexfs soft nproc 4096" | sudo tee -a /etc/security/limits.conf
echo "vexfs hard nproc 4096" | sudo tee -a /etc/security/limits.conf
```

#### Kernel Parameters

```bash
# Optimize network settings
echo "net.core.somaxconn = 65535" | sudo tee -a /etc/sysctl.conf
echo "net.ipv4.tcp_max_syn_backlog = 65535" | sudo tee -a /etc/sysctl.conf
sudo sysctl -p
```

## 📞 Support and Maintenance

### Regular Maintenance Tasks

1. **Daily**: Monitor service health and logs
2. **Weekly**: Review backup integrity and disk usage
3. **Monthly**: Update dependencies and security patches
4. **Quarterly**: Performance review and capacity planning

### Getting Help

- **Documentation**: https://github.com/vexfs/vexfs/docs
- **Issues**: https://github.com/vexfs/vexfs/issues
- **Discussions**: https://github.com/vexfs/vexfs/discussions
- **Security**: security@vexfs.org

### Version Upgrades

```bash
# Package-based upgrade
sudo apt update && sudo apt upgrade vexfs  # Debian/Ubuntu
sudo dnf update vexfs                      # Fedora/RHEL

# Manual upgrade
sudo systemctl stop vexfs
sudo cp new_vexfs_server /usr/local/bin/vexfs_server
sudo systemctl start vexfs

# Docker upgrade
docker-compose pull
docker-compose up -d

# Kubernetes upgrade
helm upgrade vexfs ./vexfs --namespace vexfs
```

---

This completes the VexFS v1.0 production deployment guide. The system is now ready for enterprise-grade deployment with comprehensive monitoring, security, and operational capabilities.