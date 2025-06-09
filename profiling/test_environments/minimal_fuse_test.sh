#!/bin/bash

# VexFS FUSE Minimal Baseline Test
# Task 23.1: Establish baseline measurements for current minimal FUSE implementation

set -euo pipefail

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROFILING_DIR="$(dirname "$SCRIPT_DIR")"
PROJECT_ROOT="$(dirname "$PROFILING_DIR")"
RESULTS_DIR="$PROFILING_DIR/results/baseline"

# Default parameters
DURATION=60
MOUNT_POINT="/tmp/vexfs_baseline_test"
OUTPUT_PREFIX="minimal_fuse_baseline"
VERBOSE=false

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

VexFS FUSE Minimal Baseline Test

OPTIONS:
    -d, --duration SECONDS     Test duration (default: 60)
    -m, --mount-point PATH     FUSE mount point (default: /tmp/vexfs_baseline_test)
    -o, --output PREFIX        Output file prefix (default: minimal_fuse_baseline)
    -v, --verbose              Enable verbose output
    -h, --help                 Show this help message

DESCRIPTION:
    This script establishes baseline measurements for the current minimal FUSE
    implementation. It measures:
    - Basic stack usage patterns
    - Memory allocation baseline
    - Performance characteristics
    - Resource utilization

EXAMPLES:
    $0                                          # Basic baseline test
    $0 -d 120 -v                              # 2-minute test with verbose output
    $0 -m /mnt/baseline -o custom_baseline     # Custom mount point and output

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
            -o|--output)
                OUTPUT_PREFIX="$2"
                shift 2
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
    log "Validating environment for baseline test..."
    
    # Check if VexFS FUSE binary exists
    local binary_path="$PROJECT_ROOT/target/profiling/vexfs_fuse"
    if [[ ! -f "$binary_path" ]]; then
        error "VexFS FUSE binary not found. Please run setup_profiling_environment.sh first."
        exit 1
    fi
    
    # Check if results directory exists
    if [[ ! -d "$RESULTS_DIR" ]]; then
        mkdir -p "$RESULTS_DIR"
        log "Created baseline results directory: $RESULTS_DIR"
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

# Create minimal workload for baseline
create_baseline_workload() {
    local script_path="$RESULTS_DIR/baseline_workload_$(generate_timestamp).sh"
    
    log "Creating minimal baseline workload..."
    
    cat > "$script_path" << 'EOF'
#!/bin/bash
# Minimal workload for baseline measurements

MOUNT_POINT="$1"
DURATION="$2"

log() {
    echo "[BASELINE] $1"
}

log "Starting minimal baseline workload on $MOUNT_POINT for ${DURATION}s"

# Basic filesystem operations to establish baseline
basic_operations() {
    local end_time=$(($(date +%s) + DURATION))
    local operation_count=0
    
    while [[ $(date +%s) -lt $end_time ]]; do
        # Simple file creation
        echo "Baseline test data $operation_count" > "$MOUNT_POINT/baseline_$operation_count.txt"
        
        # Simple file read
        cat "$MOUNT_POINT/baseline_$operation_count.txt" > /dev/null
        
        # Simple directory listing
        ls -la "$MOUNT_POINT" > /dev/null
        
        # Simple file metadata access
        stat "$MOUNT_POINT/baseline_$operation_count.txt" > /dev/null
        
        # Simple file deletion (every 5th file to maintain some files)
        if [[ $((operation_count % 5)) -eq 0 ]] && [[ $operation_count -gt 0 ]]; then
            rm -f "$MOUNT_POINT/baseline_$((operation_count - 1)).txt" 2>/dev/null || true
        fi
        
        operation_count=$((operation_count + 1))
        
        # Gentle pacing to avoid overwhelming the minimal implementation
        sleep 0.2
        
        if [[ $((operation_count % 50)) -eq 0 ]]; then
            log "Completed $operation_count baseline operations"
        fi
    done
    
    log "Baseline workload completed with $operation_count operations"
}

# Execute baseline operations
basic_operations
EOF
    
    chmod +x "$script_path"
    echo "$script_path"
}

# Start VexFS FUSE for baseline
start_baseline_fuse() {
    local binary_path="$PROJECT_ROOT/target/profiling/vexfs_fuse"
    
    log "Starting VexFS FUSE for baseline test..."
    
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
    
    success "VexFS FUSE mounted successfully for baseline (PID: $fuse_pid)"
    echo "$fuse_pid"
}

