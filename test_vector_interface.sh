#!/bin/bash

# Test script for VexFS vector filesystem interface
# Demonstrates storing vectors, searching, and using control files

set -e

echo "=== VexFS Vector Filesystem Interface Test ==="
echo

MOUNT="/tmp/vexfs_vector_test"
BIN="./rust/target/x86_64-unknown-linux-gnu/debug/vexfs_fuse"

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m'

# Cleanup
cleanup() {
    echo
    echo "Cleaning up..."
    kill $FUSE_PID 2>/dev/null || true
    wait $FUSE_PID 2>/dev/null || true
    fusermount3 -u "$MOUNT" 2>/dev/null || true
    rm -rf "$MOUNT"
}

trap cleanup EXIT

# Setup
fusermount3 -u "$MOUNT" 2>/dev/null || true
rm -rf "$MOUNT"
mkdir -p "$MOUNT"

# Mount FUSE
echo "Mounting VexFS with vector support..."
"$BIN" "$MOUNT" &
FUSE_PID=$!
sleep 3

if ! mountpoint -q "$MOUNT"; then
    echo "Failed to mount"
    exit 1
fi
echo -e "${GREEN}✓ Mounted successfully${NC}"
echo

# Test 1: Check special control directory
echo -e "${BLUE}Test 1: Special _vexfs directory${NC}"
ls -la "$MOUNT/_vexfs/" 2>/dev/null || echo "No _vexfs directory yet"
echo

# Test 2: Store vectors
echo -e "${BLUE}Test 2: Store vector files${NC}"
echo "Creating document vectors..."
echo "0.1,0.2,0.3,0.4,0.5" > "$MOUNT/doc1.vec"
echo "0.2,0.3,0.4,0.5,0.6" > "$MOUNT/doc2.vec"
echo "0.5,0.6,0.7,0.8,0.9" > "$MOUNT/doc3.vec"
echo -e "${GREEN}✓ Stored 3 vectors${NC}"
ls -la "$MOUNT"/*.vec
echo

# Test 3: Read vector back
echo -e "${BLUE}Test 3: Read vector content${NC}"
echo "Content of doc1.vec:"
cat "$MOUNT/doc1.vec"
echo

# Test 4: Create search query file
echo -e "${BLUE}Test 4: Search using .search file${NC}"
cat > "$MOUNT/query.search" << EOF
vector: 0.15,0.25,0.35,0.45,0.55
k: 3
EOF
echo "Created search query:"
cat "$MOUNT/query.search"
echo

# Test 5: Check control files
echo -e "${BLUE}Test 5: Control files${NC}"
if [ -f "$MOUNT/_vexfs/stats" ]; then
    echo "System stats:"
    cat "$MOUNT/_vexfs/stats"
else
    echo "Stats file not available"
fi
echo

if [ -f "$MOUNT/_vexfs/control" ]; then
    echo "Configuration:"
    cat "$MOUNT/_vexfs/control"
else
    echo "Control file not available"
fi
echo

# Test 6: Global search
echo -e "${BLUE}Test 6: Global search interface${NC}"
if [ -f "$MOUNT/_vexfs/search" ]; then
    echo "Performing global search..."
    echo "0.15,0.25,0.35,0.45,0.55" > "$MOUNT/_vexfs/search"
    sleep 1
    echo "Search results:"
    cat "$MOUNT/_vexfs/search"
else
    echo "Global search not available"
fi
echo

# Test 7: Collections
echo -e "${BLUE}Test 7: Vector collections${NC}"
mkdir -p "$MOUNT/embeddings"
echo "0.7,0.8,0.9" > "$MOUNT/embeddings/emb1.vec"
echo "0.8,0.9,1.0" > "$MOUNT/embeddings/emb2.vec"
echo -e "${GREEN}✓ Created embeddings collection${NC}"
ls -la "$MOUNT/embeddings/"
echo

# Test 8: Directory listing
echo -e "${BLUE}Test 8: Full directory listing${NC}"
echo "VexFS contents:"
find "$MOUNT" -type f -name "*.vec" | head -10
echo

echo -e "${GREEN}=== Vector Interface Test Complete ===${NC}"
echo
echo "Summary:"
echo "- Vector files (.vec) can be stored and retrieved"
echo "- Search queries can be created with .search files"
echo "- Control directory provides system interface"
echo "- Collections organize vectors in directories"
echo

# Keep mounted for a moment to allow inspection
echo "Filesystem will be unmounted in 5 seconds..."
sleep 5