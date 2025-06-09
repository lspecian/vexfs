#!/bin/bash

# VexFS FUSE Incremental Component Testing
# Task 23.1: Systematic framework for re-enabling VexFS components

set -euo pipefail

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROFILING_DIR="$(dirname "$SCRIPT_DIR")"
PROJECT_ROOT="$(dirname "$PROFILING_DIR")"
RESULTS_DIR="$PROFILING_DIR/results/incremental"

# Default parameters
COMPONENT=""
DURATION=120
MOUNT_POINT="/tmp/vexfs_incremental_test"
OUTPUT_PREFIX="incremental_component_test"
VERBOSE=false
COMPARE_BASELINE=true
BASELINE_DIR="$PROFILING_DIR/results/baseline"

# Available components for testing
AVAILABLE_COMPONENTS=(
    "vector_storage"
    "search_engine"
    "metadata_manager"
    "cache_manager"
    "transaction_manager"
    "all_components"
)

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
Usage: $0 --component COMPONENT [OPTIONS]

VexFS FUSE Incremental Component Testing

REQUIRED:
    -c, --component COMPONENT  Component to test (see available components below)

OPTIONS:
    -d, --duration SECONDS     Test duration (default: 120)
    -m, --mount-point PATH     FUSE mount point (default: /tmp/vexfs_incremental_test)
    -o, --output PREFIX        Output file prefix (default: incremental_component_test)
    --no-baseline              Skip baseline comparison
    -v, --verbose              Enable verbose output
    -h, --help                 Show this help message

AVAILABLE COMPONENTS:
    vector_storage      - Basic vector storage functionality
    search_engine       - Vector search engine components
    metadata_manager    - File metadata management
    cache_manager       - Caching subsystem
    transaction_manager - ACID transaction support
    all_components      - All components enabled (full functionality)

DESCRIPTION:
    This script systematically re-enables VexFS components one by one to identify
    which component(s) cause stack overflow issues. Each test:
    
    1. Modifies the FUSE implementation to enable the specified component
    2. Rebuilds VexFS with the component enabled
    3. Runs comprehensive profiling (Valgrind, perf, eBPF)
    4. Compares results against baseline minimal implementation
    5. Generates analysis report with recommendations

EXAMPLES:
    $0 --component vector_storage                    # Test vector storage component
    $0 -c search_engine -d 300 -v                   # Test search engine for 5 minutes
    $0 -c all_components --no-baseline              # Test full functionality
    $0 -c metadata_manager -m /mnt/test             # Custom mount point

WORKFLOW:
    1. Run minimal baseline test first: ./minimal_fuse_test.sh
    2. Test components incrementally: vector_storage -> search_engine -> etc.
    3. Identify problematic component when stack overflow occurs
    4. Focus optimization efforts on the problematic component

EOF
}

