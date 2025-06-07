# VexFS Ubuntu Package Distribution

This directory contains the complete packaging infrastructure for distributing VexFS as Ubuntu .deb packages.

## Package Overview

VexFS is distributed as three complementary packages:

### 1. **vexfs-dkms** (Architecture: all)
- **Purpose**: Kernel module via DKMS (Dynamic Kernel Module Support)
- **Size**: ~500KB
- **Contents**: 
  - VexFS kernel module source code
  - DKMS configuration for automatic rebuilding
  - Module loading/unloading scripts
- **Dependencies**: `dkms`, `linux-headers-generic`, `build-essential`
- **Auto-rebuilds**: Automatically recompiles for new kernel versions

### 2. **vexfs-utils** (Architecture: amd64/arm64)
- **Purpose**: Filesystem utilities and tools
- **Size**: ~30MB (includes Rust binary)
- **Contents**:
  - `mkfs.vexfs`: Format devices with VexFS filesystem
  - `fsck.vexfs`: Check and repair VexFS filesystems (future)
  - `vexfs-info`: Display filesystem information (future)
  - Documentation and man pages
- **Dependencies**: Standard C library
- **Key Features**: Vector-optimized formatting with 768+ dimensions

### 3. **vexfs-dev** (Architecture: amd64/arm64)
- **Purpose**: Development files and testing tools
- **Size**: ~50MB
- **Contents**:
  - Header files for VexFS kernel APIs
  - Performance benchmarking tools
  - Test utilities for vector operations
  - Example code and documentation
- **Dependencies**: `vexfs-dkms`, `linux-headers-generic`
- **Target Users**: Developers building VexFS applications

## Quick Installation

### For End Users (Recommended)
```bash
# Install DKMS module and utilities
sudo apt install ./vexfs-dkms_*.deb ./vexfs-utils_*.deb

# Format a device with VexFS
sudo mkfs.vexfs -V -D 768 -L "VectorDB" /dev/sdX1

# Mount the filesystem
sudo mount -t vexfs_v2_b62 /dev/sdX1 /mnt/vexfs
```

### For Developers
```bash
# Install all packages including development tools
sudo apt install ./vexfs-*.deb

# Run performance benchmarks
sudo vexfs_v2_performance_benchmark
sudo test_vector_search
```

## Building Packages

### Prerequisites

Install build dependencies:
```bash
sudo apt update
sudo apt install \
  debhelper \
  dkms \
  build-essential \
  linux-headers-generic \
  rustc \
  cargo \
  pkg-config \
  libc6-dev \
  devscripts \
  lintian \
  fakeroot
```

### Build Process

#### Option 1: Automated Build Script
```bash
# Run the complete build pipeline
./packaging/build-deb.sh
```

#### Option 2: Manual Build
```bash
# Prepare build environment
mkdir -p build/vexfs-2.0.0
cp -r kernel rust docs packaging build/vexfs-2.0.0/
cp -r packaging/debian build/vexfs-2.0.0/

# Create source tarball
cd build
tar -czf vexfs_2.0.0.orig.tar.gz vexfs-2.0.0

# Build packages
cd vexfs-2.0.0
dpkg-buildpackage -b -us -uc
```

### Build Output

Successful builds generate:
```
dist/
├── vexfs-dkms_2.0.0-1_all.deb          # Kernel module (DKMS)
├── vexfs-utils_2.0.0-1_amd64.deb       # Utilities (mkfs.vexfs)
├── vexfs-dev_2.0.0-1_amd64.deb         # Development files
├── vexfs_2.0.0-1.dsc                   # Source package description
├── vexfs_2.0.0.orig.tar.gz             # Original source tarball
└── SHA256SUMS                           # Package checksums
```

## Package Details

### DKMS Integration

The `vexfs-dkms` package uses DKMS to automatically:
- Compile the kernel module for the current kernel
- Rebuild the module when new kernels are installed
- Handle module loading/unloading
- Manage module dependencies

**DKMS Configuration** (`/usr/src/vexfs-2.0.0/dkms.conf`):
```ini
PACKAGE_NAME="vexfs"
PACKAGE_VERSION="2.0.0"
BUILT_MODULE_NAME[0]="vexfs_v2_phase3"
DEST_MODULE_LOCATION[0]="/kernel/fs/vexfs/"
AUTOINSTALL="yes"
```

### Filesystem Utilities

**mkfs.vexfs** - Primary formatting utility:
```bash
# Basic usage
mkfs.vexfs /dev/sdX1

# Vector database optimized
mkfs.vexfs -V -D 768 -L "VectorDB" /dev/sdX1

# High-performance configuration
mkfs.vexfs -b 8192 -i 8192 -V -D 1024 /dev/sdX1
```

