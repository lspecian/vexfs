#!/bin/bash

# VexFS Comprehensive VM Testing Script
# This script runs a complete test suite in the VM environment

set -e

VM_KEY="test_env/vm/keys/vexfs_vm_key"
VM_HOST="localhost"
VM_PORT="2222"
VM_USER="vexfs"

echo "🧪 VexFS Comprehensive VM Testing Suite"
echo "========================================"

# Function to run command in VM
run_in_vm() {
    ssh -i "$VM_KEY" -o StrictHostKeyChecking=no "$VM_USER@$VM_HOST" -p "$VM_PORT" "source ~/.cargo/env && $1"
}

# Test 1: Verify VM connectivity and source mount
echo "1. Testing VM connectivity and source mount..."
run_in_vm "cd /mnt/vexfs_source && pwd && ls -la | head -5"
echo "✅ VM connectivity: SUCCESS"

# Test 2: Rust compilation
echo "2. Testing Rust compilation..."
run_in_vm "cd /mnt/vexfs_source && cargo check --quiet"
echo "✅ Rust compilation: SUCCESS"

# Test 3: Vector operations
echo "3. Testing vector operations..."
run_in_vm "cd /mnt/vexfs_source && timeout 30 cargo run --bin vector_test_runner --quiet"
echo "✅ Vector operations: SUCCESS"

# Test 4: FFI integration
echo "4. Testing FFI integration..."
run_in_vm "cd /mnt/vexfs_source && ./test_ffi"
echo "✅ FFI integration: SUCCESS"

# Test 5: Kernel module build
echo "5. Testing kernel module build..."
run_in_vm "cd /mnt/vexfs_source && make clean > /dev/null 2>&1 && make > /dev/null 2>&1 && ls -la vexfs.ko"
echo "✅ Kernel module build: SUCCESS"

# Test 6: Module loading/unloading
echo "6. Testing kernel module loading..."
run_in_vm "cd /mnt/vexfs_source && sudo insmod vexfs.ko && lsmod | grep vexfs"
echo "✅ Module loading: SUCCESS"

echo "7. Testing kernel module unloading..."
run_in_vm "sudo rmmod vexfs"
echo "✅ Module unloading: SUCCESS"

# Test 8: System information
echo "8. Gathering system information..."
run_in_vm "uname -a && free -h && df -h /mnt/vexfs_source"

echo ""
echo "🎉 ALL VM TESTS COMPLETED SUCCESSFULLY!"
echo "========================================"
echo "VexFS is fully functional in VM environment"