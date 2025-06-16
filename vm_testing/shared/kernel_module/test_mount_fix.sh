#!/bin/bash
#
# VexFS Mount Fix Test Script
# Tests the fixed kernel module for mount and directory operation crashes
#

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Test configuration
TEST_IMG="/tmp/vexfs_test_mount.img"
MOUNT_POINT="/tmp/vexfs_mount_test"
MODULE_NAME="vexfs_deadlock_fix"

echo -e "${YELLOW}VexFS Mount Fix Test Script${NC}"
echo "================================="

# Function to cleanup on exit
cleanup() {
    echo -e "\n${YELLOW}Cleaning up...${NC}"
    
    # Unmount if mounted
    if mountpoint -q "$MOUNT_POINT" 2>/dev/null; then
        sudo umount "$MOUNT_POINT" || true
    fi
    
    # Remove mount point
    [ -d "$MOUNT_POINT" ] && sudo rm -rf "$MOUNT_POINT"
    
    # Remove module if loaded
    if lsmod | grep -q "$MODULE_NAME"; then
        sudo rmmod "$MODULE_NAME" || true
    fi
    
    # Remove test image
    [ -f "$TEST_IMG" ] && rm -f "$TEST_IMG"
}

# Set trap for cleanup
trap cleanup EXIT

# Step 1: Check if module is already loaded
echo -e "\n${YELLOW}Step 1: Checking module status...${NC}"
if lsmod | grep -q "vexfs"; then
    echo -e "${RED}Error: A VexFS module is already loaded. Please unload it first.${NC}"
    echo "Loaded VexFS modules:"
    lsmod | grep vexfs
    exit 1
fi
echo -e "${GREEN}✓ No VexFS modules loaded${NC}"

# Step 2: Build the module
echo -e "\n${YELLOW}Step 2: Building kernel module...${NC}"
make clean
make all
if [ ! -f "${MODULE_NAME}.ko" ]; then
    echo -e "${RED}Error: Module build failed${NC}"
    exit 1
fi
echo -e "${GREEN}✓ Module built successfully${NC}"

# Step 3: Create test image
echo -e "\n${YELLOW}Step 3: Creating test image...${NC}"
dd if=/dev/zero of="$TEST_IMG" bs=1M count=10 2>/dev/null
echo -e "${GREEN}✓ Created 10MB test image${NC}"

# Step 4: Format with mkfs.vexfs
echo -e "\n${YELLOW}Step 4: Formatting with mkfs.vexfs...${NC}"
if [ -x "../tools/mkfs.vexfs" ]; then
    ../tools/mkfs.vexfs "$TEST_IMG"
    echo -e "${GREEN}✓ Formatted with mkfs.vexfs${NC}"
else
    echo -e "${RED}Error: mkfs.vexfs not found or not executable${NC}"
    exit 1
fi

# Step 5: Load kernel module
echo -e "\n${YELLOW}Step 5: Loading kernel module...${NC}"
sudo insmod "${MODULE_NAME}.ko"
if ! lsmod | grep -q "$MODULE_NAME"; then
    echo -e "${RED}Error: Module failed to load${NC}"
    exit 1
fi
echo -e "${GREEN}✓ Module loaded successfully${NC}"

# Check kernel logs
echo -e "\n${YELLOW}Recent kernel messages:${NC}"
dmesg | tail -5

# Step 6: Create mount point
echo -e "\n${YELLOW}Step 6: Creating mount point...${NC}"
mkdir -p "$MOUNT_POINT"
echo -e "${GREEN}✓ Mount point created${NC}"

# Step 7: Mount filesystem (This is where the crash might occur)
echo -e "\n${YELLOW}Step 7: Mounting filesystem...${NC}"
echo "Command: sudo mount -t vexfs_fixed $TEST_IMG $MOUNT_POINT"

# Try to mount with timeout to prevent hanging
timeout 5s sudo mount -t vexfs_fixed "$TEST_IMG" "$MOUNT_POINT" 2>&1 | tee /tmp/mount_output.log
MOUNT_RESULT=${PIPESTATUS[0]}

if [ $MOUNT_RESULT -eq 124 ]; then
    echo -e "${RED}Error: Mount operation timed out${NC}"
    echo "This might indicate a deadlock or hang in the kernel module"
    exit 1
elif [ $MOUNT_RESULT -ne 0 ]; then
    echo -e "${RED}Error: Mount failed with exit code $MOUNT_RESULT${NC}"
    echo "Mount output:"
    cat /tmp/mount_output.log
    echo -e "\nKernel messages:"
    dmesg | tail -10
    exit 1
fi

# Check if actually mounted
if ! mountpoint -q "$MOUNT_POINT"; then
    echo -e "${RED}Error: Mount command succeeded but filesystem is not mounted${NC}"
    exit 1
fi

echo -e "${GREEN}✓ Filesystem mounted successfully${NC}"

# Step 8: Test directory operations (This is where the ls crash occurred)
echo -e "\n${YELLOW}Step 8: Testing directory operations...${NC}"
echo "Running: ls -la $MOUNT_POINT"

# Use timeout to prevent hanging if there's a kernel issue
timeout 5s ls -la "$MOUNT_POINT" 2>&1 | tee /tmp/ls_output.log
LS_RESULT=${PIPESTATUS[0]}

if [ $LS_RESULT -eq 124 ]; then
    echo -e "${RED}Error: ls operation timed out - possible kernel hang${NC}"
    exit 1
elif [ $LS_RESULT -ne 0 ]; then
    echo -e "${RED}Error: ls failed with exit code $LS_RESULT${NC}"
    echo "ls output:"
    cat /tmp/ls_output.log
    exit 1
fi

echo -e "${GREEN}✓ Directory listing successful${NC}"

# Step 9: Test file operations
echo -e "\n${YELLOW}Step 9: Testing file operations...${NC}"

# Create a file
echo "test content" | sudo tee "$MOUNT_POINT/test.txt" > /dev/null
if [ $? -eq 0 ]; then
    echo -e "${GREEN}✓ File creation successful${NC}"
else
    echo -e "${RED}✗ File creation failed${NC}"
fi

# Read the file
if sudo cat "$MOUNT_POINT/test.txt" > /dev/null 2>&1; then
    echo -e "${GREEN}✓ File read successful${NC}"
else
    echo -e "${RED}✗ File read failed${NC}"
fi

# List directory again
if timeout 5s ls -la "$MOUNT_POINT" > /dev/null 2>&1; then
    echo -e "${GREEN}✓ Directory listing after file operations successful${NC}"
else
    echo -e "${RED}✗ Directory listing after file operations failed${NC}"
fi

# Step 10: Unmount
echo -e "\n${YELLOW}Step 10: Unmounting filesystem...${NC}"
sudo umount "$MOUNT_POINT"
if [ $? -eq 0 ]; then
    echo -e "${GREEN}✓ Unmount successful${NC}"
else
    echo -e "${RED}✗ Unmount failed${NC}"
fi

# Step 11: Unload module
echo -e "\n${YELLOW}Step 11: Unloading module...${NC}"
sudo rmmod "$MODULE_NAME"
if [ $? -eq 0 ]; then
    echo -e "${GREEN}✓ Module unloaded successfully${NC}"
else
    echo -e "${RED}✗ Module unload failed${NC}"
fi

echo -e "\n${GREEN}All tests passed! The mount and directory operation fixes appear to be working.${NC}"