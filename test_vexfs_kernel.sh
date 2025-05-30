#!/bin/bash
#
# VexFS Kernel Module Testing Suite - Main Entry Point
# 
# This script orchestrates the three-level VexFS kernel module testing strategy:
# - Level 1: Basic kernel module validation (host-safe operations)
# - Level 2: Mount operation testing (VM-isolated)
# - Level 3: Stress testing (VM-isolated)
#
# Usage: ./test_vexfs_kernel.sh [level1|level2|level3|all] [options]
#
# Safety: Level 1 tests run on host, Level 2+ require VM isolation

set -euo pipefail

# Script configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$SCRIPT_DIR"
TESTS_DIR="$PROJECT_ROOT/tests"
KERNEL_MODULE_DIR="$TESTS_DIR/kernel_module"
VM_MANAGEMENT_DIR="$TESTS_DIR/legacy/vm_management"
RESULTS_DIR="$PROJECT_ROOT/test_results"

# Test configuration
DEFAULT_BUILD_VARIANT="safe"
DEFAULT_TIMEOUT=300
ENABLE_SUDO_TESTS=false
VERBOSE=false
DRY_RUN=false

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Progress tracking
CURRENT_STEP=0
TOTAL_STEPS=0

# Logging
LOG_FILE="$RESULTS_DIR/test_session_$(date +%Y%m%d_%H%M%S).log"

#
# Utility Functions
#

log() {
    local level="$1"
    shift
    local message="$*"
    local timestamp=$(date '+%Y-%m-%d %H:%M:%S')
    
    echo "[$timestamp] [$level] $message" | tee -a "$LOG_FILE"
}

info() {
    log "INFO" "$@"
    echo -e "${BLUE}ℹ️  $*${NC}"
}

success() {
    log "SUCCESS" "$@"
    echo -e "${GREEN}✅ $*${NC}"
}

warning() {
    log "WARNING" "$@"
    echo -e "${YELLOW}⚠️  $*${NC}"
}

error() {
    log "ERROR" "$@"
    echo -e "${RED}❌ $*${NC}"
}

progress() {
    CURRENT_STEP=$((CURRENT_STEP + 1))
    local message="$*"
    log "PROGRESS" "Step $CURRENT_STEP/$TOTAL_STEPS: $message"
    echo -e "${PURPLE}🔄 [$CURRENT_STEP/$TOTAL_STEPS] $message${NC}"
}

#
# Help and Usage
#

show_help() {
    cat << EOF >&2
VexFS Kernel Module Testing Suite

USAGE:
    $0 [LEVEL] [OPTIONS]

LEVELS:
    level1      Basic kernel module validation (HOST-SAFE)
                - Module compilation, loading, unloading
                - Resource leak detection
                - Kernel health checks
                
    level2      Mount operation testing (VM-ISOLATED)
                - Mount/unmount operations
                - Crash detection and recovery
                - Basic filesystem operations
                
    level3      Stress testing (VM-ISOLATED)
                - Concurrent mount/unmount operations
                - High-frequency operations
                - Resource exhaustion testing
                
    all         Run all test levels sequentially

OPTIONS:
    --build-variant VARIANT    Build variant to test (safe|safe-ffi|c-only|standard)
                              Default: safe
    --timeout SECONDS         Maximum time for operations (default: 300)
    --enable-sudo             Enable tests requiring sudo privileges
    --verbose                 Enable verbose output
    --dry-run                 Show what would be executed without running
    --help                    Show this help message

EXAMPLES:
    # Safe host-based testing (recommended first run)
    $0 level1 --build-variant safe
    
    # Full testing with sudo privileges
    $0 level1 --enable-sudo --build-variant safe-ffi
    
    # VM-isolated mount testing
    $0 level2 --build-variant standard
    
    # Complete test suite
    $0 all --enable-sudo --verbose

SAFETY NOTES:
    - Level 1 tests are HOST-SAFE and can run on development machines
    - Level 2+ tests require VM isolation due to potential system crashes
    - Always run Level 1 tests first to validate basic functionality
    - Use --dry-run to preview operations before execution

EOF
}

#
# Argument Parsing
#

parse_arguments() {
    local test_level=""
    
    while [[ $# -gt 0 ]]; do
        case $1 in
            level1|level2|level3|all)
                test_level="$1"
                shift
                ;;
            --build-variant)
                DEFAULT_BUILD_VARIANT="$2"
                shift 2
                ;;
            --timeout)
                DEFAULT_TIMEOUT="$2"
                shift 2
                ;;
            --enable-sudo)
                ENABLE_SUDO_TESTS=true
                shift
                ;;
            --verbose)
                VERBOSE=true
                shift
                ;;
            --dry-run)
                DRY_RUN=true
                shift
                ;;
            --help|-h)
                show_help
                exit 0
                ;;
            *)
                error "Unknown option: $1"
                show_help
                exit 1
                ;;
        esac
    done
    
    if [[ -z "$test_level" ]]; then
        error "Test level required. Use --help for usage information."
        show_help
        exit 1
    fi
    
    echo "$test_level"
}

