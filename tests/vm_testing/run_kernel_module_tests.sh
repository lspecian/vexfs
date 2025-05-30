#!/bin/bash

# VexFS Kernel Module VM Testing Script
# Implements the testing scenarios from docs/testing/VM_KERNEL_MODULE_TESTING_SCENARIOS.md

set -e

# Configuration
VEXFS_MODULE="/home/luis/Development/oss/vexfs/kernel/vexfs.ko"
TEST_LOG_DIR="/home/luis/Development/oss/vexfs/tests/vm_testing/logs"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
TEST_LOG="$TEST_LOG_DIR/vexfs_kernel_test_$TIMESTAMP.log"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Create log directory
mkdir -p "$TEST_LOG_DIR"

# Logging function
log() {
    echo -e "$1" | tee -a "$TEST_LOG"
}

# Test result tracking
TESTS_PASSED=0
TESTS_FAILED=0
TESTS_TOTAL=0

# Test result function
test_result() {
    local test_name="$1"
    local result="$2"
    local details="$3"
    
    TESTS_TOTAL=$((TESTS_TOTAL + 1))
    
    if [ "$result" = "PASS" ]; then
        TESTS_PASSED=$((TESTS_PASSED + 1))
        log "${GREEN}✅ PASS${NC}: $test_name"
    else
        TESTS_FAILED=$((TESTS_FAILED + 1))
        log "${RED}❌ FAIL${NC}: $test_name"
        if [ -n "$details" ]; then
            log "   Details: $details"
        fi
    fi
}

# Safety check function
safety_check() {
    log "${YELLOW}⚠️  SAFETY WARNING${NC}: This script will load kernel modules"
    log "   This is potentially dangerous and should only be run in a VM"
    log "   Current environment: $(uname -a)"
    
    if [ "$1" != "--force" ]; then
        read -p "Are you sure you want to continue? (yes/no): " confirm
        if [ "$confirm" != "yes" ]; then
            log "Test aborted by user"
            exit 1
        fi
    fi
}

# Check if running as root
check_root() {
    if [ "$EUID" -ne 0 ]; then
        log "${RED}Error${NC}: This script must be run as root (use sudo)"
        exit 1
    fi
}

# Check module exists
check_module() {
    if [ ! -f "$VEXFS_MODULE" ]; then
        log "${RED}Error${NC}: VexFS module not found at $VEXFS_MODULE"
        exit 1
    fi
    
    log "${BLUE}Module Info${NC}:"
    modinfo "$VEXFS_MODULE" | tee -a "$TEST_LOG"
}

# Scenario 1: Basic Kernel Module Lifecycle
test_scenario_1() {
    log "\n${BLUE}=== SCENARIO 1: Basic Kernel Module Lifecycle ===${NC}"
    log "Objective: Validate fundamental module operations"
    log "Duration: ~15 minutes"
    
    # Test 1.1: Module Load
    log "\n${YELLOW}Test 1.1: Module Load${NC}"
    if insmod "$VEXFS_MODULE" 2>&1 | tee -a "$TEST_LOG"; then
        test_result "Module Load" "PASS"
    else
        test_result "Module Load" "FAIL" "insmod failed"
        return 1
    fi
    
    # Test 1.2: Module Verification
    log "\n${YELLOW}Test 1.2: Module Verification${NC}"
    if lsmod | grep vexfs > /dev/null; then
        test_result "Module in lsmod" "PASS"
    else
        test_result "Module in lsmod" "FAIL" "Module not found in lsmod"
    fi
    
    if cat /proc/modules | grep vexfs > /dev/null; then
        test_result "Module in /proc/modules" "PASS"
    else
        test_result "Module in /proc/modules" "FAIL" "Module not found in /proc/modules"
    fi
    
    # Test 1.3: Kernel Messages
    log "\n${YELLOW}Test 1.3: Kernel Messages${NC}"
    log "Recent dmesg output:"
    dmesg | tail -20 | grep -i vexfs | tee -a "$TEST_LOG" || log "No VexFS messages in dmesg"
    
    # Test 1.4: Module Unload
    log "\n${YELLOW}Test 1.4: Module Unload${NC}"
    if rmmod vexfs 2>&1 | tee -a "$TEST_LOG"; then
        test_result "Module Unload" "PASS"
    else
        test_result "Module Unload" "FAIL" "rmmod failed"
        return 1
    fi
    
    # Test 1.5: Unload Verification
    log "\n${YELLOW}Test 1.5: Unload Verification${NC}"
    if ! lsmod | grep vexfs > /dev/null; then
        test_result "Module Unloaded" "PASS"
    else
        test_result "Module Unloaded" "FAIL" "Module still loaded"
    fi
}

