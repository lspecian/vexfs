#!/bin/bash

# VexFS I/O List Crash Fix - Post-Reboot Testing Script
# This script tests the critical I/O list crash fix after system reboot

set -e

echo "=== VexFS I/O List Crash Fix Testing ==="
echo "Testing the fix for kernel panic in inode_io_list_move_locked"
echo

# Check clean environment
echo "1. Verifying clean environment..."
if lsmod | grep -q vexfs; then
    echo "ERROR: VexFS modules still loaded after reboot!"
    lsmod | grep vexfs
    exit 1
fi

if cat /proc/filesystems | grep -q vexfs; then
    echo "ERROR: VexFS filesystems still registered after reboot!"
    cat /proc/filesystems | grep vexfs
    exit 1
fi
echo "✅ Clean environment confirmed"
echo

# Load new module
echo "2. Loading new VexFS module with I/O list fixes..."
cd "$(dirname "$0")/.."
if ! sudo insmod vexfs_iofix.ko; then
    echo "ERROR: Failed to load vexfs_iofix.ko"
    exit 1
fi

# Verify module loaded
if ! lsmod | grep -q vexfs_iofix; then
    echo "ERROR: vexfs_iofix module not found in lsmod"
    exit 1
fi
echo "✅ Module vexfs_iofix loaded successfully"

# Check kernel messages
echo "Recent kernel messages:"
sudo dmesg | tail -5 | grep -E "(VexFS|vexfs)" || echo "No recent VexFS messages"
echo

# Set up loop device
echo "3. Setting up loop device..."
if ! sudo losetup /dev/loop0 ../test_device.img; then
    echo "ERROR: Failed to set up loop device"
    exit 1
fi
echo "✅ Loop device /dev/loop0 set up"

# Create mount point
echo "4. Creating mount point..."
sudo mkdir -p /mnt/vexfs_test
echo "✅ Mount point created"

# Mount filesystem (this should work without deadlocks)
echo "5. Mounting VexFS filesystem..."
if ! sudo mount -t vexfs_iofix /dev/loop0 /mnt/vexfs_test/; then
    echo "ERROR: Failed to mount VexFS filesystem"
    sudo losetup -d /dev/loop0
    exit 1
fi
echo "✅ VexFS mounted successfully without deadlocks"

# CRITICAL TEST: Directory listing (previously caused kernel panic)
echo "6. CRITICAL TEST: Directory listing (previously caused crash)..."
echo "This is the test that previously caused kernel panic in inode_io_list_move_locked"
if ! ls -la /mnt/vexfs_test/; then
    echo "ERROR: Directory listing failed"
    sudo umount /mnt/vexfs_test
    sudo losetup -d /dev/loop0
    exit 1
fi
echo "✅ CRITICAL SUCCESS: Directory listing works without kernel panic!"
echo

# Test file operations
echo "7. Testing file operations..."
if ! echo "I/O list fix test" | sudo tee /mnt/vexfs_test/test_file.txt > /dev/null; then
    echo "ERROR: File creation failed"
    sudo umount /mnt/vexfs_test
    sudo losetup -d /dev/loop0
    exit 1
fi

if ! cat /mnt/vexfs_test/test_file.txt; then
    echo "ERROR: File reading failed"
    sudo umount /mnt/vexfs_test
    sudo losetup -d /dev/loop0
    exit 1
fi
echo "✅ File operations working correctly"
echo

# Test unmounting
echo "8. Testing filesystem unmount..."
if ! sudo umount /mnt/vexfs_test; then
    echo "ERROR: Failed to unmount filesystem"
    exit 1
fi
echo "✅ Filesystem unmounted successfully"

# Clean up loop device
echo "9. Cleaning up loop device..."
sudo losetup -d /dev/loop0
echo "✅ Loop device cleaned up"

# Test module unloading
echo "10. Testing module unloading..."
if ! sudo rmmod vexfs_iofix; then
    echo "ERROR: Failed to unload module"
    exit 1
fi
echo "✅ Module unloaded successfully"

echo
echo "🎉 ALL TESTS PASSED! 🎉"
echo "The I/O list crash fix is working correctly:"
echo "  ✅ Module loads without hanging"
echo "  ✅ Filesystem mounts without deadlocks"
echo "  ✅ Directory listing works without kernel panic"
echo "  ✅ File operations work correctly"
echo "  ✅ Module unloads cleanly"
echo
echo "The critical I/O list crash in inode_io_list_move_locked has been RESOLVED!"