**Options**:
- `-V, --enable-vectors`: Enable vector storage support
- `-D, --vector-dimensions N`: Vector dimensions (implies -V)
- `-L, --label LABEL`: Volume label
- `-b, --block-size SIZE`: Block size in bytes (default: 4096)
- `-i, --inode-ratio RATIO`: Bytes per inode (default: 16384)
- `-f, --force`: Force creation (skip safety checks)

### Development Tools

**Performance Benchmarks**:
- `vexfs_v2_performance_benchmark`: Comprehensive performance testing
- `vexfs_v2_working_benchmark`: Production workload simulation
- `test_vector_search`: Vector similarity search testing

**API Headers** (`/usr/include/vexfs/`):
- `vexfs_v2_uapi.h`: User-space API definitions
- `vexfs_v2_phase3.h`: Phase 3 feature headers
- `vexfs_ffi.h`: FFI interface definitions

## Continuous Integration

### GitHub Actions Workflow

The repository includes automated CI/CD via `.github/workflows/build-deb-packages.yml`:

**Triggers**:
- Push to `main` or `develop` branches
- Pull requests to `main`
- Git tags starting with `v*`
- Manual workflow dispatch

**Build Matrix**:
- Ubuntu 20.04, 22.04, 24.04
- Architecture: amd64 (arm64 planned)

**Automated Steps**:
1. **Environment Setup**: Install build dependencies
2. **Rust Compilation**: Build utilities with cargo
3. **Package Building**: Create .deb packages
4. **Quality Assurance**: Lint packages with lintian
5. **Installation Testing**: Verify packages install correctly
6. **Release Creation**: Automatic GitHub releases for tags

### Release Process

**For maintainers**:
```bash
# Create and push a release tag
git tag -a v2.0.0 -m "VexFS 2.0.0 Release"
git push origin v2.0.0

# GitHub Actions automatically:
# 1. Builds packages for all supported Ubuntu versions
# 2. Runs installation tests
# 3. Creates GitHub release with .deb files
# 4. Generates checksums and release notes
```

## Installation Verification

### Post-Installation Checks

```bash
# Verify packages are installed
dpkg -l | grep vexfs

# Check DKMS module status
sudo dkms status vexfs

# Verify kernel module can load
sudo modprobe vexfs_v2_phase3
lsmod | grep vexfs

# Test mkfs.vexfs utility
mkfs.vexfs --help

# Check filesystem registration
cat /proc/filesystems | grep vexfs
```

### Performance Verification

```bash
# Check SIMD capabilities
sudo dmesg | grep -i simd

# Run basic performance test
sudo vexfs_v2_performance_benchmark

# Monitor vector operations
cat /proc/vexfs/stats  # (when mounted)
```

## Troubleshooting

### Common Issues

**1. DKMS Build Failures**
```bash
# Check DKMS build logs
sudo dkms status vexfs
sudo dkms build vexfs/2.0.0

# Common fixes
sudo apt install linux-headers-$(uname -r)
sudo dkms remove vexfs/2.0.0 --all
sudo dkms install vexfs/2.0.0
```

**2. Module Loading Issues**
```bash
# Check kernel compatibility
uname -r
modinfo vexfs_v2_phase3

# Verify dependencies
lsmod | grep vexfs
dmesg | grep vexfs
```

**3. Filesystem Mount Failures**
```bash
# Verify filesystem was created
sudo file -s /dev/sdX1
sudo blkid /dev/sdX1

# Check mount options
sudo mount -t vexfs_v2_b62 /dev/sdX1 /mnt/vexfs -v
```

### Debug Information

**Collect debug info**:
```bash
# System information
uname -a
lsb_release -a
dpkg -l | grep vexfs

# Kernel module status
lsmod | grep vexfs
dkms status vexfs
dmesg | grep -i vexfs | tail -20

# Filesystem status
mount | grep vexfs
cat /proc/filesystems | grep vexfs
```

## Contributing

### Package Maintenance

**Adding new features**:
1. Update `packaging/debian/changelog`
2. Modify `packaging/debian/control` if dependencies change
3. Update build scripts in `packaging/debian/rules`
4. Test with `./packaging/build-deb.sh`

**Version Updates**:
1. Update version in `packaging/debian/changelog`
2. Update `packaging/dkms.conf` version
3. Update `packaging/build-deb.sh` version variables
4. Test build and installation

### Quality Assurance

**Before submitting**:
```bash
# Lint packages
lintian dist/*.deb

# Test installation
sudo dpkg -i dist/*.deb
sudo apt install -f

# Verify functionality
mkfs.vexfs --help
sudo modprobe vexfs_v2_phase3
```

## Support

- **Documentation**: See `docs/VEXFS_KERNEL_MODULE_GUIDE.md`
- **Issues**: GitHub Issues for bug reports
- **Performance**: Use included benchmarking tools
- **Development**: Install `vexfs-dev` package for APIs

---

**VexFS**: High-performance, kernel-native vector database filesystem  
**Target Performance**: 100,000+ vector operations per second  
**Supported Platforms**: Ubuntu 20.04+, Linux Kernel 5.4+