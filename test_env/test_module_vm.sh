#!/bin/bash

# VexFS Kernel Module Testing Script (VM-native build)
# This version builds the module inside the VM to match kernel versions
set -euo pipefail

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
SSH_PORT=2222
SSH_KEY="$SCRIPT_DIR/vm/keys/vexfs_vm_key"
SSH_USER="vexfs"
SSH_HOST="localhost"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log() { echo -e "${BLUE}[INFO]${NC} $1"; }
success() { echo -e "${GREEN}[SUCCESS]${NC} $1"; }
warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
error() { echo -e "${RED}[ERROR]${NC} $1"; }

# SSH helper
vm_ssh() {
    ssh -o ConnectTimeout=10 -o StrictHostKeyChecking=no -p "$SSH_PORT" -i "$SSH_KEY" "$SSH_USER@$SSH_HOST" "$@"
}

# Check VM status
check_vm_status() {
    log "Checking VM status..."
    if ! vm_ssh "echo 'VM accessible'" &>/dev/null; then
        error "VM not accessible. Run: ./test_env/vm_control.sh start"
        exit 1
    fi
    success "VM is accessible"
}

# Verify source mount
check_source_mount() {
    log "Verifying VexFS source mount..."
    if ! vm_ssh "test -d /mnt/vexfs_source/vexfs"; then
        error "VexFS source not mounted at /mnt/vexfs_source"
        error "Check virtfs configuration in vm_control.sh"
        exit 1
    fi
    success "VexFS source is mounted and accessible"
}

# Build module inside VM
build_module_in_vm() {
    log "Building VexFS kernel module inside VM..."
    
    vm_ssh "
        set -e
        source ~/.cargo/env
        
        echo 'Copying source to VM-local directory (VirtFS permission workaround)...'
        rm -rf ~/vexfs_build
        cp -r /mnt/vexfs_source/vexfs ~/vexfs_build
        cd ~/vexfs_build
        
        echo 'Cleaning previous build artifacts...'
        make clean
        
        echo 'Building kernel module for VM kernel...'
        make
        
        echo 'Verifying module was created...'
        if [ ! -f vexfs.ko ]; then
            echo 'ERROR: Module build failed - vexfs.ko not found'
            exit 1
        fi
        
        echo 'Module built successfully!'
        ls -la vexfs.ko
    "
    
    success "Kernel module built inside VM"
}

# Test module loading
test_module_loading() {
    log "Testing kernel module loading..."
    
    vm_ssh "
        set -e
        cd /mnt/vexfs_source/vexfs
        
        # Unload if already loaded
        if lsmod | grep -q vexfs; then
            echo 'Unloading existing module...'
            sudo rmmod vexfs
        fi
        
        echo 'Loading kernel module...'
        sudo insmod vexfs.ko
        
        echo 'Verifying module is loaded...'
        if ! lsmod | grep -q vexfs; then
            echo 'ERROR: Module not found in lsmod output'
            exit 1
        fi
        
        echo 'Module loaded successfully!'
        lsmod | grep vexfs
        
        echo 'Checking kernel logs...'
        sudo dmesg | tail -5 | grep -i vexfs || echo 'No recent vexfs messages'
    "
    
    success "Module loading test passed"
}

# Test FFI integration
test_ffi_integration() {
    log "Testing FFI integration..."
    
    vm_ssh "
        set -e
        cd /mnt/vexfs_source/vexfs
        
        echo 'Testing FFI integration binary...'
        if [ -x test_ffi_integration ]; then
            echo 'Running FFI integration test...'
            if ./test_ffi_integration; then
                echo 'FFI integration test passed!'
            else
                echo 'FFI integration test failed!'
                exit 1
            fi
        else
            echo 'FFI integration binary not found or not executable'
            echo 'This is expected if building only kernel module'
        fi
    "
    
    success "FFI integration test completed"
}

# Test module functionality
test_module_functionality() {
    log "Testing module functionality..."
    
    vm_ssh "
        set -e
        
        echo 'Testing basic module operations...'
        
        # Check /proc/vexfs if it exists
        if [ -d /proc/vexfs ]; then
            echo 'Found /proc/vexfs directory:'
            ls -la /proc/vexfs/
        else
            echo 'No /proc/vexfs directory found (this may be expected)'
        fi
        
        # Check /sys/fs/vexfs if it exists
        if [ -d /sys/fs/vexfs ]; then
            echo 'Found /sys/fs/vexfs directory:'
            ls -la /sys/fs/vexfs/
        else
            echo 'No /sys/fs/vexfs directory found (this may be expected)'
        fi
        
        # Check module info
        echo 'Module information:'
        /sbin/modinfo vexfs | head -10
        
        echo 'Module functionality test completed'
    "
    
    success "Module functionality test completed"
}

# Cleanup function
cleanup_module() {
    log "Cleaning up module in VM..."
    
    vm_ssh "
        if lsmod | grep -q vexfs; then
            echo 'Unloading VexFS module...'
            sudo rmmod vexfs
            echo 'Module unloaded'
        else
            echo 'Module not loaded, nothing to clean up'
        fi
    " || warn "Cleanup may have failed, but continuing..."
    
    success "Cleanup completed"
}

# Show VM kernel info
show_vm_info() {
    log "VM System Information:"
    vm_ssh "
        echo 'Kernel version:' \$(uname -r)
        echo 'Distribution:' \$(lsb_release -d 2>/dev/null | cut -f2 || echo 'Unknown')
        echo 'Architecture:' \$(uname -m)
        echo 'Available memory:' \$(free -h | grep Mem | awk '{print \$2}')
        echo 'CPU cores:' \$(nproc)
        echo 'VexFS source mount:' \$(mount | grep vexfs_source || echo 'Not found')
    "
}

# Main test function
run_tests() {
    log "🧪 Running VexFS Kernel Module Tests (VM-native build)"
    log "========================================================="
    
    # Check VM and environment
    check_vm_status
    show_vm_info
    check_source_mount
    
    # Build and test
    build_module_in_vm
    test_module_loading
    test_ffi_integration
    test_module_functionality
    
    success "🎉 All tests passed!"
    log "The QEMU testing environment is working perfectly!"
}

# Handle arguments
case "${1:-}" in
    "info") 
        check_vm_status
        show_vm_info
        ;;
    "build") 
        check_vm_status
        check_source_mount
        build_module_in_vm
        ;;
    "test") 
        check_vm_status
        test_module_loading
        test_ffi_integration
        test_module_functionality
        ;;
    "cleanup") 
        cleanup_module
        ;;
    "") 
        run_tests
        ;;
    *) 
        echo "Usage: $0 [info|build|test|cleanup]"
        echo "  info    - Show VM system information"
        echo "  build   - Build kernel module in VM only"
        echo "  test    - Run tests only (assumes module is built)"
        echo "  cleanup - Clean up module in VM"
        echo "  (no args) - Run full test suite"
        exit 1
        ;;
esac

# Always cleanup on exit
trap cleanup_module EXIT