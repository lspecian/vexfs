# VexFS Simplified VM Testing Environment

## 🎯 Overview

This document describes the new simplified QEMU testing environment for VexFS that replaces the complex Packer-based build system with a fast, iteration-friendly approach.

## ⚡ Key Improvements

### Before (Complex Packer Approach)
- **Build Time**: 10-20 minutes per iteration
- **Source Integration**: Code baked into VM during build
- **Iteration Speed**: Full rebuild required for any change
- **Dependencies**: Complex Packer + Debian installation
- **Debugging**: Difficult to access live development

### After (Simplified Cloud Image Approach)
- **Boot Time**: 1-2 minutes
- **Source Integration**: Live mounting via virtfs
- **Iteration Speed**: Instant - edit and test without rebuild
- **Dependencies**: Just QEMU + cloud image download
- **Debugging**: Live development with real-time feedback

## 🚀 Quick Start

### 1. Initial Setup (One Time)
```bash
# Setup VM environment
./test_env/setup_vm.sh

# This will:
# - Download Ubuntu 22.04 cloud image (~500MB)
# - Create development VM with all dependencies
# - Configure cloud-init for automatic setup
# - Generate SSH keys for easy access
```

### 2. Start Development VM
```bash
# Interactive development mode
./test_env/run_qemu_simple.sh interactive

# Automated testing mode
./test_env/run_qemu_simple.sh test
```

### 3. Connect and Develop
```bash
# SSH into the VM
ssh vexfs@localhost -p 2222
# Password: vexfs123

# Quick commands in VM:
vs      # Change to VexFS source directory
vt      # Run full test suite
build-kernel  # Build kernel module only
build-vexctl  # Build userspace tool only
```

## 🏗️ Architecture

### VM Configuration
- **Base Image**: Ubuntu 22.04 Server Cloud Image
- **Memory**: 4GB (configurable via `VEXFS_VM_MEMORY`)
- **CPUs**: 4 cores (configurable via `VEXFS_VM_CPUS`)
- **Disk**: 20GB resizable qcow2 image
- **Source Mounting**: Live virtfs mount at `/mnt/vexfs-src`

### File Structure
```
test_env/
├── setup_vm.sh              # One-time VM setup
├── run_qemu_simple.sh        # VM runner with source mounting
├── test_module.sh            # Comprehensive test suite
├── SIMPLIFIED_VM_SETUP.md    # This documentation
└── vm/                       # VM assets (created by setup)
    ├── vexfs-vm-base.qcow2   # VM disk image
    ├── cloud-init.iso        # Boot configuration
    ├── vexfs_test_disk.img   # Additional test disk
    └── cloud-init/           # Cloud-init configuration
        ├── user-data         # User and package setup
        └── meta-data         # VM metadata
```

## 🧪 Testing Workflow

### Fast Development Cycle
1. **Edit Code** (on host): Changes immediately visible in VM
2. **Test in VM**: `ssh vexfs@localhost -p 2222 -t 'vt'`
3. **Debug**: Access live system with `dmesg`, `gdb`, etc.
4. **Iterate**: No VM rebuild needed

### Automated Testing
```bash
# Run full automated test suite
./test_env/run_qemu_simple.sh test

# This will:
# - Boot VM in headless mode
# - Mount source directory
# - Build vexctl and kernel module
# - Run static analysis (clippy)
# - Test module loading/unloading
# - Validate FFI integration
# - Report results and shutdown
```

### Manual Testing
```bash
# Start interactive VM
./test_env/run_qemu_simple.sh interactive

# In another terminal, connect and test
ssh vexfs@localhost -p 2222

# Inside VM:
vs                    # Go to source
build-kernel          # Build kernel module
sudo insmod vexfs.ko  # Load module
dmesg | tail -20      # Check kernel messages
vt                    # Run full test suite
```

## 🔧 Configuration

### Environment Variables
```bash
# VM configuration
export VEXFS_VM_MEMORY="8G"        # VM memory (default: 4G)
export VEXFS_VM_CPUS="8"           # VM CPU cores (default: 4)
export VEXFS_VM_SSH_PORT="2222"    # SSH port (default: 2222)

# Example: Start high-performance VM
VEXFS_VM_MEMORY=8G VEXFS_VM_CPUS=8 ./test_env/run_qemu_simple.sh
```

### VM Customization
The VM can be customized by editing `test_env/vm/cloud-init/user-data`:
- Add additional packages
- Configure development tools
- Set up custom aliases
- Install debugging tools

## 📊 Test Suite Details

### Comprehensive Testing (`test_module.sh`)
The test suite validates:

