#!/bin/bash

# VexFS 200GB Testing Workbench - GPU Support Installation
# Installs NVIDIA drivers and CUDA support for accelerated embedding generation

set -euo pipefail

echo "🚀 VexFS Workbench - GPU Support Installation"
echo "=============================================="

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

# Check if running as root
if [[ $EUID -eq 0 ]]; then
   log_error "This script should not be run as root for safety"
   exit 1
fi

# Detect GPU
log_info "Detecting NVIDIA GPU..."
if lspci | grep -i nvidia > /dev/null; then
    GPU_INFO=$(lspci | grep -i nvidia | head -1)
    log_success "Found NVIDIA GPU: $GPU_INFO"
else
    log_error "No NVIDIA GPU detected"
    exit 1
fi

# Check current driver status
log_info "Checking current NVIDIA driver status..."
if nvidia-smi > /dev/null 2>&1; then
    log_success "NVIDIA drivers already working!"
    nvidia-smi --query-gpu=name,driver_version,memory.total --format=csv,noheader,nounits
    exit 0
else
    log_warning "NVIDIA drivers not working, proceeding with installation..."
fi

# Detect distribution
if [ -f /etc/os-release ]; then
    . /etc/os-release
    DISTRO=$ID
    VERSION=$VERSION_ID
else
    log_error "Cannot detect Linux distribution"
    exit 1
fi

log_info "Detected distribution: $DISTRO $VERSION"

# Install NVIDIA drivers based on distribution
case $DISTRO in
    ubuntu|debian)
        log_info "Installing NVIDIA drivers for Ubuntu/Debian..."
        
        # Add NVIDIA repository
        sudo apt update
        sudo apt install -y software-properties-common
        
        # Install drivers
        sudo apt install -y nvidia-driver-535 nvidia-dkms-535
        
        # Install CUDA toolkit
        wget https://developer.download.nvidia.com/compute/cuda/repos/ubuntu2204/x86_64/cuda-keyring_1.0-1_all.deb
        sudo dpkg -i cuda-keyring_1.0-1_all.deb
        sudo apt update
        sudo apt install -y cuda-toolkit-12-2
        
        ;;
    fedora|rhel|centos)
        log_info "Installing NVIDIA drivers for Fedora/RHEL/CentOS..."
        
        # Enable RPM Fusion
        sudo dnf install -y https://download1.rpmfusion.org/free/fedora/rpmfusion-free-release-$(rpm -E %fedora).noarch.rpm
        sudo dnf install -y https://download1.rpmfusion.org/nonfree/fedora/rpmfusion-nonfree-release-$(rpm -E %fedora).noarch.rpm
        
        # Install drivers
        sudo dnf install -y akmod-nvidia xorg-x11-drv-nvidia-cuda
        
        ;;
    arch|manjaro)
        log_info "Installing NVIDIA drivers for Arch/Manjaro..."
        
        # Install drivers
        sudo pacman -S --noconfirm nvidia nvidia-utils cuda
        
        ;;
    *)
        log_error "Unsupported distribution: $DISTRO"
        log_info "Please install NVIDIA drivers manually for your distribution"
        exit 1
        ;;
esac

# Install Python packages for GPU acceleration
log_info "Installing Python packages for GPU acceleration..."

# Create virtual environment if it doesn't exist
if [ ! -d "workbench_env" ]; then
    python3 -m venv workbench_env
fi

source workbench_env/bin/activate

# Install GPU-accelerated packages
pip install --upgrade pip
pip install torch torchvision torchaudio --index-url https://download.pytorch.org/whl/cu118
pip install sentence-transformers[gpu]
pip install cupy-cuda11x
pip install rapids-cudf
pip install faiss-gpu

log_success "GPU support installation completed!"
log_warning "Please reboot your system to ensure drivers are properly loaded"

echo ""
echo "After reboot, run 'nvidia-smi' to verify the installation"
echo "Then run the workbench with GPU acceleration enabled"