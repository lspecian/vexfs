#!/bin/bash
# Basic VexFS Persistence Test

set -e

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Test parameters
TEST_IMG="/tmp/vexfs_basic_test_$$.img"
MOUNT_POINT="/tmp/vexfs_mount_$$"

echo -e "${BLUE}VexFS Basic Persistence Test${NC}"
echo -e "${BLUE}============================${NC}"

# Cleanup function
cleanup() {
    echo "Cleaning up..."
    sudo umount "$MOUNT_POINT" 2>/dev/null || true
    rmdir "$MOUNT_POINT" 2>/dev/null || true
    rm -f "$TEST_IMG"
}

trap cleanup EXIT

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

# Step 5: Create a test file
echo -e "${YELLOW}Creating test file...${NC}"
TEST_CONTENT="Hello VexFS! This is test data: $(date)"
echo "$TEST_CONTENT" | sudo tee "$MOUNT_POINT/test.txt" > /dev/null

# Step 6: Verify immediate read
echo -e "${YELLOW}Verifying immediate read...${NC}"
READ_CONTENT=$(sudo cat "$MOUNT_POINT/test.txt")
if [ "$READ_CONTENT" = "$TEST_CONTENT" ]; then
    echo -e "${GREEN}[PASS]${NC} Immediate read successful"
else
    echo -e "${RED}[FAIL]${NC} Immediate read failed"
    echo "Expected: $TEST_CONTENT"
    echo "Got: $READ_CONTENT"
fi

# Step 7: List directory
echo -e "${YELLOW}Directory listing:${NC}"
sudo ls -la "$MOUNT_POINT"

# Step 8: Unmount
echo -e "${YELLOW}Unmounting...${NC}"
sync
sudo umount "$MOUNT_POINT"

# Step 9: Remount
echo -e "${YELLOW}Remounting...${NC}"
if sudo mount -t vexfs_fixed -o loop "$TEST_IMG" "$MOUNT_POINT"; then
    echo -e "${GREEN}[PASS]${NC} Remount successful"
else
    echo -e "${RED}[FAIL]${NC} Remount failed"
    exit 1
fi

# Step 10: Check persistence
echo -e "${YELLOW}Checking file persistence...${NC}"
if [ -f "$MOUNT_POINT/test.txt" ]; then
    PERSISTED_CONTENT=$(sudo cat "$MOUNT_POINT/test.txt")
    if [ "$PERSISTED_CONTENT" = "$TEST_CONTENT" ]; then
        echo -e "${GREEN}[PASS]${NC} File persisted correctly!"
        echo "Content: $PERSISTED_CONTENT"
    else
        echo -e "${RED}[FAIL]${NC} File content changed"
        echo "Expected: $TEST_CONTENT"
        echo "Got: $PERSISTED_CONTENT"
    fi
else
    echo -e "${RED}[FAIL]${NC} File not found after remount"
fi

# Final unmount
sudo umount "$MOUNT_POINT"

echo -e "\n${BLUE}Test complete!${NC}"