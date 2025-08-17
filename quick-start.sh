#!/bin/bash

# VexFS Quick Start Script
# Minimal setup for FUSE filesystem development

set -e

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

echo "╔══════════════════════════════════════════════════════╗"
echo "║          VexFS FUSE Quick Start                       ║"
echo "╚══════════════════════════════════════════════════════╝"
echo

# Configuration
VEXFS_ROOT=$(pwd)
MOUNT_POINT="${MOUNT_POINT:-$HOME/vexfs-mount}"

# Find the actual binary location
if [ -f "$VEXFS_ROOT/rust/target/release/vexfs_fuse" ]; then
    VEXFS_BINARY="$VEXFS_ROOT/rust/target/release/vexfs_fuse"
elif [ -f "$VEXFS_ROOT/rust/target/x86_64-unknown-linux-gnu/release/vexfs_fuse" ]; then
    VEXFS_BINARY="$VEXFS_ROOT/rust/target/x86_64-unknown-linux-gnu/release/vexfs_fuse"
else
    VEXFS_BINARY=""
fi

# Check if we're in the right directory
if [ ! -f "rust/Cargo.toml" ]; then
    echo -e "${RED}Error: Must run from VexFS root directory${NC}"
    exit 1
fi

# Function to build FUSE
build_fuse() {
    echo -e "${BLUE}Building FUSE filesystem...${NC}"
    cd rust
    if cargo build --release --features fuse_support --bin vexfs_fuse 2>&1 | grep -q "Finished"; then
        cd ..
        # Re-check for binary location after build
        if [ -f "$VEXFS_ROOT/rust/target/release/vexfs_fuse" ]; then
            VEXFS_BINARY="$VEXFS_ROOT/rust/target/release/vexfs_fuse"
        elif [ -f "$VEXFS_ROOT/rust/target/x86_64-unknown-linux-gnu/release/vexfs_fuse" ]; then
            VEXFS_BINARY="$VEXFS_ROOT/rust/target/x86_64-unknown-linux-gnu/release/vexfs_fuse"
        fi
        
        if [ -f "$VEXFS_BINARY" ]; then
            echo -e "${GREEN}✓ Build successful${NC}"
            echo "Binary: $VEXFS_BINARY"
            return 0
        fi
    fi
    cd ..
    echo -e "${RED}✗ Build failed${NC}"
    exit 1
}

# Function to start FUSE
start_fuse() {
    # Create mount point
    mkdir -p "$MOUNT_POINT"
    
    # Unmount if already mounted
    if mountpoint -q "$MOUNT_POINT" 2>/dev/null; then
        echo -e "${YELLOW}Unmounting existing mount...${NC}"
        fusermount3 -u "$MOUNT_POINT" 2>/dev/null || true
        sleep 1
    fi
    
    echo -e "${BLUE}Starting FUSE filesystem...${NC}"
    echo "Mount point: $MOUNT_POINT"
    
    # Start FUSE in foreground with debug output
    "$VEXFS_BINARY" "$MOUNT_POINT" -f -d &
    FUSE_PID=$!
    
    # Wait for mount
    sleep 2
    
    if mountpoint -q "$MOUNT_POINT"; then
        echo -e "${GREEN}✓ FUSE filesystem mounted${NC}"
        echo
        echo "PID: $FUSE_PID"
        echo
        return 0
    else
        echo -e "${RED}✗ Failed to mount FUSE filesystem${NC}"
        return 1
    fi
}

# Function to test FUSE
test_fuse() {
    echo -e "${BLUE}Testing FUSE operations...${NC}"
    
    # Test 1: Create file
    echo -n "1. File creation... "
    echo "test data" > "$MOUNT_POINT/test.txt"
    if [ -f "$MOUNT_POINT/test.txt" ]; then
        echo -e "${GREEN}✓${NC}"
    else
        echo -e "${RED}✗${NC}"
    fi
    
    # Test 2: Read file
    echo -n "2. File read... "
    content=$(cat "$MOUNT_POINT/test.txt" 2>/dev/null)
    if [ "$content" = "test data" ]; then
        echo -e "${GREEN}✓${NC}"
    else
        echo -e "${RED}✗${NC}"
    fi
    
    # Test 3: Create directory
    echo -n "3. Directory creation... "
    mkdir -p "$MOUNT_POINT/testdir"
    if [ -d "$MOUNT_POINT/testdir" ]; then
        echo -e "${GREEN}✓${NC}"
    else
        echo -e "${RED}✗${NC}"
    fi
    
    # Test 4: Vector file
    echo -n "4. Vector storage... "
    echo "0.1,0.2,0.3,0.4" > "$MOUNT_POINT/test.vec"
    if [ -f "$MOUNT_POINT/test.vec" ]; then
        echo -e "${GREEN}✓${NC}"
    else
        echo -e "${RED}✗${NC}"
    fi
    
    # Test 5: List files
    echo -n "5. Directory listing... "
    if ls "$MOUNT_POINT" > /dev/null 2>&1; then
        echo -e "${GREEN}✓${NC}"
    else
        echo -e "${RED}✗${NC}"
    fi
    
    echo
}

# Function to stop FUSE
stop_fuse() {
    echo -e "${BLUE}Stopping FUSE filesystem...${NC}"
    
    # Kill process if PID is known
    if [ ! -z "$FUSE_PID" ]; then
        kill $FUSE_PID 2>/dev/null || true
    fi
    
    # Unmount
    if mountpoint -q "$MOUNT_POINT" 2>/dev/null; then
        fusermount3 -u "$MOUNT_POINT" 2>/dev/null || true
        echo -e "${GREEN}✓ Unmounted${NC}"
    fi
}

# Main menu
show_menu() {
    echo
    echo "VexFS FUSE Quick Start"
    echo "======================"
    echo "1) Build FUSE"
    echo "2) Start FUSE (foreground)"
    echo "3) Test FUSE operations"
    echo "4) Stop FUSE"
    echo "5) Build & Start"
    echo "6) Full cycle (Build, Start, Test)"
    echo "q) Quit"
    echo
    read -p "Select option: " choice
    
    case $choice in
        1) build_fuse ;;
        2) start_fuse ;;
        3) test_fuse ;;
        4) stop_fuse ;;
        5) build_fuse && start_fuse ;;
        6) build_fuse && start_fuse && test_fuse ;;
        q) exit 0 ;;
        *) echo -e "${RED}Invalid option${NC}" ;;
    esac
}

# Parse command line arguments
case "${1:-}" in
    build)
        build_fuse
        ;;
    start)
        start_fuse
        ;;
    test)
        test_fuse
        ;;
    stop)
        stop_fuse
        ;;
    auto)
        # Automatic mode - build, start, test
        build_fuse
        start_fuse
        test_fuse
        echo
        echo -e "${GREEN}FUSE is running at: $MOUNT_POINT${NC}"
        echo "Press Ctrl+C to stop"
        wait $FUSE_PID
        ;;
    *)
        # Interactive menu
        while true; do
            show_menu
        done
        ;;
esac