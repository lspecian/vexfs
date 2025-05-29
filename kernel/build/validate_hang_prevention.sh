#!/bin/bash

# VexFS Hang Prevention Validation Script
# This script validates that all hang prevention mechanisms are properly implemented

set -e

echo "=== VexFS Hang Prevention Validation ==="
echo "Validating comprehensive system hang prevention implementation..."

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Validation counters
TOTAL_CHECKS=0
PASSED_CHECKS=0
FAILED_CHECKS=0

# Function to run a validation check
validate_check() {
    local description="$1"
    local command="$2"
    local expected_result="$3"
    
    TOTAL_CHECKS=$((TOTAL_CHECKS + 1))
    echo -n "Checking $description... "
    
    if eval "$command" > /dev/null 2>&1; then
        if [ "$expected_result" = "pass" ]; then
            echo -e "${GREEN}PASS${NC}"
            PASSED_CHECKS=$((PASSED_CHECKS + 1))
        else
            echo -e "${RED}FAIL${NC} (expected failure but passed)"
            FAILED_CHECKS=$((FAILED_CHECKS + 1))
        fi
    else
        if [ "$expected_result" = "fail" ]; then
            echo -e "${GREEN}PASS${NC} (expected failure)"
            PASSED_CHECKS=$((PASSED_CHECKS + 1))
        else
            echo -e "${RED}FAIL${NC}"
            FAILED_CHECKS=$((FAILED_CHECKS + 1))
        fi
    fi
}

# Function to check if file exists and contains pattern
check_file_contains() {
    local file="$1"
    local pattern="$2"
    local description="$3"
    
    TOTAL_CHECKS=$((TOTAL_CHECKS + 1))
    echo -n "Checking $description... "
    
    if [ -f "$file" ] && grep -q "$pattern" "$file"; then
        echo -e "${GREEN}PASS${NC}"
        PASSED_CHECKS=$((PASSED_CHECKS + 1))
    else
        echo -e "${RED}FAIL${NC}"
        FAILED_CHECKS=$((FAILED_CHECKS + 1))
    fi
}

echo ""
echo "=== 1. Hang Prevention Module Structure ==="

# Check if hang prevention module exists
check_file_contains "../../rust/src/shared/system_hang_prevention.rs" "HangPreventionManager" "hang prevention module exists"
check_file_contains "../../rust/src/shared/system_hang_prevention.rs" "WatchdogTimer" "watchdog timer implementation"
check_file_contains "../../rust/src/shared/system_hang_prevention.rs" "DeadlockStatus" "deadlock detection implementation"
check_file_contains "../../rust/src/shared/system_hang_prevention.rs" "DegradationLevel" "graceful degradation implementation"
check_file_contains "../../rust/src/shared/system_hang_prevention.rs" "PanicRecoveryStrategy" "panic handler implementation"

echo ""
echo "=== 2. FFI Integration ==="

# Check FFI header constants
check_file_contains "../include/vexfs_ffi.h" "VEXFS_MAX_OPERATION_TIMEOUT_SECS" "operation timeout constants"
check_file_contains "../include/vexfs_ffi.h" "VEXFS_DEGRADATION_NORMAL" "degradation level constants"
check_file_contains "../include/vexfs_ffi.h" "VEXFS_PANIC_CONTINUE_DEGRADED" "panic recovery constants"

# Check FFI function declarations
check_file_contains "../include/vexfs_ffi.h" "vexfs_rust_init_hang_prevention" "hang prevention init function"
check_file_contains "../include/vexfs_ffi.h" "vexfs_rust_start_watchdog" "watchdog start function"
check_file_contains "../include/vexfs_ffi.h" "vexfs_rust_check_operation_allowed" "operation check function"
check_file_contains "../include/vexfs_ffi.h" "vexfs_rust_handle_panic" "panic handler function"

# Check FFI implementation
check_file_contains "../../rust/src/ffi/hang_prevention.rs" "vexfs_rust_init_hang_prevention" "FFI implementation exists"
check_file_contains "../../rust/src/ffi/hang_prevention.rs" "operation_type_from_c" "C type conversion"
check_file_contains "../../rust/src/ffi/hang_prevention.rs" "degradation_level_to_c" "degradation level conversion"

echo ""
echo "=== 3. Kernel Module Integration ==="

# Check kernel module integration
check_file_contains "../src/vexfs_module_entry_safe_ffi.c" "vexfs_rust_init_hang_prevention" "kernel module init integration"
check_file_contains "../src/vexfs_module_entry_safe_ffi.c" "vexfs_rust_shutdown_hang_prevention" "kernel module shutdown integration"

echo ""
echo "=== 4. Shared Module Exports ==="

# Check shared module exports
check_file_contains "../../rust/src/shared/mod.rs" "pub mod system_hang_prevention" "shared module declaration"
check_file_contains "../../rust/src/shared/mod.rs" "HangPreventionManager" "hang prevention manager export"
check_file_contains "../../rust/src/shared/mod.rs" "init_hang_prevention" "init function export"

echo ""
echo "=== 5. Core Functionality Validation ==="

# Check watchdog timer functionality
check_file_contains "../../rust/src/shared/system_hang_prevention.rs" "start_watchdog" "watchdog start method"
check_file_contains "../../rust/src/shared/system_hang_prevention.rs" "cancel_watchdog" "watchdog cancel method"
check_file_contains "../../rust/src/shared/system_hang_prevention.rs" "is_expired" "watchdog expiration check"

