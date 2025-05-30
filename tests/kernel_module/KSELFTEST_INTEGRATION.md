# VexFS Kernel Module kselftest Integration

This document describes the kselftest integration for VexFS kernel module testing, implementing the recommendations from the consultancy report for standardized kernel testing compliance.

## Overview

The kselftest integration provides a standardized interface for VexFS kernel module testing that integrates with the Linux kernel's kselftest framework. It produces TAP (Test Anything Protocol) version 13 compliant output and follows kernel testing best practices.

## Architecture

### Components

1. **KselftestRunner** (`src/kselftest_integration.rs`)
   - Core kselftest framework implementation
   - TAP-compliant output generation
   - Test case management and execution
   - Integration with existing Level1TestRunner

2. **kselftest_runner Binary** (`src/bin/kselftest_runner.rs`)
   - Command-line interface for kselftest execution
   - Compatible with kernel kselftest harness
   - Supports standard kselftest options and exit codes

3. **Level1TestRunner Integration**
   - Leverages existing comprehensive test logic
   - Maintains sophisticated resource monitoring
   - Preserves kernel health checking capabilities

## Test Cases

The kselftest integration includes 7 standardized test cases:

1. **module_compilation** - Verify VexFS kernel module compiles successfully
2. **module_info_validation** - Validate module metadata using modinfo
3. **module_loading** - Test kernel module loading with insmod (requires sudo)
4. **module_listing** - Verify module appears in lsmod output
5. **module_unloading** - Test kernel module unloading with rmmod (requires sudo)
6. **resource_leak_detection** - Monitor for resource leaks after module operations
7. **kernel_health_check** - Check for kernel panics, oopses, and warnings

## TAP Output Format

The kselftest runner produces TAP version 13 compliant output:

```
TAP version 13
1..7
# VexFS Kernel Module kselftest Suite
# Test: vexfs_kernel_module
# Total test cases: 7
#
# Running test: module_compilation
# Description: Verify VexFS kernel module compiles successfully
ok 1 - module_compilation PASS
# Running test: module_info_validation
# Description: Validate module metadata using modinfo
ok 2 - module_info_validation PASS
# Running test: module_loading
# Description: Test kernel module loading with insmod
ok 3 - module_loading # SKIP requires sudo
...
```

## Usage

### Basic Usage

```bash
# Run all tests with TAP output
./target/debug/kselftest_runner

# Run tests without sudo requirements
./target/debug/kselftest_runner --no-sudo

# Run with verbose output for debugging
./target/debug/kselftest_runner --verbose
```

### Command Line Options

- `--verbose, -v` - Enable verbose output
- `--tap` - Force TAP output format (default)
- `--no-tap` - Disable TAP output format
- `--no-sudo` - Skip tests requiring sudo privileges
- `--help, -h` - Show help message

### Exit Codes

- `0` - All tests passed
- `1` - One or more tests failed
- `4` - Tests were skipped

## Integration with Kernel kselftest

### Directory Structure

For integration with kernel kselftest infrastructure:

```
tools/testing/selftests/vexfs/
├── Makefile
├── kselftest_runner
└── config
```

### Makefile Integration

```makefile
# tools/testing/selftests/vexfs/Makefile
TEST_PROGS := kselftest_runner
include ../lib.mk
```

### Running via kselftest

```bash
# Run VexFS tests via kselftest
cd tools/testing/selftests
make TARGETS=vexfs run_tests

# Run specific VexFS test
./vexfs/kselftest_runner
```

## Implementation Details

### KselftestRunner Structure

```rust
pub struct KselftestRunner {
    test_cases: Vec<KselftestCase>,
    verbose: bool,
    tap_output: bool,
    level1_runner: Level1TestRunner,
}
```

### Test Case Definition

```rust
pub struct KselftestCase {
    pub name: String,
    pub description: String,
    pub requires_sudo: bool,
    pub timeout_seconds: u64,
    pub setup_commands: Vec<String>,
    pub cleanup_commands: Vec<String>,
}
```

### Integration with Level1TestRunner

The kselftest integration leverages the existing Level1TestRunner for actual test execution:

1. **Compilation Testing** - Uses `test_module_compilation()`
2. **Module Operations** - Uses `test_module_loading()`, `test_module_unloading()`
3. **Resource Monitoring** - Uses `test_resource_leak_detection()`
4. **Kernel Health** - Uses `test_kernel_health_check()`

## Benefits

### Standardization
- TAP version 13 compliance
- Standard kselftest exit codes
- Consistent test naming and output format

### Integration
- Compatible with kernel CI systems
- Works with kselftest harness
- Supports automated test execution

### Flexibility
- Configurable test execution (sudo/no-sudo)
- Verbose and quiet modes
- Individual test case control

## Future Enhancements

### Planned Features
1. **Test Filtering** - Run specific test cases by name
2. **Parallel Execution** - Run independent tests concurrently
3. **Extended Reporting** - JSON output format for CI integration
4. **Custom Test Cases** - Plugin architecture for additional tests

### Integration Opportunities
1. **VM Testing** - Integration with Level 2 VM-based tests
2. **Stress Testing** - Integration with Level 3 stress tests
3. **Performance Metrics** - Integration with performance benchmarks

## Troubleshooting

### Common Issues

1. **Compilation Failures**
   - Ensure kernel headers are installed
   - Verify build environment setup
   - Check for missing dependencies

2. **Permission Errors**
   - Use `--no-sudo` for non-privileged testing
   - Ensure proper sudo configuration
   - Check module loading permissions

3. **TAP Output Issues**
   - Verify TAP version 13 compliance
   - Check for proper test plan format
   - Ensure consistent result reporting

### Debug Mode

Enable verbose output for detailed debugging:

```bash
./target/debug/kselftest_runner --verbose --no-sudo
```

## Compliance

### Kernel Testing Standards
- Follows Linux kernel kselftest conventions
- Implements TAP version 13 protocol
- Uses standard exit codes and output format

### Consultancy Recommendations
- Implements Basic Level Testing Framework
- Provides standardized test case management
- Enables integration with kernel testing infrastructure
- Supports automated CI/CD workflows

## References

- [Linux Kernel kselftest Documentation](https://www.kernel.org/doc/html/latest/dev-tools/kselftest.html)
- [TAP Version 13 Specification](https://testanything.org/tap-version-13-specification.html)
- [VexFS Testing Strategy Consultancy Report](../../docs/testing/)
- [Level 1 Basic Validation Framework](src/level1_basic_validation.rs)