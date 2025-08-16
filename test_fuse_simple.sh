#!/bin/bash

# Simple test for FUSE delete/rmdir fixes

set -e

echo "=== Simple FUSE Fix Test ==="

MOUNT="/tmp/vexfs_test"
BIN="./rust/target/x86_64-unknown-linux-gnu/debug/vexfs_fuse"

# Cleanup
fusermount3 -u "$MOUNT" 2>/dev/null || true
rm -rf "$MOUNT"
mkdir -p "$MOUNT"

# Mount
echo "Mounting..."
"$BIN" "$MOUNT" &
PID=$!
sleep 3

if ! mountpoint -q "$MOUNT"; then
    echo "Failed to mount"
    exit 1
fi

echo "Testing operations..."

# Create and delete file
echo "1. Create file"
echo "test" > "$MOUNT/testfile.txt"
ls -la "$MOUNT/testfile.txt"

echo "2. Delete file (THIS WAS BROKEN)"
rm "$MOUNT/testfile.txt"
echo "✓ File deleted successfully"

# Create and remove directory
echo "3. Create directory"
mkdir "$MOUNT/testdir"
ls -ld "$MOUNT/testdir"

echo "4. Remove directory (THIS WAS BROKEN)"
rmdir "$MOUNT/testdir"
echo "✓ Directory removed successfully"

# Nested test
echo "5. Nested operations"
mkdir -p "$MOUNT/dir1/dir2"
echo "nested" > "$MOUNT/dir1/dir2/file.txt"
rm "$MOUNT/dir1/dir2/file.txt"
rmdir "$MOUNT/dir1/dir2"
rmdir "$MOUNT/dir1"
echo "✓ Nested operations work"

echo
echo "=== SUCCESS: All deletion operations fixed! ==="

# Cleanup
kill $PID 2>/dev/null || true
wait $PID 2>/dev/null || true
fusermount3 -u "$MOUNT" 2>/dev/null || true
rm -rf "$MOUNT"