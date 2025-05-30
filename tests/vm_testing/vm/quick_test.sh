#!/bin/bash

# Quick verification that the kernel module can be loaded
# Run this inside the VM

echo "Quick VexFS Kernel Module Test"
echo "=============================="

if [[ ! -f ~/vexfs.ko ]]; then
    echo "ERROR: Kernel module not found at ~/vexfs.ko"
    echo "Make sure the shared directory is mounted and the module is built"
    exit 1
fi

echo "Kernel module found: $(ls -lh ~/vexfs.ko)"
echo ""

echo "Checking module info..."
modinfo ~/vexfs.ko

echo ""
echo "Attempting to load module..."
sudo insmod ~/vexfs.ko

if lsmod | grep -q vexfs; then
    echo "SUCCESS: VexFS module loaded successfully!"
    echo "Module details:"
    lsmod | grep vexfs
    
    echo ""
    echo "Checking dmesg for module messages..."
    dmesg | tail -10
    
    echo ""
    echo "Unloading module..."
    sudo rmmod vexfs
    
    if ! lsmod | grep -q vexfs; then
        echo "SUCCESS: Module unloaded successfully!"
    else
        echo "WARNING: Module may still be loaded"
    fi
else
    echo "ERROR: Failed to load VexFS module"
    echo "Check dmesg for error messages:"
    dmesg | tail -10
    exit 1
fi

echo ""
echo "Quick test completed successfully!"
