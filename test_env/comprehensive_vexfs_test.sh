#!/bin/bash

# Comprehensive VexFS VM Testing Script
# This script performs thorough testing of VexFS kernel module in VM environment

set -e

echo "🧪 VexFS Comprehensive VM Testing Suite"
echo "======================================="

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Test results tracking
TESTS_PASSED=0
TESTS_FAILED=0
TEST_LOG="/tmp/vexfs_test_results.log"

# Initialize log
echo "VexFS VM Test Results - $(date)" > "$TEST_LOG"
echo "=================================" >> "$TEST_LOG"

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

# Check if we're in the VM
test_step "Verifying VM environment"
if [ ! -d "/mnt/vexfs_source" ]; then
    error "Not in VM environment - /mnt/vexfs_source not found"
    exit 1
fi
success "VM environment detected"

# Change to the source directory
cd /mnt/vexfs_source

# Test 1: Repository structure
test_step "Checking repository structure"
if [ ! -f "kernel/vexfs_module_entry.c" ]; then
    error "kernel/vexfs_module_entry.c not found"
else
    success "Kernel module source found"
fi

if [ ! -f "kernel/vexfs_ffi.h" ]; then
    error "kernel/vexfs_ffi.h not found"
else
    success "FFI header found"
fi

if [ ! -f "Kbuild" ]; then
    error "Kbuild file not found"
else
    success "Kbuild file found"
fi

# Test 2: Build system
test_step "Testing build system"
make clean >> "$TEST_LOG" 2>&1 || warning "Clean failed (may be normal)"

# Test C-only build first
test_step "Building C-only kernel module"
if make c-only-build >> "$TEST_LOG" 2>&1; then
    success "C-only build successful"
else
    error "C-only build failed"
    cat "$TEST_LOG" | tail -20
fi

# Test 3: Module loading/unloading
test_step "Testing kernel module loading"

# Unload if already loaded
if lsmod | grep -q vexfs; then
    info "Unloading existing vexfs module"
    sudo rmmod vexfs >> "$TEST_LOG" 2>&1 || warning "Failed to unload existing module"
fi

# Load the module
if sudo insmod vexfs.ko >> "$TEST_LOG" 2>&1; then
    success "Kernel module loaded successfully"
else
    error "Failed to load kernel module"
    dmesg | tail -10 >> "$TEST_LOG"
fi

# Verify module is loaded
test_step "Verifying module is loaded"
if lsmod | grep -q vexfs; then
    success "Module verified in kernel"
else
    error "Module not found in lsmod output"
fi

# Check kernel messages
test_step "Checking kernel messages"
VEXFS_MESSAGES=$(dmesg | grep -i vexfs | tail -5)
if [ -n "$VEXFS_MESSAGES" ]; then
    success "VexFS kernel messages found"
    echo "$VEXFS_MESSAGES" >> "$TEST_LOG"
else
    warning "No VexFS messages in dmesg"
fi

# Test 4: Module information
test_step "Testing module information"
if modinfo vexfs >> "$TEST_LOG" 2>&1; then
    success "Module info retrieved successfully"
else
    error "Failed to get module info"
fi

# Test 5: /proc/modules check
test_step "Checking /proc/modules"
if grep -q vexfs /proc/modules; then
    success "Module found in /proc/modules"
else
    error "Module not found in /proc/modules"
fi

# Test 6: Create test filesystem
test_step "Creating test filesystem image"
TEST_IMG="/tmp/vexfs_test.img"
dd if=/dev/zero of="$TEST_IMG" bs=1M count=50 >> "$TEST_LOG" 2>&1
if [ $? -eq 0 ]; then
    success "Test image created (50MB)"
else
    error "Failed to create test image"
fi

# Set up loop device
test_step "Setting up loop device"
LOOP_DEV=$(sudo losetup -f)
if sudo losetup "$LOOP_DEV" "$TEST_IMG" >> "$TEST_LOG" 2>&1; then
    success "Loop device set up: $LOOP_DEV"
else
    error "Failed to set up loop device"
fi

# Test 7: Create mkfs utility
test_step "Creating safe mkfs.vexfs utility"
cat > /tmp/mkfs_vexfs_safe.c << 'EOF'
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <fcntl.h>
#include <sys/stat.h>
#include <stdint.h>

#define VEXFS_MAGIC 0x56454653  /* "VEFS" in ASCII */

