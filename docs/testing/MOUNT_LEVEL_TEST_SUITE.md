# VexFS Mount-Level Test Suite with Crash Recovery

## Overview

The VexFS Mount-Level Test Suite provides comprehensive testing for mount operations with automated crash recovery capabilities. This suite is designed to validate maximum kernel performance under various stress conditions, edge cases, and failure scenarios.

## Architecture

### Core Components

1. **[`MountTestSuite`](mdc:tests/kernel_module/src/mount_test_suite.rs)** - Comprehensive mount testing framework
2. **[`MountRecoveryManager`](mdc:tests/kernel_module/src/mount_recovery.rs)** - Automated crash recovery for mount operations
3. **[`mount_level_runner`](mdc:tests/kernel_module/src/bin/mount_level_runner.rs)** - Dedicated test runner binary
4. **[Enhanced VM Operations](mdc:tests/kernel_module/src/enhanced_vm_operations.rs)** - Advanced mount scenario testing

### Test Categories

#### 1. Normal Mount Operations
- **Basic mount/unmount operations** with timing measurements
- **Mount with various options** (ro, rw, noexec, nosuid, sync)
- **Remount operations** with different option sets
- **Mount point validation** for various directory structures
- **Filesystem type detection** and validation

#### 2. Edge Case Testing
- **Invalid mount options** testing for proper error handling
- **Already-mounted filesystem** scenarios
- **Nonexistent device** mount attempts
- **Invalid mount point** testing
- **Permission denied scenarios** without sudo
- **Corrupted filesystem** mount attempts
- **Device busy scenarios** with force unmount testing

#### 3. Concurrent Mount Testing
- **Parallel mount attempts** with race condition detection
- **Resource contention handling** during concurrent operations
- **Deadlock detection and prevention**
- **Maximum concurrent mount capacity** testing

#### 4. Stress Testing
- **Mount/unmount cycles** under sustained load
- **Memory leak detection** during repeated operations
- **Performance degradation monitoring**
- **Stability scoring** based on success rates
- **Recovery validation** after failures

#### 5. Resource Constraint Testing
- **Low memory conditions** during mount operations
- **High CPU load** impact on mount performance
- **Disk space constraints** testing
- **File descriptor limits** testing
- **Network constraint simulation**
- **Constraint recovery validation**

## Crash Recovery Features

### Automated Recovery Mechanisms

1. **System Hang Detection**
   - VM responsiveness monitoring
   - SSH connectivity testing
   - Watchdog timeout handling

2. **Crash State Preservation**
   - Detailed crash event logging
   - System state capture before/after crashes
   - Recovery action documentation

3. **Recovery Procedures**
   - Force unmount of stuck filesystems
   - Process cleanup for hung mount operations
   - Loop device cleanup and reset
   - Kernel module reload
   - VM snapshot restoration for severe crashes

4. **Recovery Validation**
   - Post-recovery functionality testing
   - System stability verification
   - Performance impact assessment

### Recovery Actions

| Action Type | Description | Use Case |
|-------------|-------------|----------|
| `ForceUnmount` | Force unmount all stuck filesystems | Hung mount operations |
| `ProcessKill` | Kill stuck mount processes | Process deadlocks |
| `ResourceCleanup` | Clean up loop devices and resources | Resource leaks |
| `ModuleReload` | Reload VexFS kernel module | Module corruption |
| `SnapshotRestore` | Restore VM from clean snapshot | System crashes |
| `FilesystemCheck` | Verify recovery success | Post-recovery validation |

## Usage

### Quick Start

```bash
# Basic mount testing
./tests/vm_testing/run_mount_level_tests.sh

# Intensive testing with multiple cycles
./tests/vm_testing/run_mount_level_tests.sh --test-cycles 5 --stress-cycles 100 --verbose

# High-concurrency testing
./tests/vm_testing/run_mount_level_tests.sh --max-concurrent 10 --vm-memory 8192 --vm-cpus 8
```

### Direct Binary Usage

