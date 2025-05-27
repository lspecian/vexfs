# VexFS QEMU Testing Environment Setup Guide

This guide covers the new simplified QEMU testing environment for VexFS kernel module development and testing.

## Overview

The new testing environment provides:
- **Fast Setup**: VM boots in <2 minutes (vs 10-20 min Packer build)
- **Live Development**: VexFS source mounted via virtfs for real-time changes
- **Automated Testing**: Integrated scripts for building and testing kernel modules
- **Minimal Dependencies**: Uses cloud images instead of complex Packer builds

## Quick Start

### 1. Initial Setup
```bash
cd test_env
./setup_vm.sh
```

This will:
- Download Ubuntu Server cloud image (if not present)
- Create SSH key pair for VM access
- Prepare cloud-init configuration
- Set up necessary directories

### 2. Start VM
```bash
./vm_control.sh start
```

### 3. Run Tests
```bash
./test_module.sh
```

## Detailed Usage

### VM Management (`vm_control.sh`)

**Start VM:**
```bash
./vm_control.sh start [gui]          # Start VM (headless by default, add 'gui' for display)
```

**Stop VM:**
```bash
./vm_control.sh stop                 # Graceful shutdown
./vm_control.sh kill                 # Force terminate
```

**VM Status:**
```bash
./vm_control.sh status               # Check if VM is running
```

**SSH Access:**
```bash
./vm_control.sh ssh                  # Connect to VM via SSH
./vm_control.sh ssh-copy-id          # Copy SSH keys to VM
```

**File Transfer:**
```bash
./vm_control.sh scp local_file:/remote/path    # Copy file to VM
./vm_control.sh scp-from /remote/path:local_file  # Copy file from VM
```

### Testing (`test_module.sh`)

**Full Test Suite:**
```bash
./test_module.sh                     # Run complete test suite
```

**Individual Test Steps:**
```bash
./test_module.sh build               # Build VexFS module only
./test_module.sh load                # Load module into kernel
./test_module.sh test                # Run FFI integration tests
./test_module.sh unload              # Unload module
```

**Cleanup:**
```bash
./test_module.sh clean               # Clean build artifacts
```

## VM Configuration

### Resources
- **Memory**: 2GB RAM
- **CPUs**: 2 cores
- **Storage**: 20GB (cloud image + overlay)
- **Network**: User-mode networking with SSH port forwarding

### Mounted Directories
- **VexFS Source**: `/mnt/vexfs` (live mount via virtfs)
- **Build Output**: Available in VM at `/mnt/vexfs/fs/`

### Pre-installed Tools
- Build essentials (gcc, make, cmake)
- Kernel development headers
- Rust toolchain with kernel components
- Debugging tools (gdb, strace, valgrind)
- Development utilities

## File Structure

```
test_env/
├── setup_vm.sh              # Initial environment setup
├── vm_control.sh             # VM lifecycle management
├── test_module.sh            # Automated testing
├── run_qemu.sh               # Legacy Packer-based script
├── cloud-init-user-data.yaml # VM configuration
├── QEMU_SETUP_GUIDE.md       # This guide
├── images/                   # VM images directory
│   └── ubuntu-server.img     # Downloaded cloud image
├── ssh_keys/                 # SSH key pairs
│   ├── vexfs_key            # Private key
│   └── vexfs_key.pub        # Public key
└── vm_state/                 # VM runtime state
    ├── vm.pid               # VM process ID
    └── vm.monitor           # QEMU monitor socket
```

## Development Workflow

### 1. Code-Test-Debug Cycle
```bash
# Edit code on host
vim fs/src/ffi.rs

# Test in VM (automatically synced)
./test_module.sh

# Debug if needed
./vm_control.sh ssh
# Inside VM:
dmesg | tail -20
gdb /mnt/vexfs/fs/test_ffi
```

### 2. Kernel Module Development
```bash
# Build module
./test_module.sh build

# Load and test
./test_module.sh load
./test_module.sh test

# Check kernel logs
./vm_control.sh ssh -c "dmesg | grep vexfs"

# Unload when done
./test_module.sh unload
```

### 3. FFI Integration Testing
```bash
# Run complete FFI test suite
./test_module.sh

# Check specific FFI functions
./vm_control.sh ssh -c "cd /mnt/vexfs/fs && ./test_ffi_integration"
```

## Troubleshooting

### VM Won't Start
```bash
# Check if KVM is available
lsmod | grep kvm
sudo modprobe kvm kvm_intel  # or kvm_amd

# Check VM status
./vm_control.sh status

# View VM logs
./vm_control.sh start gui    # Start with display for debugging
```

### SSH Connection Issues
```bash
# Reset SSH keys
rm -rf ssh_keys/
./setup_vm.sh

# Manual SSH connection
ssh -i ssh_keys/vexfs_key -p 2222 vexfs@localhost
```

### Mount Issues
```bash
# Inside VM, check mounts
mount | grep vexfs
ls -la /mnt/vexfs

# Manually mount if needed
sudo mount -t 9p -o trans=virtio,version=9p2000.L vexfs_source /mnt/vexfs
```

### Build Issues
```bash
# Check kernel headers
./vm_control.sh ssh -c "ls -la /lib/modules/\$(uname -r)/build"

# Verify Rust installation
./vm_control.sh ssh -c "rustc --version"

# Clean and rebuild
./test_module.sh clean
./test_module.sh build
```

## Performance Notes

### VM Startup Time
- **Cold start**: ~90 seconds (including cloud-init)
- **Warm start**: ~30 seconds (if image cached)
- **vs Packer**: 10x faster than previous 10-20 minute builds

### Development Iteration
- **Code changes**: Instant (virtfs mount)
- **Module rebuild**: ~10-30 seconds
- **Test cycle**: ~1-2 minutes total

### Resource Usage
- **Host RAM**: ~2.5GB (2GB VM + overhead)
- **Host CPU**: Minimal when idle
- **Disk Space**: ~2GB for base image + overlays

## Migration from Packer

If you have existing Packer-based setup:

1. **Backup existing work** (if any important state in VM)
2. **Use new scripts** for better performance
3. **Legacy support** available via `run_qemu.sh` (with warning)

The new setup is **fully compatible** with existing VexFS code and provides the same kernel testing capabilities with significantly improved performance.

## Advanced Configuration

### Custom Cloud-Init
Edit `cloud-init-user-data.yaml` to:
- Add additional packages
- Configure specific services
- Set custom environment variables
- Install additional development tools

### VM Resources
Edit `vm_control.sh` to modify:
- RAM allocation (`VM_MEMORY`)
- CPU count (`VM_CPUS`)
- Port forwarding (`SSH_PORT`)

### Network Configuration
```bash
# Port forwarding for additional services
# Edit vm_control.sh QEMU_CMD to add:
# -netdev user,id=net0,hostfwd=tcp::8080-:80,hostfwd=tcp::2222-:22
```

## Security Notes

- VM uses user-mode networking (isolated from host network)
- SSH keys are generated locally and not shared
- VM has no direct access to host filesystem (except mounted VexFS source)
- Root access in VM is contained and safe for development/testing

## Support

For issues with the testing environment:
1. Check this guide's troubleshooting section
2. Examine VM logs via `./vm_control.sh ssh -c "dmesg"`
3. Test basic QEMU functionality: `qemu-system-x86_64 --version`
4. Verify KVM support: `lsmod | grep kvm`