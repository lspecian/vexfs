#!/bin/bash
# VexFS File Persistence Test
# Tests that file data persists across unmount/remount cycles

set -e

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

# Test parameters
TEST_IMG="/tmp/vexfs_file_test.img"
MOUNT_POINT="/mnt/vexfs_file_test"
TEST_FILES=("small.txt" "medium.dat" "large.bin")
TEST_SIZES=(4096 1048576 10485760)  # 4KB, 1MB, 10MB

# Function to print colored output
print_status() {
    local status=$1
    local message=$2
    
    if [ "$status" = "PASS" ]; then
        echo -e "${GREEN}[PASS]${NC} $message"
    elif [ "$status" = "FAIL" ]; then
        echo -e "${RED}[FAIL]${NC} $message"
    else
        echo -e "${YELLOW}[$status]${NC} $message"
    fi
}

# Cleanup function
cleanup() {
    echo "Cleaning up..."
    sudo umount "$MOUNT_POINT" 2>/dev/null || true
    sudo rmmod vexfs_deadlock_fix 2>/dev/null || true
    rm -f "$TEST_IMG"
    rm -rf "$MOUNT_POINT"
}

# Set trap for cleanup
trap cleanup EXIT

echo -e "${BLUE}VexFS File Persistence Test${NC}"
echo -e "${BLUE}===========================${NC}"

# Step 1: Clean up any existing state
print_status "INFO" "Cleaning up existing state..."
cleanup

# Step 2: Build the module
print_status "INFO" "Building VexFS module..."
cd kernel_module
make clean && make
cd ..

# Step 3: Create test filesystem
print_status "INFO" "Creating test filesystem..."
dd if=/dev/zero of="$TEST_IMG" bs=1M count=50 2>/dev/null
sudo tools/mkfs.vexfs -f "$TEST_IMG"

# Step 4: Load module
print_status "INFO" "Loading VexFS module..."
sudo insmod kernel_module/vexfs_deadlock_fix.ko

# Step 5: Create mount point
mkdir -p "$MOUNT_POINT"

# Step 6: Mount filesystem
print_status "INFO" "Mounting filesystem..."
sudo mount -t vexfs_fixed "$TEST_IMG" "$MOUNT_POINT"

# Step 7: Create test files with known content
print_status "INFO" "Creating test files..."
declare -A file_checksums

for i in "${!TEST_FILES[@]}"; do
    file="${TEST_FILES[$i]}"
    size="${TEST_SIZES[$i]}"
    
    # Generate random data
    dd if=/dev/urandom of="/tmp/${file}" bs=1 count="$size" 2>/dev/null
    
    # Calculate checksum
    checksum=$(sha256sum "/tmp/${file}" | awk '{print $1}')
    file_checksums["$file"]="$checksum"
    
    # Copy to VexFS
    sudo cp "/tmp/${file}" "$MOUNT_POINT/${file}"
    
    print_status "INFO" "Created ${file} (${size} bytes, SHA256: ${checksum:0:16}...)"
done

# Step 8: Verify files can be read back immediately
print_status "INFO" "Verifying immediate read-back..."
for file in "${TEST_FILES[@]}"; do
    if sudo cmp -s "/tmp/${file}" "$MOUNT_POINT/${file}"; then
        print_status "PASS" "Immediate read-back of ${file}"
    else
        print_status "FAIL" "Immediate read-back of ${file}"
    fi
done

# Step 9: List directory to ensure all files are visible
print_status "INFO" "Directory listing:"
sudo ls -la "$MOUNT_POINT"

# Step 10: Sync and unmount
print_status "INFO" "Syncing and unmounting..."
sync
sudo umount "$MOUNT_POINT"

# Step 11: Remount
print_status "INFO" "Remounting filesystem..."
sudo mount -t vexfs_fixed "$TEST_IMG" "$MOUNT_POINT"

# Step 12: Verify files persist
print_status "INFO" "Verifying file persistence..."
persistence_ok=true

for file in "${TEST_FILES[@]}"; do
    if [ -f "$MOUNT_POINT/${file}" ]; then
        # Calculate checksum of persisted file
        persisted_checksum=$(sudo sha256sum "$MOUNT_POINT/${file}" | awk '{print $1}')
        original_checksum="${file_checksums[$file]}"
        
        if [ "$persisted_checksum" = "$original_checksum" ]; then
            print_status "PASS" "File ${file} persisted correctly"
        else
            print_status "FAIL" "File ${file} checksum mismatch"
            echo "  Expected: ${original_checksum}"
            echo "  Got:      ${persisted_checksum}"
            persistence_ok=false
        fi
    else
        print_status "FAIL" "File ${file} not found after remount"
        persistence_ok=false
    fi
done

# Step 13: Test file modification persistence
print_status "INFO" "Testing file modification persistence..."
TEST_FILE="$MOUNT_POINT/modify_test.txt"

# Create and modify a file
echo "Original content" | sudo tee "$TEST_FILE" > /dev/null
echo "Modified content" | sudo tee -a "$TEST_FILE" > /dev/null

# Read back immediately
immediate_content=$(sudo cat "$TEST_FILE")

# Unmount and remount
sudo umount "$MOUNT_POINT"
sudo mount -t vexfs_fixed "$TEST_IMG" "$MOUNT_POINT"

# Check if modifications persist
if [ -f "$TEST_FILE" ]; then
    persisted_content=$(sudo cat "$TEST_FILE")
    if [ "$immediate_content" = "$persisted_content" ]; then
        print_status "PASS" "File modifications persisted"
    else
        print_status "FAIL" "File modifications lost"
        echo "  Expected: $immediate_content"
        echo "  Got:      $persisted_content"
        persistence_ok=false
    fi
else
    print_status "FAIL" "Modified file not found after remount"
    persistence_ok=false
fi

# Step 14: Test module reload persistence
print_status "INFO" "Testing module reload persistence..."
sudo umount "$MOUNT_POINT"
sudo rmmod vexfs_deadlock_fix
sudo insmod kernel_module/vexfs_deadlock_fix.ko
sudo mount -t vexfs_fixed "$TEST_IMG" "$MOUNT_POINT"

# Verify files still exist after module reload
for file in "${TEST_FILES[@]}"; do
    if [ -f "$MOUNT_POINT/${file}" ]; then
        print_status "PASS" "File ${file} survived module reload"
    else
        print_status "FAIL" "File ${file} lost after module reload"
        persistence_ok=false
    fi
done

# Summary
echo -e "\n${BLUE}Test Summary${NC}"
echo -e "${BLUE}============${NC}"

if [ "$persistence_ok" = true ]; then
    print_status "PASS" "All file persistence tests passed!"
    exit 0
else
    print_status "FAIL" "Some file persistence tests failed"
    exit 1
fi