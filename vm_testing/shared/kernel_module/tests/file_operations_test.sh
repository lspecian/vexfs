#!/bin/bash

# VexFS Phase 1 File Operations Test
# Tests basic file I/O operations: create, write, read, persistence

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
KERNEL_MODULE_DIR="$(dirname "$SCRIPT_DIR")"
TEST_IMAGE="/tmp/vexfs_file_test.img"
LOOP_DEVICE="$(sudo losetup -f)"  # Find first available loop device
MOUNT_POINT="/mnt/vexfs_test"
TEST_FILE="$MOUNT_POINT/test_file.txt"
LARGE_FILE="$MOUNT_POINT/large_file.dat"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

cleanup() {
    log_info "Cleaning up test environment..."
    
    # Unmount if mounted
    if mountpoint -q "$MOUNT_POINT" 2>/dev/null; then
        sudo umount "$MOUNT_POINT" || log_warning "Failed to unmount $MOUNT_POINT"
    fi
    
    # Detach loop device
    if losetup "$LOOP_DEVICE" 2>/dev/null; then
        sudo losetup -d "$LOOP_DEVICE" || log_warning "Failed to detach $LOOP_DEVICE"
    fi
    
    # Remove test files
    rm -f "$TEST_IMAGE"
    sudo rmdir "$MOUNT_POINT" 2>/dev/null || true
    
    # Unload module
    if lsmod | grep -q "^vexfs_deadlock_fix "; then
        sudo rmmod vexfs_deadlock_fix || log_warning "Failed to unload vexfs_deadlock_fix module"
    fi
}

# Set up cleanup on exit
trap cleanup EXIT

test_module_build() {
    log_info "Building VexFS kernel module..."
    cd "$KERNEL_MODULE_DIR"
    
    if ! make clean; then
        log_error "Failed to clean module"
        return 1
    fi
    
    if ! make all; then
        log_error "Failed to build module"
        return 1
    fi
    
    if [ ! -f "vexfs_deadlock_fix.ko" ]; then
        log_error "Module vexfs_deadlock_fix.ko not found after build"
        return 1
    fi
    
    log_success "Module built successfully"
    return 0
}

test_module_load() {
    log_info "Loading VexFS kernel module..."
    
    # Unload if already loaded
    if lsmod | grep -q "^vexfs_deadlock_fix "; then
        sudo rmmod vexfs_deadlock_fix
    fi
    
    if ! sudo insmod vexfs_deadlock_fix.ko; then
        log_error "Failed to load vexfs_deadlock_fix module"
        return 1
    fi
    
    if ! lsmod | grep -q "^vexfs_deadlock_fix "; then
        log_error "Module not found in lsmod after loading"
        return 1
    fi
    
    log_success "Module loaded successfully"
    return 0
}

test_filesystem_setup() {
    log_info "Setting up test filesystem..."
    
    # Create test image
    if ! dd if=/dev/zero of="$TEST_IMAGE" bs=1M count=50 2>/dev/null; then
        log_error "Failed to create test image"
        return 1
    fi
    
    # Setup loop device
    if ! sudo losetup "$LOOP_DEVICE" "$TEST_IMAGE"; then
        log_error "Failed to setup loop device"
        return 1
    fi
    
    # Format with VexFS
    cd "$KERNEL_MODULE_DIR/../tools"
    if ! make; then
        log_error "Failed to build mkfs.vexfs"
        return 1
    fi
    
    if ! sudo ./mkfs.vexfs "$LOOP_DEVICE"; then
        log_error "Failed to format with VexFS"
        return 1
    fi
    
    # Create mount point
    sudo mkdir -p "$MOUNT_POINT"
    
    # Mount filesystem
    if ! sudo mount -t vexfs_fixed "$LOOP_DEVICE" "$MOUNT_POINT"; then
        log_error "Failed to mount VexFS"
        return 1
    fi
    
    if ! mountpoint -q "$MOUNT_POINT"; then
        log_error "Mount point not properly mounted"
        return 1
    fi
    
    log_success "Filesystem setup completed"
    return 0
}

