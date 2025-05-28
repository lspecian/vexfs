#!/bin/bash

# VexFS QEMU-based Automated Image Builder
# Creates deployable VexFS images using the existing QEMU infrastructure

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
VM_DIR="$SCRIPT_DIR/vm"
BUILD_DIR="$SCRIPT_DIR/build"
OUTPUT_DIR="$SCRIPT_DIR/images"

# Build configuration
VEXFS_VERSION="${VEXFS_VERSION:-1.0.0}"
IMAGE_VARIANT="${IMAGE_VARIANT:-production}"
UBUNTU_VERSION="22.04"
BUILD_TIMEOUT=1800  # 30 minutes

# Helper functions
log_info() {
    echo -e "${BLUE}[BUILD]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
    exit 1
}

show_usage() {
    cat << EOF
VexFS QEMU-based Automated Image Builder

Usage: $0 [OPTIONS]

Options:
    --variant VARIANT      Image variant: minimal, development, production (default: production)
    --version VERSION      VexFS version (default: $VEXFS_VERSION)
    --output-dir DIR       Output directory for images (default: $OUTPUT_DIR)
    --timeout SECONDS      Build timeout in seconds (default: $BUILD_TIMEOUT)
    --no-cleanup           Keep temporary build files
    -h, --help             Show this help message

Examples:
    $0                                    # Build production image
    $0 --variant minimal                 # Build minimal image
    $0 --version 1.1.0 --variant development  # Build development image with custom version

EOF
}

# Parse command line arguments
parse_arguments() {
    CLEANUP=true
    
    while [[ $# -gt 0 ]]; do
        case $1 in
            --variant)
                IMAGE_VARIANT="$2"
                shift 2
                ;;
            --version)
                VEXFS_VERSION="$2"
                shift 2
                ;;
            --output-dir)
                OUTPUT_DIR="$2"
                shift 2
                ;;
            --timeout)
                BUILD_TIMEOUT="$2"
                shift 2
                ;;
            --no-cleanup)
                CLEANUP=false
                shift
                ;;
            -h|--help)
                show_usage
                exit 0
                ;;
            *)
                log_error "Unknown option: $1"
                ;;
        esac
    done
}

# Check dependencies
check_dependencies() {
    log_info "Checking dependencies..."
    
    local missing_deps=()
    local required_tools=("qemu-system-x86_64" "qemu-img" "cloud-localds" "ssh")
    
    for tool in "${required_tools[@]}"; do
        if ! command -v "$tool" >/dev/null 2>&1; then
            missing_deps+=("$tool")
        fi
    done
    
    if [ ${#missing_deps[@]} -ne 0 ]; then
        log_error "Missing dependencies: ${missing_deps[*]}"
    fi
    
    log_success "All dependencies found"
}

# Setup build environment
setup_build_environment() {
    log_info "Setting up build environment..."
    
    mkdir -p "$BUILD_DIR"
    mkdir -p "$OUTPUT_DIR"
    
    # Create build-specific directories
    BUILD_VM_DIR="$BUILD_DIR/vm-$IMAGE_VARIANT"
    BUILD_IMAGE="$BUILD_VM_DIR/vexfs-$IMAGE_VARIANT.qcow2"
    BUILD_CLOUD_INIT="$BUILD_VM_DIR/cloud-init.iso"
    
    mkdir -p "$BUILD_VM_DIR"
    
    log_success "Build environment ready"
}

# Create variant-specific cloud-init configuration
create_build_cloud_init() {
    log_info "Creating cloud-init configuration for $IMAGE_VARIANT variant..."
    
    local user_data="$BUILD_VM_DIR/user-data"
    local meta_data="$BUILD_VM_DIR/meta-data"
    
    # Base packages for all variants
    local base_packages="build-essential linux-headers-generic curl git"
    
    # Variant-specific packages
    case "$IMAGE_VARIANT" in
        "minimal")
            local variant_packages=""
            ;;
        "development")
            local variant_packages="vim gdb strace htop tree"
            ;;
        "production")
            local variant_packages="systemd rsyslog logrotate"
            ;;
        *)
            log_error "Unknown variant: $IMAGE_VARIANT"
            ;;
    esac
    
    # Create user-data with VexFS installation
    cat > "$user_data" << EOF
