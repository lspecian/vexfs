#!/bin/bash

# Local test script for VexFS module
# WARNING: This runs on the host system - use with caution!

set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info() { echo -e "${BLUE}ℹ️  $1${NC}"; }
log_success() { echo -e "${GREEN}✅ $1${NC}"; }
log_warning() { echo -e "${YELLOW}⚠️  $1${NC}"; }
log_error() { echo -e "${RED}❌ $1${NC}"; }

echo "🧪 VexFS Module Local Testing"
echo "============================="

# Check if running as root
if [ "$EUID" -ne 0 ]; then 
    log_error "Please run as root (sudo)"
    exit 1
fi

# Check if module is already loaded
if lsmod | grep -q vexfs; then
    log_warning "VexFS module already loaded, attempting to remove..."
    rmmod vexfs_deadlock_fix || rmmod vexfs || true
fi

MODULE_PATH="kernel_module/vexfs_deadlock_fix.ko"

if [ ! -f "$MODULE_PATH" ]; then
    log_error "Module not found at $MODULE_PATH"
    exit 1
fi

# Load module
log_info "Loading VexFS module..."
if insmod "$MODULE_PATH"; then
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
if tools/mkfs.vexfs "$TEST_IMG"; then
    log_success "Filesystem formatted successfully!"
else
    log_error "Failed to format filesystem"
    rmmod vexfs_deadlock_fix
    exit 1
fi

# Create mount point
mkdir -p "$MOUNT_DIR"

# Try to mount
log_info "Attempting to mount VexFS..."
if mount -t vexfs_fixed -o loop "$TEST_IMG" "$MOUNT_DIR"; then
    log_success "Mount successful!"
    
    # Test basic operations
    log_info "Testing file creation..."
    echo "Hello VexFS!" > "$MOUNT_DIR/test.txt"
    
    log_info "Testing file read..."
    cat "$MOUNT_DIR/test.txt"
    
    log_info "Testing directory creation..."
    mkdir -p "$MOUNT_DIR/testdir"
    
    log_info "Testing directory listing..."
    ls -la "$MOUNT_DIR/"
    
    # Test persistence
    log_info "Testing persistence - unmounting..."
    umount "$MOUNT_DIR"
    
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
    fi
else
    log_error "Mount failed"
    dmesg | tail -30
fi

# Cleanup
log_info "Cleaning up..."
rmmod vexfs_deadlock_fix
rm -f "$TEST_IMG"
rmdir "$MOUNT_DIR" 2>/dev/null || true

log_success "Test completed!"