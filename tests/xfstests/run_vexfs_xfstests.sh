#!/bin/bash
#
# VexFS v2.0 xfstests Runner Script
#
# This script runs xfstests on VexFS v2.0 to validate POSIX compliance
# and filesystem behavior. It handles VexFS-specific setup, test execution,
# and result analysis.
#
# Usage: ./run_vexfs_xfstests.sh [options] [test_groups]
#

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
VEXFS_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
XFSTESTS_DIR="${SCRIPT_DIR}/xfstests-dev"
CONFIG_DIR="${SCRIPT_DIR}/configs"
RESULTS_DIR="${SCRIPT_DIR}/results"

# Default configuration
RUN_SETUP=true
RUN_CLEANUP=true
TEST_GROUPS="generic quick"
EXCLUDE_TESTS=""
PARALLEL_JOBS=1
TIMEOUT_FACTOR=2
DEBUG_LEVEL=1
SAVE_LOGS=true
GENERATE_REPORT=true

# VexFS-specific paths
VEXFS_MODULE="${VEXFS_ROOT}/kernel/vexfs_v2_build/vexfs_v2_b62.ko"
VEXFS_UAPI="${VEXFS_ROOT}/kernel/vexfs_v2_build/vexfs_v2_uapi.h"
MKFS_VEXFS="${VEXFS_ROOT}/mkfs.vexfs"

echo "🧪 VexFS v2.0 xfstests Runner"
echo "============================"
echo "VexFS root: ${VEXFS_ROOT}"
echo "xfstests directory: ${XFSTESTS_DIR}"
echo "Results directory: ${RESULTS_DIR}"
echo ""

# Function to show usage
show_usage() {
    cat << EOF
Usage: $0 [options] [test_groups]

Options:
    --no-setup              Skip initial setup (devices, module loading)
    --no-cleanup            Skip cleanup after tests
    --exclude TESTS         Comma-separated list of tests to exclude
    --parallel JOBS         Number of parallel test jobs (default: 1)
    --timeout-factor N      Timeout multiplier for VexFS (default: 2)
    --debug-level N         Debug level 0-3 (default: 1)
    --no-logs               Don't save detailed logs
    --no-report             Don't generate HTML report
    --help                  Show this help

Test Groups:
    quick                   Quick smoke tests (~30 minutes)
    generic                 Generic filesystem tests (~2 hours)
    stress                  Stress and performance tests (~4 hours)
    posix                   POSIX compliance tests (~1 hour)
    vexfs                   VexFS-specific tests (~30 minutes)
    all                     All applicable tests (~8 hours)

Examples:
    $0                      # Run default tests (generic quick)
    $0 quick                # Run only quick tests
    $0 generic stress       # Run generic and stress tests
    $0 --parallel 4 all     # Run all tests with 4 parallel jobs
    $0 --exclude generic/001,generic/002 quick
EOF
}

# Function to parse command line arguments
parse_arguments() {
    while [[ $# -gt 0 ]]; do
        case $1 in
            --no-setup)
                RUN_SETUP=false
                shift
                ;;
            --no-cleanup)
                RUN_CLEANUP=false
                shift
                ;;
            --exclude)
                EXCLUDE_TESTS="$2"
                shift 2
                ;;
            --parallel)
                PARALLEL_JOBS="$2"
                shift 2
                ;;
            --timeout-factor)
                TIMEOUT_FACTOR="$2"
                shift 2
                ;;
            --debug-level)
                DEBUG_LEVEL="$2"
                shift 2
                ;;
            --no-logs)
                SAVE_LOGS=false
                shift
                ;;
            --no-report)
                GENERATE_REPORT=false
                shift
                ;;
            --help)
                show_usage
                exit 0
                ;;
            -*)
                echo "Unknown option: $1"
                show_usage
                exit 1
                ;;
            *)
                # Remaining arguments are test groups
                TEST_GROUPS="$*"
                break
                ;;
        esac
    done
}

# Function to check prerequisites
check_prerequisites() {
    echo "🔍 Checking prerequisites..."
    
    # Check if xfstests is built
    if [ ! -f "${XFSTESTS_DIR}/check" ]; then
        echo "❌ xfstests not found. Please run setup_xfstests.sh first"
        exit 1
    fi
    
    # Check if VexFS kernel module exists
    if [ ! -f "${VEXFS_MODULE}" ]; then
        echo "❌ VexFS kernel module not found: ${VEXFS_MODULE}"
        echo "Please build the VexFS kernel module first"
        exit 1
    fi
    
    # Check if VexFS UAPI header exists
    if [ ! -f "${VEXFS_UAPI}" ]; then
        echo "❌ VexFS UAPI header not found: ${VEXFS_UAPI}"
        exit 1
    fi
    
    # Check if mkfs.vexfs exists
    if [ ! -f "${MKFS_VEXFS}" ]; then
        echo "⚠️  mkfs.vexfs not found: ${MKFS_VEXFS}"
        echo "Some tests may fail without mkfs support"
    fi
    
    # Check if device configuration exists
    if [ ! -f "${SCRIPT_DIR}/device_config.env" ]; then
        echo "❌ Device configuration not found"
        echo "Please run setup_test_devices.sh first"
        exit 1
    fi
    
    echo "✅ Prerequisites check passed"
}

