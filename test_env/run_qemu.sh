#!/bin/bash

# VexFS QEMU Testing Environment (Legacy)
# This script is kept for compatibility but the new vm_control.sh is recommended

echo "⚠️  Legacy script detected!"
echo "This script is deprecated in favor of the new simplified VM setup."
echo
echo "For the new simplified VM environment, use:"
echo "  ./setup_vm.sh      # Initial setup"
echo "  ./vm_control.sh    # VM management"
echo "  ./test_module.sh   # Testing"
echo
echo "Continue with legacy Packer-based setup? (y/N)"
read -r response

if [[ ! "$response" =~ ^[Yy]$ ]]; then
    echo "Aborting. Use the new scripts for a better experience!"
    exit 0
fi

echo "Proceeding with legacy setup..."
echo

# Variables
PACKER_OUTPUT_DIR="./packer_output"
# Find the newest QCOW2 image in the Packer output directory
VM_IMAGE=$(ls -t "$PACKER_OUTPUT_DIR"/*.qcow2 2>/dev/null | head -n1)
EXTRA_DISK_IMAGE="vexfs_disk.img"
EXTRA_DISK_SIZE="100M" # Size for the test VexFS disk

# Check if VM image exists
if [ -z "$VM_IMAGE" ]; then
  echo "Error: No QCOW2 image found in $PACKER_OUTPUT_DIR."
  echo "Please build the VM image using 'packer build vexfs.pkr.hcl' first."
  echo
  echo "Or better yet, use the new simplified setup:"
  echo "  ./setup_vm.sh"
  exit 1
fi

echo "Using VM image: $VM_IMAGE"

# Create the extra disk image if it doesn't exist
if [ ! -f "$EXTRA_DISK_IMAGE" ]; then
  echo "Creating extra disk image: $EXTRA_DISK_IMAGE with size $EXTRA_DISK_SIZE..."
  qemu-img create -f raw "$EXTRA_DISK_IMAGE" "$EXTRA_DISK_SIZE"
  if [ $? -ne 0 ]; then
    echo "Error: Failed to create $EXTRA_DISK_IMAGE."
    exit 1
  fi
  echo "Extra disk image created."
else
  echo "Using existing extra disk image: $EXTRA_DISK_IMAGE"
fi

# QEMU command
# Note: -nographic implies console=ttyS0 is often needed in kernel cmdline,
# but Debian preseed setup usually configures serial console access.
# We forward SSH port 2222 to VM's 22 for easier access.
qemu-system-x86_64 \
  -enable-kvm \
  -m 2G \
  -smp 2 \
  -drive file="$VM_IMAGE",if=virtio,format=qcow2,index=0,media=disk \
  -drive file="$EXTRA_DISK_IMAGE",if=virtio,format=raw,index=1,media=disk \
  -netdev user,id=net0,hostfwd=tcp::2222-:22 \
  -device virtio-net-pci,netdev=net0 \
  -serial stdio \
  -display none # Use this if -serial stdio is preferred over graphical window
  # For a graphical window (if headless=false in Packer and needed for debug):
  # -display gtk
  # If using -display none or -nographic, ensure the guest OS is configured for serial console.
  # The Debian preseed should handle this for ttyS0.

echo "QEMU VM started. Connect via SSH: ssh root@localhost -p 2222 (password: password)"
echo "Or interact via the serial console if -display none is used."
echo "To shut down, use 'sudo halt -p' inside the VM."
