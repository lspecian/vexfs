#!/bin/bash
# Manual disk persistence test for VexFS

set -e

echo "=== VexFS Manual Disk Persistence Test ==="
echo

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m'

# Create test image
echo "1. Creating test filesystem..."
dd if=/dev/zero of=/tmp/vexfs_persist_test.img bs=1M count=100 2>/dev/null
/home/luis/Development/oss/vexfs/tools/mkfs.vexfs -f /tmp/vexfs_persist_test.img
echo -e "${GREEN}✓ Filesystem created${NC}"
echo

# First mount and create files
echo "2. First mount - Creating test files..."
sudo mkdir -p /mnt/vexfs_persist_test
sudo mount -t vexfs_fixed -o loop /tmp/vexfs_persist_test.img /mnt/vexfs_persist_test

# Create test files
echo "Hello VexFS - File 1" | sudo tee /mnt/vexfs_persist_test/file1.txt > /dev/null
echo "This is test file 2 with more content" | sudo tee /mnt/vexfs_persist_test/file2.txt > /dev/null
sudo mkdir -p /mnt/vexfs_persist_test/testdir
echo "File in subdirectory" | sudo tee /mnt/vexfs_persist_test/testdir/subfile.txt > /dev/null

# List files
echo "Files created:"
sudo ls -la /mnt/vexfs_persist_test/
echo

# Calculate checksums
echo "Calculating checksums..."
SUM1=$(sudo sha256sum /mnt/vexfs_persist_test/file1.txt | awk '{print $1}')
SUM2=$(sudo sha256sum /mnt/vexfs_persist_test/file2.txt | awk '{print $1}')
SUM3=$(sudo sha256sum /mnt/vexfs_persist_test/testdir/subfile.txt | awk '{print $1}')
echo "file1.txt: $SUM1"
echo "file2.txt: $SUM2"
echo "subfile.txt: $SUM3"
echo

# Unmount
echo "3. Unmounting filesystem..."
sudo umount /mnt/vexfs_persist_test
echo -e "${GREEN}✓ Filesystem unmounted${NC}"
echo

# Remount and verify
echo "4. Second mount - Verifying persistence..."
sudo mount -t vexfs_fixed -o loop /tmp/vexfs_persist_test.img /mnt/vexfs_persist_test

# List files
echo "Files after remount:"
sudo ls -la /mnt/vexfs_persist_test/
echo

# Verify checksums
echo "Verifying checksums..."
NEW_SUM1=$(sudo sha256sum /mnt/vexfs_persist_test/file1.txt 2>/dev/null | awk '{print $1}' || echo "FILE_NOT_FOUND")
NEW_SUM2=$(sudo sha256sum /mnt/vexfs_persist_test/file2.txt 2>/dev/null | awk '{print $1}' || echo "FILE_NOT_FOUND")
NEW_SUM3=$(sudo sha256sum /mnt/vexfs_persist_test/testdir/subfile.txt 2>/dev/null | awk '{print $1}' || echo "FILE_NOT_FOUND")

# Compare checksums
PASS=true
if [ "$SUM1" = "$NEW_SUM1" ]; then
    echo -e "file1.txt: ${GREEN}✓ PASS${NC} (checksum matches)"
else
    echo -e "file1.txt: ${RED}✗ FAIL${NC} (expected: $SUM1, got: $NEW_SUM1)"
    PASS=false
fi

if [ "$SUM2" = "$NEW_SUM2" ]; then
    echo -e "file2.txt: ${GREEN}✓ PASS${NC} (checksum matches)"
else
    echo -e "file2.txt: ${RED}✗ FAIL${NC} (expected: $SUM2, got: $NEW_SUM2)"
    PASS=false
fi

if [ "$SUM3" = "$NEW_SUM3" ]; then
    echo -e "subfile.txt: ${GREEN}✓ PASS${NC} (checksum matches)"
else
    echo -e "subfile.txt: ${RED}✗ FAIL${NC} (expected: $SUM3, got: $NEW_SUM3)"
    PASS=false
fi
echo

# Read file contents
echo "5. Reading file contents..."
echo "file1.txt contains:"
sudo cat /mnt/vexfs_persist_test/file1.txt 2>/dev/null || echo "[FILE NOT FOUND]"
echo
echo "file2.txt contains:"
sudo cat /mnt/vexfs_persist_test/file2.txt 2>/dev/null || echo "[FILE NOT FOUND]"
echo
echo "subfile.txt contains:"
sudo cat /mnt/vexfs_persist_test/testdir/subfile.txt 2>/dev/null || echo "[FILE NOT FOUND]"
echo

# Cleanup
echo "6. Cleaning up..."
sudo umount /mnt/vexfs_persist_test
sudo rmdir /mnt/vexfs_persist_test
rm -f /tmp/vexfs_persist_test.img
echo -e "${GREEN}✓ Cleanup complete${NC}"
echo

# Final result
if [ "$PASS" = true ]; then
    echo -e "${GREEN}=== DISK PERSISTENCE TEST PASSED ===${NC}"
    echo "All files persisted correctly across unmount/remount!"
    exit 0
else
    echo -e "${RED}=== DISK PERSISTENCE TEST FAILED ===${NC}"
    echo "Files did not persist correctly!"
    exit 1
fi