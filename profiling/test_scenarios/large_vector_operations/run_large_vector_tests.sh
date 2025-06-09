#!/bin/bash

# Large Vector Operations Test Runner
# Tests VexFS FUSE implementation with progressively larger vector datasets
# to identify stack overflow thresholds in VectorStorageManager

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROFILING_ROOT="$(dirname "$(dirname "$SCRIPT_DIR")")"
VEXFS_ROOT="$(dirname "$(dirname "$PROFILING_ROOT")")"

# Source common functions
source "$PROFILING_ROOT/scripts/common_functions.sh"

# Configuration
CONFIG_FILE="$SCRIPT_DIR/test_config.yaml"
RESULTS_DIR="$PROFILING_ROOT/results/large_vector_operations"
TEST_MOUNT_POINT="/tmp/vexfs_large_vector_test"
FUSE_BINARY="$VEXFS_ROOT/target/profiling/vexfs_fuse"

# Test execution parameters
VERBOSE=false
DRY_RUN=false
SELECTED_SCENARIO=""
PROFILING_TOOLS=("valgrind" "perf" "ebpf")

# Usage information
usage() {
    cat << EOF
Usage: $0 [OPTIONS]

Run large vector operations test scenarios for VexFS FUSE stack analysis.

OPTIONS:
    -v, --verbose           Enable verbose output
    -d, --dry-run          Show what would be executed without running
    -s, --scenario NAME    Run specific scenario only
    -t, --tools TOOLS      Comma-separated profiling tools (valgrind,perf,ebpf)
    -h, --help             Show this help message

SCENARIOS:
    dataset_size_progression    Test with increasing dataset sizes
    dimension_scaling          Test with different vector dimensions
    bulk_operations           Test bulk vector operations
    concurrent_operations     Test concurrent vector operations
    metadata_operations       Test vector metadata operations
    all                       Run all scenarios (default)

EXAMPLES:
    $0 --verbose --scenario dataset_size_progression
    $0 --tools valgrind,perf --scenario bulk_operations
    $0 --dry-run --scenario all

EOF
}

# Parse command line arguments
parse_args() {
    while [[ $# -gt 0 ]]; do
        case $1 in
            -v|--verbose)
                VERBOSE=true
                shift
                ;;
            -d|--dry-run)
                DRY_RUN=true
                shift
                ;;
            -s|--scenario)
                SELECTED_SCENARIO="$2"
                shift 2
                ;;
            -t|--tools)
                IFS=',' read -ra PROFILING_TOOLS <<< "$2"
                shift 2
                ;;
            -h|--help)
                usage
                exit 0
                ;;
            *)
                echo "Unknown option: $1" >&2
                usage >&2
                exit 1
                ;;
        esac
    done
}

# Setup test environment
setup_test_environment() {
    log_info "Setting up large vector operations test environment"
    
    # Create results directory
    mkdir -p "$RESULTS_DIR"/{logs,profiles,data,analysis}
    
    # Ensure FUSE binary exists
    if [[ ! -f "$FUSE_BINARY" ]]; then
        log_error "FUSE binary not found: $FUSE_BINARY"
        log_info "Please build VexFS with profiling profile: cargo build --profile profiling"
        exit 1
    fi
    
    # Create test mount point
    mkdir -p "$TEST_MOUNT_POINT"
    
    # Ensure mount point is clean
    if mountpoint -q "$TEST_MOUNT_POINT"; then
        log_warn "Unmounting existing filesystem at $TEST_MOUNT_POINT"
        fusermount -u "$TEST_MOUNT_POINT" || true
    fi
    
    log_info "Test environment setup complete"
}

