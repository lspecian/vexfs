# VexFS eBPF Dynamic Kernel Tracing

Comprehensive eBPF/bpftrace-based dynamic tracing infrastructure for VexFS kernel module debugging, performance analysis, and optimization.

## Overview

This directory contains a complete eBPF tracing solution for VexFS that provides:

- **Real-time kernel module monitoring** with minimal overhead
- **Performance bottleneck identification** for vector operations
- **Memory leak detection** and allocation pattern analysis
- **Lock contention analysis** for concurrency debugging
- **Automated analysis and reporting** with actionable insights

## Directory Structure

```
tests/ebpf_tracing/
├── scripts/                    # bpftrace scripts
│   ├── vexfs_kernel_trace.bt      # General kernel operations
│   ├── vexfs_performance_trace.bt # Performance-focused tracing
│   └── vexfs_memory_trace.bt      # Memory analysis
├── tools/                      # Management tools
│   └── vexfs_trace_manager.sh     # Main tracing manager
├── configs/                    # Configuration files
│   └── default_trace_config.yaml  # Default configuration
├── results/                    # Trace output files
├── analysis/                   # Analysis reports
└── README.md                   # This file
```

## Quick Start

### Prerequisites

1. **Root privileges** (required for kernel tracing)
2. **bpftrace v0.20.2+** installed
3. **Linux kernel 6.11+** with eBPF support
4. **VexFS kernel module** loaded (recommended)

### Installation Check

```bash
# Check if bpftrace is available
which bpftrace

# Check kernel version
uname -r

# Check if VexFS module is loaded
lsmod | grep vexfs
```

### Basic Usage

```bash
# Navigate to the tracing directory
cd tests/ebpf_tracing

# List available tracing scripts
sudo ./tools/vexfs_trace_manager.sh list

# Run general kernel tracing for 60 seconds
sudo ./tools/vexfs_trace_manager.sh run kernel

# Run performance analysis for 5 minutes
sudo ./tools/vexfs_trace_manager.sh run performance -d 300

# Start comprehensive monitoring
sudo ./tools/vexfs_trace_manager.sh monitor

# Check current tracing status
./tools/vexfs_trace_manager.sh status

# Analyze results
sudo ./tools/vexfs_trace_manager.sh analyze results/vexfs_kernel_trace_*.txt
```

## Tracing Scripts

### 1. General Kernel Tracing (`vexfs_kernel_trace.bt`)

**Purpose**: Comprehensive monitoring of all VexFS kernel module operations

**Features**:
- Filesystem operation tracking (read/write/search)
- Memory allocation/deallocation monitoring
- Lock operation analysis
- Error detection and reporting
- Module lifecycle events
- Real-time statistics every 10 seconds

**Use Cases**:
- General debugging and monitoring
- Understanding VexFS operation patterns
- Identifying error conditions
- Baseline performance measurement

**Example Output**:
```
[14:30:15] VexFS: vexfs_vector_search() called by PID=1234 (test_app)
[14:30:15] VexFS SEARCH: vexfs_vector_search() PID=1234 (test_app)
[14:30:15] VexFS SEARCH COMPLETE: vexfs_vector_search() latency=250 μs, results=10
```

### 2. Performance Tracing (`vexfs_performance_trace.bt`)

**Purpose**: High-performance vector operations analysis and optimization

**Features**:
- Vector operation latency tracking
- HNSW algorithm performance monitoring
- LSH algorithm analysis
- I/O throughput measurement
- Cache hit/miss ratio tracking
- Lock contention detection
- Real-time performance alerts

**Use Cases**:
- Performance optimization
- Bottleneck identification
- Algorithm comparison (HNSW vs LSH)
- Cache efficiency analysis
- Concurrency issue detection

**Key Metrics**:
- Vector operation latencies (μs)
- I/O throughput (MB/s)
- Cache hit rates (%)
- Lock wait times (μs)
- Error rates (%)

### 3. Memory Analysis (`vexfs_memory_trace.bt`)

**Purpose**: Memory allocation patterns, leak detection, and usage optimization

**Features**:
- Allocation/deallocation tracking
- Memory leak detection
- Large allocation monitoring
- Vector memory usage analysis
- Memory pressure detection
- Allocation lifetime analysis
- Double-free detection

**Use Cases**:
- Memory leak debugging
- Memory usage optimization
- Allocation pattern analysis
- Memory pressure investigation
- Vector memory efficiency

**Key Metrics**:
- Total memory usage (MB)
- Outstanding allocations count
- Allocation lifetimes (ms)
- Large allocations (>1MB)
- Memory leak indicators

## Trace Manager Tool

The `vexfs_trace_manager.sh` tool provides a comprehensive interface for managing VexFS tracing:

### Commands

| Command | Description | Example |
|---------|-------------|---------|
| `list` | List available tracing scripts | `./vexfs_trace_manager.sh list` |
| `run <script>` | Run specific tracing script | `./vexfs_trace_manager.sh run kernel -d 120` |
| `monitor` | Start comprehensive monitoring | `./vexfs_trace_manager.sh monitor` |
| `performance` | Run performance-focused tracing | `./vexfs_trace_manager.sh performance` |
| `memory` | Run memory analysis tracing | `./vexfs_trace_manager.sh memory` |
| `analyze <file>` | Analyze tracing results | `./vexfs_trace_manager.sh analyze results/trace.txt` |
| `status` | Show current tracing status | `./vexfs_trace_manager.sh status` |
| `stop` | Stop all running traces | `./vexfs_trace_manager.sh stop` |
| `clean` | Clean up old results | `./vexfs_trace_manager.sh clean` |

### Options

