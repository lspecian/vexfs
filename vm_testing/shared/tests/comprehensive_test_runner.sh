#!/bin/bash

# VexFS Comprehensive Test Runner
# Task 33.1: Create Mandatory Verification Test Suite
# 
# This script orchestrates all verification tests and integrates with fstests
# for comprehensive filesystem testing.

set -e

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
TEST_DIR="/tmp/vexfs_comprehensive_test"
RESULTS_DIR="${TEST_DIR}/results"
LOG_FILE="${RESULTS_DIR}/comprehensive_test.log"

# Test configuration
FSTESTS_DIR="/opt/xfstests"
FSTESTS_AVAILABLE=false

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m'

# Logging functions
log() {
    echo -e "${BLUE}[$(date '+%Y-%m-%d %H:%M:%S')]${NC} $1" | tee -a "$LOG_FILE"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1" | tee -a "$LOG_FILE"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1" | tee -a "$LOG_FILE"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1" | tee -a "$LOG_FILE"
}

log_info() {
    echo -e "${CYAN}[INFO]${NC} $1" | tee -a "$LOG_FILE"
}

# Test results tracking
declare -A TEST_RESULTS
TEST_COUNT=0
PASSED_COUNT=0
FAILED_COUNT=0

# Record test result
record_test_result() {
    local test_name="$1"
    local result="$2"
    local details="$3"
    
    TEST_RESULTS["$test_name"]="$result:$details"
    ((TEST_COUNT++))
    
    if [ "$result" = "PASS" ]; then
        ((PASSED_COUNT++))
        log_success "Test PASSED: $test_name"
    else
        ((FAILED_COUNT++))
        log_error "Test FAILED: $test_name - $details"
    fi
}

# Check prerequisites
check_prerequisites() {
    log "Checking comprehensive test prerequisites..."
    
    # Check if running with appropriate privileges
    if [ "$EUID" -eq 0 ]; then
        log_warning "Running as root"
    elif ! sudo -n true 2>/dev/null; then
        log_error "This script requires sudo privileges"
        exit 1
    fi
    
    # Check VexFS module
    if [ ! -f "$PROJECT_ROOT/vexfs.ko" ]; then
        log_error "VexFS module not found. Please compile first."
        exit 1
    fi
    
    # Check if fstests is available
    if [ -d "$FSTESTS_DIR" ]; then
        FSTESTS_AVAILABLE=true
        log_info "fstests found at $FSTESTS_DIR"
    else
        log_warning "fstests not found at $FSTESTS_DIR - skipping fstests integration"
    fi
    
    # Check required tools
    for tool in losetup mkfs dd sha256sum; do
        if ! command -v "$tool" &> /dev/null; then
            log_error "Required tool '$tool' is not available"
            exit 1
        fi
    done
    
    log_success "Prerequisites check completed"
}

# Setup test environment
setup_test_environment() {
    log "Setting up comprehensive test environment..."
    
    # Create test directories
    mkdir -p "$TEST_DIR" "$RESULTS_DIR"
    
    # Load VexFS module if not already loaded
    if ! lsmod | grep -q vexfs; then
        log "Loading VexFS module..."
        sudo insmod "$PROJECT_ROOT/vexfs.ko"
        
        if lsmod | grep -q vexfs; then
            log_success "VexFS module loaded successfully"
        else
            log_error "Failed to load VexFS module"
            exit 1
        fi
    else
        log_info "VexFS module already loaded"
    fi
    
    # Verify filesystem registration
    if grep -q vexfs /proc/filesystems; then
        log_success "VexFS registered in /proc/filesystems"
    else
        log_error "VexFS not registered in /proc/filesystems"
        exit 1
    fi
    
    log_success "Test environment setup completed"
}

