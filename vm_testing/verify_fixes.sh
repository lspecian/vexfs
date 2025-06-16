#!/bin/bash
# Quick verification script for VexFS fixes

echo "VexFS Fix Verification"
echo "===================="

# Check if fixes are in place
echo "Checking block.c for spinlock removal..."
if grep -q "spin_lock_irqsave" kernel_module/core/block.c; then
    echo "❌ WARNING: spinlock still present in block.c"
else
    echo "✅ Spinlock removed from block.c"
fi

echo "Checking for dir_fix.c..."
if [ -f "kernel_module/core/dir_fix.c" ]; then
    echo "✅ Directory fix implemented"
else
    echo "❌ Directory fix missing"
fi

echo "Checking for file_enhanced.c..."
if [ -f "kernel_module/core/file_enhanced.c" ]; then
    echo "✅ Enhanced file operations implemented"
else
    echo "❌ Enhanced file operations missing"
fi

echo "Checking build system..."
if grep -q "file_enhanced.o" kernel_module/Kbuild; then
    echo "✅ Enhanced file operations in build"
else
    echo "❌ Enhanced file operations not in build"
fi