# Monitor baseline metrics
monitor_baseline_metrics() {
    local fuse_pid="$1"
    local output_file="$2"
    local metrics_file="${output_file}_metrics.log"
    
    log "Starting baseline metrics monitoring..."
    
    # Monitor basic system metrics
    {
        echo "# VexFS FUSE Baseline Metrics"
        echo "# Timestamp,RSS_KB,VSZ_KB,CPU_PERCENT,FD_COUNT"
        
        while kill -0 $fuse_pid 2>/dev/null; do
            local timestamp
            timestamp=$(date +'%Y-%m-%d %H:%M:%S')
            
            if [[ -f "/proc/$fuse_pid/status" ]]; then
                local vmrss vmsize
                vmrss=$(grep VmRSS "/proc/$fuse_pid/status" | awk '{print $2}')
                vmsize=$(grep VmSize "/proc/$fuse_pid/status" | awk '{print $2}')
                
                # Get CPU usage
                local cpu_percent
                cpu_percent=$(ps -p $fuse_pid -o %cpu --no-headers 2>/dev/null || echo "0.0")
                
                # Get file descriptor count
                local fd_count
                fd_count=$(ls -1 "/proc/$fuse_pid/fd" 2>/dev/null | wc -l || echo "0")
                
                echo "$timestamp,$vmrss,$vmsize,$cpu_percent,$fd_count"
            fi
            
            sleep 2
        done
    } > "$metrics_file" &
    
    echo $!
}

# Run baseline test
run_baseline_test() {
    local timestamp
    timestamp=$(generate_timestamp)
    local output_file="$RESULTS_DIR/${OUTPUT_PREFIX}_${timestamp}"
    local workload_script
    workload_script=$(create_baseline_workload)
    
    log "Starting minimal FUSE baseline test..."
    log "Output file: $output_file"
    log "Duration: ${DURATION}s"
    
    # Start VexFS FUSE
    local fuse_pid
    fuse_pid=$(start_baseline_fuse)
    
    if [[ -z "$fuse_pid" ]]; then
        error "Failed to start VexFS FUSE for baseline"
        return 1
    fi
    
    # Start metrics monitoring
    local monitor_pid
    monitor_pid=$(monitor_baseline_metrics "$fuse_pid" "$output_file")
    
    # Record initial system state
    log "Recording initial system state..."
    {
        echo "=== Initial System State ==="
        echo "Date: $(date)"
        echo "Kernel: $(uname -r)"
        echo "Memory: $(free -h)"
        echo "Load: $(uptime)"
        echo "VexFS PID: $fuse_pid"
        echo ""
    } > "${output_file}_system_state.log"
    
    # Run baseline workload
    log "Running baseline workload..."
    if ! bash "$workload_script" "$MOUNT_POINT" "$DURATION"; then
        warning "Baseline workload encountered issues"
    fi
    
    # Wait for test duration
    log "Monitoring baseline for ${DURATION}s..."
    sleep "$DURATION"
    
    # Stop monitoring
    log "Stopping baseline monitoring..."
    kill $monitor_pid 2>/dev/null || true
    
    # Record final system state
    {
        echo "=== Final System State ==="
        echo "Date: $(date)"
        echo "Memory: $(free -h)"
        echo "Load: $(uptime)"
        echo ""
    } >> "${output_file}_system_state.log"
    
    # Cleanup
    log "Cleaning up baseline test..."
    if mountpoint -q "$MOUNT_POINT"; then
        fusermount -u "$MOUNT_POINT"
        log "Unmounted FUSE filesystem"
    fi
    
    # Generate baseline analysis
    generate_baseline_analysis "$output_file"
    
    success "Baseline test completed"
}

