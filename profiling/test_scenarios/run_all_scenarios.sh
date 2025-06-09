#!/bin/bash

# VexFS FUSE Stack Overflow Test Scenarios - Master Execution Script
# Coordinates execution of all test scenario categories for comprehensive analysis

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROFILING_ROOT="$(dirname "$SCRIPT_DIR")"
VEXFS_ROOT="$(dirname "$PROFILING_ROOT")"

# Source common functions
source "$PROFILING_ROOT/scripts/common_functions.sh"

# Configuration
RESULTS_DIR="$PROFILING_ROOT/results/comprehensive_analysis"
SCENARIO_CATEGORIES=(
    "large_vector_operations"
    "hnsw_graph_traversal"
    "component_initialization"
    "stress_testing"
    "baseline_comparison"
)

# Execution parameters
VERBOSE=false
DRY_RUN=false
SELECTED_CATEGORIES=()
PROFILING_TOOLS=("valgrind" "perf" "ebpf")
PARALLEL_EXECUTION=false
MAX_PARALLEL_JOBS=4

# Usage information
usage() {
    cat << EOF
Usage: $0 [OPTIONS]

Run comprehensive VexFS FUSE stack overflow test scenarios.

OPTIONS:
    -v, --verbose              Enable verbose output
    -d, --dry-run             Show what would be executed without running
    -c, --categories CATS     Comma-separated test categories to run
    -t, --tools TOOLS         Comma-separated profiling tools (valgrind,perf,ebpf)
    -p, --parallel            Enable parallel execution of test categories
    -j, --jobs N              Maximum parallel jobs (default: 4)
    -h, --help                Show this help message

CATEGORIES:
    large_vector_operations   Test with progressively larger vector datasets
    hnsw_graph_traversal     Test deep HNSW graph traversal scenarios
    component_initialization  Test isolated and combined component initialization
    stress_testing           Test under extreme conditions and memory pressure
    baseline_comparison      Compare FUSE vs kernel module performance
    all                      Run all categories (default)

EXAMPLES:
    $0 --verbose --categories large_vector_operations,component_initialization
    $0 --tools valgrind,perf --parallel --jobs 2
    $0 --dry-run --categories all

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
            -c|--categories)
                IFS=',' read -ra SELECTED_CATEGORIES <<< "$2"
                shift 2
                ;;
            -t|--tools)
                IFS=',' read -ra PROFILING_TOOLS <<< "$2"
                shift 2
                ;;
            -p|--parallel)
                PARALLEL_EXECUTION=true
                shift
                ;;
            -j|--jobs)
                MAX_PARALLEL_JOBS="$2"
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
    
    # Default to all categories if none specified
    if [[ ${#SELECTED_CATEGORIES[@]} -eq 0 ]]; then
        SELECTED_CATEGORIES=("${SCENARIO_CATEGORIES[@]}")
    fi
}

# Validate environment and dependencies
validate_environment() {
    log_info "Validating test environment"
    
    # Check if profiling environment is set up
    if [[ ! -f "$PROFILING_ROOT/scripts/setup_profiling_environment.sh" ]]; then
        log_error "Profiling environment not found. Please run setup first."
        exit 1
    fi
    
    # Check VexFS binaries
    local fuse_binary="$VEXFS_ROOT/target/profiling/vexfs_fuse"
    if [[ ! -f "$fuse_binary" ]]; then
        log_error "FUSE binary not found: $fuse_binary"
        log_info "Please build VexFS with profiling profile: cargo build --profile profiling"
        exit 1
    fi
    
    # Check kernel module for baseline comparison
    if [[ " ${SELECTED_CATEGORIES[*]} " =~ " baseline_comparison " ]]; then
        local kernel_module="$VEXFS_ROOT/vexfs.ko"
        if [[ ! -f "$kernel_module" ]]; then
            log_warn "Kernel module not found: $kernel_module"
            log_warn "Baseline comparison tests will be limited to FUSE only"
        fi
    fi
    
    # Check profiling tools
    for tool in "${PROFILING_TOOLS[@]}"; do
        case "$tool" in
            "valgrind")
                if ! command -v valgrind &> /dev/null; then
                    log_error "Valgrind not found. Please install valgrind."
                    exit 1
                fi
                ;;
            "perf")
                if ! command -v perf &> /dev/null; then
                    log_error "Perf not found. Please install linux-tools-generic."
                    exit 1
                fi
                ;;
            "ebpf")
                if ! command -v bpftrace &> /dev/null; then
                    log_warn "bpftrace not found. eBPF profiling may be limited."
                fi
                ;;
        esac
    done
    
    log_info "Environment validation completed"
}