#
# Environment Validation
#

validate_environment() {
    local test_level="$1"
    info "Validating test environment for $test_level..."
    
    # Check basic required directories
    local required_dirs=(
        "$TESTS_DIR"
        "$KERNEL_MODULE_DIR"
    )
    
    # Add VM management directory only for Level 2+ tests
    if [[ "$test_level" == "level2" || "$test_level" == "level3" || "$test_level" == "all" ]]; then
        required_dirs+=("$VM_MANAGEMENT_DIR")
    fi
    
    for dir in "${required_dirs[@]}"; do
        if [[ ! -d "$dir" ]]; then
            error "Required directory not found: $dir"
            return 1
        fi
    done
    
    # Check for Level 1 test implementation
    if [[ ! -f "$KERNEL_MODULE_DIR/src/level1_basic_validation.rs" ]]; then
        error "Level 1 test implementation not found: $KERNEL_MODULE_DIR/src/level1_basic_validation.rs"
        return 1
    fi
    
    # Check build system - look for kernel build system or tests Makefile
    if [[ ! -f "$PROJECT_ROOT/kernel/build/Makefile" ]] && [[ ! -f "$PROJECT_ROOT/tests/Makefile" ]]; then
        error "VexFS build system not found (checked kernel/build/Makefile and tests/Makefile)"
        return 1
    fi
    
    # Create results directory
    mkdir -p "$RESULTS_DIR"
    
    # Check sudo availability if needed
    if [[ "$ENABLE_SUDO_TESTS" == "true" ]]; then
        if ! sudo -n true 2>/dev/null; then
            warning "Sudo access required for some tests. You may be prompted for password."
        fi
    fi
    
    success "Environment validation completed for $test_level"
}

#
# Build Variant Validation
#

validate_build_variant() {
    local variant="$1"
    
    case "$variant" in
        safe|safe-ffi|c-only|standard)
            info "Using build variant: $variant"
            ;;
        *)
            error "Invalid build variant: $variant"
            error "Valid variants: safe, safe-ffi, c-only, standard"
            return 1
            ;;
    esac
}

#
# Level 1: Basic Kernel Module Validation (HOST-SAFE)
#

run_level1_tests() {
    local build_variant="$1"
    
    info "Starting Level 1: Basic Kernel Module Validation"
    info "Safety level: HOST-SAFE (no mount operations)"
    info "Build variant: $build_variant"
    
    if [[ "$DRY_RUN" == "true" ]]; then
        info "[DRY RUN] Would execute Level 1 tests with variant: $build_variant"
        return 0
    fi
    
    # Set test steps for progress tracking
    TOTAL_STEPS=7
    CURRENT_STEP=0
    
    progress "Compiling Level 1 test runner"
    
    # Compile the Rust test runner
    local test_binary="$RESULTS_DIR/level1_runner"
    
    if ! cargo build --bin level1_runner --release --manifest-path "$KERNEL_MODULE_DIR/Cargo.toml" 2>&1 | tee -a "$LOG_FILE"; then
        error "Failed to compile Level 1 test runner"
        return 1
    fi
    
    # Copy binary to results directory
    cp "$KERNEL_MODULE_DIR/target/x86_64-unknown-linux-gnu/release/level1_runner" "$test_binary"
    
    progress "Executing Level 1 test suite"
    
    # Prepare test configuration
    local config_args=(
        "--build-variant" "$build_variant"
        "--timeout" "$DEFAULT_TIMEOUT"
        "--kernel-dir" "$PROJECT_ROOT"
    )
    
    if [[ "$ENABLE_SUDO_TESTS" == "true" ]]; then
        config_args+=("--enable-sudo")
    fi
    
    if [[ "$VERBOSE" == "true" ]]; then
        config_args+=("--verbose")
    fi
    
    # Execute Level 1 tests
    if "$test_binary" "${config_args[@]}" 2>&1 | tee -a "$LOG_FILE"; then
        success "Level 1 tests completed successfully"
        
        # Display summary
        if [[ -f "$RESULTS_DIR/level1_report.json" ]]; then
            info "Test report available at: $RESULTS_DIR/level1_report.json"
        fi
        
        return 0
    else
        error "Level 1 tests failed"
        return 1
    fi
}

#
# VM Integration Layer
#

