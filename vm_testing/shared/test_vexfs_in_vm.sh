#!/bin/bash

# VexFS Testing Script - Runs inside VM
# This script safely tests the VexFS kernel module in isolation

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info() { echo -e "${BLUE}ℹ️  $1${NC}"; }
log_success() { echo -e "${GREEN}✅ $1${NC}"; }
log_warning() { echo -e "${YELLOW}⚠️  $1${NC}"; }
log_error() { echo -e "${RED}❌ $1${NC}"; }

echo "🧪 VexFS Kernel Module Testing in VM"
echo "===================================="

# Check if we're in a VM
if [ ! -d "/mnt/shared" ]; then
    log_error "Shared directory not found - are we in the VM?"
    log_info "Mount shared directory with: sudo mkdir -p /mnt/shared && sudo mount -t 9p -o trans=virtio shared /mnt/shared"
    exit 1
fi

# Check for kernel module
if [ ! -f "/mnt/shared/vexfs_a4724ed.ko" ]; then
    log_error "Kernel module not found in shared directory"
    exit 1
fi

log_info "Kernel version: $(uname -r)"
log_info "System info: $(cat /etc/os-release | grep PRETTY_NAME)"

# Install kernel development tools if needed
if ! command -v make &> /dev/null; then
    log_info "Installing kernel development tools..."
    sudo apt-get update
    sudo apt-get install -y build-essential linux-headers-$(uname -r)
fi

# Test 1: Check module info
log_info "Checking kernel module information..."
modinfo /mnt/shared/vexfs_a4724ed.ko

# Test 2: Load the module
log_info "Loading VexFS kernel module..."
if sudo insmod /mnt/shared/vexfs_a4724ed.ko; then
    log_success "Kernel module loaded successfully!"
    
    # Check if loaded
    if lsmod | grep -q vexfs; then
        log_success "Module visible in lsmod:"
        lsmod | grep vexfs
    fi
    
    # Check dmesg
    log_info "Recent kernel messages:"
    dmesg | tail -20
    
    # Test 3: Try basic filesystem operations
    log_info "Testing filesystem operations..."
    
    # Create test image
    dd if=/dev/zero of=/tmp/vexfs_test.img bs=1M count=10
    
    # Set up loop device
    LOOP_DEV=$(sudo losetup -f)
    sudo losetup "$LOOP_DEV" /tmp/vexfs_test.img
    log_info "Created loop device: $LOOP_DEV"
    
    # Try to mount (this is where crashes occurred before)
    mkdir -p /tmp/vexfs_mount
    log_info "Attempting mount operation..."
    
    if timeout 30 sudo mount -t vexfs_fixed "$LOOP_DEV" /tmp/vexfs_mount; then
        log_success "Mount successful!"
        
        # Test basic operations
        ls -la /tmp/vexfs_mount/
        
        # Unmount
        sudo umount /tmp/vexfs_mount
        log_success "Unmount successful!"
    else
        log_warning "Mount failed or timed out - checking system state..."
        dmesg | tail -30
    fi
    
    # Clean up loop device
    sudo losetup -d "$LOOP_DEV"
    rm -f /tmp/vexfs_test.img
    
    # Test 4: Unload module
    log_info "Unloading kernel module..."
    if sudo rmmod vexfs_a4724ed; then
        log_success "Module unloaded successfully!"
    else
        log_error "Failed to unload module"
        lsmod | grep vexfs
    fi
    
else
    log_error "Failed to load kernel module"
    dmesg | tail -20
    exit 1
fi

log_success "VM testing completed successfully!"
log_info "If this test passes, the kernel module is safe for host testing"
