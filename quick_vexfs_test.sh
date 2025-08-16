#!/bin/bash

# Quick VexFS FUSE Test
echo "=== Quick VexFS FUSE Test ==="
echo
echo "⚠️  WARNING: VexFS is experimental alpha software"
echo "   - Vector features are NOT implemented"
echo "   - This only tests basic filesystem operations"
echo

# Check if FUSE binary exists
VEXFS_BIN="./target/x86_64-unknown-linux-gnu/release/vexfs_fuse"
if [ ! -f "$VEXFS_BIN" ]; then
    echo "❌ VexFS FUSE binary not found!"
    echo "   Building it now..."
    cd rust && cargo build --release --features="fuse_support" --bin vexfs_fuse
    cd ..
fi

# Create test mount point
MOUNT="/tmp/vexfs_test_$$"
mkdir -p "$MOUNT"

echo "1. Mounting VexFS at $MOUNT"
$VEXFS_BIN "$MOUNT" -f &
PID=$!
sleep 2

# Check if mounted
if ! mount | grep -q "$MOUNT"; then
    echo "❌ Failed to mount VexFS"
    exit 1
fi

echo "✅ VexFS mounted (PID: $PID)"
echo

# Run basic tests
echo "2. Running basic filesystem tests:"

echo -n "   Creating directory... "
mkdir -p "$MOUNT/test_dir" && echo "✅" || echo "❌"

echo -n "   Writing file... "
echo "Hello from VexFS!" > "$MOUNT/test_dir/hello.txt" && echo "✅" || echo "❌"

echo -n "   Reading file... "
cat "$MOUNT/test_dir/hello.txt" > /dev/null && echo "✅" || echo "❌"

echo -n "   Listing directory... "
ls "$MOUNT/test_dir" > /dev/null && echo "✅" || echo "❌"

echo -n "   Deleting file... "
rm "$MOUNT/test_dir/hello.txt" && echo "✅" || echo "❌"

echo
echo "3. What's missing (not implemented):"
echo "   ❌ Vector storage for files"
echo "   ❌ Semantic search capabilities"
echo "   ❌ Embedding generation"
echo "   ❌ HNSW indexing"
echo "   ❌ API compatibility layers"

echo
echo "4. Cleaning up..."
kill $PID 2>/dev/null
sleep 1
fusermount3 -u "$MOUNT" 2>/dev/null || fusermount -u "$MOUNT" 2>/dev/null
rmdir "$MOUNT"

echo "✅ Test complete"
echo
echo "Summary: VexFS FUSE provides basic POSIX filesystem operations only."
echo "         The 'vector' features that give it its name don't exist yet."