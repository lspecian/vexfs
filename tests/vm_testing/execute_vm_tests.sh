#!/bin/bash

echo "=== VexFS Kernel Module Test Execution ==="
echo "Timestamp: $(date)"
echo ""

# Check if VM is running
VM_PID=$(ps aux | grep qemu-system-x86_64 | grep vexfs-test-vm | grep -v grep | awk '{print $2}' | tail -1)
if [ -n "$VM_PID" ]; then
    echo "✅ VM is running with PID: $VM_PID"
else
    echo "❌ ERROR: VM not found running"
    exit 1
fi

echo ""
echo "=== VM Test Commands ==="
echo "The following commands will be executed in the VM:"
echo "1. Check VM responsiveness"
echo "2. Verify shared directory access"
echo "3. Check kernel module availability"
echo "4. Execute VexFS kernel module test script"
echo ""

# Create command sequence for VM
cat << 'VMEOF'

echo "=== VexFS VM Test Session Started ==="
date
echo ""

echo "=== System Information ==="
uname -a
cat /proc/version
echo ""

echo "=== Memory Information ==="
free -h
echo ""

echo "=== Checking shared directory mount ==="
mount | grep vexfs_host
echo ""

echo "=== Shared directory contents ==="
ls -la /mnt/vexfs_host/
echo ""

echo "=== Kernel module location ==="
ls -la /mnt/vexfs_host/kernel/
echo ""

echo "=== Executing VexFS kernel module test script ==="
if [ -f /mnt/vexfs_host/tests/vm_testing/vm_test_script.sh ]; then
    echo "✅ Test script found, executing..."
    sh /mnt/vexfs_host/tests/vm_testing/vm_test_script.sh
else
    echo "❌ Test script not found at /mnt/vexfs_host/tests/vm_testing/vm_test_script.sh"
    echo "Available files in tests/vm_testing:"
    ls -la /mnt/vexfs_host/tests/vm_testing/
fi

echo ""
echo "=== VexFS VM Test Session Completed ==="
date

VMEOF