1. **Build System**
   - vexctl compilation
   - Kernel module compilation
   - Static analysis (clippy)

2. **Module Operations**
   - Module loading/unloading
   - Kernel message validation
   - Symbol export verification

3. **FFI Integration**
   - FFI symbol availability
   - Basic FFI functionality
   - Integration test execution

4. **File System Operations**
   - Test disk management
   - Mount/unmount operations
   - vexctl communication

5. **Vector Operations**
   - Vector test runner execution
   - Vector search validation
   - ANNS functionality

6. **Performance & Stability**
   - Memory usage monitoring
   - CPU stress testing
   - I/O performance validation
   - System stability checks

### Test Results
- **Log Files**: Stored in `test_results/test_log_TIMESTAMP.txt`
- **Success Metrics**: Pass/fail counts and detailed logs
- **Performance Data**: Memory, CPU, and I/O metrics

## 🐛 Debugging Guide

### Kernel Module Debugging
```bash
# In VM:
sudo insmod vexfs.ko
dmesg | tail -50                # Check kernel messages
cat /proc/modules | grep vexfs  # Verify module loading
sudo rmmod vexfs               # Clean unload

# Debug with gdb (if debug symbols available)
gdb vmlinux
(gdb) target remote localhost:1234  # If QEMU started with -s -S
```

### Performance Monitoring
```bash
# In VM:
htop                 # System overview
perf top            # CPU profiling
iostat 1            # I/O monitoring
free -h             # Memory usage
strace -p <pid>     # Syscall tracing
```

### FFI Debugging
```bash
# In VM:
cd /mnt/vexfs-src/vexfs
make test_ffi                    # Build FFI test
./test_ffi                      # Run FFI test
objdump -t vexfs.ko | grep ffi  # Check FFI symbols
```

## 🚨 Troubleshooting

### Common Issues

#### VM Won't Boot
```bash
# Check if KVM is available
kvm-ok

# Check QEMU installation
qemu-system-x86_64 --version

# Re-run setup
rm -rf test_env/vm
./test_env/setup_vm.sh
```

#### Source Not Mounted
```bash
# In VM, manually mount:
sudo mount -t 9p -o trans=virtio,version=9p2000.L vexfs-src /mnt/vexfs-src

# Check mount
mount | grep vexfs-src
```

#### SSH Connection Failed
```bash
# Check if VM is running
ps aux | grep qemu

# Check port forwarding
netstat -tlnp | grep 2222

# Try different SSH port
VEXFS_VM_SSH_PORT=2223 ./test_env/run_qemu_simple.sh
```

#### Module Loading Failed
```bash
# In VM:
dmesg | grep -i error           # Check kernel errors
modinfo /mnt/vexfs-src/fs/vexfs.ko  # Verify module info
lsmod | grep -v vexfs           # Check conflicting modules
```

### Performance Issues
```bash
# Increase VM resources
VEXFS_VM_MEMORY=8G VEXFS_VM_CPUS=8 ./test_env/run_qemu_simple.sh

# Check host system resources
free -h
nproc
```

## 📈 Performance Comparison

| Metric | Old (Packer) | New (Cloud Image) | Improvement |
|--------|--------------|-------------------|-------------|
| Initial Setup | 20-30 min | 2-3 min | 10x faster |
| Code Change Iteration | 10-20 min | <30 sec | 20-40x faster |
| Test Execution | Build + Test | Test only | 5-10x faster |
| Debugging Capability | Limited | Full access | Significant |
| Disk Usage | ~2GB per build | ~1GB total | 50% less |

## 🔮 Future Enhancements

### Planned Improvements
1. **Container Integration**: Docker support for even faster iteration
2. **IDE Integration**: VS Code remote development setup
3. **CI/CD Pipeline**: GitHub Actions integration
4. **Performance Benchmarking**: Automated performance regression testing
5. **Multiple Kernel Versions**: Testing against different kernel versions

### Advanced Features
1. **Cluster Testing**: Multi-VM distributed testing
2. **Stress Testing**: Extended stress and stability testing
3. **Memory Analysis**: Valgrind and AddressSanitizer integration
4. **Security Testing**: Security vulnerability scanning

## 📚 References

- [QEMU Documentation](https://www.qemu.org/docs/master/)
- [Cloud-Init Documentation](https://cloud-init.readthedocs.io/)
- [Linux Kernel Module Programming](https://tldp.org/LDP/lkmpg/2.6/html/)
- [Rust for Linux](https://rust-for-linux.com/)

---

This simplified approach enables rapid development and testing while maintaining the comprehensive validation needed for kernel-level development.