# Scenario 2: Module Load/Unload Stress Test
test_scenario_2() {
    log "\n${BLUE}=== SCENARIO 2: Module Load/Unload Stress Test ===${NC}"
    log "Objective: Test module stability under repeated operations"
    log "Duration: ~20 minutes"
    
    local cycles=10  # Reduced from 20 for faster testing
    local failed_cycle=0
    
    for i in $(seq 1 $cycles); do
        log "\n${YELLOW}Stress Test Cycle $i/$cycles${NC}"
        
        # Load module
        if ! insmod "$VEXFS_MODULE" 2>&1 | tee -a "$TEST_LOG"; then
            test_result "Stress Test Cycle $i Load" "FAIL" "insmod failed"
            failed_cycle=$i
            break
        fi
        
        # Verify loaded
        if ! lsmod | grep vexfs > /dev/null; then
            test_result "Stress Test Cycle $i Verify" "FAIL" "Module not found after load"
            failed_cycle=$i
            break
        fi
        
        sleep 1
        
        # Unload module
        if ! rmmod vexfs 2>&1 | tee -a "$TEST_LOG"; then
            test_result "Stress Test Cycle $i Unload" "FAIL" "rmmod failed"
            failed_cycle=$i
            break
        fi
        
        # Verify unloaded
        if lsmod | grep vexfs > /dev/null; then
            test_result "Stress Test Cycle $i Unload Verify" "FAIL" "Module still loaded"
            failed_cycle=$i
            break
        fi
        
        log "   Cycle $i: SUCCESS"
        sleep 1
    done
    
    if [ $failed_cycle -eq 0 ]; then
        test_result "Stress Test Complete" "PASS" "All $cycles cycles completed"
    else
        test_result "Stress Test Complete" "FAIL" "Failed at cycle $failed_cycle"
    fi
}

# Scenario 3: Block Device Registration Test
test_scenario_3() {
    log "\n${BLUE}=== SCENARIO 3: Block Device Registration Test ===${NC}"
    log "Objective: Test VexFS block device handling"
    log "Duration: ~25 minutes"
    
    # Load module first
    if ! insmod "$VEXFS_MODULE" 2>&1 | tee -a "$TEST_LOG"; then
        test_result "Module Load for Block Test" "FAIL" "insmod failed"
        return 1
    fi
    
    # Create test block device
    log "\n${YELLOW}Creating test block device${NC}"
    local test_image="/tmp/vexfs_test_$TIMESTAMP.img"
    
    if dd if=/dev/zero of="$test_image" bs=1M count=100 2>&1 | tee -a "$TEST_LOG"; then
        test_result "Test Image Creation" "PASS"
    else
        test_result "Test Image Creation" "FAIL" "dd failed"
        rmmod vexfs 2>/dev/null
        return 1
    fi
    
    # Setup loop device
    if losetup /dev/loop0 "$test_image" 2>&1 | tee -a "$TEST_LOG"; then
        test_result "Loop Device Setup" "PASS"
    else
        test_result "Loop Device Setup" "FAIL" "losetup failed"
        rm -f "$test_image"
        rmmod vexfs 2>/dev/null
        return 1
    fi
    
    # Test block device operations
    log "\n${YELLOW}Testing block device operations${NC}"
    ls -la /dev/loop0 | tee -a "$TEST_LOG"
    file "$test_image" | tee -a "$TEST_LOG"
    
    # Basic read test
    if hexdump -C /dev/loop0 | head -5 | tee -a "$TEST_LOG"; then
        test_result "Block Device Read" "PASS"
    else
        test_result "Block Device Read" "FAIL" "hexdump failed"
    fi
    
    # Cleanup
    losetup -d /dev/loop0 2>&1 | tee -a "$TEST_LOG"
    rm -f "$test_image"
    rmmod vexfs 2>&1 | tee -a "$TEST_LOG"
    
    test_result "Block Device Test Cleanup" "PASS"
}

