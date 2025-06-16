#!/bin/bash
# VexFS Persistence Test Script
# Tests if files survive unmount/remount cycles

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${GREEN}VexFS Persistence Test${NC}"
echo "======================"

# Check if running as root
if [ "$EUID" -ne 0 ]; then 
    echo -e "${RED}Error: This script must be run as root (use sudo)${NC}"
    exit 1
fi

# Variables
MODULE_PATH="/home/luis/Development/oss/vexfs/kernel_module/vexfs_deadlock_fix.ko"
TEST_IMAGE="/home/luis/Development/oss/vexfs/tools/test_persistence.img"
MOUNT_POINT="/mnt/vexfs_persist_test"
MKFS_TOOL="/home/luis/Development/oss/vexfs/tools/mkfs.vexfs"

# Function to load module
load_module() {
    if ! lsmod | grep -q "vexfs_deadlock_fix"; then
        echo -e "${YELLOW}Loading VexFS kernel module...${NC}"
        insmod "$MODULE_PATH"
        sleep 1
    fi
}

# Function to unload module
unload_module() {
    if lsmod | grep -q "vexfs_deadlock_fix"; then
        echo -e "${YELLOW}Unloading VexFS kernel module...${NC}"
        rmmod vexfs_deadlock_fix
        sleep 1
    fi
}

# Cleanup function
cleanup() {
    # Unmount if mounted
    if mountpoint -q "$MOUNT_POINT" 2>/dev/null; then
        umount "$MOUNT_POINT" 2>/dev/null || true
    fi
    # Unload module
    unload_module
    # Remove mount point
    rmdir "$MOUNT_POINT" 2>/dev/null || true
}

# Set trap for cleanup
trap cleanup EXIT

echo -e "${BLUE}=== Phase 1: Create and Format Test Image ===${NC}"

# Create test image
echo "Creating 20MB test image..."
dd if=/dev/zero of="$TEST_IMAGE" bs=1M count=20 status=none
echo -e "${GREEN}Test image created${NC}"

# Format with mkfs.vexfs
echo "Formatting with mkfs.vexfs..."
"$MKFS_TOOL" -f "$TEST_IMAGE" > /dev/null
echo -e "${GREEN}Filesystem formatted${NC}"

echo -e "\n${BLUE}=== Phase 2: Initial Mount and File Creation ===${NC}"

# Load module and mount
load_module
mkdir -p "$MOUNT_POINT"
mount -t vexfs_fixed "$TEST_IMAGE" "$MOUNT_POINT"
echo -e "${GREEN}Filesystem mounted${NC}"

# Create test files
echo "Creating test files..."
echo "File 1 - Created at $(date)" > "$MOUNT_POINT/test1.txt"
echo "File 2 - Persistence test" > "$MOUNT_POINT/test2.txt"
mkdir -p "$MOUNT_POINT/testdir"
echo "File in directory" > "$MOUNT_POINT/testdir/subfile.txt"

# Create a larger file
dd if=/dev/urandom of="$MOUNT_POINT/random.dat" bs=1K count=100 status=none

# List files
echo -e "\nFiles created:"
ls -la "$MOUNT_POINT"
echo -e "\nDirectory contents:"
ls -la "$MOUNT_POINT/testdir"

# Calculate checksums
TEST1_SUM=$(md5sum "$MOUNT_POINT/test1.txt" | cut -d' ' -f1)
TEST2_SUM=$(md5sum "$MOUNT_POINT/test2.txt" | cut -d' ' -f1)
SUBFILE_SUM=$(md5sum "$MOUNT_POINT/testdir/subfile.txt" | cut -d' ' -f1)
RANDOM_SUM=$(md5sum "$MOUNT_POINT/random.dat" | cut -d' ' -f1)

echo -e "\nChecksums:"
echo "test1.txt: $TEST1_SUM"
echo "test2.txt: $TEST2_SUM"
echo "subfile.txt: $SUBFILE_SUM"
echo "random.dat: $RANDOM_SUM"

echo -e "\n${BLUE}=== Phase 3: Unmount and Remount ===${NC}"

# Unmount
echo "Unmounting filesystem..."
umount "$MOUNT_POINT"
echo -e "${GREEN}Filesystem unmounted${NC}"

# Wait a moment
sleep 2

# Remount
echo "Remounting filesystem..."
mount -t vexfs_fixed "$TEST_IMAGE" "$MOUNT_POINT"
echo -e "${GREEN}Filesystem remounted${NC}"

