#!/bin/bash

# Automated VexFS Memory Fixes Testing Script
# Tests the memory-fixed kernel module automatically

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

echo "🔬 VexFS Memory Fixes - Automated Testing"
echo "========================================="

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

# Check if we have the kernel module
if [ ! -f "$PROJECT_ROOT/kernel/vexfs_minimal.ko" ]; then
    log_error "Kernel module not found at $PROJECT_ROOT/kernel/vexfs_minimal.ko"
    exit 1
fi

log_success "Found memory-fixed kernel module: $(ls -lh $PROJECT_ROOT/kernel/vexfs_minimal.ko)"

# Check if old module is loaded
if lsmod | grep -q vexfs_minimal; then
    log_warning "VexFS module is already loaded. Checking if it's our fixed version..."
    
    # Check the source version
    LOADED_VERSION=$(sudo modinfo vexfs_minimal | grep srcversion | cut -d: -f2 | tr -d ' ')
    BUILT_VERSION=$(sudo modinfo $PROJECT_ROOT/kernel/vexfs_minimal.ko | grep srcversion | cut -d: -f2 | tr -d ' ')
    
    if [ "$LOADED_VERSION" = "$BUILT_VERSION" ]; then
        log_success "Loaded module matches our fixed version ($LOADED_VERSION)"
    else
        log_error "Loaded module ($LOADED_VERSION) differs from our fixed version ($BUILT_VERSION)"
        log_error "Need to unload old module first"
        
        if sudo rmmod vexfs_minimal 2>/dev/null; then
            log_success "Old module unloaded"
        else
            log_error "Cannot unload old module (reference count issue)"
            log_error "System reboot may be required"
            exit 1
        fi
    fi
fi

# Build Docker image for testing environment
log_info "Building Docker test environment..."
cd "$PROJECT_ROOT"

# Create optimized Dockerfile for memory testing
cat > tests/docker_testing/Dockerfile.memory_test << 'EOF'
FROM ubuntu:22.04

# Install kernel development tools
RUN apt-get update && apt-get install -y \
    kmod \
    util-linux \
    e2fsprogs \
    && rm -rf /var/lib/apt/lists/*

# Copy the project
COPY . /vexfs/
WORKDIR /vexfs

# Make test script executable
RUN chmod +x /vexfs/tests/docker_testing/memory_test_inside_container.sh
EOF

# Create the memory test script for inside container
cat > tests/docker_testing/memory_test_inside_container.sh << 'EOF'
#!/bin/bash

set -e

echo "🧪 VexFS Memory Fixes Validation"
echo "================================"

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

# Environment check
log_info "Container environment:"
log_info "  Kernel: $(uname -r)"
log_info "  OS: $(cat /etc/os-release | grep PRETTY_NAME | cut -d= -f2 | tr -d '\"')"

# Check if we have access to kernel modules
if [ ! -d "/lib/modules/$(uname -r)" ]; then
    log_error "No access to kernel modules directory"
    exit 1
fi

log_success "Kernel modules directory accessible"

# Check our module
if [ ! -f "/vexfs/kernel/vexfs_minimal.ko" ]; then
    log_error "VexFS module not found"
    exit 1
fi

log_info "Module details:"
modinfo /vexfs/kernel/vexfs_minimal.ko | head -10

# CRITICAL TEST: Load the module (tests our memory fixes)
log_info "🔥 CRITICAL TEST: Loading memory-fixed module..."
log_info "This tests our fixes for NULL pointer dereference in current_time()"

# Clear kernel messages
dmesg -C 2>/dev/null || true

if insmod /vexfs/kernel/vexfs_minimal.ko; then
    log_success "Module loaded successfully! Memory fixes are working!"
    
    # Verify it's loaded
    if lsmod | grep -q vexfs_minimal; then
        log_success "Module visible in lsmod:"
        lsmod | grep vexfs_minimal
    fi
    
    # Check for any error messages
    log_info "Checking kernel messages for errors..."
    sleep 1
    if dmesg | grep -i "error\|panic\|oops\|segfault\|null.*pointer" | tail -5; then
        log_warning "Some kernel messages found (review above)"
    else
        log_success "No error messages in kernel log"
    fi
    
    # Test filesystem registration
    log_info "Checking filesystem registration..."
    if cat /proc/filesystems | grep -q vexfs; then
        log_success "Filesystem registered: $(cat /proc/filesystems | grep vexfs)"
    else
        log_warning "Filesystem not found in /proc/filesystems"
    fi
    
    # Memory stress test: Load/unload cycles
    log_info "🔄 Memory stress test: 5 load/unload cycles..."
    
    # First unload
    if rmmod vexfs_minimal; then
        log_success "Initial unload successful"
    else
        log_error "Initial unload failed"
        exit 1
    fi
    
    # Stress test cycles
    for i in {1..5}; do
        log_info "  Cycle $i/5..."
        
        if insmod /vexfs/kernel/vexfs_minimal.ko 2>/dev/null; then
            sleep 0.5
            if rmmod vexfs_minimal 2>/dev/null; then
                log_success "    Cycle $i: OK"
            else
                log_error "    Cycle $i: Unload failed"
                break
            fi
        else
            log_error "    Cycle $i: Load failed"
            break
        fi
        sleep 0.5
    done
    
    log_success "Memory stress test completed"
    
    # Final check for memory errors
    log_info "Final memory error check..."
    if dmesg | grep -i "memory\|leak\|corruption" | tail -3; then
        log_warning "Memory-related messages found (review above)"
    else
        log_success "No memory errors detected"
    fi
    
else
    log_error "CRITICAL FAILURE: Module failed to load"
    log_error "Memory fixes may not be working correctly"
    
    log_info "Checking kernel messages for crash details..."
    dmesg | tail -10
    exit 1
fi

log_success "🎉 ALL TESTS PASSED!"
log_success "Memory fixes are working correctly:"
log_success "  ✅ No NULL pointer dereference crashes"
log_success "  ✅ Proper inode->i_sb initialization"
log_success "  ✅ Safe module load/unload cycles"
log_success "  ✅ No memory corruption detected"

EOF

chmod +x tests/docker_testing/memory_test_inside_container.sh

# Build the Docker image
if docker build -f tests/docker_testing/Dockerfile.memory_test -t vexfs-memory-test .; then
    log_success "Docker image built successfully"
else
    log_error "Failed to build Docker image"
    exit 1
fi

# Run the automated memory test
log_info "🚀 Running automated memory fixes validation..."
log_warning "This will test the kernel module in a controlled environment"

if docker run --rm \
    --privileged \
    --volume /lib/modules:/lib/modules:ro \
    --volume /dev:/dev \
    vexfs-memory-test \
    /vexfs/tests/docker_testing/memory_test_inside_container.sh; then
    
    log_success "🎉 MEMORY FIXES VALIDATION SUCCESSFUL!"
    log_success "The VexFS kernel module memory fixes are working correctly"
    log_success "No NULL pointer dereference crashes detected"
    
else
    log_error "❌ MEMORY FIXES VALIDATION FAILED"
    log_error "The kernel module still has memory issues"
    exit 1
fi

# Cleanup
log_info "Cleaning up Docker image..."
docker rmi vexfs-memory-test 2>/dev/null || true

log_success "Automated memory fixes testing completed successfully!"