# Run disk persistence verification test
run_disk_persistence_test() {
    log "=== RUNNING DISK PERSISTENCE VERIFICATION TEST ==="
    
    local test_script="$SCRIPT_DIR/disk_persistence_verification.sh"
    
    if [ ! -f "$test_script" ]; then
        record_test_result "Disk Persistence" "FAIL" "Test script not found"
        return 1
    fi
    
    # Make script executable
    chmod +x "$test_script"
    
    # Run the test
    if "$test_script" 2>&1 | tee -a "$LOG_FILE"; then
        record_test_result "Disk Persistence" "PASS" "All persistence tests passed"
        return 0
    else
        record_test_result "Disk Persistence" "FAIL" "Persistence verification failed"
        return 1
    fi
}

# Run reboot simulation test
run_reboot_simulation_test() {
    log "=== RUNNING REBOOT SIMULATION TEST ==="
    
    local test_script="$SCRIPT_DIR/reboot_simulation_test.sh"
    
    if [ ! -f "$test_script" ]; then
        record_test_result "Reboot Simulation" "FAIL" "Test script not found"
        return 1
    fi
    
    # Make script executable
    chmod +x "$test_script"
    
    # Run the test
    if "$test_script" 2>&1 | tee -a "$LOG_FILE"; then
        record_test_result "Reboot Simulation" "PASS" "Reboot simulation passed"
        return 0
    else
        record_test_result "Reboot Simulation" "FAIL" "Reboot simulation failed"
        return 1
    fi
}

# Run basic filesystem operations test
run_basic_operations_test() {
    log "=== RUNNING BASIC FILESYSTEM OPERATIONS TEST ==="
    
    local loop_file="${TEST_DIR}/basic_ops_test.img"
    local mount_point="${TEST_DIR}/basic_mount"
    local loop_device=""
    
    # Setup
    mkdir -p "$mount_point"
    dd if=/dev/zero of="$loop_file" bs=1M count=100 2>/dev/null
    loop_device=$(sudo losetup -f --show "$loop_file")
    
    # Create basic superblock
    cat > "${TEST_DIR}/create_basic_sb.c" << 'EOF'
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <fcntl.h>
#include <stdint.h>

#define VEXFS_MAGIC 0x56455846

struct vexfs_super_block {
    uint32_t magic;
    uint32_t version;
    uint64_t total_blocks;
    uint64_t free_blocks;
    uint32_t block_size;
    uint32_t inode_size;
    uint64_t root_inode;
    char padding[4048];
};

int main(int argc, char *argv[]) {
    if (argc != 2) return 1;
    
    int fd = open(argv[1], O_WRONLY);
    if (fd < 0) return 1;
    
    struct vexfs_super_block sb = {0};
    sb.magic = VEXFS_MAGIC;
    sb.version = 1;
    sb.total_blocks = 25600;
    sb.free_blocks = 25599;
    sb.block_size = 4096;
    sb.inode_size = 256;
    sb.root_inode = 1;
    
    write(fd, &sb, sizeof(sb));
    close(fd);
    return 0;
}
EOF
    
    gcc -o "${TEST_DIR}/create_basic_sb" "${TEST_DIR}/create_basic_sb.c"
    sudo "${TEST_DIR}/create_basic_sb" "$loop_device"
    
    # Test mount/unmount
    if sudo mount -t vexfs "$loop_device" "$mount_point"; then
        log_success "Basic mount successful"
        
        # Test basic operations
        local test_passed=true
        
        # Test file creation
        if echo "test content" | sudo tee "$mount_point/test_file.txt" > /dev/null; then
            log_success "File creation successful"
        else
            log_error "File creation failed"
            test_passed=false
        fi
        
        # Test file reading
        if sudo cat "$mount_point/test_file.txt" | grep -q "test content"; then
            log_success "File reading successful"
        else
            log_error "File reading failed"
            test_passed=false
        fi
        
        # Test directory creation
        if sudo mkdir "$mount_point/test_dir"; then
            log_success "Directory creation successful"
        else
            log_error "Directory creation failed"
            test_passed=false
        fi
        
        # Cleanup
        sudo umount "$mount_point"
        sudo losetup -d "$loop_device"
        
        if [ "$test_passed" = true ]; then
            record_test_result "Basic Operations" "PASS" "All basic operations successful"
        else
            record_test_result "Basic Operations" "FAIL" "Some basic operations failed"
        fi
    else
        sudo losetup -d "$loop_device"
        record_test_result "Basic Operations" "FAIL" "Mount failed"
    fi
}

