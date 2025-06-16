#!/bin/bash

# Check VexFS Alpine VM Status

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
VM_DIR="$SCRIPT_DIR"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}VexFS Alpine VM Status Check${NC}"
echo -e "${BLUE}===========================${NC}"

# Check if VM disk exists
if [ -f "$VM_DIR/images/vexfs_alpine_test.qcow2" ]; then
    echo -e "${GREEN}✅ VM disk image exists${NC}"
    ls -lh "$VM_DIR/images/vexfs_alpine_test.qcow2"
else
    echo -e "${RED}❌ VM disk image not found${NC}"
fi

# Check if Alpine ISO exists
if [ -f "$VM_DIR/images/alpine-virt-3.19.0-x86_64.iso" ]; then
    echo -e "${GREEN}✅ Alpine ISO exists${NC}"
else
    echo -e "${RED}❌ Alpine ISO not found${NC}"
fi

# Check if VM is running
if [ -f "$VM_DIR/qemu.pid" ]; then
    PID=$(cat "$VM_DIR/qemu.pid")
    if ps -p $PID > /dev/null; then
        echo -e "${GREEN}✅ VM is running (PID: $PID)${NC}"
        echo -e "${BLUE}   SSH: ssh -p 2222 root@localhost${NC}"
    else
        echo -e "${YELLOW}⚠️  VM PID file exists but process not running${NC}"
        rm -f "$VM_DIR/qemu.pid"
    fi
else
    echo -e "${YELLOW}⚠️  VM is not running${NC}"
fi

# Check if Alpine is installed
if [ -f "$VM_DIR/.alpine_installed" ]; then
    echo -e "${GREEN}✅ Alpine is installed in VM${NC}"
else
    echo -e "${YELLOW}⚠️  Alpine not yet installed${NC}"
    echo -e "${BLUE}   Run setup on first boot: /mnt/shared/setup_alpine_auto.sh${NC}"
fi

# Check shared directory
echo -e "\n${BLUE}Shared Directory Contents:${NC}"
if [ -d "$VM_DIR/shared" ]; then
    echo -e "${GREEN}✅ Shared directory exists${NC}"
    
    if [ -f "$VM_DIR/shared/vexfs_deadlock_fix.ko" ]; then
        echo -e "${GREEN}   ✅ Kernel module present${NC}"
    else
        echo -e "${RED}   ❌ Kernel module missing${NC}"
    fi
    
    if [ -f "$VM_DIR/shared/test_vexfs_alpine.sh" ]; then
        echo -e "${GREEN}   ✅ Test script present${NC}"
    else
        echo -e "${RED}   ❌ Test script missing${NC}"
    fi
    
    echo -e "\n${BLUE}All files in shared:${NC}"
    ls -la "$VM_DIR/shared/" | grep -v "^total\|^d"
else
    echo -e "${RED}❌ Shared directory not found${NC}"
fi

# Check SSH availability
echo -e "\n${BLUE}Testing SSH connectivity:${NC}"
if timeout 2 bash -c 'cat < /dev/null > /dev/tcp/localhost/2222' 2>/dev/null; then
    echo -e "${GREEN}✅ SSH port 2222 is open${NC}"
    echo -e "${BLUE}   Try: ssh -p 2222 root@localhost (password: vexfs)${NC}"
else
    echo -e "${YELLOW}⚠️  SSH port 2222 not accessible${NC}"
    if [ -f "$VM_DIR/qemu.pid" ]; then
        echo -e "${BLUE}   VM might still be booting or Alpine not installed${NC}"
    fi
fi

echo -e "\n${BLUE}Quick Commands:${NC}"
echo "Start VM:  ./vm_testing/scripts/start_alpine_vm.sh"
echo "Stop VM:   kill \$(cat vm_testing/qemu.pid)"
echo "SSH:       ssh -p 2222 root@localhost"
echo "Status:    ./vm_testing/check_vm_status.sh"