#!/bin/bash

# VexFS Alpine Linux VM Auto-Setup Script
# Uses Alpine Linux for lightweight, automated VM setup

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
VM_DIR="$(dirname "$SCRIPT_DIR")"
PROJECT_ROOT="$(dirname "$VM_DIR")"

# VM Configuration
VM_NAME="vexfs-alpine-test"
MEMORY="2048"  # 2GB RAM
CPUS="2"
DISK_IMAGE="$VM_DIR/images/vexfs_alpine_test.qcow2"
ALPINE_ISO_URL="https://dl-cdn.alpinelinux.org/alpine/v3.19/releases/x86_64/alpine-virt-3.19.0-x86_64.iso"
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

# Create directories if needed
mkdir -p "$VM_DIR/images"
mkdir -p "$SHARED_DIR"

# Download Alpine ISO if not present
if [ ! -f "$ALPINE_ISO" ]; then
    log_info "Downloading Alpine Linux ISO (only ~50MB)..."
    wget -O "$ALPINE_ISO" "$ALPINE_ISO_URL" || {
        log_error "Failed to download Alpine ISO"
        exit 1
    }
    log_success "Alpine ISO downloaded"
else
    log_info "Alpine ISO already exists"
fi

# Create VM disk if not present
if [ ! -f "$DISK_IMAGE" ]; then
    log_info "Creating VM disk image (8GB)..."
    qemu-img create -f qcow2 "$DISK_IMAGE" 8G
    log_success "VM disk created"
else
    log_info "VM disk already exists"
fi

# Create automated setup script for Alpine
cat > "$SHARED_DIR/setup_alpine_auto.sh" << 'ALPINE_SETUP'
#!/bin/sh
# Alpine Linux automated setup script

echo "Starting Alpine Linux automated setup..."

# Answer setup-alpine questions automatically
cat > /tmp/answerfile << EOF
KEYMAPOPTS="us us"
HOSTNAMEOPTS="-n vexfs-test"
INTERFACESOPTS="auto lo
iface lo inet loopback

auto eth0
iface eth0 inet dhcp
"
DNSOPTS="-n 8.8.8.8"
TIMEZONEOPTS="-z UTC"
PROXYOPTS="none"
APKREPOSOPTS="-1"
SSHDOPTS="-c openssh"
NTPOPTS="-c openntpd"
DISKOPTS="-m sys /dev/vda"
EOF

# Set root password to 'vexfs'
echo -e "vexfs\nvexfs" | passwd root

# Run setup with answerfile
setup-alpine -f /tmp/answerfile

# Install additional packages
apk add --no-cache \
    build-base \
    linux-headers \
    linux-virt-dev \
    bash \
    sudo \
    util-linux \
    e2fsprogs \
    git

# Create vexfs user
adduser -D -s /bin/bash vexfs
echo -e "vexfs\nvexfs" | passwd vexfs
echo "vexfs ALL=(ALL) NOPASSWD: ALL" >> /etc/sudoers

# Enable SSH root login (for testing only)
sed -i 's/^#PermitRootLogin.*/PermitRootLogin yes/' /etc/ssh/sshd_config
rc-service sshd restart

# Setup 9p mount for shared directory
mkdir -p /mnt/shared
echo "shared /mnt/shared 9p trans=virtio,version=9p2000.L,rw,_netdev 0 0" >> /etc/fstab

echo "Alpine setup complete! Rebooting..."
reboot
ALPINE_SETUP

chmod +x "$SHARED_DIR/setup_alpine_auto.sh"

# Create VM startup script for Alpine
cat > "$VM_DIR/scripts/start_alpine_vm.sh" << 'EOF'
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
EOF

chmod +x "$VM_DIR/scripts/start_alpine_vm.sh"

# Create simplified test script for Alpine
cat > "$SHARED_DIR/test_vexfs_alpine.sh" << 'EOF'
#!/bin/bash

# VexFS Testing Script for Alpine Linux VM

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

echo "🧪 VexFS Kernel Module Testing in Alpine VM"
echo "=========================================="

# Check kernel version
log_info "Kernel version: $(uname -r)"

# Check if module exists
if [ ! -f "/mnt/shared/vexfs_deadlock_fix.ko" ]; then
    log_error "Kernel module not found in /mnt/shared/"
    log_info "Make sure the module is copied to shared directory"
    exit 1
fi

# Load the module
log_info "Loading VexFS kernel module..."
if sudo insmod /mnt/shared/vexfs_deadlock_fix.ko; then
    log_success "Module loaded successfully!"
    lsmod | grep vexfs
else
    log_error "Failed to load module"
    dmesg | tail -20
    exit 1
fi

# Check filesystem registration
if grep -q vexfs /proc/filesystems; then
    log_success "VexFS filesystem registered"
else
    log_error "VexFS not found in /proc/filesystems"
    exit 1
fi

# Create test image
log_info "Creating test filesystem..."
dd if=/dev/zero of=/tmp/vexfs_test.img bs=1M count=10 2>/dev/null

# Create mount point
mkdir -p /tmp/vexfs_mount

# Try to mount
log_info "Attempting to mount VexFS..."
if sudo mount -t vexfs_fixed -o loop /tmp/vexfs_test.img /tmp/vexfs_mount; then
    log_success "Mount successful!"
    
    # Test basic operations
    log_info "Testing directory listing..."
    ls -la /tmp/vexfs_mount/
    
    log_info "Creating test file..."
    echo "Hello from Alpine VM!" | sudo tee /tmp/vexfs_mount/test.txt
    
    log_info "Reading test file..."
    cat /tmp/vexfs_mount/test.txt
    
    # Unmount
    sudo umount /tmp/vexfs_mount
    log_success "Unmount successful!"
else
    log_error "Mount failed"
    dmesg | tail -30
fi

# Unload module
log_info "Unloading module..."
sudo rmmod vexfs_deadlock_fix

log_success "Test complete!"
EOF

chmod +x "$SHARED_DIR/test_vexfs_alpine.sh"

log_success "Alpine VM setup complete!"
echo
log_info "Next steps:"
log_info "1. Start the Alpine VM:"
log_info "   $VM_DIR/scripts/start_alpine_vm.sh"
log_info ""
log_info "2. On first boot, Alpine will show a login prompt"
log_info "   Login as: root (no password initially)"
log_info "   Run: /mnt/shared/setup_alpine_auto.sh"
log_info ""
log_info "3. After reboot, connect via SSH:"
log_info "   ssh -p 2222 root@localhost (password: vexfs)"
log_info ""
log_info "4. Run VexFS tests in VM:"
log_info "   /mnt/shared/test_vexfs_alpine.sh"
log_info ""
log_info "Alpine is much lighter than Ubuntu (50MB vs 1.5GB) and boots in seconds!"