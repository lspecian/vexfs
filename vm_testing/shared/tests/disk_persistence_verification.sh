#!/bin/bash

# VexFS Disk Persistence Verification Test Suite
# Task 33.1: Create Mandatory Verification Test Suite
# 
# This script performs comprehensive testing of VexFS disk persistence
# to verify that the Phase 1 implementation actually provides real
# disk-backed storage with data integrity across unmount/remount cycles.

set -e  # Exit on any error

# Configuration
TEST_DIR="/tmp/vexfs_persistence_test"
LOOP_DEVICE=""
LOOP_FILE="${TEST_DIR}/vexfs_test.img"
MOUNT_POINT="${TEST_DIR}/mount"
RESULTS_DIR="${TEST_DIR}/results"
LOG_FILE="${RESULTS_DIR}/verification.log"

# Test file sizes (in bytes)
declare -A TEST_SIZES=(
    ["small"]="4096"        # 4KB
    ["medium"]="1048576"    # 1MB  
    ["large"]="104857600"   # 100MB
    ["xlarge"]="1073741824" # 1GB
)

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging function
log() {
    echo -e "${BLUE}[$(date '+%Y-%m-%d %H:%M:%S')]${NC} $1" | tee -a "$LOG_FILE"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1" | tee -a "$LOG_FILE"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1" | tee -a "$LOG_FILE"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1" | tee -a "$LOG_FILE"
}

# Cleanup function
cleanup() {
    log "Starting cleanup..."
    
    # Unmount if mounted
    if mountpoint -q "$MOUNT_POINT" 2>/dev/null; then
        log "Unmounting $MOUNT_POINT"
        sudo umount "$MOUNT_POINT" || log_warning "Failed to unmount $MOUNT_POINT"
    fi
    
    # Detach loop device if attached
    if [ -n "$LOOP_DEVICE" ] && [ -e "$LOOP_DEVICE" ]; then
        log "Detaching loop device $LOOP_DEVICE"
        sudo losetup -d "$LOOP_DEVICE" || log_warning "Failed to detach $LOOP_DEVICE"
    fi
    
    log "Cleanup completed"
}

# Set trap for cleanup
trap cleanup EXIT

# Check prerequisites
check_prerequisites() {
    log "Checking prerequisites..."
    
    # Check if running as root or with sudo
    if [ "$EUID" -eq 0 ]; then
        log_warning "Running as root - this is acceptable for testing"
    elif ! sudo -n true 2>/dev/null; then
        log_error "This script requires sudo privileges"
        exit 1
    fi
    
    # Check if VexFS module is loaded
    if ! lsmod | grep -q vexfs_deadlock_fix; then
        log_error "VexFS kernel module is not loaded"
        log "Please load the module with: sudo insmod vexfs.ko"
        exit 1
    fi
    
    # Check if VexFS is registered
    if ! grep -q vexfs /proc/filesystems; then
        log_error "VexFS is not registered in /proc/filesystems"
        exit 1
    fi
    
    # Check required tools
    for tool in losetup mkfs dd sha256sum; do
        if ! command -v "$tool" &> /dev/null; then
            log_error "Required tool '$tool' is not available"
            exit 1
        fi
    done
    
    log_success "All prerequisites met"
}

# Setup test environment
setup_test_environment() {
    log "Setting up test environment..."
    
    # Create test directories
    mkdir -p "$TEST_DIR" "$MOUNT_POINT" "$RESULTS_DIR"
    
    # Create loop file (1.5GB to accommodate all test files)
    log "Creating 1.5GB loop file: $LOOP_FILE"
    dd if=/dev/zero of="$LOOP_FILE" bs=1M count=1536 status=progress
    
    # Setup loop device
    LOOP_DEVICE=$(sudo losetup -f --show "$LOOP_FILE")
    log "Loop device created: $LOOP_DEVICE"
    
    # Format with VexFS
    log "Formatting $LOOP_DEVICE with VexFS..."
    
    # Check if mkfs.vexfs exists, if not we'll need to create a simple formatter
    if ! command -v mkfs.vexfs &> /dev/null; then
        log_warning "mkfs.vexfs not found, creating basic VexFS superblock..."
        create_vexfs_superblock "$LOOP_DEVICE"
    else
        sudo mkfs.vexfs "$LOOP_DEVICE"
    fi
    
    log_success "Test environment setup completed"
}

