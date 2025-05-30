#!/bin/sh

# VexFS Kernel Module Testing Script for Alpine Linux VM
# This script runs inside the Alpine VM to test the VexFS kernel module

set -e

# Colors for output (Alpine sh compatible)
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

log() {
    printf "${BLUE}[$(date '+%H:%M:%S')]${NC} %s\n" "$1"
}

warn() {
    printf "${YELLOW}[WARNING]${NC} %s\n" "$1"
}

error() {
    printf "${RED}[ERROR]${NC} %s\n" "$1"
}

success() {
    printf "${GREEN}[SUCCESS]${NC} %s\n" "$1"
}

echo "========================================"
echo "VexFS Kernel Module Testing in Alpine VM"
echo "========================================"
echo

# Check if running in VM
if grep -q "QEMU" /proc/cpuinfo 2>/dev/null; then
    success "Running in QEMU VM environment"
else
    warn "This doesn't appear to be running in a QEMU VM"
fi

# Check kernel version and Alpine info
log "Kernel version: $(uname -r)"
log "Architecture: $(uname -m)"
log "Alpine version: $(cat /etc/alpine-release 2>/dev/null || echo 'Unknown')"
echo

# Mount shared directory if not already mounted
if [ ! -d /mnt/vexfs_host ]; then
    log "Creating mount point for shared directory..."
    mkdir -p /mnt/vexfs_host
fi

if ! mountpoint -q /mnt/vexfs_host 2>/dev/null; then
    log "Mounting shared directory..."
    if mount -t 9p -o trans=virtio,version=9p2000.L vexfs_host /mnt/vexfs_host; then
        success "Shared directory mounted at /mnt/vexfs_host"
    else
        error "Failed to mount shared directory"
        error "Make sure VM was started with virtfs support"
        exit 1
    fi
else
    success "Shared directory already mounted"
fi

# Check if kernel module is available
MODULE_PATH="/mnt/vexfs_host/kernel/vexfs.ko"
if [ -f "$MODULE_PATH" ]; then
    success "Found VexFS kernel module: $(ls -lh $MODULE_PATH)"
    
    # Copy module to local filesystem for testing
    log "Copying module to local filesystem..."
    cp "$MODULE_PATH" /tmp/vexfs.ko
    chmod 644 /tmp/vexfs.ko
    
    echo
    log "Starting VexFS kernel module tests..."
    echo "======================================"
    
    # Test 1: Module Information
    echo
    log "Test 1: Module Information"
    echo "-------------------------"
    if modinfo /tmp/vexfs.ko; then
        success "Module information retrieved successfully"
    else
        error "Failed to get module information"
        exit 1
    fi
    
    # Test 2: Load Module
    echo
    log "Test 2: Loading Module"
    echo "---------------------"
    
    # Clear dmesg buffer
    dmesg -C
    
    if insmod /tmp/vexfs.ko; then
        success "VexFS module loaded successfully"
        
        # Check if module is loaded
        if lsmod | grep -q vexfs; then
            success "Module appears in lsmod:"
            lsmod | grep vexfs
        else
            warn "Module not found in lsmod output"
        fi
        
        # Check kernel messages
        echo
        log "Kernel messages after loading:"
        dmesg | tail -10
        
    else
        error "Failed to load VexFS module"
        echo
        log "Kernel error messages:"
        dmesg | tail -10
        exit 1
    fi
    
    # Test 3: Module Status Check
    echo
    log "Test 3: Module Status Check"
    echo "---------------------------"
    
    # Check /proc/modules
    if grep -q vexfs /proc/modules; then
        success "Module found in /proc/modules"
        grep vexfs /proc/modules
    else
        warn "Module not found in /proc/modules"
    fi
    
    # Check /sys/module
    if [ -d /sys/module/vexfs ]; then
        success "Module sysfs directory exists: /sys/module/vexfs"
        ls -la /sys/module/vexfs/
    else
        warn "Module sysfs directory not found"
    fi
    
    # Test 4: Unload Module
    echo
    log "Test 4: Unloading Module"
    echo "------------------------"
    
    if rmmod vexfs; then
        success "VexFS module unloaded successfully"
        
        # Verify unload
        if ! lsmod | grep -q vexfs; then
            success "Module successfully removed from lsmod"
        else
            warn "Module may still be loaded"
            lsmod | grep vexfs
        fi
        
        # Check kernel messages
        echo
        log "Kernel messages after unloading:"
        dmesg | tail -5
        
    else
        error "Failed to unload VexFS module"
        echo
        log "Kernel error messages:"
        dmesg | tail -10
    fi
    
    # Test 5: Stress Test (Load/Unload Cycles) - Alpine compatible
    echo
    log "Test 5: Stress Test (5 Load/Unload Cycles)"
    echo "==========================================="
    
    i=1
    while [ $i -le 5 ]; do
        echo
        log "Cycle $i/5..."
        
        # Load
        if insmod /tmp/vexfs.ko; then
            log "  ✓ Loaded"
        else
            error "  ✗ Failed to load on cycle $i"
            break
        fi
        
        sleep 1
        
        # Unload
        if rmmod vexfs; then
            log "  ✓ Unloaded"
        else
            error "  ✗ Failed to unload on cycle $i"
            break
        fi
        
        sleep 1
        i=$((i + 1))
    done
    
    success "Stress test completed"
    
    # Test 6: Final Status Check
    echo
    log "Test 6: Final Status Check"
    echo "--------------------------"
    
    if ! lsmod | grep -q vexfs; then
        success "No VexFS modules loaded (clean state)"
    else
        warn "VexFS module still appears to be loaded:"
        lsmod | grep vexfs
    fi
    
    # Final kernel messages
    echo
    log "Final kernel messages:"
    dmesg | tail -10
    
    echo
    success "All VexFS kernel module tests completed!"
    echo "========================================"
    
else
    error "VexFS kernel module not found at $MODULE_PATH"
    error "Available files in shared directory:"
    ls -la /mnt/vexfs_host/ 2>/dev/null || echo "Shared directory not accessible"
    exit 1
fi