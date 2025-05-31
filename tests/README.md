# VexFS Testing Framework

## Overview

The VexFS testing framework provides comprehensive validation of the VexFS kernel module through a sophisticated three-level testing architecture. This framework ensures reliability, performance, and stability across all deployment scenarios.

## Quick Start

### Prerequisites

```bash
# Install required packages
sudo apt-get update
sudo apt-get install -y \
    linux-headers-$(uname -r) \
    build-essential \
    qemu-system-x86_64 \
    qemu-utils \
    jq \
    bc

# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
```

### Running Tests

#### Option 1: Shell Script (Recommended)
```bash
# Run complete test suite
./tests/vm_testing/run_complete_test_suite.sh

# Quick validation (15 minutes)
./tests/vm_testing/run_complete_test_suite.sh quick

# Individual test levels
./tests/vm_testing/run_complete_test_suite.sh level1  # Basic validation
./tests/vm_testing/run_complete_test_suite.sh level2  # VM mount operations
./tests/vm_testing/run_complete_test_suite.sh level3  # Stress testing
```

#### Option 2: Unified Test Runner
```bash
cd tests/kernel_module

# Standard test suite
cargo run --release --bin unified_test_runner

# Quick validation
cargo run --release --bin unified_test_runner quick

# Full suite with extended stress testing
cargo run --release --bin unified_test_runner full --extended-stress

# Performance benchmarking
cargo run --release --bin unified_test_runner benchmark --baseline
```

#### Option 3: Individual Test Runners
```bash
cd tests/kernel_module

# Level 1: Basic validation
cargo run --release --bin kselftest_runner

# Level 2: VM mount operations
cargo run --release --bin mount_level_runner

# Level 3: Stress testing
cargo run --release --bin stress_test_runner quick
```

## Testing Architecture

### Three-Level Testing System

#### Level 1: Basic Validation (1-5 minutes)
- **Purpose**: Fundamental kernel module functionality
- **Environment**: Host system with kselftest framework
- **Tests**: Module loading, unloading, basic interface validation
- **Binary**: [`kselftest_runner`](kernel_module/src/bin/kselftest_runner.rs)

#### Level 2: VM Mount Operations (5-30 minutes)
- **Purpose**: Real-world filesystem operations in isolated environment
- **Environment**: QEMU VM with Ubuntu Live ISO
- **Tests**: Mount/unmount cycles, file operations, error handling
- **Binary**: [`mount_level_runner`](kernel_module/src/bin/mount_level_runner.rs)

#### Level 3: Ultimate Stress Testing (15 minutes - 24+ hours)
- **Purpose**: Extreme stress testing with comprehensive monitoring
- **Environment**: VM with advanced kernel instrumentation
- **Tests**: High-frequency operations, resource exhaustion, crash recovery
- **Binary**: [`stress_test_runner`](kernel_module/src/bin/stress_test_runner.rs)

### Unified Integration
- **Master Orchestrator**: [`unified_test_runner`](kernel_module/src/bin/unified_test_runner.rs)
- **Shell Integration**: [`run_complete_test_suite.sh`](vm_testing/run_complete_test_suite.sh)
- **Comprehensive Reporting**: JSON, HTML, and text formats
- **CI/CD Integration**: GitHub Actions workflow

## Test Results and Reporting

### Output Formats

#### JSON Results
```bash
# Unified results
unified_test_results/unified_test_results.json

# Individual level results
unified_test_results/level1/level1_results.json
unified_test_results/level2/level2_results.json
unified_test_results/level3/level3_results.json
```

#### HTML Reports
```bash
# Comprehensive HTML report
unified_test_results/reports/comprehensive_report.html

# Open in browser
xdg-open unified_test_results/reports/comprehensive_report.html
```

#### Summary Reports
```bash
# Text summary
unified_test_results/reports/summary_report.txt

# View summary
cat unified_test_results/reports/summary_report.txt
```

### Key Metrics

- **Overall Status**: Success/Failed/Warning
- **Test Duration**: Total execution time
- **Test Coverage**: Passed/Failed/Skipped counts per level
- **Crash Analysis**: Crash types, recovery rates, critical incidents
- **Performance Metrics**: Mount times, throughput, resource usage
- **Recommendations**: Automated suggestions for improvements

## Advanced Usage

### Custom Configuration

#### VM Configuration
```json
{
  "memory": "2048",
  "cpus": "2",
  "disk_size": "4G",
  "enable_kvm": true,
  "timeout_seconds": 3600
}
```

#### Test Configuration
```json
{
  "enable_level1": true,
  "enable_level2": true,
  "enable_level3": true,
  "continue_on_failure": false,
  "parallel_execution": false,
  "comprehensive_reporting": true,
  "crash_recovery_enabled": true,
  "performance_baseline_capture": true
}
```

