#!/bin/bash

# VexFS 200GB Testing - Environment Preparation Script
# Sets up the complete testing environment for VexFS kernel module testing

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
WORKBENCH_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VEXFS_ROOT="$(cd "$WORKBENCH_ROOT/.." && pwd)"
TARGET_DEVICE="/dev/sda1"
MOUNT_POINT="/mnt/vexfs_test"

echo -e "${BLUE}🔧 VexFS Environment Preparation${NC}"
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
        print_status "ERROR" "This script must be run as root for kernel operations"
        echo "Please run: sudo $0"
        exit 1
    fi
    print_status "OK" "Running with root privileges"
}

# Function to install system dependencies
install_dependencies() {
    echo -e "\n${BLUE}📦 Installing system dependencies...${NC}"
    
    # Detect package manager
    if command -v apt-get >/dev/null 2>&1; then
        PKG_MANAGER="apt-get"
        UPDATE_CMD="apt-get update"
        INSTALL_CMD="apt-get install -y"
    elif command -v yum >/dev/null 2>&1; then
        PKG_MANAGER="yum"
        UPDATE_CMD="yum update -y"
        INSTALL_CMD="yum install -y"
    elif command -v dnf >/dev/null 2>&1; then
        PKG_MANAGER="dnf"
        UPDATE_CMD="dnf update -y"
        INSTALL_CMD="dnf install -y"
    else
        print_status "ERROR" "No supported package manager found (apt-get, yum, dnf)"
        exit 1
    fi
    
    print_status "OK" "Using package manager: $PKG_MANAGER"
    
    # Update package lists
    echo "Updating package lists..."
    $UPDATE_CMD >/dev/null 2>&1
    
    # Install kernel development packages
    echo "Installing kernel development packages..."
    if [ "$PKG_MANAGER" = "apt-get" ]; then
        $INSTALL_CMD linux-headers-$(uname -r) build-essential >/dev/null 2>&1
    else
        $INSTALL_CMD kernel-devel kernel-headers gcc make >/dev/null 2>&1
    fi
    
    # Install development tools
    echo "Installing development tools..."
    $INSTALL_CMD git curl wget python3 python3-pip >/dev/null 2>&1
    
    # Install monitoring tools
    echo "Installing monitoring tools..."
    $INSTALL_CMD htop iotop sysstat strace >/dev/null 2>&1
    
    # Install filesystem tools
    echo "Installing filesystem tools..."
    $INSTALL_CMD e2fsprogs util-linux parted >/dev/null 2>&1
    
    print_status "OK" "System dependencies installed"
}

# Function to install Rust if not present
install_rust() {
    echo -e "\n${BLUE}🦀 Checking Rust installation...${NC}"
    
    if command -v rustc >/dev/null 2>&1; then
        local rust_version=$(rustc --version)
        print_status "OK" "Rust already installed: $rust_version"
        return
    fi
    
    echo "Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y >/dev/null 2>&1
    source ~/.cargo/env
    
    print_status "OK" "Rust installed successfully"
}

# Function to install Python dependencies
install_python_deps() {
    echo -e "\n${BLUE}🐍 Installing Python dependencies...${NC}"
    
    # Install scientific computing libraries
    pip3 install --quiet numpy scipy scikit-learn pandas matplotlib seaborn
    
    # Install vector processing libraries
    pip3 install --quiet faiss-cpu sentence-transformers transformers
    
    # Install monitoring and analysis libraries
    pip3 install --quiet psutil prometheus-client jupyter notebook
    
    # Install data generation libraries
    pip3 install --quiet requests beautifulsoup4 gitpython
    
    print_status "OK" "Python dependencies installed"
}

# Function to create directory structure
create_directories() {
    echo -e "\n${BLUE}📁 Creating directory structure...${NC}"
    
    # Create mount point
    mkdir -p "$MOUNT_POINT"
    print_status "OK" "Mount point created: $MOUNT_POINT"
    
    # Create workbench directories
    local dirs=(
        "$WORKBENCH_ROOT/data-generation/datasets"
        "$WORKBENCH_ROOT/data-generation/embeddings"
        "$WORKBENCH_ROOT/testing/logs"
        "$WORKBENCH_ROOT/testing/scripts"
        "$WORKBENCH_ROOT/monitoring/metrics"
        "$WORKBENCH_ROOT/monitoring/logs"
        "$WORKBENCH_ROOT/benchmarks/results"
        "$WORKBENCH_ROOT/analysis/reports"
        "$WORKBENCH_ROOT/safety/backups"
        "$WORKBENCH_ROOT/results/$(date +%Y%m%d)"
    )
    
    for dir in "${dirs[@]}"; do
        mkdir -p "$dir"
    done
    
    print_status "OK" "Workbench directories created"
}

