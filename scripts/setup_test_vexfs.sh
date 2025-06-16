#!/bin/bash
# VexFS Real Testing Setup Script
# Copyright 2025 VexFS Contributors
# Licensed under the Apache License, Version 2.0

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
TEST_DIR="/tmp/test"
VEXFS_IMG="/tmp/vexfs.img"
VEXFS_SIZE="100M"
VEXCTL_BINARY="./vexctl/target/release/vexctl"

print_header() {
    echo -e "${BLUE}================================${NC}"
    echo -e "${BLUE}  VexFS Real Testing Setup${NC}"
    echo -e "${BLUE}================================${NC}"
    echo
}

print_section() {
    echo -e "${GREEN}>>> $1${NC}"
}

print_warning() {
    echo -e "${YELLOW}WARNING: $1${NC}"
}

print_error() {
    echo -e "${RED}ERROR: $1${NC}"
}

print_info() {
    echo -e "${BLUE}INFO: $1${NC}"
}

check_root() {
    if [[ $EUID -eq 0 ]]; then
        print_error "This script should not be run as root initially"
        print_info "Run as regular user first, it will ask for sudo when needed"
        exit 1
    fi
}

check_dependencies() {
    print_section "Checking dependencies"
    
    # Check if we're in the VexFS project root
    if [[ ! -f "Cargo.toml" ]] || [[ ! -d "vexctl" ]] || [[ ! -f "Makefile" ]]; then
        print_error "This script must be run from the VexFS project root directory"
        exit 1
    fi
    
    # Check for required tools
    local missing_tools=()
    
    if ! command -v cargo &> /dev/null; then
        missing_tools+=("cargo (Rust)")
    fi
    
    if ! command -v make &> /dev/null; then
        missing_tools+=("make")
    fi
    
    if ! command -v gcc &> /dev/null; then
        missing_tools+=("gcc")
    fi
    
    # Check for kernel headers
    local kernel_version=$(uname -r)
    if [[ ! -d "/lib/modules/$kernel_version/build" ]]; then
        missing_tools+=("linux-headers-$kernel_version")
    fi
    
    if [[ ${#missing_tools[@]} -gt 0 ]]; then
        print_error "Missing required dependencies:"
        for tool in "${missing_tools[@]}"; do
            echo "  - $tool"
        done
        echo
        print_info "Install missing dependencies:"
        print_info "  Ubuntu/Debian: sudo apt-get install build-essential linux-headers-\$(uname -r)"
        print_info "  CentOS/RHEL: sudo yum groupinstall 'Development Tools' && sudo yum install kernel-devel"
        print_info "  Rust: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
        exit 1
    fi
    
    print_info "All dependencies found"
}

build_vexctl() {
    print_section "Building vexctl"
    
    cd vexctl
    if cargo build --release; then
        print_info "vexctl built successfully"
    else
        print_error "Failed to build vexctl"
        exit 1
    fi
    cd ..
}

build_kernel_module() {
    print_section "Building VexFS kernel module"
    
    print_info "Cleaning previous builds..."
    make clean
    
    print_info "Building kernel module..."
    if make; then
        print_info "VexFS kernel module built successfully"
        if [[ -f "vexfs.ko" ]]; then
            print_info "Module file: vexfs.ko"
        else
            print_error "Module file vexfs.ko not found after build"
            exit 1
        fi
    else
        print_error "Failed to build VexFS kernel module"
        print_info "Common issues:"
        print_info "  - Missing kernel headers: sudo apt-get install linux-headers-\$(uname -r)"
        print_info "  - Wrong kernel version: make sure headers match running kernel"
        print_info "  - Missing build tools: sudo apt-get install build-essential"
        exit 1
    fi
}

load_kernel_module() {
    print_section "Loading VexFS kernel module"
    
    # Check if module is already loaded
    if lsmod | grep -q "^vexfs "; then
        print_warning "VexFS module already loaded, unloading first..."
        sudo rmmod vexfs || true
    fi
    
    print_info "Loading VexFS kernel module..."
    if sudo insmod vexfs.ko; then
        print_info "VexFS kernel module loaded successfully"
        
        # Verify module is loaded
        if lsmod | grep -q "^vexfs "; then
            print_info "Module verification: OK"
        else
            print_error "Module loaded but not found in lsmod output"
            exit 1
        fi
        
        # Check dmesg for any errors
        print_info "Checking kernel messages..."
        dmesg | tail -5 | grep -i vexfs || print_info "No VexFS messages in recent dmesg"
    else
        print_error "Failed to load VexFS kernel module"
        print_info "Check dmesg for error details: dmesg | tail -10"
        exit 1
    fi
}

create_filesystem_image() {
    print_section "Creating VexFS filesystem image"
    
    # Remove existing image if present
    if [[ -f "$VEXFS_IMG" ]]; then
        print_warning "Removing existing filesystem image..."
        rm -f "$VEXFS_IMG"
    fi
    
    print_info "Creating ${VEXFS_SIZE} filesystem image at $VEXFS_IMG..."
    if dd if=/dev/zero of="$VEXFS_IMG" bs=1M count=${VEXFS_SIZE%M} status=progress; then
        print_info "Filesystem image created successfully"
    else
        print_error "Failed to create filesystem image"
        exit 1
    fi
    
    # TODO: Format the image with mkfs.vexfs when available
    print_warning "mkfs.vexfs not yet implemented - using raw image"
    print_info "Image will be formatted during first mount (if supported)"
}

mount_filesystem() {
    print_section "Mounting VexFS filesystem"
    
    # Create mount point
    if [[ ! -d "$TEST_DIR" ]]; then
        print_info "Creating mount point: $TEST_DIR"
        sudo mkdir -p "$TEST_DIR"
    fi
    
    # Check if already mounted
    if mount | grep -q "$TEST_DIR"; then
        print_warning "Something already mounted at $TEST_DIR, unmounting..."
        sudo umount "$TEST_DIR" || true
    fi
    
    print_info "Mounting VexFS at $TEST_DIR..."
    if sudo mount -t vexfs "$VEXFS_IMG" "$TEST_DIR"; then
        print_info "VexFS mounted successfully"
        
        # Verify mount
        if mount | grep -q "vexfs.*$TEST_DIR"; then
            print_info "Mount verification: OK"
            mount | grep "vexfs.*$TEST_DIR"
        else
            print_error "Mount command succeeded but filesystem not found in mount table"
            exit 1
        fi
    else
        print_error "Failed to mount VexFS"
        print_info "Common issues:"
        print_info "  - Module not loaded: check 'lsmod | grep vexfs'"
        print_info "  - Permission denied: script should ask for sudo"
        print_info "  - Invalid filesystem: mkfs.vexfs not yet implemented"
        print_info "Check dmesg for kernel error messages: dmesg | tail -10"
        exit 1
    fi
}

test_vexctl() {
    print_section "Testing vexctl"
    
    print_info "Testing vexctl status command..."
    if "$VEXCTL_BINARY" status "$TEST_DIR"; then
        print_info "✓ vexctl status command works!"
    else
        print_error "✗ vexctl status command failed"
        print_info "This might be expected if IOCTL interface is not fully implemented"
    fi
    
    print_info "Testing basic filesystem operations..."
    if sudo touch "$TEST_DIR/test_file"; then
        print_info "✓ File creation works"
        sudo rm -f "$TEST_DIR/test_file"
    else
        print_warning "File creation failed - filesystem might be read-only"
    fi
    
    if ls -la "$TEST_DIR"; then
        print_info "✓ Directory listing works"
    else
        print_warning "Directory listing failed"
    fi
}

create_test_script() {
    print_section "Creating test script"
    
    cat > scripts/test_vexfs.sh << 'EOF'
#!/bin/bash
# VexFS Testing Script

set -e

VEXCTL="./vexctl/target/release/vexctl"
TEST_DIR="/tmp/test"

echo "VexFS Testing Suite"
echo "=================="
echo

echo "1. Testing vexctl status:"
$VEXCTL status $TEST_DIR
echo

echo "2. Testing filesystem operations:"
echo "Mount status:"
mount | grep vexfs || echo "No VexFS mounts found"
echo

echo "Directory contents:"
ls -la $TEST_DIR
echo

echo "3. Kernel module status:"
lsmod | grep vexfs || echo "VexFS module not loaded"
echo

echo "4. Recent kernel messages:"
dmesg | tail -10 | grep -i vexfs || echo "No recent VexFS kernel messages"
echo

echo "Testing complete!"
EOF

    chmod +x scripts/test_vexfs.sh
    print_info "Test script created at scripts/test_vexfs.sh"
}

create_cleanup_script() {
    print_section "Creating cleanup script"
    
    cat > scripts/cleanup_vexfs.sh << 'EOF'
#!/bin/bash
# VexFS Cleanup Script

set -e

TEST_DIR="/tmp/test"
VEXFS_IMG="/tmp/vexfs.img"

echo "VexFS Cleanup"
echo "============="
echo

echo "Unmounting filesystem..."
if mount | grep -q "$TEST_DIR"; then
    sudo umount "$TEST_DIR" && echo "✓ Unmounted $TEST_DIR"
else
    echo "No filesystem mounted at $TEST_DIR"
fi

echo "Removing kernel module..."
if lsmod | grep -q "^vexfs "; then
    sudo rmmod vexfs && echo "✓ Removed vexfs module"
else
    echo "VexFS module not loaded"
fi

echo "Cleaning up files..."
if [[ -f "$VEXFS_IMG" ]]; then
    rm -f "$VEXFS_IMG" && echo "✓ Removed $VEXFS_IMG"
fi

if [[ -d "$TEST_DIR" ]]; then
    sudo rmdir "$TEST_DIR" 2>/dev/null && echo "✓ Removed $TEST_DIR" || echo "Could not remove $TEST_DIR (not empty?)"
fi

echo "Cleanup complete!"
EOF

    chmod +x scripts/cleanup_vexfs.sh
    print_info "Cleanup script created at scripts/cleanup_vexfs.sh"
}

show_usage_instructions() {
    print_section "Setup Complete!"
    echo
    print_info "VexFS is now set up and ready for testing:"
    echo
    echo "📁 Filesystem mounted at: ${GREEN}$TEST_DIR${NC}"
    echo "💾 Image file: ${GREEN}$VEXFS_IMG${NC}"
    echo "🔧 vexctl binary: ${GREEN}$VEXCTL_BINARY${NC}"
    echo
    print_info "Test commands:"
    echo "  ${GREEN}$VEXCTL_BINARY status $TEST_DIR${NC}"
    echo "  ${GREEN}./scripts/test_vexfs.sh${NC}"
    echo
    print_info "Filesystem operations:"
    echo "  ${GREEN}ls -la $TEST_DIR${NC}"
    echo "  ${GREEN}sudo touch $TEST_DIR/test_file${NC}"
    echo
    print_info "Cleanup when done:"
    echo "  ${GREEN}./scripts/cleanup_vexfs.sh${NC}"
    echo
    print_warning "Remember: This is a development filesystem - don't store important data!"
}

main() {
    print_header
    
    check_root
    check_dependencies
    build_vexctl
    build_kernel_module
    load_kernel_module
    create_filesystem_image
    mount_filesystem
    test_vexctl
    create_test_script
    create_cleanup_script
    
    show_usage_instructions
}

# Handle script interruption
trap 'echo -e "\n${RED}Script interrupted. Run ./scripts/cleanup_vexfs.sh to clean up.${NC}"; exit 1' INT TERM

# Run main function
main "$@"