# Function to load VexFS module
load_vexfs_module() {
    echo "📦 Loading VexFS kernel module..."
    
    # Unload existing module if loaded
    if lsmod | grep -q vexfs_v2; then
        echo "Unloading existing VexFS module..."
        sudo rmmod vexfs_v2 || true
    fi
    
    # Load VexFS module
    echo "Loading VexFS module: ${VEXFS_MODULE}"
    sudo insmod "${VEXFS_MODULE}"
    
    # Verify module is loaded
    if ! lsmod | grep -q vexfs_v2; then
        echo "❌ Failed to load VexFS module"
        exit 1
    fi
    
    echo "✅ VexFS module loaded successfully"
    
    # Show module information
    echo "Module information:"
    modinfo "${VEXFS_MODULE}" | head -10
}

# Function to setup test environment
setup_test_environment() {
    echo "⚙️  Setting up test environment..."
    
    # Load device configuration
    source "${SCRIPT_DIR}/device_config.env"
    
    # Create results directory
    mkdir -p "${RESULTS_DIR}"
    
    # Create test timestamp
    TEST_TIMESTAMP=$(date +"%Y%m%d_%H%M%S")
    export TEST_TIMESTAMP
    
    # Set up environment variables for xfstests
    export FSTYP=vexfs
    export RESULT_BASE="${RESULTS_DIR}/${TEST_TIMESTAMP}"
    export TIMEOUT_FACTOR
    export DEBUG_LEVEL
    
    # VexFS-specific environment
    export VEXFS_MODULE_PATH="${VEXFS_MODULE}"
    export VEXFS_UAPI_HEADER="${VEXFS_UAPI}"
    export VEXFS_DEBUG_LEVEL="${DEBUG_LEVEL}"
    
    # Create result directory for this run
    mkdir -p "${RESULT_BASE}"
    
    echo "✅ Test environment configured"
    echo "Test devices: ${TEST_DEV} (test), ${SCRATCH_DEV} (scratch)"
    echo "Mount points: ${TEST_DIR} (test), ${SCRATCH_MNT} (scratch)"
    echo "Results: ${RESULT_BASE}"
}

# Function to format VexFS devices
format_vexfs_devices() {
    echo "💾 Formatting VexFS devices..."
    
    # Unmount devices if mounted
    sudo umount "${TEST_DIR}" 2>/dev/null || true
    sudo umount "${SCRATCH_MNT}" 2>/dev/null || true
    
    # Format test device
    echo "Formatting test device: ${TEST_DEV}"
    if [ -f "${MKFS_VEXFS}" ]; then
        sudo "${MKFS_VEXFS}" "${TEST_DEV}"
    else
        echo "⚠️  mkfs.vexfs not available, using basic initialization"
        # Create a simple filesystem structure
        sudo dd if=/dev/zero of="${TEST_DEV}" bs=1M count=1 2>/dev/null
    fi
    
    # Format scratch device
    echo "Formatting scratch device: ${SCRATCH_DEV}"
    if [ -f "${MKFS_VEXFS}" ]; then
        sudo "${MKFS_VEXFS}" "${SCRATCH_DEV}"
    else
        sudo dd if=/dev/zero of="${SCRATCH_DEV}" bs=1M count=1 2>/dev/null
    fi
    
    echo "✅ VexFS devices formatted"
}

# Function to mount VexFS devices
mount_vexfs_devices() {
    echo "🔗 Mounting VexFS devices..."
    
    # Mount test device
    echo "Mounting test device: ${TEST_DEV} -> ${TEST_DIR}"
    sudo mount -t vexfs "${TEST_DEV}" "${TEST_DIR}"
    
    # Verify mount
    if ! mountpoint -q "${TEST_DIR}"; then
        echo "❌ Failed to mount test device"
        exit 1
    fi
    
    echo "✅ VexFS devices mounted successfully"
    
    # Show mount information
    echo "Mount information:"
    mount | grep vexfs
}

