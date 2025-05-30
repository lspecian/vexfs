#!/bin/bash

# Script to run inside the VM to prepare for kernel module testing

echo "Preparing VM for VexFS kernel module testing..."

# Update system
sudo apt update
sudo apt upgrade -y

# Install kernel development tools
sudo apt install -y \
    build-essential \
    linux-headers-$(uname -r) \
    dkms \
    git \
    vim \
    htop \
    stress \
    sysstat

# Create mount point for shared directory
sudo mkdir -p /mnt/vexfs_host
sudo mount -t 9p -o trans=virtio,version=9p2000.L vexfs_host /mnt/vexfs_host

# Add to fstab for automatic mounting
echo "vexfs_host /mnt/vexfs_host 9p trans=virtio,version=9p2000.L 0 0" | sudo tee -a /etc/fstab

# Create symbolic link to kernel module
ln -sf /mnt/vexfs_host/kernel/vexfs.ko ~/vexfs.ko
ln -sf /mnt/vexfs_host/tests/vm_testing/run_comprehensive_kernel_tests.sh ~/run_tests.sh

echo "VM preparation complete!"
echo "Kernel module available at: ~/vexfs.ko"
echo "Test script available at: ~/run_tests.sh"
echo "Shared directory mounted at: /mnt/vexfs_host"
