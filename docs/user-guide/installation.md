# VexFS v2.0 Installation Guide

This comprehensive guide covers all installation methods for VexFS v2.0, the world's first production-ready vector-extended filesystem.

## 📋 System Requirements

### Minimum Requirements

| Component | Requirement |
|-----------|-------------|
| **Operating System** | Linux (Ubuntu 20.04+, CentOS 8+, or equivalent) |
| **Kernel Version** | 5.4+ (for kernel module) |
| **Memory** | 8GB RAM minimum, 16GB+ recommended |
| **Storage** | SSD recommended for optimal performance |
| **CPU** | Multi-core processor (4+ cores recommended) |
| **Rust** | 1.70+ (for building from source) |
| **Docker** | 20.10+ (for containerized deployment) |

### Recommended Production Setup

| Component | Recommendation |
|-----------|----------------|
| **Memory** | 32GB+ RAM for large datasets |
| **Storage** | NVMe SSD with 2TB+ capacity |
| **CPU** | 16+ cores, 3.0GHz+ |
| **Network** | 10Gbps for distributed setups |

## 🚀 Installation Methods

### Method 1: Kernel Module Installation (Recommended for Production)

VexFS v2.0 provides a true kernel-level filesystem for maximum performance:

```bash
# Clone the repository
git clone https://github.com/lspecian/vexfs.git
cd vexfs

# Build the kernel module
cd kernel/vexfs_v2_build
make

# Load the kernel module
sudo insmod vexfs_v2.ko

# Verify module is loaded
lsmod | grep vexfs
cat /proc/filesystems | grep vexfs
```

### Method 2: FUSE Implementation (Development/Testing)

For development and cross-platform compatibility:

```bash
# Build FUSE implementation
cd rust
cargo build --release --bin vexfs_fuse

# Create mount point
mkdir /tmp/vexfs_mount

# Mount FUSE filesystem
./target/release/vexfs_fuse /tmp/vexfs_mount

# Verify mount
mount | grep vexfs
ls /tmp/vexfs_mount
```

### Method 3: Docker Deployment

Containerized deployment for easy setup:

```bash
# Start VexFS with Docker Compose
docker-compose -f deployment/docker/docker-compose.production.yml up -d

# Verify installation
curl http://localhost:8000/api/v2/version
```

## 🔧 Kernel Module Setup

### Prerequisites

```bash
# Ubuntu/Debian
sudo apt update
sudo apt install -y build-essential linux-headers-$(uname -r) pkg-config

# CentOS/RHEL
sudo yum groupinstall -y "Development Tools"
sudo yum install -y kernel-devel kernel-headers

# Install Rust (if building from source)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
```

### Build and Install

```bash
# Navigate to kernel module directory
cd kernel/vexfs_v2_build

# Build the kernel module
make clean
make

# Load the module
sudo insmod vexfs_v2.ko

# Verify module loaded successfully
dmesg | tail -20
lsmod | grep vexfs_v2
```

### Format and Mount a Partition

```bash
# Format a partition with VexFS v2.0
sudo mkfs.vexfs /dev/sdb1

# Create mount point
sudo mkdir -p /mnt/vexfs

# Mount the filesystem
sudo mount -t vexfs_v2 /dev/sdb1 /mnt/vexfs

# Verify mount
mount | grep vexfs
df -h /mnt/vexfs
```

## 🐍 Python SDK Installation

### From PyPI

```bash
pip install vexfs-v2
```

### From Source

```bash
cd bindings/python

# Install build dependencies
pip install maturin

# Build and install
maturin develop --release

# Verify installation
python -c "import vexfs; print(vexfs.__version__)"
```

### Virtual Environment Setup

```bash
# Create virtual environment
python -m venv vexfs-env
source vexfs-env/bin/activate

# Install VexFS v2.0
pip install vexfs-v2

# Install ML dependencies
pip install numpy pandas sentence-transformers scikit-learn
```

## 🔷 TypeScript SDK Installation

### From npm

```bash
npm install @vexfs/sdk-v2
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

## ⚙️ Configuration

### Kernel Module Configuration

Create `/etc/vexfs/vexfs.conf`:

```ini
# VexFS v2.0 Kernel Module Configuration

[filesystem]
# Default vector dimension
default_dimension = 384

# Cache settings
vector_cache_size = 2GB
metadata_cache_size = 512MB

# Performance tuning
max_concurrent_operations = 1000
batch_insert_size = 10000

[indexing]
# ANNS algorithm (hnsw, lsh)
default_algorithm = hnsw

# HNSW parameters
hnsw_m = 16
hnsw_ef_construction = 200
hnsw_ef_search = 100

# LSH parameters
lsh_num_tables = 10
lsh_num_functions = 20

[security]
# Access control
enable_acl = true
default_permissions = 644

# Encryption (if enabled)
enable_encryption = false
encryption_algorithm = aes256
```

### Environment Variables

```bash
# Kernel module settings
export VEXFS_DEFAULT_DIMENSION=384
export VEXFS_CACHE_SIZE=2GB
export VEXFS_MAX_OPERATIONS=1000

