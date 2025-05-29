#!/bin/bash

# Simple VexFS Kernel Module Test - Host Based
# This script tests the kernel module build and basic loading on the host system
# IMPORTANT: This is for build testing only - NO filesystem operations

# Note: Not using 'set -e' to handle dmesg permission issues gracefully
set +e

echo "🧪 VexFS Simple Kernel Module Test"
echo "=================================="

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Test results tracking
TESTS_PASSED=0
TESTS_FAILED=0
TEST_LOG="/tmp/vexfs_simple_test_results.log"

# Initialize log
echo "VexFS Simple Test Results - $(date)" > "$TEST_LOG"
echo "====================================" >> "$TEST_LOG"

# Test function
test_step() {
    echo -e "\n${YELLOW}➤ $1${NC}"
    echo "TEST: $1" >> "$TEST_LOG"
}

success() {
    echo -e "${GREEN}✅ $1${NC}"
    echo "PASS: $1" >> "$TEST_LOG"
    ((TESTS_PASSED++))
}

error() {
    echo -e "${RED}❌ $1${NC}"
    echo "FAIL: $1" >> "$TEST_LOG"
    ((TESTS_FAILED++))
    return 1
}

warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
    echo "WARN: $1" >> "$TEST_LOG"
}

info() {
    echo -e "${BLUE}ℹ️  $1${NC}"
    echo "INFO: $1" >> "$TEST_LOG"
}

# Test 1: Repository structure
test_step "Checking repository structure"
if [ ! -f "../kernel/vexfs_module_entry.c" ]; then
    error "kernel/vexfs_module_entry.c not found"
else
    success "Kernel module source found"
fi

if [ ! -f "../kernel/vexfs_ffi.h" ]; then
    error "kernel/vexfs_ffi.h not found"
else
    success "FFI header found"
fi

if [ ! -f "../Kbuild" ]; then
    error "Kbuild file not found"
else
    success "Kbuild file found"
fi

# Test 2: Build system
test_step "Testing build system"
cd ..
make clean >> "$TEST_LOG" 2>&1 || warning "Clean failed (may be normal)"

# Test C-only build
test_step "Building C-only kernel module"
if make c-only-build >> "$TEST_LOG" 2>&1; then
    success "C-only build successful"
else
    error "C-only build failed"
    echo "Build log:" >> "$TEST_LOG"
    tail -20 "$TEST_LOG" | head -10
    cd test_env
    exit 1
fi
cd test_env

# Test 3: Module file verification
test_step "Verifying kernel module file"
if [ -f "../vexfs.ko" ]; then
    success "Kernel module file created: vexfs.ko"
    
    # Get module info
    if modinfo ../vexfs.ko >> "$TEST_LOG" 2>&1; then
        success "Module info retrieved successfully"
    else
        error "Failed to get module info"
    fi
else
    error "Kernel module file not found"
fi

# Test 4: Module loading test (CAREFUL - only if build succeeded)
if [ -f "../vexfs.ko" ] && [ $TESTS_FAILED -eq 0 ]; then
    test_step "Testing kernel module loading (SAFE TEST)"
    
    # Check if module is already loaded
    if lsmod | grep -q vexfs; then
        info "VexFS module already loaded, unloading first"
        sudo rmmod vexfs >> "$TEST_LOG" 2>&1 || warning "Failed to unload existing module"
    fi
    
    # Load the module
    if sudo insmod ../vexfs.ko >> "$TEST_LOG" 2>&1; then
        success "Kernel module loaded successfully"
        
        # Verify it's loaded
        if lsmod | grep -q vexfs; then
            success "Module verified in kernel"
        else
            error "Module not found in lsmod output"
        fi
        
        # Check kernel messages (try with sudo first, fallback to user)
        VEXFS_MESSAGES=$(sudo dmesg 2>/dev/null | grep -i vexfs | tail -3 || dmesg 2>/dev/null | grep -i vexfs | tail -3 || echo "")
        if [ -n "$VEXFS_MESSAGES" ]; then
            success "VexFS kernel messages found"
            echo "$VEXFS_MESSAGES" >> "$TEST_LOG"
        else
            warning "No VexFS messages accessible (permission issue)"
        fi
        
        # Unload the module
        test_step "Unloading kernel module"
        if sudo rmmod vexfs >> "$TEST_LOG" 2>&1; then
            success "Module unloaded successfully"
        else
            error "Failed to unload module"
        fi
        
        # Verify it's unloaded
        if ! lsmod | grep -q vexfs; then
            success "Module successfully removed from kernel"
        else
            error "Module still loaded after rmmod"
        fi
    else
        error "Failed to load kernel module"
        echo "Recent dmesg:" >> "$TEST_LOG"
        dmesg | tail -5 >> "$TEST_LOG"
    fi
else
    warning "Skipping module loading test due to build failures"
fi

# Test Results Summary
echo -e "\n${BLUE}📊 Test Results Summary${NC}"
echo "======================="
echo -e "Tests Passed: ${GREEN}$TESTS_PASSED${NC}"
echo -e "Tests Failed: ${RED}$TESTS_FAILED${NC}"
echo -e "Total Tests: $((TESTS_PASSED + TESTS_FAILED))"

# Write summary to log
echo "" >> "$TEST_LOG"
echo "SUMMARY:" >> "$TEST_LOG"
echo "Tests Passed: $TESTS_PASSED" >> "$TEST_LOG"
echo "Tests Failed: $TESTS_FAILED" >> "$TEST_LOG"
echo "Total Tests: $((TESTS_PASSED + TESTS_FAILED))" >> "$TEST_LOG"

if [ $TESTS_FAILED -eq 0 ]; then
    echo -e "\n${GREEN}🎉 All tests passed! VexFS kernel module builds and loads correctly.${NC}"
    echo -e "${GREEN}✅ Ready for VM-based filesystem testing.${NC}"
    echo "RESULT: ALL TESTS PASSED - READY FOR FILESYSTEM TESTING" >> "$TEST_LOG"
    exit 0
else
    echo -e "\n${RED}❌ Some tests failed. Fix issues before proceeding.${NC}"
    echo -e "${RED}🚫 NOT ready for filesystem testing.${NC}"
    echo "RESULT: TESTS FAILED - FIX ISSUES BEFORE PROCEEDING" >> "$TEST_LOG"
    exit 1
fi