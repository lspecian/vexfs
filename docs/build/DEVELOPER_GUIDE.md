# VexFS Build System Developer Guide

## Quick Start

### Basic Build Commands

```bash
# Build everything
make all

# Build just the kernel module
make kernel

# Build and run tests
make tests

# Clean all build artifacts
make clean

# Get help on available targets
make help
```

### First Time Setup

1. **Install Dependencies**
   ```bash
   # Ubuntu/Debian
   sudo apt-get install build-essential linux-headers-$(uname -r)
   
   # Install Rust for userspace components
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. **Verify Environment**
   ```bash
   # Check kernel headers
   ls /lib/modules/$(uname -r)/build
   
   # Check compiler
   gcc --version
   make --version
   ```

3. **Build VexFS**
   ```bash
   make all
   ```

## Development Workflow

### Standard Development Cycle

1. **Make Changes** - Edit source files
2. **Quick Syntax Check** - `make syntax-check`
3. **Build** - `make kernel` (for kernel changes) or `make userspace`
4. **Test** - `make test-unit` or specific test targets
5. **Verify** - `./scripts/build/verify_module.sh`

### Working with Kernel Module

#### Building the Kernel Module

```bash
# Primary VexFS v2.0 implementation
make kernel

# Alternative: Build directly in kernel directory
cd kernel/vexfs_v2_build
make
```

#### Testing Kernel Module

```bash
# Build and load module for testing
make kernel
sudo insmod kernel/vexfs_v2_build/vexfs.ko

# Verify module loaded
lsmod | grep vexfs

# Remove module
sudo rmmod vexfs
```

#### Kernel Module Variants

```bash
# Build specific variants
make kernel-minimal    # Minimal feature set
make kernel-fixed      # Stable implementation
make kernel-complex    # Advanced features

# Or use specific Makefiles
make -f scripts/build/Makefile.minimal
make -f scripts/build/Makefile.vexfs_fixed
make -f scripts/build/Makefile.complex
```

### Working with Userspace Components

#### FUSE Implementation

```bash
# Build FUSE filesystem
make userspace

# Run FUSE filesystem
mkdir /tmp/vexfs_mount
./target/release/vexfs_fuse /tmp/vexfs_mount

# Test FUSE filesystem
ls /tmp/vexfs_mount
echo "test" > /tmp/vexfs_mount/test.txt

# Unmount
fusermount -u /tmp/vexfs_mount
```

#### Utilities and Tools

```bash
# Build all userspace tools
cargo build --release

# Build specific tools
cargo build --bin vexfs_fuse
cargo build --bin vexctl
```

## Testing Guide

### Test Categories

#### Unit Tests
```bash
# Run all unit tests
make test-unit

# Run specific unit test suites
cd kernel/vexfs_v2_build
make test-phase2
make test-phase3
```

#### Integration Tests
```bash
# Full integration test suite
make test-integration

# Specific integration tests
./scripts/build/run_breakthrough_demonstration.sh
```

#### Performance Tests
```bash
# Performance benchmarks
make test-performance

# Custom performance tests
make -f scripts/build/Makefile.performance benchmark
```

#### Search Functionality Tests
```bash
# Search algorithm tests
make test-search

# Advanced search tests
make -f scripts/build/Makefile.search test-hnsw
make -f scripts/build/Makefile.search test-lsh
```

#### Comparison Tests
```bash
# Compare implementations
make test-comparison

# Detailed comparison analysis
make -f scripts/build/Makefile.comparison_tests compare-all
```

### Test Infrastructure

#### Test Environment Setup

```bash
# Convert test infrastructure (if needed)
./scripts/build/convert_test_infrastructure.sh

# Verify test environment
./scripts/build/verify_module.sh
```

#### Custom Test Execution

```bash
# Run specific test programs
cd kernel/vexfs_v2_build
./test_phase2_search_clean
./simple_phase2_test
./standalone_phase3_test
./test_hnsw_functionality
```

## Debugging and Troubleshooting

### Build Issues

#### Common Problems

1. **Missing Kernel Headers**
   ```bash
   # Error: No rule to make target '/lib/modules/.../build'
   sudo apt-get install linux-headers-$(uname -r)
   ```

2. **Compiler Errors**
   ```bash
   # Clean and rebuild
   make clean
   make kernel
   ```

3. **Module Loading Issues**
   ```bash
   # Check kernel log
   dmesg | tail -20
   
   # Verify module symbols
   nm kernel/vexfs_v2_build/vexfs.ko | grep vexfs
   ```

#### Debug Build

```bash
# Enable debug symbols
export EXTRA_CFLAGS="-g -DDEBUG"
make kernel

# Use debug-enabled variants
make -f scripts/build/Makefile.complex DEBUG=1
```

### Runtime Debugging

#### Kernel Module Debugging

```bash
# Enable kernel debugging
echo 8 > /proc/sys/kernel/printk

