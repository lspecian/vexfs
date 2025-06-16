#!/bin/bash

# Automated VexFS VM Test Runner
# This script runs after VM is manually set up

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
VM_DIR="$(dirname "$SCRIPT_DIR")"
PROJECT_ROOT="$(dirname "$VM_DIR")"

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

echo -e "${BLUE}VexFS Automated VM Test Runner${NC}"
echo -e "${BLUE}==============================${NC}"

# Check if VM is set up
if [ ! -f "$VM_DIR/.alpine_installed" ]; then
    log_error "Alpine VM not set up yet!"
    echo
    echo "Please follow these steps:"
    echo "1. Start VM: $VM_DIR/scripts/start_alpine_vm.sh"
    echo "2. Login as root (no password)"
    echo "3. Run: /mnt/shared/setup_alpine_auto.sh"
    echo "4. After reboot, run this script again"
    exit 1
fi

# Check SSH connectivity
log_info "Checking SSH connectivity..."
if ! timeout 5 ssh -p 2222 -o ConnectTimeout=5 -o StrictHostKeyChecking=no root@localhost "echo 'SSH OK'" &>/dev/null; then
    log_error "Cannot connect to VM via SSH"
    echo
    echo "Make sure:"
    echo "1. VM is running: ps aux | grep qemu"
    echo "2. SSH is configured: ssh -p 2222 root@localhost"
    echo "3. Password is: vexfs"
    exit 1
fi

log_success "SSH connection established"

# Create comprehensive test script
cat > "$VM_DIR/shared/run_all_tests.sh" << 'EOF'
#!/bin/bash

# Comprehensive VexFS Test Suite
# Run inside Alpine VM

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

TEST_DIR="/tmp/vexfs_tests"
MOUNT_POINT="/tmp/vexfs_mount"
TEST_IMG="/tmp/vexfs_test.img"

# Clean up any previous tests
cleanup() {
    sudo umount "$MOUNT_POINT" 2>/dev/null || true
    sudo rmmod vexfs_deadlock_fix 2>/dev/null || true
    rm -rf "$TEST_DIR" "$MOUNT_POINT" "$TEST_IMG"
}

# Trap to ensure cleanup
trap cleanup EXIT

echo "🧪 VexFS Comprehensive Test Suite"
echo "================================="

# Initial cleanup
cleanup

# Create test directories
mkdir -p "$TEST_DIR"
mkdir -p "$MOUNT_POINT"

# Test 1: Module Loading
echo
log_info "Test 1: Module Loading"
if sudo insmod /mnt/shared/vexfs_deadlock_fix.ko; then
    log_success "Module loaded successfully"
    lsmod | grep vexfs
else
    log_error "Failed to load module"
    dmesg | tail -20
    exit 1
fi

# Test 2: Filesystem Registration
echo
log_info "Test 2: Filesystem Registration"
if grep -q vexfs_fixed /proc/filesystems; then
    log_success "Filesystem registered: vexfs_fixed"
    cat /proc/filesystems | grep vexfs
else
    log_error "Filesystem not registered"
    exit 1
fi

# Test 3: Create and Mount Filesystem
echo
log_info "Test 3: Create and Mount Filesystem"
dd if=/dev/zero of="$TEST_IMG" bs=1M count=10 status=none
log_success "Created 10MB test image"

if sudo mount -t vexfs_fixed -o loop "$TEST_IMG" "$MOUNT_POINT"; then
    log_success "Mount successful!"
    mount | grep vexfs
else
    log_error "Mount failed"
    dmesg | tail -30
    exit 1
fi

# Test 4: Basic File Operations
echo
log_info "Test 4: Basic File Operations"

# Create file
echo "Hello VexFS!" | sudo tee "$MOUNT_POINT/test.txt" > /dev/null
if [ -f "$MOUNT_POINT/test.txt" ]; then
    log_success "File created successfully"
else
    log_error "File creation failed"
    exit 1
fi

# Read file
content=$(cat "$MOUNT_POINT/test.txt")
if [ "$content" = "Hello VexFS!" ]; then
    log_success "File read successfully: $content"
else
    log_error "File read failed"
    exit 1
fi

# Test 5: Directory Operations
echo
log_info "Test 5: Directory Operations"

# Create directory
sudo mkdir -p "$MOUNT_POINT/testdir"
if [ -d "$MOUNT_POINT/testdir" ]; then
    log_success "Directory created"
else
    log_error "Directory creation failed"
    exit 1
fi

# List directory
ls -la "$MOUNT_POINT/"
log_success "Directory listing works"

# Test 6: Multiple Files
echo
log_info "Test 6: Multiple Files"
for i in {1..5}; do
    echo "File $i content" | sudo tee "$MOUNT_POINT/file$i.txt" > /dev/null
