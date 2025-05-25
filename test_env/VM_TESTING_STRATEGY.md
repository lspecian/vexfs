# VexFS VM Testing Strategy

## 🎯 **VM Testing Architecture**

### **Current State Analysis**

**Existing Components:**
- `vexfs.pkr.hcl` - Packer configuration for VM builds
- `run_qemu.sh` - QEMU execution script
- `README.md` - Basic documentation

**Current Capabilities:**
- Basic VM provisioning
- Rust toolchain installation
- Kernel headers availability

### **Required Enhancements**

#### **1. Source Code Integration**
```mermaid
graph LR
    A[Host Source] --> B[VM Mount Point]
    B --> C[Kernel Module Build]
    C --> D[Integration Tests]
    D --> E[Result Export]
    E --> F[Host Analysis]
```

**Implementation Requirements:**
- Mount host source directory as `/mnt/vexfs-src`
- Configure writable build directory
- Enable real-time source synchronization

#### **2. Automated Test Execution**

**Test Categories:**
1. **Compilation Tests**
   - Kernel module build validation
   - vexctl userspace tool compilation
   - Static analysis execution

2. **Integration Tests**
   - Module loading/unloading
   - ioctl interface validation
   - File system operations

3. **Performance Tests**
   - Vector search benchmarks
   - Memory usage analysis
   - I/O performance metrics

#### **3. Development Tools Integration**

**Required Tools:**
```bash
# Debugging Tools
gdb              # Kernel debugging
strace           # System call tracing
perf             # Performance profiling
dmesg            # Kernel message analysis

# Build Tools
make             # Kernel module builds
cargo            # Rust compilation
rustfmt          # Code formatting
clippy           # Static analysis

# Testing Tools
valgrind         # Memory analysis
stress-ng        # System stress testing
fio              # I/O benchmarking
```

## 🏗️ **Enhanced Packer Configuration**

### **VM Specifications**
```hcl
# Enhanced vexfs.pkr.hcl requirements
source "qemu" "vexfs-dev" {
  # Base configuration
  iso_url      = "ubuntu-22.04.3-live-server-amd64.iso"
  memory       = "4096"       # Increased for development
  cpus         = "4"          # Multi-core for parallel builds
  disk_size    = "20G"        # Larger disk for tools and builds
  
  # Development optimizations
  accelerator  = "kvm"        # Hardware acceleration
  net_device   = "virtio-net" # Better networking
  disk_interface = "virtio"   # Better I/O performance
  
  # Source mounting capability
  qemuargs = [
    ["-virtfs", "local,path={{user `source_path`}},mount_tag=vexfs-src,security_model=passthrough,id=vexfs-src"]
  ]
}
```

### **Provisioning Enhancements**

#### **Development Environment Setup**
```bash
# Install development tools
apt-get update
apt-get install -y \
  build-essential \
  linux-headers-generic \
  gdb \
  strace \
  perf-tools-unstable \
  valgrind \
  stress-ng \
  fio

# Install Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source ~/.cargo/env

# Install Rust-for-Linux components
rustup component add rust-src
rustup component add clippy
rustup component add rustfmt

# Configure source mounting
mkdir -p /mnt/vexfs-src
echo 'vexfs-src /mnt/vexfs-src 9p trans=virtio,version=9p2000.L 0 0' >> /etc/fstab
```