# Setup comprehensive test environment
setup_comprehensive_environment() {
    log_info "Setting up comprehensive test environment"
    
    # Create results directory structure
    mkdir -p "$RESULTS_DIR"/{logs,profiles,data,analysis,reports}
    
    # Create category-specific result directories
    for category in "${SCENARIO_CATEGORIES[@]}"; do
        mkdir -p "$RESULTS_DIR/$category"/{logs,profiles,data,analysis}
    done
    
    # Setup profiling environment if not already done
    if [[ ! -f "$PROFILING_ROOT/results/.setup_complete" ]]; then
        log_info "Running profiling environment setup"
        sudo "$PROFILING_ROOT/scripts/setup_profiling_environment.sh"
    fi
    
    # Create test execution log
    local execution_log="$RESULTS_DIR/logs/execution_$(date +%Y%m%d_%H%M%S).log"
    exec 1> >(tee -a "$execution_log")
    exec 2> >(tee -a "$execution_log" >&2)
    
    log_info "Comprehensive test environment setup complete"
    log_info "Execution log: $execution_log"
}

# Execute test category
execute_test_category() {
    local category=$1
    local category_dir="$SCRIPT_DIR/$category"
    
    log_info "Executing test category: $category"
    
    if [[ ! -d "$category_dir" ]]; then
        log_error "Test category directory not found: $category_dir"
        return 1
    fi
    
    # Find the execution script for this category
    local exec_script=""
    if [[ -f "$category_dir/run_${category}_tests.sh" ]]; then
        exec_script="$category_dir/run_${category}_tests.sh"
    elif [[ -f "$category_dir/run_tests.sh" ]]; then
        exec_script="$category_dir/run_tests.sh"
    else
        log_error "No execution script found for category: $category"
        return 1
    fi
    
    # Make script executable
    chmod +x "$exec_script"
    
    # Prepare execution arguments
    local exec_args=()
    if [[ "$VERBOSE" == "true" ]]; then
        exec_args+=("--verbose")
    fi
    if [[ "$DRY_RUN" == "true" ]]; then
        exec_args+=("--dry-run")
    fi
    
    # Add tools argument
    local tools_arg=$(IFS=','; echo "${PROFILING_TOOLS[*]}")
    exec_args+=("--tools" "$tools_arg")
    
    # Execute the test category
    local start_time=$(date +%s)
    local category_log="$RESULTS_DIR/$category/logs/execution_$(date +%Y%m%d_%H%M%S).log"
    
    if [[ "$DRY_RUN" == "true" ]]; then
        log_info "[DRY RUN] Would execute: $exec_script ${exec_args[*]}"
        return 0
    fi
    
    log_info "Starting execution of $category"
    log_info "Command: $exec_script ${exec_args[*]}"
    log_info "Category log: $category_log"
    
    # Execute with timeout and error handling
    if timeout 7200 "$exec_script" "${exec_args[@]}" > "$category_log" 2>&1; then
        local end_time=$(date +%s)
        local duration=$((end_time - start_time))
        log_info "Category $category completed successfully in ${duration}s"
        return 0
    else
        local exit_code=$?
        local end_time=$(date +%s)
        local duration=$((end_time - start_time))
        
        if [[ $exit_code -eq 124 ]]; then
            log_error "Category $category timed out after ${duration}s"
        else
            log_error "Category $category failed with exit code $exit_code after ${duration}s"
        fi
        
        # Copy error log to main results
        cp "$category_log" "$RESULTS_DIR/logs/${category}_error_$(date +%Y%m%d_%H%M%S).log"
        return $exit_code
    fi
}

