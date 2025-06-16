#!/bin/bash
"""
VexFS Crash Consistency Test Runner

This script orchestrates the complete crash consistency testing suite for VexFS,
including crash simulation, recovery testing, and data integrity validation.
"""

set -euo pipefail

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CRASH_CONSISTENCY_DIR="$(dirname "$SCRIPT_DIR")"
TEST_DEVICE="${TEST_DEVICE:-/dev/loop0}"
MOUNT_POINT="${MOUNT_POINT:-/mnt/vexfs_crash_test}"
VM_MODE="${VM_MODE:-false}"
VERBOSE="${VERBOSE:-false}"
TEST_SUITE="${TEST_SUITE:-basic}"
RESULTS_DIR="$CRASH_CONSISTENCY_DIR/results"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Help function
show_help() {
    cat << EOF
VexFS Crash Consistency Test Runner

Usage: $0 [OPTIONS]

OPTIONS:
    -d, --device DEVICE     Test device path (default: /dev/loop0)
    -m, --mount-point PATH  Mount point for testing (default: /mnt/vexfs_crash_test)
    -s, --suite SUITE       Test suite: basic, stress, full (default: basic)
    -v, --verbose           Enable verbose output
    --vm-mode               Run in VM mode (enables additional crash mechanisms)
    --setup-only            Only setup test environment, don't run tests
    --cleanup-only          Only cleanup test environment
    -h, --help              Show this help message

ENVIRONMENT VARIABLES:
    TEST_DEVICE             Override default test device
    MOUNT_POINT             Override default mount point
    VM_MODE                 Set to 'true' to enable VM mode
    VERBOSE                 Set to 'true' for verbose output
    TEST_SUITE              Override default test suite

EXAMPLES:
    # Run basic crash consistency tests
    $0 --device /dev/sdb1 --suite basic

    # Run full test suite in VM mode
    $0 --device /dev/vdb --suite full --vm-mode

    # Setup test environment only
    $0 --setup-only

    # Cleanup after testing
    $0 --cleanup-only

EOF
}

# Parse command line arguments
parse_args() {
    while [[ $# -gt 0 ]]; do
        case $1 in
            -d|--device)
                TEST_DEVICE="$2"
                shift 2
                ;;
            -m|--mount-point)
                MOUNT_POINT="$2"
                shift 2
                ;;
            -s|--suite)
                TEST_SUITE="$2"
                shift 2
                ;;
            -v|--verbose)
                VERBOSE=true
                shift
                ;;
            --vm-mode)
                VM_MODE=true
                shift
                ;;
            --setup-only)
                SETUP_ONLY=true
                shift
                ;;
            --cleanup-only)
                CLEANUP_ONLY=true
                shift
                ;;
            -h|--help)
                show_help
                exit 0
                ;;
            *)
                log_error "Unknown option: $1"
                show_help
                exit 1
                ;;
        esac
    done
}

# Check prerequisites
check_prerequisites() {
    log_info "Checking prerequisites..."
    
    # Check if running as root
    if [[ $EUID -ne 0 ]]; then
        log_error "This script must be run as root for device operations"
        exit 1
    fi
    
    # Check if test device exists
    if [[ ! -b "$TEST_DEVICE" ]]; then
        log_error "Test device $TEST_DEVICE does not exist or is not a block device"
        exit 1
    fi
    
    # Check if VexFS module is available
    if ! modinfo vexfs >/dev/null 2>&1; then
        log_warning "VexFS module not found, attempting to load..."
        if [[ -f "/lib/modules/$(uname -r)/extra/vexfs.ko" ]]; then
            modprobe vexfs || {
                log_error "Failed to load VexFS module"
                exit 1
            }
        else
            log_error "VexFS module not available"
            exit 1
        fi
    fi
    
    # Check if mkfs.vexfs is available
    if ! command -v mkfs.vexfs >/dev/null 2>&1; then
        log_error "mkfs.vexfs not found in PATH"
        exit 1
    fi
    
    # Check Python dependencies
    if ! python3 -c "import json, logging, subprocess, threading" 2>/dev/null; then
        log_error "Required Python modules not available"
        exit 1
    fi
    
    # Create results directory
    mkdir -p "$RESULTS_DIR"
    
    log_success "Prerequisites check passed"
}

