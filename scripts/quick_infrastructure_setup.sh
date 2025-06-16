#!/bin/bash

# VexFS Quick Infrastructure Setup Script
# Sets up the testing infrastructure quickly for immediate use

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
INFRASTRUCTURE_DIR="$PROJECT_ROOT/infrastructure"

# Quick setup function
quick_setup() {
    log_info "VexFS Quick Infrastructure Setup"
    log_info "================================"
    
    # Check if we're in the right directory
    if [[ ! -d "$INFRASTRUCTURE_DIR" ]]; then
        log_error "Infrastructure directory not found at $INFRASTRUCTURE_DIR"
        exit 1
    fi
    
    # Create SSH keys if they don't exist
    local ssh_key_path="$HOME/.ssh/vexfs_test_key"
    if [[ ! -f "$ssh_key_path" ]]; then
        log_info "Creating VexFS test SSH key..."
        mkdir -p "$HOME/.ssh"
        ssh-keygen -t rsa -b 4096 -f "$ssh_key_path" -N "" -C "vexfs-test-infrastructure"
        log_success "SSH key created at $ssh_key_path"
    else
        log_info "VexFS test SSH key already exists"
    fi
    
    # Create Terraform variables file
    local tfvars_file="$INFRASTRUCTURE_DIR/terraform/test.tfvars"
    log_info "Creating Terraform variables file..."
    
    cat > "$tfvars_file" << EOF
# VexFS Infrastructure Configuration - Test Environment
# Generated on $(date)

environment = "test"

# VM Configuration (simplified for quick testing)
kernel_module_vm_count = 1

# Network Configuration
network_cidr = "192.168.100.0/24"
bridge_name = "vexfs-test-br0"

# SSH Configuration
ssh_public_key = "$(cat ${ssh_key_path}.pub 2>/dev/null || echo '')"

# Storage Configuration
storage_pool = "default"

# Test Configuration
test_timeout_minutes = 30
parallel_test_execution = false
max_parallel_tests = 1

# Resource Limits (conservative for testing)
max_total_memory_gb = 4
max_total_cpus = 4
max_total_disk_gb = 20

# Additional tags
additional_tags = {
  "deployment_time" = "$(date -Iseconds)"
  "deployed_by" = "$(whoami)"
  "setup_type" = "quick_test"
}
EOF
    
    log_success "Terraform variables file created at $tfvars_file"
    
    # Create environment file
    local env_file="$PROJECT_ROOT/.env.testing"
    log_info "Creating testing environment file..."
    
    cat > "$env_file" << EOF
# VexFS Testing Environment Configuration
# Generated on $(date)

# SSH Configuration
export VEXFS_SSH_KEY_PATH="$ssh_key_path"
export VEXFS_SSH_PUBLIC_KEY_PATH="${ssh_key_path}.pub"

# Libvirt Configuration
export VEXFS_LIBVIRT_URI="qemu:///system"
export VEXFS_STORAGE_POOL="default"

# Network Configuration
export VEXFS_NETWORK_CIDR="192.168.100.0/24"

# Terraform Configuration
export TERRAFORM_LOG="INFO"

# Ansible Configuration
export ANSIBLE_LOG_LEVEL="2"
export ANSIBLE_HOST_KEY_CHECKING="False"

# Project Paths
export VEXFS_PROJECT_ROOT="$PROJECT_ROOT"
export VEXFS_TEST_ARTIFACTS_PATH="/tmp/vexfs_test_artifacts"
export VEXFS_RESULT_OUTPUT_PATH="/tmp/vexfs_test_results"
EOF
    
    log_success "Testing environment file created at $env_file"
    
    # Create test artifacts directories
    log_info "Creating test directories..."
    mkdir -p /tmp/vexfs_test_artifacts
    mkdir -p /tmp/vexfs_test_results
    
    # Check libvirt status
    if systemctl is-active --quiet libvirtd; then
        log_success "Libvirt is running"
    else
        log_warning "Libvirt is not running. Starting it..."
        sudo systemctl start libvirtd
    fi
    
    # Check if user is in libvirt group
    if groups | grep -q libvirt; then
        log_success "User is in libvirt group"
    else
        log_warning "User is not in libvirt group. You may need to log out and back in."
    fi
    
    log_success "Quick infrastructure setup completed!"
    
    echo
    log_info "Next steps:"
    echo "1. Source the testing environment:"
    echo "   source $env_file"
    echo
    echo "2. Navigate to the Terraform directory:"
    echo "   cd $INFRASTRUCTURE_DIR/terraform"
    echo
    echo "3. Initialize Terraform:"
    echo "   terraform init"
    echo
    echo "4. Plan the deployment:"
    echo "   terraform plan -var-file=test.tfvars"
    echo
    echo "5. Apply the deployment:"
    echo "   terraform apply -var-file=test.tfvars"
    echo
    echo "6. Run kernel module tests:"
    echo "   cd ../ansible"
    echo "   ansible-playbook -i inventory/test/hosts.yml playbooks/run_domain_tests.yml"
    echo
}

# Main execution
quick_setup