echo -e "\n${BLUE}=== Phase 4: Verify Persistence ===${NC}"

# Check if files exist
PERSIST_PASS=true

echo "Checking file existence..."
for file in "test1.txt" "test2.txt" "testdir/subfile.txt" "random.dat"; do
    if [ -f "$MOUNT_POINT/$file" ]; then
        echo -e "  ${GREEN}✓${NC} $file exists"
    else
        echo -e "  ${RED}✗${NC} $file missing"
        PERSIST_PASS=false
    fi
done

# Check directory
if [ -d "$MOUNT_POINT/testdir" ]; then
    echo -e "  ${GREEN}✓${NC} testdir exists"
else
    echo -e "  ${RED}✗${NC} testdir missing"
    PERSIST_PASS=false
fi

# Verify checksums
echo -e "\nVerifying file contents..."
if [ -f "$MOUNT_POINT/test1.txt" ]; then
    NEW_TEST1_SUM=$(md5sum "$MOUNT_POINT/test1.txt" | cut -d' ' -f1)
    if [ "$TEST1_SUM" = "$NEW_TEST1_SUM" ]; then
        echo -e "  ${GREEN}✓${NC} test1.txt checksum matches"
    else
        echo -e "  ${RED}✗${NC} test1.txt checksum mismatch"
        PERSIST_PASS=false
    fi
fi

if [ -f "$MOUNT_POINT/test2.txt" ]; then
    NEW_TEST2_SUM=$(md5sum "$MOUNT_POINT/test2.txt" | cut -d' ' -f1)
    if [ "$TEST2_SUM" = "$NEW_TEST2_SUM" ]; then
        echo -e "  ${GREEN}✓${NC} test2.txt checksum matches"
    else
        echo -e "  ${RED}✗${NC} test2.txt checksum mismatch"
        PERSIST_PASS=false
    fi
fi

if [ -f "$MOUNT_POINT/testdir/subfile.txt" ]; then
    NEW_SUBFILE_SUM=$(md5sum "$MOUNT_POINT/testdir/subfile.txt" | cut -d' ' -f1)
    if [ "$SUBFILE_SUM" = "$NEW_SUBFILE_SUM" ]; then
        echo -e "  ${GREEN}✓${NC} subfile.txt checksum matches"
    else
        echo -e "  ${RED}✗${NC} subfile.txt checksum mismatch"
        PERSIST_PASS=false
    fi
fi

if [ -f "$MOUNT_POINT/random.dat" ]; then
    NEW_RANDOM_SUM=$(md5sum "$MOUNT_POINT/random.dat" | cut -d' ' -f1)
    if [ "$RANDOM_SUM" = "$NEW_RANDOM_SUM" ]; then
        echo -e "  ${GREEN}✓${NC} random.dat checksum matches"
    else
        echo -e "  ${RED}✗${NC} random.dat checksum mismatch"
        PERSIST_PASS=false
    fi
fi

echo -e "\n${BLUE}=== Phase 5: Module Reload Test ===${NC}"

# Unmount and unload module
echo "Unmounting and unloading module..."
umount "$MOUNT_POINT"
unload_module

# Reload module and remount
echo "Reloading module and remounting..."
load_module
mount -t vexfs_fixed "$TEST_IMAGE" "$MOUNT_POINT"

# Quick check
echo "Quick persistence check after module reload:"
ls -la "$MOUNT_POINT" > /dev/null
if [ -f "$MOUNT_POINT/test1.txt" ] && [ -f "$MOUNT_POINT/test2.txt" ]; then
    echo -e "  ${GREEN}✓${NC} Files still present after module reload"
else
    echo -e "  ${RED}✗${NC} Files missing after module reload"
    PERSIST_PASS=false
fi

echo -e "\n${BLUE}=== Test Results ===${NC}"

if [ "$PERSIST_PASS" = true ]; then
    echo -e "${GREEN}✓ PERSISTENCE TEST PASSED!${NC}"
    echo "Files successfully persisted across unmount/remount cycles"
else
    echo -e "${RED}✗ PERSISTENCE TEST FAILED!${NC}"
    echo "Files did not persist correctly"
fi

# Cleanup is handled by trap

# Remove test image
rm -f "$TEST_IMAGE"

exit $([ "$PERSIST_PASS" = true ] && echo 0 || echo 1)