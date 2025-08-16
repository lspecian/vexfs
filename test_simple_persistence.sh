#!/bin/bash
# Simple VexFS persistence test

set -e

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m'

TEST_IMG="/tmp/vexfs_simple_test.img"
MOUNT_POINT="/mnt/vexfs_test"

echo -e "${YELLOW}Simple VexFS Persistence Test${NC}"
echo "=============================="

# Step 1: Clean up
echo "1. Cleaning up..."
sudo umount "$MOUNT_POINT" 2>/dev/null || true
rm -f "$TEST_IMG"
rm -rf "$MOUNT_POINT"

# Step 2: Create test filesystem
echo "2. Creating test filesystem..."
dd if=/dev/zero of="$TEST_IMG" bs=1M count=10 2>/dev/null
sudo tools/mkfs.vexfs -f "$TEST_IMG"

# Step 3: Create mount point
mkdir -p "$MOUNT_POINT"

# Step 4: Mount filesystem
echo "3. Mounting filesystem..."
if sudo mount -t vexfs_fixed "$TEST_IMG" "$MOUNT_POINT"; then
    echo -e "${GREEN}Mount successful${NC}"
else
    echo -e "${RED}Mount failed${NC}"
    exit 1
fi

# Step 5: Create a test file
echo "4. Creating test file..."
echo "Hello VexFS!" | sudo tee "$MOUNT_POINT/test.txt" > /dev/null
echo -e "${GREEN}File created${NC}"

# Step 6: Verify file exists
echo "5. Verifying file exists..."
if [ -f "$MOUNT_POINT/test.txt" ]; then
    content=$(sudo cat "$MOUNT_POINT/test.txt")
    echo -e "${GREEN}File exists with content: $content${NC}"
else
    echo -e "${RED}File not found!${NC}"
fi

# Step 7: List directory
echo "6. Directory listing:"
sudo ls -la "$MOUNT_POINT"

# Step 8: Sync filesystem
echo "7. Syncing filesystem..."
sync

# Step 9: Unmount
echo "8. Unmounting..."
if sudo umount "$MOUNT_POINT"; then
    echo -e "${GREEN}Unmount successful${NC}"
else
    echo -e "${RED}Unmount failed${NC}"
    exit 1
fi

# Step 10: Remount
echo "9. Remounting..."
if sudo mount -t vexfs_fixed "$TEST_IMG" "$MOUNT_POINT"; then
    echo -e "${GREEN}Remount successful${NC}"
else
    echo -e "${RED}Remount failed${NC}"
    exit 1
fi

# Step 11: Check if file persisted
echo "10. Checking if file persisted..."
if [ -f "$MOUNT_POINT/test.txt" ]; then
    content=$(sudo cat "$MOUNT_POINT/test.txt")
    echo -e "${GREEN}SUCCESS: File persisted with content: $content${NC}"
else
    echo -e "${RED}FAILURE: File did not persist!${NC}"
    
    # Debug: Check what's in the directory
    echo "Directory contents after remount:"
    sudo ls -la "$MOUNT_POINT"
fi

# Cleanup
sudo umount "$MOUNT_POINT" 2>/dev/null || true
rm -f "$TEST_IMG"
rm -rf "$MOUNT_POINT"