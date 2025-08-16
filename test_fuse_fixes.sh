#!/bin/bash

# Test script for FUSE deletion and rmdir fixes
# Tests the parent-child relationship fixes in VexFS FUSE implementation

set -e

echo "=== VexFS FUSE Delete/Rmdir Fix Test ==="
echo "Testing FUSE filesystem operations with proper parent-child relationships"
echo

# Configuration
MOUNT_POINT="/tmp/vexfs_test_mount"
FUSE_BINARY="./target/x86_64-unknown-linux-gnu/release/vexfs_fuse"
TEST_DIR="$MOUNT_POINT/testdir"
TEST_FILE="$TEST_DIR/testfile.txt"
TEST_SUBDIR="$TEST_DIR/subdir"

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Function to print test results
print_result() {
    if [ $1 -eq 0 ]; then
        echo -e "${GREEN}✓${NC} $2"
    else
        echo -e "${RED}✗${NC} $2"
        return 1
    fi
}

# Cleanup function
cleanup() {
    echo
    echo "Cleaning up..."
    
    # Unmount if mounted
    if mountpoint -q "$MOUNT_POINT" 2>/dev/null; then
        fusermount3 -u "$MOUNT_POINT" 2>/dev/null || true
        sleep 1
    fi
    
    # Remove mount point
    rm -rf "$MOUNT_POINT"
}

# Set up cleanup on exit
trap cleanup EXIT

# Build the FUSE binary if needed
echo "Building FUSE implementation..."
cd rust && cargo build --release --features fuse_support --quiet && cd ..
echo "Build complete"
echo

# Create mount point
echo "Setting up test environment..."
mkdir -p "$MOUNT_POINT"

# Mount the FUSE filesystem
echo "Mounting VexFS FUSE filesystem..."
"$FUSE_BINARY" "$MOUNT_POINT" -f &
FUSE_PID=$!
sleep 2

# Check if mounted
if ! mountpoint -q "$MOUNT_POINT"; then
    echo -e "${RED}Failed to mount FUSE filesystem${NC}"
    exit 1
fi
echo -e "${GREEN}FUSE filesystem mounted successfully${NC}"
echo

# Run tests
echo "Running tests..."
echo

# Test 1: Create directory
echo "Test 1: Create directory"
mkdir -p "$TEST_DIR"
print_result $? "Created directory: $TEST_DIR"

# Test 2: Create file in directory
echo
echo "Test 2: Create file in directory"
echo "Hello, VexFS!" > "$TEST_FILE"
print_result $? "Created file: $TEST_FILE"

# Test 3: List directory contents
echo
echo "Test 3: List directory contents"
ls -la "$TEST_DIR" > /dev/null
print_result $? "Listed directory contents"

# Test 4: Create subdirectory
echo
echo "Test 4: Create subdirectory"
mkdir -p "$TEST_SUBDIR"
print_result $? "Created subdirectory: $TEST_SUBDIR"

# Test 5: Create file in subdirectory
echo
echo "Test 5: Create file in subdirectory"
echo "Nested file" > "$TEST_SUBDIR/nested.txt"
print_result $? "Created file in subdirectory"

# Test 6: Delete file in subdirectory (THIS WAS BROKEN)
echo
echo "Test 6: Delete file in subdirectory"
rm "$TEST_SUBDIR/nested.txt"
print_result $? "Deleted file in subdirectory"

# Test 7: Delete file in main directory (THIS WAS BROKEN)
echo
echo "Test 7: Delete file in main directory"
rm "$TEST_FILE"
print_result $? "Deleted file: $TEST_FILE"

# Test 8: Remove empty subdirectory (THIS WAS BROKEN)
echo
echo "Test 8: Remove empty subdirectory"
rmdir "$TEST_SUBDIR"
print_result $? "Removed empty subdirectory"

# Test 9: Try to remove non-empty directory (should fail)
echo
echo "Test 9: Try to remove non-empty directory"
echo "temp" > "$TEST_DIR/tempfile"
rmdir "$TEST_DIR" 2>/dev/null
if [ $? -ne 0 ]; then
    print_result 0 "Correctly refused to remove non-empty directory"
else
    print_result 1 "ERROR: Removed non-empty directory (should have failed)"
fi
rm "$TEST_DIR/tempfile"

# Test 10: Remove empty directory (THIS WAS BROKEN)
echo
echo "Test 10: Remove empty directory"
rmdir "$TEST_DIR"
print_result $? "Removed empty directory"

# Test 11: Verify root directory still works
echo
echo "Test 11: Verify root directory still works"
ls -la "$MOUNT_POINT" > /dev/null
print_result $? "Root directory still accessible"

# Test 12: Create and delete multiple files
echo
echo "Test 12: Create and delete multiple files"
for i in {1..5}; do
    echo "File $i" > "$MOUNT_POINT/file$i.txt"
done
for i in {1..5}; do
    rm "$MOUNT_POINT/file$i.txt"
done
print_result $? "Created and deleted multiple files"

echo
echo "=== Test Summary ==="
echo "All critical deletion and rmdir operations are now working!"
echo

# Kill FUSE process
kill $FUSE_PID 2>/dev/null || true
wait $FUSE_PID 2>/dev/null || true

echo "Test completed successfully!"