#cloud-config
users:
  - name: vexfs
    sudo: ALL=(ALL) NOPASSWD:ALL
    shell: /bin/bash
    lock_passwd: false
    passwd: \$6\$rounds=4096\$saltsalt\$L9.LKkHxed8Nq2pLurdnUXGi.2XmQ7Z7rAD6mDKJKPOtJ1KqKzYaP4QMKjIckVZQjSU9kLb1nFjxjKtP.nJ8C1

package_update: true
package_upgrade: true

packages:
  - $base_packages
  - $variant_packages

runcmd:
  # Install Rust
  - curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sudo -u vexfs sh -s -- -y --default-toolchain nightly
  - sudo -u vexfs /home/vexfs/.cargo/bin/rustup target add x86_64-unknown-linux-gnu
  
  # Create VexFS directories
  - mkdir -p /usr/src/vexfs
  - mkdir -p /usr/local/bin
  - mkdir -p /etc/vexfs
  - mkdir -p /var/log
  
  # Set up environment
  - echo 'source ~/.cargo/env' >> /home/vexfs/.bashrc
  - echo 'export PATH="\$HOME/.cargo/bin:\$PATH"' >> /home/vexfs/.bashrc
  
  # Create VexFS configuration
  - |
    cat > /etc/vexfs/vexfs.conf << 'VEXFS_EOF'
    # VexFS Configuration
    version=$VEXFS_VERSION
    variant=$IMAGE_VARIANT
    build_date=$(date -u +%Y-%m-%dT%H:%M:%SZ)
    
    # Default mount options
    default_mount_options=rw,relatime
    
    # Vector cache settings
    vector_cache_size=64M
    vector_cache_enabled=true
    
    # Logging settings
    log_level=info
    log_file=/var/log/vexfs.log
VEXFS_EOF
  
  # Create VexFS systemd service
  - |
    cat > /etc/systemd/system/vexfs.service << 'SYSTEMD_EOF'
    [Unit]
    Description=VexFS Vector Filesystem Service
    After=local-fs.target
    Before=multi-user.target
    
    [Service]
    Type=oneshot
    RemainAfterExit=yes
    ExecStart=/usr/local/bin/vexfs-mount-helper
    ExecStop=/usr/local/bin/vexfs-umount-helper
    TimeoutSec=30
    
    [Install]
    WantedBy=multi-user.target
SYSTEMD_EOF
  
  # Create mount helpers
  - |
    cat > /usr/local/bin/vexfs-mount-helper << 'MOUNT_EOF'
    #!/bin/bash
    # VexFS Mount Helper
    set -e
    
    # Load VexFS module
    if ! lsmod | grep -q vexfs; then
        modprobe vexfs
        echo "VexFS module loaded"
    fi
    
    # Create default mount point
    mkdir -p /mnt/vexfs
    
    # Log startup
    echo "\$(date): VexFS service started" >> /var/log/vexfs.log
MOUNT_EOF
  
  - |
    cat > /usr/local/bin/vexfs-umount-helper << 'UMOUNT_EOF'
    #!/bin/bash
    # VexFS Unmount Helper
    
    # Unmount any VexFS filesystems
    umount -t vexfs -a 2>/dev/null || true
    
    # Unload module if no filesystems are mounted
    if ! mount | grep -q 'type vexfs'; then
        rmmod vexfs 2>/dev/null || true
        echo "VexFS module unloaded"
    fi
    
    # Log shutdown
    echo "\$(date): VexFS service stopped" >> /var/log/vexfs.log
UMOUNT_EOF
  
  - chmod +x /usr/local/bin/vexfs-mount-helper
  - chmod +x /usr/local/bin/vexfs-umount-helper
  
  # Add VexFS to modules load list
  - echo 'vexfs' >> /etc/modules-load.d/vexfs.conf
  
  # Create log file
  - touch /var/log/vexfs.log
  - chmod 644 /var/log/vexfs.log
  
  # Signal build completion
  - touch /tmp/vexfs-build-ready

final_message: "VexFS $IMAGE_VARIANT image is ready!"
EOF
    
    # Create meta-data
    cat > "$meta_data" << EOF
instance-id: vexfs-$IMAGE_VARIANT-build
local-hostname: vexfs-$IMAGE_VARIANT
EOF
    
    # Generate cloud-init ISO
    cloud-localds "$BUILD_CLOUD_INIT" "$user_data" "$meta_data"
    
    log_success "Cloud-init configuration created"
}

