#!/bin/bash
#
# VexFS Test Device Setup Script
#
# This script sets up test devices for xfstests execution.
# It can use loop devices, RAM disks, or real block devices.
#

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
VEXFS_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"

# Default configuration
TEST_SIZE="1G"
SCRATCH_SIZE="2G"
USE_LOOP_DEVICES=true
USE_RAM_DISK=false
REAL_DEVICES=""

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --test-size)
            TEST_SIZE="$2"
            shift 2
            ;;
        --scratch-size)
            SCRATCH_SIZE="$2"
            shift 2
            ;;
        --use-real-devices)
            USE_LOOP_DEVICES=false
            REAL_DEVICES="$2"
            shift 2
            ;;
        --use-ram-disk)
            USE_RAM_DISK=true
            USE_LOOP_DEVICES=false
            shift
            ;;
        --help)
            echo "Usage: $0 [options]"
            echo "Options:"
            echo "  --test-size SIZE        Size of test device (default: 1G)"
            echo "  --scratch-size SIZE     Size of scratch device (default: 2G)"
            echo "  --use-real-devices DEV1,DEV2  Use real block devices"
            echo "  --use-ram-disk          Use RAM disk for testing"
            echo "  --help                  Show this help"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            exit 1
            ;;
    esac
done

echo "🔧 VexFS Test Device Setup"
echo "=========================="
echo "Test device size: ${TEST_SIZE}"
echo "Scratch device size: ${SCRATCH_SIZE}"
echo "Use loop devices: ${USE_LOOP_DEVICES}"
echo "Use RAM disk: ${USE_RAM_DISK}"
echo ""

# Function to setup loop devices
setup_loop_devices() {
    echo "📀 Setting up loop devices..."
    
    # Create test images directory
    mkdir -p "${SCRIPT_DIR}/test_images"
    
    # Create test device image
    TEST_IMG="${SCRIPT_DIR}/test_images/vexfs_test.img"
    if [ ! -f "${TEST_IMG}" ]; then
        echo "Creating test device image: ${TEST_IMG}"
        dd if=/dev/zero of="${TEST_IMG}" bs=1M count=$(echo ${TEST_SIZE} | sed 's/G/*1024/g' | bc) 2>/dev/null
    fi
    
    # Create scratch device image
    SCRATCH_IMG="${SCRIPT_DIR}/test_images/vexfs_scratch.img"
    if [ ! -f "${SCRATCH_IMG}" ]; then
        echo "Creating scratch device image: ${SCRATCH_IMG}"
        dd if=/dev/zero of="${SCRATCH_IMG}" bs=1M count=$(echo ${SCRATCH_SIZE} | sed 's/G/*1024/g' | bc) 2>/dev/null
    fi
    
    # Setup loop devices
    TEST_LOOP=$(sudo losetup -f --show "${TEST_IMG}")
    SCRATCH_LOOP=$(sudo losetup -f --show "${SCRATCH_IMG}")
    
    echo "Test loop device: ${TEST_LOOP}"
    echo "Scratch loop device: ${SCRATCH_LOOP}"
    
    # Export for xfstests
    export TEST_DEV="${TEST_LOOP}"
    export SCRATCH_DEV="${SCRATCH_LOOP}"
}