# Generate test vectors
generate_test_vectors() {
    local vector_count=$1
    local dimensions=$2
    local output_file=$3
    
    log_info "Generating $vector_count vectors with $dimensions dimensions"
    
    # Create test vector generation script
    cat > "$RESULTS_DIR/generate_vectors.py" << 'EOF'
#!/usr/bin/env python3
import sys
import json
import random
import struct

def generate_vectors(count, dimensions, output_file):
    vectors = []
    for i in range(count):
        vector = [random.uniform(-1.0, 1.0) for _ in range(dimensions)]
        vectors.append({
            'id': i,
            'vector': vector,
            'metadata': {
                'generated_id': i,
                'dimension_count': dimensions,
                'test_batch': 'large_vector_test'
            }
        })
    
    with open(output_file, 'w') as f:
        json.dump(vectors, f)
    
    print(f"Generated {count} vectors with {dimensions} dimensions to {output_file}")

if __name__ == "__main__":
    if len(sys.argv) != 4:
        print("Usage: generate_vectors.py <count> <dimensions> <output_file>")
        sys.exit(1)
    
    count = int(sys.argv[1])
    dimensions = int(sys.argv[2])
    output_file = sys.argv[3]
    
    generate_vectors(count, dimensions, output_file)
EOF
    
    chmod +x "$RESULTS_DIR/generate_vectors.py"
    python3 "$RESULTS_DIR/generate_vectors.py" "$vector_count" "$dimensions" "$output_file"
}

# Run vector storage test
run_vector_storage_test() {
    local test_name=$1
    local vector_file=$2
    local profiling_tool=$3
    
    log_info "Running vector storage test: $test_name with $profiling_tool"
    
    local test_script="$RESULTS_DIR/vector_test_${test_name}.py"
    
    # Create test script
    cat > "$test_script" << EOF
#!/usr/bin/env python3
import os
import sys
import json
import time
import traceback

def run_vector_operations(mount_point, vector_file):
    """Run vector operations that stress VectorStorageManager"""
    
    print(f"Loading vectors from {vector_file}")
    with open(vector_file, 'r') as f:
        vectors = json.load(f)
    
    print(f"Loaded {len(vectors)} vectors")
    
    # Test vector file creation and writing
    for i, vector_data in enumerate(vectors):
        try:
            vector_file_path = os.path.join(mount_point, f"vector_{i}.vec")
            
            # Write vector data to file
            with open(vector_file_path, 'w') as f:
                json.dump(vector_data, f)
            
            # Simulate vector storage operations
            if i % 100 == 0:
                print(f"Processed {i} vectors")
                
        except Exception as e:
            print(f"Error processing vector {i}: {e}")
            traceback.print_exc()
            return False
    
    print(f"Successfully processed {len(vectors)} vectors")
    return True

if __name__ == "__main__":
    if len(sys.argv) != 3:
        print("Usage: vector_test.py <mount_point> <vector_file>")
        sys.exit(1)
    
    mount_point = sys.argv[1]
    vector_file = sys.argv[2]
    
    success = run_vector_operations(mount_point, vector_file)
    sys.exit(0 if success else 1)
EOF
    
    chmod +x "$test_script"
    
    # Run with profiling
    local profile_output="$RESULTS_DIR/profiles/${test_name}_${profiling_tool}"
    
    case "$profiling_tool" in
        "valgrind")
            run_with_valgrind "$test_script" "$TEST_MOUNT_POINT" "$vector_file" "$profile_output"
            ;;
        "perf")
            run_with_perf "$test_script" "$TEST_MOUNT_POINT" "$vector_file" "$profile_output"
            ;;
        "ebpf")
            run_with_ebpf "$test_script" "$TEST_MOUNT_POINT" "$vector_file" "$profile_output"
            ;;
        *)
            log_error "Unknown profiling tool: $profiling_tool"
            return 1
            ;;
    esac
}

# Run with Valgrind profiling
run_with_valgrind() {
    local test_script=$1
    local mount_point=$2
    local vector_file=$3
    local output_prefix=$4
    
    log_info "Running with Valgrind stack analysis"
    
    # Start FUSE with Valgrind
    valgrind \
        --tool=memcheck \
        --leak-check=full \
        --show-leak-kinds=all \
        --track-origins=yes \
        --verbose \
        --log-file="${output_prefix}_fuse.log" \
        "$FUSE_BINARY" "$mount_point" &
    
    local fuse_pid=$!
    
    # Wait for mount
    sleep 3
    
    # Run test
    valgrind \
        --tool=memcheck \
        --leak-check=full \
        --track-origins=yes \
        --log-file="${output_prefix}_test.log" \
        python3 "$test_script" "$mount_point" "$vector_file"
    
    # Cleanup
    fusermount -u "$mount_point" || true
    wait $fuse_pid || true
}