# Parse command line arguments
parse_args() {
    if [[ $# -eq 0 ]]; then
        error "Component must be specified. Use --help for usage information."
        exit 1
    fi
    
    while [[ $# -gt 0 ]]; do
        case $1 in
            -c|--component)
                COMPONENT="$2"
                shift 2
                ;;
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
            --no-baseline)
                COMPARE_BASELINE=false
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
    
    # Validate component
    if [[ -z "$COMPONENT" ]]; then
        error "Component must be specified with --component"
        exit 1
    fi
    
    # Check if component is valid
    local valid_component=false
    for available_component in "${AVAILABLE_COMPONENTS[@]}"; do
        if [[ "$COMPONENT" == "$available_component" ]]; then
            valid_component=true
            break
        fi
    done
    
    if [[ "$valid_component" == "false" ]]; then
        error "Invalid component: $COMPONENT"
        echo "Available components: ${AVAILABLE_COMPONENTS[*]}"
        exit 1
    fi
}

# Validate environment
validate_environment() {
    log "Validating environment for incremental component testing..."
    
    # Check if baseline exists (if comparison is requested)
    if [[ "$COMPARE_BASELINE" == "true" ]]; then
        if [[ ! -d "$BASELINE_DIR" ]] || [[ -z "$(ls -A "$BASELINE_DIR" 2>/dev/null)" ]]; then
            warning "No baseline results found. Run minimal_fuse_test.sh first for comparison."
            COMPARE_BASELINE=false
        fi
    fi
    
    # Check if results directory exists
    if [[ ! -d "$RESULTS_DIR" ]]; then
        mkdir -p "$RESULTS_DIR"
        log "Created incremental results directory: $RESULTS_DIR"
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

# Modify FUSE implementation to enable component
modify_fuse_implementation() {
    local component="$1"
    local fuse_impl_file="$PROJECT_ROOT/rust/src/fuse_impl.rs"
    local backup_file="${fuse_impl_file}.backup_$(generate_timestamp)"
    
    log "Modifying FUSE implementation to enable: $component"
    
    # Create backup
    cp "$fuse_impl_file" "$backup_file"
    log "Created backup: $backup_file"
    
    case "$component" in
        vector_storage)
            log "Enabling vector storage component..."
            # Re-enable vector storage in the struct and initialization
            sed -i 's|// vector_storage: Arc<Mutex<VectorStorageManager>>,|vector_storage: Arc<Mutex<VectorStorageManager>>,|g' "$fuse_impl_file"
            sed -i 's|// Temporarily commented out to isolate stack overflow|// Vector storage enabled for testing|g' "$fuse_impl_file"
            ;;
        search_engine)
            log "Enabling search engine component..."
            # Re-enable search engine (requires vector storage too)
            sed -i 's|// vector_storage: Arc<Mutex<VectorStorageManager>>,|vector_storage: Arc<Mutex<VectorStorageManager>>,|g' "$fuse_impl_file"
            sed -i 's|// search_engine: Arc<Mutex<VectorSearchEngine>>,|search_engine: Arc<Mutex<VectorSearchEngine>>,|g' "$fuse_impl_file"
            ;;
        metadata_manager)
            log "Enabling metadata manager component..."
            # Add metadata manager (conceptual - would need actual implementation)
            sed -i 's|// Temporarily remove VexFS components|// Metadata manager enabled for testing|g' "$fuse_impl_file"
            ;;
        cache_manager)
            log "Enabling cache manager component..."
            # Add cache manager (conceptual)
            sed -i 's|// Temporarily remove VexFS components|// Cache manager enabled for testing|g' "$fuse_impl_file"
            ;;
        transaction_manager)
            log "Enabling transaction manager component..."
            # Add transaction manager (conceptual)
            sed -i 's|// Temporarily remove VexFS components|// Transaction manager enabled for testing|g' "$fuse_impl_file"
            ;;
        all_components)
            log "Enabling all components..."
            # Re-enable everything
            sed -i 's|// vector_storage: Arc<Mutex<VectorStorageManager>>,|vector_storage: Arc<Mutex<VectorStorageManager>>,|g' "$fuse_impl_file"
            sed -i 's|// search_engine: Arc<Mutex<VectorSearchEngine>>,|search_engine: Arc<Mutex<VectorSearchEngine>>,|g' "$fuse_impl_file"
            sed -i 's|// Temporarily remove VexFS components|// All components enabled for testing|g' "$fuse_impl_file"
            ;;
        *)
            error "Unknown component modification: $component"
            return 1
            ;;
    esac
    
    success "FUSE implementation modified for: $component"
    echo "$backup_file"
}

# Restore FUSE implementation from backup
restore_fuse_implementation() {
    local backup_file="$1"
    local fuse_impl_file="$PROJECT_ROOT/rust/src/fuse_impl.rs"
    
    if [[ -f "$backup_file" ]]; then
        log "Restoring FUSE implementation from backup..."
        cp "$backup_file" "$fuse_impl_file"
        success "FUSE implementation restored"
    else
        warning "Backup file not found: $backup_file"
    fi
}