check_vm_availability() {
    info "Checking VM infrastructure availability..."
    
    # Check for VM management scripts
    local vm_scripts=(
        "$VM_MANAGEMENT_DIR/vm/start_vm.sh"
        "$VM_MANAGEMENT_DIR/vm/stop_vm.sh"
        "$VM_MANAGEMENT_DIR/vm/reset_vm.sh"
    )
    
    for script in "${vm_scripts[@]}"; do
        if [[ ! -f "$script" ]]; then
            error "VM management script not found: $script"
            return 1
        fi
        
        if [[ ! -x "$script" ]]; then
            error "VM management script not executable: $script"
            return 1
        fi
    done
    
    # Check for QEMU/KVM
    if ! command -v qemu-system-x86_64 &> /dev/null; then
        error "QEMU not found. VM testing requires QEMU/KVM."
        return 1
    fi
    
    # Check for VM image
    local vm_image="$VM_MANAGEMENT_DIR/vm/images/vexfs-test.qcow2"
    if [[ ! -f "$vm_image" ]]; then
        warning "VM image not found: $vm_image"
        warning "VM tests may require image creation"
    fi
    
    success "VM infrastructure check completed"
}

start_test_vm() {
    local vm_name="vexfs-test-$$"
    
    info "Starting test VM: $vm_name"
    
    if [[ "$DRY_RUN" == "true" ]]; then
        info "[DRY RUN] Would start VM: $vm_name"
        echo "$vm_name"
        return 0
    fi
    
    # Start VM with crash detection
    if "$VM_MANAGEMENT_DIR/vm/start_vm.sh" \
        --name "$vm_name" \
        --enable-crash-detection \
        --timeout "$DEFAULT_TIMEOUT" 2>&1 | tee -a "$LOG_FILE"; then
        
        success "Test VM started: $vm_name"
        echo "$vm_name"
        return 0
    else
        error "Failed to start test VM"
        return 1
    fi
}

stop_test_vm() {
    local vm_name="$1"
    
    info "Stopping test VM: $vm_name"
    
    if [[ "$DRY_RUN" == "true" ]]; then
        info "[DRY RUN] Would stop VM: $vm_name"
        return 0
    fi
    
    if "$VM_MANAGEMENT_DIR/vm/stop_vm.sh" --name "$vm_name" 2>&1 | tee -a "$LOG_FILE"; then
        success "Test VM stopped: $vm_name"
        return 0
    else
        warning "Failed to stop test VM cleanly"
        return 1
    fi
}

#
# Level 2: Mount Operation Testing (VM-ISOLATED)
#

run_level2_tests() {
    local build_variant="$1"
    
    info "Starting Level 2: Mount Operation Testing"
    info "Safety level: VM-ISOLATED (mount operations)"
    info "Build variant: $build_variant"
    
    if [[ "$DRY_RUN" == "true" ]]; then
        info "[DRY RUN] Would execute Level 2 tests with variant: $build_variant"
        return 0
    fi
    
    # Check VM availability
    if ! check_vm_availability; then
        error "VM infrastructure not available for Level 2 tests"
        return 1
    fi
    
    # Start test VM
    local vm_name
    if ! vm_name=$(start_test_vm); then
        error "Failed to start VM for Level 2 tests"
        return 1
    fi
    
    # Ensure VM cleanup on exit
    trap "stop_test_vm '$vm_name'" EXIT
    
    # Set test steps for progress tracking
    TOTAL_STEPS=5
    CURRENT_STEP=0
    
    progress "Deploying VexFS module to VM"
    progress "Testing mount operations"
    progress "Testing unmount operations"
    progress "Testing crash recovery"
    progress "Collecting results"
    
    # TODO: Implement Level 2 test execution
    warning "Level 2 tests not yet implemented"
    
    # Stop VM
    stop_test_vm "$vm_name"
    trap - EXIT
    
    return 0
}

#
# Level 3: Stress Testing (VM-ISOLATED)
#

run_level3_tests() {
    local build_variant="$1"
    
    info "Starting Level 3: Stress Testing"
    info "Safety level: VM-ISOLATED (stress operations)"
    info "Build variant: $build_variant"
    
    if [[ "$DRY_RUN" == "true" ]]; then
        info "[DRY RUN] Would execute Level 3 tests with variant: $build_variant"
        return 0
    fi
    
    # Check VM availability
    if ! check_vm_availability; then
        error "VM infrastructure not available for Level 3 tests"
        return 1
    fi
    
    # TODO: Implement Level 3 test execution
    warning "Level 3 tests not yet implemented"
    
    return 0
}

#
# Main Test Orchestration
#

