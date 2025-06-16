#!/bin/bash

# VexFS FUSE eBPF Tracing Script
# Task 23.1: Kernel-level FUSE operation tracing and stack monitoring

set -euo pipefail

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROFILING_DIR="$(dirname "$SCRIPT_DIR")"
PROJECT_ROOT="$(dirname "$PROFILING_DIR")"
RESULTS_DIR="$PROFILING_DIR/results/ebpf"
CONFIG_FILE="$PROFILING_DIR/configs/ebpf_fuse_tracing.yaml"
EBPF_TOOLS_DIR="$PROJECT_ROOT/tests/ebpf_tracing"

# Default parameters
DURATION=300
MOUNT_POINT="/tmp/vexfs_profiling_mount"
OUTPUT_PREFIX="vexfs_fuse_ebpf_trace"
VERBOSE=false
REAL_TIME=false
TRACE_TYPE="comprehensive"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Logging functions
log() {
    echo -e "${BLUE}[$(date +'%Y-%m-%d %H:%M:%S')]${NC} $1"
}

error() {
    echo -e "${RED}[ERROR]${NC} $1" >&2
}

warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

# Usage information
usage() {
    cat << EOF
Usage: $0 [OPTIONS]

VexFS FUSE eBPF Tracing and Stack Monitoring

OPTIONS:
    -d, --duration SECONDS     Tracing duration (default: 300)
    -m, --mount-point PATH     FUSE mount point (default: /tmp/vexfs_profiling_mount)
    -t, --trace-type TYPE      Trace type (stack|memory|performance|comprehensive) (default: comprehensive)
    -o, --output PREFIX        Output file prefix (default: vexfs_fuse_ebpf_trace)
    -r, --real-time            Enable real-time monitoring
    -v, --verbose              Enable verbose output
    -h, --help                 Show this help message

TRACE TYPES:
    stack          - Focus on stack usage and overflow detection
    memory         - Memory allocation and leak detection
    performance    - Performance bottleneck analysis
    comprehensive  - All tracing capabilities combined

EXAMPLES:
    $0                                          # Comprehensive tracing
    $0 -t stack -d 600                         # Stack-focused tracing for 10 minutes
    $0 -t memory -r -v                         # Memory tracing with real-time monitoring
    $0 -m /mnt/test -o custom_trace            # Custom mount point and output

EOF
}

# Parse command line arguments
parse_args() {
    while [[ $# -gt 0 ]]; do
        case $1 in
            -d|--duration)
                DURATION="$2"
                shift 2
                ;;
            -m|--mount-point)
                MOUNT_POINT="$2"
                shift 2
                ;;
            -t|--trace-type)
                TRACE_TYPE="$2"
                shift 2
                ;;
            -o|--output)
                OUTPUT_PREFIX="$2"
                shift 2
                ;;
            -r|--real-time)
                REAL_TIME=true
                shift
                ;;
            -v|--verbose)
                VERBOSE=true
                shift
                ;;
            -h|--help)
                usage
                exit 0
                ;;
            *)
                error "Unknown option: $1"
                usage
                exit 1
                ;;
        esac
    done
}

# Validate environment
validate_environment() {
    log "Validating environment..."
    
    # Check if bpftrace is available
    if ! command -v bpftrace &> /dev/null; then
        error "bpftrace not found. Please install bpftrace."
        exit 1
    fi
    
    # Check if running as root
    if [[ $EUID -ne 0 ]]; then
        error "eBPF tracing requires root privileges. Please run with sudo."
        exit 1
    fi
    
    # Check eBPF support
    if [[ ! -d /sys/kernel/debug/tracing ]]; then
        error "eBPF tracing not available. Please enable CONFIG_BPF_SYSCALL and mount debugfs."
        exit 1
    fi
    
    # Check if VexFS FUSE binary exists
    local binary_path="$PROJECT_ROOT/target/profiling/vexfs_fuse"
    if [[ ! -f "$binary_path" ]]; then
        error "VexFS FUSE binary not found. Please run setup_profiling_environment.sh first."
        exit 1
    fi
    
    # Check existing eBPF infrastructure
    if [[ ! -d "$EBPF_TOOLS_DIR" ]]; then
        error "Existing eBPF infrastructure not found at: $EBPF_TOOLS_DIR"
        exit 1
    fi
    
    # Check if results directory exists
    if [[ ! -d "$RESULTS_DIR" ]]; then
        mkdir -p "$RESULTS_DIR"
        log "Created results directory: $RESULTS_DIR"
    fi
    
    # Check if mount point is available
    if [[ -d "$MOUNT_POINT" ]]; then
        if mountpoint -q "$MOUNT_POINT"; then
            warning "Mount point $MOUNT_POINT is already mounted. Unmounting..."
            fusermount -u "$MOUNT_POINT" || true
        fi
    else
        mkdir -p "$MOUNT_POINT"
        log "Created mount point: $MOUNT_POINT"
    fi
    
    success "Environment validation completed"
}

