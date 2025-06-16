#!/bin/bash
# Cleanup stuck mount and test fresh

set -e

# Check if running as root
if [ "$EUID" -ne 0 ]; then 
    echo "Error: This script must be run as root (use sudo)"
    exit 1
fi

echo "Cleaning up existing VexFS mounts and modules..."

# Force unmount if stuck
if mount | grep -q "/mnt/vexfs_persist_test"; then
    echo "Force unmounting /mnt/vexfs_persist_test..."
    umount -f /mnt/vexfs_persist_test 2>/dev/null || umount -l /mnt/vexfs_persist_test 2>/dev/null || true
fi

# Kill any processes using the mount
fuser -km /mnt/vexfs_persist_test 2>/dev/null || true

# Unload module
if lsmod | grep -q "vexfs_deadlock_fix"; then
    echo "Unloading VexFS module..."
    rmmod -f vexfs_deadlock_fix 2>/dev/null || true
fi

# Wait a bit
sleep 2

# Check status
echo -e "\nCurrent status:"
echo "Modules:"
lsmod | grep vexfs || echo "  No VexFS modules loaded"
echo -e "\nMounts:"
mount | grep vexfs || echo "  No VexFS mounts"

echo -e "\nCleanup complete!"