# VexFS VM Testing - Quick Start

## Overview
This setup provides a lightweight, fast-iteration testing environment for VexFS kernel development.

## Quick Commands

```bash
# 1. Setup VM (one-time)
./test_env/setup_vm.sh

# 2. Start VM
./test_env/run_qemu.sh

# 3. SSH into VM (in another terminal)
./test_env/ssh_vm.sh

# 4. Build in VM
./test_env/build_in_vm.sh

# 5. Test kernel module
./test_env/test_in_vm.sh
```

## Key Features

- **Fast Boot**: VM boots in ~30 seconds (vs 10-20 min Packer builds)
- **Live Source**: VexFS source mounted via virtfs - changes are instant
- **No Rebuilds**: Edit code on host, build in VM immediately
- **Automated Setup**: Dependencies installed automatically via cloud-init
- **Kernel Ready**: Includes kernel headers and build tools

## VM Details

- **OS**: Ubuntu 22.04 Server (cloud image)
- **Memory**: 2GB RAM
- **CPUs**: 2 cores
- **Disk**: 20GB (dynamic)
- **User**: vexfs (passwordless sudo)
- **SSH Port**: 2222 (host) → 22 (guest)
- **VNC Port**: 5900 (if display needed)

## Development Workflow

1. **Edit** VexFS source code on host (any editor)
2. **Build** in VM: `./test_env/build_in_vm.sh`
3. **Test** kernel module: `./test_env/test_in_vm.sh`
4. **Debug** via SSH: `./test_env/ssh_vm.sh`

## Troubleshooting

- **VM won't start**: Check `./test_env/setup_vm.sh` was run
- **SSH fails**: Wait 30-60s for VM to fully boot
- **Build fails**: Ensure Rust environment: `source ~/.cargo/env`
- **Module load fails**: Check dmesg for kernel errors

## Architecture Benefits

- No complex Packer dependencies
- No static VM images to rebuild
- Fast edit-test-debug cycles
- Real kernel environment validation
- Minimal resource usage