# Run with Perf profiling
run_with_perf() {
    local test_script=$1
    local mount_point=$2
    local vector_file=$3
    local output_prefix=$4
    
    log_info "Running with Perf memory profiling"
    
    # Start FUSE
    "$FUSE_BINARY" "$mount_point" &
    local fuse_pid=$!
    
    # Wait for mount
    sleep 3
    
    # Run test with perf
    perf record \
        -g \
        -e cycles,cache-misses,page-faults \
        -o "${output_prefix}.data" \
        python3 "$test_script" "$mount_point" "$vector_file"
    
    # Generate report
    perf report -i "${output_prefix}.data" > "${output_prefix}_report.txt"
    
    # Cleanup
    fusermount -u "$mount_point" || true
    wait $fuse_pid || true
}

# Run with eBPF profiling
run_with_ebpf() {
    local test_script=$1
    local mount_point=$2
    local vector_file=$3
    local output_prefix=$4
    
    log_info "Running with eBPF tracing"
    
    # Start FUSE
    "$FUSE_BINARY" "$mount_point" &
    local fuse_pid=$!
    
    # Wait for mount
    sleep 3
    
    # Start eBPF tracing
    "$PROFILING_ROOT/scripts/run_ebpf_fuse_tracing.sh" \
        --trace-type stack \
        --output "${output_prefix}_ebpf.log" \
        --pid $fuse_pid &
    local ebpf_pid=$!
    
    # Run test
    python3 "$test_script" "$mount_point" "$vector_file" > "${output_prefix}_test.log" 2>&1
    
    # Cleanup
    kill $ebpf_pid || true
    fusermount -u "$mount_point" || true
    wait $fuse_pid || true
}

# Run dataset size progression tests
run_dataset_size_progression() {
    log_info "Running dataset size progression tests"
    
    local scenarios=(
        "small_dataset:1000:128"
        "medium_dataset:10000:256"
        "large_dataset:100000:512"
        "xlarge_dataset:1000000:1024"
    )
    
    for scenario in "${scenarios[@]}"; do
        IFS=':' read -r name count dims <<< "$scenario"
        
        log_info "Running scenario: $name ($count vectors, $dims dimensions)"
        
        if [[ "$DRY_RUN" == "true" ]]; then
            log_info "[DRY RUN] Would run $name with $count vectors, $dims dimensions"
            continue
        fi
        
        # Generate test data
        local vector_file="$RESULTS_DIR/data/${name}_vectors.json"
        generate_test_vectors "$count" "$dims" "$vector_file"
        
        # Run with each profiling tool
        for tool in "${PROFILING_TOOLS[@]}"; do
            run_vector_storage_test "$name" "$vector_file" "$tool"
        done
    done
}

# Run dimension scaling tests
run_dimension_scaling() {
    log_info "Running dimension scaling tests"
    
    local scenarios=(
        "low_dim_high_count:50000:128"
        "medium_dim_medium_count:25000:512"
        "high_dim_low_count:10000:2048"
        "max_dim_test:5000:4096"
    )
    
    for scenario in "${scenarios[@]}"; do
        IFS=':' read -r name count dims <<< "$scenario"
        
        log_info "Running scenario: $name ($count vectors, $dims dimensions)"
        
        if [[ "$DRY_RUN" == "true" ]]; then
            log_info "[DRY RUN] Would run $name with $count vectors, $dims dimensions"
            continue
        fi
        
        # Generate test data
        local vector_file="$RESULTS_DIR/data/${name}_vectors.json"
        generate_test_vectors "$count" "$dims" "$vector_file"
        
        # Run with each profiling tool
        for tool in "${PROFILING_TOOLS[@]}"; do
            run_vector_storage_test "$name" "$vector_file" "$tool"
        done
    done
}

