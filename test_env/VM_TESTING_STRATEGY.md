# VexFS VM Testing Strategy

This document outlines the simplified VM-based testing approach for VexFS kernel module development.

## Overview

To properly test VexFS as a kernel module, we need a controlled environment where we can:

1. **Load and unload kernel modules safely**
2. **Test FFI integration between Rust and C kernel code**
3. **Validate filesystem operations in kernel space**
4. **Debug kernel panics and memory issues**
5. **Iterate quickly during development**

## New Simplified Approach: Cloud Images + virtfs

### Advantages
- **Fast setup**: VM ready in <2 minutes (vs 10-20 min Packer builds)
- **Live development**: Source code mounted via virtfs for instant updates
- **No rebuilds**: Changes don't require VM image recreation
- **Simple dependencies**: Only QEMU and cloud-utils required
- **Development friendly**: Edit-test-debug cycle optimized

### Key Improvements
- **Pre-built base**: Ubuntu cloud images (download once, use many times)
- **Cloud-init setup**: Automatic dependency installation on first boot
- **Source mounting**: VexFS source accessible at `/mnt/vexfs_source`
- **Persistent VM**: Keep running between development sessions

## Architecture

```
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│   Host System   │    │   Cloud Image    │    │   QEMU VM       │
│                 │    │                  │    │                 │
│ VexFS Source   ├────► Ubuntu 22.04     ├────► Live Testing    │
│ Development    │    │ + Cloud-init     │    │ + virtfs Mount  │
│ Tools          │    │ + SSH Access     │    │ + Fast Builds   │
└─────────────────┘    └──────────────────┘    └─────────────────┘
         │                                             │
         └─────────────── virtfs mount ────────────────┘
```

## Quick Start

### 1. One-time Setup
```bash
# Setup VM environment (downloads cloud image, creates VM)
./test_env/setup_vm.sh

# This creates:
# - Ubuntu 22.04 cloud image VM
# - Cloud-init configuration for dependencies
# - SSH keys for access
# - Helper scripts for VM management
```

### 2. Development Workflow
```bash
# Start VM (boots in ~1 minute)
./test_env/vm_control.sh start

# SSH into VM
./test_env/vm_control.sh ssh

# In VM: Mount VexFS source
sudo mkdir -p /mnt/vexfs_source
sudo mount -t 9p -o trans=virtio,version=9p2000.L vexfs_source /mnt/vexfs_source

# Build and test (source is live-mounted)
cd /mnt/vexfs_source/vexfs
make clean && make
sudo insmod vexfs.ko
dmesg | tail
sudo rmmod vexfs
```

### 3. Automated Testing
```bash
# Run full test suite
./test_env/test_module.sh

# Individual test components
./test_env/test_module.sh build   # Build only
./test_env/test_module.sh load    # Test loading only
./test_env/test_module.sh ffi     # Test FFI only
```

## VM Management Scripts

### setup_vm.sh
- **One-time setup**: Downloads cloud image, creates VM disk
- **Cloud-init config**: Installs Rust, GCC, kernel headers
- **SSH access**: Generates keys for passwordless access
- **Helper scripts**: Creates vm_control.sh and test_module.sh

### vm_control.sh
```bash
./vm_control.sh start          # Start VM with GUI
./vm_control.sh start-headless # Start without GUI (VNC)
./vm_control.sh stop           # Graceful shutdown
./vm_control.sh ssh [command]  # SSH into VM
./vm_control.sh status         # Check if VM is running
./vm_control.sh monitor        # Access QEMU monitor
```

### test_module.sh
```bash
./test_module.sh test    # Full test suite
./test_module.sh build   # Build module only
./test_module.sh load    # Test loading only
./test_module.sh ffi     # FFI integration tests
```

## Enhanced run_qemu.sh Features

### Source Code Mounting
- **virtfs integration**: VexFS source mounted live in VM
- **No copy delays**: Instant access to host changes
- **Bidirectional**: Changes visible immediately

### Flexible Configuration
```bash
./run_qemu.sh --headless     # VNC only (port 5900)
./run_qemu.sh --background   # Daemon mode
./run_qemu.sh --no-mount     # Don't mount source
./run_qemu.sh --memory 4G    # Adjust RAM
./run_qemu.sh --cpus 4       # Adjust CPU count
```

### Network Access
- **SSH**: Port 2222 → VM port 22
- **VNC**: Port 5900 (headless mode)
- **Monitor**: Port 55555 for QEMU commands

## Development Workflow

### Fast Iteration Process
1. **Edit VexFS source** on host system
2. **Changes immediately visible** in VM via virtfs
3. **Build in VM**: `cd /mnt/vexfs_source/vexfs && make`
4. **Test loading**: `sudo insmod vexfs.ko`
5. **Check results**: `dmesg | tail`
6. **Repeat cycle** (30 seconds vs 20 minutes)

### Debugging Workflow
```bash
# Start VM with debugging
./vm_control.sh start

# SSH and enable debugging
./vm_control.sh ssh
echo 'file vexfs.c +p' | sudo tee /sys/kernel/debug/dynamic_debug/control

# Load module with verbose logging
cd /mnt/vexfs_source/vexfs
sudo insmod vexfs.ko
sudo dmesg -w  # Watch kernel messages live
```

