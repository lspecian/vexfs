#!/bin/bash
# VexFS Mount Test Script

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}VexFS Kernel Module Mount Test${NC}"
echo "================================"

# Check if running as root
if [ "$EUID" -ne 0 ]; then 
    echo -e "${RED}Error: This script must be run as root (use sudo)${NC}"
    exit 1
fi

# Variables
MODULE_PATH="/home/luis/Development/oss/vexfs/kernel_module/vexfs_deadlock_fix.ko"
TEST_IMAGE="/home/luis/Development/oss/vexfs/tools/test_vexfs_new.img"
MOUNT_POINT="/mnt/vexfs_test"

# Check if module exists
if [ ! -f "$MODULE_PATH" ]; then
    echo -e "${RED}Error: Kernel module not found at $MODULE_PATH${NC}"
    echo "Please build the module first with: cd kernel_module && make"
    exit 1
fi

# Check if test image exists
if [ ! -f "$TEST_IMAGE" ]; then
    echo -e "${RED}Error: Test image not found at $TEST_IMAGE${NC}"
    echo "Please create and format a test image first"
    exit 1
fi

# Unload module if already loaded
if lsmod | grep -q "vexfs_deadlock_fix"; then
    echo -e "${YELLOW}Unloading existing module...${NC}"
    rmmod vexfs_deadlock_fix || true
fi

# Load the module
echo -e "${YELLOW}Loading VexFS kernel module...${NC}"
insmod "$MODULE_PATH"
if ! lsmod | grep -q "vexfs_deadlock_fix"; then
    echo -e "${RED}Error: Failed to load kernel module${NC}"
    exit 1
fi
echo -e "${GREEN}Module loaded successfully${NC}"

# Create mount point
echo -e "${YELLOW}Creating mount point...${NC}"
mkdir -p "$MOUNT_POINT"

# Mount the filesystem
echo -e "${YELLOW}Mounting VexFS filesystem...${NC}"
if mount -t vexfs_fixed "$TEST_IMAGE" "$MOUNT_POINT"; then
    echo -e "${GREEN}Filesystem mounted successfully!${NC}"
    
    # Test basic operations
    echo -e "\n${YELLOW}Testing basic operations...${NC}"
    
    # List directory
    echo "1. Listing root directory:"
    ls -la "$MOUNT_POINT"
    
    # Create a test file
    echo "2. Creating test file:"
    echo "Hello VexFS!" > "$MOUNT_POINT/test.txt" || echo -e "${RED}Failed to create file${NC}"
    
    # List again
    echo "3. Listing after file creation:"
    ls -la "$MOUNT_POINT" || echo -e "${RED}Failed to list directory${NC}"
    
    # Read the file
    echo "4. Reading test file:"
    cat "$MOUNT_POINT/test.txt" 2>/dev/null || echo -e "${RED}Failed to read file${NC}"
    
    # Unmount
    echo -e "\n${YELLOW}Unmounting filesystem...${NC}"
    umount "$MOUNT_POINT"
    echo -e "${GREEN}Filesystem unmounted successfully${NC}"
else
    echo -e "${RED}Error: Failed to mount filesystem${NC}"
    dmesg | tail -20
fi

# Unload module
echo -e "${YELLOW}Unloading kernel module...${NC}"
rmmod vexfs_deadlock_fix
echo -e "${GREEN}Module unloaded successfully${NC}"

echo -e "\n${GREEN}Test completed!${NC}"