# Run fstests integration (if available)
run_fstests_integration() {
    if [ "$FSTESTS_AVAILABLE" != true ]; then
        log_warning "Skipping fstests integration - not available"
        record_test_result "fstests Integration" "SKIP" "fstests not available"
        return 0
    fi
    
    log "=== RUNNING FSTESTS INTEGRATION ==="
    
    # Create fstests configuration for VexFS
    local fstests_config="${TEST_DIR}/vexfs_fstests.config"
    
    cat > "$fstests_config" << EOF
export TEST_DEV=/dev/loop10
export TEST_DIR=${TEST_DIR}/fstests_test
export SCRATCH_DEV=/dev/loop11
export SCRATCH_MNT=${TEST_DIR}/fstests_scratch
export FSTYP=vexfs
EOF
    
    # Setup test devices
    local test_img="${TEST_DIR}/fstests_test.img"
    local scratch_img="${TEST_DIR}/fstests_scratch.img"
    
    dd if=/dev/zero of="$test_img" bs=1M count=500 2>/dev/null
    dd if=/dev/zero of="$scratch_img" bs=1M count=500 2>/dev/null
    
    sudo losetup /dev/loop10 "$test_img" || true
    sudo losetup /dev/loop11 "$scratch_img" || true
    
    # Create basic superblocks
    sudo "${TEST_DIR}/create_basic_sb" /dev/loop10
    sudo "${TEST_DIR}/create_basic_sb" /dev/loop11
    
    # Create mount points
    mkdir -p "${TEST_DIR}/fstests_test" "${TEST_DIR}/fstests_scratch"
    
    # Run basic fstests
    cd "$FSTESTS_DIR"
    
    # Run a subset of generic tests suitable for new filesystems
    local test_list="generic/001 generic/002 generic/005 generic/006 generic/007"
    local tests_passed=0
    local tests_total=0
    
    for test in $test_list; do
        ((tests_total++))
        log "Running fstest: $test"
        
        if ./check -vexfs "$test" 2>&1 | tee -a "$LOG_FILE"; then
            ((tests_passed++))
            log_success "fstest $test passed"
        else
            log_error "fstest $test failed"
        fi
    done
    
    # Cleanup
    sudo umount "${TEST_DIR}/fstests_test" 2>/dev/null || true
    sudo umount "${TEST_DIR}/fstests_scratch" 2>/dev/null || true
    sudo losetup -d /dev/loop10 2>/dev/null || true
    sudo losetup -d /dev/loop11 2>/dev/null || true
    
    if [ "$tests_passed" -eq "$tests_total" ]; then
        record_test_result "fstests Integration" "PASS" "$tests_passed/$tests_total tests passed"
    else
        record_test_result "fstests Integration" "PARTIAL" "$tests_passed/$tests_total tests passed"
    fi
}

