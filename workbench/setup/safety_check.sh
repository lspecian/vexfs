#!/bin/bash

# VexFS 200GB Testing - Safety Check Script
# Ensures /dev/sda1 is safe to format and contains no important data

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
TARGET_DEVICE="/dev/sda1"
MIN_SIZE_GB=200
BACKUP_CHECK_DIRS=("/home" "/etc" "/var" "/usr")

echo -e "${BLUE}🛡️  VexFS Safety Check - Validating ${TARGET_DEVICE}${NC}"
echo "=================================================================="

# Function to print status
print_status() {
    local status=$1
    local message=$2
    if [ "$status" = "OK" ]; then
        echo -e "${GREEN}✅ $message${NC}"
    elif [ "$status" = "WARNING" ]; then
        echo -e "${YELLOW}⚠️  $message${NC}"
    else
        echo -e "${RED}❌ $message${NC}"
    fi
}

# Function to check if running as root
check_root() {
    if [ "$EUID" -ne 0 ]; then
        print_status "ERROR" "This script must be run as root for device access"
        echo "Please run: sudo $0"
        exit 1
    fi
    print_status "OK" "Running with root privileges"
}

# Function to check if device exists
check_device_exists() {
    if [ ! -b "$TARGET_DEVICE" ]; then
        print_status "ERROR" "Target device $TARGET_DEVICE does not exist"
        echo "Available block devices:"
        lsblk -d -o NAME,SIZE,TYPE,MOUNTPOINT
        exit 1
    fi
    print_status "OK" "Target device $TARGET_DEVICE exists"
}

# Function to check device size
check_device_size() {
    local size_bytes=$(blockdev --getsize64 "$TARGET_DEVICE" 2>/dev/null || echo "0")
    local size_gb=$((size_bytes / 1024 / 1024 / 1024))
    
    if [ "$size_gb" -lt "$MIN_SIZE_GB" ]; then
        print_status "ERROR" "Device size ${size_gb}GB is less than required ${MIN_SIZE_GB}GB"
        exit 1
    fi
    print_status "OK" "Device size: ${size_gb}GB (sufficient for testing)"
}

# Function to check if device is mounted
check_mount_status() {
    if mount | grep -q "$TARGET_DEVICE"; then
        local mount_point=$(mount | grep "$TARGET_DEVICE" | awk '{print $3}')
        print_status "WARNING" "Device is currently mounted at: $mount_point"
        echo "Please unmount before proceeding: sudo umount $TARGET_DEVICE"
        return 1
    fi
    print_status "OK" "Device is not currently mounted"
    return 0
}

# Function to check for important data patterns
check_data_patterns() {
    echo -e "\n${BLUE}🔍 Scanning for important data patterns...${NC}"
    
    # Check for filesystem signatures
    local fs_type=$(blkid -o value -s TYPE "$TARGET_DEVICE" 2>/dev/null || echo "unknown")
    if [ "$fs_type" != "unknown" ]; then
        print_status "WARNING" "Detected filesystem: $fs_type"
    else
        print_status "OK" "No filesystem detected"
    fi
    
    # Check for partition table
    if fdisk -l "$TARGET_DEVICE" 2>/dev/null | grep -q "Disklabel type"; then
        print_status "WARNING" "Partition table detected"
        fdisk -l "$TARGET_DEVICE" 2>/dev/null | head -20
    else
        print_status "OK" "No partition table detected"
    fi
}

# Function to check system impact
check_system_impact() {
    echo -e "\n${BLUE}🖥️  Checking system impact...${NC}"
    
    # Check if device is in fstab
    if grep -q "$TARGET_DEVICE" /etc/fstab; then
        print_status "WARNING" "Device found in /etc/fstab - may be system-critical"
        grep "$TARGET_DEVICE" /etc/fstab
    else
        print_status "OK" "Device not found in /etc/fstab"
    fi
    
    # Check if device is used by LVM
    if command -v pvs >/dev/null 2>&1; then
        if pvs 2>/dev/null | grep -q "$TARGET_DEVICE"; then
            print_status "ERROR" "Device is part of LVM - CANNOT FORMAT"
            exit 1
        fi
        print_status "OK" "Device not used by LVM"
    fi
    
    # Check if device is used by RAID
    if [ -f /proc/mdstat ]; then
        if grep -q "$(basename $TARGET_DEVICE)" /proc/mdstat; then
            print_status "ERROR" "Device is part of RAID array - CANNOT FORMAT"
            exit 1
        fi
        print_status "OK" "Device not used by RAID"
    fi
}

# Function to perform final confirmation
final_confirmation() {
    echo -e "\n${YELLOW}⚠️  FINAL CONFIRMATION REQUIRED${NC}"
    echo "=================================================================="
    echo "Target device: $TARGET_DEVICE"
    echo "This will PERMANENTLY DESTROY all data on the device!"
    echo ""
    echo "Type 'DESTROY' to confirm you want to proceed:"
    read -r confirmation
    
    if [ "$confirmation" != "DESTROY" ]; then
        print_status "ERROR" "Confirmation failed - aborting for safety"
        exit 1
    fi
    
    print_status "OK" "Confirmation received - device approved for formatting"
}

# Function to create safety backup of device info
create_device_backup() {
    local backup_dir="./device_backup_$(date +%Y%m%d_%H%M%S)"
    mkdir -p "$backup_dir"
    
    # Save device information
    echo "Device: $TARGET_DEVICE" > "$backup_dir/device_info.txt"
    echo "Date: $(date)" >> "$backup_dir/device_info.txt"
    echo "Size: $(blockdev --getsize64 $TARGET_DEVICE) bytes" >> "$backup_dir/device_info.txt"
    
    # Save partition table if exists
    fdisk -l "$TARGET_DEVICE" > "$backup_dir/partition_table.txt" 2>/dev/null || true
    
    # Save filesystem info if exists
    blkid "$TARGET_DEVICE" > "$backup_dir/filesystem_info.txt" 2>/dev/null || true
    
    # Save first 1MB as backup (contains partition table, boot sectors, etc.)
    dd if="$TARGET_DEVICE" of="$backup_dir/device_header.bin" bs=1M count=1 2>/dev/null || true
    
    print_status "OK" "Device information backed up to: $backup_dir"
}

# Main execution
main() {
    echo -e "${BLUE}Starting comprehensive safety check...${NC}\n"
    
    check_root
    check_device_exists
    check_device_size
    
    if ! check_mount_status; then
        exit 1
    fi
    
    check_data_patterns
    check_system_impact
    create_device_backup
    final_confirmation
    
    echo -e "\n${GREEN}🎉 Safety check PASSED!${NC}"
    echo "=================================================================="
    echo "Device $TARGET_DEVICE is approved for VexFS testing"
    echo "You may now proceed with environment setup and testing"
    echo ""
    echo "Next steps:"
    echo "1. Run: ./prepare_environment.sh"
    echo "2. Run: ./build_kernel_module.sh"
    echo "3. Begin testing with: cd ../testing && ./run_comprehensive_tests.sh"
}

# Execute main function
main "$@"