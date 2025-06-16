#!/bin/bash
# Run VexFS directory operations test

set -e

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo -e "${GREEN}VexFS Test Runner${NC}"
echo "=================="

# Check if running as root
if [ "$EUID" -ne 0 ]; then 
    echo -e "${RED}Error: This script must be run as root (use sudo)${NC}"
    exit 1
fi

# Clean up any existing mounts
echo -e "${YELLOW}Cleaning up existing mounts...${NC}"
if mount | grep -q "/mnt/vexfs_persist_test"; then
    umount -l /mnt/vexfs_persist_test 2>/dev/null || true
fi

# Unload existing module
if lsmod | grep -q "vexfs_deadlock_fix"; then
    echo "Unloading existing module..."
    rmmod vexfs_deadlock_fix 2>/dev/null || true
fi

# Wait for cleanup
sleep 2

# Run the directory operations test
echo -e "\n${YELLOW}Running directory operations test...${NC}"
cd /home/luis/Development/oss/vexfs
./test_dir_fix.sh