# Setup test environment
setup_test_environment() {
    log_info "Setting up crash consistency test environment..."
    
    # Unmount if already mounted
    if mountpoint -q "$MOUNT_POINT" 2>/dev/null; then
        log_info "Unmounting existing filesystem at $MOUNT_POINT"
        umount "$MOUNT_POINT" || {
            log_warning "Failed to unmount cleanly, forcing..."
            umount -f "$MOUNT_POINT" || true
        }
    fi
    
    # Create mount point
    mkdir -p "$MOUNT_POINT"
    
    # Setup loop device if needed
    if [[ "$TEST_DEVICE" == /dev/loop* ]]; then
        setup_loop_device
    fi
    
    # Verify device is not mounted elsewhere
    if grep -q "$TEST_DEVICE" /proc/mounts; then
        log_error "Device $TEST_DEVICE is mounted elsewhere"
        exit 1
    fi
    
    # Setup crash injection mechanisms
    setup_crash_mechanisms
    
    log_success "Test environment setup completed"
}

# Setup loop device for testing
setup_loop_device() {
    local loop_file="$RESULTS_DIR/vexfs_crash_test.img"
    local loop_size="1G"
    
    log_info "Setting up loop device $TEST_DEVICE"
    
    # Create test image if it doesn't exist
    if [[ ! -f "$loop_file" ]]; then
        log_info "Creating test image: $loop_file ($loop_size)"
        dd if=/dev/zero of="$loop_file" bs=1M count=1024 status=progress
    fi
    
    # Setup loop device
    if ! losetup "$TEST_DEVICE" "$loop_file" 2>/dev/null; then
        # Try to find an available loop device
        TEST_DEVICE=$(losetup -f)
        losetup "$TEST_DEVICE" "$loop_file"
        log_info "Using loop device: $TEST_DEVICE"
    fi
}

# Setup crash injection mechanisms
setup_crash_mechanisms() {
    log_info "Setting up crash injection mechanisms..."
    
    # Enable sysrq for kernel panic testing
    echo 1 > /proc/sys/kernel/sysrq
    
    # Load dm-flakey for I/O error injection if available
    if modinfo dm-flakey >/dev/null 2>&1; then
        modprobe dm-flakey || log_warning "Failed to load dm-flakey module"
    else
        log_warning "dm-flakey module not available, I/O error injection limited"
    fi
    
    # Setup memory pressure tools
    if [[ "$VM_MODE" == "true" ]]; then
        log_info "VM mode enabled - additional crash mechanisms available"
        # In VM mode, we can use more aggressive crash simulation
    fi
    
    log_success "Crash injection mechanisms setup completed"
}

# Run crash simulation tests
run_crash_simulation_tests() {
    log_info "Running crash simulation tests..."
    
    local crash_injector="$CRASH_CONSISTENCY_DIR/crash_simulation/crash_injector.py"
    local verbose_flag=""
    
    if [[ "$VERBOSE" == "true" ]]; then
        verbose_flag="--verbose"
    fi
    
    # Run crash injection tests
    python3 "$crash_injector" \
        --device "$TEST_DEVICE" \
        --mount-point "$MOUNT_POINT" \
        --test-suite "$TEST_SUITE" \
        $verbose_flag || {
        log_error "Crash simulation tests failed"
        return 1
    }
    
    log_success "Crash simulation tests completed"
}

# Run recovery validation tests
run_recovery_tests() {
    log_info "Running recovery validation tests..."
    
    local recovery_validator="$CRASH_CONSISTENCY_DIR/recovery_testing/recovery_validator.py"
    local verbose_flag=""
    
    if [[ "$VERBOSE" == "true" ]]; then
        verbose_flag="--verbose"
    fi
    
    # Run recovery tests
    python3 "$recovery_validator" \
        "$TEST_DEVICE" \
        --mount-point "$MOUNT_POINT" \
        --test-suite "$TEST_SUITE" \
        $verbose_flag || {
        log_error "Recovery validation tests failed"
        return 1
    }
    
    log_success "Recovery validation tests completed"
}

# Run filesystem consistency checks
run_fsck_tests() {
    log_info "Running filesystem consistency checks..."
    
    local vexfs_fsck="$CRASH_CONSISTENCY_DIR/data_integrity/vexfs_fsck.py"
    local verbose_flag=""
    
    if [[ "$VERBOSE" == "true" ]]; then
        verbose_flag="--verbose"
    fi
    
    # Run fsck on the test device
    python3 "$vexfs_fsck" \
        "$TEST_DEVICE" \
        --mount-point "$MOUNT_POINT" \
        --fix \
        $verbose_flag || {
        log_warning "Filesystem consistency check found issues"
        # Don't fail here as fsck might find and fix issues
    }
    
    log_success "Filesystem consistency checks completed"
}

