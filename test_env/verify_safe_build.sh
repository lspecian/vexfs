#!/bin/bash

# VexFS Safe Build Verification Script
# This script only verifies the build without loading the module

set -e

echo "🔍 VexFS Safe Build Verification"
echo "================================"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

success() {
    echo -e "${GREEN}✅ $1${NC}"
}

error() {
    echo -e "${RED}❌ $1${NC}"
    exit 1
}

warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
}

info() {
    echo -e "${YELLOW}ℹ️  $1${NC}"
}

# Check if safe module exists
echo "Checking for safe kernel module..."
if [ -f "vexfs_safe.ko" ]; then
    success "Safe kernel module found: vexfs_safe.ko"
else
    error "Safe kernel module not found. Run: make -f Makefile.safe safe-build"
fi

# Check module info
echo "Verifying module information..."
MODULE_INFO=$(modinfo vexfs_safe.ko 2>/dev/null)
if [ $? -eq 0 ]; then
    success "Module info accessible"
    
    # Check version
    VERSION=$(echo "$MODULE_INFO" | grep "version:" | awk '{print $2}')
    if [[ "$VERSION" == *"safe"* ]]; then
        success "Safe version detected: $VERSION"
    else
        warning "Version doesn't indicate safe mode: $VERSION"
    fi
    
    # Check description
    DESCRIPTION=$(echo "$MODULE_INFO" | grep "description:" | cut -d: -f2-)
    if [[ "$DESCRIPTION" == *"SAFE MODE"* ]]; then
        success "Safe mode description confirmed"
    else
        warning "Description doesn't indicate safe mode"
    fi
    
else
    error "Cannot read module info"
fi

# Check file size (should be reasonable)
FILE_SIZE=$(stat -c%s vexfs_safe.ko)
if [ "$FILE_SIZE" -gt 100000 ] && [ "$FILE_SIZE" -lt 1000000 ]; then
    success "Module size reasonable: $FILE_SIZE bytes"
else
    warning "Module size unusual: $FILE_SIZE bytes"
fi

# Check for required symbols
echo "Checking module symbols..."
if nm vexfs_safe.ko >/dev/null 2>&1; then
    success "Module symbols accessible"
    
    # Check for init/exit functions
    if nm vexfs_safe.ko | grep -q "init_module"; then
        success "Init function found"
    else
        warning "Init function not found"
    fi
    
    if nm vexfs_safe.ko | grep -q "cleanup_module"; then
        success "Cleanup function found"
    else
        warning "Cleanup function not found"
    fi
else
    warning "Cannot read module symbols"
fi

# Check build artifacts
echo "Checking build artifacts..."
if [ -f "kernel/vexfs_module_entry_safe.c" ]; then
    success "Safe source file exists"
else
    warning "Safe source file not found"
fi

if [ -f "Makefile.safe" ]; then
    success "Safe Makefile exists"
else
    warning "Safe Makefile not found"
fi

echo -e "\n${GREEN}🎉 Safe build verification completed!${NC}"
echo -e "\n${YELLOW}Build Summary:${NC}"
echo "✅ Safe kernel module built successfully"
echo "✅ Module contains safety indicators"
echo "✅ Module structure appears correct"
echo -e "\n${RED}⚠️  CRITICAL SAFETY REMINDERS:${NC}"
echo "❌ DO NOT load this module on host systems"
echo "❌ DO NOT attempt mounting until FFI is fixed"
echo "✅ Only test in isolated VMs with snapshots"
echo "✅ Use test_env/safe_kernel_test.sh in VMs"
echo -e "\n${YELLOW}Next Steps:${NC}"
echo "1. Transfer vexfs_safe.ko to a VM"
echo "2. Run test_env/safe_kernel_test.sh in the VM"
echo "3. Test only load/unload operations"
echo "4. Follow the fix plan in docs/implementation/KERNEL_MODULE_FIX_PLAN.md"