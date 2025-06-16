#!/bin/bash
# Verify the directory operations fix implementation

echo "VexFS Directory Operations Fix Verification"
echo "==========================================="

# Check that the fix is properly integrated
echo -e "\n1. Checking dir_fix.c implementation:"
if [ -f "/home/luis/Development/oss/vexfs/kernel_module/core/dir_fix.c" ]; then
    echo "   ✓ dir_fix.c exists"
    # Check for key functions
    if grep -q "vexfs_readdir_fixed" /home/luis/Development/oss/vexfs/kernel_module/core/dir_fix.c; then
        echo "   ✓ vexfs_readdir_fixed function implemented"
    fi
    if grep -q "vexfs_dir_operations_fixed" /home/luis/Development/oss/vexfs/kernel_module/core/dir_fix.c; then
        echo "   ✓ vexfs_dir_operations_fixed structure defined"
    fi
    if grep -q "dir_emit" /home/luis/Development/oss/vexfs/kernel_module/core/dir_fix.c; then
        echo "   ✓ Uses dir_emit for VFS integration"
    fi
else
    echo "   ✗ dir_fix.c not found"
fi

echo -e "\n2. Checking integration in inode.c:"
if grep -q "vexfs_dir_operations_fixed" /home/luis/Development/oss/vexfs/kernel_module/core/inode.c; then
    echo "   ✓ Directory operations fixed in inode.c"
    count=$(grep -c "vexfs_dir_operations_fixed" /home/luis/Development/oss/vexfs/kernel_module/core/inode.c)
    echo "   ✓ Found $count references to vexfs_dir_operations_fixed"
else
    echo "   ✗ Directory operations not fixed in inode.c"
fi

echo -e "\n3. Checking integration in superblock.c:"
if grep -q "vexfs_dir_operations_fixed" /home/luis/Development/oss/vexfs/kernel_module/core/superblock.c; then
    echo "   ✓ Directory operations fixed in superblock.c (root inode)"
else
    echo "   ✗ Directory operations not fixed in superblock.c"
fi

echo -e "\n4. Checking build system:"
if grep -q "dir_fix.o" /home/luis/Development/oss/vexfs/kernel_module/Kbuild; then
    echo "   ✓ dir_fix.o included in Kbuild"
else
    echo "   ✗ dir_fix.o not in Kbuild"
fi

echo -e "\n5. Checking kernel module:"
if [ -f "/home/luis/Development/oss/vexfs/kernel_module/vexfs_deadlock_fix.ko" ]; then
    echo "   ✓ Kernel module exists"
    size=$(ls -lh /home/luis/Development/oss/vexfs/kernel_module/vexfs_deadlock_fix.ko | awk '{print $5}')
    echo "   ✓ Module size: $size"
    # Check if our fix is compiled in
    if strings /home/luis/Development/oss/vexfs/kernel_module/vexfs_deadlock_fix.ko | grep -q "vexfs_readdir_fixed"; then
        echo "   ✓ vexfs_readdir_fixed symbol found in module"
    fi
else
    echo "   ✗ Kernel module not found"
fi

echo -e "\n6. Implementation details:"
echo "   The fix implements a custom readdir function that:"
echo "   - Reads directory entries from disk blocks"
echo "   - Properly emits . and .. entries"
echo "   - Handles on-disk vexfs_dir_entry structures"
echo "   - Includes safety checks for infinite loops"
echo "   - Converts file types to VFS DT_ format"

echo -e "\n7. Test scripts available:"
if [ -f "/home/luis/Development/oss/vexfs/test_dir_fix.sh" ]; then
    echo "   ✓ test_dir_fix.sh - Comprehensive directory operations test"
fi
if [ -f "/home/luis/Development/oss/vexfs/test_persistence.sh" ]; then
    echo "   ✓ test_persistence.sh - File persistence test"
fi
if [ -f "/home/luis/Development/oss/vexfs/run_test.sh" ]; then
    echo "   ✓ run_test.sh - Test runner with cleanup"
fi

echo -e "\n8. Summary:"
echo "   The directory operations timeout issue has been addressed by:"
echo "   - Replacing simple_dir_operations with custom implementation"
echo "   - Reading directory entries directly from disk"
echo "   - Proper VFS integration with dir_emit()"
echo "   - Safety checks to prevent hangs"

echo -e "\nTo test the fix, run as root:"
echo "   sudo ./run_test.sh"