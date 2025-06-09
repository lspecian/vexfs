# VexFS Installation Guide

This guide covers installing VexFS in various environments and deployment scenarios.

## System Requirements

### Minimum Requirements
- **OS**: Linux 5.4+ (Ubuntu 20.04+, RHEL 8+, SUSE 15+)
- **Memory**: 4GB RAM minimum, 8GB recommended
- **Storage**: 10GB free space for installation
- **CPU**: x86_64 architecture with SSE4.2 support

### Recommended Requirements
- **Memory**: 16GB+ RAM for production workloads
- **Storage**: SSD storage for optimal performance
- **CPU**: AVX2 support for SIMD optimizations
- **Network**: Gigabit Ethernet for distributed deployments

### Development Requirements
- **Rust**: 1.70+ with cargo
- **Build Tools**: gcc, make, pkg-config
- **Kernel Headers**: linux-headers package for kernel module compilation

## Installation Methods

### 1. Binary Installation (Recommended)

#### Ubuntu/Debian
```bash
# Add VexFS repository
curl -fsSL https://packages.vexfs.io/gpg | sudo apt-key add -
echo "deb https://packages.vexfs.io/ubuntu $(lsb_release -cs) main" | sudo tee /etc/apt/sources.list.d/vexfs.list

# Install VexFS
sudo apt update
sudo apt install vexfs-fuse vexfs-tools vexfs-dev
```

#### RHEL/CentOS/Fedora
```bash
# Add VexFS repository
sudo rpm --import https://packages.vexfs.io/gpg
sudo tee /etc/yum.repos.d/vexfs.repo << EOF
[vexfs]
name=VexFS Repository
baseurl=https://packages.vexfs.io/rhel/\$releasever/\$basearch/
enabled=1
gpgcheck=1
gpgkey=https://packages.vexfs.io/gpg
EOF

# Install VexFS
sudo dnf install vexfs-fuse vexfs-tools vexfs-dev
```

#### SUSE/openSUSE
```bash
# Add VexFS repository
sudo zypper addrepo https://packages.vexfs.io/suse/vexfs.repo
sudo zypper refresh

# Install VexFS
sudo zypper install vexfs-fuse vexfs-tools vexfs-dev
```

### 2. Container Installation

#### Docker
```bash
# Pull VexFS container
docker pull vexfs/vexfs:latest

# Run VexFS container
docker run -d \
  --name vexfs \
  --privileged \
  --device /dev/fuse \
  -v /mnt/vexfs:/mnt/vexfs:shared \
  vexfs/vexfs:latest
```

#### Kubernetes
```yaml
# Apply VexFS DaemonSet
kubectl apply -f https://raw.githubusercontent.com/vexfs/vexfs/main/deployment/kubernetes/vexfs-daemonset.yaml
```

### 3. Source Installation

#### Prerequisites
```bash
# Ubuntu/Debian
sudo apt install build-essential pkg-config libfuse3-dev linux-headers-$(uname -r)

# RHEL/CentOS/Fedora
sudo dnf groupinstall "Development Tools"
sudo dnf install pkgconfig fuse3-devel kernel-devel

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
```

#### Build and Install
```bash
# Clone repository
git clone https://github.com/vexfs/vexfs.git
cd vexfs

# Build VexFS
cargo build --release

# Install binaries
sudo cp target/release/vexfs_fuse /usr/local/bin/
sudo cp target/release/vexctl /usr/local/bin/
sudo cp target/release/mkfs.vexfs /usr/local/sbin/

# Install FUSE helper
sudo cp scripts/mount.vexfs /sbin/
sudo chmod +x /sbin/mount.vexfs

# Create configuration directory
sudo mkdir -p /etc/vexfs
sudo cp config/vexfs.conf.example /etc/vexfs/vexfs.conf
```

#### Kernel Module Installation (Optional)
```bash
# Build kernel module
cd kernel
make

# Install kernel module
sudo make install
sudo depmod -a

# Load kernel module
sudo modprobe vexfs
```

## Post-Installation Setup