# Function to build VexFS kernel module
build_kernel_module() {
    echo -e "\n${BLUE}🔨 Building VexFS kernel module...${NC}"
    
    cd "$VEXFS_ROOT"
    
    # Clean previous builds
    if [ -f "Makefile" ]; then
        make clean >/dev/null 2>&1 || true
    fi
    
    # Build kernel module
    echo "Compiling kernel module..."
    if make >/dev/null 2>&1; then
        print_status "OK" "Kernel module compiled successfully"
    else
        print_status "ERROR" "Kernel module compilation failed"
        echo "Check build logs for details"
        exit 1
    fi
    
    # Verify module file exists
    if [ -f "vexfs.ko" ]; then
        print_status "OK" "Kernel module file: vexfs.ko"
    else
        print_status "ERROR" "Kernel module file not found"
        exit 1
    fi
}

# Function to build Rust components
build_rust_components() {
    echo -e "\n${BLUE}🦀 Building Rust components...${NC}"
    
    cd "$VEXFS_ROOT"
    
    # Build in release mode for performance
    echo "Building Rust components..."
    if cargo build --release >/dev/null 2>&1; then
        print_status "OK" "Rust components built successfully"
    else
        print_status "ERROR" "Rust build failed"
        echo "Check cargo output for details"
        exit 1
    fi
    
    # Verify key binaries
    local binaries=("vexfs_fuse" "vexfs_server")
    for binary in "${binaries[@]}"; do
        if [ -f "target/release/$binary" ]; then
            print_status "OK" "Binary available: $binary"
        else
            print_status "WARNING" "Binary not found: $binary"
        fi
    done
}

# Function to setup monitoring
setup_monitoring() {
    echo -e "\n${BLUE}📊 Setting up monitoring...${NC}"
    
    # Create monitoring configuration
    cat > "$WORKBENCH_ROOT/monitoring/monitor_config.conf" << EOF
# VexFS Monitoring Configuration
MONITOR_INTERVAL=5
METRICS_DIR="$WORKBENCH_ROOT/monitoring/metrics"
LOG_DIR="$WORKBENCH_ROOT/monitoring/logs"
TARGET_DEVICE="$TARGET_DEVICE"
MOUNT_POINT="$MOUNT_POINT"
EOF
    
    # Create systemd service for monitoring (optional)
    cat > "/tmp/vexfs-monitor.service" << EOF
[Unit]
Description=VexFS Performance Monitor
After=multi-user.target

[Service]
Type=simple
ExecStart=$WORKBENCH_ROOT/monitoring/start_monitoring.sh
Restart=always
User=root

[Install]
WantedBy=multi-user.target
EOF
    
    print_status "OK" "Monitoring configuration created"
}

# Function to create test configuration
create_test_config() {
    echo -e "\n${BLUE}⚙️  Creating test configuration...${NC}"
    
    cat > "$WORKBENCH_ROOT/test_config.conf" << EOF
# VexFS 200GB Testing Configuration
# Generated on $(date)

# Device Configuration
TARGET_DEVICE="$TARGET_DEVICE"
MOUNT_POINT="$MOUNT_POINT"
FILESYSTEM_TYPE="vexfs"

# Test Data Configuration
TOTAL_DATA_SIZE_GB=200
TEXT_EMBEDDINGS_GB=80
IMAGE_EMBEDDINGS_GB=80
CODE_EMBEDDINGS_GB=40

# Performance Targets
TARGET_INGESTION_RATE=10000  # vectors/second
TARGET_QUERY_LATENCY=100     # milliseconds
MAX_MEMORY_USAGE_GB=8        # maximum memory usage

# Test Duration
STRESS_TEST_HOURS=24
STABILITY_TEST_HOURS=48

# Paths
VEXFS_ROOT="$VEXFS_ROOT"
WORKBENCH_ROOT="$WORKBENCH_ROOT"
KERNEL_MODULE="$VEXFS_ROOT/vexfs.ko"

# Monitoring
ENABLE_CONTINUOUS_MONITORING=true
METRICS_COLLECTION_INTERVAL=5
ALERT_THRESHOLDS_ENABLED=true

# Safety
ENABLE_AUTOMATIC_BACKUPS=true
BACKUP_INTERVAL_HOURS=6
MAX_BACKUP_RETENTION_DAYS=7
EOF
    
    print_status "OK" "Test configuration created"
}

