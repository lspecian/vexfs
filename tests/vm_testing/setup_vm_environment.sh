#!/bin/bash

# VexFS VM Testing Environment Setup Script
# This script sets up a QEMU VM environment for kernel module testing

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
VM_DIR="$SCRIPT_DIR/vm"
LOGS_DIR="$VM_DIR/logs"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

log() {
    echo -e "${BLUE}[$(date '+%Y-%m-%d %H:%M:%S')]${NC} $1"
}

warn() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

# Check if we're in the right directory
if [[ ! -f "$PROJECT_ROOT/kernel/vexfs.ko" ]]; then
    error "VexFS kernel module not found at $PROJECT_ROOT/kernel/vexfs.ko"
    error "Please build the kernel module first with 'make' in the project root"
    exit 1
fi

# Create VM directory structure
log "Setting up VM directory structure..."
mkdir -p "$VM_DIR"
mkdir -p "$LOGS_DIR"

# Check for existing VM setup
if [[ -d "$VM_DIR/ubuntu-vm" ]]; then
    log "Existing VM setup found at $VM_DIR/ubuntu-vm"
else
    log "No existing VM setup found. You may need to create a VM first."
    warn "Consider using the existing VM setup scripts or creating a new Ubuntu VM"
fi

# Create VM startup script
log "Creating VM startup script..."
cat > "$VM_DIR/start_test_vm.sh" << 'EOF'
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
EOF

chmod +x "$VM_DIR/start_test_vm.sh"

# Create VM connection helper script
log "Creating VM connection helper..."
cat > "$VM_DIR/connect_vm.sh" << 'EOF'
#!/bin/bash

# Connect to the testing VM via SSH
echo "Connecting to VexFS Testing VM..."
echo "Default connection: ssh -p 2222 user@localhost"
echo "Make sure the VM is running and SSH is enabled"

# Try to connect
ssh -p 2222 -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null user@localhost
EOF

chmod +x "$VM_DIR/connect_vm.sh"

# Create VM testing preparation script
log "Creating VM testing preparation script..."
cat > "$VM_DIR/prepare_vm_for_testing.sh" << 'EOF'
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
EOF

chmod +x "$VM_DIR/prepare_vm_for_testing.sh"

# Create monitoring script
log "Creating monitoring script..."
cat > "$VM_DIR/monitor_vm_testing.sh" << 'EOF'
#!/bin/bash

# Multi-terminal monitoring script for VM testing
# This script helps set up the 3-terminal monitoring system

echo "VexFS VM Testing Monitor Setup"
echo "=============================="
echo ""
echo "This script will help you set up the 3-terminal monitoring system:"
echo "1. Terminal 1: VM Console monitoring"
echo "2. Terminal 2: dmesg monitoring in VM"
echo "3. Terminal 3: Resource monitoring in VM"
echo ""
echo "Instructions:"
echo "1. Start the VM with: ./start_test_vm.sh"
echo "2. Connect to VM in Terminal 2: ./connect_vm.sh"
echo "3. Connect to VM in Terminal 3: ./connect_vm.sh"
echo "4. In Terminal 2, run: sudo dmesg -w"
echo "5. In Terminal 3, run: watch -n 1 'free -h && echo && ps aux | head -10'"
echo "6. In Terminal 1, monitor VM console output"
echo "7. Execute tests with: ~/run_tests.sh"
echo ""
echo "Log files will be created in: $(dirname "$0")/logs/"
EOF

chmod +x "$VM_DIR/monitor_vm_testing.sh"

# Create quick test script
log "Creating quick test verification script..."
cat > "$VM_DIR/quick_test.sh" << 'EOF'
#!/bin/bash

# Quick verification that the kernel module can be loaded
# Run this inside the VM

echo "Quick VexFS Kernel Module Test"
echo "=============================="

if [[ ! -f ~/vexfs.ko ]]; then
    echo "ERROR: Kernel module not found at ~/vexfs.ko"
    echo "Make sure the shared directory is mounted and the module is built"
    exit 1
fi

echo "Kernel module found: $(ls -lh ~/vexfs.ko)"
echo ""

echo "Checking module info..."
modinfo ~/vexfs.ko

echo ""
echo "Attempting to load module..."
sudo insmod ~/vexfs.ko

if lsmod | grep -q vexfs; then
    echo "SUCCESS: VexFS module loaded successfully!"
    echo "Module details:"
    lsmod | grep vexfs
    
    echo ""
    echo "Checking dmesg for module messages..."
    dmesg | tail -10
    
    echo ""
    echo "Unloading module..."
    sudo rmmod vexfs
    
    if ! lsmod | grep -q vexfs; then
        echo "SUCCESS: Module unloaded successfully!"
    else
        echo "WARNING: Module may still be loaded"
    fi
else
    echo "ERROR: Failed to load VexFS module"
    echo "Check dmesg for error messages:"
    dmesg | tail -10
    exit 1
fi

echo ""
echo "Quick test completed successfully!"
EOF

chmod +x "$VM_DIR/quick_test.sh"

# Summary
success "VM testing environment setup complete!"
echo ""
log "Created files:"
echo "  - $VM_DIR/start_test_vm.sh          (Start the testing VM)"
echo "  - $VM_DIR/connect_vm.sh             (Connect to VM via SSH)"
echo "  - $VM_DIR/prepare_vm_for_testing.sh (Prepare VM environment)"
echo "  - $VM_DIR/monitor_vm_testing.sh     (Monitoring setup guide)"
echo "  - $VM_DIR/quick_test.sh             (Quick module test)"
echo ""
log "Next steps:"
echo "1. Start VM: cd $VM_DIR && ./start_test_vm.sh"
echo "2. Prepare VM: Run prepare_vm_for_testing.sh inside the VM"
echo "3. Run tests: Execute ~/run_tests.sh inside the VM"
echo "4. Monitor: Follow instructions in monitor_vm_testing.sh"
echo ""
warn "Note: You may need to create a VM first if one doesn't exist"
warn "The comprehensive test script is ready at: tests/vm_testing/run_comprehensive_kernel_tests.sh"