struct vexfs_superblock {
    uint32_t magic;
    uint32_t version;
    uint64_t block_size;
    uint64_t total_blocks;
    uint64_t free_blocks;
    char label[64];
    uint8_t reserved[448];
} __attribute__((packed));

int main(int argc, char *argv[]) {
    if (argc != 2) {
        fprintf(stderr, "Usage: %s <device>\n", argv[0]);
        return 1;
    }
    
    const char *device = argv[1];
    struct stat st;
    
    if (stat(device, &st) == -1) {
        perror("stat");
        return 1;
    }
    
    int fd = open(device, O_WRONLY);
    if (fd == -1) {
        perror("open");
        return 1;
    }
    
    struct vexfs_superblock sb;
    memset(&sb, 0, sizeof(sb));
    
    sb.magic = VEXFS_MAGIC;
    sb.version = 1;
    sb.block_size = 4096;
    sb.total_blocks = 12800;  // 50MB / 4KB
    sb.free_blocks = 12700;
    strcpy(sb.label, "VexFS_Test");
    
    if (write(fd, &sb, sizeof(sb)) != sizeof(sb)) {
        perror("write");
        close(fd);
        return 1;
    }
    
    fsync(fd);
    close(fd);
    
    printf("VexFS test filesystem created on %s\n", device);
    return 0;
}
EOF

if gcc -o /tmp/mkfs_vexfs_safe /tmp/mkfs_vexfs_safe.c >> "$TEST_LOG" 2>&1; then
    success "Safe mkfs utility compiled"
else
    error "Failed to compile mkfs utility"
fi

# Test 8: Format test device
test_step "Formatting test device with VexFS"
if sudo /tmp/mkfs_vexfs_safe "$LOOP_DEV" >> "$TEST_LOG" 2>&1; then
    success "Test device formatted successfully"
else
    error "Failed to format test device"
fi

# Test 9: Verify superblock
test_step "Verifying VexFS superblock"
MAGIC_CHECK=$(sudo hexdump -C "$LOOP_DEV" | head -1 | grep "53 46 45 56")
if [ -n "$MAGIC_CHECK" ]; then
    success "VexFS magic number verified"
else
    error "VexFS magic number not found"
fi

# Test 10: Mount test (read-only first)
test_step "Testing read-only mount"
sudo mkdir -p /mnt/vexfs_test
if sudo mount -t vexfs -o ro "$LOOP_DEV" /mnt/vexfs_test >> "$TEST_LOG" 2>&1; then
    success "Read-only mount successful"
    
    # Test basic operations
    test_step "Testing basic filesystem operations"
    if ls -la /mnt/vexfs_test >> "$TEST_LOG" 2>&1; then
        success "Directory listing successful"
    else
        error "Directory listing failed"
    fi
    
    # Unmount
    test_step "Unmounting filesystem"
    if sudo umount /mnt/vexfs_test >> "$TEST_LOG" 2>&1; then
        success "Unmount successful"
    else
        error "Unmount failed"
    fi
else
    error "Read-only mount failed"
    dmesg | tail -10 >> "$TEST_LOG"
fi

# Test 11: Module stress test
test_step "Module stress test (load/unload cycles)"
for i in {1..5}; do
    sudo rmmod vexfs >> "$TEST_LOG" 2>&1
    sleep 1
    sudo insmod vexfs.ko >> "$TEST_LOG" 2>&1
    if [ $? -ne 0 ]; then
        error "Stress test failed at cycle $i"
        break
    fi
done
success "Stress test completed (5 cycles)"

# Test 12: Final module unload
test_step "Final module unload"
if sudo rmmod vexfs >> "$TEST_LOG" 2>&1; then
    success "Module unloaded successfully"
else
    error "Failed to unload module"
fi

# Cleanup
test_step "Cleaning up test environment"
sudo losetup -d "$LOOP_DEV" >> "$TEST_LOG" 2>&1 || warning "Failed to detach loop device"
rm -f "$TEST_IMG" /tmp/mkfs_vexfs_safe /tmp/mkfs_vexfs_safe.c
success "Cleanup completed"

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
    echo -e "\n${GREEN}🎉 All tests passed! VexFS kernel module is ready for production testing.${NC}"
    echo "RESULT: ALL TESTS PASSED - READY FOR PRODUCTION" >> "$TEST_LOG"
    exit 0
else
    echo -e "\n${RED}❌ Some tests failed. Review the issues before production testing.${NC}"
    echo "RESULT: TESTS FAILED - NOT READY FOR PRODUCTION" >> "$TEST_LOG"
    exit 1
fi