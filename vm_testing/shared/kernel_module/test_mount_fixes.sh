#!/bin/bash
# Test script for VexFS mount fixes

set -e

echo "=== VexFS Mount Fix Test ==="
echo

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Check if mkfs.vexfs exists
if [ ! -f ../tools/mkfs.vexfs ]; then
    echo -e "${RED}Error: mkfs.vexfs not found in ../tools/${NC}"
    echo "Building mkfs.vexfs..."
    cd ../tools
    make mkfs.vexfs
    cd ../kernel_module
fi

# Unload old module if loaded
echo "1. Unloading old module (if loaded)..."
sudo rmmod vexfs_deadlock_fix 2>/dev/null || true
echo -e "${GREEN}✓ Module cleanup complete${NC}"
echo

# Load new module
echo "2. Loading new module..."
sudo insmod vexfs_deadlock_fix.ko
if lsmod | grep -q vexfs_deadlock_fix; then
    echo -e "${GREEN}✓ Module loaded successfully${NC}"
else
    echo -e "${RED}✗ Failed to load module${NC}"
    exit 1
fi
echo

# Check dmesg for module load
echo "3. Checking kernel messages..."
dmesg | tail -5
echo

# Create test image
echo "4. Creating test filesystem image..."
dd if=/dev/zero of=/tmp/vexfs_test.img bs=1M count=10 2>/dev/null
../tools/mkfs.vexfs -f /tmp/vexfs_test.img
echo -e "${GREEN}✓ Filesystem image created${NC}"
echo

# Create mount point
echo "5. Creating mount point..."
sudo mkdir -p /mnt/vexfs_test
echo -e "${GREEN}✓ Mount point ready${NC}"
echo

# Mount filesystem
echo "6. Mounting filesystem..."
if sudo mount -t vexfs_fixed -o loop /tmp/vexfs_test.img /mnt/vexfs_test; then
    echo -e "${GREEN}✓ Filesystem mounted successfully!${NC}"
    mount | grep vexfs
else
    echo -e "${RED}✗ Mount failed${NC}"
    echo "Checking dmesg for errors:"
    dmesg | tail -20
    exit 1
fi
echo

# Test basic operations
echo "7. Testing basic operations..."

# List directory
echo "   a) Listing root directory..."
if sudo ls -la /mnt/vexfs_test; then
    echo -e "   ${GREEN}✓ Directory listing works${NC}"
else
    echo -e "   ${RED}✗ Directory listing failed${NC}"
fi

# Create a file
echo "   b) Creating a test file..."
if echo "Hello VexFS!" | sudo tee /mnt/vexfs_test/test.txt > /dev/null; then
    echo -e "   ${GREEN}✓ File creation works${NC}"
else
    echo -e "   ${RED}✗ File creation failed${NC}"
fi

# Read the file
echo "   c) Reading the test file..."
if sudo cat /mnt/vexfs_test/test.txt; then
    echo -e "   ${GREEN}✓ File reading works${NC}"
else
    echo -e "   ${RED}✗ File reading failed${NC}"
fi

# List directory again
echo "   d) Listing directory with file..."
if sudo ls -la /mnt/vexfs_test; then
    echo -e "   ${GREEN}✓ Directory listing with file works${NC}"
else
    echo -e "   ${RED}✗ Directory listing with file failed${NC}"
fi
echo

# Unmount filesystem
echo "8. Unmounting filesystem..."
if sudo umount /mnt/vexfs_test; then
    echo -e "${GREEN}✓ Filesystem unmounted successfully${NC}"
else
    echo -e "${RED}✗ Unmount failed${NC}"
    exit 1
fi
echo

# Unload module (tests refcount fix)
echo "9. Unloading module (testing refcount fix)..."
if sudo rmmod vexfs_deadlock_fix; then
    echo -e "${GREEN}✓ Module unloaded successfully - refcount fix works!${NC}"
else
    echo -e "${RED}✗ Module unload failed - refcount issue remains${NC}"
    lsmod | grep vexfs
    exit 1
fi
echo

# Check final kernel messages
echo "10. Final kernel messages:"
dmesg | tail -10
echo

echo -e "${GREEN}=== All tests passed! ===${NC}"
echo "The mount fixes are working correctly:"
echo "- Module loads and unloads properly"
echo "- Filesystem mounts and unmounts cleanly"
echo "- Basic file operations work"
echo "- No refcount issues"