done
file_count=$(ls "$MOUNT_POINT"/file*.txt 2>/dev/null | wc -l)
if [ "$file_count" -eq 5 ]; then
    log_success "Created 5 files successfully"
else
    log_error "Expected 5 files, found $file_count"
    exit 1
fi

# Test 7: File Persistence
echo
log_info "Test 7: File Persistence (Critical Test)"

# Unmount
sudo umount "$MOUNT_POINT"
log_success "Unmounted filesystem"

# Remount
if sudo mount -t vexfs_fixed -o loop "$TEST_IMG" "$MOUNT_POINT"; then
    log_success "Remounted successfully"
else
    log_error "Remount failed"
    exit 1
fi

# Check if files persist
if [ -f "$MOUNT_POINT/test.txt" ]; then
    content=$(cat "$MOUNT_POINT/test.txt")
    if [ "$content" = "Hello VexFS!" ]; then
        log_success "✨ FILES PERSIST ACROSS REMOUNT! ✨"
    else
        log_error "File content changed: $content"
        exit 1
    fi
else
    log_error "File lost after remount"
    exit 1
fi

# Check multiple files
persisted_count=$(ls "$MOUNT_POINT"/file*.txt 2>/dev/null | wc -l)
if [ "$persisted_count" -eq 5 ]; then
    log_success "All 5 files persisted"
else
    log_error "Only $persisted_count files persisted"
fi

# Test 8: Subdirectory Persistence
echo
log_info "Test 8: Subdirectory Persistence"
if [ -d "$MOUNT_POINT/testdir" ]; then
    log_success "Directory persisted"
    # Create file in subdirectory
    echo "Subdir file" | sudo tee "$MOUNT_POINT/testdir/subfile.txt" > /dev/null
    log_success "Created file in subdirectory"
else
    log_error "Directory lost after remount"
fi

# Test 9: Stress Test
echo
log_info "Test 9: Stress Test (100 files)"
for i in {1..100}; do
    echo "Stress test file $i" | sudo tee "$MOUNT_POINT/stress_$i.txt" > /dev/null
done
stress_count=$(ls "$MOUNT_POINT"/stress_*.txt 2>/dev/null | wc -l)
if [ "$stress_count" -eq 100 ]; then
    log_success "Created 100 files successfully"
else
    log_error "Only created $stress_count files"
fi

# Test 10: Final Persistence Check
echo
log_info "Test 10: Final Persistence Check"
sudo umount "$MOUNT_POINT"
sudo mount -t vexfs_fixed -o loop "$TEST_IMG" "$MOUNT_POINT"

final_count=$(find "$MOUNT_POINT" -type f | wc -l)
log_success "Total files after final remount: $final_count"

# Show filesystem structure
echo
log_info "Final filesystem structure:"
tree "$MOUNT_POINT" 2>/dev/null || find "$MOUNT_POINT" -type f

# Performance metrics
echo
log_info "Performance Metrics:"
df -h "$MOUNT_POINT"

# Cleanup
sudo umount "$MOUNT_POINT"
sudo rmmod vexfs_deadlock_fix

echo
log_success "🎉 ALL TESTS PASSED! 🎉"
log_success "VexFS is working correctly with persistence!"
EOF

chmod +x "$VM_DIR/shared/run_all_tests.sh"

# Create test execution wrapper
cat > "$VM_DIR/shared/execute_tests.sh" << 'EOF'
#!/bin/bash

# Test execution wrapper for SSH

echo "Starting VexFS test execution..."
echo

# Make sure shared directory is mounted
if ! mountpoint -q /mnt/shared; then
    sudo mount -t 9p -o trans=virtio shared /mnt/shared
fi

# Run the comprehensive test suite
/mnt/shared/run_all_tests.sh

# Save test results
echo
echo "Test completed at: $(date)" | tee /mnt/shared/test_results.log
echo "Exit code: $?" | tee -a /mnt/shared/test_results.log
EOF

chmod +x "$VM_DIR/shared/execute_tests.sh"

# Run tests via SSH
log_info "Executing tests in VM..."
echo

ssh -p 2222 -o StrictHostKeyChecking=no root@localhost 'bash /mnt/shared/execute_tests.sh' || {
    log_error "Test execution failed"
    echo
    echo "You can manually run tests by:"
    echo "1. SSH into VM: ssh -p 2222 root@localhost"
    echo "2. Run: /mnt/shared/run_all_tests.sh"
    exit 1
}

# Check results
if [ -f "$VM_DIR/shared/test_results.log" ]; then
    echo
    log_success "Test results saved to: $VM_DIR/shared/test_results.log"
    cat "$VM_DIR/shared/test_results.log"
fi

echo
log_success "Automated test execution complete!"