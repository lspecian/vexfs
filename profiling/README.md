# VexFS FUSE Stack Overflow Profiling Environment

## Task 23.1: Comprehensive Profiling Setup for FUSE Stack Analysis

This directory contains the complete profiling infrastructure for analyzing and resolving FUSE stack overflow issues in VexFS.

## Overview

The profiling environment provides:

- **Debug Build Configuration**: Optimized profiling builds with preserved stack frames
- **Multi-Tool Profiling**: Valgrind, perf, and eBPF integration
- **Automated Testing**: Controlled reproduction of stack overflow conditions
- **Incremental Analysis**: Framework for systematic component re-enablement
- **Comprehensive Documentation**: Methodology and usage guides

## Directory Structure

```
profiling/
├── README.md                           # This file
├── configs/                            # Configuration files
│   ├── valgrind_stack_analysis.conf    # Valgrind stack analysis config
│   ├── perf_memory_profile.conf        # Perf memory profiling config
│   └── ebpf_fuse_tracing.yaml          # eBPF FUSE-specific tracing config
├── scripts/                            # Profiling execution scripts
│   ├── run_valgrind_stack_analysis.sh  # Valgrind stack usage analysis
│   ├── run_perf_memory_profile.sh      # Performance memory profiling
│   ├── run_ebpf_fuse_tracing.sh        # eBPF FUSE operation tracing
│   ├── setup_profiling_environment.sh  # Environment setup
│   └── analyze_profiling_results.sh    # Results analysis
├── test_environments/                  # Controlled test setups
│   ├── minimal_fuse_test.sh            # Minimal FUSE baseline test
│   ├── incremental_component_test.sh   # Component re-enablement test
│   └── stack_overflow_reproduction.sh  # Stack overflow reproduction
├── results/                            # Profiling output directory
│   ├── valgrind/                       # Valgrind analysis results
│   ├── perf/                           # Perf profiling results
│   ├── ebpf/                           # eBPF tracing results
│   └── analysis/                       # Combined analysis reports
└── docs/                               # Profiling documentation
    ├── PROFILING_METHODOLOGY.md        # Systematic profiling approach
    ├── TOOL_CONFIGURATIONS.md          # Tool setup and configuration
    └── COMPONENT_REACTIVATION_CHECKLIST.md # Systematic component re-enablement
```

## Quick Start

### Prerequisites

1. **Root privileges** (required for some profiling tools)
2. **Valgrind** installed (`sudo apt-get install valgrind`)
3. **Linux perf tools** installed (`sudo apt-get install linux-tools-$(uname -r)`)
4. **bpftrace** installed (for eBPF tracing)
5. **VexFS FUSE binary** built with profiling profile

### Setup Environment

```bash
# Navigate to profiling directory
cd profiling/

# Set up profiling environment
sudo ./scripts/setup_profiling_environment.sh

# Build VexFS with profiling configuration
cargo build --profile profiling --features fuse_support --bin vexfs_fuse
```

### Basic Usage

```bash
# Run comprehensive profiling suite
./scripts/run_comprehensive_profiling.sh

# Run individual profiling tools
./scripts/run_valgrind_stack_analysis.sh
./scripts/run_perf_memory_profile.sh
./scripts/run_ebpf_fuse_tracing.sh

# Analyze results
./scripts/analyze_profiling_results.sh
```

## Profiling Tools

### 1. Valgrind Stack Analysis

**Purpose**: Deep stack usage analysis and overflow detection

**Features**:
- Stack frame tracking
- Memory allocation patterns
- Function call depth analysis
- Stack overflow prediction
- Memory leak detection

**Configuration**: [`configs/valgrind_stack_analysis.conf`](configs/valgrind_stack_analysis.conf)

### 2. Perf Memory Profiling

**Purpose**: Performance-oriented memory usage analysis

**Features**:
- Memory allocation hotspots
- Cache performance analysis
- Memory bandwidth utilization
- Function-level memory attribution
- Real-time memory pressure monitoring

