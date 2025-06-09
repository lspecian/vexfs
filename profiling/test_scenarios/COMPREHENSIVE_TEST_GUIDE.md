# VexFS FUSE Stack Overflow Test Scenarios - Comprehensive Guide

This guide provides detailed instructions for using the comprehensive test scenarios designed to reproduce and analyze stack overflow issues in VexFS FUSE implementation, specifically targeting VectorStorageManager and VectorSearchEngine initialization problems.

## Overview

The test scenarios are organized into five categories, each targeting specific aspects of the stack overflow problem:

1. **Large Vector Operations** - Tests with progressively larger vector datasets
2. **HNSW Graph Traversal** - Tests deep graph traversal scenarios
3. **Component Initialization** - Tests isolated and combined component initialization
4. **Stress Testing** - Tests under extreme conditions and memory pressure
5. **Baseline Comparison** - Compares FUSE vs kernel module performance

## Quick Start

### Prerequisites

1. **System Requirements:**
   - Linux system with kernel 6.1+
   - At least 8GB RAM (16GB recommended)
   - 20GB free disk space
   - Root access for some profiling tools

2. **Software Dependencies:**
   ```bash
   # Install profiling tools
   sudo apt-get update
   sudo apt-get install valgrind linux-tools-generic bpftrace
   
   # Install Python dependencies
   pip3 install pyyaml
   ```

3. **VexFS Build:**
   ```bash
   # Build VexFS with profiling configuration
   cd /path/to/vexfs
   cargo build --profile profiling
   ```

### Basic Usage

1. **Setup Environment:**
   ```bash
   # Setup profiling environment (run once)
   sudo ./profiling/scripts/setup_profiling_environment.sh
   ```

2. **Run All Test Scenarios:**
   ```bash
   # Run comprehensive test suite
   ./profiling/test_scenarios/run_all_scenarios.sh --verbose
   ```

3. **Run Specific Category:**
   ```bash
   # Run only large vector operations tests
   ./profiling/test_scenarios/run_all_scenarios.sh \
     --categories large_vector_operations \
     --verbose
   ```

## Detailed Usage

### Test Category Execution

#### 1. Large Vector Operations

Tests VectorStorageManager with progressively larger datasets to identify stack overflow thresholds.

```bash
# Run large vector operations tests
./profiling/test_scenarios/large_vector_operations/run_large_vector_tests.sh \
  --verbose \
  --scenario dataset_size_progression \
  --tools valgrind,perf

# Available scenarios:
# - dataset_size_progression: 1K to 1M vectors
# - dimension_scaling: 128 to 4096 dimensions
# - bulk_operations: Batch insert/update/delete
# - concurrent_operations: Multi-threaded operations
# - metadata_operations: Metadata-heavy operations
```

**Key Parameters:**
- `vector_count`: Number of vectors (1K to 1M)
- `dimensions`: Vector dimensions (128 to 4096)
- `batch_size`: Batch operation size
- `threads`: Concurrent operation threads

#### 2. HNSW Graph Traversal

Tests VectorSearchEngine with deep graph traversal scenarios that may cause stack overflow.

```bash
# Run HNSW graph traversal tests
./profiling/test_scenarios/hnsw_graph_traversal/run_hnsw_tests.sh \
  --verbose \
  --scenario deep_traversal \
  --tools valgrind,ebpf

# Available scenarios:
# - graph_construction: Different HNSW parameters
# - deep_traversal: High ef values for deep search
# - connectivity_patterns: Various graph topologies
# - memory_intensive: Large batch operations
# - recursive_algorithms: Recursion depth testing
```

**Key Parameters:**
- `M`: HNSW connectivity parameter (16 to 128)
- `efConstruction`: Construction parameter (200 to 1600)
- `search_ef`: Search parameter (50 to 2000)
- `max_recursion_depth`: Recursion limit testing

#### 3. Component Initialization

Tests isolated and combined component initialization to identify the problematic combination.

```bash
# Run component initialization tests
./profiling/test_scenarios/component_initialization/run_init_tests.sh \
  --verbose \
  --scenario combined_initialization \
  --tools valgrind,perf

# Available scenarios:
# - isolated_initialization: Single component tests
# - combined_initialization: Multi-component tests
# - memory_configurations: Different memory limits
# - initialization_order: Order dependency tests
# - error_recovery: Failure recovery tests
# - rapid_cycles: Rapid init/deinit cycles
```

