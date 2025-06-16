#!/bin/bash

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
VM_DIR="$(dirname "$SCRIPT_DIR")"
PROJECT_ROOT="$(dirname "$VM_DIR")"

# VM Configuration
VM_NAME="vexfs-alpine-test"
MEMORY="2048"
CPUS="2"
DISK_IMAGE="$VM_DIR/images/vexfs_alpine_test.qcow2"
ALPINE_ISO="$VM_DIR/images/alpine-virt-3.19.0-x86_64.iso"
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

# Copy kernel module to shared directory
if [ -f "$PROJECT_ROOT/kernel_module/vexfs_deadlock_fix.ko" ]; then
    cp "$PROJECT_ROOT/kernel_module/vexfs_deadlock_fix.ko" "$SHARED_DIR/"
    log_success "Kernel module copied to shared directory"
fi

# Copy test scripts
cp -r "$PROJECT_ROOT/kernel_module/tests" "$SHARED_DIR/" 2>/dev/null || true

# Check if Alpine is installed
if [ ! -f "$VM_DIR/.alpine_installed" ]; then
    log_info "Starting Alpine VM for first boot..."
    log_info "The VM will auto-install Alpine Linux"
    
    # Start VM with ISO for installation
    qemu-system-x86_64 \
        -name "$VM_NAME" \
        -machine type=pc,accel=kvm \
        -cpu host \
        -smp "$CPUS" \
        -m "$MEMORY" \
        -drive file="$DISK_IMAGE",format=qcow2,if=virtio \
        -cdrom "$ALPINE_ISO" \
        -boot order=dc \
        -netdev user,id=net0,hostfwd=tcp::2222-:22 \
        -device virtio-net-pci,netdev=net0 \
        -virtfs local,path="$SHARED_DIR",mount_tag=shared,security_model=passthrough,id=shared \
        -nographic \
        -serial mon:stdio
        
    # After installation completes, mark as installed
    echo "Alpine installation should be complete. Creating marker file..."
    touch "$VM_DIR/.alpine_installed"
else
    log_info "Starting Alpine VM..."
    
    # Start VM normally (headless)
    qemu-system-x86_64 \
        -name "$VM_NAME" \
        -machine type=pc,accel=kvm \
        -cpu host \
        -smp "$CPUS" \
        -m "$MEMORY" \
        -drive file="$DISK_IMAGE",format=qcow2,if=virtio \
        -netdev user,id=net0,hostfwd=tcp::2222-:22 \
        -device virtio-net-pci,netdev=net0 \
        -virtfs local,path="$SHARED_DIR",mount_tag=shared,security_model=passthrough,id=shared \
        -nographic \
        -serial mon:stdio \
        -daemonize \
        -pidfile "$VM_DIR/qemu.pid"
    
    log_success "Alpine VM started in background"
    log_info "SSH access: ssh -p 2222 root@localhost (password: vexfs)"
    log_info "Or: ssh -p 2222 vexfs@localhost (password: vexfs)"
    log_info "Shared directory: /mnt/shared in VM"
    log_info "To stop VM: kill $(cat $VM_DIR/qemu.pid)"
fi