```bash
# Build and run the mount test runner
cargo build --release --bin mount_level_runner -p kernel_module_tests
./target/release/mount_level_runner --help

# Example with custom configuration
./target/release/mount_level_runner \
    --vm-memory 8192 \
    --vm-cpus 8 \
    --test-cycles 3 \
    --max-concurrent 10 \
    --stress-cycles 50 \
    --verbose
```

### Programmatic Usage

```rust
use kernel_module_tests::{
    VmConfig, 
    mount_test_suite::MountTestSuite,
    mount_recovery::{MountRecoveryManager, RecoveryConfig}
};

// Initialize test suite
let vm_config = VmConfig::default();
let mut mount_suite = MountTestSuite::new(vm_config.clone())
    .with_crash_detection(true)
    .with_stress_cycles(50)
    .with_max_concurrent_mounts(10);

// Initialize recovery manager
let recovery_config = RecoveryConfig::default();
let mut recovery_manager = MountRecoveryManager::new(vm_config)
    .with_recovery_config(recovery_config)
    .with_crash_detection(true);

// Start monitoring
recovery_manager.start_monitoring()?;

// Run comprehensive tests
let results = mount_suite.run_comprehensive_mount_tests()?;

// Stop monitoring
recovery_manager.stop_monitoring()?;
```

## Configuration Options

### VM Configuration

| Parameter | Default | Description |
|-----------|---------|-------------|
| `vm_memory_mb` | 4096 | VM memory allocation in MB |
| `vm_cpus` | 4 | Number of VM CPU cores |
| `ssh_port` | 2222 | SSH port for VM access |
| `enable_kvm` | true | Enable KVM acceleration |

### Test Configuration

| Parameter | Default | Description |
|-----------|---------|-------------|
| `test_cycles` | 1 | Number of complete test cycles |
| `max_concurrent_mounts` | 10 | Maximum concurrent mount operations |
| `stress_test_cycles` | 50 | Number of stress test iterations |
| `test_timeout` | 3600s | Overall test timeout |

### Recovery Configuration

| Parameter | Default | Description |
|-----------|---------|-------------|
| `max_recovery_attempts` | 3 | Maximum recovery attempts per crash |
| `recovery_timeout_seconds` | 300 | Recovery operation timeout |
| `auto_recovery_enabled` | true | Enable automatic crash recovery |
| `preserve_crash_state` | true | Preserve crash state for analysis |
| `mount_timeout_seconds` | 60 | Individual mount operation timeout |
| `hang_detection_threshold_seconds` | 120 | VM hang detection threshold |

## Test Results and Reporting

### Result Structure

```rust
pub struct MountTestResult {
    pub test_name: String,
    pub status: TestStatus,
    pub duration_ms: u64,
    pub normal_mount_tests: NormalMountTestResults,
    pub edge_case_tests: EdgeCaseTestResults,
    pub concurrent_mount_tests: ConcurrentMountTestResults,
    pub stress_test_results: StressTestResults,
    pub resource_constraint_tests: ResourceConstraintTestResults,
    pub performance_metrics: PerformanceMetrics,
    pub crash_events: Vec<CrashEvent>,
    pub error_details: Option<String>,
}
```

### Performance Metrics

- **Mount/Unmount Timing** - Precise timing measurements for all operations
- **Memory Usage** - System and kernel memory consumption tracking
- **CPU Usage** - CPU utilization during mount operations
- **I/O Operations** - Disk I/O performance metrics
- **Stability Score** - Overall system stability rating (0-100%)

### Output Files

| File Type | Location | Description |
|-----------|----------|-------------|
| JSON Report | `tests/vm_testing/results/mount_test_report_*.json` | Detailed test results |
| Summary Report | `tests/vm_testing/results/mount_test_summary_*.txt` | Human-readable summary |
| Crash Logs | `tests/vm_testing/logs/mount_crash_recovery.log` | Crash detection logs |
| Performance Logs | `tests/vm_testing/logs/mount_performance.log` | Performance monitoring data |
| Recovery Logs | `tests/vm_testing/logs/mount_recovery_actions.log` | Recovery action history |

## Advanced Features

### Failure Injection Testing

The test suite includes sophisticated failure injection capabilities:

- **Mount Failure Injection** - Intentional mount failures to test error handling
- **Filesystem Corruption** - Simulated corruption scenarios
- **Memory Pressure** - Artificial memory constraints during operations
- **I/O Error Simulation** - Simulated disk I/O errors
- **Network Constraints** - Network-related failure simulation

### Race Condition Detection

- **Concurrent Operation Monitoring** - Detection of race conditions in parallel mounts
- **Resource Contention Analysis** - Identification of resource conflicts
- **Deadlock Prevention** - Proactive deadlock detection and prevention
- **Timing Analysis** - Precise timing analysis for race condition identification

### Performance Optimization

- **Benchmark Comparison** - Performance comparison against baseline metrics
- **Regression Detection** - Automatic detection of performance regressions
- **Optimization Recommendations** - Suggestions for performance improvements
- **Scalability Analysis** - Analysis of performance under increasing load

## Integration with CI/CD

### GitHub Actions Integration

```yaml
- name: Run Mount-Level Tests
  run: |
    ./tests/vm_testing/run_mount_level_tests.sh \
      --test-cycles 3 \
      --stress-cycles 25 \
      --max-concurrent 5
```

### Test Result Validation

```bash
# Check test results
if [[ -f "tests/vm_testing/results/mount_test_summary_*.txt" ]]; then
    if grep -q "Overall Assessment: EXCELLENT" tests/vm_testing/results/mount_test_summary_*.txt; then
        echo "✅ Mount tests passed"
        exit 0
    else
        echo "❌ Mount tests failed"
        exit 1
    fi
fi
```

## Troubleshooting

### Common Issues

1. **VM Setup Failures**
   - Verify VM image exists: `tests/vm_images/vexfs-test.qcow2`
   - Check SSH key permissions: `tests/vm_keys/vexfs_test_key`
   - Ensure QEMU is installed and accessible

2. **Mount Operation Failures**
   - Check kernel module compilation
   - Verify loop device availability
   - Ensure sufficient disk space

3. **Recovery Failures**
   - Check VM snapshot creation
   - Verify recovery timeout settings
   - Review crash detection logs

### Debug Mode

```bash
# Enable verbose logging
./tests/vm_testing/run_mount_level_tests.sh --verbose

# Check specific logs
tail -f tests/vm_testing/logs/mount_crash_recovery.log
tail -f tests/vm_testing/logs/mount_recovery_actions.log
```

### Manual Recovery

```bash
# Manual cleanup if needed
./tests/vm_testing/run_mount_level_tests.sh --cleanup-only

# Reset VM state
sudo pkill -f qemu-system-x86_64
sudo losetup -D
```

## Performance Benchmarks

### Expected Performance Metrics

| Operation | Target Time | Acceptable Range |
|-----------|-------------|------------------|
| Basic Mount | < 100ms | 50-200ms |
| Basic Unmount | < 50ms | 20-100ms |
| Concurrent Mount (5x) | < 500ms | 200ms-1s |
| Stress Test Cycle | < 10s | 5-20s |
| Recovery Time | < 30s | 10-60s |

### Stability Targets

- **Overall Success Rate**: > 95%
- **Concurrent Mount Success**: > 90%
- **Recovery Success Rate**: > 80%
- **Zero Critical Crashes**: No kernel panics or system hangs

## Future Enhancements

### Planned Features

1. **Real-time Monitoring Dashboard** - Web-based monitoring interface
2. **Machine Learning Analysis** - AI-powered failure prediction
3. **Distributed Testing** - Multi-node concurrent testing
4. **Performance Profiling** - Detailed kernel-level profiling
5. **Automated Optimization** - Self-tuning performance parameters

### Integration Opportunities

- **Kubernetes Testing** - Container orchestration mount testing
- **Cloud Provider Integration** - AWS/GCP/Azure specific testing
- **Hardware Validation** - Different storage hardware testing
- **Filesystem Comparison** - Comparative analysis with other filesystems

## Conclusion

The VexFS Mount-Level Test Suite with Crash Recovery provides comprehensive validation of mount operations under extreme conditions. It ensures maximum kernel performance while maintaining system stability and reliability through automated recovery mechanisms.

For questions or issues, refer to the [troubleshooting section](#troubleshooting) or check the detailed logs in `tests/vm_testing/logs/`.