| Option | Description | Default |
|--------|-------------|---------|
| `-d, --duration <sec>` | Set tracing duration | 60s |
| `-o, --output <file>` | Set output file | auto-generated |
| `-v, --verbose` | Enable verbose output | disabled |
| `-q, --quiet` | Suppress non-essential output | disabled |
| `--no-module-check` | Skip VexFS module check | disabled |

## Configuration

The tracing infrastructure uses YAML configuration files in the `configs/` directory:

### Default Configuration (`default_trace_config.yaml`)

Key configuration sections:

- **General Settings**: Duration, output directory, log level
- **Kernel Trace**: Filesystem ops, memory ops, sampling rates
- **Performance Trace**: Vector ops, HNSW/LSH tracking, alerts
- **Memory Trace**: Leak detection, pressure monitoring
- **Output**: Format options, real-time monitoring
- **Advanced**: eBPF settings, debug options

### Environment-Specific Overrides

The configuration supports environment-specific settings:

- **Development**: Debug mode enabled, verbose output
- **Production**: Reduced logging, stricter thresholds
- **Testing**: Shorter durations, sampling enabled

## Analysis and Reporting

### Automated Analysis

The trace manager automatically generates analysis reports including:

- **Error Analysis**: Error counts, patterns, samples
- **Performance Highlights**: Slow operations, bottlenecks
- **Memory Analysis**: Usage patterns, leak indicators
- **Operation Summary**: Counts by operation type
- **Recommendations**: Actionable optimization suggestions

### Manual Analysis

For detailed analysis, examine the raw trace output:

```bash
# View recent results
ls -la results/

# Examine specific trace
less results/vexfs_kernel_trace_20250605_141530.txt

# Search for specific patterns
grep "ERROR\|SLOW\|LEAK" results/vexfs_*.txt

# Count operations by type
grep -c "VECTOR\|READ\|WRITE" results/vexfs_performance_*.txt
```

## Performance Impact

The eBPF tracing infrastructure is designed for minimal performance impact:

- **Kernel Tracing**: ~1-2% CPU overhead
- **Performance Tracing**: ~2-3% CPU overhead  
- **Memory Tracing**: ~1-2% CPU overhead
- **Comprehensive Monitoring**: ~3-5% CPU overhead

Impact varies based on:
- VexFS operation frequency
- Trace duration
- Enabled features
- System load

## Troubleshooting

### Common Issues

1. **Permission Denied**
   ```bash
   # Solution: Run with sudo
   sudo ./vexfs_trace_manager.sh run kernel
   ```

2. **bpftrace Not Found**
   ```bash
   # Solution: Install bpftrace
   sudo apt-get install bpftrace linux-tools-$(uname -r)
   ```

3. **VexFS Module Not Loaded**
   ```bash
   # Check module status
   lsmod | grep vexfs
   
   # Load module if needed
   sudo insmod /path/to/vexfs.ko
   ```

4. **No Trace Output**
   - Verify VexFS module is active
   - Check if VexFS operations are occurring
   - Ensure correct function names in scripts

### Debug Mode

Enable debug mode for troubleshooting:

```bash
# Run with verbose output
sudo ./vexfs_trace_manager.sh run kernel -v

# Check trace manager logs
tail -f results/trace_manager.log
```

## Integration

### CI/CD Integration

Example GitHub Actions workflow:

```yaml
- name: Run VexFS Performance Tests
  run: |
    sudo ./tests/ebpf_tracing/tools/vexfs_trace_manager.sh run performance -d 60
    sudo ./tests/ebpf_tracing/tools/vexfs_trace_manager.sh analyze results/vexfs_performance_*.txt
```

### Monitoring Integration

The tracing infrastructure can integrate with:

- **Grafana**: Real-time dashboards
- **Prometheus**: Metrics collection
- **Syslog**: Centralized logging
- **Custom Tools**: JSON/CSV output support

## Best Practices

### Development Workflow

1. **Start with General Tracing**: Use `kernel` trace for overview
2. **Focus on Specific Areas**: Use `performance` or `memory` traces
3. **Analyze Results**: Use automated analysis first
4. **Iterate**: Adjust based on findings

### Production Monitoring

1. **Use Shorter Durations**: 30-60 seconds typically sufficient
2. **Enable Sampling**: Reduce overhead in high-load scenarios
3. **Monitor Alerts**: Set up automated alerting
4. **Regular Cleanup**: Remove old trace files

### Performance Optimization

1. **Identify Bottlenecks**: Look for slow operations
2. **Analyze Memory Patterns**: Check for leaks and inefficiencies
3. **Monitor Cache Performance**: Optimize hit rates
4. **Check Lock Contention**: Reduce concurrency issues

## Contributing

To add new tracing capabilities:

1. **Create New Script**: Add `.bt` file in `scripts/`
2. **Update Manager**: Add command support in `vexfs_trace_manager.sh`
3. **Add Configuration**: Update `default_trace_config.yaml`
4. **Document**: Update this README

### Script Development Guidelines

- Use consistent naming conventions
- Include comprehensive comments
- Provide real-time statistics
- Generate final summary reports
- Handle cleanup properly

## Support

For issues or questions:

1. Check this documentation
2. Review trace manager logs
3. Examine bpftrace output
4. Open GitHub issue with trace samples

## Version History

- **v1.0.0**: Initial eBPF tracing infrastructure
  - General kernel tracing
  - Performance analysis
  - Memory leak detection
  - Comprehensive trace manager
  - Configuration system
  - Automated analysis

---

**Note**: This tracing infrastructure requires root privileges and is intended for development, testing, and debugging purposes. Use with caution in production environments.