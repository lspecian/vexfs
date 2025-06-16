#!/bin/bash

# VexFS Fixed Module Testing Script for Alpine Linux VM
# Tests the vexfs_deadlock_fix.ko module with proper formatting

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

echo "🧪 VexFS Fixed Module Testing in Alpine VM"
echo "========================================"

# Mount shared directory if needed
if [ ! -d "/mnt/shared" ]; then
    log_info "Mounting shared directory..."
    mkdir -p /mnt/shared
    mount -t 9p -o trans=virtio shared /mnt/shared || log_error "Failed to mount shared directory"
fi

# Check if files exist
if [ ! -f "/mnt/shared/vexfs_deadlock_fix.ko" ]; then
    log_error "Kernel module not found in /mnt/shared/"
    log_info "Available files:"
    ls -la /mnt/shared/
    exit 1
fi

if [ ! -f "/mnt/shared/mkfs.vexfs" ]; then
    log_error "mkfs.vexfs not found in /mnt/shared/"
    exit 1
fi

# Make mkfs.vexfs executable
chmod +x /mnt/shared/mkfs.vexfs

# Check kernel version
log_info "Kernel version: $(uname -r)"

# Check if module is already loaded
if lsmod | grep -q vexfs; then
    log_warning "VexFS module already loaded, removing..."
    rmmod vexfs_deadlock_fix || rmmod vexfs || true
fi

# Load the module
log_info "Loading VexFS kernel module..."
if insmod /mnt/shared/vexfs_deadlock_fix.ko; then
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
    cat /proc/filesystems | grep vexfs
else
    log_error "VexFS not found in /proc/filesystems"
    rmmod vexfs_deadlock_fix
    exit 1
fi

# Create test image
TEST_IMG="/tmp/vexfs_test.img"
MOUNT_DIR="/tmp/vexfs_mount"

log_info "Creating test filesystem image..."
dd if=/dev/zero of="$TEST_IMG" bs=1M count=10 2>/dev/null

# Format the filesystem
log_info "Formatting filesystem with mkfs.vexfs..."
if /mnt/shared/mkfs.vexfs "$TEST_IMG"; then
    log_success "Filesystem formatted successfully!"
else
    log_error "Failed to format filesystem"
    rmmod vexfs_deadlock_fix
    exit 1
fi

# Create mount point
mkdir -p "$MOUNT_DIR"

# Clear dmesg before mount
dmesg -C 2>/dev/null || true

# Try to mount
log_info "Attempting to mount VexFS..."
if mount -t vexfs_fixed -o loop "$TEST_IMG" "$MOUNT_DIR"; then
    log_success "Mount successful!"
    
    # Test basic operations
    log_info "Testing directory listing..."
    ls -la "$MOUNT_DIR/" || log_warning "Directory listing failed"
    
    log_info "Creating test file..."
    if echo "Hello from Alpine VM!" > "$MOUNT_DIR/test.txt"; then
        log_success "File created successfully!"
        
        log_info "Reading test file..."
        if cat "$MOUNT_DIR/test.txt"; then
            log_success "File read successfully!"
        else
            log_error "File read failed"
        fi
    else
        log_error "File creation failed"
    fi
    
    log_info "Testing directory creation..."
    if mkdir -p "$MOUNT_DIR/testdir"; then
        log_success "Directory created successfully!"
        
        log_info "Testing file in subdirectory..."
        if echo "Subdirectory test" > "$MOUNT_DIR/testdir/subfile.txt"; then
            log_success "Subdirectory file created!"
        else
            log_warning "Subdirectory file creation failed"
        fi
    else
        log_warning "Directory creation failed"
    fi
    
    log_info "Final directory listing..."
    ls -laR "$MOUNT_DIR/" || true
    
    # Test persistence
    log_info "Testing persistence - unmounting..."
    if umount "$MOUNT_DIR"; then
        log_success "Unmount successful!"
        
        log_info "Remounting to test persistence..."
        if mount -t vexfs_fixed -o loop "$TEST_IMG" "$MOUNT_DIR"; then
            log_success "Remount successful!"
            
            if [ -f "$MOUNT_DIR/test.txt" ]; then
                log_success "File persisted!"
                cat "$MOUNT_DIR/test.txt"
            else
                log_error "File did not persist"
            fi
            
            umount "$MOUNT_DIR"
        else
            log_error "Remount failed"
            dmesg | tail -20
        fi
    else
        log_error "Unmount failed"
        dmesg | tail -20
    fi
else
    log_error "Mount failed"
    log_info "Checking kernel messages:"
    dmesg | tail -30
fi

# Cleanup
log_info "Cleaning up..."
rm -f "$TEST_IMG"
rmdir "$MOUNT_DIR" 2>/dev/null || true

# Unload module
log_info "Unloading module..."
if rmmod vexfs_deadlock_fix; then
    log_success "Module unloaded successfully!"
else
    log_error "Module unload failed"
    dmesg | tail -10
fi

# Final check
if lsmod | grep -q vexfs; then
    log_warning "VexFS module still loaded:"
    lsmod | grep vexfs
else
    log_success "All VexFS modules unloaded"
fi

log_success "Test completed!"