# Create basic VexFS superblock if mkfs.vexfs doesn't exist
create_vexfs_superblock() {
    local device="$1"
    log "Creating basic VexFS superblock on $device"
    
    # Create a simple superblock structure
    # This is a minimal implementation for testing
    cat > "${TEST_DIR}/create_sb.c" << 'EOF'
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <fcntl.h>
#include <sys/types.h>
#include <stdint.h>

#define VEXFS_MAGIC 0x56455846  // "VEXF"
#define VEXFS_BLOCK_SIZE 4096

struct vexfs_super_block {
    uint32_t s_magic;           /* Magic number */
    uint32_t s_block_size;      /* Block size */
    uint32_t s_blocks_count;    /* Total blocks */
    uint32_t s_free_blocks;     /* Free blocks */
    uint32_t s_inodes_count;    /* Total inodes */
    uint32_t s_free_inodes;     /* Free inodes */
    uint32_t s_first_data_block; /* First data block */
    uint32_t s_log_block_size;  /* Block size = 1024 << s_log_block_size */
    uint32_t s_blocks_per_group; /* Blocks per group */
    uint32_t s_inodes_per_group; /* Inodes per group */
    uint32_t s_mtime;           /* Mount time */
    uint32_t s_wtime;           /* Write time */
    uint16_t s_mnt_count;       /* Mount count */
    uint16_t s_max_mnt_count;   /* Maximum mount count */
    uint16_t s_state;           /* Filesystem state */
    uint16_t s_errors;          /* Error handling */
    uint16_t s_minor_rev_level; /* Minor revision level */
    uint32_t s_lastcheck;       /* Last check time */
    uint32_t s_checkinterval;   /* Check interval */
    uint32_t s_creator_os;      /* Creator OS */
    uint32_t s_rev_level;       /* Revision level */
    uint16_t s_def_resuid;      /* Default reserved user ID */
    uint16_t s_def_resgid;      /* Default reserved group ID */
    uint32_t s_first_ino;       /* First non-reserved inode */
    uint16_t s_inode_size;      /* Inode size */
    uint16_t s_block_group_nr;  /* Block group number */
    char padding[3968];         /* Pad to 4096 bytes */
};

int main(int argc, char *argv[]) {
    if (argc != 2) {
        fprintf(stderr, "Usage: %s <device>\n", argv[0]);
        return 1;
    }
    
    int fd = open(argv[1], O_WRONLY);
    if (fd < 0) {
        perror("open");
        return 1;
    }
    
    struct vexfs_super_block sb = {0};
    sb.s_magic = VEXFS_MAGIC;
    sb.s_block_size = VEXFS_BLOCK_SIZE;
    sb.s_blocks_count = 393216;  // 1.5GB / 4KB
    // Reserved blocks: 1 superblock + 1 bitmap + 64 inode table = 66 blocks
    sb.s_free_blocks = 393216 - 66;   // Total minus reserved blocks
    sb.s_inodes_count = 8192;    // Standard inode count
    sb.s_free_inodes = 8191;     // All except root inode
    sb.s_first_data_block = 66;  // First data block after reserved blocks
    sb.s_inode_size = 256;       // Size of each inode
    sb.s_first_ino = 11;         // First non-reserved inode (like ext2)
    
    if (write(fd, &sb, sizeof(sb)) != sizeof(sb)) {
        perror("write");
        close(fd);
        return 1;
    }
    
    close(fd);
    printf("VexFS superblock created successfully\n");
    return 0;
}
EOF
    
    # Compile and run the superblock creator
    gcc -o "${TEST_DIR}/create_sb" "${TEST_DIR}/create_sb.c"
    sudo "${TEST_DIR}/create_sb" "$device"
}

# Generate test file with known content
generate_test_file() {
    local size="$1"
    local filename="$2"
    local content_pattern="$3"
    
    log "Generating test file: $filename (size: $size bytes)"
    
    # Create file with repeating pattern
    if [ "$size" -le 1048576 ]; then
        # For smaller files, use a text pattern
        yes "$content_pattern" | head -c "$size" > "$filename"
    else
        # For larger files, use dd with pattern
        dd if=/dev/zero of="$filename" bs=1024 count=$((size/1024)) 2>/dev/null
        # Overwrite with pattern every 1KB
        for ((i=0; i<size; i+=1024)); do
            echo -n "$content_pattern" | dd of="$filename" bs=1 seek=$i conv=notrunc 2>/dev/null || true
        done
    fi
    
    # Calculate and store checksum
    local checksum=$(sha256sum "$filename" | cut -d' ' -f1)
    echo "$checksum" > "${filename}.sha256"
    
    log "Generated $filename with checksum: $checksum"
}

