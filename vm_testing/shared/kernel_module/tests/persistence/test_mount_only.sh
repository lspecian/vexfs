#!/bin/bash
# VexFS Mount-Only Test
# Tests basic mount/unmount functionality

set -e

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Test parameters
TEST_IMG="/tmp/vexfs_mount_test_$$.img"
MOUNT_POINT="/tmp/vexfs_mount_$$"

echo -e "${BLUE}VexFS Mount-Only Test${NC}"
echo -e "${BLUE}=====================${NC}"

# Cleanup function
cleanup() {
    echo "Cleaning up..."
    sudo umount "$MOUNT_POINT" 2>/dev/null || true
    rmdir "$MOUNT_POINT" 2>/dev/null || true
    rm -f "$TEST_IMG"
}

trap cleanup EXIT

# Note about module status
echo -e "${YELLOW}NOTE: VexFS module must be reloaded after kernel crash${NC}"
echo -e "${YELLOW}Please reboot or manually clean up the module first${NC}"
echo ""

# Step 1: Create test image
echo -e "${YELLOW}Creating test image...${NC}"
dd if=/dev/zero of="$TEST_IMG" bs=1M count=10 2>/dev/null

# Step 2: Format with VexFS
echo -e "${YELLOW}Formatting with VexFS...${NC}"
sudo /home/luis/Development/oss/vexfs/tools/mkfs.vexfs -f "$TEST_IMG"

# Step 3: Create mount point
mkdir -p "$MOUNT_POINT"

# Step 4: Mount filesystem
echo -e "${YELLOW}Mounting filesystem...${NC}"
if sudo mount -t vexfs_fixed -o loop "$TEST_IMG" "$MOUNT_POINT"; then
    echo -e "${GREEN}[PASS]${NC} Mount successful"
else
    echo -e "${RED}[FAIL]${NC} Mount failed"
    exit 1
fi

# Step 5: List root directory (should work with dir_fix)
echo -e "${YELLOW}Testing directory listing...${NC}"
if sudo ls -la "$MOUNT_POINT"; then
    echo -e "${GREEN}[PASS]${NC} Directory listing works"
else
    echo -e "${RED}[FAIL]${NC} Directory listing failed"
fi

# Step 6: Check filesystem statistics
echo -e "${YELLOW}Filesystem statistics:${NC}"
df -h "$MOUNT_POINT"

# Step 7: Unmount
echo -e "${YELLOW}Unmounting...${NC}"
if sudo umount "$MOUNT_POINT"; then
    echo -e "${GREEN}[PASS]${NC} Unmount successful"
else
    echo -e "${RED}[FAIL]${NC} Unmount failed"
fi

# Step 8: Remount
echo -e "${YELLOW}Remounting...${NC}"
if sudo mount -t vexfs_fixed -o loop "$TEST_IMG" "$MOUNT_POINT"; then
    echo -e "${GREEN}[PASS]${NC} Remount successful"
else
    echo -e "${RED}[FAIL]${NC} Remount failed"
    exit 1
fi

# Step 9: Final unmount
echo -e "${YELLOW}Final unmount...${NC}"
if sudo umount "$MOUNT_POINT"; then
    echo -e "${GREEN}[PASS]${NC} Final unmount successful"
else
    echo -e "${RED}[FAIL]${NC} Final unmount failed"
fi

echo -e "\n${BLUE}Mount test complete!${NC}"
echo -e "${YELLOW}Module status:${NC}"
lsmod | grep vexfs || echo "No VexFS module loaded"