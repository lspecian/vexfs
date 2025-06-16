#!/bin/bash

# VexFS FUSE Valgrind Stack Analysis Script
# Task 23.1: Deep stack usage analysis and overflow detection

set -euo pipefail

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROFILING_DIR="$(dirname "$SCRIPT_DIR")"
PROJECT_ROOT="$(dirname "$PROFILING_DIR")"
RESULTS_DIR="$PROFILING_DIR/results/valgrind"
CONFIG_FILE="$PROFILING_DIR/configs/valgrind_stack_analysis.conf"

# Default parameters
DURATION=300
MOUNT_POINT="/tmp/vexfs_profiling_mount"
TEST_WORKLOAD="basic"
OUTPUT_PREFIX="vexfs_fuse_stack_analysis"
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

VexFS FUSE Valgrind Stack Analysis

OPTIONS:
    -d, --duration SECONDS     Analysis duration (default: 300)
    -m, --mount-point PATH     FUSE mount point (default: /tmp/vexfs_profiling_mount)
    -w, --workload TYPE        Test workload type (basic|stress|recursive) (default: basic)
    -o, --output PREFIX        Output file prefix (default: vexfs_fuse_stack_analysis)
    -v, --verbose              Enable verbose output
    -h, --help                 Show this help message

WORKLOAD TYPES:
    basic       - Basic filesystem operations
    stress      - High-load stress testing
    recursive   - Deep recursion testing for stack overflow detection

EXAMPLES:
    $0                                          # Basic analysis
    $0 -d 600 -w stress                        # 10-minute stress test
    $0 -w recursive -v                         # Recursive workload with verbose output
    $0 -m /mnt/test -o custom_analysis         # Custom mount point and output

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
            -w|--workload)
                TEST_WORKLOAD="$2"
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
    log "Validating environment..."
    
    # Check if valgrind is available
    if ! command -v valgrind &> /dev/null; then
        error "Valgrind not found. Please install valgrind."
        exit 1
    fi
    
    # Check if VexFS FUSE binary exists
    local binary_path="$PROJECT_ROOT/target/profiling/vexfs_fuse"
    if [[ ! -f "$binary_path" ]]; then
        error "VexFS FUSE binary not found. Please run setup_profiling_environment.sh first."
        exit 1
    fi
    
    # Check if configuration file exists
    if [[ ! -f "$CONFIG_FILE" ]]; then
        error "Valgrind configuration file not found: $CONFIG_FILE"
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

# Create test workload script
create_workload_script() {
    local workload_type="$1"
    local script_path="$RESULTS_DIR/workload_${workload_type}_$(generate_timestamp).sh"
    
    log "Creating $workload_type workload script..."
    
    case "$workload_type" in
        basic)
            cat > "$script_path" << 'EOF'
#!/bin/bash
# Basic FUSE workload for stack analysis

MOUNT_POINT="$1"
DURATION="$2"

log() {
    echo "[WORKLOAD] $1"
}

log "Starting basic workload on $MOUNT_POINT for ${DURATION}s"

# Basic file operations
for i in $(seq 1 100); do
    echo "Test data $i" > "$MOUNT_POINT/test_file_$i.txt"
    cat "$MOUNT_POINT/test_file_$i.txt" > /dev/null
    ls -la "$MOUNT_POINT" > /dev/null
done

# Directory operations
for i in $(seq 1 10); do
    mkdir -p "$MOUNT_POINT/test_dir_$i"
    touch "$MOUNT_POINT/test_dir_$i/nested_file.txt"
    echo "Nested content" > "$MOUNT_POINT/test_dir_$i/nested_file.txt"
done

# Read operations
find "$MOUNT_POINT" -type f -exec cat {} \; > /dev/null

log "Basic workload completed"
EOF
            ;;
        stress)
            cat > "$script_path" << 'EOF'
#!/bin/bash
# Stress workload for stack analysis

MOUNT_POINT="$1"
DURATION="$2"

log() {
    echo "[WORKLOAD] $1"
}

log "Starting stress workload on $MOUNT_POINT for ${DURATION}s"

