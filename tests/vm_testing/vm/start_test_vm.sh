#!/bin/bash

# VexFS Testing VM Startup Script

VM_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$VM_DIR/../.." && pwd)"
LOGS_DIR="$VM_DIR/logs"

# VM Configuration
VM_NAME="vexfs-test-vm"
VM_MEMORY="2G"
VM_CORES="2"
VM_DISK_SIZE="20G"

# Create timestamp for this session
TIMESTAMP=$(date '+%Y%m%d_%H%M%S')
LOG_FILE="$LOGS_DIR/vm_session_${TIMESTAMP}.log"

echo "Starting VexFS Testing VM..."
echo "Logs will be written to: $LOG_FILE"

# Check if VM image exists
if [[ ! -f "$VM_DIR/ubuntu-test.qcow2" ]]; then
    echo "VM disk image not found. Creating new VM..."
    
    # Download Ubuntu Server ISO if not present
    if [[ ! -f "$VM_DIR/ubuntu-22.04-server.iso" ]]; then
        echo "Downloading Ubuntu Server 22.04 ISO..."
        wget -O "$VM_DIR/ubuntu-22.04-server.iso" \
            "https://releases.ubuntu.com/22.04/ubuntu-22.04.3-live-server-amd64.iso"
    fi
    
    # Create VM disk
    qemu-img create -f qcow2 "$VM_DIR/ubuntu-test.qcow2" $VM_DISK_SIZE
    
    echo "Starting VM installation..."
    echo "Please complete the Ubuntu installation manually"
    qemu-system-x86_64 \
        -name "$VM_NAME" \
        -m $VM_MEMORY \
        -smp $VM_CORES \
        -hda "$VM_DIR/ubuntu-test.qcow2" \
        -cdrom "$VM_DIR/ubuntu-22.04-server.iso" \
        -boot d \
        -enable-kvm \
        -netdev user,id=net0,hostfwd=tcp::2222-:22 \
        -device e1000,netdev=net0 \
        -vnc :1 \
        2>&1 | tee "$LOG_FILE"
else
    echo "Starting existing VM..."
    
    # Start VM with shared directory for kernel module
    qemu-system-x86_64 \
        -name "$VM_NAME" \
        -m $VM_MEMORY \
        -smp $VM_CORES \
        -hda "$VM_DIR/ubuntu-test.qcow2" \
        -enable-kvm \
        -netdev user,id=net0,hostfwd=tcp::2222-:22 \
        -device e1000,netdev=net0 \
        -virtfs local,path="$PROJECT_ROOT",mount_tag=vexfs_host,security_model=passthrough,id=vexfs_share \
        -vnc :1 \
        -nographic \
        2>&1 | tee "$LOG_FILE"
fi
