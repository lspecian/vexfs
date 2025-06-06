# VexFS Scripts

This directory contains scripts for setting up and testing VexFS.

## Scripts

### `setup_test_vexfs.sh`
**Main setup script** - Sets up a complete VexFS testing environment.

**What it does:**
- Builds vexctl userspace tool
- Compiles VexFS kernel module
- Loads the kernel module
- Creates a filesystem image file
- Mounts VexFS at `/tmp/test`
- Creates test and cleanup scripts

**Usage:**
```bash
./scripts/setup_test_vexfs.sh
```

**Requirements:**
- Linux with kernel headers
- Build tools (gcc, make)
- Rust toolchain
- sudo access for module loading and mounting

### `test_vexfs.sh` (created by setup)
**Testing script** - Tests vexctl functionality with the mounted filesystem.

**Usage:**
```bash
./scripts/test_vexfs.sh
```

### `cleanup_vexfs.sh` (created by setup)
**Cleanup script** - Removes the test filesystem and kernel module.

**Usage:**
```bash
./scripts/cleanup_vexfs.sh
```

## Quick Start

1. **Setup everything:**
   ```bash
   ./scripts/setup_test_vexfs.sh
   ```

2. **Test vexctl:**
   ```bash
   ./scripts/test_vexfs.sh
   ```

3. **Clean up:**
   ```bash
   ./scripts/cleanup_vexfs.sh
   ```

## Understanding the Error

If you see:
```
Error: VexFS filesystem not found at path: /tmp/test
```

This means vexctl is looking for a mounted VexFS filesystem but can't find one. The setup script resolves this by:

1. Building and loading the VexFS kernel module
2. Creating a filesystem image
3. Mounting it at the expected location

## Safety Notes

- **Kernel modules can crash your system** - Test in VMs when possible
- **Don't store important data** on development filesystems
- **Always run cleanup** when done testing

## Troubleshooting

### Module won't load
```bash
# Check kernel messages
dmesg | tail -10

# Verify module file
ls -la vexfs.ko
modinfo vexfs.ko
```

### Mount fails
```bash
# Check if module is loaded
lsmod | grep vexfs

# Check filesystem support
cat /proc/filesystems | grep vexfs
```

### Permission errors
```bash
# Run with sudo when needed
sudo ./vexctl/target/release/vexctl status /tmp/test
```

For detailed troubleshooting, see [docs/testing/VEXCTL_TESTING_GUIDE.md](../docs/testing/VEXCTL_TESTING_GUIDE.md).