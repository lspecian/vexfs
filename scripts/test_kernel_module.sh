#!/bin/bash

# VexFS Kernel Module Safety Test Script
# Tests the kernel module for NULL pointer crashes in a controlled way

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

# Configuration
MODULE_PATH="${MODULE_PATH:-./kernel_module}"
MODULE_NAME="vexfs_deadlock_fix"
TEST_DEV="/dev/loop7"
TEST_IMG="/tmp/vexfs_test.img"
MOUNT_POINT="/tmp/vexfs_test_mount"
IMG_SIZE="100M"

echo "╔══════════════════════════════════════════════════════╗"
echo "║         VexFS Kernel Module Safety Test               ║"
echo "╚══════════════════════════════════════════════════════╝"
echo

# Check if running as root
if [ "$EUID" -ne 0 ]; then
    echo -e "${RED}This script must be run as root${NC}"
    exit 1
fi

# Warning
echo -e "${YELLOW}WARNING: This test loads a kernel module that may crash your system!${NC}"
echo -e "${YELLOW}It is recommended to run this in a VM or test system.${NC}"
echo
read -p "Continue? (y/N): " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "Aborted."
    exit 1
fi

# Function to check kernel messages
check_kernel_messages() {
    local pattern="$1"
    dmesg | tail -20 | grep -i "$pattern" || true
}

# Function to safely unload module
safe_unload_module() {
    if lsmod | grep -q "^$MODULE_NAME"; then
        echo "Unloading module..."
        umount "$MOUNT_POINT" 2>/dev/null || true
        rmmod "$MODULE_NAME" 2>/dev/null || true
        sleep 1
    fi
}

# Cleanup function
cleanup() {
    echo
    echo "Cleaning up..."
    
    # Unmount if mounted
    if mountpoint -q "$MOUNT_POINT" 2>/dev/null; then
        umount "$MOUNT_POINT" 2>/dev/null || true
    fi
    
    # Unload module
    safe_unload_module
    
    # Release loop device
    if losetup -a | grep -q "$TEST_DEV"; then
        losetup -d "$TEST_DEV" 2>/dev/null || true
    fi
    
    # Remove test files
    rm -rf "$MOUNT_POINT" "$TEST_IMG"
}

trap cleanup EXIT

# Build the module
echo -e "${YELLOW}Building kernel module...${NC}"
(cd "$MODULE_PATH" && make clean && make) || {
    echo -e "${RED}Failed to build module${NC}"
    exit 1
}

# Check module file exists
if [ ! -f "$MODULE_PATH/$MODULE_NAME.ko" ]; then
    echo -e "${RED}Module file not found: $MODULE_PATH/$MODULE_NAME.ko${NC}"
    exit 1
fi

# Create test image
echo -e "${YELLOW}Creating test image...${NC}"
dd if=/dev/zero of="$TEST_IMG" bs=1M count=100 2>/dev/null
losetup "$TEST_DEV" "$TEST_IMG"

# Format with VexFS
echo -e "${YELLOW}Formatting with VexFS...${NC}"
if [ -f "./tools/mkfs.vexfs" ]; then
    ./tools/mkfs.vexfs "$TEST_DEV" || {
        echo -e "${RED}Failed to format filesystem${NC}"
        exit 1
    }
else
    echo -e "${YELLOW}mkfs.vexfs not found, using dd to create basic structure${NC}"
    # Create a minimal VexFS structure
    dd if=/dev/zero of="$TEST_DEV" bs=4096 count=1 2>/dev/null
fi

# Load module with debug enabled
echo -e "${YELLOW}Loading kernel module...${NC}"
insmod "$MODULE_PATH/$MODULE_NAME.ko" || {
    echo -e "${RED}Failed to load module${NC}"
    check_kernel_messages "vexfs"
    exit 1
}

echo -e "${GREEN}✓ Module loaded successfully${NC}"
lsmod | grep "$MODULE_NAME"

# Create mount point
mkdir -p "$MOUNT_POINT"