# Mount VexFS filesystem
mount_vexfs() {
    log "Mounting VexFS filesystem..."
    
    # Mount the filesystem
    if sudo mount -t vexfs_fixed "$LOOP_DEVICE" "$MOUNT_POINT"; then
        log_success "VexFS mounted successfully at $MOUNT_POINT"
        
        # Verify mount
        if mountpoint -q "$MOUNT_POINT"; then
            log_success "Mount point verified"
            mount | grep vexfs_fixed | tee -a "$LOG_FILE"
        else
            log_error "Mount verification failed"
            return 1
        fi
    else
        log_error "Failed to mount VexFS"
        return 1
    fi
}

# Unmount VexFS filesystem
unmount_vexfs() {
    log "Unmounting VexFS filesystem..."
    
    # Sync before unmount
    sync
    
    if sudo umount "$MOUNT_POINT"; then
        log_success "VexFS unmounted successfully"
    else
        log_error "Failed to unmount VexFS"
        return 1
    fi
}

# Create test files on mounted filesystem
create_test_files() {
    log "Creating test files on VexFS..."
    
    # Create directory structure
    sudo mkdir -p "$MOUNT_POINT/test_data"
    sudo mkdir -p "$MOUNT_POINT/test_data/nested/deep/structure"
    
    # Generate test files of various sizes
    for size_name in "${!TEST_SIZES[@]}"; do
        local size="${TEST_SIZES[$size_name]}"
        local filename="$MOUNT_POINT/test_data/test_${size_name}_${size}.dat"
        local temp_file="${RESULTS_DIR}/temp_${size_name}.dat"
        
        log "Creating test file: test_${size_name}_${size}.dat"
        
        # Generate file in temp location first
        generate_test_file "$size" "$temp_file" "VexFS_Test_Pattern_${size_name}_"
        
        # Copy to mounted filesystem
        sudo cp "$temp_file" "$filename"
        sudo cp "${temp_file}.sha256" "${filename}.sha256"
        
        # Verify immediate read
        if sudo sha256sum -c "${filename}.sha256" > /dev/null 2>&1; then
            log_success "Immediate verification passed for $filename"
        else
            log_error "Immediate verification failed for $filename"
            return 1
        fi
    done
    
    # Create some text files with known content
    echo "VexFS Persistence Test - $(date)" | sudo tee "$MOUNT_POINT/test_data/timestamp.txt" > /dev/null
    echo "This file tests basic text persistence" | sudo tee "$MOUNT_POINT/test_data/simple.txt" > /dev/null
    
    # Create nested file
    echo "Nested directory test" | sudo tee "$MOUNT_POINT/test_data/nested/deep/structure/nested_file.txt" > /dev/null
    
    # Sync to ensure data is written
    sync
    
    log_success "All test files created successfully"
}

# Verify test files exist and have correct content
verify_test_files() {
    local test_phase="$1"
    log "Verifying test files ($test_phase)..."
    
    local verification_passed=true
    
    # Check basic files
    for file in "timestamp.txt" "simple.txt" "nested/deep/structure/nested_file.txt"; do
        if [ -f "$MOUNT_POINT/test_data/$file" ]; then
            log_success "File exists: $file"
        else
            log_error "File missing: $file"
            verification_passed=false
        fi
    done
    
    # Check test data files with checksums
    for size_name in "${!TEST_SIZES[@]}"; do
        local size="${TEST_SIZES[$size_name]}"
        local filename="$MOUNT_POINT/test_data/test_${size_name}_${size}.dat"
        local checksum_file="${filename}.sha256"
        
        if [ -f "$filename" ] && [ -f "$checksum_file" ]; then
            if sudo sha256sum -c "$checksum_file" > /dev/null 2>&1; then
                log_success "Checksum verification passed: test_${size_name}_${size}.dat"
            else
                log_error "Checksum verification failed: test_${size_name}_${size}.dat"
                verification_passed=false
            fi
        else
            log_error "Missing file or checksum: test_${size_name}_${size}.dat"
            verification_passed=false
        fi
    done
    
    if [ "$verification_passed" = true ]; then
        log_success "All files verified successfully ($test_phase)"
        return 0
    else
        log_error "File verification failed ($test_phase)"
        return 1
    fi
}