# Generate timestamp for output files
generate_timestamp() {
    date +'%Y%m%d_%H%M%S'
}

# Create FUSE-specific bpftrace script
create_fuse_trace_script() {
    local trace_type="$1"
    local script_path="$RESULTS_DIR/fuse_trace_${trace_type}_$(generate_timestamp).bt"
    
    log "Creating $trace_type eBPF trace script..."
    
    case "$trace_type" in
        stack)
            cat > "$script_path" << 'EOF'
#!/usr/bin/env bpftrace

BEGIN {
    printf("VexFS FUSE Stack Monitoring Started\n");
    printf("%-20s %-10s %-15s %-10s %s\n", "TIME", "PID", "COMM", "STACK_KB", "FUNCTION");
}

uprobe:/*/vexfs_fuse:* {
    @stack_depth[pid] = @stack_depth[pid] + 1;
    $sp = reg("sp");
    $stack_size = (0x7fffffffffff - $sp) / 1024;
    
    if ($stack_size > 1024) {
        printf("⚠️  HIGH STACK: %-20s %-10d %-15s %-10d %s\n", 
               strftime("%H:%M:%S", nsecs), pid, comm, $stack_size, func);
    }
}

uretprobe:/*/vexfs_fuse:* {
    @stack_depth[pid] = @stack_depth[pid] - 1;
}

interval:s:10 {
    printf("\n=== Stack Summary ===\n");
    print(@stack_depth);
    printf("\n");
}

END {
    clear(@stack_depth);
}
EOF
            ;;
        memory)
            cat > "$script_path" << 'EOF'
#!/usr/bin/env bpftrace

BEGIN {
    printf("VexFS FUSE Memory Monitoring Started\n");
    printf("%-20s %-10s %-15s %-10s %s\n", "TIME", "PID", "COMM", "SIZE", "OPERATION");
}

uprobe:libc:malloc {
    if (comm == "vexfs_fuse") {
        @allocations[pid] = @allocations[pid] + arg0;
        if (arg0 > 1048576) {
            printf("🧠 LARGE MALLOC: %-20s %-10d %-15s %-10d %s\n",
                   strftime("%H:%M:%S", nsecs), pid, comm, arg0, "malloc");
        }
    }
}

uprobe:libc:free {
    if (comm == "vexfs_fuse") {
        @frees[pid] = @frees[pid] + 1;
    }
}

interval:s:15 {
    printf("\n=== Memory Summary ===\n");
    print(@allocations);
    printf("\n");
}

END {
    clear(@allocations);
    clear(@frees);
}
EOF
            ;;
        performance)
            cat > "$script_path" << 'EOF'
#!/usr/bin/env bpftrace

BEGIN {
    printf("VexFS FUSE Performance Monitoring Started\n");
    printf("%-20s %-10s %-15s %-10s %s\n", "TIME", "PID", "COMM", "LATENCY", "FUNCTION");
}

uprobe:/*/vexfs_fuse:* {
    @start_time[pid, tid, func] = nsecs;
}

uretprobe:/*/vexfs_fuse:* {
    $start = @start_time[pid, tid, func];
    if ($start > 0) {
        $latency_us = (nsecs - $start) / 1000;
        @latencies[func] = hist($latency_us);
        
        if ($latency_us > 10000) {
            printf("🐌 SLOW OP: %-20s %-10d %-15s %-10d %s\n",
                   strftime("%H:%M:%S", nsecs), pid, comm, $latency_us, func);
        }
        
        delete(@start_time[pid, tid, func]);
    }
}

interval:s:20 {
    printf("\n=== Performance Summary ===\n");
    print(@latencies);
    printf("\n");
}

END {
    clear(@start_time);
    clear(@latencies);
}
EOF
            ;;
        comprehensive)
            cat > "$script_path" << 'EOF'
#!/usr/bin/env bpftrace

