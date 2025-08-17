#!/bin/bash

# VexFS FUSE Example: Storing and Retrieving Memories
# This script demonstrates basic VexFS usage with the FUSE implementation

set -e

echo "=== VexFS Memory Storage Example ==="
echo

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Configuration
MOUNT_POINT="/tmp/vexfs_memories"
VEXFS_BINARY="./target/x86_64-unknown-linux-gnu/release/vexfs_fuse"

# Check if binary exists
if [ ! -f "$VEXFS_BINARY" ]; then
    echo -e "${RED}Error: VexFS FUSE binary not found at $VEXFS_BINARY${NC}"
    echo "Please build it first with: cd rust && cargo build --release --features=\"fuse_support\""
    exit 1
fi

# Create mount point
echo -e "${YELLOW}1. Creating mount point at $MOUNT_POINT${NC}"
mkdir -p "$MOUNT_POINT"

# Check if already mounted and unmount
if mount | grep -q "$MOUNT_POINT"; then
    echo -e "${YELLOW}   Unmounting existing VexFS mount...${NC}"
    fusermount3 -u "$MOUNT_POINT" 2>/dev/null || fusermount -u "$MOUNT_POINT" 2>/dev/null || true
    sleep 1
fi

# Mount VexFS
echo -e "${YELLOW}2. Mounting VexFS FUSE filesystem${NC}"
echo "   Command: $VEXFS_BINARY $MOUNT_POINT -f &"

# Start VexFS in background
$VEXFS_BINARY "$MOUNT_POINT" -f > /tmp/vexfs.log 2>&1 &
VEXFS_PID=$!

# Wait for mount
sleep 2

# Check if mounted successfully
if ! mount | grep -q "$MOUNT_POINT"; then
    echo -e "${RED}Error: Failed to mount VexFS${NC}"
    echo "Check /tmp/vexfs.log for errors"
    cat /tmp/vexfs.log
    exit 1
fi

echo -e "${GREEN}   ✓ VexFS mounted successfully (PID: $VEXFS_PID)${NC}"
echo

# Function to cleanup on exit
cleanup() {
    echo
    echo -e "${YELLOW}Cleaning up...${NC}"
    if [ ! -z "$VEXFS_PID" ]; then
        kill $VEXFS_PID 2>/dev/null || true
    fi
    fusermount3 -u "$MOUNT_POINT" 2>/dev/null || fusermount -u "$MOUNT_POINT" 2>/dev/null || true
    echo -e "${GREEN}✓ Cleanup complete${NC}"
}
trap cleanup EXIT

# Now let's use VexFS to store memories
echo -e "${YELLOW}3. Creating memory storage structure${NC}"

# Create directories for organizing memories
mkdir -p "$MOUNT_POINT/memories/personal"
mkdir -p "$MOUNT_POINT/memories/technical"
mkdir -p "$MOUNT_POINT/memories/projects"

echo -e "${GREEN}   ✓ Created directory structure${NC}"

# Store some memories as files
echo -e "${YELLOW}4. Storing memories as files${NC}"

# Personal memory
cat > "$MOUNT_POINT/memories/personal/first_computer.txt" << EOF
My First Computer Memory
========================
Date: 1995
System: 486 DX2 66MHz
RAM: 8MB
OS: MS-DOS 6.22 with Windows 3.11

I remember the excitement of hearing the hard drive spin up and the 
satisfying beep of the POST. Learning to navigate DOS commands and 
playing Prince of Persia for hours.
EOF
echo -e "${GREEN}   ✓ Stored personal memory: first_computer.txt${NC}"

# Technical memory
cat > "$MOUNT_POINT/memories/technical/learning_rust.md" << EOF
# Learning Rust Journey

## First Encounter (2020)
- Struggled with the borrow checker
- "Fighting the compiler" phase
- Reading "The Book" multiple times

## Breakthrough Moment
- Finally understood lifetimes
- Realized the compiler is your friend
- Started thinking in ownership patterns