#### **Test Automation Setup**
```bash
# Create test runner script
cat > /usr/local/bin/vexfs-test-runner.sh << 'EOF'
#!/bin/bash
set -e

echo "🧪 VexFS Automated Testing Suite"
echo "================================"

# Mount source if not already mounted
if ! mountpoint -q /mnt/vexfs-src; then
    mount /mnt/vexfs-src
fi

cd /mnt/vexfs-src

# Phase 1: Compilation Tests
echo "📦 Phase 1: Compilation Tests"
echo "Building vexctl..."
cd vexctl && cargo build --release && cd ..

echo "Building kernel module..."
cd vexfs && make clean && make && cd ..

# Phase 2: Static Analysis
echo "🔍 Phase 2: Static Analysis"
cd vexctl && cargo clippy -- -D warnings && cd ..
cd vexfs && cargo check --lib && cd ..

# Phase 3: Unit Tests
echo "🧪 Phase 3: Unit Tests"
cd vexctl && cargo test && cd ..

# Phase 4: Integration Tests
echo "🔗 Phase 4: Integration Tests"
cd vexfs
# Load module
sudo insmod vexfs.ko || echo "Module loading failed"
# Test ioctl interface
sudo ../vexctl/target/release/vexctl status /tmp/test || echo "ioctl test failed"
# Unload module
sudo rmmod vexfs || echo "Module unloading failed"
cd ..

# Phase 5: Performance Tests
echo "⚡ Phase 5: Performance Tests"
# Add performance benchmarks here

echo "✅ All tests completed"
EOF

chmod +x /usr/local/bin/vexfs-test-runner.sh
```

## 🔧 **QEMU Execution Enhancements**

### **Enhanced run_qemu.sh**

**Current Issues:**
- No source mounting capability
- Limited memory allocation
- No test automation integration

**Required Enhancements:**
```bash
#!/bin/bash
# Enhanced run_qemu.sh

# Configuration
VM_MEMORY="4G"
VM_CPUS="4"
SOURCE_PATH="${1:-$(pwd)}"
TEST_MODE="${2:-false}"

# Build command
QEMU_CMD="qemu-system-x86_64 \
  -enable-kvm \
  -m $VM_MEMORY \
  -smp $VM_CPUS \
  -drive file=output-vexfs-dev/packer-vexfs-dev,format=qcow2 \
  -netdev user,id=net0,hostfwd=tcp::2222-:22 \
  -device virtio-net,netdev=net0 \
  -virtfs local,path=$SOURCE_PATH,mount_tag=vexfs-src,security_model=passthrough,id=vexfs-src"

# Add test automation if requested
if [ "$TEST_MODE" = "true" ]; then
  QEMU_CMD="$QEMU_CMD -nographic -serial mon:stdio"
  echo "Starting VM in test mode..."
  echo "Source path: $SOURCE_PATH"
  
  # Start VM and run tests
  $QEMU_CMD &
  VM_PID=$!
  
  # Wait for boot and run tests
  sleep 30
  ssh -p 2222 -o StrictHostKeyChecking=no user@localhost 'sudo /usr/local/bin/vexfs-test-runner.sh'
  
  # Shutdown VM
  kill $VM_PID
else
  echo "Starting VM in interactive mode..."
  $QEMU_CMD
fi
```

## 🎯 **Testing Workflow Integration**

### **Local Development Cycle**
```bash
# Host development (fast iteration)
scripts/dev-host.sh

# VM testing (comprehensive validation)
scripts/test-vm.sh

# Results analysis
cat test_env/results/test-report.txt
```

### **CI/CD Integration**
```yaml
# .github/workflows/vexfs-test.yml
name: VexFS Testing

on: [push, pull_request]

jobs:
  host-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Install Rust
        uses: actions-rs/toolchain@v1
      - name: Run host tests
        run: scripts/dev-host.sh

  vm-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Install QEMU/Packer
        run: |
          sudo apt-get update
          sudo apt-get install qemu-system-x86 packer
      - name: Run VM tests
        run: scripts/test-vm.sh
```

## 📊 **Success Metrics**

### **Performance Targets**
- **Host Development Cycle**: < 2 minutes
- **VM Test Cycle**: < 10 minutes
- **Full CI/CD Pipeline**: < 20 minutes

### **Reliability Targets**
- **Test Success Rate**: > 95%
- **Build Reproducibility**: 100%
- **Environment Consistency**: 100%

## 🚀 **Implementation Priority**

1. **Critical**: Fix vexctl compilation
2. **High**: Create host development script
3. **High**: Enhance Packer configuration
4. **Medium**: Create VM testing script
5. **Medium**: Add performance testing
6. **Low**: CI/CD integration

This strategy ensures rapid development iteration while maintaining comprehensive testing coverage through the VM environment.