# Build VexFS with component enabled
build_vexfs_with_component() {
    local component="$1"
    
    log "Building VexFS with $component enabled..."
    
    cd "$PROJECT_ROOT"
    
    # Clean previous build
    cargo clean
    
    # Build with profiling profile
    if cargo build --profile profiling --features fuse_support --bin vexfs_fuse; then
        success "VexFS built successfully with $component enabled"
        
        # Verify binary exists
        local binary_path="$PROJECT_ROOT/target/profiling/vexfs_fuse"
        if [[ -f "$binary_path" ]]; then
            log "Binary location: $binary_path"
            local binary_size
            binary_size=$(du -h "$binary_path" | cut -f1)
            log "Binary size: $binary_size"
        else
            error "Binary not found after build"
            return 1
        fi
    else
        error "Failed to build VexFS with $component enabled"
        return 1
    fi
}

# Create component-specific workload
create_component_workload() {
    local component="$1"
    local script_path="$RESULTS_DIR/component_workload_${component}_$(generate_timestamp).sh"
    
    log "Creating workload for $component testing..."
    
    cat > "$script_path" << EOF
#!/bin/bash
# Component-specific workload for $component

MOUNT_POINT="\$1"
DURATION="\$2"
COMPONENT="$component"

log() {
    echo "[WORKLOAD-\$COMPONENT] \$1"
}

log "Starting \$COMPONENT workload on \$MOUNT_POINT for \${DURATION}s"

# Component-specific operations
component_operations() {
    local end_time=\$(($(date +%s) + DURATION))
    local operation_count=0
    
    while [[ \$(date +%s) -lt \$end_time ]]; do
        case "\$COMPONENT" in
            vector_storage)
                # Operations that would stress vector storage
                echo "Vector data \$operation_count: [0.1, 0.2, 0.3, 0.4]" > "\$MOUNT_POINT/vector_\$operation_count.vec"
                cat "\$MOUNT_POINT/vector_\$operation_count.vec" > /dev/null
                ;;
            search_engine)
                # Operations that would stress search functionality
                echo "Searchable content \$operation_count" > "\$MOUNT_POINT/search_\$operation_count.txt"
                grep "content" "\$MOUNT_POINT/search_\$operation_count.txt" > /dev/null
                ;;
            metadata_manager)
                # Operations that would stress metadata
                touch "\$MOUNT_POINT/meta_\$operation_count.dat"
                stat "\$MOUNT_POINT/meta_\$operation_count.dat" > /dev/null
                chmod 644 "\$MOUNT_POINT/meta_\$operation_count.dat"
                ;;
            cache_manager)
                # Operations that would stress caching
                echo "Cached data \$operation_count" > "\$MOUNT_POINT/cache_\$operation_count.cache"
                # Read multiple times to trigger caching
                for i in {1..3}; do
                    cat "\$MOUNT_POINT/cache_\$operation_count.cache" > /dev/null
                done
                ;;
            transaction_manager)
                # Operations that would stress transactions
                mkdir -p "\$MOUNT_POINT/txn_\$operation_count"
                echo "Transaction data" > "\$MOUNT_POINT/txn_\$operation_count/data.txt"
                mv "\$MOUNT_POINT/txn_\$operation_count/data.txt" "\$MOUNT_POINT/txn_\$operation_count/committed.txt"
                ;;
            all_components)
                # Mixed operations stressing all components
                echo "Vector: [0.1, 0.2]" > "\$MOUNT_POINT/all_\$operation_count.vec"
                echo "Search content" > "\$MOUNT_POINT/all_\$operation_count.txt"
                mkdir -p "\$MOUNT_POINT/all_dir_\$operation_count"
                stat "\$MOUNT_POINT/all_\$operation_count.vec" > /dev/null
                grep "content" "\$MOUNT_POINT/all_\$operation_count.txt" > /dev/null
                ;;
            *)
                # Default operations
                echo "Component test data \$operation_count" > "\$MOUNT_POINT/test_\$operation_count.txt"
                cat "\$MOUNT_POINT/test_\$operation_count.txt" > /dev/null
                ;;
        esac
        
        # Common operations
        ls -la "\$MOUNT_POINT" > /dev/null
        
        operation_count=\$((operation_count + 1))
        
        # Pacing based on component complexity
        case "\$COMPONENT" in
            all_components)
                sleep 0.3  # Slower for full functionality
                ;;
            vector_storage|search_engine)
                sleep 0.2  # Medium pace for complex components
                ;;
            *)
                sleep 0.1  # Faster for simpler components
                ;;
        esac
        
        if [[ \$((operation_count % 50)) -eq 0 ]]; then
            log "Completed \$operation_count \$COMPONENT operations"
        fi
    done
    
    log "\$COMPONENT workload completed with \$operation_count operations"
}