# Download and prepare base image
prepare_base_image() {
    log_info "Preparing base Ubuntu $UBUNTU_VERSION image..."
    
    local temp_image="/tmp/ubuntu-$UBUNTU_VERSION-cloudimg.img"
    local cloud_image_url="https://cloud-images.ubuntu.com/releases/$UBUNTU_VERSION/release/ubuntu-$UBUNTU_VERSION-server-cloudimg-amd64.img"
    
    # Download base image if needed
    if [ ! -f "$temp_image" ]; then
        log_info "Downloading Ubuntu cloud image..."
        curl -L -o "$temp_image" "$cloud_image_url" || log_error "Failed to download cloud image"
    fi
    
    # Create build image
    qemu-img convert -f qcow2 -O qcow2 "$temp_image" "$BUILD_IMAGE"
    qemu-img resize "$BUILD_IMAGE" 10G
    
    log_success "Base image prepared"
}

# Build VexFS in the VM
build_vexfs_in_vm() {
    log_info "Starting VM to build VexFS..."
    
    # Start QEMU in background
    local qemu_pid_file="$BUILD_VM_DIR/qemu.pid"
    local ssh_port=$((2222 + RANDOM % 1000))
    
    qemu-system-x86_64 \
        -name "vexfs-$IMAGE_VARIANT-build" \
        -m 2048 \
        -smp 2 \
        -drive "file=$BUILD_IMAGE,format=qcow2,if=virtio" \
        -drive "file=$BUILD_CLOUD_INIT,format=raw,if=virtio,readonly=on" \
        -netdev "user,id=net0,hostfwd=tcp::${ssh_port}-:22" \
        -device "virtio-net,netdev=net0" \
        -virtfs "local,path=$PROJECT_ROOT,mount_tag=vexfs_source,security_model=passthrough,id=vexfs_source" \
        -display none \
        -enable-kvm \
        -cpu host \
        -daemonize \
        -pidfile "$qemu_pid_file" || log_error "Failed to start QEMU"
    
    local qemu_pid=$(cat "$qemu_pid_file")
    log_info "QEMU started with PID: $qemu_pid"
    
    # Wait for VM to be ready
    log_info "Waiting for VM to boot and cloud-init to complete..."
    local start_time=$(date +%s)
    local ssh_ready=false
    
    while [ $(($(date +%s) - start_time)) -lt $BUILD_TIMEOUT ]; do
        if timeout 10 ssh -o ConnectTimeout=5 -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null \
           -p "$ssh_port" vexfs@localhost "test -f /tmp/vexfs-build-ready" >/dev/null 2>&1; then
            ssh_ready=true
            break
        fi
        sleep 10
    done
    
    if [ "$ssh_ready" != true ]; then
        kill "$qemu_pid" 2>/dev/null || true
        log_error "VM boot timeout"
    fi
    
    log_success "VM is ready, starting VexFS build..."
    
    # Copy VexFS source and build
    ssh -o ConnectTimeout=10 -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null \
        -p "$ssh_port" vexfs@localhost << 'BUILD_SCRIPT'
set -e

# Mount VexFS source
sudo mkdir -p /mnt/vexfs_source
sudo mount -t 9p -o trans=virtio,version=9p2000.L vexfs_source /mnt/vexfs_source

# Copy source to build location
sudo cp -r /mnt/vexfs_source/* /usr/src/vexfs/
sudo chown -R vexfs:vexfs /usr/src/vexfs

# Build VexFS kernel module
cd /usr/src/vexfs
source ~/.cargo/env
make clean
make vm-build

# Install kernel module
sudo mkdir -p /lib/modules/$(uname -r)/extra/vexfs
sudo cp vexfs.ko /lib/modules/$(uname -r)/extra/vexfs/
sudo depmod -a

# Build and install vexctl
cd vexctl
cargo build --release
sudo cp target/release/vexctl /usr/local/bin/
sudo chmod +x /usr/local/bin/vexctl

# Test installation
sudo modprobe vexfs
lsmod | grep vexfs
vexctl --version
sudo rmmod vexfs

echo "VexFS build completed successfully!"
BUILD_SCRIPT
    
    if [ $? -eq 0 ]; then
        log_success "VexFS build completed successfully"
    else
        kill "$qemu_pid" 2>/dev/null || true
        log_error "VexFS build failed"
    fi
    
    # Shutdown VM gracefully
    log_info "Shutting down VM..."
    ssh -o ConnectTimeout=10 -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null \
        -p "$ssh_port" vexfs@localhost "sudo shutdown -h now" 2>/dev/null || true
    
    # Wait for shutdown
    sleep 10
    
    # Force kill if still running
    if kill -0 "$qemu_pid" 2>/dev/null; then
        log_warning "Force killing QEMU process"
        kill "$qemu_pid" 2>/dev/null || true
    fi
    
    rm -f "$qemu_pid_file"
}

# Optimize and finalize image
finalize_image() {
    log_info "Finalizing image..."
    
    local final_image="$OUTPUT_DIR/vexfs-$IMAGE_VARIANT-$VEXFS_VERSION.qcow2"
    local compressed_image="$OUTPUT_DIR/vexfs-$IMAGE_VARIANT-$VEXFS_VERSION.qcow2.gz"
    
    # Copy to final location
    cp "$BUILD_IMAGE" "$final_image"
    
    # Compress image
    log_info "Compressing image..."
    gzip -c "$final_image" > "$compressed_image"
    
    # Generate checksums
    cd "$OUTPUT_DIR"
    sha256sum "$(basename "$final_image")" > "$(basename "$final_image").sha256"
    sha256sum "$(basename "$compressed_image")" > "$(basename "$compressed_image").sha256"
    
    # Create manifest
    cat > "vexfs-$IMAGE_VARIANT-$VEXFS_VERSION.manifest" << EOF
VexFS Image Manifest
===================

Image: vexfs-$IMAGE_VARIANT-$VEXFS_VERSION.qcow2
Variant: $IMAGE_VARIANT
Version: $VEXFS_VERSION
Build Date: $(date -u +%Y-%m-%dT%H:%M:%SZ)
Base OS: Ubuntu $UBUNTU_VERSION

Components:
- VexFS kernel module: /lib/modules/\$(uname -r)/extra/vexfs/vexfs.ko
- vexctl binary: /usr/local/bin/vexctl
- Configuration: /etc/vexfs/vexfs.conf
- Systemd service: /etc/systemd/system/vexfs.service

Usage:
  qemu-system-x86_64 -m 2048 -drive file=$final_image,format=qcow2 -netdev user,id=net0,hostfwd=tcp::2222-:22 -device virtio-net,netdev=net0

Login:
  User: vexfs
  Password: vexfs
  SSH: ssh -p 2222 vexfs@localhost
EOF
    
    log_success "Image finalized: $final_image"
    log_info "Compressed: $compressed_image"
    log_info "Manifest: vexfs-$IMAGE_VARIANT-$VEXFS_VERSION.manifest"
}

# Cleanup build files
cleanup_build() {
    if [ "$CLEANUP" = true ]; then
        log_info "Cleaning up build files..."
        rm -rf "$BUILD_VM_DIR"
        log_success "Cleanup completed"
    else
        log_info "Skipping cleanup (disabled)"
    fi
}

# Main build process
main() {
    echo "🚀 VexFS QEMU-based Automated Image Builder"
    echo "============================================"
    echo "Variant: $IMAGE_VARIANT"
    echo "Version: $VEXFS_VERSION"
    echo "Output: $OUTPUT_DIR"
    echo ""
    
    parse_arguments "$@"
    check_dependencies
    setup_build_environment
    create_build_cloud_init
    prepare_base_image
    build_vexfs_in_vm
    finalize_image
    cleanup_build
    
    echo ""
    log_success "🎉 VexFS $IMAGE_VARIANT image build completed successfully!"
    log_info "Image available at: $OUTPUT_DIR/vexfs-$IMAGE_VARIANT-$VEXFS_VERSION.qcow2"
    echo ""
    echo "To test the image:"
    echo "  qemu-system-x86_64 -m 2048 -drive file=$OUTPUT_DIR/vexfs-$IMAGE_VARIANT-$VEXFS_VERSION.qcow2,format=qcow2 -netdev user,id=net0,hostfwd=tcp::2222-:22 -device virtio-net,netdev=net0"
    echo "  ssh -p 2222 vexfs@localhost"
}

# Run main function with all arguments
main "$@"