# Function to verify environment
verify_environment() {
    echo -e "\n${BLUE}🔍 Verifying environment...${NC}"
    
    # Check kernel module
    if [ -f "$VEXFS_ROOT/vexfs.ko" ]; then
        print_status "OK" "Kernel module ready"
    else
        print_status "ERROR" "Kernel module not found"
        exit 1
    fi
    
    # Check target device
    if [ -b "$TARGET_DEVICE" ]; then
        print_status "OK" "Target device accessible"
    else
        print_status "ERROR" "Target device not accessible"
        exit 1
    fi
    
    # Check mount point
    if [ -d "$MOUNT_POINT" ]; then
        print_status "OK" "Mount point ready"
    else
        print_status "ERROR" "Mount point not created"
        exit 1
    fi
    
    # Check Rust toolchain
    if command -v cargo >/dev/null 2>&1; then
        print_status "OK" "Rust toolchain available"
    else
        print_status "ERROR" "Rust toolchain not found"
        exit 1
    fi
    
    # Check Python environment
    if python3 -c "import numpy, scipy, sklearn" 2>/dev/null; then
        print_status "OK" "Python environment ready"
    else
        print_status "ERROR" "Python dependencies missing"
        exit 1
    fi
}

# Function to create quick start script
create_quick_start() {
    echo -e "\n${BLUE}🚀 Creating quick start script...${NC}"
    
    cat > "$WORKBENCH_ROOT/quick_start.sh" << 'EOF'
#!/bin/bash

# VexFS 200GB Testing - Quick Start Script
# Run this script to begin comprehensive testing

set -euo pipefail

WORKBENCH_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "🚀 VexFS 200GB Testing - Quick Start"
echo "===================================="

# Load configuration
source "$WORKBENCH_ROOT/test_config.conf"

echo "1. Loading kernel module..."
sudo insmod "$KERNEL_MODULE"

echo "2. Formatting device..."
sudo mkfs.vexfs "$TARGET_DEVICE"

echo "3. Mounting filesystem..."
sudo mount -t vexfs "$TARGET_DEVICE" "$MOUNT_POINT"

echo "4. Starting monitoring..."
cd "$WORKBENCH_ROOT/monitoring" && ./start_monitoring.sh &

echo "5. Beginning data generation..."
cd "$WORKBENCH_ROOT/data-generation" && ./generate_mixed_embeddings.sh &

echo "6. Running core tests..."
cd "$WORKBENCH_ROOT/testing" && ./run_comprehensive_tests.sh

echo "✅ Quick start complete! Check results in: $WORKBENCH_ROOT/results/"
EOF
    
    chmod +x "$WORKBENCH_ROOT/quick_start.sh"
    print_status "OK" "Quick start script created"
}

# Main execution
main() {
    echo -e "${BLUE}Starting environment preparation...${NC}\n"
    
    check_root
    install_dependencies
    install_rust
    install_python_deps
    create_directories
    build_kernel_module
    build_rust_components
    setup_monitoring
    create_test_config
    verify_environment
    create_quick_start
    
    echo -e "\n${GREEN}🎉 Environment preparation COMPLETE!${NC}"
    echo "=================================================================="
    echo "VexFS testing environment is ready for 200GB testing"
    echo ""
    echo "Configuration file: $WORKBENCH_ROOT/test_config.conf"
    echo "Quick start script: $WORKBENCH_ROOT/quick_start.sh"
    echo ""
    echo "Next steps:"
    echo "1. Run safety check: cd setup && ./safety_check.sh"
    echo "2. Generate test data: cd data-generation && ./generate_mixed_embeddings.sh"
    echo "3. Start testing: ./quick_start.sh"
    echo ""
    echo "⚠️  Remember: This operates on VexFS KERNEL MODULE, not FUSE!"
}

# Execute main function
main "$@"