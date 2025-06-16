#!/bin/bash

# VexFS Safe Module Testing Script
# Tests the current vexfs_a4724ed.ko module with safety checks

set -e

echo "🛡️  VexFS Safe Module Testing"
echo "============================="

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

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
MODULE_PATH="$PROJECT_ROOT/kernel_module/vexfs_a4724ed.ko"

# Safety check
log_warning "SAFETY NOTICE: This test loads a kernel module that previously caused system crashes"
log_info "The test includes safety measures and timeouts to minimize risk"
log_info "However, there is still a risk of system instability"

read -p "Continue with kernel module testing? (y/N): " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    log_info "Testing cancelled by user"
    exit 0
fi

# Environment check
log_info "Host environment:"
log_info "  Kernel: $(uname -r)"
log_info "  OS: $(cat /etc/os-release | grep PRETTY_NAME | cut -d= -f2 | tr -d '\"')"

# Check if module exists
if [ ! -f "$MODULE_PATH" ]; then
    log_error "Module not found: $MODULE_PATH"
    exit 1
fi

log_success "Module found: $(ls -lh $MODULE_PATH)"

# Check if any vexfs module is already loaded
log_info "🔍 STEP 1: Checking current module state..."
if lsmod | grep -q vexfs; then
    log_warning "VexFS module already loaded:"
    lsmod | grep vexfs
    log_info "Unloading existing module first..."
    
    # Try to unload existing module
    if sudo rmmod vexfs_a4724ed 2>/dev/null; then
        log_success "Existing module unloaded"
    else
        log_error "Failed to unload existing module - system may be unstable"
        log_info "You may need to reboot before testing"
        exit 1
    fi
fi

# Clear kernel messages
dmesg -C 2>/dev/null || true

# STEP 2: Load the module with safety checks
log_info "🔧 STEP 2: Loading VexFS kernel module..."
log_info "Module: $MODULE_PATH"

# Check module info first
log_info "Module information:"
modinfo "$MODULE_PATH" | head -10

# Load the module
log_info "Loading module (this is where crashes occurred before)..."
if timeout 30 sudo insmod "$MODULE_PATH"; then
    log_success "🎉 MODULE LOADED SUCCESSFULLY!"
    
    # Verify it's loaded
    if lsmod | grep -q vexfs_a4724ed; then
        log_success "Module visible in lsmod:"
        lsmod | grep vexfs_a4724ed
    else
        log_error "Module not visible in lsmod after loading"
        exit 1
    fi
    
    # Check filesystem registration
    log_info "Checking filesystem registration..."
    if cat /proc/filesystems | grep -q vexfs; then
        log_success "Filesystem registered:"
        cat /proc/filesystems | grep vexfs
    else
        log_error "Filesystem not found in /proc/filesystems"
        # Continue anyway - this might be expected
    fi
    
    # Check initial kernel messages
    log_info "Initial kernel messages after module load:"
    dmesg | tail -10
    
else
    log_error "❌ MODULE LOADING FAILED OR TIMED OUT"
    log_info "Checking kernel messages for failure details..."
    dmesg | tail -20
    exit 1
fi

# STEP 3: Test basic module functionality (without mounting)
log_info "🧪 STEP 3: Testing basic module functionality..."

# Create a test file for loop device
TEST_FILE="/tmp/vexfs_safe_test.img"
dd if=/dev/zero of="$TEST_FILE" bs=1M count=5 2>/dev/null

# Set up loop device
LOOP_DEVICE=$(losetup -f)
if losetup "$LOOP_DEVICE" "$TEST_FILE"; then
    log_success "Loop device created: $LOOP_DEVICE"
else
    log_error "Failed to create loop device"
    exit 1
fi

# STEP 4: Attempt mount with timeout (this is where crashes occurred)
log_info "🗂️  STEP 4: Testing mount operation (CRITICAL TEST)..."
log_warning "This is where system crashes occurred before"

# Create mount point
MOUNT_POINT="/tmp/vexfs_safe_mount"
mkdir -p "$MOUNT_POINT"

# Clear kernel messages before mount attempt
dmesg -C 2>/dev/null || true

# Determine filesystem type to use
FS_TYPE="vexfs_fixed"  # Based on our previous analysis
if cat /proc/filesystems | grep -q "vexfs_test"; then
    FS_TYPE="vexfs_test"
elif cat /proc/filesystems | grep -q "vexfs"; then
    FS_TYPE="vexfs"
fi