# Function to create VexFS-specific test configuration
create_test_config() {
    echo "📝 Creating test configuration..."
    
    # Create xfstests local configuration
    cat > "${XFSTESTS_DIR}/local.config" << EOF
# VexFS v2.0 xfstests Configuration
# Generated on $(date)

export FSTYP=vexfs
export TEST_DEV="${TEST_DEV}"
export TEST_DIR="${TEST_DIR}"
export SCRATCH_DEV="${SCRATCH_DEV}"
export SCRATCH_MNT="${SCRATCH_MNT}"

# VexFS-specific settings
export MOUNT_OPTIONS=""
export MKFS_OPTIONS=""
export FSCK_OPTIONS=""

# Test execution settings
export LOAD_FACTOR=1
export TIMEOUT_FACTOR=${TIMEOUT_FACTOR}
export FSSTRESS_AVOID="-f resvsp=0 -f unresvsp=0"

# Result settings
export RESULT_BASE="${RESULT_BASE}"
export REPORT_EMAIL=""

# VexFS module information
export VEXFS_MODULE_PATH="${VEXFS_MODULE}"
export VEXFS_UAPI_HEADER="${VEXFS_UAPI}"
export VEXFS_DEBUG_LEVEL=${DEBUG_LEVEL}
EOF

    echo "✅ Test configuration created"
}

# Function to run xfstests
run_xfstests() {
    echo "🧪 Running xfstests..."
    
    cd "${XFSTESTS_DIR}"
    
    # Create exclude list
    EXCLUDE_FILE="${RESULT_BASE}/exclude_list.txt"
    if [ -n "${EXCLUDE_TESTS}" ]; then
        echo "${EXCLUDE_TESTS}" | tr ',' '\n' > "${EXCLUDE_FILE}"
        echo "Excluding tests: ${EXCLUDE_TESTS}"
    fi
    
    # Prepare test command
    TEST_CMD="./check"
    
    if [ -f "${EXCLUDE_FILE}" ]; then
        TEST_CMD="${TEST_CMD} -E ${EXCLUDE_FILE}"
    fi
    
    if [ "${PARALLEL_JOBS}" -gt 1 ]; then
        TEST_CMD="${TEST_CMD} -j ${PARALLEL_JOBS}"
    fi
    
    # Add test groups
    for group in ${TEST_GROUPS}; do
        case ${group} in
            quick)
                TEST_CMD="${TEST_CMD} -g quick"
                ;;
            generic)
                TEST_CMD="${TEST_CMD} -g generic"
                ;;
            stress)
                TEST_CMD="${TEST_CMD} -g stress"
                ;;
            posix)
                TEST_CMD="${TEST_CMD} -g posix"
                ;;
            vexfs)
                # VexFS-specific tests (we'll create these)
                TEST_CMD="${TEST_CMD} tests/vexfs/*"
                ;;
            all)
                TEST_CMD="${TEST_CMD} -g auto"
                ;;
            *)
                echo "⚠️  Unknown test group: ${group}"
                ;;
        esac
    done
    
    echo "Test command: ${TEST_CMD}"
    echo "Starting test execution at $(date)"
    
    # Run tests with logging
    if [ "${SAVE_LOGS}" = true ]; then
        ${TEST_CMD} 2>&1 | tee "${RESULT_BASE}/test_execution.log"
    else
        ${TEST_CMD}
    fi
    
    echo "Test execution completed at $(date)"
}

