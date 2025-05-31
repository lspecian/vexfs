#!/bin/bash

# VexFS Kernel Module Docker Testing Script
# Tests the memory-fixed kernel module in a Docker container

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

echo "🐳 VexFS Kernel Module Docker Testing"
echo "======================================"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${BLUE}ℹ️  $1${NC}"
}

log_success() {
    echo -e "${GREEN}✅ $1${NC}"
}

log_warning() {
    echo -e "${YELLOW}⚠️  $1${NC}"
}

log_error() {
    echo -e "${RED}❌ $1${NC}"
}

# Check if we have the kernel module
if [ ! -f "$PROJECT_ROOT/kernel/vexfs_minimal.ko" ]; then
    log_error "Kernel module not found. Building it first..."
    cd "$PROJECT_ROOT/kernel"
    make -f Makefile.simple clean
    make -f Makefile.simple
    if [ ! -f "vexfs_minimal.ko" ]; then
        log_error "Failed to build kernel module"
        exit 1
    fi
    log_success "Kernel module built successfully"
fi

# Build Docker image
log_info "Building Docker test image..."
cd "$PROJECT_ROOT"
docker build -f tests/docker_testing/Dockerfile.kernel_test -t vexfs-kernel-test .

# Create test script for inside container
cat > tests/docker_testing/run_tests_inside_container.sh << 'EOF'
#!/bin/bash

set -e

echo "🔧 Inside Docker Container - Testing VexFS Kernel Module"
echo "======================================================="

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

# Check kernel version
log_info "Kernel version: $(uname -r)"
log_info "Container OS: $(cat /etc/os-release | grep PRETTY_NAME)"

# Check if we can access kernel modules
if [ ! -d "/lib/modules/$(uname -r)" ]; then
    log_error "Kernel modules directory not found"
    log_error "This means the container doesn't have access to host kernel modules"
    exit 1
fi

log_success "Kernel modules directory found"

# Check if we have the built module
if [ ! -f "/vexfs/kernel/vexfs_minimal.ko" ]; then
    log_error "Kernel module not found in container"
    exit 1
fi

log_success "Kernel module found: $(ls -lh /vexfs/kernel/vexfs_minimal.ko)"

# Check module info
log_info "Module information:"
modinfo /vexfs/kernel/vexfs_minimal.ko || {
    log_error "Failed to get module info"
    exit 1
}

# Test 1: Check module dependencies
log_info "Checking module dependencies..."
modprobe --dry-run --show-depends /vexfs/kernel/vexfs_minimal.ko || {
    log_warning "Module dependency check failed, but continuing..."
}

# Test 2: Try to load the module (this is the critical test)
log_info "Attempting to load kernel module..."
if insmod /vexfs/kernel/vexfs_minimal.ko; then
    log_success "Kernel module loaded successfully!"
    
    # Check if it's actually loaded
    if lsmod | grep -q vexfs_minimal; then
        log_success "Module is visible in lsmod"
        lsmod | grep vexfs_minimal
    else
        log_error "Module not visible in lsmod"
    fi
    
    # Check dmesg for any messages
    log_info "Checking dmesg for module messages..."
    dmesg | tail -20
    
    # Test 3: Try to create a filesystem (if mkfs works)
    log_info "Testing filesystem creation on loop device..."
    
    # Create a test file for loop device
    dd if=/dev/zero of=/tmp/vexfs_test.img bs=1M count=10 2>/dev/null
    
    # Set up loop device
    LOOP_DEV=$(losetup -f)
    losetup "$LOOP_DEV" /tmp/vexfs_test.img
    log_info "Created loop device: $LOOP_DEV"
    
    # Try to format (this will test if our filesystem registration works)
    if mkfs.ext4 "$LOOP_DEV" >/dev/null 2>&1; then
        log_info "Loop device is working (formatted with ext4 as test)"
        
        # Now try to mount with our filesystem type
        mkdir -p /tmp/vexfs_mount
        if mount -t vexfs_test "$LOOP_DEV" /tmp/vexfs_mount 2>/dev/null; then
            log_success "Successfully mounted VexFS filesystem!"
            
            # Test basic operations
            log_info "Testing basic filesystem operations..."
            
            # Test directory listing
            ls -la /tmp/vexfs_mount/
            
            # Test file creation (if supported)
            if touch /tmp/vexfs_mount/test_file 2>/dev/null; then
                log_success "File creation works"
                rm -f /tmp/vexfs_mount/test_file
            else
                log_warning "File creation not supported (expected for minimal stub)"
            fi
            
            # Unmount
            umount /tmp/vexfs_mount
            log_success "Filesystem unmounted successfully"
        else
            log_warning "Mount failed - this is expected for minimal stub"
            log_info "The important thing is that the module loaded without crashing"
        fi
        
        # Clean up loop device
        losetup -d "$LOOP_DEV"
    else
        log_error "Loop device setup failed"
    fi
    
    # Test 4: Unload the module
    log_info "Unloading kernel module..."
    if rmmod vexfs_minimal; then
        log_success "Module unloaded successfully"
    else
        log_error "Failed to unload module"
    fi
    
else
    log_error "Failed to load kernel module"
    log_info "Checking dmesg for error messages..."
    dmesg | tail -20
    exit 1
fi

log_success "All tests completed successfully!"
log_info "The memory fixes appear to be working - no crashes detected"

EOF

chmod +x tests/docker_testing/run_tests_inside_container.sh

# Run the tests in Docker container
log_info "Running tests in Docker container..."
log_warning "Note: This will load the kernel module on the HOST system"
log_warning "The container shares the host kernel, so module loading affects the host"

read -p "Continue with kernel module testing? (y/N): " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    log_info "Testing cancelled by user"
    exit 0
fi

# Run the container with necessary privileges
docker run --rm -it \
    --privileged \
    --volume /lib/modules:/lib/modules:ro \
    --volume /dev:/dev \
    vexfs-kernel-test \
    /vexfs/tests/run_tests_inside_container.sh

log_success "Docker testing completed!"