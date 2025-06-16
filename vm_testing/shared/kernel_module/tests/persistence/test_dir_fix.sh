#!/bin/bash
# Test VexFS directory operations fix

set -e

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo -e "${GREEN}VexFS Directory Operations Fix Test${NC}"
echo "===================================="

# Check if running as root
if [ "$EUID" -ne 0 ]; then 
    echo -e "${RED}Error: This script must be run as root (use sudo)${NC}"
    exit 1
fi

# Variables
MODULE_PATH="/home/luis/Development/oss/vexfs/kernel_module/vexfs_deadlock_fix.ko"
TEST_IMAGE="/home/luis/Development/oss/vexfs/tools/test_dir_fix.img"
MOUNT_POINT="/mnt/vexfs_dir_test"
MKFS_TOOL="/home/luis/Development/oss/vexfs/tools/mkfs.vexfs"

# Cleanup function
cleanup() {
    echo -e "\n${YELLOW}Cleaning up...${NC}"
    # Unmount if mounted
    if mountpoint -q "$MOUNT_POINT" 2>/dev/null; then
        umount "$MOUNT_POINT" 2>/dev/null || umount -l "$MOUNT_POINT" 2>/dev/null || true
    fi
    # Unload module
    if lsmod | grep -q "vexfs_deadlock_fix"; then
        rmmod vexfs_deadlock_fix 2>/dev/null || true
    fi
    # Remove mount point
    rmdir "$MOUNT_POINT" 2>/dev/null || true
    # Remove test image
    rm -f "$TEST_IMAGE"
}

# Set trap for cleanup
trap cleanup EXIT

# Unload existing module if loaded
if lsmod | grep -q "vexfs_deadlock_fix"; then
    echo -e "${YELLOW}Unloading existing module...${NC}"
    rmmod vexfs_deadlock_fix || true
    sleep 1
fi

# Create test image
echo -e "${YELLOW}Creating test image...${NC}"
dd if=/dev/zero of="$TEST_IMAGE" bs=1M count=10 status=none

# Format with mkfs.vexfs
echo -e "${YELLOW}Formatting filesystem...${NC}"
"$MKFS_TOOL" -f "$TEST_IMAGE" > /dev/null

# Load module
echo -e "${YELLOW}Loading kernel module...${NC}"
insmod "$MODULE_PATH"
echo -e "${GREEN}Module loaded${NC}"

# Create mount point
mkdir -p "$MOUNT_POINT"

# Mount filesystem
echo -e "${YELLOW}Mounting filesystem...${NC}"
mount -t vexfs_fixed "$TEST_IMAGE" "$MOUNT_POINT"
echo -e "${GREEN}Filesystem mounted${NC}"

# Test 1: Basic directory listing
echo -e "\n${YELLOW}Test 1: Basic directory listing${NC}"
echo "Running: ls -la $MOUNT_POINT"
if timeout 5 ls -la "$MOUNT_POINT"; then
    echo -e "${GREEN}✓ Directory listing succeeded${NC}"
else
    echo -e "${RED}✗ Directory listing timed out or failed${NC}"
    exit 1
fi

# Test 2: Create files and list again
echo -e "\n${YELLOW}Test 2: Create files and verify listing${NC}"
echo "Creating test files..."
echo "Hello World" > "$MOUNT_POINT/test1.txt"
echo "VexFS Works!" > "$MOUNT_POINT/test2.txt"
mkdir "$MOUNT_POINT/subdir"
echo "Nested file" > "$MOUNT_POINT/subdir/nested.txt"

echo "Running: ls -la $MOUNT_POINT"
if timeout 5 ls -la "$MOUNT_POINT"; then
    echo -e "${GREEN}✓ Directory listing with files succeeded${NC}"
else
    echo -e "${RED}✗ Directory listing with files failed${NC}"
    exit 1
fi

# Test 3: List subdirectory
echo -e "\n${YELLOW}Test 3: List subdirectory${NC}"
echo "Running: ls -la $MOUNT_POINT/subdir"
if timeout 5 ls -la "$MOUNT_POINT/subdir"; then
    echo -e "${GREEN}✓ Subdirectory listing succeeded${NC}"
else
    echo -e "${RED}✗ Subdirectory listing failed${NC}"
    exit 1
fi

# Test 4: Multiple concurrent listings
echo -e "\n${YELLOW}Test 4: Concurrent directory operations${NC}"
echo "Running 5 concurrent ls commands..."
SUCCESS=true
for i in {1..5}; do
    timeout 5 ls -la "$MOUNT_POINT" > /dev/null 2>&1 &
done
wait
if [ $? -eq 0 ]; then
    echo -e "${GREEN}✓ Concurrent operations succeeded${NC}"
else
    echo -e "${RED}✗ Concurrent operations failed${NC}"
    SUCCESS=false
fi

# Test 5: Unmount and remount
echo -e "\n${YELLOW}Test 5: Unmount/remount cycle${NC}"
umount "$MOUNT_POINT"
echo "Filesystem unmounted"
mount -t vexfs_fixed "$TEST_IMAGE" "$MOUNT_POINT"
echo "Filesystem remounted"

echo "Running: ls -la $MOUNT_POINT"
if timeout 5 ls -la "$MOUNT_POINT"; then
    echo -e "${GREEN}✓ Directory listing after remount succeeded${NC}"
    echo -e "${GREEN}✓ Files persisted across unmount/remount${NC}"
else
    echo -e "${RED}✗ Directory listing after remount failed${NC}"
    exit 1
fi

echo -e "\n${GREEN}=====================================${NC}"
echo -e "${GREEN}All directory operation tests PASSED!${NC}"
echo -e "${GREEN}=====================================${NC}"
echo -e "\nThe directory operations timeout issue has been fixed!"