### Command Line Options

#### Unified Test Runner
```bash
# Configuration options
--config CONFIG_FILE          # Custom configuration file
--output-dir DIR              # Output directory for results
--continue-on-failure         # Continue testing even if a level fails
--parallel                    # Run test levels in parallel (experimental)
--disable-level1              # Skip Level 1 basic validation tests
--disable-level2              # Skip Level 2 VM mount operation tests
--disable-level3              # Skip Level 3 stress testing
--verbose                     # Enable verbose output

# Subcommands
full                          # Run complete test suite with all levels
quick                         # Run quick validation suite
benchmark                     # Run performance benchmark suite
regression                    # Run regression testing against baseline
```

#### Shell Script Options
```bash
# Basic options
-h, --help                    # Show help message
-o, --output-dir DIR          # Output directory for results
-c, --config FILE             # Configuration file for test settings
-v, --verbose                 # Enable verbose output

# Test control
--continue-on-failure         # Continue testing even if a level fails
--parallel                    # Run test levels in parallel (experimental)
--disable-level1              # Skip Level 1 basic validation tests
--disable-level2              # Skip Level 2 VM mount operation tests
--disable-level3              # Skip Level 3 stress testing
--quick                       # Run quick test suite (reduced duration)

# Commands
full                          # Run complete test suite with all levels
quick                         # Run quick validation suite
benchmark                     # Run performance benchmark suite
regression                    # Run regression testing against baseline
level1                        # Run only Level 1 tests
level2                        # Run only Level 2 tests
level3                        # Run only Level 3 tests
clean                         # Clean up test artifacts and results
```

### Environment Variables

```bash
# Output configuration
export OUTPUT_DIR="/tmp/vexfs_test_results"

# Logging configuration
export RUST_LOG="debug"                    # Rust logging level
export VEXFS_TEST_CONFIG="custom.json"     # Default configuration file

# VM configuration
export VM_MEMORY="4096"                    # VM memory in MB
export VM_CPUS="4"                         # VM CPU count
export VM_TIMEOUT="7200"                   # VM timeout in seconds
```

## CI/CD Integration

### GitHub Actions

The testing framework includes comprehensive CI/CD integration:

- **Workflow File**: [`.github/workflows/kernel_module_testing.yml`](../.github/workflows/kernel_module_testing.yml)
- **Triggers**: Push, PR, scheduled, manual dispatch
- **Matrix Testing**: Multiple kernel versions and configurations
- **Artifact Collection**: Test results, logs, reports
- **Performance Regression Detection**: Automated baseline comparison

#### Workflow Jobs

1. **Build and Validate**: Compile and run basic validation
2. **VM Mount Testing**: Execute Level 2 tests in VM environment
3. **Stress Testing**: Run Level 3 stress tests with monitoring
4. **Unified Testing**: Execute complete test suite
5. **Performance Analysis**: Regression detection and baseline updates
6. **Cleanup and Notification**: Result aggregation and notifications

### Local CI Simulation

```bash
# Simulate CI environment locally
export CI=true
export GITHUB_ACTIONS=true

# Run tests as CI would
./tests/vm_testing/run_complete_test_suite.sh quick --verbose
```

## Troubleshooting

### Common Issues

#### 1. VM Boot Failures
```bash
# Check QEMU installation
qemu-system-x86_64 --version

# Verify KVM support
ls -la /dev/kvm

# Check available memory
free -h
```

#### 2. Module Load Failures
```bash
# Check kernel version compatibility
uname -r
modinfo kernel/vexfs.ko

# Verify module build
cd kernel && make clean && make

# Check kernel logs
dmesg | tail -20
```

#### 3. Test Timeouts
```bash
# Increase timeout values
export VM_TIMEOUT=7200

# Check system resources
htop
iotop
```

#### 4. Permission Issues
```bash
# Fix VM permissions
sudo chmod 666 /dev/kvm

# Fix file permissions
chmod +x tests/vm_testing/*.sh
```

### Debug Mode

#### Enable Verbose Logging
```bash
# Rust logging
export RUST_LOG=debug

# Shell script debugging
./tests/vm_testing/run_complete_test_suite.sh --verbose

# Individual runner debugging
cargo run --release --bin unified_test_runner -- --verbose
```

#### Collect Debug Information
```bash
# System information
uname -a
lscpu
free -h
df -h

# Kernel module information
lsmod | grep vexfs
modinfo kernel/vexfs.ko

# VM information
qemu-system-x86_64 --version
ls -la /dev/kvm
```

## Performance Optimization

### Test Execution Speed

#### Parallel Execution (Experimental)
```bash
# Enable parallel test execution
./tests/vm_testing/run_complete_test_suite.sh --parallel
```

