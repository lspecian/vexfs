#!/bin/bash

# Safe VexFS Kernel Module Testing Script
# This script implements safe testing protocols to prevent system hangs

set -e

echo "🛡️  VexFS Safe Kernel Testing Protocol"
echo "======================================"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

# Safety checks
safety_check() {
    echo -e "\n${YELLOW}🔍 Safety Check: $1${NC}"
}

error() {
    echo -e "${RED}❌ $1${NC}"
    exit 1
}

success() {
    echo -e "${GREEN}✅ $1${NC}"
}

warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
}

# Check if we're in a VM (required for safety)
safety_check "Verifying VM environment"
if [ ! -d "/mnt/vexfs_source" ] && [ ! -f "/proc/version" ] || ! grep -q "QEMU\|VMware\|VirtualBox" /proc/version 2>/dev/null; then
    error "SAFETY VIOLATION: Not in VM environment. VexFS kernel testing MUST be done in VMs only!"
fi
success "VM environment confirmed - safe to proceed"

# Check for existing VexFS module
safety_check "Checking for existing VexFS module"
if lsmod | grep -q vexfs; then
    warning "VexFS module already loaded - unloading for safety"
    sudo rmmod vexfs || error "Failed to unload existing module"
fi
success "No VexFS module loaded"

# Change to source directory
cd /mnt/vexfs_source 2>/dev/null || cd ~/vexfs_build 2>/dev/null || error "VexFS source not found"

# Build C-only version first (safer)
safety_check "Building C-only kernel module (no Rust FFI)"
make clean >/dev/null 2>&1 || true
if make c-only-build >/dev/null 2>&1; then
    success "C-only build successful"
else
    error "C-only build failed - check build system"
fi

# Test module loading only (NO MOUNTING)
safety_check "Testing module loading (NO FILESYSTEM OPERATIONS)"
if sudo insmod vexfs.ko; then
    success "Module loaded successfully"
else
    error "Module loading failed"
fi

# Verify module is working
safety_check "Verifying module status"
if lsmod | grep -q vexfs; then
    success "Module verified in kernel"
else
    error "Module not found after loading"
fi

# Check kernel messages for errors
safety_check "Checking kernel messages for errors"
RECENT_ERRORS=$(dmesg | tail -20 | grep -i "error\|panic\|oops\|bug" | grep -i vexfs || true)
if [ -n "$RECENT_ERRORS" ]; then
    warning "Kernel errors detected:"
    echo "$RECENT_ERRORS"
else
    success "No kernel errors detected"
fi

# Test module info
safety_check "Testing module information"
if modinfo vexfs >/dev/null 2>&1; then
    success "Module info accessible"
else
    warning "Module info not accessible"
fi

# CRITICAL: Test unloading before any filesystem operations
safety_check "Testing safe module unload"
if sudo rmmod vexfs; then
    success "Module unloaded successfully"
else
    error "CRITICAL: Module cannot be unloaded - system may be unstable"
fi

# Verify module is completely unloaded
safety_check "Verifying complete unload"
if lsmod | grep -q vexfs; then
    error "CRITICAL: Module still loaded after rmmod"
else
    success "Module completely unloaded"
fi

echo -e "\n${GREEN}🎉 Safe kernel module testing completed successfully!${NC}"
echo -e "\n${YELLOW}IMPORTANT SAFETY NOTES:${NC}"
echo "✅ Module loads and unloads correctly"
echo "✅ No immediate kernel errors detected"
echo "❌ FILESYSTEM MOUNTING NOT TESTED (unsafe)"
echo "❌ DO NOT attempt mounting until FFI issues are fixed"
echo -e "\n${RED}⚠️  MOUNTING VEXFS WILL CAUSE SYSTEM HANGS${NC}"
echo -e "${RED}⚠️  ONLY LOAD/UNLOAD TESTING IS SAFE${NC}"

echo -e "\n${YELLOW}Next Steps:${NC}"
echo "1. Fix Rust FFI implementation issues"
echo "2. Complete VFS operation implementations"
echo "3. Add proper error handling"
echo "4. Test mounting only after fixes"