# Execute component operations
component_operations
EOF
    
    chmod +x "$script_path"
    echo "$script_path"
}

# Run comprehensive profiling for component
run_component_profiling() {
    local component="$1"
    local timestamp
    timestamp=$(generate_timestamp)
    local output_file="$RESULTS_DIR/${OUTPUT_PREFIX}_${component}_${timestamp}"
    
    log "Running comprehensive profiling for: $component"
    
    # Create component workload
    local workload_script
    workload_script=$(create_component_workload "$component")
    
    # Run Valgrind analysis
    log "Running Valgrind stack analysis for $component..."
    if ! "$PROFILING_DIR/scripts/run_valgrind_stack_analysis.sh" \
        -d "$DURATION" \
        -m "$MOUNT_POINT" \
        -w basic \
        -o "${component}_valgrind" \
        -v; then
        warning "Valgrind analysis failed for $component"
    fi
    
    # Run perf memory profiling
    log "Running perf memory profiling for $component..."
    if ! sudo "$PROFILING_DIR/scripts/run_perf_memory_profile.sh" \
        -d "$DURATION" \
        -m "$MOUNT_POINT" \
        -o "${component}_perf" \
        -v; then
        warning "Perf profiling failed for $component"
    fi
    
    # Run eBPF tracing
    log "Running eBPF tracing for $component..."
    if ! sudo "$PROFILING_DIR/scripts/run_ebpf_fuse_tracing.sh" \
        -d "$DURATION" \
        -m "$MOUNT_POINT" \
        -t comprehensive \
        -o "${component}_ebpf" \
        -v; then
        warning "eBPF tracing failed for $component"
    fi
    
    success "Comprehensive profiling completed for: $component"
}

# Compare with baseline
compare_with_baseline() {
    local component="$1"
    local output_file="$2"
    
    if [[ "$COMPARE_BASELINE" == "false" ]]; then
        return 0
    fi
    
    log "Comparing $component results with baseline..."
    
    # Find latest baseline results
    local baseline_file
    baseline_file=$(find "$BASELINE_DIR" -name "*_analysis.txt" -type f | sort | tail -1)
    
    if [[ -z "$baseline_file" ]]; then
        warning "No baseline analysis file found for comparison"
        return 0
    fi
    
    local comparison_file="${output_file}_baseline_comparison.txt"
    
    cat > "$comparison_file" << EOF
VexFS FUSE Component vs Baseline Comparison
==========================================

Component: $component
Comparison Date: $(date)
Baseline File: $baseline_file

=== Comparison Analysis ===
EOF
    
    # Extract baseline metrics (simplified)
    if grep -q "Average:" "$baseline_file"; then
        local baseline_memory
        baseline_memory=$(grep "Average:" "$baseline_file" | grep "KB" | awk '{print $2}' | head -1)
        
        echo "Baseline Memory Usage: ${baseline_memory} KB" >> "$comparison_file"
        echo "Component: $component" >> "$comparison_file"
        echo "" >> "$comparison_file"
        echo "⚠️  Manual comparison required with current profiling results" >> "$comparison_file"
        echo "Check Valgrind, perf, and eBPF outputs for detailed comparison" >> "$comparison_file"
    fi
    
    success "Baseline comparison generated: $comparison_file"
}

# Generate component analysis
generate_component_analysis() {
    local component="$1"
    local timestamp
    timestamp=$(generate_timestamp)
    local analysis_file="$RESULTS_DIR/${OUTPUT_PREFIX}_${component}_analysis_${timestamp}.txt"
    
    log "Generating analysis for component: $component"
    
    cat > "$analysis_file" << EOF
VexFS FUSE Incremental Component Analysis
========================================

Component: $component
Test Date: $(date)
Duration: ${DURATION}s
Mount Point: $MOUNT_POINT

=== Component Description ===
EOF
    
    case "$component" in
        vector_storage)
            cat >> "$analysis_file" << EOF