# Generate baseline analysis
generate_baseline_analysis() {
    local output_prefix="$1"
    local analysis_file="${output_prefix}_analysis.txt"
    
    log "Generating baseline analysis..."
    
    cat > "$analysis_file" << EOF
VexFS FUSE Minimal Baseline Analysis
===================================

Test Date: $(date)
Duration: ${DURATION}s
Mount Point: $MOUNT_POINT

=== Test Configuration ===
- Implementation: Minimal FUSE (vector components disabled)
- Build Profile: profiling (debug symbols, light optimization)
- Test Type: Basic filesystem operations

=== Files Generated ===
- Metrics: ${output_prefix}_metrics.log
- System State: ${output_prefix}_system_state.log
- Analysis: ${analysis_file}

EOF
    
    # Analyze metrics if available
    if [[ -f "${output_prefix}_metrics.log" ]]; then
        echo "=== Resource Usage Analysis ===" >> "$analysis_file"
        
        # Calculate basic statistics from metrics
        local metrics_file="${output_prefix}_metrics.log"
        
        # Skip header and calculate stats
        if [[ $(wc -l < "$metrics_file") -gt 1 ]]; then
            local avg_rss max_rss avg_cpu max_cpu
            
            # RSS statistics (KB)
            avg_rss=$(tail -n +2 "$metrics_file" | awk -F, '{sum+=$2; count++} END {if(count>0) print int(sum/count); else print 0}')
            max_rss=$(tail -n +2 "$metrics_file" | awk -F, '{if($2>max) max=$2} END {print max+0}')
            
            # CPU statistics
            avg_cpu=$(tail -n +2 "$metrics_file" | awk -F, '{sum+=$4; count++} END {if(count>0) printf "%.1f", sum/count; else print 0}')
            max_cpu=$(tail -n +2 "$metrics_file" | awk -F, '{if($4>max) max=$4} END {printf "%.1f", max+0}')
            
            echo "Memory Usage (RSS):" >> "$analysis_file"
            echo "  Average: ${avg_rss} KB" >> "$analysis_file"
            echo "  Peak: ${max_rss} KB" >> "$analysis_file"
            echo "" >> "$analysis_file"
            echo "CPU Usage:" >> "$analysis_file"
            echo "  Average: ${avg_cpu}%" >> "$analysis_file"
            echo "  Peak: ${max_cpu}%" >> "$analysis_file"
            echo "" >> "$analysis_file"
            
            # Calculate baseline scores
            echo "=== Baseline Scores ===" >> "$analysis_file"
            echo "Memory Efficiency: $(( 10000 / (avg_rss + 1) )) (higher is better)" >> "$analysis_file"
            echo "CPU Efficiency: $(( 1000 / (${avg_cpu%.*} + 1) )) (higher is better)" >> "$analysis_file"
        else
            echo "Insufficient metrics data for analysis" >> "$analysis_file"
        fi
    fi
    
    # Add recommendations
    cat >> "$analysis_file" << EOF

=== Baseline Characteristics ===
This baseline represents the minimal FUSE implementation with:
- No vector storage components
- No search engine components
- Basic file operations only
- Minimal memory allocations

=== Next Steps ===
1. Use this baseline for comparison when re-enabling components
2. Monitor for stack usage increases above baseline levels
3. Track memory allocation patterns as components are added
4. Identify performance regressions during component integration

=== Component Re-enablement Strategy ===
Based on this baseline, components should be re-enabled in order:
1. Basic vector storage (without search)
2. Metadata management
3. Search engine components
4. Advanced vector operations

Each step should be compared against this baseline to identify
the specific component causing stack overflow issues.
EOF
    
    success "Baseline analysis generated: $analysis_file"
    
    # Display summary
    if [[ "$VERBOSE" == "true" ]]; then
        echo ""
        log "Baseline Analysis Summary:"
        cat "$analysis_file"
    else
        echo ""
        log "Baseline Test Summary:"
        echo "  Results: $RESULTS_DIR"
        echo "  Analysis: $analysis_file"
        if [[ -f "${output_prefix}_metrics.log" ]]; then
            local avg_rss
            avg_rss=$(tail -n +2 "${output_prefix}_metrics.log" | awk -F, '{sum+=$2; count++} END {if(count>0) print int(sum/count); else print 0}')
            echo "  Average Memory: ${avg_rss} KB"
        fi
    fi
}

# Main execution
main() {
    log "Starting VexFS FUSE Minimal Baseline Test"
    
    parse_args "$@"
    validate_environment
    run_baseline_test
    
    success "Minimal baseline test completed"
    log "Baseline results available in: $RESULTS_DIR"
    log "Use these results as comparison baseline for component re-enablement"
}

# Execute main function
main "$@"