# Execute categories in parallel
execute_categories_parallel() {
    log_info "Executing test categories in parallel (max jobs: $MAX_PARALLEL_JOBS)"
    
    local pids=()
    local job_count=0
    
    for category in "${SELECTED_CATEGORIES[@]}"; do
        # Wait if we've reached the maximum number of parallel jobs
        while [[ $job_count -ge $MAX_PARALLEL_JOBS ]]; do
            # Check for completed jobs
            for i in "${!pids[@]}"; do
                if ! kill -0 "${pids[$i]}" 2>/dev/null; then
                    wait "${pids[$i]}"
                    local exit_code=$?
                    if [[ $exit_code -eq 0 ]]; then
                        log_info "Parallel job completed successfully"
                    else
                        log_error "Parallel job failed with exit code $exit_code"
                    fi
                    unset "pids[$i]"
                    ((job_count--))
                fi
            done
            sleep 1
        done
        
        # Start new job
        execute_test_category "$category" &
        local pid=$!
        pids+=("$pid")
        ((job_count++))
        
        log_info "Started parallel execution of $category (PID: $pid)"
    done
    
    # Wait for all remaining jobs to complete
    for pid in "${pids[@]}"; do
        if kill -0 "$pid" 2>/dev/null; then
            wait "$pid"
            local exit_code=$?
            if [[ $exit_code -eq 0 ]]; then
                log_info "Final parallel job completed successfully"
            else
                log_error "Final parallel job failed with exit code $exit_code"
            fi
        fi
    done
}

