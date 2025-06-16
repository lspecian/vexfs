#!/bin/bash

# VexFS Module Reload Script
# Handles complete cleanup and reloading of the VexFS kernel module

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MODULE_DIR="$(dirname "$SCRIPT_DIR")"
PROJECT_ROOT="$(dirname "$MODULE_DIR")"

echo "[$(date '+%Y-%m-%d %H:%M:%S')] === VexFS Module Reload Script ==="

# Function to cleanup everything VexFS related
cleanup_vexfs() {
    echo "Cleaning up VexFS resources..."
    
    # Unmount any VexFS filesystems
    echo "Checking for VexFS mounts..."
    if mount | grep -q "type vexfs\|type fuse.vexfs"; then
        echo "Found VexFS mounts, unmounting..."
        mount | grep "type vexfs\|type fuse.vexfs" | while read -r line; do
            mount_point=$(echo "$line" | awk '{print $3}')
            echo "Unmounting: $mount_point"
            sudo umount "$mount_point" 2>/dev/null || echo "Failed to unmount $mount_point"
        done
    else
        echo "No VexFS mounts found"
    fi
    
    # Detach any loop devices with VexFS
    echo "Checking for VexFS loop devices..."
    if losetup -a | grep -q vexfs; then
        echo "Found VexFS loop devices, detaching..."
        losetup -a | grep vexfs | while read -r line; do
            loop_dev=$(echo "$line" | cut -d: -f1)
            echo "Detaching: $loop_dev"
            sudo losetup -d "$loop_dev" 2>/dev/null || echo "Failed to detach $loop_dev"
        done
    else
        echo "No VexFS loop devices found"
    fi
    
    # Kill any VexFS FUSE processes
    echo "Checking for VexFS FUSE processes..."
    if pgrep -f "vexfs_fuse" > /dev/null; then
        echo "Found VexFS FUSE processes, terminating..."
        sudo pkill -f "vexfs_fuse" || echo "Failed to kill FUSE processes"
        sleep 1
    else
        echo "No VexFS FUSE processes found"
    fi
    
    # Remove kernel module
    echo "Checking for VexFS kernel module..."
    if lsmod | grep -q "^vexfs "; then
        echo "VexFS module loaded, attempting removal..."
        
        # Try normal removal first
        if sudo rmmod vexfs 2>/dev/null; then
            echo "Module removed successfully"
        else
            echo "Normal removal failed, trying force removal..."
            if sudo rmmod -f vexfs 2>/dev/null; then
                echo "Module force-removed successfully"
            else
                echo "Force removal failed, module may be stuck"
                echo "Current module usage:"
                lsmod | grep vexfs || echo "Module not found in lsmod"
                return 1
            fi
        fi
    else
        echo "VexFS module not loaded"
    fi
    
    echo "Cleanup completed"
}

# Function to build and load the module
build_and_load() {
    echo "Building VexFS kernel module..."
    
    cd "$MODULE_DIR"
    
    # Clean and build
    make clean 2>/dev/null || true
    if ! make; then
        echo "ERROR: Failed to build VexFS module"
        return 1
    fi
    
    echo "Loading VexFS kernel module..."
    if sudo insmod vexfs.ko; then
        echo "Module loaded successfully"
        
        # Verify module is loaded and registered
        echo "Verifying module status..."
        if lsmod | grep -q "^vexfs "; then
            echo "✓ Module appears in lsmod"
        else
            echo "✗ Module not found in lsmod"
            return 1
        fi
        
        if grep -q "vexfs" /proc/filesystems; then
            echo "✓ Filesystem registered in /proc/filesystems"
        else
            echo "✗ Filesystem not registered in /proc/filesystems"
            return 1
        fi
        
        echo "Module loaded and verified successfully"
        return 0
    else
        echo "ERROR: Failed to load VexFS module"
        return 1
    fi
}

# Function to run a quick test
quick_test() {
    echo "Running quick mount test..."
    
    # Create test environment
    TEST_DIR="/tmp/vexfs_reload_test"
    TEST_IMG="$TEST_DIR/test.img"
    MOUNT_POINT="$TEST_DIR/mount"
    
    mkdir -p "$TEST_DIR" "$MOUNT_POINT"
    
    # Create test image
    dd if=/dev/zero of="$TEST_IMG" bs=1M count=100 2>/dev/null
    
    # Setup loop device
    LOOP_DEV=$(sudo losetup -f --show "$TEST_IMG")
    echo "Created loop device: $LOOP_DEV"
    
    # Create basic VexFS superblock
    echo "Creating VexFS superblock..."
    python3 -c "
import struct
import hashlib

# VexFS superblock structure
magic = b'VEXFS100'
version = 1
block_size = 4096
total_blocks = 25600  # 100MB / 4KB
free_blocks = 25590
inode_count = 1000
free_inodes = 999
root_inode = 1

# Pack superblock
sb = struct.pack('<8sIIQQQQQ', magic, version, block_size, total_blocks, free_blocks, inode_count, free_inodes, root_inode)
sb += b'\x00' * (4096 - len(sb))  # Pad to block size

with open('$TEST_IMG', 'r+b') as f:
    f.write(sb)
print('VexFS superblock created')
"
    
    # Test mount
    echo "Testing mount operation..."
    if timeout 10 sudo mount -t vexfs "$LOOP_DEV" "$MOUNT_POINT" 2>/dev/null; then
        echo "✓ Mount successful"
        
        # Test basic operations
        if sudo touch "$MOUNT_POINT/test_file" 2>/dev/null; then
            echo "✓ File creation successful"
        else
            echo "✗ File creation failed"
        fi
        
        # Unmount
        sudo umount "$MOUNT_POINT" 2>/dev/null || echo "Unmount failed"
        echo "✓ Test completed successfully"
        TEST_RESULT=0
    else
        echo "✗ Mount failed"
        TEST_RESULT=1
    fi
    
    # Cleanup
    sudo losetup -d "$LOOP_DEV" 2>/dev/null || true
    rm -rf "$TEST_DIR"
    
    return $TEST_RESULT
}

# Main execution
main() {
    local do_test=false
    local force_cleanup=false
    
    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            --test)
                do_test=true
                shift
                ;;
            --force)
                force_cleanup=true
                shift
                ;;
            --help)
                echo "Usage: $0 [--test] [--force] [--help]"
                echo "  --test   Run a quick mount test after reload"
                echo "  --force  Force cleanup even if it might be risky"
                echo "  --help   Show this help message"
                exit 0
                ;;
            *)
                echo "Unknown option: $1"
                exit 1
                ;;
        esac
    done
    
    # Step 1: Cleanup
    echo "=== Step 1: Cleanup ==="
    if ! cleanup_vexfs; then
        if [[ "$force_cleanup" == "true" ]]; then
            echo "Cleanup failed but continuing due to --force"
        else
            echo "ERROR: Cleanup failed. Use --force to continue anyway."
            exit 1
        fi
    fi
    
    # Step 2: Build and load
    echo "=== Step 2: Build and Load ==="
    if ! build_and_load; then
        echo "ERROR: Failed to build and load module"
        exit 1
    fi
    
    # Step 3: Optional test
    if [[ "$do_test" == "true" ]]; then
        echo "=== Step 3: Quick Test ==="
        if quick_test; then
            echo "✓ All tests passed"
        else
            echo "✗ Tests failed"
            exit 1
        fi
    fi
    
    echo "=== Module Reload Completed Successfully ==="
}

# Check if running as root for some operations
if [[ $EUID -eq 0 ]]; then
    echo "WARNING: Running as root"
fi

main "$@"