BEGIN {
    printf("VexFS FUSE Comprehensive Monitoring Started\n");
    printf("%-20s %-10s %-15s %-10s %-15s %s\n", 
           "TIME", "PID", "COMM", "VALUE", "TYPE", "DETAILS");
}

// Stack monitoring
uprobe:/*/vexfs_fuse:* {
    @stack_depth[pid] = @stack_depth[pid] + 1;
    $sp = reg("sp");
    $stack_size = (0x7fffffffffff - $sp) / 1024;
    
    if ($stack_size > 1024) {
        printf("STACK: %-20s %-10d %-15s %-10d %-15s %s\n", 
               strftime("%H:%M:%S", nsecs), pid, comm, $stack_size, "HIGH_USAGE", func);
    }
}

uretprobe:/*/vexfs_fuse:* {
    @stack_depth[pid] = @stack_depth[pid] - 1;
}

// Memory monitoring
uprobe:libc:malloc {
    if (comm == "vexfs_fuse" && arg0 > 1048576) {
        printf("MEMORY: %-20s %-10d %-15s %-10d %-15s %s\n",
               strftime("%H:%M:%S", nsecs), pid, comm, arg0, "LARGE_ALLOC", "malloc");
    }
}

// Performance monitoring
uprobe:/*/vexfs_fuse:* {
    @perf_start[pid, tid, func] = nsecs;
}

uretprobe:/*/vexfs_fuse:* {
    $start = @perf_start[pid, tid, func];
    if ($start > 0) {
        $latency_us = (nsecs - $start) / 1000;
        if ($latency_us > 10000) {
            printf("PERF: %-20s %-10d %-15s %-10d %-15s %s\n",
                   strftime("%H:%M:%S", nsecs), pid, comm, $latency_us, "SLOW_OP", func);
        }
        delete(@perf_start[pid, tid, func]);
    }
}

interval:s:30 {
    printf("\n=== Comprehensive Summary ===\n");
    print(@stack_depth);
    printf("\n");
}

END {
    clear(@stack_depth);
    clear(@perf_start);
}
EOF
            ;;
        *)
            error "Unknown trace type: $trace_type"
            return 1
            ;;
    esac
    
    chmod +x "$script_path"
    echo "$script_path"
}

# Start VexFS FUSE
start_vexfs_fuse() {
    local binary_path="$PROJECT_ROOT/target/profiling/vexfs_fuse"
    
    log "Starting VexFS FUSE..."
    
    # Start FUSE in background
    "$binary_path" "$MOUNT_POINT" &
    local fuse_pid=$!
    
    # Wait for FUSE to be ready
    local retry_count=0
    while ! mountpoint -q "$MOUNT_POINT" && [[ $retry_count -lt 30 ]]; do
        sleep 1
        retry_count=$((retry_count + 1))
    done
    
    if ! mountpoint -q "$MOUNT_POINT"; then
        error "FUSE mount failed to become ready"
        kill $fuse_pid 2>/dev/null || true
        return 1
    fi
    
    success "VexFS FUSE mounted successfully (PID: $fuse_pid)"
    echo "$fuse_pid"
}

# Create FUSE workload for tracing
create_fuse_workload() {
    local script_path="$RESULTS_DIR/fuse_workload_$(generate_timestamp).sh"
    
    log "Creating FUSE workload for tracing..."
    
    cat > "$script_path" << 'EOF'
#!/bin/bash
MOUNT_POINT="$1"
DURATION="$2"

log() {
    echo "[WORKLOAD] $1"
}

log "Starting FUSE workload on $MOUNT_POINT for ${DURATION}s"

mixed_operations() {
    local end_time=$(($(date +%s) + DURATION))
    local counter=0
    
    while [[ $(date +%s) -lt $end_time ]]; do
        echo "Test data $counter" > "$MOUNT_POINT/test_$counter.txt"
        cat "$MOUNT_POINT/test_$counter.txt" > /dev/null
        mkdir -p "$MOUNT_POINT/dir_$counter"
        ls -la "$MOUNT_POINT/dir_$counter" > /dev/null
        
        if [[ $((counter % 10)) -eq 0 ]]; then
            rm -f "$MOUNT_POINT/test_$((counter - 5)).txt" 2>/dev/null || true
            rmdir "$MOUNT_POINT/dir_$((counter - 5))" 2>/dev/null || true
        fi
        
        counter=$((counter + 1))
        sleep 0.1
    done
}

mixed_operations
log "FUSE workload completed with $counter operations"
EOF
    
    chmod +x "$script_path"
    echo "$script_path"
}