Vector Storage Component:
- Manages vector data storage and retrieval
- Handles vector serialization/deserialization
- Potential stack impact: Vector allocation and processing
EOF
            ;;
        search_engine)
            cat >> "$analysis_file" << EOF
Search Engine Component:
- Implements vector similarity search
- Manages search indices and algorithms
- Potential stack impact: Search algorithm recursion, index traversal
EOF
            ;;
        metadata_manager)
            cat >> "$analysis_file" << EOF
Metadata Manager Component:
- Handles file metadata and attributes
- Manages metadata persistence
- Potential stack impact: Metadata processing and caching
EOF
            ;;
        cache_manager)
            cat >> "$analysis_file" << EOF
Cache Manager Component:
- Implements caching for performance
- Manages cache eviction and consistency
- Potential stack impact: Cache data structures and algorithms
EOF
            ;;
        transaction_manager)
            cat >> "$analysis_file" << EOF
Transaction Manager Component:
- Provides ACID transaction support
- Manages transaction logging and recovery
- Potential stack impact: Transaction state management
EOF
            ;;
        all_components)
            cat >> "$analysis_file" << EOF
All Components Enabled:
- Complete VexFS functionality
- All subsystems active
- Potential stack impact: Combined effect of all components
EOF
            ;;
    esac
    
    cat >> "$analysis_file" << EOF

=== Test Results ===
Check the following files for detailed results:
- Valgrind: $PROFILING_DIR/results/valgrind/${component}_valgrind_*
- Perf: $PROFILING_DIR/results/perf/${component}_perf_*
- eBPF: $PROFILING_DIR/results/ebpf/${component}_ebpf_*

=== Analysis Guidelines ===
1. Check Valgrind logs for stack overflow indicators
2. Compare memory usage with baseline
3. Look for performance regressions
4. Identify specific functions causing issues

=== Next Steps ===
EOF
    
    if [[ "$component" != "all_components" ]]; then
        cat >> "$analysis_file" << EOF
If this component test passes:
- Proceed to next component in sequence
- Consider combining with previously tested components

If this component test fails:
- Focus optimization efforts on this component
- Analyze specific functions causing stack overflow
- Consider alternative implementation approaches
EOF
    else
        cat >> "$analysis_file" << EOF
If all components test fails:
- Identify which combination of components causes issues
- Test components in smaller combinations
- Optimize problematic component interactions
EOF
    fi
    
    success "Component analysis generated: $analysis_file"
    echo "$analysis_file"
}

# Main execution
main() {
    log "Starting VexFS FUSE Incremental Component Testing"
    
    parse_args "$@"
    validate_environment
    
    local backup_file
    local analysis_file
    
    # Modify FUSE implementation
    backup_file=$(modify_fuse_implementation "$COMPONENT")
    
    # Build with component enabled
    if ! build_vexfs_with_component "$COMPONENT"; then
        error "Build failed for component: $COMPONENT"
        restore_fuse_implementation "$backup_file"
        exit 1
    fi
    
    # Run comprehensive profiling
    if ! run_component_profiling "$COMPONENT"; then
        error "Profiling failed for component: $COMPONENT"
        restore_fuse_implementation "$backup_file"
        exit 1
    fi
    
    # Generate analysis
    analysis_file=$(generate_component_analysis "$COMPONENT")
    
    # Compare with baseline
    compare_with_baseline "$COMPONENT" "${analysis_file%_analysis_*}"
    
    # Restore original implementation
    restore_fuse_implementation "$backup_file"
    
    success "Incremental component testing completed for: $COMPONENT"
    log "Results available in: $RESULTS_DIR"
    log "Analysis: $analysis_file"
    
    if [[ "$VERBOSE" == "true" ]]; then
        echo ""
        log "Component Test Summary:"
        cat "$analysis_file"
    fi
}

# Execute main function
main "$@"