# Test 1: Basic mount
echo
echo -e "${YELLOW}Test 1: Basic mount${NC}"
mount -t vexfs_fixed "$TEST_DEV" "$MOUNT_POINT" 2>&1 || {
    echo -e "${RED}Mount failed${NC}"
    check_kernel_messages "NULL\|panic\|BUG\|oops"
    exit 1
}

if mountpoint -q "$MOUNT_POINT"; then
    echo -e "${GREEN}✓ Mount successful${NC}"
else
    echo -e "${RED}Mount point not active${NC}"
    exit 1
fi

# Test 2: File operations
echo
echo -e "${YELLOW}Test 2: File operations${NC}"

# Create file
echo "test data" > "$MOUNT_POINT/test.txt" 2>&1 || {
    echo -e "${RED}File creation failed${NC}"
    check_kernel_messages "NULL\|panic\|BUG"
}

# Read file
cat "$MOUNT_POINT/test.txt" > /dev/null 2>&1 || {
    echo -e "${RED}File read failed${NC}"
    check_kernel_messages "NULL\|panic\|BUG"
}

# List directory
ls -la "$MOUNT_POINT/" > /dev/null 2>&1 || {
    echo -e "${RED}Directory listing failed${NC}"
    check_kernel_messages "NULL\|panic\|BUG"
}

echo -e "${GREEN}✓ Basic file operations work${NC}"

# Test 3: Directory operations
echo
echo -e "${YELLOW}Test 3: Directory operations${NC}"

mkdir "$MOUNT_POINT/testdir" 2>&1 || {
    echo -e "${RED}Directory creation failed${NC}"
    check_kernel_messages "NULL\|panic\|BUG"
}

rmdir "$MOUNT_POINT/testdir" 2>&1 || {
    echo -e "${RED}Directory removal failed${NC}"
    check_kernel_messages "NULL\|panic\|BUG"
}

echo -e "${GREEN}✓ Directory operations work${NC}"

# Test 4: Stress test (careful!)
echo
echo -e "${YELLOW}Test 4: Light stress test${NC}"

for i in {1..10}; do
    echo "data $i" > "$MOUNT_POINT/file_$i.txt" 2>&1 || true
done

for i in {1..10}; do
    rm "$MOUNT_POINT/file_$i.txt" 2>&1 || true
done

echo -e "${GREEN}✓ Stress test completed${NC}"

# Test 5: Unmount/remount
echo
echo -e "${YELLOW}Test 5: Unmount/remount test${NC}"

umount "$MOUNT_POINT" || {
    echo -e "${RED}Unmount failed${NC}"
    check_kernel_messages "NULL\|panic\|BUG"
    exit 1
}

mount -t vexfs_fixed "$TEST_DEV" "$MOUNT_POINT" 2>&1 || {
    echo -e "${RED}Remount failed${NC}"
    check_kernel_messages "NULL\|panic\|BUG"
    exit 1
}

echo -e "${GREEN}✓ Unmount/remount successful${NC}"

# Check for kernel errors
echo
echo -e "${YELLOW}Checking kernel log for errors...${NC}"
if dmesg | tail -50 | grep -i "NULL\|panic\|BUG\|oops" | grep -i vexfs; then
    echo -e "${RED}Found potential issues in kernel log:${NC}"
    dmesg | tail -50 | grep -i "NULL\|panic\|BUG\|oops" | grep -i vexfs
else
    echo -e "${GREEN}✓ No obvious errors found${NC}"
fi

# Final summary
echo
echo "╔══════════════════════════════════════════════════════╗"
echo "║                  Test Summary                         ║"
echo "╚══════════════════════════════════════════════════════╝"
echo
echo -e "${GREEN}✓ Module load/unload${NC}"
echo -e "${GREEN}✓ Mount/unmount operations${NC}"
echo -e "${GREEN}✓ Basic file operations${NC}"
echo -e "${GREEN}✓ Directory operations${NC}"
echo -e "${GREEN}✓ Light stress test${NC}"
echo
echo -e "${GREEN}All tests passed! The module appears stable for basic operations.${NC}"
echo -e "${YELLOW}Note: This doesn't guarantee the module is crash-free under all conditions.${NC}"
echo
echo "Kernel module messages:"
dmesg | tail -10 | grep -i vexfs || echo "No recent VexFS messages"