# Start real-time monitoring
start_realtime_monitoring() {
    local fuse_pid="$1"
    local output_prefix="$2"
    local monitor_file="${output_prefix}_realtime.log"
    
    log "Starting real-time monitoring..."
    
    while kill -0 $fuse_pid 2>/dev/null; do
        local timestamp
        timestamp=$(date +'%Y-%m-%d %H:%M:%S')
        
        if [[ -f "/proc/$fuse_pid/status" ]]; then
            local vmrss
            vmrss=$(grep VmRSS "/proc/$fuse_pid/status" | awk '{print $2}')
            echo "$timestamp,RSS:${vmrss}kB" >> "$monitor_file"
        fi
        
        sleep 1
    done
}

# Run eBPF tracing
run_ebpf_tracing() {
    local timestamp
    timestamp=$(generate_timestamp)
    local output_file="$RESULTS_DIR/${OUTPUT_PREFIX}_${timestamp}"
    local trace_script
    trace_script=$(create_fuse_trace_script "$TRACE_TYPE")
    local workload_script
    workload_script=$(create_fuse_workload)
    
    log "Starting eBPF FUSE tracing..."
    log "Trace type: $TRACE_TYPE"
    log "Output file: $output_file"
    log "Duration: ${DURATION}s"
    
    # Start VexFS FUSE
    local fuse_pid
    fuse_pid=$(start_vexfs_fuse)
    
    if [[ -z "$fuse_pid" ]]; then
        error "Failed to start VexFS FUSE"
        return 1
    fi
    
    # Start eBPF tracing
    log "Starting bpftrace script..."
    bpftrace "$trace_script" > "${output_file}.log" 2>&1 &
    local bpftrace_pid=$!
    
    sleep 2
    
    # Start real-time monitoring if requested
    if [[ "$REAL_TIME" == "true" ]]; then
        start_realtime_monitoring "$fuse_pid" "$output_file" &
        local monitor_pid=$!
    fi
    
    # Run workload
    log "Running FUSE workload..."
    bash "$workload_script" "$MOUNT_POINT" "$DURATION" || warning "Workload issues"
    
    # Wait for tracing duration
    log "Tracing for ${DURATION}s..."
    sleep "$DURATION"
    
    # Stop bpftrace
    log "Stopping eBPF tracing..."
    kill -INT $bpftrace_pid 2>/dev/null || true
    wait $bpftrace_pid 2>/dev/null || true
    
    # Stop real-time monitoring
    if [[ "$REAL_TIME" == "true" ]] && [[ -n "${monitor_pid:-}" ]]; then
        kill $monitor_pid 2>/dev/null || true
    fi
    
    # Cleanup
    log "Cleaning up..."
    if mountpoint -q "$MOUNT_POINT"; then
        fusermount -u "$MOUNT_POINT"
        log "Unmounted FUSE filesystem"
    fi
    
    # Generate summary
    generate_summary_report "$output_file"
    
    success "eBPF tracing completed"
}

# Generate summary report
generate_summary_report() {
    local output_prefix="$1"
    local summary_file="${output_prefix}_summary.txt"
    
    log "Generating summary report..."
    
    cat > "$summary_file" << EOF
VexFS FUSE eBPF Tracing Summary
==============================

Analysis Date: $(date)
Trace Type: $TRACE_TYPE
Duration: ${DURATION}s
Mount Point: $MOUNT_POINT

Files Generated:
- Trace Log: ${output_prefix}.log
- Summary: ${summary_file}

EOF
    
    if [[ -f "${output_prefix}.log" ]]; then
        echo "=== Key Events ===" >> "$summary_file"
        grep -E "(STACK|MEMORY|PERF|⚠️|🧠|🐌)" "${output_prefix}.log" | head -20 >> "$summary_file" 2>/dev/null || true
    fi
    
    success "Summary report generated: $summary_file"
    
    if [[ "$VERBOSE" == "true" ]]; then
        echo ""
        log "Tracing Summary:"
        cat "$summary_file"
    fi
}

# Main execution
main() {
    log "Starting VexFS FUSE eBPF Tracing"
    
    parse_args "$@"
    validate_environment
    run_ebpf_tracing
    
    success "eBPF tracing completed"
    log "Results available in: $RESULTS_DIR"
}

# Execute main function
main "$@"