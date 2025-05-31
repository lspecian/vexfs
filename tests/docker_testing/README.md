# VexFS Docker Kernel Module Testing

This directory contains Docker-based testing for the VexFS kernel module. Docker is ideal for kernel module testing because:

- **Shared Kernel**: Containers share the host kernel, so we can load/test kernel modules
- **Isolation**: Container provides process isolation while accessing host kernel
- **No VM Overhead**: Much faster than full VM testing
- **Reproducible**: Consistent testing environment

## Quick Start

```bash
# Run the complete test suite
./tests/docker_testing/test_kernel_module.sh
```

## What the Tests Do

1. **Build Docker Image**: Creates Ubuntu container with kernel development tools
2. **Module Loading**: Tests loading the memory-fixed kernel module
3. **Basic Operations**: Tests filesystem registration and basic operations
4. **Memory Safety**: Validates that the memory fixes prevent crashes
5. **Cleanup**: Properly unloads the module

## Safety Notes

⚠️ **Important**: The Docker container shares the host kernel, so:
- Loading the kernel module affects the host system
- If the module crashes, it could crash the host
- Always test on a development machine, not production

## Test Results

The test validates that our memory fixes work:
- `vi->vfs_inode.i_sb = sb;` prevents NULL pointer dereferences
- Removed unsafe `mark_inode_dirty()` calls
- Proper inode initialization order

## Files

- `Dockerfile.kernel_test`: Docker image with kernel development tools
- `test_kernel_module.sh`: Main test script
- `run_tests_inside_container.sh`: Generated script that runs inside container

## Manual Testing

If you want to test manually:

```bash
# Build the image
docker build -f tests/docker_testing/Dockerfile.kernel_test -t vexfs-kernel-test .

# Run interactive container
docker run --rm -it --privileged \
    --volume /lib/modules:/lib/modules:ro \
    --volume /dev:/dev \
    vexfs-kernel-test /bin/bash

# Inside container:
insmod /vexfs/kernel/vexfs_minimal.ko
lsmod | grep vexfs
rmmod vexfs_minimal
```

## Advantages over VM Testing

- **Speed**: No VM boot time, instant testing
- **Simplicity**: No complex VM management or SSH setup
- **Resource Efficient**: Uses host kernel directly
- **Debugging**: Easy to add debugging tools to container