# High-frequency operations
end_time=$(($(date +%s) + DURATION))
counter=0

while [[ $(date +%s) -lt $end_time ]]; do
    # Rapid file creation and deletion
    echo "Stress test data $counter" > "$MOUNT_POINT/stress_$counter.txt"
    cat "$MOUNT_POINT/stress_$counter.txt" > /dev/null
    rm "$MOUNT_POINT/stress_$counter.txt"
    
    # Directory operations
    mkdir -p "$MOUNT_POINT/stress_dir_$counter"
    rmdir "$MOUNT_POINT/stress_dir_$counter"
    
    # Metadata operations
    ls -la "$MOUNT_POINT" > /dev/null
    
    counter=$((counter + 1))
    
    if [[ $((counter % 100)) -eq 0 ]]; then
        log "Completed $counter stress operations"
    fi
done

log "Stress workload completed with $counter operations"
EOF
            ;;
        recursive)
            cat > "$script_path" << 'EOF'
#!/bin/bash
# Recursive workload for deep stack analysis

MOUNT_POINT="$1"
DURATION="$2"

log() {
    echo "[WORKLOAD] $1"
}

log "Starting recursive workload on $MOUNT_POINT for ${DURATION}s"

# Create deep directory structure
create_deep_dirs() {
    local base_path="$1"
    local depth="$2"
    local max_depth="$3"
    
    if [[ $depth -ge $max_depth ]]; then
        return
    fi
    
    local dir_path="$base_path/level_$depth"
    mkdir -p "$dir_path"
    echo "Deep content at level $depth" > "$dir_path/content.txt"
    
    # Recursive call
    create_deep_dirs "$dir_path" $((depth + 1)) $max_depth
}

# Create nested directory structure (potential stack stress)
create_deep_dirs "$MOUNT_POINT/deep_structure" 0 50