# Run bulk operations tests
run_bulk_operations() {
    log_info "Running bulk operations tests"
    
    # Implementation for bulk operations testing
    log_info "Bulk operations tests - implementation pending"
}

# Run concurrent operations tests
run_concurrent_operations() {
    log_info "Running concurrent operations tests"
    
    # Implementation for concurrent operations testing
    log_info "Concurrent operations tests - implementation pending"
}

# Run metadata operations tests
run_metadata_operations() {
    log_info "Running metadata operations tests"
    
    # Implementation for metadata operations testing
    log_info "Metadata operations tests - implementation pending"
}

# Generate analysis report
generate_analysis_report() {
    log_info "Generating analysis report"
    
    local report_file="$RESULTS_DIR/analysis/large_vector_operations_report.md"
    
    cat > "$report_file" << EOF
# Large Vector Operations Test Analysis Report

Generated: $(date)

## Test Summary

This report analyzes stack overflow behavior in VexFS FUSE implementation
during large vector operations testing.

## Test Scenarios Executed

EOF
    
    # Add scenario results
    for scenario_dir in "$RESULTS_DIR/profiles"/*; do
        if [[ -d "$scenario_dir" ]]; then
            echo "- $(basename "$scenario_dir")" >> "$report_file"
        fi
    done
    
    cat >> "$report_file" << EOF

## Stack Usage Analysis

### Valgrind Results
$(find "$RESULTS_DIR/profiles" -name "*valgrind*.log" | wc -l) Valgrind profiles generated

### Perf Results  
$(find "$RESULTS_DIR/profiles" -name "*perf*.data" | wc -l) Perf profiles generated

### eBPF Results
$(find "$RESULTS_DIR/profiles" -name "*ebpf*.log" | wc -l) eBPF traces generated

## Recommendations

Based on the profiling data collected, the following optimizations are recommended:

1. **Stack Usage Optimization**: [To be filled based on analysis]
2. **Memory Management**: [To be filled based on analysis]  
3. **Component Initialization**: [To be filled based on analysis]

## Next Steps

1. Analyze collected profiling data
2. Identify stack overflow root causes
3. Implement targeted optimizations
4. Re-run tests to validate improvements

EOF
    
    log_info "Analysis report generated: $report_file"
}

# Cleanup function
cleanup() {
    log_info "Cleaning up test environment"
    
    # Unmount if still mounted
    if mountpoint -q "$TEST_MOUNT_POINT" 2>/dev/null; then
        fusermount -u "$TEST_MOUNT_POINT" || true
    fi
    
    # Remove temporary files
    rm -f "$RESULTS_DIR/generate_vectors.py"
    rm -f "$RESULTS_DIR"/vector_test_*.py
}

# Main execution function
main() {
    parse_args "$@"
    
    log_info "Starting large vector operations test suite"
    log_info "Selected scenario: ${SELECTED_SCENARIO:-all}"
    log_info "Profiling tools: ${PROFILING_TOOLS[*]}"
    log_info "Results directory: $RESULTS_DIR"
    
    # Setup
    setup_test_environment
    
    # Set cleanup trap
    trap cleanup EXIT
    
    # Run selected scenarios
    case "${SELECTED_SCENARIO:-all}" in
        "dataset_size_progression")
            run_dataset_size_progression
            ;;
        "dimension_scaling")
            run_dimension_scaling
            ;;
        "bulk_operations")
            run_bulk_operations
            ;;
        "concurrent_operations")
            run_concurrent_operations
            ;;
        "metadata_operations")
            run_metadata_operations
            ;;
        "all"|"")
            run_dataset_size_progression
            run_dimension_scaling
            run_bulk_operations
            run_concurrent_operations
            run_metadata_operations
            ;;
        *)
            log_error "Unknown scenario: $SELECTED_SCENARIO"
            usage >&2
            exit 1
            ;;
    esac
    
    # Generate analysis
    generate_analysis_report
    
    log_info "Large vector operations test suite completed"
    log_info "Results available in: $RESULTS_DIR"
}

# Execute main function
main "$@"