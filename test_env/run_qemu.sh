#!/bin/bash

# Enhanced QEMU run script for VexFS development

# Get the directory where this script is located
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
VM_DIR="$SCRIPT_DIR/vm"
VM_NAME="vexfs-dev"
VM_IMAGE="$VM_DIR/images/$VM_NAME.qcow2"
CLOUD_INIT_ISO="$VM_DIR/config/cloud-init.iso"

# Check if VM image exists
if [ ! -f "$VM_IMAGE" ]; then
    echo "❌ VM image not found: $VM_IMAGE"
    echo "Run: ./test_env/setup_vm.sh"
    exit 1
fi

# Check if cloud-init ISO exists
if [ ! -f "$CLOUD_INIT_ISO" ]; then
    echo "❌ Cloud-init ISO not found: $CLOUD_INIT_ISO"
    echo "Run: ./test_env/setup_vm.sh"
    exit 1
fi

echo "🚀 Starting VexFS development VM..."
echo "📁 VM Image: $VM_IMAGE"
echo "☁️  Cloud-init: $CLOUD_INIT_ISO"
echo "🌐 SSH: ssh -p 2222 -i $VM_DIR/keys/vexfs_vm_key vexfs@localhost"
echo "🖥️  VNC: localhost:5900 (if needed)"
echo ""

# Start QEMU with optimized settings
exec qemu-system-x86_64 \
  -name "$VM_NAME" \
  -m 2048 \
  -smp 2 \
  -drive file="$VM_IMAGE",format=qcow2,if=virtio \
  -drive file="$CLOUD_INIT_ISO",format=raw,if=virtio,readonly=on \
  -netdev user,id=net0,hostfwd=tcp::2222-:22 \
  -device virtio-net,netdev=net0 \
  -virtfs local,path="$(dirname "$SCRIPT_DIR")",mount_tag=vexfs_source,security_model=passthrough,id=vexfs_source \
  -display none \
  -vnc :0 \
  -enable-kvm \
  -cpu host \
  "$@"
