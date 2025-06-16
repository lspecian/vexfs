#!/bin/bash

# VexFS Development Environment Setup Script
# Installs all required dependencies for VexFS kernel module testing

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging functions
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

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

# Default values
INSTALL_VIRTUALIZATION=true
INSTALL_ANSIBLE=true
INSTALL_TERRAFORM=true
INSTALL_PYTHON_DEPS=true
SETUP_SSH_KEYS=true
SETUP_LIBVIRT=true
DRY_RUN=false

# Help function
show_help() {
    cat << EOF
VexFS Development Environment Setup Script

Usage: $0 [OPTIONS]

OPTIONS:
    --no-virtualization    Skip virtualization tools installation
    --no-ansible          Skip Ansible installation
    --no-terraform        Skip Terraform installation
    --no-python-deps      Skip Python dependencies installation
    --no-ssh-keys         Skip SSH key setup
    --no-libvirt          Skip libvirt setup
    --dry-run             Show what would be installed without installing
    -h, --help            Show this help message

EXAMPLES:
    # Full setup (recommended)
    $0

    # Setup without virtualization (for CI environments)
    $0 --no-virtualization --no-libvirt

    # Dry run to see what would be installed
    $0 --dry-run

EOF
}

# Parse command line arguments
parse_arguments() {
    while [[ $# -gt 0 ]]; do
        case $1 in
            --no-virtualization)
                INSTALL_VIRTUALIZATION=false
                shift
                ;;
            --no-ansible)
                INSTALL_ANSIBLE=false
                shift
                ;;
            --no-terraform)
                INSTALL_TERRAFORM=false
                shift
                ;;
            --no-python-deps)
                INSTALL_PYTHON_DEPS=false
                shift
                ;;
            --no-ssh-keys)
                SETUP_SSH_KEYS=false
                shift
                ;;
            --no-libvirt)
                SETUP_LIBVIRT=false
                shift
                ;;
            --dry-run)
                DRY_RUN=true
                shift
                ;;
            -h|--help)
                show_help
                exit 0
                ;;
            *)
                log_error "Unknown option: $1"
                show_help
                exit 1
                ;;
        esac
    done
}

# Check if running as root
check_root() {
    if [[ $EUID -eq 0 ]]; then
        log_error "This script should not be run as root"
        log_info "Run as a regular user - sudo will be used when needed"
        exit 1
    fi
}

# Detect OS
detect_os() {
    if [[ -f /etc/os-release ]]; then
        . /etc/os-release
        OS=$ID
        OS_VERSION=$VERSION_ID
    else
        log_error "Cannot detect OS. This script supports Ubuntu/Debian."
        exit 1
    fi
    
    log_info "Detected OS: $OS $OS_VERSION"
    
    if [[ "$OS" != "ubuntu" && "$OS" != "debian" ]]; then
        log_error "This script currently supports Ubuntu and Debian only"
        exit 1
    fi
}

# Update package lists
update_packages() {
    log_info "Updating package lists..."
    if [[ "$DRY_RUN" == "false" ]]; then
        sudo apt update
    else
        log_info "[DRY RUN] Would run: sudo apt update"
    fi
}

# Install basic development tools
install_basic_tools() {
    log_info "Installing basic development tools..."
    
    local packages=(
        "curl"
        "wget"
        "git"
        "build-essential"
        "pkg-config"
        "jq"
        "unzip"
        "software-properties-common"
        "apt-transport-https"
        "ca-certificates"
        "gnupg"
        "lsb-release"
    )
    
    if [[ "$DRY_RUN" == "false" ]]; then
        sudo apt install -y "${packages[@]}"
    else
        log_info "[DRY RUN] Would install: ${packages[*]}"
    fi
}

# Install Rust toolchain
install_rust() {
    log_info "Installing Rust toolchain..."
    
    if command -v rustc &> /dev/null; then
        log_info "Rust is already installed: $(rustc --version)"
        return 0
    fi
    
    if [[ "$DRY_RUN" == "false" ]]; then
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        source "$HOME/.cargo/env"
        rustup component add clippy rustfmt
    else
        log_info "[DRY RUN] Would install Rust toolchain"
    fi
}

# Install kernel development tools
install_kernel_dev_tools() {
    log_info "Installing kernel development tools..."
    
    local packages=(
        "linux-headers-$(uname -r)"
        "linux-source"
        "dkms"
        "make"
        "gcc"
        "libc6-dev"
        "libssl-dev"
        "libelf-dev"
        "bc"
        "kmod"
        "cpio"
        "flex"
        "bison"
        "libncurses5-dev"
    )
    
    if [[ "$DRY_RUN" == "false" ]]; then
        sudo apt install -y "${packages[@]}"
    else
        log_info "[DRY RUN] Would install kernel dev tools: ${packages[*]}"
    fi
}