run_all_tests() {
    local build_variant="$1"
    
    info "Starting complete VexFS kernel module test suite"
    
    # Run Level 1 (required for all subsequent levels)
    if ! run_level1_tests "$build_variant"; then
        error "Level 1 tests failed. Aborting test suite."
        return 1
    fi
    
    # Run Level 2 if VM infrastructure is available
    if check_vm_availability; then
        if ! run_level2_tests "$build_variant"; then
            warning "Level 2 tests failed, but continuing with Level 3"
        fi
        
        if ! run_level3_tests "$build_variant"; then
            warning "Level 3 tests failed"
        fi
    else
        warning "VM infrastructure not available. Skipping Level 2 and Level 3 tests."
    fi
    
    success "Complete test suite finished"
}

#
# Test Report Generation
#

generate_final_report() {
    local test_level="$1"
    local build_variant="$2"
    
    info "Generating final test report..."
    
    local report_file="$RESULTS_DIR/final_report_$(date +%Y%m%d_%H%M%S).md"
    
    cat > "$report_file" << EOF
# VexFS Kernel Module Test Report

**Test Session:** $(date)
**Test Level:** $test_level
**Build Variant:** $build_variant
**Enable Sudo:** $ENABLE_SUDO_TESTS

## Test Configuration

- Project Root: $PROJECT_ROOT
- Results Directory: $RESULTS_DIR
- Timeout: $DEFAULT_TIMEOUT seconds
- Log File: $LOG_FILE

## Test Results

EOF
    
    # Add Level 1 results if available
    if [[ -f "$RESULTS_DIR/level1_report.json" ]]; then
        echo "### Level 1: Basic Kernel Module Validation" >> "$report_file"
        echo "" >> "$report_file"
        echo "Detailed results available in: \`level1_report.json\`" >> "$report_file"
        echo "" >> "$report_file"
    fi
    
    # Add summary
    echo "## Summary" >> "$report_file"
    echo "" >> "$report_file"
    echo "Test session completed at $(date)" >> "$report_file"
    echo "" >> "$report_file"
    
    info "Final report generated: $report_file"
}

#
# Main Entry Point
#

main() {
    # Check for help flag first
    for arg in "$@"; do
        if [[ "$arg" == "--help" || "$arg" == "-h" ]]; then
            show_help
            exit 0
        fi
    done
    
    # Initialize logging
    mkdir -p "$RESULTS_DIR"
    
    # Parse command line arguments
    local test_level
    test_level=$(parse_arguments "$@")
    
    # Show banner
    echo -e "${CYAN}"
    echo "╔══════════════════════════════════════════════════════════════╗"
    echo "║                VexFS Kernel Module Testing Suite            ║"
    echo "║                                                              ║"
    echo "║  Three-Level Testing Strategy for Crash Prevention          ║"
    echo "║  Level 1: Host-Safe Basic Validation                        ║"
    echo "║  Level 2: VM-Isolated Mount Operations                      ║"
    echo "║  Level 3: VM-Isolated Stress Testing                        ║"
    echo "╚══════════════════════════════════════════════════════════════╝"
    echo -e "${NC}"
    
    info "Starting VexFS kernel module testing session"
    info "Test level: $test_level"
    info "Build variant: $DEFAULT_BUILD_VARIANT"
    info "Log file: $LOG_FILE"
    
    # Validate environment
    if ! validate_environment "$test_level"; then
        error "Environment validation failed"
        exit 1
    fi
    
    # Validate build variant
    if ! validate_build_variant "$DEFAULT_BUILD_VARIANT"; then
        error "Build variant validation failed"
        exit 1
    fi
    
    # Execute tests based on level
    local exit_code=0
    
    case "$test_level" in
        level1)
            if ! run_level1_tests "$DEFAULT_BUILD_VARIANT"; then
                exit_code=1
            fi
            ;;
        level2)
            if ! run_level2_tests "$DEFAULT_BUILD_VARIANT"; then
                exit_code=1
            fi
            ;;
        level3)
            if ! run_level3_tests "$DEFAULT_BUILD_VARIANT"; then
                exit_code=1
            fi
            ;;
        all)
            if ! run_all_tests "$DEFAULT_BUILD_VARIANT"; then
                exit_code=1
            fi
            ;;
        *)
            error "Invalid test level: $test_level"
            exit_code=1
            ;;
    esac
    
    # Generate final report
    generate_final_report "$test_level" "$DEFAULT_BUILD_VARIANT"
    
    if [[ $exit_code -eq 0 ]]; then
        success "VexFS kernel module testing completed successfully"
    else
        error "VexFS kernel module testing completed with errors"
    fi
    
    exit $exit_code
}

# Execute main function with all arguments
main "$@"