**Configuration**: [`configs/perf_memory_profile.conf`](configs/perf_memory_profile.conf)

### 3. eBPF FUSE Tracing

**Purpose**: Kernel-level FUSE operation tracing

**Features**:
- FUSE operation lifecycle tracking
- Stack depth monitoring during FUSE calls
- Memory allocation patterns in FUSE context
- Performance bottleneck identification
- Real-time stack usage alerts

**Configuration**: [`configs/ebpf_fuse_tracing.yaml`](configs/ebpf_fuse_tracing.yaml)

## Test Environments

### Minimal FUSE Test

Establishes baseline measurements with the current minimal FUSE implementation:

```bash
./test_environments/minimal_fuse_test.sh
```

### Incremental Component Test

Systematic framework for re-enabling VexFS components:

```bash
./test_environments/incremental_component_test.sh --component vector_storage
./test_environments/incremental_component_test.sh --component search_engine
```

### Stack Overflow Reproduction

Controlled reproduction of stack overflow conditions:

```bash
./test_environments/stack_overflow_reproduction.sh --scenario deep_recursion
./test_environments/stack_overflow_reproduction.sh --scenario large_allocations
```

## Analysis Workflow

### 1. Baseline Establishment

```bash
# Run minimal FUSE test to establish baseline
./test_environments/minimal_fuse_test.sh

# Analyze baseline results
./scripts/analyze_profiling_results.sh --baseline
```

### 2. Component Analysis

```bash
# Test individual components
for component in vector_storage search_engine metadata_manager; do
    ./test_environments/incremental_component_test.sh --component $component
    ./scripts/analyze_profiling_results.sh --component $component
done
```

### 3. Stack Overflow Investigation

```bash
# Reproduce stack overflow scenarios
./test_environments/stack_overflow_reproduction.sh --all-scenarios

# Comprehensive analysis
./scripts/analyze_profiling_results.sh --stack-overflow-analysis
```

## Integration with Existing eBPF Infrastructure

This profiling environment integrates with the existing eBPF tracing infrastructure in [`tests/ebpf_tracing/`](../tests/ebpf_tracing/):

- **Shared Configuration**: Extends existing eBPF configurations
- **Tool Integration**: Uses existing `vexfs_trace_manager.sh`
- **Result Correlation**: Correlates FUSE-specific traces with kernel traces
- **Unified Analysis**: Combined analysis across all tracing tools

## Best Practices

### Development Workflow

1. **Start with Baseline**: Always establish minimal FUSE baseline
2. **Incremental Testing**: Add components one at a time
3. **Continuous Monitoring**: Use real-time profiling during development
4. **Document Findings**: Record all observations and decisions

### Profiling Optimization

1. **Use Profiling Build**: Always use `--profile profiling` for accurate results
2. **Isolate Components**: Test components individually before combining
3. **Monitor Resource Usage**: Track profiling tool overhead
4. **Validate Results**: Cross-reference between different profiling tools

### Safety Considerations

1. **VM Environment**: Run stack overflow tests in isolated VM
2. **Resource Limits**: Set appropriate ulimits for stack size
3. **Backup Data**: Ensure no important data in test directories
4. **Monitor System**: Watch for system instability during tests

## Expected Outcomes

### Immediate Goals

- [ ] Establish baseline stack usage for minimal FUSE implementation
- [ ] Identify specific components causing stack overflow
- [ ] Quantify stack usage per component
- [ ] Create safe re-enablement strategy

### Long-term Goals

- [ ] Optimize stack usage across all VexFS components
- [ ] Implement stack-safe alternatives for problematic components
- [ ] Create continuous profiling integration for development
- [ ] Establish performance regression detection

## Support

For issues or questions:

1. Check profiling logs in `results/` directory
2. Review tool configurations in `configs/` directory
3. Examine test environment scripts for debugging
4. Consult methodology documentation in `docs/`

---

**Note**: This profiling environment is designed for development and debugging. Some tools require root privileges and may impact system performance.