# Install virtualization tools
install_virtualization() {
    if [[ "$INSTALL_VIRTUALIZATION" == "false" ]]; then
        log_info "Skipping virtualization tools installation"
        return 0
    fi
    
    log_info "Installing virtualization tools..."
    
    local packages=(
        "libvirt-daemon-system"
        "libvirt-clients"
        "bridge-utils"
        "virt-manager"
        "qemu-kvm"
        "qemu-system-x86"
        "qemu-utils"
    )
    
    if [[ "$DRY_RUN" == "false" ]]; then
        sudo apt install -y "${packages[@]}"
    else
        log_info "[DRY RUN] Would install virtualization tools: ${packages[*]}"
    fi
}

# Install Terraform
install_terraform() {
    if [[ "$INSTALL_TERRAFORM" == "false" ]]; then
        log_info "Skipping Terraform installation"
        return 0
    fi
    
    log_info "Installing Terraform..."
    
    if command -v terraform &> /dev/null; then
        log_info "Terraform is already installed: $(terraform version | head -n1)"
        return 0
    fi
    
    if [[ "$DRY_RUN" == "false" ]]; then
        # Add HashiCorp GPG key
        wget -O- https://apt.releases.hashicorp.com/gpg | sudo gpg --dearmor -o /usr/share/keyrings/hashicorp-archive-keyring.gpg
        
        # Add HashiCorp repository
        echo "deb [signed-by=/usr/share/keyrings/hashicorp-archive-keyring.gpg] https://apt.releases.hashicorp.com $(lsb_release -cs) main" | sudo tee /etc/apt/sources.list.d/hashicorp.list
        
        # Update and install
        sudo apt update
        sudo apt install -y terraform
    else
        log_info "[DRY RUN] Would install Terraform from HashiCorp repository"
    fi
}

# Install Ansible
install_ansible() {
    if [[ "$INSTALL_ANSIBLE" == "false" ]]; then
        log_info "Skipping Ansible installation"
        return 0
    fi
    
    log_info "Installing Ansible..."
    
    if command -v ansible &> /dev/null; then
        log_info "Ansible is already installed: $(ansible --version | head -n1)"
        return 0
    fi
    
    local packages=(
        "ansible"
        "python3-pip"
    )
    
    if [[ "$DRY_RUN" == "false" ]]; then
        sudo apt install -y "${packages[@]}"
    else
        log_info "[DRY RUN] Would install Ansible: ${packages[*]}"
    fi
}

# Install Python dependencies
install_python_deps() {
    if [[ "$INSTALL_PYTHON_DEPS" == "false" ]]; then
        log_info "Skipping Python dependencies installation"
        return 0
    fi
    
    log_info "Installing Python dependencies..."
    
    local pip_packages=(
        "aiofiles"
        "aiohttp"
        "psutil"
        "jinja2"
        "pyyaml"
    )
    
    if [[ "$DRY_RUN" == "false" ]]; then
        pip3 install "${pip_packages[@]}"
    else
        log_info "[DRY RUN] Would install Python packages: ${pip_packages[*]}"
    fi
}

# Setup SSH keys
setup_ssh_keys() {
    if [[ "$SETUP_SSH_KEYS" == "false" ]]; then
        log_info "Skipping SSH key setup"
        return 0
    fi
    
    log_info "Setting up SSH keys..."
    
    local ssh_key_path="$HOME/.ssh/id_rsa"
    local vexfs_key_path="$HOME/.ssh/vexfs_test_key"
    
    if [[ ! -f "$ssh_key_path" ]]; then
        if [[ "$DRY_RUN" == "false" ]]; then
            log_info "Generating default SSH key..."
            ssh-keygen -t rsa -b 4096 -f "$ssh_key_path" -N "" -C "$(whoami)@$(hostname)"
        else
            log_info "[DRY RUN] Would generate SSH key at $ssh_key_path"
        fi
    else
        log_info "Default SSH key already exists"
    fi
    
    if [[ ! -f "$vexfs_key_path" ]]; then
        if [[ "$DRY_RUN" == "false" ]]; then
            log_info "Generating VexFS test SSH key..."
            ssh-keygen -t rsa -b 4096 -f "$vexfs_key_path" -N "" -C "vexfs-test-infrastructure"
        else
            log_info "[DRY RUN] Would generate VexFS test SSH key at $vexfs_key_path"
        fi
    else
        log_info "VexFS test SSH key already exists"
    fi
}

