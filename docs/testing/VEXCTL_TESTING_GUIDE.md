# VexFS vexctl Testing Guide

This guide explains how to properly test the vexctl tool with a real VexFS filesystem.

## Quick Start

1. **Run the setup script:**
   ```bash
   ./scripts/setup_test_vexfs.sh
   ```

2. **Test vexctl:**
   ```bash
   ./scripts/test_vexfs.sh
   ```

3. **Clean up when done:**
   ```bash
   ./scripts/cleanup_vexfs.sh
   ```

## Understanding the "Filesystem Not Found" Error

When you run vexctl commands like:
```bash
./vexctl/target/release/vexctl status /tmp/test
```

You get the error:
```
Error: VexFS filesystem not found at path: /tmp/test
```

### Why This Happens

The vexctl tool expects to connect to a **mounted VexFS filesystem**. The error occurs because:

1. **No VexFS kernel module loaded** - The VexFS kernel module isn't compiled or loaded
2. **No filesystem mounted** - Even if the module exists, no VexFS filesystem is mounted at `/tmp/test`
3. **Path doesn't exist** - The specified path doesn't exist or isn't accessible

### What vexctl Actually Needs

vexctl performs these checks when connecting:
1. **Path exists** - The mount point directory must exist
2. **VexFS filesystem mounted** - A VexFS filesystem must be mounted at that path
3. **IOCTL interface available** - The kernel module must support VexFS-specific IOCTL calls
4. **Proper permissions** - User must have access to the filesystem

## Prerequisites

### System Requirements

1. **Linux kernel headers:**
   ```bash
   # Ubuntu/Debian
   sudo apt-get install linux-headers-$(uname -r)
   
   # CentOS/RHEL
   sudo yum install kernel-devel
   
   # Arch Linux
   sudo pacman -S linux-headers
   ```

2. **Build tools:**
   ```bash
   # Ubuntu/Debian
   sudo apt-get install build-essential
   
   # CentOS/RHEL
   sudo yum groupinstall "Development Tools"
   
   # Arch Linux
   sudo pacman -S base-devel
   ```

3. **Rust toolchain:**
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   source ~/.cargo/env
   ```

## Manual Setup Process

If you prefer to understand each step, here's what the setup script does:

### 1. Build vexctl

```bash
cd vexctl
cargo build --release
cd ..
```

### 2. Build the Kernel Module

```bash
make clean
make
```

This creates `vexfs.ko` - the kernel module file.

### 3. Load the Kernel Module

```bash
sudo insmod vexfs.ko
```

Verify it's loaded:
```bash
lsmod | grep vexfs
dmesg | tail -10  # Check for any error messages
```

### 4. Create a Filesystem Image

```bash
dd if=/dev/zero of=/tmp/vexfs.img bs=1M count=100
```

This creates a 100MB file to use as the filesystem.

### 5. Mount the Filesystem

```bash
sudo mkdir -p /tmp/test
sudo mount -t vexfs /tmp/vexfs.img /tmp/test
```

Verify the mount:
```bash
mount | grep vexfs
```

### 6. Test vexctl

```bash
./vexctl/target/release/vexctl status /tmp/test
```

## Testing Commands

Once VexFS is mounted, you can test various vexctl commands:

### Status Command
```bash
./vexctl/target/release/vexctl status /tmp/test
```

### Search Operations (when implemented)
```bash
./vexctl/target/release/vexctl search --query "0.1,0.2,0.3" --top-k 5 /tmp/test
```

### Index Management (when implemented)
```bash
./vexctl/target/release/vexctl list-indexes /tmp/test
./vexctl/target/release/vexctl create-index --name test --dimensions 128 /tmp/test
```

### Filesystem Check (when implemented)
```bash
./vexctl/target/release/vexctl fsck /tmp/test
```

## Troubleshooting

### Common Issues

1. **"Permission denied" errors:**
   ```bash
   sudo ./vexctl/target/release/vexctl status /tmp/test
   ```

2. **"Invalid filesystem" errors:**
   - Verify VexFS module is loaded: `lsmod | grep vexfs`
   - Check mount: `mount | grep vexfs`
   - Check dmesg for kernel errors: `dmesg | tail -20`

3. **Build failures:**
   - Install kernel headers for your kernel version
   - Ensure build tools are installed
   - Check for missing dependencies

4. **Module loading failures:**
   ```bash
   # Check kernel messages
   dmesg | grep vexfs
   
   # Verify module file exists
   ls -la vexfs.ko
   
   # Check module info
   modinfo vexfs.ko
   ```

5. **Mount failures:**
   ```bash
   # Check if module supports mounting
   dmesg | tail -20
   
   # Verify filesystem type is registered
   cat /proc/filesystems | grep vexfs
   ```

### Getting Help

1. **Check kernel logs:**
   ```bash
   dmesg | grep vexfs
   journalctl -k | grep vexfs
   ```

2. **Verify filesystem mount:**
   ```bash
   mount | grep vexfs
   cat /proc/mounts | grep vexfs
   ```

3. **Test basic filesystem operations:**
   ```bash
   ls -la /tmp/test
   sudo touch /tmp/test/test_file
   sudo rm /tmp/test/test_file
   ```

4. **Check module status:**
   ```bash
   lsmod | grep vexfs
   modinfo vexfs.ko
   ```

## Cleanup

### Manual Cleanup

```bash
# Unmount filesystem
sudo umount /tmp/test

# Remove kernel module
sudo rmmod vexfs

# Clean up files
rm /tmp/vexfs.img
sudo rmdir /tmp/test
```

### Automated Cleanup

```bash
./scripts/cleanup_vexfs.sh
```

## Development Workflow

### Recommended Testing Sequence

1. **Build and test userspace first** - Ensure vexctl compiles and basic CLI works
2. **Build kernel module** - Compile VexFS for your kernel
3. **Test module loading** - Verify the module loads without errors
4. **Test mounting** - Check if filesystem can be mounted
5. **Test vexctl integration** - Verify userspace-kernel communication

### Iterative Development

```bash
# After making changes to kernel code
make clean && make
sudo rmmod vexfs
sudo insmod vexfs.ko

# After making changes to vexctl
cd vexctl && cargo build --release && cd ..

# Test the changes
./vexctl/target/release/vexctl status /tmp/test
```

### Debugging Tips

1. **Use verbose kernel logging:**
   ```bash
   echo 8 > /proc/sys/kernel/printk  # Enable debug messages
   dmesg -w  # Watch kernel messages in real-time
   ```

2. **Check IOCTL interface:**
   ```bash
   strace ./vexctl/target/release/vexctl status /tmp/test
   ```

3. **Monitor filesystem operations:**
   ```bash
   sudo strace -e trace=file ls /tmp/test
   ```

## Safety Notes

- **Development filesystem** - Don't store important data on VexFS during development
- **Kernel module risks** - Buggy kernel modules can crash the system
- **Use VMs for testing** - Consider using virtual machines for safer kernel development
- **Backup your work** - Always commit code changes before testing kernel modules

## Next Steps

1. **Implement IOCTL interface** - Add proper kernel-userspace communication
2. **Add filesystem operations** - Implement file creation, deletion, etc.
3. **Vector operations** - Add vector storage and search functionality
4. **Performance testing** - Benchmark filesystem and vector operations
5. **Stability testing** - Long-running tests and stress testing

For VM-based testing, see [VM_TESTING_STRATEGY.md](VM_TESTING_STRATEGY.md).