# Run stress tests
run_stress_tests() {
    log "=== RUNNING STRESS TESTS ==="
    
    local loop_file="${TEST_DIR}/stress_test.img"
    local mount_point="${TEST_DIR}/stress_mount"
    local loop_device=""
    
    # Setup
    mkdir -p "$mount_point"
    dd if=/dev/zero of="$loop_file" bs=1M count=200 2>/dev/null
    loop_device=$(sudo losetup -f --show "$loop_file")
    sudo "${TEST_DIR}/create_basic_sb" "$loop_device"
    
    if sudo mount -t vexfs "$loop_device" "$mount_point"; then
        local stress_passed=true
        
        # Stress test: Create many small files
        log "Stress test: Creating 1000 small files..."
        for i in {1..1000}; do
            if ! echo "stress test file $i" | sudo tee "$mount_point/stress_$i.txt" > /dev/null; then
                log_error "Failed to create stress file $i"
                stress_passed=false
                break
            fi
        done
        
        # Stress test: Create large file
        log "Stress test: Creating large file..."
        if ! dd if=/dev/zero of="$mount_point/large_file.dat" bs=1M count=50 2>/dev/null; then
            log_error "Failed to create large file"
            stress_passed=false
        fi
        
        # Stress test: Concurrent operations
        log "Stress test: Concurrent file operations..."
        for i in {1..10}; do
            (
                for j in {1..100}; do
                    echo "concurrent test $i-$j" | sudo tee "$mount_point/concurrent_${i}_${j}.txt" > /dev/null
                done
            ) &
        done
        wait
        
        # Verify all files exist
        local expected_files=$((1000 + 1000 + 1))  # stress files + concurrent files + large file
        local actual_files=$(find "$mount_point" -type f | wc -l)
        
        if [ "$actual_files" -eq "$expected_files" ]; then
            log_success "All stress test files created successfully"
        else
            log_error "File count mismatch: expected $expected_files, got $actual_files"
            stress_passed=false
        fi
        
        # Cleanup
        sudo umount "$mount_point"
        sudo losetup -d "$loop_device"
        
        if [ "$stress_passed" = true ]; then
            record_test_result "Stress Tests" "PASS" "All stress tests passed"
        else
            record_test_result "Stress Tests" "FAIL" "Some stress tests failed"
        fi
    else
        sudo losetup -d "$loop_device"
        record_test_result "Stress Tests" "FAIL" "Mount failed"
    fi
}

# Generate comprehensive report
generate_comprehensive_report() {
    local report_file="${RESULTS_DIR}/comprehensive_verification_report.md"
    
    log "Generating comprehensive verification report..."
    
    cat > "$report_file" << EOF
# VexFS Comprehensive Verification Report

**Test Date:** $(date)
**Test Duration:** $(date -d@$(($(date +%s) - START_TIME)) -u +%H:%M:%S)
**VexFS Version:** 2.0.0
**Kernel Version:** $(uname -r)

## Executive Summary

**Total Tests:** $TEST_COUNT
**Passed:** $PASSED_COUNT
**Failed:** $FAILED_COUNT
**Success Rate:** $(( (PASSED_COUNT * 100) / TEST_COUNT ))%

## Test Results Summary

| Test Category | Result | Details |
|---------------|--------|---------|
EOF

    for test_name in "${!TEST_RESULTS[@]}"; do
        local result_data="${TEST_RESULTS[$test_name]}"
        local result=$(echo "$result_data" | cut -d':' -f1)
        local details=$(echo "$result_data" | cut -d':' -f2-)
        
        local status_icon=""
        case "$result" in
            "PASS") status_icon="✅" ;;
            "FAIL") status_icon="❌" ;;
            "SKIP") status_icon="⏭️" ;;
            "PARTIAL") status_icon="⚠️" ;;
        esac
        
        echo "| $test_name | $status_icon $result | $details |" >> "$report_file"
    done

    cat >> "$report_file" << EOF

## Test Categories

### 1. Disk Persistence Verification
- **Purpose:** Verify that VexFS provides true disk-backed storage
- **Method:** Create files, unmount/remount, verify data integrity
- **File Sizes Tested:** 4KB, 1MB, 100MB, 1GB
- **Verification:** SHA-256 checksums

### 2. Reboot Simulation
- **Purpose:** Test persistence across module reload cycles
- **Method:** Module unload/reload with data verification
- **Scope:** Complete filesystem lifecycle simulation

### 3. Basic Operations
- **Purpose:** Verify fundamental filesystem operations
- **Operations:** File create/read, directory create, mount/unmount
- **Scope:** Core VFS integration functionality