## Key Learnings
- Move semantics prevent entire classes of bugs
- Zero-cost abstractions are real
- The ecosystem (cargo, crates.io) is fantastic
EOF
echo -e "${GREEN}   ✓ Stored technical memory: learning_rust.md${NC}"

# Project memory
cat > "$MOUNT_POINT/memories/projects/vexfs_research.json" << EOF
{
  "project": "VexFS",
  "date": "2024-08-03",
  "status": "experimental",
  "key_findings": {
    "architecture": "Dual implementation (kernel + FUSE)",
    "challenges": [
      "Kernel module stability issues",
      "Vector features not yet implemented",
      "Documentation vs reality gap"
    ],
    "potential": "Interesting concept for AI-native filesystem",
    "current_state": "Alpha - not production ready"
  },
  "recommendation": "Use FUSE for development, fix kernel module basics first"
}
EOF
echo -e "${GREEN}   ✓ Stored project memory: vexfs_research.json${NC}"

# Create a memory index
cat > "$MOUNT_POINT/memories/INDEX.txt" << EOF
VexFS Memory Storage Index
==========================

Personal Memories:
- first_computer.txt: Nostalgic memory of first computer

Technical Memories:
- learning_rust.md: Journey of learning Rust programming language

Project Memories:
- vexfs_research.json: Research findings on VexFS filesystem project

Note: These are stored as regular files since VexFS vector features
      are not yet implemented. In the future, these could be
      semantically indexed and searchable.
EOF
echo -e "${GREEN}   ✓ Created memory index${NC}"

echo
echo -e "${YELLOW}5. Reading back memories${NC}"

# List the structure
echo -e "\n${YELLOW}Directory structure:${NC}"
tree "$MOUNT_POINT/memories" 2>/dev/null || find "$MOUNT_POINT/memories" -type f

# Read a memory
echo -e "\n${YELLOW}Reading a memory (first_computer.txt):${NC}"
echo "----------------------------------------"
cat "$MOUNT_POINT/memories/personal/first_computer.txt"
echo "----------------------------------------"

# Show file stats
echo -e "\n${YELLOW}6. File statistics:${NC}"
ls -la "$MOUNT_POINT/memories/personal/first_computer.txt"

# Demonstrate file operations
echo -e "\n${YELLOW}7. Testing file operations:${NC}"

# Append to a file
echo -e "\nPS: Also learned to love QBASIC programming!" >> "$MOUNT_POINT/memories/personal/first_computer.txt"
echo -e "${GREEN}   ✓ Successfully appended to file${NC}"

# Copy a file
cp "$MOUNT_POINT/memories/technical/learning_rust.md" "$MOUNT_POINT/memories/technical/rust_backup.md"
echo -e "${GREEN}   ✓ Successfully copied file${NC}"

# Create a symlink (if supported)
ln -s "../personal/first_computer.txt" "$MOUNT_POINT/memories/favorite_memory.txt" 2>/dev/null && \
    echo -e "${GREEN}   ✓ Created symlink${NC}" || \
    echo -e "${YELLOW}   ⚠ Symlinks not supported${NC}"

echo
echo -e "${GREEN}=== VexFS Memory Storage Demo Complete ===${NC}"
echo
echo -e "${YELLOW}What would this look like with vector features?${NC}"
echo "- Each memory could have an embedding vector"
echo "- Semantic search: 'Find memories about learning programming'"
echo "- Similarity: 'Find memories similar to this one'"
echo "- Clustering: 'Group related memories together'"
echo
echo -e "${YELLOW}Current limitations:${NC}"
echo "- No vector storage/search (not implemented)"
echo "- No semantic operations"
echo "- Basic POSIX filesystem operations only"
echo
echo -e "${YELLOW}The filesystem will remain mounted at: $MOUNT_POINT${NC}"
echo "To unmount: fusermount3 -u $MOUNT_POINT"
echo "To explore: cd $MOUNT_POINT/memories"
echo
echo "Press Ctrl+C to unmount and cleanup..."

# Keep running until interrupted
while true; do
    sleep 1
done