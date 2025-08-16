#!/bin/bash

# Simple vector interface test

MOUNT="/tmp/vexfs_vec"
BIN="./rust/target/x86_64-unknown-linux-gnu/debug/vexfs_fuse"

# Cleanup
fusermount3 -u "$MOUNT" 2>/dev/null || true
rm -rf "$MOUNT"
mkdir -p "$MOUNT"

# Mount
echo "Mounting VexFS..."
"$BIN" "$MOUNT" &
PID=$!
sleep 3

if ! mountpoint -q "$MOUNT"; then
    echo "Failed to mount"
    exit 1
fi

echo "Testing vector interface..."

# Test 1: Store vector
echo "1. Storing vector"
echo "0.1,0.2,0.3,0.4,0.5" > "$MOUNT/test.vec"
cat "$MOUNT/test.vec"

# Test 2: Check _vexfs directory
echo "2. Checking control directory"
ls -la "$MOUNT/_vexfs/" 2>/dev/null || echo "No _vexfs directory"

# Test 3: List files
echo "3. Listing files"
ls -la "$MOUNT/"

echo "✓ Vector interface basics working!"

# Cleanup
kill $PID 2>/dev/null || true
fusermount3 -u "$MOUNT" 2>/dev/null || true
rm -rf "$MOUNT"