### 4. fstests Integration
- **Purpose:** Industry-standard filesystem testing
- **Tests:** Generic filesystem tests from xfstests suite
- **Coverage:** Standard POSIX filesystem behavior

### 5. Stress Testing
- **Purpose:** Test filesystem under load
- **Scenarios:** Many small files, large files, concurrent operations
- **Metrics:** File creation success rate, data integrity

## System Information

- **VexFS Module:** $(lsmod | grep vexfs || echo "Not loaded")
- **Filesystem Registration:** $(grep vexfs /proc/filesystems || echo "Not registered")
- **Available Memory:** $(free -h | grep Mem | awk '{print $7}')
- **Disk Space:** $(df -h /tmp | tail -1 | awk '{print $4}') available

## Detailed Test Log

\`\`\`
$(cat "$LOG_FILE")
\`\`\`

## Conclusion

EOF

    if [ "$FAILED_COUNT" -eq 0 ]; then
        cat >> "$report_file" << EOF
**OVERALL RESULT: ✅ ALL TESTS PASSED**

The VexFS Phase 1 implementation has successfully passed all verification tests:

1. **Disk Persistence:** ✅ VERIFIED - Files persist across unmount/remount cycles
2. **Data Integrity:** ✅ VERIFIED - SHA-256 checksums match after persistence tests
3. **Module Stability:** ✅ VERIFIED - Module reload cycles work correctly
4. **Basic Operations:** ✅ VERIFIED - Core filesystem operations functional
5. **Stress Handling:** ✅ VERIFIED - Filesystem handles concurrent operations

**RECOMMENDATION:** Phase 1 implementation is ready for Phase 2 development.
EOF
    else
        cat >> "$report_file" << EOF
**OVERALL RESULT: ❌ SOME TESTS FAILED**

The VexFS Phase 1 implementation has failed $FAILED_COUNT out of $TEST_COUNT tests.

**RECOMMENDATION:** Address failing tests before proceeding to Phase 2 development.

### Failed Tests Analysis

EOF
        for test_name in "${!TEST_RESULTS[@]}"; do
            local result_data="${TEST_RESULTS[$test_name]}"
            local result=$(echo "$result_data" | cut -d':' -f1)
            local details=$(echo "$result_data" | cut -d':' -f2-)
            
            if [ "$result" = "FAIL" ]; then
                echo "- **$test_name:** $details" >> "$report_file"
            fi
        done
    fi

    cat >> "$report_file" << EOF

---
*Report generated by VexFS Comprehensive Test Suite*
*Task 33.1: Create Mandatory Verification Test Suite*
EOF

    log_success "Comprehensive report generated: $report_file"
}

# Main execution
main() {
    START_TIME=$(date +%s)
    
    log "=== VexFS COMPREHENSIVE VERIFICATION TEST SUITE ==="
    log "Task 33.1: Create Mandatory Verification Test Suite"
    log "Testing VexFS Phase 1 Implementation for Disk Persistence"
    
    check_prerequisites
    setup_test_environment
    
    # Run all test categories
    run_basic_operations_test
    run_disk_persistence_test
    run_reboot_simulation_test
    run_stress_tests
    run_fstests_integration
    
    # Generate final report
    generate_comprehensive_report
    
    # Final summary
    if [ "$FAILED_COUNT" -eq 0 ]; then
        log_success "=== ALL VERIFICATION TESTS PASSED ==="
        log_success "VexFS Phase 1 implementation verified for disk persistence"
        log_success "Ready to proceed to Phase 2 development"
    else
        log_error "=== SOME VERIFICATION TESTS FAILED ==="
        log_error "$FAILED_COUNT out of $TEST_COUNT tests failed"
        log_error "Please address failures before proceeding to Phase 2"
    fi
    
    log "Comprehensive verification report: ${RESULTS_DIR}/comprehensive_verification_report.md"
}

# Run main function
main "$@"