# Execute categories sequentially
execute_categories_sequential() {
    log_info "Executing test categories sequentially"
    
    local failed_categories=()
    
    for category in "${SELECTED_CATEGORIES[@]}"; do
        if ! execute_test_category "$category"; then
            failed_categories+=("$category")
            log_error "Category $category failed"
        fi
    done
    
    if [[ ${#failed_categories[@]} -gt 0 ]]; then
        log_error "Failed categories: ${failed_categories[*]}"
        return 1
    fi
    
    return 0
}

# Generate comprehensive analysis report
generate_comprehensive_report() {
    log_info "Generating comprehensive analysis report"
    
    local report_file="$RESULTS_DIR/reports/comprehensive_stack_overflow_analysis_$(date +%Y%m%d_%H%M%S).md"
    
    cat > "$report_file" << EOF
# VexFS FUSE Stack Overflow Analysis - Comprehensive Report

**Generated:** $(date)
**Test Categories:** ${SELECTED_CATEGORIES[*]}
**Profiling Tools:** ${PROFILING_TOOLS[*]}
**Execution Mode:** $(if [[ "$PARALLEL_EXECUTION" == "true" ]]; then echo "Parallel"; else echo "Sequential"; fi)

## Executive Summary

This comprehensive report analyzes stack overflow behavior in VexFS FUSE implementation
across multiple test scenario categories designed to identify and reproduce the root
causes of stack overflow issues in VectorStorageManager and VectorSearchEngine initialization.

## Test Categories Executed

EOF
    
    # Add category summaries
    for category in "${SELECTED_CATEGORIES[@]}"; do
        echo "### $category" >> "$report_file"
        echo "" >> "$report_file"
        
        local category_results="$RESULTS_DIR/$category"
        if [[ -d "$category_results" ]]; then
            echo "- **Status:** $(if [[ -f "$category_results/logs/execution_"*".log" ]]; then echo "Completed"; else echo "Not executed"; fi)" >> "$report_file"
            echo "- **Profiles Generated:** $(find "$category_results/profiles" -name "*.log" -o -name "*.data" 2>/dev/null | wc -l)" >> "$report_file"
            echo "- **Data Files:** $(find "$category_results/data" -type f 2>/dev/null | wc -l)" >> "$report_file"
        else
            echo "- **Status:** Not executed" >> "$report_file"
        fi
        echo "" >> "$report_file"
    done
    
    cat >> "$report_file" << EOF

## Stack Overflow Analysis Summary

### Key Findings

1. **VectorStorageManager Initialization**
   - [Analysis pending - to be filled based on profiling data]

2. **VectorSearchEngine Initialization**
   - [Analysis pending - to be filled based on profiling data]

3. **Combined Component Initialization**
   - [Analysis pending - to be filled based on profiling data]

4. **HNSW Graph Traversal**
   - [Analysis pending - to be filled based on profiling data]

### Stack Usage Patterns

#### Valgrind Analysis
- **Total Profiles:** $(find "$RESULTS_DIR" -name "*valgrind*.log" 2>/dev/null | wc -l)
- **Stack Overflow Incidents:** [To be analyzed]
- **Maximum Stack Depth:** [To be analyzed]

#### Perf Analysis
- **Total Profiles:** $(find "$RESULTS_DIR" -name "*perf*.data" 2>/dev/null | wc -l)
- **Memory Allocation Hotspots:** [To be analyzed]
- **Performance Bottlenecks:** [To be analyzed]

#### eBPF Analysis
- **Total Traces:** $(find "$RESULTS_DIR" -name "*ebpf*.log" 2>/dev/null | wc -l)
- **Real-time Stack Monitoring:** [To be analyzed]
- **Function Call Patterns:** [To be analyzed]

## Recommendations

Based on the comprehensive analysis across all test categories:

### Immediate Actions
1. **Stack Optimization:** [To be filled based on analysis]
2. **Memory Management:** [To be filled based on analysis]
3. **Component Architecture:** [To be filled based on analysis]

### Long-term Improvements
1. **Architectural Changes:** [To be filled based on analysis]
2. **Algorithm Optimization:** [To be filled based on analysis]
3. **Resource Management:** [To be filled based on analysis]

## Implementation Priority

### High Priority
- [Critical stack overflow fixes]

### Medium Priority
- [Performance optimizations]

### Low Priority
- [Nice-to-have improvements]

## Next Steps

1. **Detailed Analysis:** Analyze collected profiling data from all categories
2. **Root Cause Identification:** Identify specific functions causing stack overflow
3. **Targeted Optimization:** Implement fixes based on analysis findings
4. **Validation Testing:** Re-run test scenarios to validate improvements
5. **Performance Monitoring:** Establish ongoing monitoring for stack usage

## Test Data Location

All test data and profiling results are available in:
\`$RESULTS_DIR\`

### Directory Structure
\`\`\`
$RESULTS_DIR/
├── logs/                    # Execution logs
├── profiles/                # Profiling data
├── data/                    # Test data
├── analysis/                # Analysis results
├── reports/                 # Generated reports
$(for category in "${SELECTED_CATEGORIES[@]}"; do echo "├── $category/               # $category specific results"; done)
\`\`\`

---

**Report Generated:** $(date)
**Total Execution Time:** [To be calculated]
**Test Environment:** $(uname -a)

EOF
    
    log_info "Comprehensive analysis report generated: $report_file"
}

# Cleanup function
cleanup() {
    log_info "Cleaning up test environment"
    
    # Kill any remaining background processes
    jobs -p | xargs -r kill 2>/dev/null || true
    
    # Unmount any test filesystems
    for mount_point in /tmp/vexfs_*_test; do
        if mountpoint -q "$mount_point" 2>/dev/null; then
            fusermount -u "$mount_point" || true
        fi
    done
    
    log_info "Cleanup completed"
}

# Main execution function
main() {
    parse_args "$@"
    
    log_info "Starting VexFS FUSE Stack Overflow Comprehensive Test Suite"
    log_info "Selected categories: ${SELECTED_CATEGORIES[*]}"
    log_info "Profiling tools: ${PROFILING_TOOLS[*]}"
    log_info "Execution mode: $(if [[ "$PARALLEL_EXECUTION" == "true" ]]; then echo "Parallel ($MAX_PARALLEL_JOBS jobs)"; else echo "Sequential"; fi)"
    log_info "Results directory: $RESULTS_DIR"
    
    # Set cleanup trap
    trap cleanup EXIT
    
    # Validate environment
    validate_environment
    
    # Setup test environment
    setup_comprehensive_environment
    
    # Record start time
    local start_time=$(date +%s)
    
    # Execute test categories
    if [[ "$PARALLEL_EXECUTION" == "true" ]]; then
        execute_categories_parallel
    else
        execute_categories_sequential
    fi
    
    # Record end time and calculate duration
    local end_time=$(date +%s)
    local total_duration=$((end_time - start_time))
    
    # Generate comprehensive report
    generate_comprehensive_report
    
    log_info "VexFS FUSE Stack Overflow Comprehensive Test Suite completed"
    log_info "Total execution time: ${total_duration}s"
    log_info "Results available in: $RESULTS_DIR"
    
    # Display summary
    echo ""
    echo "=== EXECUTION SUMMARY ==="
    echo "Categories tested: ${#SELECTED_CATEGORIES[@]}"
    echo "Total duration: ${total_duration}s"
    echo "Results directory: $RESULTS_DIR"
    echo "Comprehensive report: $RESULTS_DIR/reports/"
    echo ""
    echo "Next steps:"
    echo "1. Analyze profiling data in $RESULTS_DIR"
    echo "2. Review comprehensive report for findings"
    echo "3. Implement optimizations based on analysis"
    echo "4. Re-run tests to validate improvements"
}

# Execute main function
main "$@"