# Function to setup RAM disk
setup_ram_disk() {
    echo "💾 Setting up RAM disk..."
    
    # Create RAM disk mount points
    sudo mkdir -p /mnt/vexfs_ram_test
    sudo mkdir -p /mnt/vexfs_ram_scratch
    
    # Mount RAM disks
    sudo mount -t tmpfs -o size=${TEST_SIZE} tmpfs /mnt/vexfs_ram_test
    sudo mount -t tmpfs -o size=${SCRATCH_SIZE} tmpfs /mnt/vexfs_ram_scratch
    
    # Create device files in RAM disk
    TEST_IMG="/mnt/vexfs_ram_test/test.img"
    SCRATCH_IMG="/mnt/vexfs_ram_scratch/scratch.img"
    
    sudo dd if=/dev/zero of="${TEST_IMG}" bs=1M count=$(echo ${TEST_SIZE} | sed 's/G/*1024/g' | bc) 2>/dev/null
    sudo dd if=/dev/zero of="${SCRATCH_IMG}" bs=1M count=$(echo ${SCRATCH_SIZE} | sed 's/G/*1024/g' | bc) 2>/dev/null
    
    # Setup loop devices
    TEST_LOOP=$(sudo losetup -f --show "${TEST_IMG}")
    SCRATCH_LOOP=$(sudo losetup -f --show "${SCRATCH_IMG}")
    
    echo "RAM-backed test device: ${TEST_LOOP}"
    echo "RAM-backed scratch device: ${SCRATCH_LOOP}"
    
    export TEST_DEV="${TEST_LOOP}"
    export SCRATCH_DEV="${SCRATCH_LOOP}"
}

# Function to setup real devices
setup_real_devices() {
    echo "🔧 Setting up real block devices..."
    
    if [ -z "${REAL_DEVICES}" ]; then
        echo "❌ No real devices specified"
        exit 1
    fi
    
    IFS=',' read -ra DEVICES <<< "${REAL_DEVICES}"
    
    if [ ${#DEVICES[@]} -lt 2 ]; then
        echo "❌ Need at least 2 devices for test and scratch"
        exit 1
    fi
    
    TEST_DEV="${DEVICES[0]}"
    SCRATCH_DEV="${DEVICES[1]}"
    
    # Verify devices exist
    if [ ! -b "${TEST_DEV}" ]; then
        echo "❌ Test device ${TEST_DEV} does not exist"
        exit 1
    fi
    
    if [ ! -b "${SCRATCH_DEV}" ]; then
        echo "❌ Scratch device ${SCRATCH_DEV} does not exist"
        exit 1
    fi
    
    echo "Test device: ${TEST_DEV}"
    echo "Scratch device: ${SCRATCH_DEV}"
    
    export TEST_DEV
    export SCRATCH_DEV
}

# Function to create mount points
create_mount_points() {
    echo "📁 Creating mount points..."
    
    export TEST_DIR="/mnt/vexfs_test"
    export SCRATCH_MNT="/mnt/vexfs_scratch"
    
    sudo mkdir -p "${TEST_DIR}"
    sudo mkdir -p "${SCRATCH_MNT}"
    
    echo "Test mount point: ${TEST_DIR}"
    echo "Scratch mount point: ${SCRATCH_MNT}"
}

# Function to save device configuration
save_device_config() {
    echo "💾 Saving device configuration..."
    
    cat > "${SCRIPT_DIR}/device_config.env" << EOF
# VexFS Test Device Configuration
# Generated by setup_test_devices.sh on $(date)

export TEST_DEV="${TEST_DEV}"
export TEST_DIR="${TEST_DIR}"
export SCRATCH_DEV="${SCRATCH_DEV}"
export SCRATCH_MNT="${SCRATCH_MNT}"

# Device type information
export USE_LOOP_DEVICES=${USE_LOOP_DEVICES}
export USE_RAM_DISK=${USE_RAM_DISK}
export TEST_SIZE="${TEST_SIZE}"
export SCRATCH_SIZE="${SCRATCH_SIZE}"
EOF

    echo "✅ Device configuration saved to ${SCRIPT_DIR}/device_config.env"
}

# Main execution
main() {
    if [ "${USE_RAM_DISK}" = true ]; then
        setup_ram_disk
    elif [ "${USE_LOOP_DEVICES}" = true ]; then
        setup_loop_devices
    else
        setup_real_devices
    fi
    
    create_mount_points
    save_device_config
    
    echo ""
    echo "✅ Test device setup completed!"
    echo "Test device: ${TEST_DEV}"
    echo "Scratch device: ${SCRATCH_DEV}"
    echo "Test mount: ${TEST_DIR}"
    echo "Scratch mount: ${SCRATCH_MNT}"
}

main "$@"
