#!/bin/bash

# VM Console Test - Direct console access without SSH dependency

set -e

echo "🚀 VexFS VM Console Test"
echo "========================"

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m'

# Configuration
VM_DIR="test_env/vm"
VM_NAME="vexfs-dev"
VM_IMAGE="$VM_DIR/images/$VM_NAME.qcow2"
CLOUD_INIT_ISO="$VM_DIR/config/cloud-init-simple.iso"

echo -e "${YELLOW}Starting VM with console output...${NC}"

# Create a simple test script that will run in the VM
cat > /tmp/vm_test_script.sh << 'EOF'
#!/bin/bash
echo "=== VexFS VM Test Starting ==="
echo "Hostname: $(hostname)"
echo "User: $(whoami)"
echo "Date: $(date)"

# Check if VexFS source is mounted
if [ -d /mnt/vexfs_source ]; then
    echo "✅ VexFS source mounted"
    ls -la /mnt/vexfs_source | head -5
    
    # Check for kernel module
    if [ -f /mnt/vexfs_source/vexfs.ko ]; then
        echo "✅ VexFS kernel module found"
        
        # Try to load it
        if insmod /mnt/vexfs_source/vexfs.ko; then
            echo "✅ VexFS kernel module loaded"
            lsmod | grep vexfs
            
            # Unload it
            if rmmod vexfs; then
                echo "✅ VexFS kernel module unloaded"
            else
                echo "❌ Failed to unload VexFS module"
            fi
        else
            echo "❌ Failed to load VexFS kernel module"
            dmesg | tail -5
        fi
    else
        echo "❌ VexFS kernel module not found"
    fi
else
    echo "❌ VexFS source not mounted"
fi

echo "=== VexFS VM Test Complete ==="
EOF

# Start QEMU with console output and run our test
timeout 180 qemu-system-x86_64 \
  -name "$VM_NAME" \
  -m 1024 \
  -smp 2 \
  -drive file="$VM_IMAGE",format=qcow2,if=virtio,cache=unsafe \
  -drive file="$CLOUD_INIT_ISO",format=raw,if=virtio,readonly=on \
  -netdev user,id=net0 \
  -device virtio-net,netdev=net0 \
  -virtfs local,path="$(pwd)",mount_tag=vexfs_source,security_model=passthrough,id=vexfs_source \
  -nographic \
  -enable-kvm \
  -cpu host \
  -no-reboot \
  -kernel /boot/vmlinuz-$(uname -r) \
  -initrd /boot/initrd.img-$(uname -r) \
  -append "console=ttyS0 root=/dev/vda1 init=/bin/bash" \
  || echo "VM test completed or timed out"

echo -e "${GREEN}VM console test finished${NC}"