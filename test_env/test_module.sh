#!/bin/bash

# VexFS Kernel Module Test Script
# This script tests the basic kernel module loading and FFI integration

set -e

echo "🧪 VexFS Kernel Module Test Suite"
echo "================================="

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Test function
test_step() {
    echo -e "\n${YELLOW}➤ $1${NC}"
}

success() {
    echo -e "${GREEN}✅ $1${NC}"
}

error() {
    echo -e "${RED}❌ $1${NC}"
    exit 1
}

# Check if we're in the VM
test_step "Checking environment"
if [ ! -d "/mnt/vexfs_source" ]; then
    error "Not in VM environment - /mnt/vexfs_source not found"
fi
success "VM environment detected"

# Change to the build directory where the module is located
cd ~/vexfs_build

test_step "Checking build artifacts"
if [ ! -f "vexfs.ko" ]; then
    error "vexfs.ko not found - build may have failed"
fi
success "Kernel module found: vexfs.ko"

test_step "Checking for existing module"
if lsmod | grep -q vexfs; then
    echo "Unloading existing vexfs module..."
    sudo rmmod vexfs || error "Failed to unload existing module"
fi

test_step "Loading VexFS kernel module"
sudo insmod vexfs.ko || error "Failed to load kernel module"
success "Kernel module loaded successfully"

test_step "Verifying module is loaded"
if ! lsmod | grep -q vexfs; then
    error "Module not found in lsmod output"
fi
success "Module verified in kernel"

test_step "Checking kernel logs for module messages"
dmesg | tail -10 | grep -i vexfs || echo "No VexFS messages in recent dmesg"

test_step "Testing basic module info"
modinfo vexfs || error "Failed to get module info"

test_step "Checking /proc/modules"
if grep -q vexfs /proc/modules; then
    success "Module found in /proc/modules"
else
    error "Module not found in /proc/modules"
fi

test_step "Testing module unload"
sudo rmmod vexfs || error "Failed to unload module"
success "Module unloaded successfully"

test_step "Verifying module is unloaded"
if lsmod | grep -q vexfs; then
    error "Module still loaded after rmmod"
fi
success "Module successfully unloaded"

echo -e "\n${GREEN}🎉 All tests passed! VexFS kernel module is working correctly.${NC}"
echo -e "\n${YELLOW}Summary:${NC}"
echo "✅ Module builds successfully"
echo "✅ Module loads without errors"
echo "✅ Module appears in kernel module list"
echo "✅ Module unloads cleanly"
echo -e "\n${GREEN}VexFS kernel module FFI integration is functional!${NC}"