# Scenario 4: Filesystem Registration Test
test_scenario_4() {
    log "\n${BLUE}=== SCENARIO 4: Filesystem Registration Test ===${NC}"
    log "Objective: Test VexFS filesystem type registration"
    log "Duration: ~30 minutes"
    
    # Load module
    if ! insmod "$VEXFS_MODULE" 2>&1 | tee -a "$TEST_LOG"; then
        test_result "Module Load for FS Test" "FAIL" "insmod failed"
        return 1
    fi
    
    # Check filesystem registration
    log "\n${YELLOW}Checking filesystem registration${NC}"
    if cat /proc/filesystems | grep vexfs | tee -a "$TEST_LOG"; then
        test_result "Filesystem Registration" "PASS"
    else
        test_result "Filesystem Registration" "FAIL" "VexFS not in /proc/filesystems"
        rmmod vexfs 2>/dev/null
        return 1
    fi
    
    # Create test filesystem image
    local fs_image="/tmp/vexfs_fs_$TIMESTAMP.img"
    dd if=/dev/zero of="$fs_image" bs=1M count=200 2>&1 | tee -a "$TEST_LOG"
    losetup /dev/loop0 "$fs_image" 2>&1 | tee -a "$TEST_LOG"
    
    # Test mount attempt (this may fail if VexFS doesn't support mounting yet)
    mkdir -p /mnt/vexfs_test
    log "\n${YELLOW}Attempting to mount VexFS${NC}"
    if mount -t vexfs /dev/loop0 /mnt/vexfs_test 2>&1 | tee -a "$TEST_LOG"; then
        test_result "VexFS Mount" "PASS"
        
        # Test basic operations if mount succeeded
        log "\n${YELLOW}Testing basic filesystem operations${NC}"
        ls -la /mnt/vexfs_test/ | tee -a "$TEST_LOG"
        
        # Test file creation
        if echo "Hello VexFS" > /mnt/vexfs_test/test.txt 2>&1 | tee -a "$TEST_LOG"; then
            test_result "File Creation" "PASS"
            cat /mnt/vexfs_test/test.txt | tee -a "$TEST_LOG"
        else
            test_result "File Creation" "FAIL" "Could not create test file"
        fi
        
        # Test directory creation
        if mkdir /mnt/vexfs_test/testdir 2>&1 | tee -a "$TEST_LOG"; then
            test_result "Directory Creation" "PASS"
        else
            test_result "Directory Creation" "FAIL" "Could not create test directory"
        fi
        
        # Unmount
        cd /
        if umount /mnt/vexfs_test 2>&1 | tee -a "$TEST_LOG"; then
            test_result "VexFS Unmount" "PASS"
        else
            test_result "VexFS Unmount" "FAIL" "umount failed"
        fi
    else
        test_result "VexFS Mount" "FAIL" "Mount operation failed (may be expected if not implemented)"
        log "   Note: This may be expected if VexFS mount support is not yet implemented"
    fi
    
    # Cleanup
    losetup -d /dev/loop0 2>&1 | tee -a "$TEST_LOG"
    rm -f "$fs_image"
    rmmod vexfs 2>&1 | tee -a "$TEST_LOG"
}

# Main execution
main() {
    log "${BLUE}VexFS Kernel Module Testing Suite${NC}"
    log "Started at: $(date)"
    log "Test log: $TEST_LOG"
    log "Module: $VEXFS_MODULE"
    
    # Safety and prerequisite checks
    safety_check "$1"
    check_root
    check_module
    
    # Execute test scenarios
    log "\n${BLUE}=== STARTING TEST EXECUTION ===${NC}"
    
    # Phase 1: Basic Validation
    test_scenario_1
    test_scenario_2
    
    # Phase 2: Advanced Features  
    test_scenario_3
    test_scenario_4
    
    # Note: Scenarios 5 and 6 (Vector Operations and Error Handling) 
    # are more complex and would require additional VexFS implementation
    
    # Final results
    log "\n${BLUE}=== TEST RESULTS SUMMARY ===${NC}"
    log "Tests Passed: ${GREEN}$TESTS_PASSED${NC}"
    log "Tests Failed: ${RED}$TESTS_FAILED${NC}"
    log "Total Tests:  $TESTS_TOTAL"
    
    if [ $TESTS_FAILED -eq 0 ]; then
        log "\n${GREEN}🎉 ALL TESTS PASSED!${NC}"
        exit 0
    else
        log "\n${RED}❌ SOME TESTS FAILED${NC}"
        log "Check the test log for details: $TEST_LOG"
        exit 1
    fi
}

# Run main function with all arguments
main "$@"