# Recursive file operations
traverse_and_process() {
    local dir_path="$1"
    local depth="$2"
    
    if [[ $depth -gt 30 ]]; then
        return
    fi
    
    for item in "$dir_path"/*; do
        if [[ -d "$item" ]]; then
            traverse_and_process "$item" $((depth + 1))
        elif [[ -f "$item" ]]; then
            cat "$item" > /dev/null
        fi
    done
}

# Process the deep structure
traverse_and_process "$MOUNT_POINT/deep_structure" 0

log "Recursive workload completed"
EOF
            ;;
        *)
            error "Unknown workload type: $workload_type"
            exit 1
            ;;
    esac
    
    chmod +x "$script_path"
    echo "$script_path"
}

# Run Valgrind analysis
run_valgrind_analysis() {
    local timestamp
    timestamp=$(generate_timestamp)
    local output_file="$RESULTS_DIR/${OUTPUT_PREFIX}_${timestamp}"
    local workload_script
    workload_script=$(create_workload_script "$TEST_WORKLOAD")
    
    log "Starting Valgrind stack analysis..."
    log "Output file: $output_file"
    log "Workload: $TEST_WORKLOAD"
    log "Duration: ${DURATION}s"
    
    # Prepare Valgrind command
    local valgrind_cmd="valgrind"
    
    # Add configuration file options
    if [[ -f "$CONFIG_FILE" ]]; then
        while IFS= read -r line; do
            # Skip comments and empty lines
            if [[ ! "$line" =~ ^[[:space:]]*# ]] && [[ -n "$line" ]]; then
                valgrind_cmd="$valgrind_cmd $line"
            fi
        done < "$CONFIG_FILE"
    fi
    
    # Override output files with our naming
    valgrind_cmd="$valgrind_cmd --xml-file=${output_file}.xml"
    valgrind_cmd="$valgrind_cmd --log-file=${output_file}.log"
    
    # Add the VexFS FUSE binary and mount point
    local binary_path="$PROJECT_ROOT/target/profiling/vexfs_fuse"
    valgrind_cmd="$valgrind_cmd $binary_path $MOUNT_POINT"
    
    if [[ "$VERBOSE" == "true" ]]; then
        log "Valgrind command: $valgrind_cmd"
    fi
    
    # Start Valgrind in background
    log "Starting VexFS FUSE under Valgrind..."
    eval "$valgrind_cmd" &
    local valgrind_pid=$!
    
    # Wait for FUSE to be ready
    local retry_count=0
    while ! mountpoint -q "$MOUNT_POINT" && [[ $retry_count -lt 30 ]]; do
        sleep 1
        retry_count=$((retry_count + 1))
    done
    
    if ! mountpoint -q "$MOUNT_POINT"; then
        error "FUSE mount failed to become ready"
        kill $valgrind_pid 2>/dev/null || true
        return 1
    fi
    
    success "VexFS FUSE mounted successfully under Valgrind"
    
    # Run workload
    log "Running $TEST_WORKLOAD workload..."
    if ! bash "$workload_script" "$MOUNT_POINT" "$DURATION"; then
        warning "Workload execution encountered issues"
    fi
    
    # Additional monitoring during execution
    log "Monitoring Valgrind process for ${DURATION}s..."
    sleep "$DURATION"
    
    # Cleanup
    log "Cleaning up..."
    if mountpoint -q "$MOUNT_POINT"; then
        fusermount -u "$MOUNT_POINT"
        log "Unmounted FUSE filesystem"
    fi
    
    # Wait for Valgrind to finish and generate reports
    wait $valgrind_pid
    local valgrind_exit_code=$?
    
    if [[ $valgrind_exit_code -eq 0 ]]; then
        success "Valgrind analysis completed successfully"
    else
        warning "Valgrind exited with code: $valgrind_exit_code"
    fi
    
    # Generate summary
    generate_analysis_summary "$output_file"
    
    return $valgrind_exit_code
}

# Generate analysis summary
generate_analysis_summary() {
    local output_file="$1"
    local summary_file="${output_file}_summary.txt"
    
    log "Generating analysis summary..."
    
    cat > "$summary_file" << EOF
VexFS FUSE Valgrind Stack Analysis Summary
==========================================

Analysis Date: $(date)
Workload Type: $TEST_WORKLOAD
Duration: ${DURATION}s
Mount Point: $MOUNT_POINT

Files Generated:
- Log File: ${output_file}.log
- XML Report: ${output_file}.xml
- Summary: ${summary_file}

EOF
    
    # Extract key information from Valgrind log
    if [[ -f "${output_file}.log" ]]; then
        echo "=== Error Summary ===" >> "$summary_file"
        grep -E "(ERROR SUMMARY|LEAK SUMMARY)" "${output_file}.log" >> "$summary_file" 2>/dev/null || true
        
        echo "" >> "$summary_file"
        echo "=== Memory Usage ===" >> "$summary_file"
        grep -E "(total heap usage|definitely lost|indirectly lost|possibly lost)" "${output_file}.log" >> "$summary_file" 2>/dev/null || true
        
        echo "" >> "$summary_file"
        echo "=== Stack Information ===" >> "$summary_file"
        grep -E "(stack|Stack)" "${output_file}.log" | head -20 >> "$summary_file" 2>/dev/null || true
    fi
    
    # Check for stack overflow indicators
    if [[ -f "${output_file}.log" ]]; then
        if grep -q -E "(stack overflow|segmentation fault|SIGSEGV)" "${output_file}.log"; then
            echo "" >> "$summary_file"
            echo "⚠️  STACK OVERFLOW INDICATORS DETECTED ⚠️" >> "$summary_file"
            grep -E "(stack overflow|segmentation fault|SIGSEGV)" "${output_file}.log" >> "$summary_file"
        fi
    fi
    
    success "Analysis summary generated: $summary_file"
    
    # Display summary
    if [[ "$VERBOSE" == "true" ]]; then
        echo ""
        log "Analysis Summary:"
        cat "$summary_file"
    fi
}

# Main execution
main() {
    log "Starting VexFS FUSE Valgrind Stack Analysis"
    
    parse_args "$@"
    validate_environment
    run_valgrind_analysis
    
    success "Valgrind stack analysis completed"
    log "Results available in: $RESULTS_DIR"
}

# Execute main function
main "$@"