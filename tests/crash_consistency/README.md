# VexFS Crash Consistency Testing Framework

This directory contains a comprehensive crash consistency testing framework for VexFS, designed to ensure data integrity and filesystem reliability after unexpected system failures, power outages, and kernel crashes.

## Overview

The crash consistency testing framework provides:

- **Crash Simulation**: Controlled simulation of various crash scenarios
- **Data Integrity Verification**: Comprehensive filesystem consistency checking
- **Recovery Testing**: Automated recovery procedures and validation
- **Vector Consistency**: VexFS-specific vector index integrity testing
- **Automated Reporting**: Detailed test results and analysis

## Directory Structure

```
tests/crash_consistency/
├── README.md                          # This documentation
├── crash_simulation/                  # Crash injection framework
│   └── crash_injector.py             # Main crash simulation tool
├── data_integrity/                    # Filesystem consistency checking
│   └── vexfs_fsck.py                 # VexFS filesystem checker
├── recovery_testing/                  # Recovery validation framework
│   └── recovery_validator.py         # Recovery testing tool
├── vector_consistency/                # Vector-specific integrity tests
├── tools/                            # Additional testing utilities
├── configs/                          # Test configuration files
├── scripts/                          # Test execution scripts
│   └── run_crash_consistency_tests.sh # Main test runner
└── results/                          # Test results and reports
```

## Quick Start

### Prerequisites

1. **Root Access**: Crash consistency testing requires root privileges for device operations
2. **VexFS Module**: VexFS kernel module must be available and loadable
3. **Test Device**: A block device for testing (can be a loop device)
4. **Python 3**: Required for test scripts
5. **Dependencies**: Standard Linux utilities (mount, umount, mkfs, etc.)

### Basic Usage

```bash
# Run basic crash consistency tests on a loop device
sudo ./scripts/run_crash_consistency_tests.sh --device /dev/loop0 --suite basic

# Run comprehensive tests on a real device
sudo ./scripts/run_crash_consistency_tests.sh --device /dev/sdb1 --suite full

# Run in VM mode with additional crash mechanisms
sudo ./scripts/run_crash_consistency_tests.sh --device /dev/vdb --suite stress --vm-mode
```

### Setup Only

```bash
# Setup test environment without running tests
sudo ./scripts/run_crash_consistency_tests.sh --setup-only

# Cleanup test environment
sudo ./scripts/run_crash_consistency_tests.sh --cleanup-only
```

## Test Components

### 1. Crash Simulation (`crash_simulation/`)

The crash simulation framework provides controlled crash injection capabilities:

#### Supported Crash Types

- **Power Failure**: Simulates sudden power loss during filesystem operations
- **Kernel Panic**: Simulates kernel panic scenarios
- **I/O Errors**: Injects I/O errors using dm-flakey or similar mechanisms
- **Memory Corruption**: Simulates memory pressure and corruption
- **Force Unmount**: Simulates forced filesystem unmounting

#### Usage Example

```python
from crash_simulation.crash_injector import CrashInjector, CrashScenario, CrashType, CrashTiming

# Create crash injector
injector = CrashInjector(vm_config={"memory": "2G"})

# Define crash scenario
scenario = CrashScenario(
    crash_type=CrashType.POWER_FAILURE,
    timing=CrashTiming.DURING_OPERATION,
    operation="file_write",
    description="Power failure during file write"
)

# Execute crash test
result = injector.execute_crash_scenario(scenario)
```

### 2. Data Integrity Verification (`data_integrity/`)

The VexFS filesystem checker provides comprehensive integrity validation:

#### Checks Performed

- **Superblock Integrity**: Magic number, checksum, and field validation
- **Inode Table Consistency**: Inode structure and metadata validation
- **Directory Structure**: Directory tree and entry consistency
- **File Integrity**: File content and permission validation
- **Vector Indices**: VexFS-specific vector index integrity
- **Free Space Consistency**: Free space accounting validation
- **Journal Integrity**: Transaction log consistency (if present)