# Monitor kernel messages
dmesg -w

# Use ftrace for detailed tracing
echo function > /sys/kernel/debug/tracing/current_tracer
echo vexfs_* > /sys/kernel/debug/tracing/set_ftrace_filter
```

#### FUSE Debugging

```bash
# Run FUSE with debug output
./target/release/vexfs_fuse -d /tmp/vexfs_mount

# Enable FUSE debug logging
export RUST_LOG=debug
./target/release/vexfs_fuse /tmp/vexfs_mount
```

## Advanced Build Configuration

### Environment Variables

```bash
# Kernel build customization
export EXTRA_CFLAGS="-O2 -g"
export KDIR="/path/to/kernel/source"

# Rust build customization
export CARGO_TARGET_DIR="./target"
export RUSTFLAGS="-C target-cpu=native"
```

### Cross-Compilation

#### ARM64 Cross-Compilation

```bash
# Install cross-compilation tools
sudo apt-get install gcc-aarch64-linux-gnu

# Set cross-compilation environment
export ARCH=arm64
export CROSS_COMPILE=aarch64-linux-gnu-
export KDIR="/path/to/arm64/kernel/headers"

# Build for ARM64
make kernel
```

#### Rust Cross-Compilation

```bash
# Add ARM64 target
rustup target add aarch64-unknown-linux-gnu

# Cross-compile userspace components
cargo build --target aarch64-unknown-linux-gnu --release
```

### Custom Build Variants

#### Creating Custom Variants

1. **Copy Base Makefile**
   ```bash
   cp scripts/build/Makefile.minimal scripts/build/Makefile.custom
   ```

2. **Modify Configuration**
   ```makefile
   # Edit scripts/build/Makefile.custom
   EXTRA_CFLAGS += -DCUSTOM_FEATURE
   obj-m += custom_module.o
   ```

3. **Build Custom Variant**
   ```bash
   make -f scripts/build/Makefile.custom
   ```

## Performance Optimization

### Build Performance

#### Parallel Builds

```bash
# Use multiple CPU cores
make -j$(nproc) kernel

# Parallel Rust builds
cargo build --release --jobs $(nproc)
```

#### Build Caching

```bash
# Use ccache for C compilation
export CC="ccache gcc"
make kernel

# Use sccache for Rust compilation
export RUSTC_WRAPPER=sccache
cargo build --release
```

### Runtime Performance

#### Kernel Module Optimization

```bash
# Build with optimizations
export EXTRA_CFLAGS="-O3 -march=native"
make kernel

# Profile-guided optimization
make -f scripts/build/Makefile.performance profile-build
```

#### FUSE Performance

```bash
# Build optimized FUSE implementation
cargo build --release --features "performance"

# Use performance-tuned configuration
export VEXFS_CACHE_SIZE=1048576
./target/release/vexfs_fuse /tmp/vexfs_mount
```

## Continuous Integration

### Automated Testing

```bash
# CI test script
#!/bin/bash
make clean
make all
make tests
./scripts/build/verify_module.sh
```

### Build Verification

```bash
# Comprehensive build verification
make clean
make syntax-check
make all
make test-unit
make test-integration
```

## Contributing Guidelines

### Code Style

```bash
# Format code before committing
make format

# Check code style
make lint

# Verify syntax
make syntax-check
```

### Testing Requirements

- All new features must include unit tests
- Integration tests for kernel module changes
- Performance tests for optimization changes
- Documentation updates for API changes

### Build System Changes

When modifying the build system:

1. **Test all variants** - Ensure all Makefile variants still work
2. **Update documentation** - Keep this guide current
3. **Verify CI compatibility** - Test automated build processes
4. **Maintain backward compatibility** - Don't break existing workflows

## Troubleshooting Reference

### Error Messages

| Error | Cause | Solution |
|-------|-------|----------|
| `No rule to make target` | Missing dependencies | Install kernel headers |
| `Module not found` | Build failed | Check compilation errors |
| `Permission denied` | Insufficient privileges | Use sudo for module operations |
| `Symbol not found` | Missing exports | Check kernel configuration |
| `FUSE mount failed` | FUSE not available | Install fuse package |

### Log Locations

- **Kernel messages**: `/var/log/kern.log`, `dmesg`
- **Build logs**: Terminal output, redirect with `make 2>&1 | tee build.log`
- **FUSE logs**: stderr output, use `-d` flag for debug
- **Test logs**: `kernel/vexfs_v2_build/test_*.log`

### Support Resources

- **Build system documentation**: `docs/build/BUILD_SYSTEM.md`
- **Architecture documentation**: `docs/architecture/`
- **Implementation notes**: `docs/implementation/`
- **Test documentation**: `docs/testing/`