**Key Components:**
- `vector_storage_manager`: Vector storage component
- `vector_search_engine`: Search engine component
- `metadata_manager`: Metadata management
- `cache_manager`: Caching system
- `transaction_manager`: Transaction handling

#### 4. Stress Testing

Tests under extreme conditions and memory pressure.

```bash
# Run stress tests
./profiling/test_scenarios/stress_testing/run_stress_tests.sh \
  --verbose \
  --scenario memory_pressure \
  --tools valgrind,perf,ebpf

# Available scenarios:
# - memory_pressure: Low memory conditions
# - limited_stack: Reduced stack space
# - rapid_cycles: Rapid mount/unmount cycles
# - error_recovery: Error injection and recovery
# - high_concurrency: Many concurrent operations
# - resource_exhaustion: Resource limit testing
# - long_running: Extended stability tests
```

**Stress Conditions:**
- Memory limits: 128MB to unlimited
- Stack limits: 64KB to 256KB
- Concurrency: 1 to 50 threads
- Duration: 5 minutes to 24 hours

#### 5. Baseline Comparison

Compares FUSE vs kernel module performance and stack usage.

```bash
# Run baseline comparison tests
./profiling/test_scenarios/baseline_comparison/run_comparison_tests.sh \
  --verbose \
  --scenario performance_benchmarks \
  --implementations fuse,kernel

# Available scenarios:
# - equivalent_operations: Same operations on both
# - performance_benchmarks: Throughput/latency comparison
# - memory_usage: Memory footprint analysis
# - stack_usage: Stack depth comparison
# - resource_utilization: CPU/IO efficiency
# - error_handling: Error behavior comparison
```

### Profiling Tools

#### Valgrind
- **Purpose:** Memory error detection and stack analysis
- **Output:** Detailed memory usage and stack traces
- **Best for:** Memory leaks and stack overflow detection

```bash
# Valgrind-specific options
--tools valgrind
# Generates: *_valgrind.log files with detailed analysis
```

#### Perf
- **Purpose:** Performance profiling and hotspot identification
- **Output:** Performance data and call graphs
- **Best for:** CPU usage and performance bottlenecks

```bash
# Perf-specific options
--tools perf
# Generates: *.data files and *_report.txt analysis
```

#### eBPF
- **Purpose:** Real-time kernel tracing and monitoring
- **Output:** Live tracing data and stack monitoring
- **Best for:** Real-time stack depth monitoring

```bash
# eBPF-specific options
--tools ebpf
# Generates: *_ebpf.log files with trace data
```

### Advanced Usage

#### Parallel Execution

Run multiple test categories in parallel for faster execution:

```bash
./profiling/test_scenarios/run_all_scenarios.sh \
  --parallel \
  --jobs 4 \
  --categories large_vector_operations,component_initialization
```

#### Custom Configuration

Modify test parameters by editing configuration files:

```bash
# Edit large vector operations config
vim ./profiling/test_scenarios/large_vector_operations/test_config.yaml

# Edit HNSW traversal config
vim ./profiling/test_scenarios/hnsw_graph_traversal/test_config.yaml
```

#### Dry Run Mode

Test configuration without actual execution:

```bash
./profiling/test_scenarios/run_all_scenarios.sh \
  --dry-run \
  --verbose \
  --categories all
```

## Results Analysis

### Result Structure

```
profiling/results/comprehensive_analysis/
├── logs/                           # Execution logs
├── profiles/                       # Profiling data
│   ├── valgrind/                  # Valgrind outputs
│   ├── perf/                      # Perf data files
│   └── ebpf/                      # eBPF traces
├── data/                          # Test data files
├── analysis/                      # Analysis results
├── reports/                       # Generated reports
├── large_vector_operations/       # Category-specific results
├── hnsw_graph_traversal/
├── component_initialization/
├── stress_testing/
└── baseline_comparison/
```

### Key Files to Examine

1. **Execution Logs:**
   - `logs/execution_*.log` - Overall execution logs
   - `{category}/logs/execution_*.log` - Category-specific logs