# Setup libvirt
setup_libvirt() {
    if [[ "$SETUP_LIBVIRT" == "false" ]]; then
        log_info "Skipping libvirt setup"
        return 0
    fi
    
    log_info "Setting up libvirt..."
    
    if [[ "$DRY_RUN" == "false" ]]; then
        # Start and enable libvirt
        sudo systemctl start libvirtd
        sudo systemctl enable libvirtd
        
        # Add user to libvirt group
        sudo usermod -a -G libvirt "$USER"
        
        # Create default storage pool if it doesn't exist
        if ! virsh pool-list --all | grep -q "default"; then
            sudo virsh pool-define-as default dir --target /var/lib/libvirt/images
            sudo virsh pool-autostart default
            sudo virsh pool-start default
        fi
        
        log_warning "You may need to log out and back in for libvirt group membership to take effect"
    else
        log_info "[DRY RUN] Would setup libvirt service and add user to libvirt group"
    fi
}

# Create environment configuration
create_env_config() {
    log_info "Creating environment configuration..."
    
    local env_file="$PROJECT_ROOT/.env.development"
    
    if [[ "$DRY_RUN" == "false" ]]; then
        cat > "$env_file" << EOF
# VexFS Development Environment Configuration
# Generated on $(date)

# SSH Configuration
VEXFS_SSH_KEY_PATH="$HOME/.ssh/vexfs_test_key"
VEXFS_SSH_PUBLIC_KEY_PATH="$HOME/.ssh/vexfs_test_key.pub"

# Libvirt Configuration
VEXFS_LIBVIRT_URI="qemu:///system"
VEXFS_STORAGE_POOL="default"
VEXFS_BASE_IMAGE_PATH="/var/lib/libvirt/images/vexfs-base.qcow2"

# Network Configuration
VEXFS_NETWORK_CIDR="192.168.100.0/24"
VEXFS_BRIDGE_NAME="vexfs-test-br0"

# Terraform Configuration
TERRAFORM_LOG="INFO"

# Ansible Configuration
ANSIBLE_LOG_LEVEL="2"
ANSIBLE_HOST_KEY_CHECKING="False"

# Test Configuration
VEXFS_TEST_TIMEOUT="1800"
VEXFS_TEST_RETRY_COUNT="2"
VEXFS_PARALLEL_EXECUTION="true"

# Project Paths
VEXFS_PROJECT_ROOT="$PROJECT_ROOT"
VEXFS_TEST_ARTIFACTS_PATH="/tmp/vexfs_test_artifacts"
VEXFS_RESULT_OUTPUT_PATH="/tmp/vexfs_test_results"
EOF
        log_success "Environment configuration created at $env_file"
        log_info "Source this file in your shell: source $env_file"
    else
        log_info "[DRY RUN] Would create environment configuration at $env_file"
    fi
}

# Print next steps
print_next_steps() {
    log_success "VexFS development environment setup completed!"
    
    echo
    log_info "Next steps:"
    echo "1. Source the environment configuration:"
    echo "   source $PROJECT_ROOT/.env.development"
    echo
    echo "2. If you installed libvirt, log out and back in to apply group membership"
    echo
    echo "3. Run the quick infrastructure setup:"
    echo "   $PROJECT_ROOT/scripts/quick_infrastructure_setup.sh"
    echo
    echo "4. Navigate to the infrastructure directory:"
    echo "   cd $PROJECT_ROOT/tests/infrastructure/terraform"
    echo
    echo "5. Initialize and deploy the testing infrastructure:"
    echo "   terraform init"
    echo "   terraform plan -var-file=test.tfvars"
    echo "   terraform apply -var-file=test.tfvars"
    echo
    log_info "For more information, see: $PROJECT_ROOT/tests/infrastructure/README.md"
}

# Main function
main() {
    log_info "Starting VexFS development environment setup"
    
    check_root
    detect_os
    
    if [[ "$DRY_RUN" == "true" ]]; then
        log_warning "DRY RUN MODE - No changes will be made"
    fi
    
    update_packages
    install_basic_tools
    install_rust
    install_kernel_dev_tools
    install_virtualization
    install_terraform
    install_ansible
    install_python_deps
    setup_ssh_keys
    setup_libvirt
    create_env_config
    
    if [[ "$DRY_RUN" == "false" ]]; then
        print_next_steps
    else
        log_info "DRY RUN completed - no changes were made"
    fi
}

# Script entry point
parse_arguments "$@"
main