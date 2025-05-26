#!/bin/bash

# VexFS VM Setup Script
# Automates the complete setup of a lightweight QEMU testing environment

set -e

echo "🚀 VexFS VM Testing Environment Setup"
echo "====================================="

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
VM_DIR="test_env/vm"
VM_NAME="vexfs-dev"
VM_IMAGE="$VM_DIR/images/$VM_NAME.qcow2"
CLOUD_INIT_ISO="$VM_DIR/config/cloud-init.iso"
VM_KEY="$VM_DIR/keys/vexfs_vm_key"
UBUNTU_VERSION="22.04"
CLOUD_IMAGE_URL="https://cloud-images.ubuntu.com/releases/$UBUNTU_VERSION/release/ubuntu-$UBUNTU_VERSION-server-cloudimg-amd64.img"

# Helper functions
log_step() {
    echo -e "\n${BLUE}➤ $1${NC}"
}

log_success() {
    echo -e "${GREEN}✅ $1${NC}"
}

log_warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
}

log_error() {
    echo -e "${RED}❌ $1${NC}"
    exit 1
}

# Check dependencies
check_dependencies() {
    log_step "Checking dependencies"
    
    local missing_deps=()
    
    if ! command -v qemu-system-x86_64 >/dev/null 2>&1; then
        missing_deps+=("qemu-system-x86_64")
    fi
    
    if ! command -v qemu-img >/dev/null 2>&1; then
        missing_deps+=("qemu-img")
    fi
    
    if ! command -v cloud-localds >/dev/null 2>&1; then
        missing_deps+=("cloud-image-utils")
    fi
    
    if ! command -v ssh-keygen >/dev/null 2>&1; then
        missing_deps+=("openssh-client")
    fi
    
    if [ ${#missing_deps[@]} -ne 0 ]; then
        log_error "Missing dependencies: ${missing_deps[*]}"
        echo "Install with: sudo apt update && sudo apt install -y qemu-system-x86 qemu-utils cloud-image-utils openssh-client"
        exit 1
    fi
    
    log_success "All dependencies found"
}

# Create directory structure
setup_directories() {
    log_step "Setting up directory structure"
    
    mkdir -p "$VM_DIR"/{images,config,keys}
    
    log_success "Directory structure created"
}

# Generate SSH key pair
generate_ssh_key() {
    log_step "Generating SSH key pair"
    
    if [ -f "$VM_KEY" ]; then
        log_warning "SSH key already exists, skipping generation"
        return
    fi
    
    ssh-keygen -t rsa -b 4096 -f "$VM_KEY" -N "" -C "vexfs-vm-key"
    chmod 600 "$VM_KEY"
    chmod 644 "$VM_KEY.pub"
    
    log_success "SSH key pair generated"
}

# Download Ubuntu cloud image
download_cloud_image() {
    log_step "Downloading Ubuntu $UBUNTU_VERSION cloud image"
    
    local temp_image="/tmp/ubuntu-$UBUNTU_VERSION-cloudimg.img"
    
    if [ -f "$VM_IMAGE" ]; then
        log_warning "VM image already exists, skipping download"
        return
    fi
    
    if [ ! -f "$temp_image" ]; then
        echo "Downloading from: $CLOUD_IMAGE_URL"
        curl -L -o "$temp_image" "$CLOUD_IMAGE_URL" || log_error "Failed to download cloud image"
    fi
    
    # Convert to qcow2 and resize
    qemu-img convert -f qcow2 -O qcow2 "$temp_image" "$VM_IMAGE"
    qemu-img resize "$VM_IMAGE" 20G
    
    log_success "Cloud image prepared and resized to 20GB"
}

# Create cloud-init configuration
create_cloud_init() {
    log_step "Creating cloud-init configuration"
    
    local user_data="$VM_DIR/config/user-data"
    local meta_data="$VM_DIR/config/meta-data"
    
    # Create user-data
    cat > "$user_data" << 'EOF'
#cloud-config
users:
  - name: vexfs
    sudo: ALL=(ALL) NOPASSWD:ALL
    shell: /bin/bash
    ssh_authorized_keys:
      - SSH_PUBLIC_KEY_PLACEHOLDER

package_update: true
package_upgrade: true

packages:
  - build-essential
  - linux-headers-generic
  - curl
  - git
  - vim
  - htop

runcmd:
  # Install Rust
  - curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sudo -u vexfs sh -s -- -y
  - sudo -u vexfs /home/vexfs/.cargo/bin/rustup target add x86_64-unknown-linux-gnu
  
  # Mount VexFS source
  - mkdir -p /mnt/vexfs_source
  - echo "vexfs_source /mnt/vexfs_source 9p trans=virtio,version=9p2000.L,posix,rw 0 0" >> /etc/fstab
  - mount -a
  
  # Create build directory symlink
  - sudo -u vexfs ln -sf /mnt/vexfs_source /home/vexfs/vexfs_build
  
  # Set up environment
  - echo 'source ~/.cargo/env' >> /home/vexfs/.bashrc
  - echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> /home/vexfs/.bashrc

final_message: "VexFS development VM is ready! Connect via SSH on port 2222"
EOF

    # Insert the actual SSH public key
    if [ -f "$VM_KEY.pub" ]; then
        local ssh_pub_key=$(cat "$VM_KEY.pub")
        sed -i "s|SSH_PUBLIC_KEY_PLACEHOLDER|$ssh_pub_key|" "$user_data"
    else
        log_error "SSH public key not found at $VM_KEY.pub"
    fi
    
    # Create meta-data
    cat > "$meta_data" << EOF
instance-id: vexfs-dev-001
local-hostname: vexfs-dev
EOF
    
    # Generate cloud-init ISO
    cloud-localds "$CLOUD_INIT_ISO" "$user_data" "$meta_data"
    
    log_success "Cloud-init configuration created"
}

# Create run script
create_run_script() {
    log_step "Creating VM run script"
    
    cat > "test_env/run_qemu.sh" << 'EOF'
#!/bin/bash

# Enhanced QEMU run script for VexFS development

VM_DIR="test_env/vm"
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
echo "🌐 SSH: ssh -p 2222 -i test_env/vm/keys/vexfs_vm_key vexfs@localhost"
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
  -virtfs local,path="$(pwd)",mount_tag=vexfs_source,security_model=passthrough,id=vexfs_source \
  -display none \
  -vnc :0 \
  -enable-kvm \
  -cpu host \
  "$@"
EOF
    
    chmod +x "test_env/run_qemu.sh"
    
    log_success "VM run script created"
}

# Create helper scripts
create_helper_scripts() {
    log_step "Creating helper scripts"
    
    # SSH helper
    cat > "test_env/ssh_vm.sh" << EOF
#!/bin/bash
ssh -o ConnectTimeout=10 -o StrictHostKeyChecking=no -p 2222 -i test_env/vm/keys/vexfs_vm_key vexfs@localhost "\$@"
EOF
    chmod +x "test_env/ssh_vm.sh"
    
    # Build helper
    cat > "test_env/build_in_vm.sh" << 'EOF'
#!/bin/bash
echo "🔨 Building VexFS in VM..."
./test_env/ssh_vm.sh "cd ~/vexfs_build/vexfs && source ~/.cargo/env && make clean && make vm-build"
EOF
    chmod +x "test_env/build_in_vm.sh"
    
    # Test helper
    cat > "test_env/test_in_vm.sh" << 'EOF'
#!/bin/bash
echo "🧪 Testing VexFS kernel module in VM..."
./test_env/ssh_vm.sh "/mnt/vexfs_source/test_env/test_module.sh"
EOF
    chmod +x "test_env/test_in_vm.sh"
    
    log_success "Helper scripts created"
}

# Update documentation
update_documentation() {
    log_step "Updating documentation"
    
    cat > "test_env/QUICK_START.md" << 'EOF'
# VexFS VM Testing - Quick Start

## Overview
This setup provides a lightweight, fast-iteration testing environment for VexFS kernel development.

## Quick Commands

```bash
# 1. Setup VM (one-time)
./test_env/setup_vm.sh

# 2. Start VM
./test_env/run_qemu.sh

# 3. SSH into VM (in another terminal)
./test_env/ssh_vm.sh

# 4. Build in VM
./test_env/build_in_vm.sh

# 5. Test kernel module
./test_env/test_in_vm.sh
```

## Key Features

- **Fast Boot**: VM boots in ~30 seconds (vs 10-20 min Packer builds)
- **Live Source**: VexFS source mounted via virtfs - changes are instant
- **No Rebuilds**: Edit code on host, build in VM immediately
- **Automated Setup**: Dependencies installed automatically via cloud-init
- **Kernel Ready**: Includes kernel headers and build tools

## VM Details

- **OS**: Ubuntu 22.04 Server (cloud image)
- **Memory**: 2GB RAM
- **CPUs**: 2 cores
- **Disk**: 20GB (dynamic)
- **User**: vexfs (passwordless sudo)
- **SSH Port**: 2222 (host) → 22 (guest)
- **VNC Port**: 5900 (if display needed)

## Development Workflow

1. **Edit** VexFS source code on host (any editor)
2. **Build** in VM: `./test_env/build_in_vm.sh`
3. **Test** kernel module: `./test_env/test_in_vm.sh`
4. **Debug** via SSH: `./test_env/ssh_vm.sh`

## Troubleshooting

- **VM won't start**: Check `./test_env/setup_vm.sh` was run
- **SSH fails**: Wait 30-60s for VM to fully boot
- **Build fails**: Ensure Rust environment: `source ~/.cargo/env`
- **Module load fails**: Check dmesg for kernel errors

## Architecture Benefits

- No complex Packer dependencies
- No static VM images to rebuild
- Fast edit-test-debug cycles
- Real kernel environment validation
- Minimal resource usage
EOF
    
    log_success "Documentation updated"
}

# Main setup process
main() {
    echo "Starting VexFS VM setup..."
    
    check_dependencies
    setup_directories
    generate_ssh_key
    download_cloud_image
    create_cloud_init
    create_run_script
    create_helper_scripts
    update_documentation
    
    echo ""
    log_success "VexFS VM testing environment setup complete!"
    echo ""
    echo -e "${YELLOW}Next steps:${NC}"
    echo "1. Start VM: ./test_env/run_qemu.sh"
    echo "2. Wait 30-60s for boot, then SSH: ./test_env/ssh_vm.sh"
    echo "3. Build: ./test_env/build_in_vm.sh"
    echo "4. Test: ./test_env/test_in_vm.sh"
    echo ""
    echo -e "${BLUE}Quick reference: cat test_env/QUICK_START.md${NC}"
}

# Run main function
main "$@"