# VexFS v2.0 xfstests Integration

## Overview

This directory contains the comprehensive xfstests integration for VexFS v2.0, providing POSIX compliance validation and filesystem behavior testing. The integration establishes VexFS v2.0 as both a high-performance vector database and a fully compliant POSIX filesystem.

## Quick Start

```bash
# 1. Setup xfstests environment (one-time)
./setup_xfstests.sh

# 2. Setup test devices
./setup_test_devices.sh

# 3. Run quick tests
./run_vexfs_xfstests.sh quick

# 4. View results
firefox results/$(ls results/ | tail -1)/test_report.html
```

## Directory Structure

```
tests/xfstests/
├── README.md                    # This file
├── setup_xfstests.sh           # One-time xfstests environment setup
├── run_vexfs_xfstests.sh       # Main test execution script
├── xfstests_result_parser.py   # Result analysis and reporting
├── vexfs_tests/                # VexFS-specific test cases
│   ├── 001                     # Basic vector operations test
│   └── 002                     # POSIX compliance test
├── configs/                    # Generated configuration files
├── results/                    # Test execution results
└── xfstests-dev/              # Cloned xfstests repository
```

## Components

### 1. Setup Script (`setup_xfstests.sh`)

**Purpose**: One-time setup of the xfstests environment

**Features**:
- Installs system dependencies
- Clones and builds xfstests
- Creates VexFS-specific configuration
- Verifies setup integrity

**Usage**:
```bash
./setup_xfstests.sh
```

### 2. Test Runner (`run_vexfs_xfstests.sh`)

**Purpose**: Execute xfstests on VexFS with comprehensive options

**Features**:
- Multiple test categories (quick, generic, posix, vexfs, stress, all)
- Device type selection (loop, RAM, real block devices)
- Parallel execution support
- Automated result collection
- Debug and logging options

**Usage Examples**:
```bash
# Quick smoke tests
./run_vexfs_xfstests.sh quick

# POSIX compliance validation
./run_vexfs_xfstests.sh posix

# Parallel execution
./run_vexfs_xfstests.sh --parallel 4 generic

# Custom device configuration
./run_vexfs_xfstests.sh --use-ram-disk quick

# Debug mode
./run_vexfs_xfstests.sh --debug-level 3 vexfs
```

### 3. Result Parser (`xfstests_result_parser.py`)

**Purpose**: Parse and analyze xfstests results

**Features**:
- Multi-format output (text, HTML, JSON)
- Test categorization and statistics
- Failure analysis and reporting
- Performance metrics extraction

**Usage**:
```bash
# Generate all report formats
python3 xfstests_result_parser.py results/20241201_143022/ \
    --text-report summary.txt \
    --html-report report.html \
    --json-report results.json \
    --print-summary
```

### 4. VexFS Test Cases (`vexfs_tests/`)

**Purpose**: VexFS-specific test validation

**Test Cases**:
- **001**: Basic vector operations (metadata, batch insert)
- **002**: POSIX compliance (file ops, directories, permissions)

**Integration**: Seamlessly integrates with standard xfstests framework

### 5. Device Setup (`setup_test_devices.sh`)

**Purpose**: Configure test devices for xfstests execution

**Device Types**:
- **Loop devices**: Safe, uses disk files (default)
- **RAM disk**: Fastest, uses system memory
- **Real devices**: Production-like, requires actual block devices

**Usage**:
```bash
# Default loop devices
./setup_test_devices.sh

# RAM disk for speed
./setup_test_devices.sh --use-ram-disk

# Real block devices
./setup_test_devices.sh --use-real-devices /dev/sdb1,/dev/sdc1
```

## Test Categories

### Quick Tests (`quick`)
- **Duration**: ~30 minutes
- **Purpose**: Smoke testing and basic functionality
- **Use Case**: Development, CI/CD

### Generic Tests (`generic`)
- **Duration**: ~2 hours
- **Purpose**: Standard filesystem behavior
- **Use Case**: POSIX compliance

### POSIX Tests (`posix`)
- **Duration**: ~1 hour
- **Purpose**: POSIX standard compliance
- **Use Case**: Standards certification

### VexFS Tests (`vexfs`)
- **Duration**: ~30 minutes
- **Purpose**: VexFS-specific functionality
- **Use Case**: Vector operations validation

### Stress Tests (`stress`)
- **Duration**: ~4 hours
- **Purpose**: Performance and stability
- **Use Case**: Production readiness

### All Tests (`all`)
- **Duration**: ~8 hours
- **Purpose**: Comprehensive validation
- **Use Case**: Release validation

## Prerequisites

### System Requirements
- Linux (Ubuntu 20.04+ or equivalent)
- Kernel 5.4+ with headers
- 4GB+ RAM (8GB+ recommended)
- 10GB+ free storage