# Logging
export VEXFS_LOG_LEVEL=info
export VEXFS_LOG_FILE=/var/log/vexfs/vexfs.log

# Performance tuning
export VEXFS_BATCH_SIZE=10000
export VEXFS_WORKER_THREADS=8
```

## ✅ Verification

### Kernel Module Verification

```bash
# Check module status
lsmod | grep vexfs_v2

# Check filesystem registration
cat /proc/filesystems | grep vexfs

# Check kernel logs
dmesg | grep vexfs

# Test basic functionality
sudo mkfs.vexfs /dev/loop0  # Using loop device for testing
sudo mount -t vexfs_v2 /dev/loop0 /mnt/test
echo "Hello VexFS v2.0" | sudo tee /mnt/test/hello.txt
cat /mnt/test/hello.txt
```

### FUSE Verification

```bash
# Check FUSE mount
mount | grep vexfs_fuse

# Test basic operations
echo "Test data" > /tmp/vexfs_mount/test.txt
cat /tmp/vexfs_mount/test.txt

# Test vector operations (if supported)
python3 -c "
import vexfs
client = vexfs.Client('/tmp/vexfs_mount')
print('VexFS v2.0 FUSE working!')
"
```

### Performance Benchmark

```bash
# Run kernel module benchmarks
cd kernel/vexfs_v2_build
./test_hnsw_functionality

# Run comprehensive tests
./standalone_phase3_test

# Expected output:
# ✅ HNSW indexing: PASSED
# ✅ Vector search: PASSED
# ✅ Batch operations: PASSED
# ✅ Performance: >100k ops/sec
```

## 🔒 Security Setup

### Kernel Module Security

```bash
# Set up proper permissions
sudo chown root:root /lib/modules/$(uname -r)/extra/vexfs_v2.ko
sudo chmod 644 /lib/modules/$(uname -r)/extra/vexfs_v2.ko

# Configure module signing (if required)
sudo /usr/src/linux-headers-$(uname -r)/scripts/sign-file \
  sha256 /path/to/signing_key.priv /path/to/signing_key.x509 vexfs_v2.ko
```

### Access Control

```bash
# Create VexFS group
sudo groupadd vexfs

# Add users to VexFS group
sudo usermod -a -G vexfs $USER

# Set filesystem permissions
sudo chmod 755 /mnt/vexfs
sudo chgrp vexfs /mnt/vexfs
```

## 🐛 Troubleshooting

### Common Kernel Module Issues

**Module fails to load:**
```bash
# Check kernel compatibility
uname -r
modinfo vexfs_v2.ko

# Check for missing symbols
dmesg | grep vexfs | grep "Unknown symbol"

# Rebuild for current kernel
make clean && make
```

**Permission denied:**
```bash
# Check module permissions
ls -la vexfs_v2.ko

# Load with proper privileges
sudo insmod vexfs_v2.ko
```

**Mount fails:**
```bash
# Check filesystem type registration
cat /proc/filesystems | grep vexfs

# Check device permissions
ls -la /dev/sdb1
sudo chmod 666 /dev/sdb1  # Temporary for testing
```

### FUSE Issues

**Mount point busy:**
```bash
# Unmount existing mount
fusermount -u /tmp/vexfs_mount

# Kill any hanging processes
sudo pkill -f vexfs_fuse
```

**Permission errors:**
```bash
# Check FUSE permissions
ls -la /dev/fuse
sudo chmod 666 /dev/fuse

# Add user to fuse group
sudo usermod -a -G fuse $USER
```

## 🔄 Upgrading

### Kernel Module Upgrade

```bash
# Backup current configuration
sudo cp /etc/vexfs/vexfs.conf /etc/vexfs/vexfs.conf.backup

# Unmount filesystems
sudo umount /mnt/vexfs

# Remove old module
sudo rmmod vexfs_v2

# Install new module
cd kernel/vexfs_v2_build
git pull
make clean && make
sudo insmod vexfs_v2.ko

# Remount filesystems
sudo mount -t vexfs_v2 /dev/sdb1 /mnt/vexfs
```

### FUSE Upgrade

```bash
# Stop FUSE daemon
fusermount -u /tmp/vexfs_mount

# Update and rebuild
git pull
cd rust
cargo build --release --bin vexfs_fuse

# Restart FUSE
./target/release/vexfs_fuse /tmp/vexfs_mount
```

## 🎉 Next Steps

Now that VexFS v2.0 is installed:

1. **[Quick Start Guide](quick-start.md)** - Get familiar with basic operations
2. **[Basic Usage](usage.md)** - Learn core filesystem concepts
3. **[Vector Search Tutorial](../tutorials/vector-search.md)** - Explore vector capabilities
4. **[API Reference](../developer-guide/api-reference.md)** - Complete API documentation
5. **[Performance Tuning](../reference/performance.md)** - Optimize for your workload

**Need help?** Check our [troubleshooting guide](troubleshooting.md) or [open an issue](https://github.com/lspecian/vexfs/issues).

---

**VexFS v2.0** - The world's first production-ready vector-extended filesystem! 🚀