# VexFS Testing Environment

This directory contains the **simplified** testing infrastructure for VexFS development, featuring fast VM setup with live source mounting for rapid iteration.

## 🚀 Quick Start

### 1. One-Time Setup
```bash
# Setup VM environment (downloads Ubuntu cloud image ~500MB)
./test_env/setup_vm.sh
```

### 2. Start Development
```bash
# Start VM (headless by default, add 'gui' for display)
./test_env/vm_control.sh start

# Run comprehensive tests
./test_env/test_module.sh
```

### 3. Connect and Test
```bash
# SSH into VM
./test_env/vm_control.sh ssh

# Quick commands in VM:
cd /mnt/vexfs      # Go to VexFS source
make -C vexfs      # Build kernel module
./test_ffi         # Test FFI integration
```

## 📁 Files

### New Cloud-Init Approach
- `setup_vm.sh` - **One-time VM setup** (downloads cloud image, creates SSH keys)
- `vm_control.sh` - **VM lifecycle management** (start/stop/ssh/status)
- `test_module.sh` - **Comprehensive test suite** for kernel module validation
- `cloud-init-user-data.yaml` - **VM configuration** with automated setup
- `QEMU_SETUP_GUIDE.md` - **Complete documentation** for modern approach

### Legacy Files (Deprecated)
- `vexfs.pkr.hcl` - ⚠️ Old Packer configuration (slow, complex)
- `run_qemu.sh` - ⚠️ Old QEMU runner (static image)
- `http/preseed.cfg` - ⚠️ Old Debian preseed (not needed)

## ⚡ Key Improvements

| Feature | Old (Packer) | New (Cloud Image) | Improvement |
|---------|--------------|-------------------|-------------|
| **Setup Time** | 20-30 min | 2-3 min | **10x faster** |
| **Code Changes** | Full rebuild | Live mounting | **20-40x faster** |
| **Source Integration** | Baked into VM | virtfs mount | **Live development** |
| **Debugging** | Limited access | Full SSH access | **Complete access** |

## 🧪 Testing Modes

### VM Management
```bash
./test_env/vm_control.sh start        # Start VM (headless)
./test_env/vm_control.sh start gui    # Start VM with display
./test_env/vm_control.sh ssh          # Connect via SSH
./test_env/vm_control.sh stop         # Graceful shutdown
```

### Automated Testing
```bash
./test_env/test_module.sh             # Full test suite
./test_env/test_module.sh build       # Build module only
./test_env/test_module.sh load        # Load into kernel
./test_env/test_module.sh test        # Run FFI tests
```

### Manual Development
```bash
# Connect to VM for manual testing
./test_env/vm_control.sh ssh
# Inside VM:
cd /mnt/vexfs/fs
make && sudo insmod vexfs.ko
./test_ffi_integration
```

## 🛠️ Development Workflow

### Fast Iteration Cycle
1. **Edit code** on host (any editor/IDE)
2. **Test in VM**: `./test_env/test_module.sh`
3. **Debug**: `./test_env/vm_control.sh ssh` for full access
4. **Iterate**: Changes appear instantly via virtfs mount!

### Comprehensive Testing
```bash
# Full automated test pipeline
./test_env/test_module.sh

# Validates:
# ✅ Build system (vexctl + kernel module)
# ✅ Module loading/unloading
# ✅ FFI integration
# ✅ Vector operations
# ✅ System stability
```

## 🔧 Configuration

### Environment Variables
```bash
export VEXFS_VM_MEMORY="8G"      # VM memory (default: 4G)
export VEXFS_VM_CPUS="8"         # VM CPUs (default: 4)  
export VEXFS_VM_SSH_PORT="2222"  # SSH port (default: 2222)
```

### Custom VM Settings
```bash
# High-performance VM
VEXFS_VM_MEMORY=8G VEXFS_VM_CPUS=8 ./test_env/run_qemu_simple.sh
```

## 🐛 Debugging

### Kernel Module Debug
```bash
# Connect to VM
./test_env/vm_control.sh ssh

# In VM:
cd /mnt/vexfs/fs
sudo insmod vexfs.ko
dmesg | tail -50              # Check kernel messages
sudo rmmod vexfs             # Clean unload
```

### Performance Monitoring
```bash
# In VM (via SSH):
htop                # System overview
perf top           # CPU profiling
iostat 1           # I/O monitoring
free -h            # Memory usage
```

### FFI Debug
```bash
# In VM:
cd /mnt/vexfs/fs
make test_ffi                 # Build FFI test
./test_ffi_integration       # Run FFI test
objdump -t vexfs.ko | grep ffi  # Check FFI symbols
```

## 📊 Test Results

Test results are logged to `test_results/test_log_TIMESTAMP.txt` with:
- **Pass/fail counts** for each test category
- **Performance metrics** (memory, CPU, I/O)  
- **Detailed logs** for debugging failures
- **System stability** validation

## 🚨 Troubleshooting

### VM Won't Start
```bash
# Check requirements
kvm-ok                        # KVM available?
qemu-system-x86_64 --version # QEMU installed?

# Check VM status
./test_env/vm_control.sh status

# Clean setup
rm -rf test_env/images test_env/ssh_keys test_env/vm_state
./test_env/setup_vm.sh
```

### Source Not Mounted
```bash
# In VM, manually mount:
sudo mount -t 9p -o trans=virtio,version=9p2000.L vexfs_source /mnt/vexfs
```

### SSH Connection Issues
```bash
# Reset SSH keys and try again
rm -rf test_env/ssh_keys
./test_env/setup_vm.sh
./test_env/vm_control.sh ssh-copy-id
```

See `QEMU_SETUP_GUIDE.md` for comprehensive troubleshooting guide.

## 📚 Documentation

- **`QEMU_SETUP_GUIDE.md`** - Complete setup and usage guide
- **`VM_TESTING_STRATEGY.md`** - Testing strategy and architecture
- **Project root `/docs/`** - Overall project documentation

---

## 🔄 Migration from Legacy Packer Setup

If you were using the old Packer-based setup:

### Old Workflow (Deprecated)
```bash
# ❌ Slow, complex process
cd test_env
packer build vexfs.pkr.hcl     # 20-30 minutes
./run_qemu.sh                  # Static VM image
# Code changes required full rebuild
```

### New Workflow (Recommended)
```bash
# ✅ Fast, simple process
./test_env/setup_vm.sh         # 2-3 minutes, once only
./test_env/vm_control.sh start # Start VM
./test_env/test_module.sh      # Live development & testing
# Code changes are instant via virtfs mount
```

### Legacy Test Procedure (Now Simplified)

The old manual testing steps:
1. Build VM image with Packer
2. Start VM with `run_qemu.sh`
3. SSH as root with password
4. Navigate to `/usr/src/vexfs`
5. Build, load module, mount, test, unmount, unload

Are now replaced with:
```bash
# Automated equivalent
./test_env/test_module.sh

# Or interactive equivalent
./test_env/vm_control.sh start
./test_env/vm_control.sh ssh
# Inside VM: cd /mnt/vexfs && make -C vexfs
```

The testing validates the same functionality:
- ✅ Module loading with FFI integration
- ✅ Filesystem registration and mounting
- ✅ Basic VFS operations (lookup, readdir)
- ✅ IOCTL communication with `vexctl`
- ✅ Clean unmounting and module unloading

But with much faster iteration and better debugging capabilities.

---

**🎯 Result: Fast, practical kernel development environment that enables rapid iteration without complex build dependencies.**
