#!/bin/bash

# VexFS VM Kernel Module Testing Script
# This script creates a lightweight VM environment for testing the VexFS kernel module

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

# VM Configuration
VM_NAME="vexfs-test-vm"
VM_MEMORY="2G"
VM_CORES="2"
TIMESTAMP=$(date '+%Y%m%d_%H%M%S')
LOG_FILE="$LOGS_DIR/vm_test_${TIMESTAMP}.log"

# Check if kernel module exists
if [[ ! -f "$PROJECT_ROOT/kernel/vexfs.ko" ]]; then
    error "VexFS kernel module not found at $PROJECT_ROOT/kernel/vexfs.ko"
    error "Please build the kernel module first with 'make' in the project root"
    exit 1
fi

# Create directories
mkdir -p "$VM_DIR"
mkdir -p "$LOGS_DIR"

log "VexFS VM Kernel Module Testing"
log "=============================="
log "Kernel module: $(ls -lh $PROJECT_ROOT/kernel/vexfs.ko)"
log "Log file: $LOG_FILE"
echo

# Check dependencies
if ! command -v qemu-system-x86_64 &> /dev/null; then
    error "QEMU not found. Please install qemu-system-x86_64"
    exit 1
fi

# Download Alpine Linux ISO if not present (lightweight ~150MB)
ALPINE_ISO="$VM_DIR/alpine-virt-3.19.0-x86_64.iso"
if [[ ! -f "$ALPINE_ISO" ]]; then
    log "Alpine Linux ISO not found. Downloading lightweight distro (~150MB)..."
    warn "This will download ~150MB. Continue? (y/N)"
    read -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        wget -O "$ALPINE_ISO" \
            "https://dl-cdn.alpinelinux.org/alpine/v3.19/releases/x86_64/alpine-virt-3.19.0-x86_64.iso"
        success "Alpine Linux ISO downloaded"
    else
        error "Alpine Linux ISO required for VM testing"
        exit 1
    fi
fi

# Create a temporary disk for the VM session
TEMP_DISK="$VM_DIR/temp_disk_${TIMESTAMP}.qcow2"
qemu-img create -f qcow2 "$TEMP_DISK" 8G

log "Starting VM with Alpine Linux environment..."
log "VM will boot from Alpine Linux ISO (~150MB lightweight distro)"
log "Alpine Linux is perfect for kernel testing - minimal and fast!"
echo

# Create a script to copy into the VM
cat > "$VM_DIR/vm_test_script.sh" << 'EOF'
#!/bin/bash

echo "VexFS Kernel Module Testing in VM"
echo "================================="

# Check if running in VM
if ! grep -q "QEMU" /proc/cpuinfo 2>/dev/null; then
    echo "Warning: This doesn't appear to be running in a QEMU VM"
fi

# Check if kernel module is available
if [[ -f /mnt/vexfs_host/kernel/vexfs.ko ]]; then
    echo "Found VexFS kernel module: $(ls -lh /mnt/vexfs_host/kernel/vexfs.ko)"
    
    # Copy module to local filesystem
    sudo cp /mnt/vexfs_host/kernel/vexfs.ko /tmp/vexfs.ko
    
    echo "Running basic kernel module tests..."
    
    # Test 1: Module info
    echo "=== Test 1: Module Information ==="
    modinfo /tmp/vexfs.ko
    
    # Test 2: Load module
    echo "=== Test 2: Loading Module ==="
    sudo insmod /tmp/vexfs.ko
    
    if lsmod | grep -q vexfs; then
        echo "SUCCESS: VexFS module loaded"
        lsmod | grep vexfs
        
        # Check dmesg for module messages
        echo "=== Kernel Messages ==="
        dmesg | tail -10
        
        # Test 3: Unload module
        echo "=== Test 3: Unloading Module ==="
        sudo rmmod vexfs
        
        if ! lsmod | grep -q vexfs; then
            echo "SUCCESS: VexFS module unloaded"
        else
            echo "WARNING: Module may still be loaded"
        fi
    else
        echo "ERROR: Failed to load VexFS module"
        dmesg | tail -10
    fi
    
    # Test 4: Stress test (load/unload cycles)
    echo "=== Test 4: Stress Test (5 cycles) ==="
    for i in {1..5}; do
        echo "Cycle $i..."
        sudo insmod /tmp/vexfs.ko
        sleep 1
        sudo rmmod vexfs
        sleep 1
    done
    echo "Stress test completed"
    
else
    echo "ERROR: VexFS kernel module not found"
    echo "Make sure the shared directory is mounted"
fi
EOF

chmod +x "$VM_DIR/vm_test_script.sh"

# Start QEMU VM
log "Starting QEMU VM..."
log "VM Console will open. To run tests in Alpine:"
log "1. Boot Alpine Linux (select option 1 or just wait)"
log "2. Login as 'root' (no password needed)"
log "3. Setup networking: setup-interfaces (press Enter for defaults)"
log "4. Mount shared directory:"
log "   mkdir -p /mnt/vexfs_host"
log "   mount -t 9p -o trans=virtio,version=9p2000.L vexfs_host /mnt/vexfs_host"
log "5. Run test script: sh /mnt/vexfs_host/tests/vm_testing/vm_test_script.sh"
echo

# Start VM with shared directory (use system QEMU with clean environment and headless display)
env -i PATH="/usr/bin:/bin:/usr/sbin:/sbin" LD_LIBRARY_PATH="/lib/x86_64-linux-gnu:/usr/lib/x86_64-linux-gnu" \
/usr/bin/qemu-system-x86_64 \
    -name "$VM_NAME" \
    -m $VM_MEMORY \
    -smp $VM_CORES \
    -hda "$TEMP_DISK" \
    -cdrom "$ALPINE_ISO" \
    -boot d \
    -enable-kvm \
    -netdev user,id=net0 \
    -device e1000,netdev=net0 \
    -virtfs local,path="$PROJECT_ROOT",mount_tag=vexfs_host,security_model=passthrough,id=vexfs_share \
    -nographic \
    2>&1 | tee "$LOG_FILE"

# Cleanup
log "VM session ended. Cleaning up..."
rm -f "$TEMP_DISK"

success "VM testing session completed. Log saved to: $LOG_FILE"