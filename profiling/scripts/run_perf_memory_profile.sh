#!/bin/bash

# VexFS FUSE Perf Memory Profiling Script
# Task 23.1: Performance-oriented memory usage analysis

set -euo pipefail

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROFILING_DIR="$(dirname "$SCRIPT_DIR")"
PROJECT_ROOT="$(dirname "$PROFILING_DIR")"
RESULTS_DIR="$PROFILING_DIR/results/perf"
CONFIG_FILE="$PROFILING_DIR/configs/perf_memory_profile.conf"

# Default parameters
DURATION=300
MOUNT_POINT="/tmp/vexfs_profiling_mount"
FREQUENCY=1000
OUTPUT_PREFIX="vexfs_fuse_memory_profile"
VERBOSE=false
REAL_TIME=false

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

VexFS FUSE Perf Memory Profiling

OPTIONS:
    -d, --duration SECONDS     Profiling duration (default: 300)
    -f, --frequency HZ         Sampling frequency (default: 1000)
    -m, --mount-point PATH     FUSE mount point (default: /tmp/vexfs_profiling_mount)
    -o, --output PREFIX        Output file prefix (default: vexfs_fuse_memory_profile)
    -r, --real-time            Enable real-time monitoring
    -v, --verbose              Enable verbose output
    -h, --help                 Show this help message

EXAMPLES:
    $0                                          # Basic memory profiling
    $0 -d 600 -f 2000                         # 10-minute profiling with higher frequency
    $0 -r -v                                   # Real-time monitoring with verbose output
    $0 -m /mnt/test -o custom_profile         # Custom mount point and output

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
            -f|--frequency)
                FREQUENCY="$2"
                shift 2
                ;;
            -m|--mount-point)
                MOUNT_POINT="$2"
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
    
    # Check if perf is available
    if ! command -v perf &> /dev/null; then
        error "Perf not found. Please install linux-tools-\$(uname -r)."
        exit 1
    fi
    
    # Check perf permissions
    if ! perf list &> /dev/null; then
        error "Perf requires elevated privileges. Please run with sudo or configure perf_event_paranoid."
        exit 1
    fi
    
    # Check if VexFS FUSE binary exists
    local binary_path="$PROJECT_ROOT/target/profiling/vexfs_fuse"
    if [[ ! -f "$binary_path" ]]; then
        error "VexFS FUSE binary not found. Please run setup_profiling_environment.sh first."
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

