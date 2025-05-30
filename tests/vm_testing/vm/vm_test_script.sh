#!/bin/bash

echo "VexFS Kernel Module Testing in VM"
echo "================================="

# Check if running in VM
if ! grep -q "QEMU" /proc/cpuinfo 2>/dev/null; then
    echo "Warning: This doesn't appear to be running in a QEMU VM"
fi

# Check if kernel module is available
if [[ -f /mnt/vexfs_host/kernel/vexfs.ko ]]; then
    echo "Found VexFS kernel module: $(ls -lh /mnt/vexfs_host/kernel/vexfs.ko)"
    
    # Copy module to local filesystem
    sudo cp /mnt/vexfs_host/kernel/vexfs.ko /tmp/vexfs.ko
    
    echo "Running basic kernel module tests..."
    
    # Test 1: Module info
    echo "=== Test 1: Module Information ==="
    modinfo /tmp/vexfs.ko
    
    # Test 2: Load module
    echo "=== Test 2: Loading Module ==="
    sudo insmod /tmp/vexfs.ko
    
    if lsmod | grep -q vexfs; then
        echo "SUCCESS: VexFS module loaded"
        lsmod | grep vexfs
        
        # Check dmesg for module messages
        echo "=== Kernel Messages ==="
        dmesg | tail -10
        
        # Test 3: Unload module
        echo "=== Test 3: Unloading Module ==="
        sudo rmmod vexfs
        
        if ! lsmod | grep -q vexfs; then
            echo "SUCCESS: VexFS module unloaded"
        else
            echo "WARNING: Module may still be loaded"
        fi
    else
        echo "ERROR: Failed to load VexFS module"
        dmesg | tail -10
    fi
    
    # Test 4: Stress test (load/unload cycles)
    echo "=== Test 4: Stress Test (5 cycles) ==="
    for i in {1..5}; do
        echo "Cycle $i..."
        sudo insmod /tmp/vexfs.ko
        sleep 1
        sudo rmmod vexfs
        sleep 1
    done
    echo "Stress test completed"
    
else
    echo "ERROR: VexFS kernel module not found"
    echo "Make sure the shared directory is mounted"
fi