## Testing Scenarios

### 1. FFI Integration Validation
```bash
# Pre-built validation (runs on host)
./test_env/validate_ffi_integration.sh

# In-VM testing
./vm_control.sh ssh "cd /mnt/vexfs_source/vexfs && ./test_ffi"
```

### 2. Module Loading Tests
```bash
# Automated testing
./test_module.sh load

# Manual testing
./vm_control.sh ssh
cd /mnt/vexfs_source/vexfs
sudo insmod vexfs.ko
lsmod | grep vexfs
sudo rmmod vexfs
```

### 3. Kernel Integration Tests
```bash
# Test with kernel debugging
./vm_control.sh ssh
echo 8 | sudo tee /proc/sys/kernel/printk  # Enable debug messages
cd /mnt/vexfs_source/vexfs
sudo insmod vexfs.ko
# Module logs appear in dmesg
```

## Performance Benefits

### Time Comparisons
| Operation | Old (Packer) | New (Cloud-init) |
|-----------|--------------|------------------|
| Initial setup | 15-20 min | 2-3 min |
| Code change test | 15-20 min | 30 seconds |
| VM boot | 2-3 min | 1-2 min |
| Dependency install | Every build | One-time |

### Resource Usage
- **Disk**: ~2GB VM image (vs multiple rebuild artifacts)
- **Network**: One-time cloud image download
- **CPU**: No rebuilding overhead during development

## VM Configuration

### Installed Packages (via cloud-init)
- **Build essentials**: gcc, make, build-essential
- **Kernel development**: linux-headers-generic
- **Rust toolchain**: via rustup
- **Development tools**: git, vim, htop, tree
- **Debugging tools**: gdb (available on demand)

### VM Specifications
- **Base**: Ubuntu 22.04 LTS Server
- **Memory**: 2GB (configurable)
- **CPUs**: 2 (configurable) 
- **Disk**: 10GB (expandable)
- **User**: vexfs (passwordless sudo)

### Network Configuration
- **SSH**: Host port 2222 → VM port 22
- **VNC**: Host port 5900 (headless mode)
- **Internet**: Full access for package installation

## Implementation Details

### Cloud-init Configuration
The setup automatically creates cloud-init configuration that:
- Creates `vexfs` user with sudo access
- Installs development dependencies
- Configures Rust toolchain
- Sets up SSH access
- Prepares virtfs mount points

### virtfs Integration
VexFS source code is mounted using QEMU's virtfs feature:
```bash
-virtfs local,path=${PROJECT_ROOT},mount_tag=vexfs_source,security_model=passthrough,id=vexfs_source
```

This provides:
- **Live updates**: Changes on host immediately visible in VM
- **No synchronization lag**: Direct filesystem access
- **Bidirectional access**: VM can write build artifacts back to host

### SSH Key Management
- **Automatic generation**: SSH keys created during setup
- **Secure access**: No passwords required
- **Easy connection**: Helper scripts handle connection details

## Script Architecture

### setup_vm.sh
1. **Dependency checking**: Validates QEMU, cloud-utils installation
2. **Image download**: Fetches Ubuntu 22.04 cloud image
3. **VM creation**: Creates qcow2 image with additional space
4. **Cloud-init setup**: Generates user-data and meta-data
5. **SSH configuration**: Creates access keys
6. **Helper creation**: Generates management scripts

### vm_control.sh
- **Process management**: Start/stop VM instances
- **Access control**: SSH connections and monitoring
- **Status checking**: VM health and port availability

### test_module.sh
- **Build automation**: Compile kernel module in VM
- **Test execution**: Load/unload module testing
- **FFI validation**: Integration test execution
- **Result reporting**: Clear success/failure indication

## Troubleshooting

### Common Issues
1. **virtfs mount fails**: Fallback to manual source copying
2. **SSH connection refused**: VM may still be booting
3. **Module build fails**: Check kernel headers installation
4. **Permission denied**: Ensure sudo access in VM

### Debug Techniques
```bash
# Check VM status
./vm_control.sh status

# Monitor QEMU directly
./vm_control.sh monitor

# Check cloud-init progress
./vm_control.sh ssh "sudo cloud-init status"

# View kernel messages
./vm_control.sh ssh "sudo dmesg | tail -20"
```

## Security Considerations

- **Isolated testing**: VM crashes don't affect host
- **SSH key authentication**: Secure access without passwords
- **Kernel module safety**: Contained environment for testing
- **No persistent storage**: Easy to reset VM state

## Future Enhancements

### Automation Opportunities
- **Git hooks**: Auto-test on commits
- **CI integration**: Automated VM testing
- **Performance benchmarks**: Track module performance

### Development Tools
- **VSCode remote**: Direct editing in VM
- **GDB integration**: Kernel debugging
- **Log aggregation**: Centralized logging

This simplified strategy provides fast iteration while maintaining robust testing capabilities for VexFS kernel module development.