# Create memory stress workload
create_memory_workload() {
    local script_path="$RESULTS_DIR/memory_workload_$(generate_timestamp).sh"
    
    log "Creating memory stress workload..."
    
    cat > "$script_path" << 'EOF'
#!/bin/bash
# Memory-focused workload for perf analysis

MOUNT_POINT="$1"
DURATION="$2"

log() {
    echo "[WORKLOAD] $1"
}

log "Starting memory workload on $MOUNT_POINT for ${DURATION}s"

# Memory allocation patterns
create_large_files() {
    local size_mb="$1"
    local count="$2"
    
    for i in $(seq 1 "$count"); do
        dd if=/dev/zero of="$MOUNT_POINT/large_file_${size_mb}mb_$i.dat" bs=1M count="$size_mb" 2>/dev/null
        log "Created ${size_mb}MB file $i"
    done
}

# Variable-size file operations
create_variable_files() {
    local base_size=1
    
    for i in $(seq 1 20); do
        local size=$((base_size * i))
        dd if=/dev/urandom of="$MOUNT_POINT/var_file_${size}kb.dat" bs=1K count="$size" 2>/dev/null
        
        # Read the file back (memory pressure)
        cat "$MOUNT_POINT/var_file_${size}kb.dat" > /dev/null
    done
}

# Memory fragmentation test
fragmentation_test() {
    # Create many small files
    for i in $(seq 1 1000); do
        echo "Small file content $i" > "$MOUNT_POINT/small_$i.txt"
    done
    
    # Delete every other file
    for i in $(seq 2 2 1000); do
        rm -f "$MOUNT_POINT/small_$i.txt"
    done
    
    # Create medium files in gaps
    for i in $(seq 2 2 1000); do
        dd if=/dev/zero of="$MOUNT_POINT/medium_$i.dat" bs=1K count=10 2>/dev/null
    done
}

# Concurrent memory operations
concurrent_operations() {
    # Start background operations
    for i in $(seq 1 5); do
        (
            while [[ -f "$MOUNT_POINT/.continue_test" ]]; do
                echo "Background data $i $(date)" >> "$MOUNT_POINT/bg_$i.log"
                sleep 0.1
            done
        ) &
    done
    
    # Signal to continue
    touch "$MOUNT_POINT/.continue_test"
    
    # Main operations
    create_large_files 5 3
    create_variable_files
    fragmentation_test
    
    # Stop background operations
    rm -f "$MOUNT_POINT/.continue_test"
    wait
}

# Execute workload phases
log "Phase 1: Large file creation"
create_large_files 10 2

log "Phase 2: Variable size files"
create_variable_files

log "Phase 3: Memory fragmentation test"
fragmentation_test

log "Phase 4: Concurrent operations"
concurrent_operations

log "Phase 5: Memory pressure test"
# Create files until we hit memory pressure
counter=0
while [[ $counter -lt 50 ]]; do
    if ! dd if=/dev/zero of="$MOUNT_POINT/pressure_$counter.dat" bs=1M count=5 2>/dev/null; then
        log "Memory pressure detected at file $counter"
        break
    fi
    counter=$((counter + 1))
done

log "Memory workload completed"
EOF
    
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

# Run perf memory profiling
run_perf_profiling() {
    local timestamp
    timestamp=$(generate_timestamp)
    local output_file="$RESULTS_DIR/${OUTPUT_PREFIX}_${timestamp}"
    local workload_script
    workload_script=$(create_memory_workload)
    
    log "Starting perf memory profiling..."
    log "Output file: $output_file"
    log "Duration: ${DURATION}s"
    log "Frequency: ${FREQUENCY}Hz"
    
    # Start VexFS FUSE
    local fuse_pid
    fuse_pid=$(start_vexfs_fuse)
    
    if [[ -z "$fuse_pid" ]]; then
        error "Failed to start VexFS FUSE"
        return 1
    fi
    
    # Prepare perf command
    local perf_events=(
        "cycles"
        "instructions"
        "cache-references"
        "cache-misses"
        "page-faults"
        "context-switches"
        "cpu-migrations"
        "L1-dcache-loads"
        "L1-dcache-load-misses"
        "LLC-loads"
        "LLC-load-misses"
        "branch-loads"
        "branch-load-misses"
    )
    
    # Add memory-specific events
    local memory_events=(
        "mem-loads"
        "mem-stores"
        "dTLB-loads"
        "dTLB-load-misses"
        "iTLB-loads"
        "iTLB-load-misses"
    )
    
    # Combine events
    local all_events="${perf_events[*]},${memory_events[*]}"
    all_events=$(echo "$all_events" | tr ' ' ',')
    
    # Build perf record command
    local perf_cmd="perf record"
    perf_cmd="$perf_cmd -e $all_events"
    perf_cmd="$perf_cmd -F $FREQUENCY"
    perf_cmd="$perf_cmd --call-graph=dwarf,16384"
    perf_cmd="$perf_cmd -p $fuse_pid"
    perf_cmd="$perf_cmd -o ${output_file}.data"
    
    if [[ "$VERBOSE" == "true" ]]; then
        log "Perf command: $perf_cmd"
    fi
    
    # Start perf recording
    log "Starting perf recording..."
    eval "$perf_cmd" &
    local perf_pid=$!
    
    # Start real-time monitoring if requested
    if [[ "$REAL_TIME" == "true" ]]; then
        start_realtime_monitoring "$fuse_pid" "$output_file" &
        local monitor_pid=$!
    fi
    
    # Run workload
    log "Running memory workload..."
    if ! bash "$workload_script" "$MOUNT_POINT" "$DURATION"; then
        warning "Workload execution encountered issues"
    fi
    
    # Wait for profiling duration
    log "Profiling for ${DURATION}s..."
    sleep "$DURATION"
    
    # Stop perf recording
    log "Stopping perf recording..."
    kill -INT $perf_pid 2>/dev/null || true
    wait $perf_pid 2>/dev/null || true
    
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
    
    # Generate reports
    generate_perf_reports "$output_file"
    
    success "Perf memory profiling completed"
}

# Start real-time monitoring
start_realtime_monitoring() {
    local fuse_pid="$1"
    local output_prefix="$2"
    local monitor_file="${output_prefix}_realtime.log"
    
    log "Starting real-time monitoring..."
    
    # Monitor memory usage, CPU usage, and I/O
    while kill -0 $fuse_pid 2>/dev/null; do
        local timestamp
        timestamp=$(date +'%Y-%m-%d %H:%M:%S')
        
        # Get process statistics
        if [[ -f "/proc/$fuse_pid/status" ]]; then
            local vmrss
            local vmsize
            vmrss=$(grep VmRSS "/proc/$fuse_pid/status" | awk '{print $2}')
            vmsize=$(grep VmSize "/proc/$fuse_pid/status" | awk '{print $2}')
            
            echo "$timestamp,RSS:${vmrss}kB,VSIZE:${vmsize}kB" >> "$monitor_file"
        fi
        
        sleep 1
    done
}

# Generate perf reports
generate_perf_reports() {
    local output_prefix="$1"
    local data_file="${output_prefix}.data"
    
    log "Generating perf reports..."
    
    if [[ ! -f "$data_file" ]]; then
        error "Perf data file not found: $data_file"
        return 1
    fi
    
    # Generate text report
    log "Generating text report..."
    perf report -i "$data_file" --stdio > "${output_prefix}_report.txt" 2>/dev/null || true
    
    # Generate annotated report
    log "Generating annotated report..."
    perf annotate -i "$data_file" --stdio > "${output_prefix}_annotate.txt" 2>/dev/null || true
    
    # Generate memory-specific analysis
    log "Generating memory analysis..."
    perf mem report -i "$data_file" --stdio > "${output_prefix}_memory.txt" 2>/dev/null || true
    
    # Generate call graph
    log "Generating call graph..."
    perf report -i "$data_file" --call-graph=graph,0.5,caller --stdio > "${output_prefix}_callgraph.txt" 2>/dev/null || true
    
    # Generate statistics
    log "Generating statistics..."
    perf stat -i "$data_file" > "${output_prefix}_stats.txt" 2>&1 || true
    
    # Generate summary
    generate_summary_report "$output_prefix"
    
    success "Perf reports generated"
}

# Generate summary report
generate_summary_report() {
    local output_prefix="$1"
    local summary_file="${output_prefix}_summary.txt"
    
    log "Generating summary report..."
    
    cat > "$summary_file" << EOF
VexFS FUSE Perf Memory Profiling Summary
========================================

Analysis Date: $(date)
Duration: ${DURATION}s
Sampling Frequency: ${FREQUENCY}Hz
Mount Point: $MOUNT_POINT

Files Generated:
- Raw Data: ${output_prefix}.data
- Text Report: ${output_prefix}_report.txt
- Annotated Report: ${output_prefix}_annotate.txt
- Memory Analysis: ${output_prefix}_memory.txt
- Call Graph: ${output_prefix}_callgraph.txt
- Statistics: ${output_prefix}_stats.txt
- Summary: ${summary_file}

EOF
    
    # Extract key metrics from reports
    if [[ -f "${output_prefix}_stats.txt" ]]; then
        echo "=== Performance Statistics ===" >> "$summary_file"
        grep -E "(cycles|instructions|cache-misses|page-faults)" "${output_prefix}_stats.txt" >> "$summary_file" 2>/dev/null || true
    fi
    
    if [[ -f "${output_prefix}_memory.txt" ]]; then
        echo "" >> "$summary_file"
        echo "=== Memory Analysis ===" >> "$summary_file"
        head -20 "${output_prefix}_memory.txt" >> "$summary_file" 2>/dev/null || true
    fi
    
    if [[ -f "${output_prefix}_realtime.log" ]]; then
        echo "" >> "$summary_file"
        echo "=== Real-time Memory Usage ===" >> "$summary_file"
        echo "Peak RSS: $(sort -t, -k2 -nr "${output_prefix}_realtime.log" | head -1)" >> "$summary_file"
        echo "Final RSS: $(tail -1 "${output_prefix}_realtime.log")" >> "$summary_file"
    fi
    
    success "Summary report generated: $summary_file"
    
    # Display summary
    if [[ "$VERBOSE" == "true" ]]; then
        echo ""
        log "Profiling Summary:"
        cat "$summary_file"
    fi
}

# Main execution
main() {
    log "Starting VexFS FUSE Perf Memory Profiling"
    
    parse_args "$@"
    validate_environment
    run_perf_profiling
    
    success "Perf memory profiling completed"
    log "Results available in: $RESULTS_DIR"
}

# Execute main function
main "$@"