# VexFS VM Kernel Module Testing

This directory contains scripts for testing the VexFS kernel module in a safe VM environment.

## Quick Start

### Prerequisites

1. **QEMU installed**: `sudo apt install qemu-system-x86_64`
2. **VexFS kernel module built**: Run `make` in project root to build `kernel/vexfs.ko`
3. **Internet connection**: For downloading Ubuntu Live ISO (~4GB)

### Running VM Tests

1. **Start VM Testing Environment**:
   ```bash
   cd tests/vm_testing
   ./run_vm_tests.sh
   ```

2. **In the VM** (after Ubuntu Live boots):
   - Open terminal
   - Mount shared directory:
     ```bash
     sudo mkdir -p /mnt/vexfs_host
     sudo mount -t 9p -o trans=virtio,version=9p2000.L vexfs_host /mnt/vexfs_host
     ```
   - Run tests:
     ```bash
     bash /mnt/vexfs_host/tests/vm_testing/vm_test_script.sh
     ```

## Test Scenarios

The VM test script implements 6 comprehensive test scenarios:

### 1. **Module Information Test**
- Validates module metadata using `modinfo`
- Checks module structure and dependencies

### 2. **Basic Load/Unload Test**
- Tests `insmod` and `rmmod` operations
- Monitors kernel messages via `dmesg`
- Verifies module appears in `lsmod`

### 3. **Module Status Verification**
- Checks `/proc/modules` entries
- Validates `/sys/module/vexfs` directory structure
- Confirms proper kernel integration

### 4. **Unload Verification**
- Ensures clean module removal
- Verifies no residual kernel state
- Monitors unload messages

### 5. **Stress Testing**
- Performs 5 load/unload cycles
- Tests module stability under repeated operations
- Validates consistent behavior

### 6. **Final Status Check**
- Confirms clean system state
- Reviews all kernel messages
- Validates test completion

## Files Overview

### Main Scripts

- **`run_vm_tests.sh`**: Main VM launcher script
  - Downloads Ubuntu Live ISO if needed
  - Configures QEMU with shared directory
  - Starts VM with proper virtfs support

- **`vm_test_script.sh`**: In-VM testing script
  - Comprehensive kernel module testing
  - Real-time monitoring and logging
  - Safety checks and error handling

### Legacy Scripts (Reference)

- **`run_comprehensive_kernel_tests.sh`**: Original comprehensive test suite
- **`setup_vm_environment.sh`**: VM environment setup utilities
- **`create_test_vm.sh`**: VM creation utilities

## Safety Features

### VM Isolation
- All testing performed in isolated VM environment
- No risk to host system kernel
- Safe module loading/unloading testing

### Error Handling
- Comprehensive error detection
- Graceful failure handling
- Detailed logging and reporting

### Monitoring
- Real-time kernel message monitoring
- Module status verification
- System resource tracking

## Expected Output

### Successful Test Run
```
========================================
VexFS Kernel Module Testing in VM
========================================

[SUCCESS] Running in QEMU VM environment
[SUCCESS] Shared directory mounted at /mnt/vexfs_host
[SUCCESS] Found VexFS kernel module: -rw-r--r-- 1 root root 3.6M vexfs.ko

Test 1: Module Information
-------------------------
filename:       /tmp/vexfs.ko
license:        GPL v2
description:    VexFS - Vector-Enhanced Filesystem
author:         VexFS Development Team
...

[SUCCESS] VexFS module loaded successfully
[SUCCESS] Module appears in lsmod
[SUCCESS] All VexFS kernel module tests completed!
```

### Error Scenarios
- Module loading failures with detailed error messages
- Kernel compatibility issues
- Missing dependencies or permissions

## Troubleshooting

### Common Issues

1. **QEMU not found**:
   ```bash
   sudo apt install qemu-system-x86_64
   ```

2. **Kernel module not built**:
   ```bash
   cd /path/to/vexfs
   make
   ```

3. **Shared directory mount fails**:
   - Ensure VM started with virtfs support
   - Check QEMU command line includes `-virtfs` option

4. **Module loading fails**:
   - Check kernel version compatibility
   - Verify module was built for correct kernel
   - Review `dmesg` output for specific errors

### Debug Mode

For detailed debugging, monitor kernel messages in real-time:
```bash
# In VM terminal
sudo dmesg -w
```

## Performance Notes

- **VM Memory**: 2GB allocated (configurable in `run_vm_tests.sh`)
- **VM CPUs**: 2 cores allocated (configurable)
- **ISO Download**: ~4GB Ubuntu Live ISO (one-time download)
- **Test Duration**: ~2-5 minutes for complete test suite

## Integration with CI/CD

The VM testing approach is designed to be CI/CD friendly:

- Automated ISO download and caching
- Headless operation support (modify QEMU display options)
- Structured logging for automated parsing
- Exit codes for success/failure detection

## Next Steps

After successful VM testing:

1. **Production Testing**: Test on actual hardware
2. **Performance Benchmarking**: Measure filesystem performance
3. **Integration Testing**: Test with real workloads
4. **Stress Testing**: Extended load testing scenarios

## References

- [VM_KERNEL_MODULE_TESTING_SCENARIOS.md](../legacy/vm_management/VM_KERNEL_MODULE_TESTING_SCENARIOS.md)
- [VexFS Architecture Documentation](../../docs/architecture/)
- [Kernel Module Build Guide](../../README.md)