#### Usage Example

```bash
# Run filesystem check with automatic fixes
python3 data_integrity/vexfs_fsck.py /dev/sdb1 --fix --verbose

# Check specific mount point
python3 data_integrity/vexfs_fsck.py /dev/sdb1 --mount-point /mnt/vexfs
```

### 3. Recovery Testing (`recovery_testing/`)

The recovery validation framework tests filesystem recovery capabilities:

#### Recovery Scenarios

- **Mount Recovery**: Automatic filesystem mounting after crashes
- **Fsck Recovery**: Filesystem repair and recovery procedures
- **Data Validation**: Post-recovery data integrity verification
- **Performance Impact**: Recovery time and performance measurement

#### Usage Example

```python
from recovery_testing.recovery_validator import RecoveryValidator, RecoveryTest

# Create recovery validator
validator = RecoveryValidator("/dev/sdb1", "/mnt/recovery_test")

# Define recovery test
test = RecoveryTest(
    test_id="recovery_001",
    description="Power failure during file creation",
    pre_crash_operations=["create_files"],
    crash_type="power_failure",
    expected_recovery_time=5.0,
    validation_checks=["file_integrity", "checksum_validation"]
)

# Execute recovery test
result = validator.execute_recovery_test(test)
```

### 4. Vector Consistency (`vector_consistency/`)

VexFS-specific vector integrity testing (implementation in progress):

- **Vector Index Validation**: HNSW and other index structure integrity
- **Vector Data Consistency**: Vector content and metadata validation
- **Search Result Verification**: Vector search accuracy after recovery
- **Index Reconstruction**: Automatic index rebuilding capabilities

## Test Suites

### Basic Suite
- Essential crash scenarios (power failure, kernel panic)
- Basic file operations (create, delete, modify)
- Standard recovery procedures
- Fundamental integrity checks

### Stress Suite
- High-frequency crash injection
- Concurrent operations during crashes
- Complex recovery scenarios
- Extensive data validation

### Full Suite
- All crash types and scenarios
- Comprehensive operation coverage
- Long-running stability tests
- Complete integrity validation

## Configuration

### Environment Variables

```bash
export TEST_DEVICE="/dev/loop0"        # Test device path
export MOUNT_POINT="/mnt/vexfs_test"   # Mount point for testing
export VM_MODE="true"                  # Enable VM-specific features
export VERBOSE="true"                  # Enable verbose output
export TEST_SUITE="basic"              # Default test suite
```

### Test Configuration Files

Configuration files in `configs/` directory:

- `crash_scenarios.json`: Predefined crash test scenarios
- `recovery_tests.json`: Recovery test configurations
- `fsck_config.json`: Filesystem checker settings

## Results and Reporting

### Output Files

Test results are saved in the `results/` directory:

- `crash_test_results_*.json`: Crash simulation results
- `recovery_test_results_*.json`: Recovery test results
- `fsck_results_*.json`: Filesystem consistency check results
- `crash_consistency_report_*.html`: Comprehensive HTML reports
- `test_summary.json`: Overall test summary

### Report Format

HTML reports include:

- **Test Overview**: Configuration and execution summary
- **Crash Simulation Results**: Success/failure rates and timing
- **Recovery Analysis**: Recovery times and success rates
- **Integrity Assessment**: Data corruption and consistency issues
- **Recommendations**: Suggested improvements and fixes

## Advanced Usage

### Custom Crash Scenarios

Create custom crash scenarios by extending the framework:

```python
# Define custom crash scenario
custom_scenario = CrashScenario(
    crash_type=CrashType.POWER_FAILURE,
    timing=CrashTiming.RANDOM_TIMING,
    operation="vector_insert",
    delay_ms=100,
    repeat_count=5,
    description="Repeated power failures during vector operations"
)
```

### Integration with CI/CD

Integrate crash consistency testing into CI/CD pipelines:

```yaml
# GitHub Actions example
- name: Run Crash Consistency Tests
  run: |
    sudo ./tests/crash_consistency/scripts/run_crash_consistency_tests.sh \
      --device /dev/loop0 \
      --suite basic \
      --verbose
```

### VM-Based Testing

For comprehensive testing in virtual machines:

```bash
# Setup VM with additional crash mechanisms
sudo ./scripts/run_crash_consistency_tests.sh \
  --device /dev/vdb \
  --suite full \
  --vm-mode \
  --verbose
```

## Safety Considerations

### Data Safety

- **Never run on production systems**: Always use dedicated test devices
- **Backup important data**: Crash testing will destroy filesystem content
- **Use isolated environments**: Prefer VMs or dedicated test machines

### System Safety

- **Monitor system resources**: Crash testing can consume significant resources
- **Avoid nested virtualization**: May interfere with crash simulation
- **Check kernel compatibility**: Ensure crash mechanisms work with your kernel

## Troubleshooting

### Common Issues

1. **Permission Denied**
   ```bash
   # Ensure running as root
   sudo ./scripts/run_crash_consistency_tests.sh
   ```

2. **VexFS Module Not Found**
   ```bash
   # Load VexFS module
   sudo modprobe vexfs
   
   # Or build and install
   make && sudo make install
   ```

3. **Device Busy**
   ```bash
   # Check for existing mounts
   mount | grep /dev/sdb1
   
   # Force unmount if necessary
   sudo umount -f /dev/sdb1
   ```

4. **Loop Device Issues**
   ```bash
   # List loop devices
   losetup -a
   
   # Detach loop device
   sudo losetup -d /dev/loop0
   ```

### Debug Mode

Enable debug output for troubleshooting:

```bash
# Run with maximum verbosity
sudo ./scripts/run_crash_consistency_tests.sh \
  --device /dev/loop0 \
  --suite basic \
  --verbose
```

### Log Files

Check log files in `results/` directory:

- `crash_injection.log`: Crash simulation logs
- `recovery_testing.log`: Recovery test logs
- `fsck.log`: Filesystem checker logs

## Development

### Adding New Crash Types

1. Extend `CrashType` enum in `crash_injector.py`
2. Implement crash trigger method
3. Add validation and recovery procedures
4. Update test scenarios

### Custom Recovery Tests

1. Create new `RecoveryTest` configurations
2. Implement custom validation checks
3. Add to test suite definitions
4. Update documentation

### Vector-Specific Tests

1. Implement vector consistency checks
2. Add vector-specific crash scenarios
3. Create vector index validation tools
4. Integrate with main test framework

## Performance Considerations

### Resource Usage

- **Memory**: Crash testing may require significant memory
- **CPU**: Intensive I/O and computation during tests
- **Storage**: Multiple filesystem copies and logs
- **Time**: Full test suites can take hours to complete

### Optimization Tips

- Use SSDs for faster test execution
- Allocate sufficient RAM for VM testing
- Run tests during off-peak hours
- Use parallel test execution where possible

## Contributing

### Guidelines

1. **Test Coverage**: Ensure new features include crash consistency tests
2. **Documentation**: Update documentation for new test types
3. **Safety**: Verify tests don't compromise system safety
4. **Performance**: Consider impact on test execution time

### Code Style

- Follow Python PEP 8 for Python code
- Use clear, descriptive variable names
- Include comprehensive error handling
- Add logging for debugging

## References

- [VexFS Architecture Documentation](../../docs/architecture/)
- [Filesystem Testing Best Practices](../../docs/testing/)
- [Linux Filesystem Development Guide](https://www.kernel.org/doc/html/latest/filesystems/)
- [Crash Consistency Research Papers](https://research.cs.wisc.edu/adsl/Publications/)

## License

This crash consistency testing framework is part of the VexFS project and is licensed under the same terms as VexFS. See the main project LICENSE file for details.