### 1. User and Group Setup
```bash
# Create vexfs user and group
sudo groupadd vexfs
sudo useradd -r -g vexfs -s /bin/false vexfs

# Add your user to vexfs group
sudo usermod -a -G vexfs $USER
```

### 2. Directory Structure
```bash
# Create standard directories
sudo mkdir -p /var/lib/vexfs
sudo mkdir -p /var/log/vexfs
sudo mkdir -p /etc/vexfs/conf.d

# Set permissions
sudo chown -R vexfs:vexfs /var/lib/vexfs
sudo chown -R vexfs:vexfs /var/log/vexfs
sudo chmod 755 /var/lib/vexfs
sudo chmod 755 /var/log/vexfs
```

### 3. Service Configuration
```bash
# Enable and start VexFS service
sudo systemctl enable vexfs
sudo systemctl start vexfs

# Check service status
sudo systemctl status vexfs
```

### 4. Firewall Configuration
```bash
# Open required ports (if using remote API)
sudo firewall-cmd --permanent --add-port=8080/tcp  # REST API
sudo firewall-cmd --permanent --add-port=8081/tcp  # WebSocket API
sudo firewall-cmd --reload
```

## Verification

### 1. Basic Functionality Test
```bash
# Check VexFS version
vexfs_fuse --version

# Test FUSE mount
mkdir /tmp/vexfs_test
vexfs_fuse /tmp/vexfs_test

# Test basic operations
echo "Hello VexFS" > /tmp/vexfs_test/test.txt
cat /tmp/vexfs_test/test.txt

# Unmount
fusermount3 -u /tmp/vexfs_test
```

### 2. Performance Test
```bash
# Run performance benchmark
cargo run --bin performance_benchmark

# Check results
cat performance_analysis_report.md
```

### 3. API Test
```bash
# Test REST API (if enabled)
curl -X GET http://localhost:8080/api/v1/status

# Test vector search
curl -X POST http://localhost:8080/api/v1/search \
  -H "Content-Type: application/json" \
  -d '{"vector": [0.1, 0.2, 0.3], "k": 10}'
```

## Configuration

### Basic Configuration
Edit `/etc/vexfs/vexfs.conf`:

```toml
[general]
log_level = "info"
data_dir = "/var/lib/vexfs"
cache_size = "1GB"

[fuse]
mount_options = "allow_other,default_permissions"
max_background = 12
congestion_threshold = 10

[performance]
memory_pool_size = "512MB"
enable_simd = true
enable_compression = true

[api]
enable_rest = true
rest_port = 8080
enable_websocket = true
websocket_port = 8081
```

### Advanced Configuration
See [Configuration Reference](config-reference.md) for complete options.

## Troubleshooting

### Common Issues

#### FUSE Mount Fails
```bash
# Check FUSE availability
ls -l /dev/fuse

# Check user permissions
groups $USER

# Check mount point permissions
ls -ld /mnt/vexfs
```

#### Performance Issues
```bash
# Check system resources
htop
iostat -x 1

# Check VexFS logs
sudo journalctl -u vexfs -f

# Run performance analysis
cargo run --bin performance_benchmark
```

#### API Connection Issues
```bash
# Check service status
sudo systemctl status vexfs

# Check port availability
sudo netstat -tlnp | grep :8080

# Check firewall rules
sudo firewall-cmd --list-all
```

### Getting Help

- **Documentation**: [docs.vexfs.io](https://docs.vexfs.io)
- **Community**: [community.vexfs.io](https://community.vexfs.io)
- **Issues**: [github.com/vexfs/vexfs/issues](https://github.com/vexfs/vexfs/issues)
- **Support**: [support@vexfs.io](mailto:support@vexfs.io)

## Next Steps

After successful installation:

1. [Basic Usage Guide](basic-usage.md) - Learn essential operations
2. [Configuration Guide](configuration.md) - Customize your setup
3. [Performance Tuning](performance-tuning.md) - Optimize for your workload
4. [API Integration](api-integration.md) - Integrate with applications