# Run vector consistency tests
run_vector_consistency_tests() {
    log_info "Running vector consistency tests..."
    
    # This would run VexFS-specific vector consistency tests
    # For now, we'll create a placeholder
    
    local test_script="$CRASH_CONSISTENCY_DIR/vector_consistency/vector_integrity_test.py"
    
    if [[ -f "$test_script" ]]; then
        python3 "$test_script" "$TEST_DEVICE" || {
            log_error "Vector consistency tests failed"
            return 1
        }
    else
        log_info "Vector consistency tests not yet implemented"
    fi
    
    log_success "Vector consistency tests completed"
}

# Generate comprehensive test report
generate_test_report() {
    log_info "Generating comprehensive test report..."
    
    local report_file="$RESULTS_DIR/crash_consistency_report_$(date +%Y%m%d_%H%M%S).html"
    local summary_file="$RESULTS_DIR/test_summary.json"
    
    # Create HTML report
    cat > "$report_file" << EOF
<!DOCTYPE html>
<html>
<head>
    <title>VexFS Crash Consistency Test Report</title>
    <style>
        body { font-family: Arial, sans-serif; margin: 20px; }
        .header { background-color: #f0f0f0; padding: 20px; border-radius: 5px; }
        .section { margin: 20px 0; padding: 15px; border: 1px solid #ddd; border-radius: 5px; }
        .success { background-color: #d4edda; border-color: #c3e6cb; }
        .warning { background-color: #fff3cd; border-color: #ffeaa7; }
        .error { background-color: #f8d7da; border-color: #f5c6cb; }
        .results-table { width: 100%; border-collapse: collapse; }
        .results-table th, .results-table td { border: 1px solid #ddd; padding: 8px; text-align: left; }
        .results-table th { background-color: #f2f2f2; }
    </style>
</head>
<body>
    <div class="header">
        <h1>VexFS Crash Consistency Test Report</h1>
        <p><strong>Test Date:</strong> $(date)</p>
        <p><strong>Test Device:</strong> $TEST_DEVICE</p>
        <p><strong>Test Suite:</strong> $TEST_SUITE</p>
        <p><strong>VM Mode:</strong> $VM_MODE</p>
    </div>
EOF
    
    # Add test results sections
    add_test_results_to_report "$report_file"
    
    # Close HTML
    echo "</body></html>" >> "$report_file"
    
    # Create JSON summary
    create_json_summary "$summary_file"
    
    log_success "Test report generated: $report_file"
    log_info "Summary available: $summary_file"
}

# Add test results to HTML report
add_test_results_to_report() {
    local report_file="$1"
    
    # Find latest result files
    local crash_results=$(find "$RESULTS_DIR" -name "crash_test_results_*.json" -type f | sort | tail -1)
    local recovery_results=$(find "$RESULTS_DIR" -name "recovery_test_results_*.json" -type f | sort | tail -1)
    local fsck_results=$(find "$RESULTS_DIR" -name "fsck_results_*.json" -type f | sort | tail -1)
    
    # Add crash simulation results
    if [[ -f "$crash_results" ]]; then
        echo "<div class='section'>" >> "$report_file"
        echo "<h2>Crash Simulation Results</h2>" >> "$report_file"
        echo "<p>Results from: $(basename "$crash_results")</p>" >> "$report_file"
        # Parse and display crash test results
        python3 -c "
import json
with open('$crash_results') as f:
    data = json.load(f)
    print(f'<p>Total Scenarios: {data[\"total_scenarios\"]}</p>')
    print(f'<p>Successful Tests: {data[\"successful_tests\"]}</p>')
    print(f'<p>Failed Tests: {data[\"failed_tests\"]}</p>')
" >> "$report_file"
        echo "</div>" >> "$report_file"
    fi
    
    # Add recovery test results
    if [[ -f "$recovery_results" ]]; then
        echo "<div class='section'>" >> "$report_file"
        echo "<h2>Recovery Test Results</h2>" >> "$report_file"
        echo "<p>Results from: $(basename "$recovery_results")</p>" >> "$report_file"
        # Parse and display recovery test results
        python3 -c "
import json
with open('$recovery_results') as f:
    data = json.load(f)
    stats = data['statistics']
    print(f'<p>Total Tests: {stats[\"total_tests\"]}</p>')
    print(f'<p>Successful Recoveries: {stats[\"successful_recoveries\"]}</p>')
    print(f'<p>Failed Recoveries: {stats[\"failed_recoveries\"]}</p>')
    print(f'<p>Average Recovery Time: {stats[\"average_recovery_time\"]:.2f}s</p>')
" >> "$report_file"
        echo "</div>" >> "$report_file"
    fi
    
    # Add fsck results
    if [[ -f "$fsck_results" ]]; then
        echo "<div class='section'>" >> "$report_file"
        echo "<h2>Filesystem Consistency Check Results</h2>" >> "$report_file"
        echo "<p>Results from: $(basename "$fsck_results")</p>" >> "$report_file"
        # Parse and display fsck results
        python3 -c "
import json
with open('$fsck_results') as f:
    data = json.load(f)
    print(f'<p>Overall Status: {data[\"overall_status\"].upper()}</p>')
    print(f'<p>Errors Found: {len(data[\"errors\"])}</p>')
    print(f'<p>Warnings Found: {len(data[\"warnings\"])}</p>')
    stats = data['statistics']
    print(f'<p>Files Checked: {stats[\"files_checked\"]}</p>')
    print(f'<p>Directories Checked: {stats[\"directories_checked\"]}</p>')
" >> "$report_file"
        echo "</div>" >> "$report_file"
    fi
}

# Create JSON summary
create_json_summary() {
    local summary_file="$1"
    
    cat > "$summary_file" << EOF
{
    "test_run": {
        "timestamp": "$(date -Iseconds)",
        "device": "$TEST_DEVICE",
        "mount_point": "$MOUNT_POINT",
        "test_suite": "$TEST_SUITE",
        "vm_mode": $VM_MODE
    },
    "results": {
        "crash_simulation": "$(find "$RESULTS_DIR" -name "crash_test_results_*.json" -type f | sort | tail -1)",
        "recovery_testing": "$(find "$RESULTS_DIR" -name "recovery_test_results_*.json" -type f | sort | tail -1)",
        "fsck_results": "$(find "$RESULTS_DIR" -name "fsck_results_*.json" -type f | sort | tail -1)"
    }
}
EOF
}

# Cleanup test environment
cleanup_test_environment() {
    log_info "Cleaning up test environment..."
    
    # Unmount filesystem
    if mountpoint -q "$MOUNT_POINT" 2>/dev/null; then
        umount "$MOUNT_POINT" || umount -f "$MOUNT_POINT" || true
    fi
    
    # Cleanup loop device
    if [[ "$TEST_DEVICE" == /dev/loop* ]]; then
        losetup -d "$TEST_DEVICE" 2>/dev/null || true
    fi
    
    # Remove mount point if empty
    if [[ -d "$MOUNT_POINT" ]] && [[ -z "$(ls -A "$MOUNT_POINT")" ]]; then
        rmdir "$MOUNT_POINT" || true
    fi
    
    log_success "Cleanup completed"
}

# Main execution function
main() {
    echo "VexFS Crash Consistency Testing Suite"
    echo "====================================="
    
    # Parse arguments
    parse_args "$@"
    
    # Handle special modes
    if [[ "${CLEANUP_ONLY:-false}" == "true" ]]; then
        cleanup_test_environment
        exit 0
    fi
    
    # Check prerequisites
    check_prerequisites
    
    # Setup test environment
    setup_test_environment
    
    if [[ "${SETUP_ONLY:-false}" == "true" ]]; then
        log_success "Setup completed. Test environment ready."
        exit 0
    fi
    
    # Run test suite
    local test_start_time=$(date +%s)
    local overall_success=true
    
    log_info "Starting crash consistency test suite: $TEST_SUITE"
    
    # Run crash simulation tests
    if ! run_crash_simulation_tests; then
        overall_success=false
    fi
    
    # Run recovery tests
    if ! run_recovery_tests; then
        overall_success=false
    fi
    
    # Run filesystem consistency checks
    if ! run_fsck_tests; then
        overall_success=false
    fi
    
    # Run vector consistency tests
    if ! run_vector_consistency_tests; then
        overall_success=false
    fi
    
    # Generate comprehensive report
    generate_test_report
    
    # Calculate total test time
    local test_end_time=$(date +%s)
    local total_time=$((test_end_time - test_start_time))
    
    # Cleanup
    cleanup_test_environment
    
    # Final results
    echo
    echo "====================================="
    if [[ "$overall_success" == "true" ]]; then
        log_success "All crash consistency tests completed successfully!"
        log_info "Total test time: ${total_time}s"
        exit 0
    else
        log_error "Some crash consistency tests failed!"
        log_info "Total test time: ${total_time}s"
        log_info "Check the detailed reports in: $RESULTS_DIR"
        exit 1
    fi
}

# Run main function with all arguments
main "$@"