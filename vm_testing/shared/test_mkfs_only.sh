#!/bin/sh

# Simple test of mkfs.vexfs without kernel module
echo "=== Testing mkfs.vexfs tool ==="

# Create test image
TEST_IMG="/tmp/test_mkfs.img"

echo "Creating test image..."
dd if=/dev/zero of="$TEST_IMG" bs=1M count=5 2>/dev/null

echo "Running mkfs.vexfs..."
if /mnt/shared/mkfs.vexfs "$TEST_IMG"; then
    echo "SUCCESS: mkfs.vexfs completed"
    
    echo "Checking filesystem structure with hexdump..."
    hexdump -C "$TEST_IMG" | head -20
    
    echo "Checking magic number (should be 46455856 = VEXF)..."
    hexdump -C "$TEST_IMG" | head -1
else
    echo "ERROR: mkfs.vexfs failed"
fi

rm -f "$TEST_IMG"
echo "=== Test completed ===