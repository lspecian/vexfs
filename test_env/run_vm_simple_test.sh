#!/bin/bash

# Simple VM Test - Try to get basic VM working with minimal SSH requirements

set -e

echo "🚀 VexFS Simple VM Test"
echo "======================="

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m'

# Start VM in background with console output
echo -e "${YELLOW}Starting VM with console access...${NC}"

# Use a simpler approach - start VM and try to connect via VNC or console
./run_qemu_fast.sh &
VM_PID=$!

echo -e "${BLUE}VM started with PID: $VM_PID${NC}"

# Wait a bit for VM to boot
echo -e "${YELLOW}Waiting 60 seconds for VM to boot...${NC}"
sleep 60

# Try to connect via SSH with more verbose output
echo -e "${YELLOW}Testing SSH connection...${NC}"
ssh -v -p 2222 -i vm/keys/vexfs_vm_key -o StrictHostKeyChecking=no -o ConnectTimeout=10 \
    vexfs@localhost "echo 'SSH connection successful'" 2>&1 | head -20

ssh_result=$?

if [ $ssh_result -eq 0 ]; then
    echo -e "${GREEN}✅ SSH connection successful!${NC}"
    
    # Run a simple test
    echo -e "${YELLOW}Running simple test in VM...${NC}"
    ssh -p 2222 -i vm/keys/vexfs_vm_key -o StrictHostKeyChecking=no \
        vexfs@localhost << 'EOF'
        echo "✅ Inside VM successfully"
        echo "Hostname: $(hostname)"
        echo "User: $(whoami)"
        echo "Date: $(date)"
        
        # Check if VexFS source is mounted
        if [ -d /mnt/vexfs_source ]; then
            echo "✅ VexFS source mounted"
            ls -la /mnt/vexfs_source | head -3
        else
            echo "❌ VexFS source not mounted"
        fi
EOF
    
    echo -e "${GREEN}🎉 VM test successful!${NC}"
else
    echo -e "${RED}❌ SSH connection failed${NC}"
    echo -e "${YELLOW}Checking VM process...${NC}"
    if kill -0 "$VM_PID" 2>/dev/null; then
        echo -e "${BLUE}VM process is still running${NC}"
        echo -e "${YELLOW}Checking network...${NC}"
        netstat -ln | grep :2222 || echo "Port 2222 not listening"
    else
        echo -e "${RED}VM process died${NC}"
    fi
fi

# Clean up
echo -e "${YELLOW}Shutting down VM...${NC}"
kill "$VM_PID" 2>/dev/null || true
wait "$VM_PID" 2>/dev/null || true

if [ $ssh_result -eq 0 ]; then
    echo -e "${GREEN}✅ VM test completed successfully${NC}"
    exit 0
else
    echo -e "${RED}❌ VM test failed - SSH not ready${NC}"
    exit 1
fi