#!/bin/bash
# Quick VexFS Verification Script
# Non-destructive test to verify kernel module and basic operations

set -e

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo -e "${GREEN}VexFS Quick Verification${NC}"
echo "========================"

# Check components
echo -e "\n${YELLOW}Checking VexFS components:${NC}"

# 1. Kernel module
if [ -f "/home/luis/Development/oss/vexfs/kernel_module/vexfs_deadlock_fix.ko" ]; then
    echo -e "  ${GREEN}✓${NC} Kernel module built"
    modinfo /home/luis/Development/oss/vexfs/kernel_module/vexfs_deadlock_fix.ko | grep -E "version|description" || true
else
    echo -e "  ${RED}✗${NC} Kernel module not found"
fi

# 2. mkfs.vexfs tool
if [ -f "/home/luis/Development/oss/vexfs/tools/mkfs.vexfs" ]; then
    echo -e "  ${GREEN}✓${NC} mkfs.vexfs tool exists"
else
    echo -e "  ${RED}✗${NC} mkfs.vexfs tool not found"
fi

# 3. Test images
echo -e "\n${YELLOW}Test images:${NC}"
ls -lh /home/luis/Development/oss/vexfs/tools/*.img 2>/dev/null | grep -E "test_.*\.img" || echo "  No test images found"

# 4. Check if module is currently loaded
echo -e "\n${YELLOW}Module status:${NC}"
if lsmod | grep -q vexfs; then
    echo -e "  ${YELLOW}⚠${NC}  VexFS module is currently loaded:"
    lsmod | grep vexfs
else
    echo -e "  ${GREEN}✓${NC} No VexFS module currently loaded"
fi

# 5. Check mount status
echo -e "\n${YELLOW}Mount status:${NC}"
if mount | grep -q vexfs; then
    echo -e "  ${YELLOW}⚠${NC}  VexFS filesystems mounted:"
    mount | grep vexfs
else
    echo -e "  ${GREEN}✓${NC} No VexFS filesystems currently mounted"
fi

# 6. Recent kernel messages
echo -e "\n${YELLOW}Recent VexFS kernel messages:${NC}"
if [ "$EUID" -eq 0 ]; then
    dmesg | grep -i vexfs | tail -5 || echo "  No recent VexFS messages"
else
    echo "  (Run as root to see kernel messages)"
fi

echo -e "\n${GREEN}Verification complete!${NC}"
echo -e "\nTo run full tests:"
echo "  - Basic mount test: sudo ./test_mount.sh"
echo "  - Persistence test: sudo ./test_persistence.sh"
echo "  - VM testing: cd vm_testing && ./run_vm_test.sh"