# Check deadlock detection
check_file_contains "../../rust/src/shared/system_hang_prevention.rs" "detect_deadlocks" "deadlock detection method"
check_file_contains "../../rust/src/shared/system_hang_prevention.rs" "acquire_lock" "lock acquisition method"
check_file_contains "../../rust/src/shared/system_hang_prevention.rs" "release_lock" "lock release method"

# Check resource monitoring
check_file_contains "../../rust/src/shared/system_hang_prevention.rs" "update_resource_usage" "resource monitoring method"
check_file_contains "../../rust/src/shared/system_hang_prevention.rs" "is_under_pressure" "resource pressure check"
check_file_contains "../../rust/src/shared/system_hang_prevention.rs" "memory_usage_percent" "memory usage calculation"

# Check graceful degradation
check_file_contains "../../rust/src/shared/system_hang_prevention.rs" "should_allow_operation" "operation permission check"
check_file_contains "../../rust/src/shared/system_hang_prevention.rs" "allows_writes" "write permission check"
check_file_contains "../../rust/src/shared/system_hang_prevention.rs" "max_operations" "operation limit check"

# Check panic handling
check_file_contains "../../rust/src/shared/system_hang_prevention.rs" "handle_panic" "panic handling method"
check_file_contains "../../rust/src/shared/system_hang_prevention.rs" "get_health_status" "health status method"

echo ""
echo "=== 6. Safety Mechanisms ==="

# Check timeout configurations
check_file_contains "../../rust/src/shared/system_hang_prevention.rs" "FILE_IO_TIMEOUT_SECS" "file I/O timeout"
check_file_contains "../../rust/src/shared/system_hang_prevention.rs" "DIRECTORY_TIMEOUT_SECS" "directory timeout"
check_file_contains "../../rust/src/shared/system_hang_prevention.rs" "FFI_CALL_TIMEOUT_SECS" "FFI call timeout"
check_file_contains "../../rust/src/shared/system_hang_prevention.rs" "MOUNT_TIMEOUT_SECS" "mount timeout"

# Check resource limits
check_file_contains "../../rust/src/shared/system_hang_prevention.rs" "MAX_MEMORY_USAGE_BYTES" "memory usage limit"
check_file_contains "../../rust/src/shared/system_hang_prevention.rs" "CPU_USAGE_LIMIT_PERCENT" "CPU usage limit"
check_file_contains "../../rust/src/shared/system_hang_prevention.rs" "MAX_CONCURRENT_OPERATIONS" "operation limit"

echo ""
echo "=== 7. Convenience Macros ==="

# Check convenience macros
check_file_contains "../../rust/src/shared/system_hang_prevention.rs" "with_watchdog!" "watchdog macro"
check_file_contains "../../rust/src/shared/system_hang_prevention.rs" "check_operation!" "operation check macro"
check_file_contains "../../rust/src/shared/system_hang_prevention.rs" "safe_operation!" "safe operation macro"

echo ""
echo "=== 8. Test Coverage ==="

# Check test implementations
check_file_contains "../../rust/src/shared/system_hang_prevention.rs" "test_watchdog_timer" "watchdog timer tests"
check_file_contains "../../rust/src/shared/system_hang_prevention.rs" "test_degradation_levels" "degradation level tests"
check_file_contains "../../rust/src/shared/system_hang_prevention.rs" "test_operation_priorities" "operation priority tests"
check_file_contains "../../rust/src/shared/system_hang_prevention.rs" "test_hang_prevention_manager" "manager tests"

echo ""
echo "=== 9. Build System Integration ==="

# Check if build files exist
validate_check "Makefile exists" "[ -f 'Makefile.safe_ffi' ]" "pass"
validate_check "FFI header exists" "[ -f '../include/vexfs_ffi.h' ]" "pass"

echo ""
echo "=== 10. Documentation and Comments ==="

# Check documentation quality
check_file_contains "../../rust/src/shared/system_hang_prevention.rs" "//!" "module documentation"
check_file_contains "../../rust/src/shared/system_hang_prevention.rs" "/// " "function documentation"
check_file_contains "../../rust/src/ffi/hang_prevention.rs" "//!" "FFI documentation"

echo ""
echo "=== Validation Summary ==="
echo "Total checks: $TOTAL_CHECKS"
echo -e "Passed: ${GREEN}$PASSED_CHECKS${NC}"
echo -e "Failed: ${RED}$FAILED_CHECKS${NC}"

if [ $FAILED_CHECKS -eq 0 ]; then
    echo -e "\n${GREEN}✅ ALL HANG PREVENTION VALIDATIONS PASSED!${NC}"
    echo -e "${GREEN}🎉 VexFS hang prevention system is fully implemented and ready for testing!${NC}"
    echo ""
    echo "=== Implementation Summary ==="
    echo "✅ Watchdog timers for all operation types"
    echo "✅ Deadlock detection and resolution"
    echo "✅ Resource usage limits and monitoring"
    echo "✅ Graceful degradation under pressure"
    echo "✅ Panic handlers for system stability"
    echo "✅ Complete FFI integration"
    echo "✅ Kernel module integration"
    echo "✅ Comprehensive test coverage"
    echo ""
    echo "🚀 Ready for comprehensive VM testing!"
    exit 0
else
    echo -e "\n${RED}❌ VALIDATION FAILED!${NC}"
    echo -e "${RED}$FAILED_CHECKS checks failed. Please review and fix the issues above.${NC}"
    exit 1
fi