test_basic_file_operations() {
    log_info "Testing basic file operations..."
    
    # Test file creation
    if ! sudo touch "$TEST_FILE"; then
        log_error "Failed to create test file"
        return 1
    fi
    
    if [ ! -f "$TEST_FILE" ]; then
        log_error "Test file not found after creation"
        return 1
    fi
    
    log_success "File creation: PASSED"
    
    # Test writing to file
    local test_content="Hello VexFS! This is a test of file write operations."
    if ! echo "$test_content" | sudo tee "$TEST_FILE" > /dev/null; then
        log_error "Failed to write to test file"
        return 1
    fi
    
    log_success "File writing: PASSED"
    
    # Test reading from file
    local read_content
    if ! read_content=$(sudo cat "$TEST_FILE"); then
        log_error "Failed to read from test file"
        return 1
    fi
    
    if [ "$read_content" != "$test_content" ]; then
        log_error "Read content doesn't match written content"
        log_error "Expected: '$test_content'"
        log_error "Got: '$read_content'"
        return 1
    fi
    
    log_success "File reading: PASSED"
    
    # Test file size
    local file_size
    file_size=$(sudo stat -c%s "$TEST_FILE")
    local expected_size=$((${#test_content} + 1)) # +1 for newline
    
    if [ "$file_size" -ne "$expected_size" ]; then
        log_error "File size mismatch. Expected: $expected_size, Got: $file_size"
        return 1
    fi
    
    log_success "File size verification: PASSED"
    
    return 0
}

test_large_file_operations() {
    log_info "Testing large file operations..."
    
    # Create a 1MB test file
    local test_size=1048576  # 1MB
    if ! sudo dd if=/dev/urandom of="$LARGE_FILE" bs=1024 count=1024 2>/dev/null; then
        log_error "Failed to create large test file"
        return 1
    fi
    
    if [ ! -f "$LARGE_FILE" ]; then
        log_error "Large test file not found after creation"
        return 1
    fi
    
    # Verify file size
    local file_size
    file_size=$(sudo stat -c%s "$LARGE_FILE")
    
    if [ "$file_size" -ne "$test_size" ]; then
        log_error "Large file size mismatch. Expected: $test_size, Got: $file_size"
        return 1
    fi
    
    log_success "Large file creation: PASSED"
    
    # Test reading the large file
    local checksum1
    checksum1=$(sudo md5sum "$LARGE_FILE" | cut -d' ' -f1)
    
    # Copy and verify
    if ! sudo cp "$LARGE_FILE" "$LARGE_FILE.copy"; then
        log_error "Failed to copy large file"
        return 1
    fi
    
    local checksum2
    checksum2=$(sudo md5sum "$LARGE_FILE.copy" | cut -d' ' -f1)
    
    if [ "$checksum1" != "$checksum2" ]; then
        log_error "Large file copy checksum mismatch"
        log_error "Original: $checksum1"
        log_error "Copy: $checksum2"
        return 1
    fi
    
    log_success "Large file operations: PASSED"
    
    return 0
}

test_persistence() {
    log_info "Testing data persistence across mount/unmount..."
    
    # Create test data
    local persistence_file="$MOUNT_POINT/persistence_test.txt"
    local test_data="This data should persist across mount/unmount cycles."
    
    if ! echo "$test_data" | sudo tee "$persistence_file" > /dev/null; then
        log_error "Failed to create persistence test file"
        return 1
    fi
    
    # Get checksum before unmount
    local checksum_before
    checksum_before=$(sudo md5sum "$persistence_file" | cut -d' ' -f1)
    
    # Unmount
    if ! sudo umount "$MOUNT_POINT"; then
        log_error "Failed to unmount for persistence test"
        return 1
    fi
    
    # Remount
    if ! sudo mount -t vexfs_fixed "$LOOP_DEVICE" "$MOUNT_POINT"; then
        log_error "Failed to remount for persistence test"
        return 1
    fi
    
    # Verify file still exists
    if [ ! -f "$persistence_file" ]; then
        log_error "Persistence test file not found after remount"
        return 1
    fi
    
    # Verify content
    local read_data
    if ! read_data=$(sudo cat "$persistence_file"); then
        log_error "Failed to read persistence test file after remount"
        return 1
    fi
    
    if [ "$read_data" != "$test_data" ]; then
        log_error "Persistence test data mismatch"
        log_error "Expected: '$test_data'"
        log_error "Got: '$read_data'"
        return 1
    fi
    
    # Verify checksum
    local checksum_after
    checksum_after=$(sudo md5sum "$persistence_file" | cut -d' ' -f1)
    
    if [ "$checksum_before" != "$checksum_after" ]; then
        log_error "Persistence test checksum mismatch"
        log_error "Before: $checksum_before"
        log_error "After: $checksum_after"
        return 1
    fi
    
    log_success "Data persistence: PASSED"
    
    return 0
}

test_multiple_files() {
    log_info "Testing multiple file operations..."
    
    local num_files=10
    local base_name="$MOUNT_POINT/multi_test"
    
    # Create multiple files
    for i in $(seq 1 $num_files); do
        local filename="${base_name}_${i}.txt"
        local content="This is test file number $i with some unique content."
        
        if ! echo "$content" | sudo tee "$filename" > /dev/null; then
            log_error "Failed to create file $filename"
            return 1
        fi
    done
    
    # Verify all files
    for i in $(seq 1 $num_files); do
        local filename="${base_name}_${i}.txt"
        local expected_content="This is test file number $i with some unique content."
        
        if [ ! -f "$filename" ]; then
            log_error "File $filename not found"
            return 1
        fi
        
        local actual_content
        if ! actual_content=$(sudo cat "$filename"); then
            log_error "Failed to read file $filename"
            return 1
        fi
        
        if [ "$actual_content" != "$expected_content" ]; then
            log_error "Content mismatch in file $filename"
            return 1
        fi
    done
    
    log_success "Multiple file operations: PASSED"
    
    return 0
}

run_all_tests() {
    log_info "Starting VexFS Phase 1 File Operations Test Suite"
    log_info "=============================================="
    
    local tests=(
        "test_module_build"
        "test_module_load" 
        "test_filesystem_setup"
        "test_basic_file_operations"
        "test_large_file_operations"
        "test_persistence"
        "test_multiple_files"
    )
    
    local passed=0
    local total=${#tests[@]}
    
    for test in "${tests[@]}"; do
        log_info "Running $test..."
        if $test; then
            ((passed++))
        else
            log_error "Test $test FAILED"
            return 1
        fi
        echo
    done
    
    log_success "=============================================="
    log_success "All tests passed! ($passed/$total)"
    log_success "VexFS Phase 1 Core File Operations: WORKING"
    log_success "=============================================="
    
    return 0
}

# Check if running as root or with sudo
if [ "$EUID" -eq 0 ]; then
    log_warning "Running as root. This is required for kernel module operations."
fi

# Run the test suite
if run_all_tests; then
    exit 0
else
    log_error "Test suite failed!"
    exit 1
fi