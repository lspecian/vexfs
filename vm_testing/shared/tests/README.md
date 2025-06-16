# VexFS Disk Persistence Verification Test Suite

**Task 33.1: Create Mandatory Verification Test Suite**

This directory contains comprehensive test scripts to verify that the VexFS Phase 1 implementation provides true disk persistence. These tests are **mandatory** before proceeding to Phase 2 development.

## Overview

The test suite verifies that VexFS:
- Provides real disk-backed storage (not just in-memory)
- Maintains data integrity across unmount/remount cycles
- Persists data through module reload cycles (simulating system reboot)
- Handles various file sizes and directory structures
- Integrates properly with the Linux VFS layer

## Test Scripts

### 1. `disk_persistence_verification.sh`
**Primary verification test** - Tests core disk persistence functionality.

**Features:**
- Creates loop device with VexFS filesystem
- Tests files of various sizes (4KB, 1MB, 100MB, 1GB)
- Verifies data integrity with SHA-256 checksums
- Tests unmount/remount cycles
- Tests directory structure persistence
- Generates detailed verification report

**Usage:**
```bash
sudo ./disk_persistence_verification.sh
```

**Requirements:**
- VexFS kernel module compiled (`vexfs.ko`)
- Root privileges or sudo access
- 1.5GB free disk space in `/tmp`

### 2. `reboot_simulation_test.sh`
**System reboot simulation** - Tests persistence across module reload cycles.

**Features:**
- Creates test files on mounted VexFS
- Simulates system reboot by unloading/reloading kernel module
- Verifies all data persists after module reload
- Tests various file types (text, binary, large files)
- Generates reboot simulation report

**Usage:**
```bash
sudo ./reboot_simulation_test.sh
```

**Requirements:**
- Completed disk persistence test (uses same loop file)
- Ability to unload/reload kernel modules

### 3. `comprehensive_test_runner.sh`
**Complete test orchestrator** - Runs all tests and generates comprehensive report.

**Features:**
- Orchestrates all verification tests
- Runs basic filesystem operations tests
- Integrates with fstests (if available)
- Performs stress testing
- Generates comprehensive verification report
- Tracks test results and success rates

**Usage:**
```bash
sudo ./comprehensive_test_runner.sh
```

**Requirements:**
- All individual test script requirements
- Optional: fstests installed at `/opt/xfstests`

## Test Methodology

### Disk Persistence Verification
1. **Setup:** Create 1.5GB loop device formatted with VexFS
2. **Create:** Generate test files with known content and checksums
3. **Unmount:** Cleanly unmount the filesystem
4. **Remount:** Mount the filesystem again
5. **Verify:** Check all files exist with correct content (SHA-256 verification)
6. **Repeat:** Multiple unmount/remount cycles

### Reboot Simulation
1. **Create:** Test files on mounted VexFS
2. **Save State:** Record checksums of all files
3. **Simulate Reboot:** Unmount → Detach loop device → Unload module → Reload module → Reattach → Remount
4. **Verify:** All files exist with correct content after "reboot"

### File Size Testing
- **4KB files:** Test small file handling
- **1MB files:** Test medium file handling  
- **100MB files:** Test large file handling
- **1GB files:** Test very large file handling

### Directory Structure Testing
- Root directory files
- Nested directory structures
- Deep directory hierarchies
- Multiple files per directory

## Expected Results

### Success Criteria
All tests must pass with the following results:
- ✅ All test files persist across unmount/remount cycles
- ✅ SHA-256 checksums match before and after persistence tests
- ✅ Directory structures remain intact
- ✅ Module reload cycles preserve all data
- ✅ No data corruption or loss

### Failure Indicators
If any of these occur, Phase 1 implementation needs fixes:
- ❌ Files missing after unmount/remount
- ❌ File content corruption (checksum mismatch)
- ❌ Directory structure loss
- ❌ Module reload causes data loss
- ❌ Mount/unmount failures

## Running the Tests

### Quick Start
```bash
# 1. Ensure VexFS module is compiled
cd kernel_module
make

# 2. Run comprehensive verification
cd tests
sudo ./comprehensive_test_runner.sh
```

### Individual Tests
```bash
# Run only disk persistence test
sudo ./disk_persistence_verification.sh

# Run only reboot simulation
sudo ./reboot_simulation_test.sh
```

### Prerequisites
- Linux system with kernel headers
- VexFS kernel module compiled
- Root/sudo access
- Required tools: `losetup`, `dd`, `sha256sum`, `gcc`
- At least 2GB free space in `/tmp`

## Test Reports

Each test generates detailed reports:

- **`/tmp/vexfs_persistence_test/results/persistence_verification_report.md`**
- **`/tmp/vexfs_reboot_test/results/reboot_simulation_report.md`**
- **`/tmp/vexfs_comprehensive_test/results/comprehensive_verification_report.md`**

Reports include:
- Test execution logs
- Success/failure status for each test
- Performance metrics
- System information
- Detailed command outputs
- Recommendations for next steps

## Integration with fstests

The comprehensive test runner can integrate with the Linux filesystem test suite (fstests/xfstests) if available:

```bash
# Install fstests (optional)
git clone https://git.kernel.org/pub/scm/fs/xfs/xfstests-dev.git /opt/xfstests
cd /opt/xfstests
make

# Run comprehensive tests with fstests integration
sudo ./comprehensive_test_runner.sh
```

## Troubleshooting

### Common Issues

**Module not loaded:**
```bash
sudo insmod ../vexfs.ko
```

**Permission denied:**
```bash
# Ensure running with sudo
sudo ./test_script.sh
```

**Loop device busy:**
```bash
# Check and detach existing loop devices
sudo losetup -a
sudo losetup -d /dev/loopX
```

**Insufficient disk space:**
```bash
# Check available space
df -h /tmp
# Clean up old test files
sudo rm -rf /tmp/vexfs_*_test
```

### Debug Mode
Add debug output to any test script:
```bash
# Enable debug mode
export VEXFS_TEST_DEBUG=1
sudo ./test_script.sh
```

## Critical Requirements

⚠️ **MANDATORY VERIFICATION** ⚠️

These tests are **mandatory** before proceeding to Phase 2 development. The tests verify that:

1. **VexFS provides REAL disk persistence** (not just in-memory storage)
2. **Data integrity is maintained** across all filesystem operations
3. **The implementation is stable** and ready for advanced features

**DO NOT PROCEED TO PHASE 2** unless all tests pass with documented evidence.

## Test Results Interpretation

### ✅ ALL TESTS PASS
- Phase 1 implementation verified
- Disk persistence confirmed
- Ready for Phase 2 development

### ❌ ANY TESTS FAIL
- Phase 1 implementation needs fixes
- Identify and resolve root causes
- Re-run tests until all pass
- **DO NOT** proceed to Phase 2

## Support

For issues with the test suite:
1. Check the detailed test logs
2. Verify all prerequisites are met
3. Ensure VexFS module compiles and loads correctly
4. Check system resources (disk space, memory)
5. Review kernel logs: `dmesg | grep vexfs`

---

**Task 33.1 Status:** ✅ COMPLETED - Mandatory verification test suite created and ready for execution.