# Run persistence test cycle
run_persistence_test() {
    log "=== STARTING DISK PERSISTENCE TEST ==="
    
    # Phase 1: Mount and create files
    log "Phase 1: Creating test files..."
    mount_vexfs || return 1
    create_test_files || return 1
    verify_test_files "Initial Creation" || return 1
    unmount_vexfs || return 1
    
    # Phase 2: Remount and verify persistence
    log "Phase 2: Testing persistence after unmount/remount..."
    mount_vexfs || return 1
    verify_test_files "After Remount" || return 1
    
    # Phase 3: Multiple unmount/remount cycles
    for cycle in {1..3}; do
        log "Phase 3.$cycle: Unmount/remount cycle $cycle..."
        unmount_vexfs || return 1
        sleep 1
        mount_vexfs || return 1
        verify_test_files "Cycle $cycle" || return 1
    done
    
    unmount_vexfs || return 1
    
    log_success "=== DISK PERSISTENCE TEST COMPLETED SUCCESSFULLY ==="
}

# Generate comprehensive report
generate_report() {
    local report_file="${RESULTS_DIR}/persistence_verification_report.md"
    
    log "Generating comprehensive verification report..."
    
    cat > "$report_file" << EOF
# VexFS Disk Persistence Verification Report

**Test Date:** $(date)
**Test Duration:** $(date -d@$(($(date +%s) - START_TIME)) -u +%H:%M:%S)
**VexFS Version:** 2.0.0

## Test Environment

- **Loop Device:** $LOOP_DEVICE
- **Loop File:** $LOOP_FILE (1.5GB)
- **Mount Point:** $MOUNT_POINT
- **Kernel Version:** $(uname -r)
- **VexFS Module:** $(lsmod | grep vexfs || echo "Not loaded")

## Test Results Summary

### File Persistence Tests

| File Size | Status | Checksum Verification |
|-----------|--------|----------------------|
EOF

    for size_name in "${!TEST_SIZES[@]}"; do
        local size="${TEST_SIZES[$size_name]}"
        echo "| ${size_name} (${size} bytes) | ✅ PASSED | ✅ VERIFIED |" >> "$report_file"
    done

    cat >> "$report_file" << EOF

### Unmount/Remount Cycles

- **Initial Creation:** ✅ PASSED
- **After First Remount:** ✅ PASSED
- **Cycle 1:** ✅ PASSED
- **Cycle 2:** ✅ PASSED
- **Cycle 3:** ✅ PASSED

### Directory Structure Tests

- **Root Directory:** ✅ PASSED
- **Nested Directories:** ✅ PASSED
- **Deep Directory Structure:** ✅ PASSED

## Detailed Test Log

\`\`\`
$(cat "$LOG_FILE")
\`\`\`

## Conclusion

**RESULT: ✅ DISK PERSISTENCE VERIFIED**

The VexFS kernel module Phase 1 implementation successfully provides:

1. **Real disk-backed storage** using block devices
2. **Data persistence** across unmount/remount cycles
3. **Data integrity** verified with SHA-256 checksums
4. **Support for various file sizes** from 4KB to 1GB
5. **Directory structure persistence**
6. **Multiple unmount/remount cycle stability**

The filesystem demonstrates true disk persistence capabilities required for Phase 2 development.
EOF

    log_success "Report generated: $report_file"
}

# Main execution
main() {
    START_TIME=$(date +%s)
    
    log "=== VexFS DISK PERSISTENCE VERIFICATION TEST SUITE ==="
    log "Task 33.1: Create Mandatory Verification Test Suite"
    log "Testing VexFS Phase 1 Implementation"
    
    check_prerequisites
    setup_test_environment
    run_persistence_test
    generate_report
    
    log_success "=== ALL TESTS COMPLETED SUCCESSFULLY ==="
    log "Verification report available at: ${RESULTS_DIR}/persistence_verification_report.md"
}

# Run main function
main "$@"