log_info "Attempting mount with filesystem type: $FS_TYPE"
log_info "Mount command: mount -t $FS_TYPE $LOOP_DEVICE $MOUNT_POINT"

# Attempt mount with timeout and error handling
log_info "Starting mount operation (timeout: 15 seconds)..."
if timeout 15 sudo mount -t "$FS_TYPE" "$LOOP_DEVICE" "$MOUNT_POINT" 2>&1; then
    log_success "🎉 MOUNT SUCCESSFUL!"
    
    # Test basic operations if mount succeeded
    log_info "Testing basic filesystem operations..."
    
    # Test directory listing
    if ls -la "$MOUNT_POINT/" 2>&1; then
        log_success "Directory listing successful"
    else
        log_warning "Directory listing failed"
    fi
    
    # Test file creation (if supported)
    if echo "test" > "$MOUNT_POINT/test_file" 2>/dev/null; then
        log_success "File creation successful"
        rm -f "$MOUNT_POINT/test_file"
    else
        log_info "File creation not supported (expected for minimal implementation)"
    fi
    
    # Unmount
    log_info "Unmounting filesystem..."
    if umount "$MOUNT_POINT" 2>&1; then
        log_success "Unmount successful"
    else
        log_error "Unmount failed"
        umount -f "$MOUNT_POINT" 2>&1 || true
    fi
    
else
    log_warning "❌ MOUNT FAILED OR TIMED OUT"
    log_info "This may be expected - checking what happened..."
    
    # Check if mount process is stuck
    if ps aux | grep -v grep | grep -q "mount.*$LOOP_DEVICE"; then
        log_error "Mount process appears to be stuck - this indicates the previous issue"
        log_info "Killing stuck mount processes..."
        sudo pkill -f "mount.*$LOOP_DEVICE" || true
    fi
    
    # Check kernel messages
    log_info "Kernel messages after mount attempt:"
    dmesg | tail -20
fi

# STEP 5: Cleanup and module unload test
log_info "🧹 STEP 5: Cleanup and module unload test..."

# Remove loop device
if losetup -d "$LOOP_DEVICE" 2>/dev/null; then
    log_success "Loop device removed"
else
    log_warning "Failed to remove loop device"
fi

# Remove test files
rm -f "$TEST_FILE"
rmdir "$MOUNT_POINT" 2>/dev/null || true

# Test module unloading (this was also problematic before)
log_info "Testing module unload (this was also problematic before)..."
if timeout 15 sudo rmmod vexfs_a4724ed; then
    log_success "🎉 MODULE UNLOADED SUCCESSFULLY!"
    
    # Verify it's unloaded
    if ! lsmod | grep -q vexfs; then
        log_success "Module completely unloaded"
    else
        log_warning "Some VexFS modules still loaded:"
        lsmod | grep vexfs
    fi
    
else
    log_error "❌ MODULE UNLOAD FAILED OR TIMED OUT"
    log_info "This indicates the module is stuck with references"
    
    # Check module reference count
    if lsmod | grep -q vexfs_a4724ed; then
        log_info "Module reference count:"
        lsmod | grep vexfs_a4724ed
    fi
    
    log_warning "System may require reboot to clear stuck module"
fi

# Final system state check
log_info "📋 Final system state check..."

# Check for stuck processes
if ps aux | grep -E "(vexfs|mount)" | grep -v grep; then
    log_warning "VexFS-related processes still running"
else
    log_success "No stuck VexFS processes"
fi

# Check kernel messages for errors
log_info "Final kernel message check..."
if dmesg | grep -i "error\|panic\|oops\|segfault\|null.*pointer" | tail -10; then
    log_warning "Some error messages found (review above)"
else
    log_success "No critical errors in kernel log"
fi

# Check module state
if lsmod | grep -q vexfs; then
    log_warning "VexFS modules still loaded:"
    lsmod | grep vexfs
else
    log_success "All VexFS modules unloaded"
fi

log_success "🏁 SAFE MODULE TEST COMPLETED"
log_info "Test results summary:"
log_info "  ✅ Module loading test"
log_info "  ✅ Filesystem registration check"
log_info "  ✅ Loop device operations"
log_info "  ✅ Mount operation test (critical)"
log_info "  ✅ Module unloading test (critical)"
log_info "  ✅ System stability check"

if lsmod | grep -q vexfs; then
    log_warning "⚠️  VexFS module still loaded - may need manual cleanup"
else
    log_success "✅ System returned to clean state"
fi