2. **Profiling Data:**
   - `profiles/*_valgrind.log` - Memory analysis
   - `profiles/*.data` - Perf performance data
   - `profiles/*_ebpf.log` - Real-time traces

3. **Analysis Reports:**
   - `reports/comprehensive_stack_overflow_analysis_*.md` - Main report
   - `analysis/*_analysis.md` - Detailed analysis per category

### Stack Overflow Indicators

Look for these patterns in the results:

1. **Valgrind Output:**
   ```
   ==PID== Stack overflow in thread 1: can't grow stack to 0x...
   ==PID== Process terminating with default action of signal 11 (SIGSEGV)
   ```

2. **System Logs:**
   ```bash
   # Check system logs for segmentation faults
   dmesg | grep -i "segfault\|stack"
   journalctl -f | grep -i "vexfs\|fuse"
   ```

3. **Performance Degradation:**
   - Sudden performance drops
   - Memory usage spikes
   - High CPU usage without progress

## Troubleshooting

### Common Issues

1. **Permission Denied:**
   ```bash
   # Ensure proper permissions
   sudo chown -R $USER:$USER ./profiling/results/
   chmod +x ./profiling/test_scenarios/*/run_*.sh
   ```

2. **FUSE Mount Failures:**
   ```bash
   # Check FUSE availability
   modprobe fuse
   
   # Clean up stale mounts
   fusermount -u /tmp/vexfs_*_test
   ```

3. **Profiling Tool Issues:**
   ```bash
   # Check tool availability
   which valgrind perf bpftrace
   
   # Install missing tools
   sudo apt-get install valgrind linux-tools-generic bpftrace
   ```

4. **Memory Issues:**
   ```bash
   # Check available memory
   free -h
   
   # Reduce test parameters if needed
   # Edit test_config.yaml files to use smaller datasets
   ```

### Debug Mode

Enable debug output for troubleshooting:

```bash
export VERBOSE=true
./profiling/test_scenarios/run_all_scenarios.sh --verbose
```

### Manual Cleanup

If tests fail to clean up properly:

```bash
# Unmount all test filesystems
for mount in /tmp/vexfs_*_test; do
    fusermount -u "$mount" 2>/dev/null || true
done

# Kill any remaining processes
pkill -f vexfs_fuse
pkill -f valgrind

# Clean up temporary files
rm -rf /tmp/vexfs_*_test
```

## Expected Outcomes

### Success Criteria

1. **Stack Usage Analysis:**
   - Identification of maximum stack depth
   - Detection of stack overflow conditions
   - Mapping of stack usage to specific components

2. **Component Isolation:**
   - Identification of problematic component combinations
   - Isolation of VectorStorageManager vs VectorSearchEngine issues
   - Understanding of initialization order dependencies

3. **Performance Baseline:**
   - FUSE vs kernel module comparison
   - Performance impact of stack optimizations
   - Resource usage patterns

### Deliverables

1. **Comprehensive Report:**
   - Root cause analysis of stack overflow issues
   - Specific recommendations for optimization
   - Performance comparison data

2. **Profiling Data:**
   - Detailed stack traces showing overflow points
   - Memory allocation patterns
   - Performance bottleneck identification

3. **Optimization Roadmap:**
   - Prioritized list of fixes
   - Implementation strategies
   - Validation test plans

## Next Steps

After running the test scenarios:

1. **Analyze Results:**
   - Review comprehensive report
   - Examine profiling data
   - Identify root causes

2. **Implement Fixes:**
   - Apply stack optimization techniques
   - Modify component initialization order
   - Implement memory management improvements

3. **Validate Improvements:**
   - Re-run test scenarios
   - Compare before/after results
   - Ensure no regressions

4. **Monitor Ongoing:**
   - Integrate tests into CI/CD
   - Set up continuous monitoring
   - Establish performance baselines

## Support

For issues or questions:

1. **Check Logs:** Review execution logs in `profiling/results/`
2. **Debug Mode:** Run with `--verbose` flag
3. **System Check:** Verify system requirements and dependencies
4. **Manual Testing:** Try individual test scenarios first

---

**Note:** These test scenarios are designed to stress-test the VexFS FUSE implementation and may cause system instability under extreme conditions. Always run on dedicated test systems, not production environments.