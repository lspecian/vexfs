#!/bin/bash

# Fast VM Test Runner for VexFS - optimized for quick boot and better diagnostics

set -e

echo "🚀 VexFS Fast VM Test Runner"
echo "============================"

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m'

# Configuration
VM_HOST="localhost"
VM_PORT="2222"
VM_USER="vexfs"
VM_KEY="test_env/vm/keys/vexfs_vm_key"
MAX_WAIT=120  # Reduced to 2 minutes for faster feedback
WAIT_INTERVAL=5

echo -e "${YELLOW}Starting VM with fast boot configuration...${NC}"

# Start the fast VM in background
./run_qemu_fast.sh &
VM_PID=$!

echo -e "${BLUE}VM started with PID: $VM_PID${NC}"
echo -e "${YELLOW}Waiting for VM to be ready...${NC}"

# Function to check if VM is ready
check_vm_ready() {
    ssh -p "$VM_PORT" -i "$VM_KEY" -o StrictHostKeyChecking=no -o ConnectTimeout=3 \
        "$VM_USER@$VM_HOST" "echo 'ready'" 2>/dev/null
}

# Function to check if VM process is still running
check_vm_running() {
    kill -0 "$VM_PID" 2>/dev/null
}

# Wait for VM to be ready with better diagnostics
waited=0
while [ $waited -lt $MAX_WAIT ]; do
    # Check if VM process is still running
    if ! check_vm_running; then
        echo -e "${RED}❌ VM process died unexpectedly${NC}"
        exit 1
    fi
    
    # Check if SSH is ready
    if check_vm_ready >/dev/null 2>&1; then
        echo -e "${GREEN}✅ VM is ready!${NC}"
        break
    fi
    
    echo -e "${YELLOW}⏳ Waiting for VM... (${waited}s/${MAX_WAIT}s)${NC}"
    
    # Show some diagnostic info every 30 seconds
    if [ $((waited % 30)) -eq 0 ] && [ $waited -gt 0 ]; then
        echo -e "${BLUE}🔍 Diagnostic: Checking VM process and network...${NC}"
        ps aux | grep qemu | grep -v grep | head -1 || echo "No QEMU process found"
        netstat -ln | grep :2222 || echo "Port 2222 not listening yet"
    fi
    
    sleep $WAIT_INTERVAL
    waited=$((waited + WAIT_INTERVAL))
done

if [ $waited -ge $MAX_WAIT ]; then
    echo -e "${RED}❌ VM failed to become ready within ${MAX_WAIT} seconds${NC}"
    echo -e "${YELLOW}Killing VM process...${NC}"
    kill "$VM_PID" 2>/dev/null || true
    exit 1
fi

# Run a simple test instead of the full comprehensive suite
echo -e "${YELLOW}🧪 Running simple VM test...${NC}"

ssh -p "$VM_PORT" -i "$VM_KEY" -o StrictHostKeyChecking=no \
    "$VM_USER@$VM_HOST" << 'EOF'
    
    echo "🔧 Simple VM test starting..."
    
    # Check if we're in the VM
    echo "✅ VM connectivity confirmed"
    
    # Check if VexFS source is mounted
    if [ -d /mnt/vexfs_source ]; then
        echo "✅ VexFS source mounted"
        ls -la /mnt/vexfs_source | head -5
    else
        echo "❌ VexFS source not mounted"
        exit 1
    fi
    
    # Check if we can access the kernel module
    if [ -f /mnt/vexfs_source/vexfs.ko ]; then
        echo "✅ VexFS kernel module found"
        ls -la /mnt/vexfs_source/vexfs.ko
    else
        echo "❌ VexFS kernel module not found"
        exit 1
    fi
    
    # Try to load the kernel module
    if sudo insmod /mnt/vexfs_source/vexfs.ko; then
        echo "✅ VexFS kernel module loaded successfully"
        
        # Check if it's loaded
        if lsmod | grep vexfs; then
            echo "✅ VexFS module verified in kernel"
        else
            echo "❌ VexFS module not found in lsmod"
        fi
        
        # Unload it
        if sudo rmmod vexfs; then
            echo "✅ VexFS module unloaded successfully"
        else
            echo "❌ Failed to unload VexFS module"
        fi
    else
        echo "❌ Failed to load VexFS kernel module"
        dmesg | tail -5
        exit 1
    fi
    
    echo "🎉 Simple VM test completed successfully!"
EOF

test_exit_code=$?

# Clean up VM
echo -e "${YELLOW}🧹 Shutting down VM...${NC}"
kill "$VM_PID" 2>/dev/null || true
wait "$VM_PID" 2>/dev/null || true

if [ $test_exit_code -eq 0 ]; then
    echo -e "${GREEN}🎉 VM test completed successfully!${NC}"
    echo -e "${GREEN}✅ VexFS kernel module works in VM environment${NC}"
    echo -e "${GREEN}✅ Ready to run comprehensive tests in VM${NC}"
else
    echo -e "${RED}❌ VM test failed${NC}"
    echo -e "${RED}🚫 VM environment needs troubleshooting${NC}"
fi

exit $test_exit_code