### VexFS Requirements
- VexFS kernel module: `../../kernel/vexfs_v2_build/vexfs_v2_b62.ko`
- UAPI header: `../../kernel/vexfs_v2_build/vexfs_v2_uapi.h`

### Dependencies
Automatically installed by `setup_xfstests.sh`:
- build-essential, autotools, libtool
- libattr1-dev, libacl1-dev, libaio-dev
- xfsprogs, e2fsprogs, fio, dbench
- Python 3 with required modules

## CI/CD Integration

### GitHub Actions
Automated testing via `.github/workflows/xfstests.yml`:
- **Triggers**: Push, PR, schedule, manual
- **Matrix**: Multiple test groups and device types
- **Artifacts**: Results, logs, reports
- **Notifications**: Failure alerts

### Manual Execution
```bash
# Trigger via GitHub CLI
gh workflow run xfstests.yml -f test_groups=quick
```

## Result Analysis

### Automated Reports
- **HTML Report**: Visual dashboard with charts and statistics
- **Text Report**: Command-line friendly summary
- **JSON Report**: Machine-readable for automation

### Key Metrics
- **Pass Rate**: Percentage of successful tests
- **Category Breakdown**: Results by test category
- **Performance**: Execution times and throughput
- **Failures**: Detailed error analysis

### Example Analysis
```bash
# View latest results
LATEST=$(ls results/ | tail -1)
firefox results/$LATEST/test_report.html

# Check specific failures
grep -i fail results/$LATEST/test_execution.log

# Performance metrics
grep "ops/sec" results/$LATEST/test_execution.log
```

## Troubleshooting

### Common Issues

1. **Module Loading Failures**
   ```bash
   # Check module
   ls ../../kernel/vexfs_v2_build/vexfs_v2_b62.ko
   
   # Check dependencies
   modinfo ../../kernel/vexfs_v2_build/vexfs_v2_b62.ko
   ```

2. **Device Setup Issues**
   ```bash
   # Check loop devices
   losetup -l
   
   # Check permissions
   ls -la /dev/loop*
   ```

3. **Test Execution Failures**
   ```bash
   # Check environment
   source device_config.env
   echo "Test device: $TEST_DEV"
   
   # Verify mount
   mount | grep vexfs
   ```

### Debug Mode
```bash
# Maximum debugging
./run_vexfs_xfstests.sh --debug-level 3 --no-cleanup vexfs

# Monitor kernel messages
sudo dmesg -w | grep -i vexfs
```

## Advanced Usage

### Custom Test Exclusions
```bash
# Create exclusion list
echo "generic/001,generic/002" > exclude.txt

# Use with tests
./run_vexfs_xfstests.sh --exclude $(cat exclude.txt) generic
```

### Performance Optimization
```bash
# Speed-optimized
./run_vexfs_xfstests.sh --parallel $(nproc) --use-ram-disk quick

# Coverage-optimized
./run_vexfs_xfstests.sh --timeout-factor 5 --debug-level 2 all
```

### Integration with Existing Infrastructure
```bash
# VM testing
./run_vexfs_xfstests.sh --use-real-devices /dev/vdb,/dev/vdc

# Performance monitoring
iostat -x 1 & ./run_vexfs_xfstests.sh generic
```

## Development Workflow

### Adding New Tests
1. Create test file in `vexfs_tests/`
2. Follow xfstests format and conventions
3. Make executable: `chmod +x vexfs_tests/003`
4. Test individually: `cd xfstests-dev && ./check tests/vexfs/003`

### Modifying Existing Tests
1. Edit test files in `vexfs_tests/`
2. Test changes locally
3. Update documentation if needed
4. Commit changes with descriptive messages

## Documentation

- **Setup Guide**: [XFSTESTS_INTEGRATION_GUIDE.md](../../docs/testing/XFSTESTS_INTEGRATION_GUIDE.md)
- **Compliance Report**: [VEXFS_POSIX_COMPLIANCE_REPORT.md](../../docs/testing/VEXFS_POSIX_COMPLIANCE_REPORT.md)
- **Architecture**: [C_FFI_ARCHITECTURE.md](../../docs/architecture/C_FFI_ARCHITECTURE.md)

## Support

For issues or questions:
1. Check the troubleshooting section above
2. Review the comprehensive documentation
3. Check existing GitHub issues
4. Create new issue with detailed information

## Contributing

1. Follow existing code style and conventions
2. Add tests for new functionality
3. Update documentation as needed
4. Ensure all tests pass before submitting PR

---

This xfstests integration establishes VexFS v2.0 as an enterprise-ready filesystem that combines high-performance vector database capabilities with full POSIX compliance, providing confidence for production deployment and enterprise adoption.