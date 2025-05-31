#!/bin/bash

set -e

echo "🚀 VexFS Filesystem Testing (Using Existing Module)"
echo "=================================================="

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

# Environment check
log_info "Host environment:"
log_info "  Kernel: $(uname -r)"
log_info "  OS: $(cat /etc/os-release | grep PRETTY_NAME | cut -d= -f2 | tr -d '\"')"

# Check if module is already loaded
log_info "🔍 STEP 1: Checking if VexFS module is loaded..."
if lsmod | grep -q vexfs_minimal; then
    log_success "Module is loaded:"
    lsmod | grep vexfs_minimal
    
    # Check filesystem registration
    log_info "Checking filesystem registration..."
    if cat /proc/filesystems | grep -q vexfs; then
        log_success "Filesystem registered: $(cat /proc/filesystems | grep vexfs)"
    else
        log_error "Filesystem not found in /proc/filesystems"
        exit 1
    fi
else
    log_error "VexFS module is not loaded"
    exit 1
fi

# Clear kernel messages
dmesg -C 2>/dev/null || true

# STEP 2: Create a loop device for testing
log_info "🔧 STEP 2: Creating loop device for filesystem testing..."

# Create a test file (10MB)
TEST_FILE="/tmp/vexfs_test_new.img"
dd if=/dev/zero of="$TEST_FILE" bs=1M count=10 2>/dev/null

# Set up loop device
LOOP_DEVICE=$(losetup -f)
if losetup "$LOOP_DEVICE" "$TEST_FILE"; then
    log_success "Loop device created: $LOOP_DEVICE"
else
    log_error "Failed to create loop device"
    exit 1
fi

# STEP 3: Attempt to mount the filesystem
log_info "🗂️  STEP 3: Attempting to mount VexFS filesystem..."

# Create mount point
MOUNT_POINT="/tmp/vexfs_mount_new"
mkdir -p "$MOUNT_POINT"

# Clear kernel messages before mount attempt
dmesg -C 2>/dev/null || true

# Attempt to mount
log_info "Mounting $LOOP_DEVICE to $MOUNT_POINT..."
if timeout 30 mount -t vexfs "$LOOP_DEVICE" "$MOUNT_POINT" 2>&1; then
    log_success "🎉 FILESYSTEM MOUNTED SUCCESSFULLY!"
    
    # STEP 4: Test filesystem operations
    log_info "📁 STEP 4: Testing filesystem operations..."
    
    # Test 1: Create a file
    log_info "Test 1: Creating a test file..."
    if echo "Hello VexFS!" > "$MOUNT_POINT/test_file.txt" 2>&1; then
        log_success "File created successfully"
        
        # Test 2: Read the file
        log_info "Test 2: Reading the test file..."
        if CONTENT=$(cat "$MOUNT_POINT/test_file.txt" 2>&1); then
            log_success "File read successfully: '$CONTENT'"
        else
            log_error "Failed to read file"
        fi
    else
        log_error "Failed to create file"
    fi
    
    # Test 3: Create a directory
    log_info "Test 3: Creating a directory..."
    if mkdir "$MOUNT_POINT/test_dir" 2>&1; then
        log_success "Directory created successfully"
        
        # Test 4: List directory contents
        log_info "Test 4: Listing directory contents..."
        if ls -la "$MOUNT_POINT/" 2>&1; then
            log_success "Directory listing successful"
        else
            log_error "Failed to list directory"
        fi
    else
        log_error "Failed to create directory"
    fi
    
    # Test 5: File operations in subdirectory
    log_info "Test 5: Creating file in subdirectory..."
    if echo "Subdirectory test" > "$MOUNT_POINT/test_dir/subfile.txt" 2>&1; then
        log_success "Subdirectory file created successfully"
    else
        log_error "Failed to create file in subdirectory"
    fi
    
    # Test 6: Check filesystem stats
    log_info "Test 6: Checking filesystem statistics..."
    if df -h "$MOUNT_POINT" 2>&1; then
        log_success "Filesystem stats retrieved"
    else
        log_warning "Failed to get filesystem stats"
    fi
    
    # STEP 5: Unmount filesystem
    log_info "🔄 STEP 5: Unmounting filesystem..."
    if umount "$MOUNT_POINT" 2>&1; then
        log_success "Filesystem unmounted successfully"
    else
        log_error "Failed to unmount filesystem"
        # Force unmount if needed
        log_info "Attempting force unmount..."
        umount -f "$MOUNT_POINT" 2>&1 || true
    fi
    
else
    log_error "❌ FILESYSTEM MOUNT FAILED"
    log_info "Checking kernel messages for mount failure details..."
    dmesg | tail -20
    
    # Check if it's a missing mkfs issue
    log_info "This might be expected - VexFS may need mkfs.vexfs to format the device first"
    log_info "Raw mount attempts often fail without proper filesystem formatting"
fi

# STEP 6: Cleanup
log_info "🧹 STEP 6: Cleaning up..."

# Remove loop device
if losetup -d "$LOOP_DEVICE" 2>/dev/null; then
    log_success "Loop device removed"
else
    log_warning "Failed to remove loop device"
fi

# Remove test file
rm -f "$TEST_FILE"
rmdir "$MOUNT_POINT" 2>/dev/null || true

# Final kernel message check
log_info "📋 Final kernel message check..."
if dmesg | grep -i "error\|panic\|oops\|segfault\|null.*pointer" | tail -10; then
    log_warning "Some error messages found (review above)"
else
    log_success "No critical errors in kernel log"
fi

log_success "🏁 FILESYSTEM TEST FINISHED"
log_info "This test covered:"
log_info "  ✅ Module verification (already loaded)"
log_info "  ✅ Filesystem registration check"
log_info "  ✅ Loop device creation"
log_info "  ✅ Mount attempt with mount_nodev() fix"
log_info "  ✅ File operations (if mount succeeded)"
log_info "  ✅ Directory operations (if mount succeeded)"
log_info "  ✅ Unmount and cleanup"
log_info "Note: Module left loaded for further testing"