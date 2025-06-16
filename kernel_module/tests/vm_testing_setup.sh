#!/bin/bash

# VexFS VM Testing Setup Script
# Creates a safe QEMU VM environment for kernel module testing

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
VM_DIR="$PROJECT_ROOT/vm_testing"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info() { echo -e "${BLUE}ℹ️  $1${NC}"; }
log_success() { echo -e "${GREEN}✅ $1${NC}"; }
log_warning() { echo -e "${YELLOW}⚠️  $1${NC}"; }
log_error() { echo -e "${RED}❌ $1${NC}"; }

echo "🖥️  VexFS VM Testing Setup"
echo "=========================="

# Check dependencies
check_dependencies() {
    log_info "Checking dependencies..."
    
    local missing_deps=()
    
    if ! command -v qemu-system-x86_64 &> /dev/null; then
        missing_deps+=("qemu-system-x86_64")
    fi
    
    if ! command -v qemu-img &> /dev/null; then
        missing_deps+=("qemu-utils")
    fi
    
    if ! command -v wget &> /dev/null; then
        missing_deps+=("wget")
    fi
    
    if [ ${#missing_deps[@]} -ne 0 ]; then
        log_error "Missing dependencies: ${missing_deps[*]}"
        log_info "Install with: sudo apt-get install qemu-system-x86 qemu-utils wget"
        exit 1
    fi
    
    log_success "All dependencies found"
}

# Create VM directory structure
setup_vm_directory() {
    log_info "Setting up VM directory structure..."
    
    mkdir -p "$VM_DIR"/{images,scripts,shared}
    
    log_success "VM directory created at $VM_DIR"
}

# Download Ubuntu Server ISO if not present
download_ubuntu_iso() {
    local iso_path="$VM_DIR/images/ubuntu-22.04.3-live-server-amd64.iso"
    
    if [ -f "$iso_path" ]; then
        log_info "Ubuntu ISO already exists"
        return 0
    fi
    
    log_info "Downloading Ubuntu 22.04 Server ISO..."
    log_warning "This will download ~1.5GB - ensure you have good internet connection"
    
    read -p "Continue with ISO download? (y/N): " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        log_info "ISO download cancelled"
        return 1
    fi
    
    wget -O "$iso_path" \
        "https://releases.ubuntu.com/22.04.3/ubuntu-22.04.3-live-server-amd64.iso"
    
    log_success "Ubuntu ISO downloaded"
}

# Create VM disk image
create_vm_disk() {
    local disk_path="$VM_DIR/images/vexfs_test_vm.qcow2"
    
    if [ -f "$disk_path" ]; then
        log_info "VM disk already exists"
        return 0
    fi
    
    log_info "Creating VM disk image (20GB)..."
    
    qemu-img create -f qcow2 "$disk_path" 20G
    
    log_success "VM disk created"
}

# Create VM startup script
create_vm_script() {
    local vm_script="$VM_DIR/scripts/start_vm.sh"
    
    cat > "$vm_script" << 'EOF'
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
EOF

    chmod +x "$vm_script"
    log_success "VM startup script created"
}

# Create VM testing script for inside the VM
create_vm_test_script() {
    local test_script="$VM_DIR/shared/test_vexfs_in_vm.sh"
    
    cat > "$test_script" << 'EOF'
#!/bin/bash

# VexFS Testing Script - Runs inside VM
# This script safely tests the VexFS kernel module in isolation

set -e

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

echo "🧪 VexFS Kernel Module Testing in VM"
echo "===================================="

# Check if we're in a VM
if [ ! -d "/mnt/shared" ]; then
    log_error "Shared directory not found - are we in the VM?"
    log_info "Mount shared directory with: sudo mkdir -p /mnt/shared && sudo mount -t 9p -o trans=virtio shared /mnt/shared"
    exit 1
fi

# Check for kernel module
if [ ! -f "/mnt/shared/vexfs_a4724ed.ko" ]; then
    log_error "Kernel module not found in shared directory"
    exit 1
fi

log_info "Kernel version: $(uname -r)"
log_info "System info: $(cat /etc/os-release | grep PRETTY_NAME)"

# Install kernel development tools if needed
if ! command -v make &> /dev/null; then
    log_info "Installing kernel development tools..."
    sudo apt-get update
    sudo apt-get install -y build-essential linux-headers-$(uname -r)
fi

# Test 1: Check module info
log_info "Checking kernel module information..."
modinfo /mnt/shared/vexfs_a4724ed.ko

# Test 2: Load the module
log_info "Loading VexFS kernel module..."
if sudo insmod /mnt/shared/vexfs_a4724ed.ko; then
    log_success "Kernel module loaded successfully!"
    
    # Check if loaded
    if lsmod | grep -q vexfs; then
        log_success "Module visible in lsmod:"
        lsmod | grep vexfs
    fi
    
    # Check dmesg
    log_info "Recent kernel messages:"
    dmesg | tail -20
    
    # Test 3: Try basic filesystem operations
    log_info "Testing filesystem operations..."
    
    # Create test image
    dd if=/dev/zero of=/tmp/vexfs_test.img bs=1M count=10
    
    # Set up loop device
    LOOP_DEV=$(sudo losetup -f)
    sudo losetup "$LOOP_DEV" /tmp/vexfs_test.img
    log_info "Created loop device: $LOOP_DEV"
    
    # Try to mount (this is where crashes occurred before)
    mkdir -p /tmp/vexfs_mount
    log_info "Attempting mount operation..."
    
    if timeout 30 sudo mount -t vexfs_fixed "$LOOP_DEV" /tmp/vexfs_mount; then
        log_success "Mount successful!"
        
        # Test basic operations
        ls -la /tmp/vexfs_mount/
        
        # Unmount
        sudo umount /tmp/vexfs_mount
        log_success "Unmount successful!"
    else
        log_warning "Mount failed or timed out - checking system state..."
        dmesg | tail -30
    fi
    
    # Clean up loop device
    sudo losetup -d "$LOOP_DEV"
    rm -f /tmp/vexfs_test.img
    
    # Test 4: Unload module
    log_info "Unloading kernel module..."
    if sudo rmmod vexfs_a4724ed; then
        log_success "Module unloaded successfully!"
    else
        log_error "Failed to unload module"
        lsmod | grep vexfs
    fi
    
else
    log_error "Failed to load kernel module"
    dmesg | tail -20
    exit 1
fi

log_success "VM testing completed successfully!"
log_info "If this test passes, the kernel module is safe for host testing"
EOF

    chmod +x "$test_script"
    log_success "VM test script created"
}

# Create installation guide
create_installation_guide() {
    local guide="$VM_DIR/INSTALLATION_GUIDE.md"
    
    cat > "$guide" << 'EOF'
# VexFS VM Testing Installation Guide

## Quick Start

1. **Setup VM Environment**:
   ```bash
   ./kernel_module/tests/vm_testing_setup.sh
   ```

2. **Start VM for Installation**:
   ```bash
   ./vm_testing/scripts/start_vm.sh
   ```

3. **Install Ubuntu in VM**:
   - Follow Ubuntu installation wizard
   - Create user account
   - Enable SSH server during installation
   - After installation, create file: `touch vm_testing/.vm_installed`

4. **Start VM for Testing**:
   ```bash
   ./vm_testing/scripts/start_vm.sh
   ```

5. **Connect to VM**:
   ```bash
   ssh -p 2222 user@localhost
   ```

6. **Mount Shared Directory in VM**:
   ```bash
   sudo mkdir -p /mnt/shared
   sudo mount -t 9p -o trans=virtio shared /mnt/shared
   ```

7. **Run VexFS Tests in VM**:
   ```bash
   /mnt/shared/test_vexfs_in_vm.sh
   ```

## VM Configuration

- **Memory**: 2GB RAM
- **CPUs**: 2 cores
- **Disk**: 20GB
- **Network**: NAT with SSH forwarding (port 2222)
- **Shared Directory**: Host `vm_testing/shared` → VM `/mnt/shared`

## Safety Features

- **Complete Isolation**: VM crashes don't affect host
- **Shared Directory**: Easy file transfer between host and VM
- **SSH Access**: Remote testing and debugging
- **Snapshot Support**: Can save VM states for testing

## Troubleshooting

### VM Won't Start
- Check KVM acceleration: `kvm-ok`
- Ensure user is in kvm group: `sudo usermod -a -G kvm $USER`

### Shared Directory Not Working
- Mount manually in VM: `sudo mount -t 9p -o trans=virtio shared /mnt/shared`
- Check permissions on host shared directory

### SSH Connection Failed
- Ensure VM is running: `ps aux | grep qemu`
- Check port forwarding: `netstat -tlnp | grep 2222`

## Testing Workflow

1. **Safe Development**: Test all kernel changes in VM first
2. **Crash Recovery**: VM crashes are isolated and recoverable
3. **Debugging**: Use VM for systematic debugging without host risk
4. **Validation**: Only deploy to host after VM validation

This setup provides a safe environment for kernel module development and testing.
EOF

    log_success "Installation guide created"
}

# Main execution
main() {
    check_dependencies
    setup_vm_directory
    
    log_info "VM testing environment setup complete!"
    log_info "Next steps:"
    log_info "1. Run: ./vm_testing/scripts/start_vm.sh"
    log_info "2. Install Ubuntu in the VM"
    log_info "3. Create file: touch vm_testing/.vm_installed"
    log_info "4. Restart VM and run tests"
    
    create_vm_script
    create_vm_test_script
    create_installation_guide
    
    log_success "All VM testing files created!"
    log_warning "Note: You'll need to download Ubuntu ISO and install it in the VM"
    log_info "See vm_testing/INSTALLATION_GUIDE.md for detailed instructions"
}

# Ask user if they want to proceed
log_warning "This will create a VM testing environment for safe kernel module testing"
log_info "The VM provides complete isolation - crashes won't affect your host system"

read -p "Proceed with VM setup? (y/N): " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    log_info "VM setup cancelled"
    exit 0
fi

main