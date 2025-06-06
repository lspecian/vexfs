#!/bin/bash
#
# VexFS v2.0 xfstests Setup Script
# 
# This script sets up the xfstests environment for VexFS POSIX compliance testing.
# It clones xfstests, installs dependencies, and configures the environment.
#
# Usage: ./setup_xfstests.sh
#

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
XFSTESTS_DIR="${SCRIPT_DIR}/xfstests-dev"
VEXFS_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"

echo "🔧 VexFS v2.0 xfstests Setup"
echo "============================"
echo "Script directory: ${SCRIPT_DIR}"
echo "VexFS root: ${VEXFS_ROOT}"
echo "xfstests target: ${XFSTESTS_DIR}"
echo ""

# Function to check if command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Function to install dependencies
install_dependencies() {
    echo "📦 Installing xfstests dependencies..."
    
    # Check if we're on Ubuntu/Debian
    if command_exists apt-get; then
        sudo apt-get update
        sudo apt-get install -y \
            autotools-dev \
            automake \
            autoconf \
            libtool \
            build-essential \
            pkg-config \
            libattr1-dev \
            libacl1-dev \
            libaio-dev \
            libgdbm-dev \
            uuid-dev \
            xfsprogs \
            e2fsprogs \
            btrfs-progs \
            fio \
            dbench \
            git \
            bc \
            dump \
            indent \
            libtool-bin \
            libssl-dev \
            libcap-dev \
            liburing-dev
    elif command_exists yum; then
        # RHEL/CentOS/Fedora
        sudo yum install -y \
            autotools \
            automake \
            autoconf \
            libtool \
            gcc \
            make \
            pkgconfig \
            libattr-devel \
            libacl-devel \
            libaio-devel \
            gdbm-devel \
            libuuid-devel \
            xfsprogs \
            e2fsprogs \
            btrfs-progs \
            fio \
            git \
            bc \
            dump \
            indent \
            openssl-devel \
            libcap-devel
    else
        echo "⚠️  Unknown package manager. Please install dependencies manually."
        echo "Required packages: autotools, automake, autoconf, libtool, build tools"
        echo "                  libattr-dev, libacl-dev, libaio-dev, uuid-dev"
        echo "                  xfsprogs, e2fsprogs, fio, git, bc"
    fi
}

# Function to clone xfstests
clone_xfstests() {
    echo "📥 Cloning xfstests repository..."
    
    if [ -d "${XFSTESTS_DIR}" ]; then
        echo "xfstests directory already exists. Updating..."
        cd "${XFSTESTS_DIR}"
        git pull origin master
    else
        git clone https://git.kernel.org/pub/scm/fs/xfs/xfstests-dev.git "${XFSTESTS_DIR}"
        cd "${XFSTESTS_DIR}"
    fi
    
    echo "Current xfstests commit: $(git rev-parse --short HEAD)"
}

# Function to build xfstests
build_xfstests() {
    echo "🔨 Building xfstests..."
    
    cd "${XFSTESTS_DIR}"
    
    # Clean previous builds
    if [ -f Makefile ]; then
        make clean || true
    fi
    
    # Configure and build
    make configure
    ./configure
    make -j$(nproc)
    
    echo "✅ xfstests build completed"
}

# Function to create VexFS configuration
create_vexfs_config() {
    echo "⚙️  Creating VexFS-specific configuration..."
    
    # Create VexFS test configuration directory
    mkdir -p "${SCRIPT_DIR}/configs"
    
    # Create VexFS configuration file
    cat > "${SCRIPT_DIR}/configs/vexfs.cfg" << 'EOF'
# VexFS v2.0 xfstests Configuration
# 
# This configuration defines test parameters for VexFS POSIX compliance testing.
# It specifies test and scratch devices, mount options, and VexFS-specific settings.

# Export all variables for xfstests
export FSTYP=vexfs

# Test device configuration
# These will be set dynamically by the test runner based on available devices
export TEST_DEV=""
export TEST_DIR=""
export SCRATCH_DEV=""
export SCRATCH_MNT=""

# VexFS-specific mount options
export MOUNT_OPTIONS=""

# Test execution parameters
export FSSTRESS_AVOID="-f resvsp=0 -f unresvsp=0"
export MKFS_OPTIONS=""

# Timeout settings (VexFS may need longer timeouts for vector operations)
export LOAD_FACTOR=1
export TIMEOUT_FACTOR=2

# VexFS kernel module information
export VEXFS_MODULE_PATH=""
export VEXFS_UAPI_HEADER=""

# Test result directory
export RESULT_BASE="${PWD}/results"

# VexFS-specific test exclusions
# These tests are not applicable to VexFS or require special handling
export VEXFS_EXCLUDE_TESTS="
generic/001
generic/002
generic/003
"

# VexFS vector operation test parameters
export VEXFS_VECTOR_DIMENSIONS=128
export VEXFS_VECTOR_COUNT=1000
export VEXFS_TEST_VECTORS=true

# Performance test parameters
export VEXFS_PERF_TEST=true
export VEXFS_PERF_ITERATIONS=100

# Debug and logging
export VEXFS_DEBUG_LEVEL=1
export VEXFS_LOG_IOCTL=true
EOF

    echo "✅ VexFS configuration created at ${SCRIPT_DIR}/configs/vexfs.cfg"
}

# Function to create test device setup script
create_device_setup() {
    echo "💾 Creating test device setup script..."
    
    cat > "${SCRIPT_DIR}/setup_test_devices.sh" << 'DEVICE_SETUP_EOF'
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
DEVICE_SETUP_EOF

    chmod +x "${SCRIPT_DIR}/setup_test_devices.sh"
    echo "✅ Test device setup script created"
}

# Function to verify setup
verify_setup() {
    echo "🔍 Verifying xfstests setup..."
    
    # Check if xfstests was built successfully
    if [ ! -f "${XFSTESTS_DIR}/check" ]; then
        echo "❌ xfstests check script not found"
        return 1
    fi
    
    # Check if VexFS kernel module exists
    VEXFS_MODULE="${VEXFS_ROOT}/kernel/vexfs_v2_build/vexfs_v2_b62.ko"
    if [ ! -f "${VEXFS_MODULE}" ]; then
        echo "⚠️  VexFS kernel module not found at ${VEXFS_MODULE}"
        echo "    Please build the VexFS kernel module first"
    else
        echo "✅ VexFS kernel module found: ${VEXFS_MODULE}"
    fi
    
    # Check if VexFS UAPI header exists
    VEXFS_UAPI="${VEXFS_ROOT}/kernel/vexfs_v2_build/vexfs_v2_uapi.h"
    if [ ! -f "${VEXFS_UAPI}" ]; then
        echo "❌ VexFS UAPI header not found at ${VEXFS_UAPI}"
        return 1
    else
        echo "✅ VexFS UAPI header found: ${VEXFS_UAPI}"
    fi
    
    echo "✅ Setup verification completed"
}

# Main execution
main() {
    echo "Starting VexFS xfstests setup..."
    
    install_dependencies
    clone_xfstests
    build_xfstests
    create_vexfs_config
    create_device_setup
    verify_setup
    
    echo ""
    echo "🎉 VexFS xfstests setup completed successfully!"
    echo ""
    echo "Next steps:"
    echo "1. Setup test devices: ./setup_test_devices.sh"
    echo "2. Run VexFS tests: ./run_vexfs_xfstests.sh"
    echo ""
    echo "For more information, see the documentation in docs/testing/"
}

# Execute main function
main "$@"