# Function to collect test results
collect_results() {
    echo "📊 Collecting test results..."
    
    cd "${XFSTESTS_DIR}"
    
    # Copy results to our results directory
    if [ -d "results" ]; then
        cp -r results/* "${RESULT_BASE}/" 2>/dev/null || true
    fi
    
    # Collect system information
    cat > "${RESULT_BASE}/system_info.txt" << EOF
VexFS v2.0 xfstests Results
===========================
Test run: ${TEST_TIMESTAMP}
Date: $(date)
Hostname: $(hostname)
Kernel: $(uname -r)
Architecture: $(uname -m)

VexFS Module Information:
$(modinfo "${VEXFS_MODULE}" 2>/dev/null || echo "Module info not available")

Test Configuration:
Test groups: ${TEST_GROUPS}
Parallel jobs: ${PARALLEL_JOBS}
Timeout factor: ${TIMEOUT_FACTOR}
Debug level: ${DEBUG_LEVEL}
Excluded tests: ${EXCLUDE_TESTS}

Device Information:
Test device: ${TEST_DEV}
Scratch device: ${SCRATCH_DEV}
Test mount: ${TEST_DIR}
Scratch mount: ${SCRATCH_MNT}

Mount Information:
$(mount | grep vexfs || echo "No VexFS mounts found")

Disk Usage:
$(df -h "${TEST_DIR}" "${SCRATCH_MNT}" 2>/dev/null || echo "Mount points not available")
EOF

    # Collect dmesg logs
    if [ "${SAVE_LOGS}" = true ]; then
        echo "Collecting kernel logs..."
        dmesg | grep -i vexfs > "${RESULT_BASE}/vexfs_dmesg.log" 2>/dev/null || true
        dmesg > "${RESULT_BASE}/full_dmesg.log"
    fi
    
    echo "✅ Results collected in ${RESULT_BASE}"
}

# Function to generate test report
generate_report() {
    if [ "${GENERATE_REPORT}" = false ]; then
        return
    fi
    
    echo "📋 Generating test report..."
    
    # Create HTML report
    cat > "${RESULT_BASE}/test_report.html" << EOF
<!DOCTYPE html>
<html>
<head>
    <title>VexFS v2.0 xfstests Report - ${TEST_TIMESTAMP}</title>
    <style>
        body { font-family: Arial, sans-serif; margin: 20px; }
        .header { background-color: #f0f0f0; padding: 20px; border-radius: 5px; }
        .section { margin: 20px 0; }
        .pass { color: green; font-weight: bold; }
        .fail { color: red; font-weight: bold; }
        .skip { color: orange; font-weight: bold; }
        table { border-collapse: collapse; width: 100%; }
        th, td { border: 1px solid #ddd; padding: 8px; text-align: left; }
        th { background-color: #f2f2f2; }
        .log { background-color: #f8f8f8; padding: 10px; font-family: monospace; white-space: pre-wrap; }
    </style>
</head>
<body>
    <div class="header">
        <h1>VexFS v2.0 xfstests Report</h1>
        <p><strong>Test Run:</strong> ${TEST_TIMESTAMP}</p>
        <p><strong>Date:</strong> $(date)</p>
        <p><strong>Test Groups:</strong> ${TEST_GROUPS}</p>
    </div>
    
    <div class="section">
        <h2>Test Summary</h2>
        <p>Detailed test results will be populated by the result parser.</p>
        <p>Check the following files for detailed information:</p>
        <ul>
            <li><a href="test_execution.log">Test Execution Log</a></li>
            <li><a href="system_info.txt">System Information</a></li>
            <li><a href="vexfs_dmesg.log">VexFS Kernel Messages</a></li>
        </ul>
    </div>
    
    <div class="section">
        <h2>System Information</h2>
        <div class="log">$(cat "${RESULT_BASE}/system_info.txt")</div>
    </div>
</body>
</html>
EOF

    echo "✅ Test report generated: ${RESULT_BASE}/test_report.html"
}

# Function to cleanup test environment
cleanup_test_environment() {
    if [ "${RUN_CLEANUP}" = false ]; then
        return
    fi
    
    echo "🧹 Cleaning up test environment..."
    
    # Unmount VexFS devices
    sudo umount "${TEST_DIR}" 2>/dev/null || true
    sudo umount "${SCRATCH_MNT}" 2>/dev/null || true
    
    # Clean up loop devices if used
    if [ "${USE_LOOP_DEVICES}" = true ]; then
        sudo losetup -d "${TEST_DEV}" 2>/dev/null || true
        sudo losetup -d "${SCRATCH_DEV}" 2>/dev/null || true
    fi
    
    # Clean up RAM disks if used
    if [ "${USE_RAM_DISK}" = true ]; then
        sudo umount /mnt/vexfs_ram_test 2>/dev/null || true
        sudo umount /mnt/vexfs_ram_scratch 2>/dev/null || true
    fi
    
    echo "✅ Cleanup completed"
}

# Function to show test summary
show_summary() {
    echo ""
    echo "🎉 VexFS v2.0 xfstests Execution Summary"
    echo "========================================"
    echo "Test run: ${TEST_TIMESTAMP}"
    echo "Test groups: ${TEST_GROUPS}"
    echo "Results directory: ${RESULT_BASE}"
    echo ""
    
    if [ -f "${RESULT_BASE}/test_report.html" ]; then
        echo "📋 Test report: ${RESULT_BASE}/test_report.html"
    fi
    
    if [ -f "${RESULT_BASE}/test_execution.log" ]; then
        echo "📝 Execution log: ${RESULT_BASE}/test_execution.log"
    fi
    
    echo ""
    echo "To view results:"
    echo "  cd ${RESULT_BASE}"
    echo "  firefox test_report.html"
    echo ""
}

# Main execution function
main() {
    parse_arguments "$@"
    
    echo "Starting VexFS xfstests execution..."
    echo "Test groups: ${TEST_GROUPS}"
    echo "Parallel jobs: ${PARALLEL_JOBS}"
    echo ""
    
    check_prerequisites
    
    if [ "${RUN_SETUP}" = true ]; then
        load_vexfs_module
        setup_test_environment
        format_vexfs_devices
        mount_vexfs_devices
        create_test_config
    fi
    
    run_xfstests
    collect_results
    generate_report
    
    if [ "${RUN_CLEANUP}" = true ]; then
        cleanup_test_environment
    fi
    
    show_summary
}

# Trap to ensure cleanup on exit
trap cleanup_test_environment EXIT

# Execute main function
main "$@"