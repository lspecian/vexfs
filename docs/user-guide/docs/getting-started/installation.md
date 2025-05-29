# Installation Guide

This comprehensive guide covers all installation methods for VexFS v1.0 across different platforms and use cases.

## 📋 System Requirements

### Minimum Requirements

| Component | Requirement |
|-----------|-------------|
| **Operating System** | Linux (Ubuntu 20.04+, CentOS 8+, or equivalent) |
| **Kernel Version** | 5.4+ (for kernel module) or FUSE support |
| **Memory** | 4GB RAM minimum, 8GB+ recommended |
| **Storage** | SSD recommended for optimal performance |
| **CPU** | Multi-core processor (4+ cores recommended) |
| **Rust** | 1.70+ (for building from source) |
| **Docker** | 20.10+ (for containerized deployment) |

### Recommended Production Setup

| Component | Recommendation |
|-----------|----------------|
| **Memory** | 16GB+ RAM for large datasets |
| **Storage** | NVMe SSD with 1TB+ capacity |
| **CPU** | 8+ cores, 3.0GHz+ |
| **Network** | 10Gbps for distributed setups |

## 🚀 Installation Methods

### Method 1: Docker Deployment (Recommended)

The fastest and most reliable way to get VexFS running:

```bash
# Clone the repository
git clone https://github.com/lspecian/vexfs.git
cd vexfs

# Start VexFS with Docker Compose
docker-compose up -d

# Verify installation
curl http://localhost:8000/api/v1/version
```

**Docker Compose Configuration:**

```yaml
# docker-compose.yml
version: '3.8'
services:
  vexfs-server:
    build: .
    ports:
      - "8000:8000"
    volumes:
      - vexfs_data:/data
    environment:
      - RUST_LOG=info
      - VEXFS_DATA_DIR=/data
    restart: unless-stopped

volumes:
  vexfs_data:
```

### Method 2: Pre-built Binaries

Download pre-built binaries for your platform:

```bash
# Download latest release
wget https://github.com/lspecian/vexfs/releases/latest/download/vexfs-linux-x86_64.tar.gz

# Extract
tar -xzf vexfs-linux-x86_64.tar.gz

# Install
sudo cp vexfs-linux-x86_64/bin/* /usr/local/bin/
sudo cp vexfs-linux-x86_64/lib/* /usr/local/lib/

# Verify installation
vexfs --version
```

### Method 3: Build from Source

For the latest features and custom configurations:

#### Prerequisites

```bash
# Ubuntu/Debian
sudo apt update
sudo apt install -y build-essential pkg-config libssl-dev libfuse-dev

# CentOS/RHEL
sudo yum groupinstall -y "Development Tools"
sudo yum install -y openssl-devel fuse-devel

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
```

#### Build Process

```bash
# Clone repository
git clone https://github.com/lspecian/vexfs.git
cd vexfs

# Build release version
cargo build --release

# Run tests to verify build
cargo test

# Install binaries
sudo cp target/release/vexfs_server /usr/local/bin/
sudo cp target/release/vexfs_fuse /usr/local/bin/
sudo cp vexctl/target/release/vexctl /usr/local/bin/
```

## 🐍 Python SDK Installation

### From PyPI (Recommended)

```bash
pip install vexfs
```

### From Source

```bash
cd bindings/python

# Install build dependencies
pip install maturin

# Build and install
maturin develop --release

# Or build wheel for distribution
maturin build --release
pip install target/wheels/vexfs-*.whl
```

### Virtual Environment Setup

```bash
# Create virtual environment
python -m venv vexfs-env
source vexfs-env/bin/activate  # On Windows: vexfs-env\Scripts\activate

# Install VexFS
pip install vexfs

# Install optional dependencies for ML workflows
pip install numpy pandas sentence-transformers scikit-learn
```

## 🔷 TypeScript SDK Installation

### From npm

```bash
npm install vexfs-sdk
```

### From Source

```bash
cd bindings/typescript

# Install dependencies
npm install

# Build TypeScript
npm run build

# Link for local development
npm link
```

### Project Setup

```bash
# Initialize new project
mkdir my-vexfs-app
cd my-vexfs-app
npm init -y

# Install VexFS SDK
npm install vexfs-sdk

# Install TypeScript (if not already installed)
npm install -D typescript @types/node ts-node

# Create tsconfig.json
npx tsc --init
```

## 🖥️ Platform-Specific Installation

### Ubuntu 20.04/22.04

```bash
# Update system
sudo apt update && sudo apt upgrade -y

# Install dependencies
sudo apt install -y curl build-essential pkg-config libssl-dev libfuse-dev

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Clone and build VexFS
git clone https://github.com/lspecian/vexfs.git
cd vexfs
cargo build --release

# Install system service (optional)
sudo cp scripts/vexfs.service /etc/systemd/system/
sudo systemctl enable vexfs
sudo systemctl start vexfs
```

### CentOS 8/RHEL 8

```bash
# Enable PowerTools repository
sudo dnf config-manager --set-enabled powertools

# Install dependencies
sudo dnf groupinstall -y "Development Tools"
sudo dnf install -y openssl-devel fuse-devel

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Build VexFS
git clone https://github.com/lspecian/vexfs.git
cd vexfs
cargo build --release
```

### Arch Linux

```bash
# Install dependencies
sudo pacman -S base-devel openssl fuse3

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Build VexFS
git clone https://github.com/lspecian/vexfs.git
cd vexfs
cargo build --release
```

## 🔧 Configuration

### Basic Configuration

Create `/etc/vexfs/config.toml`:

```toml
[server]
bind_address = "0.0.0.0"
port = 8000
data_directory = "/var/lib/vexfs"

[vector]
default_dimension = 384
cache_size = "1GB"
index_type = "hnsw"

[performance]
max_connections = 1000
worker_threads = 8
batch_size = 1000

[security]
enable_tls = false
cert_file = "/etc/vexfs/cert.pem"
key_file = "/etc/vexfs/key.pem"

[logging]
level = "info"
file = "/var/log/vexfs/vexfs.log"
```

### Environment Variables

```bash
# Server configuration
export VEXFS_BIND_ADDRESS="0.0.0.0"
export VEXFS_PORT="8000"
export VEXFS_DATA_DIR="/var/lib/vexfs"

# Performance tuning
export VEXFS_CACHE_SIZE="2GB"
export VEXFS_WORKER_THREADS="8"
export VEXFS_MAX_CONNECTIONS="1000"

# Logging
export RUST_LOG="vexfs=info"
export VEXFS_LOG_FILE="/var/log/vexfs/vexfs.log"
```

## 🚀 Starting VexFS

### Docker Method

```bash
# Start with Docker Compose
docker-compose up -d

# Check status
docker-compose ps

# View logs
docker-compose logs -f vexfs-server
```

### Binary Method

```bash
# Start VexFS server
vexfs_server --config /etc/vexfs/config.toml

# Or with environment variables
VEXFS_PORT=8000 vexfs_server

# Start as daemon
nohup vexfs_server > /var/log/vexfs/server.log 2>&1 &
```

### Systemd Service

```bash
# Enable and start service
sudo systemctl enable vexfs
sudo systemctl start vexfs

# Check status
sudo systemctl status vexfs

# View logs
sudo journalctl -u vexfs -f
```

## ✅ Verification

### Health Check

```bash
# Check server health
curl http://localhost:8000/api/v1/version

# Expected response:
# {"version": "VexFS 1.0.0"}
```

### Basic Functionality Test

```bash
# Run compatibility test
python3 test_chromadb_compatibility.py

# Expected output:
# ✅ All tests passed (7/7)
```

### Performance Benchmark

```bash
# Run performance tests
cargo run --bin vector_benchmark --release

# Expected metrics:
# Vector insertion: ~263,852 vectors/second
# Search latency: 21.98-52.34µs
# Memory efficiency: 94.2%
```

## 🔒 Security Setup

### TLS Configuration

```bash
# Generate self-signed certificate (for testing)
openssl req -x509 -newkey rsa:4096 -keyout /etc/vexfs/key.pem \
  -out /etc/vexfs/cert.pem -days 365 -nodes

# Update config.toml
[security]
enable_tls = true
cert_file = "/etc/vexfs/cert.pem"
key_file = "/etc/vexfs/key.pem"
```

### Firewall Configuration

```bash
# Ubuntu/Debian (ufw)
sudo ufw allow 8000/tcp

# CentOS/RHEL (firewalld)
sudo firewall-cmd --permanent --add-port=8000/tcp
sudo firewall-cmd --reload
```

## 📊 Monitoring Setup

### Log Configuration

```bash
# Create log directory
sudo mkdir -p /var/log/vexfs
sudo chown vexfs:vexfs /var/log/vexfs

# Configure log rotation
sudo tee /etc/logrotate.d/vexfs << EOF
/var/log/vexfs/*.log {
    daily
    rotate 30
    compress
    delaycompress
    missingok
    notifempty
    create 644 vexfs vexfs
}
EOF
```

### Metrics Collection

```bash
# Install Prometheus exporter (optional)
cargo install vexfs-prometheus-exporter

# Start metrics endpoint
vexfs-prometheus-exporter --port 9090
```

## 🐛 Troubleshooting

### Common Issues

**Port already in use:**
```bash
# Find process using port 8000
sudo netstat -tlnp | grep :8000
sudo kill -9 <PID>
```

**Permission denied:**
```bash
# Fix permissions
sudo chown -R vexfs:vexfs /var/lib/vexfs
sudo chmod -R 755 /var/lib/vexfs
```

**Memory issues:**
```bash
# Check available memory
free -h

# Adjust cache size in config
[vector]
cache_size = "512MB"  # Reduce if needed
```

### Log Analysis

```bash
# Check server logs
tail -f /var/log/vexfs/vexfs.log

# Check system logs
sudo journalctl -u vexfs -f

# Enable debug logging
export RUST_LOG="vexfs=debug"
```

## 🔄 Upgrading

### Docker Upgrade

```bash
# Pull latest image
docker-compose pull

# Restart with new image
docker-compose up -d
```

### Binary Upgrade

```bash
# Backup data
sudo cp -r /var/lib/vexfs /var/lib/vexfs.backup

# Stop service
sudo systemctl stop vexfs

# Install new version
wget https://github.com/lspecian/vexfs/releases/latest/download/vexfs-linux-x86_64.tar.gz
tar -xzf vexfs-linux-x86_64.tar.gz
sudo cp vexfs-linux-x86_64/bin/* /usr/local/bin/

# Start service
sudo systemctl start vexfs
```

## 🎉 Next Steps

Now that VexFS is installed:

1. **[Quick Start Guide](quick-start.md)** - Get familiar with basic operations
2. **[Basic Operations](../user-guide/basic-operations.md)** - Learn core concepts
3. **[Python Examples](../examples/python.md)** - Start building with Python
4. **[TypeScript Examples](../examples/typescript.md)** - Start building with TypeScript
5. **[Production Deployment](../deployment/production.md)** - Deploy to production

**Need help?** Check our [troubleshooting guide](../troubleshooting/common-issues.md) or [open an issue](https://github.com/lspecian/vexfs/issues).