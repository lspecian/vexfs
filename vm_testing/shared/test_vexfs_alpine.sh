#!/bin/bash

# VexFS Testing Script for Alpine Linux VM

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

echo "🧪 VexFS Kernel Module Testing in Alpine VM"
echo "=========================================="

# Check kernel version
log_info "Kernel version: $(uname -r)"

# Check if module exists
if [ ! -f "/mnt/shared/vexfs_deadlock_fix.ko" ]; then
    log_error "Kernel module not found in /mnt/shared/"
    log_info "Make sure the module is copied to shared directory"
    exit 1
fi

# Load the module
log_info "Loading VexFS kernel module..."
if sudo insmod /mnt/shared/vexfs_deadlock_fix.ko; then
    log_success "Module loaded successfully!"
    lsmod | grep vexfs
else
    log_error "Failed to load module"
    dmesg | tail -20
    exit 1
fi

# Check filesystem registration
if grep -q vexfs /proc/filesystems; then
    log_success "VexFS filesystem registered"
else
    log_error "VexFS not found in /proc/filesystems"
    exit 1
fi

# Create test image
log_info "Creating test filesystem..."
dd if=/dev/zero of=/tmp/vexfs_test.img bs=1M count=10 2>/dev/null

# Create mount point
mkdir -p /tmp/vexfs_mount

# Try to mount
log_info "Attempting to mount VexFS..."
if sudo mount -t vexfs_fixed -o loop /tmp/vexfs_test.img /tmp/vexfs_mount; then
    log_success "Mount successful!"
    
    # Test basic operations
    log_info "Testing directory listing..."
    ls -la /tmp/vexfs_mount/
    
    log_info "Creating test file..."
    echo "Hello from Alpine VM!" | sudo tee /tmp/vexfs_mount/test.txt
    
    log_info "Reading test file..."
    cat /tmp/vexfs_mount/test.txt
    
    # Unmount
    sudo umount /tmp/vexfs_mount
    log_success "Unmount successful!"
else
    log_error "Mount failed"
    dmesg | tail -30
fi

# Unload module
log_info "Unloading module..."
sudo rmmod vexfs_deadlock_fix

log_success "Test complete!"