#### VM Optimization
```bash
# Enable KVM acceleration
sudo chmod 666 /dev/kvm

# Increase VM resources
export VM_MEMORY=4096
export VM_CPUS=4
```

#### Build Optimization
```bash
# Use release builds
cargo build --release

# Enable link-time optimization
export RUSTFLAGS="-C lto=fat"
```

### Resource Management

#### Memory Usage
- Monitor memory usage during tests
- Adjust VM memory allocation based on available resources
- Use memory-efficient test patterns

#### CPU Usage
- Balance VM CPU allocation with host requirements
- Monitor CPU usage during stress testing
- Adjust concurrency levels based on available cores

#### Disk Usage
- Clean up test artifacts regularly
- Use temporary directories for test data
- Monitor disk space during extended testing

## Extending the Framework

### Adding New Test Scenarios

#### Level 1 Extension
```rust
// Add to tests/kernel_module/src/level1_basic_validation.rs
pub fn test_new_functionality() -> Result<(), TestError> {
    // Implement new basic validation test
    Ok(())
}
```

#### Level 2 Extension
```rust
// Add to tests/kernel_module/src/mount_test_suite.rs
pub fn test_new_mount_scenario() -> Result<(), TestError> {
    // Implement new mount operation test
    Ok(())
}
```

#### Level 3 Extension
```rust
// Add to tests/kernel_module/src/stress_testing_framework.rs
pub fn create_new_stress_pattern() -> OperationPattern {
    // Implement new stress testing pattern
    OperationPattern { /* ... */ }
}
```

### Custom Test Runners

#### Create New Binary
```rust
// tests/kernel_module/src/bin/custom_runner.rs
use kernel_module_tests::*;

fn main() {
    // Implement custom test logic
}
```

#### Update Cargo.toml
```toml
[[bin]]
name = "custom_runner"
path = "src/bin/custom_runner.rs"
```

### Integration with External Tools

#### Custom Monitoring
```rust
// Integrate with external monitoring systems
pub struct CustomMonitor {
    // Custom monitoring implementation
}

impl Monitor for CustomMonitor {
    fn start_monitoring(&mut self) -> Result<(), Error> {
        // Start custom monitoring
    }
}
```

#### Custom Reporting
```rust
// Implement custom report formats
pub struct CustomReporter {
    // Custom reporting implementation
}

impl Reporter for CustomReporter {
    fn generate_report(&self, results: &TestResults) -> Result<(), Error> {
        // Generate custom report format
    }
}
```

## Documentation

### Comprehensive Documentation

- **[Three-Level Testing Architecture](testing/THREE_LEVEL_TESTING_ARCHITECTURE.md)**: Complete architecture overview
- **[Crash Scenario Database](testing/CRASH_SCENARIO_DATABASE.md)**: Crash classification and recovery procedures
- **[VM Testing Strategy](testing/VM_TESTING_STRATEGY.md)**: VM-based testing approach
- **[Performance Benchmarking](../docs/architecture/REAL_WORLD_PERFORMANCE_BENCHMARKING_STRATEGY.md)**: Performance testing strategy

### API Documentation

```bash
# Generate Rust documentation
cd tests/kernel_module
cargo doc --open
```

### Code Examples

See the [`examples/`](../examples/) directory for:
- Basic usage examples
- Performance benchmarking examples
- Custom test implementations
- Integration examples

## Contributing

### Development Workflow

1. **Fork and Clone**: Fork the repository and clone locally
2. **Create Branch**: Create feature branch for changes
3. **Implement Changes**: Add new tests or improve existing ones
4. **Test Changes**: Run complete test suite to verify changes
5. **Submit PR**: Submit pull request with detailed description

### Testing Guidelines

- All new features must include comprehensive tests
- Tests must pass on all supported kernel versions
- Performance impact must be evaluated
- Documentation must be updated for new features

### Code Quality

```bash
# Format code
cargo fmt --all

# Check code quality
cargo clippy --all-targets --all-features -- -D warnings

# Run tests
cargo test --all
```

## Support

### Getting Help

- **Issues**: Report bugs and request features via GitHub issues
- **Discussions**: Ask questions in GitHub discussions
- **Documentation**: Check comprehensive documentation in [`docs/`](../docs/)

### Known Limitations

- Parallel execution is experimental and may have stability issues
- Extended stress testing (24+ hours) requires significant system resources
- Some VM features require KVM support for optimal performance
- Performance baselines are system-dependent

### Future Enhancements

- Enhanced parallel execution support
- Cloud-based testing infrastructure
- Machine learning-based failure prediction
- Real-world workload simulation
- Production environment testing capabilities

---

**The VexFS testing framework provides comprehensive validation ensuring reliability, performance, and stability across all deployment scenarios.**