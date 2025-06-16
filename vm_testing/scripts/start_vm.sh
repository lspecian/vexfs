#!/bin/bash

# VexFS Testing VM Startup Script

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
VM_DIR="$(dirname "$SCRIPT_DIR")"
PROJECT_ROOT="$(dirname "$VM_DIR")"

# VM Configuration
VM_NAME="vexfs-test"
MEMORY="2048"  # 2GB RAM
CPUS="2"
DISK_IMAGE="$VM_DIR/images/vexfs_test_vm.qcow2"
ISO_IMAGE="$VM_DIR/images/ubuntu-22.04.3-live-server-amd64.iso"
SHARED_DIR="$VM_DIR/shared"

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

# Check if this is first boot (needs installation)
if [ ! -f "$VM_DIR/.vm_installed" ]; then
    log_info "Starting VM for first-time installation..."
    log_warning "You'll need to install Ubuntu manually in the VM"
    log_info "After installation, create the file $VM_DIR/.vm_installed"
    
    # Start VM with ISO for installation
    qemu-system-x86_64 \
        -name "$VM_NAME" \
        -machine type=pc,accel=kvm \
        -cpu host \
        -smp "$CPUS" \
        -m "$MEMORY" \
        -drive file="$DISK_IMAGE",format=qcow2,if=virtio \
        -cdrom "$ISO_IMAGE" \
        -boot order=dc \
        -netdev user,id=net0,hostfwd=tcp::2222-:22 \
        -device virtio-net-pci,netdev=net0 \
        -virtfs local,path="$SHARED_DIR",mount_tag=shared,security_model=passthrough \
        -display gtk \
        -vga virtio
else
    log_info "Starting installed VM..."
    
    # Copy kernel module to shared directory
    if [ -f "$PROJECT_ROOT/kernel_module/vexfs_a4724ed.ko" ]; then
        cp "$PROJECT_ROOT/kernel_module/vexfs_a4724ed.ko" "$SHARED_DIR/"
        log_success "Kernel module copied to shared directory"
    fi
    
    # Copy test scripts
    cp -r "$PROJECT_ROOT/kernel_module/tests" "$SHARED_DIR/"
    
    # Start VM normally
    qemu-system-x86_64 \
        -name "$VM_NAME" \
        -machine type=pc,accel=kvm \
        -cpu host \
        -smp "$CPUS" \
        -m "$MEMORY" \
        -drive file="$DISK_IMAGE",format=qcow2,if=virtio \
        -netdev user,id=net0,hostfwd=tcp::2222-:22 \
        -device virtio-net-pci,netdev=net0 \
        -virtfs local,path="$SHARED_DIR",mount_tag=shared,security_model=passthrough \
        -display gtk \
        -vga virtio \
        -daemonize
    
    log_success "VM started in background"
    log_info "SSH access: ssh -p 2222 user@localhost"
    log_info "Shared directory mounted at /mnt/shared in VM"
fi
