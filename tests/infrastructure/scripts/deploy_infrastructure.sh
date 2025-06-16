#!/bin/bash

# VexFS Infrastructure-as-Code Deployment Script
# Deploys the complete testing infrastructure using Terraform and Ansible

set -e

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
INFRASTRUCTURE_DIR="$(dirname "$SCRIPT_DIR")"
PROJECT_ROOT="$(dirname "$INFRASTRUCTURE_DIR")"
TERRAFORM_DIR="$INFRASTRUCTURE_DIR/terraform"
ANSIBLE_DIR="$INFRASTRUCTURE_DIR/ansible"

# Default values
ENVIRONMENT="test"
DEPLOY_MODE="full"  # full, terraform-only, ansible-only
DRY_RUN=false
FORCE_DESTROY=false
SKIP_VALIDATION=false
PARALLEL_EXECUTION=true

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

# Help function
show_help() {
    cat << EOF
VexFS Infrastructure-as-Code Deployment Script

Usage: $0 [OPTIONS]

OPTIONS:
    -e, --environment ENV       Environment to deploy (dev, test, staging, prod) [default: test]
    -m, --mode MODE            Deployment mode (full, terraform-only, ansible-only) [default: full]
    -d, --dry-run              Perform dry run without making changes
    -f, --force-destroy        Force destroy existing infrastructure
    -s, --skip-validation      Skip pre-deployment validation
    -p, --no-parallel          Disable parallel execution
    -h, --help                 Show this help message

EXAMPLES:
    # Deploy full test environment
    $0 --environment test

    # Deploy only Terraform infrastructure
    $0 --mode terraform-only --environment dev

    # Dry run deployment
    $0 --dry-run --environment staging

    # Force destroy and redeploy
    $0 --force-destroy --environment test

ENVIRONMENT VARIABLES:
    VEXFS_SSH_KEY_PATH         Path to SSH private key for VM access
    VEXFS_BASE_IMAGE_PATH      Path to base VM image
    VEXFS_STORAGE_POOL         Libvirt storage pool name
    VEXFS_NETWORK_CIDR         Network CIDR for test VMs
    TERRAFORM_LOG              Terraform log level (TRACE, DEBUG, INFO, WARN, ERROR)
    ANSIBLE_LOG_LEVEL          Ansible log level (0-4)

EOF
}

# Parse command line arguments
parse_arguments() {
    while [[ $# -gt 0 ]]; do
        case $1 in
            -e|--environment)
                ENVIRONMENT="$2"
                shift 2
                ;;
            -m|--mode)
                DEPLOY_MODE="$2"
                shift 2
                ;;
            -d|--dry-run)
                DRY_RUN=true
                shift
                ;;
            -f|--force-destroy)
                FORCE_DESTROY=true
                shift
                ;;
            -s|--skip-validation)
                SKIP_VALIDATION=true
                shift
                ;;
            -p|--no-parallel)
                PARALLEL_EXECUTION=false
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

# Validate environment
validate_environment() {
    log_info "Validating deployment environment..."
    
    # Check required tools
    local required_tools=("terraform" "ansible" "ansible-playbook" "ssh" "git")
    for tool in "${required_tools[@]}"; do
        if ! command -v "$tool" &> /dev/null; then
            log_error "Required tool '$tool' is not installed"
            exit 1
        fi
    done
    
    # Check Terraform version
    local tf_version=$(terraform version -json | jq -r '.terraform_version')
    log_info "Terraform version: $tf_version"
    
    # Check Ansible version
    local ansible_version=$(ansible --version | head -n1 | awk '{print $2}')
    log_info "Ansible version: $ansible_version"
    
    # Validate environment parameter
    if [[ ! "$ENVIRONMENT" =~ ^(dev|test|staging|prod)$ ]]; then
        log_error "Invalid environment: $ENVIRONMENT. Must be one of: dev, test, staging, prod"
        exit 1
    fi
    
    # Check SSH key
    local ssh_key_path="${VEXFS_SSH_KEY_PATH:-$HOME/.ssh/id_rsa}"
    if [[ ! -f "$ssh_key_path" ]]; then
        log_warning "SSH key not found at $ssh_key_path"
        log_info "Generating new SSH key pair..."
        ssh-keygen -t rsa -b 4096 -f "$ssh_key_path" -N "" -C "vexfs-test-infrastructure"
    fi
    
    # Check base image
    local base_image_path="${VEXFS_BASE_IMAGE_PATH:-/var/lib/libvirt/images/vexfs-base.qcow2}"
    if [[ ! -f "$base_image_path" ]]; then
        log_warning "Base image not found at $base_image_path"
        log_info "You may need to build the base image first using Packer"
    fi
    
    # Check libvirt connection
    if command -v virsh &> /dev/null; then
        if ! virsh list &> /dev/null; then
            log_warning "Cannot connect to libvirt. Make sure libvirtd is running and you have permissions."
        fi
    fi
    
    log_success "Environment validation completed"
}

# Initialize Terraform
initialize_terraform() {
    log_info "Initializing Terraform..."
    
    cd "$TERRAFORM_DIR"
    
    # Initialize Terraform
    terraform init -upgrade
    
    # Validate Terraform configuration
    terraform validate
    
    # Format Terraform files
    terraform fmt -recursive
    
    log_success "Terraform initialized successfully"
}

# Plan Terraform deployment
plan_terraform() {
    log_info "Planning Terraform deployment..."
    
    cd "$TERRAFORM_DIR"
    
    # Create terraform.tfvars file
    create_terraform_vars
    
    # Plan deployment
    local plan_args=()
    if [[ "$DRY_RUN" == "true" ]]; then
        plan_args+=("-detailed-exitcode")
    fi
    
    terraform plan "${plan_args[@]}" -var-file="environments/${ENVIRONMENT}.tfvars" -out="tfplan-${ENVIRONMENT}"
    
    log_success "Terraform plan completed"
}

# Create Terraform variables file
create_terraform_vars() {
    local vars_file="environments/${ENVIRONMENT}.tfvars"
    
    log_info "Creating Terraform variables file: $vars_file"
    
    mkdir -p "environments"
    
    cat > "$vars_file" << EOF
# VexFS Infrastructure Configuration - ${ENVIRONMENT}
# Generated on $(date)

environment = "${ENVIRONMENT}"

# VM Configuration
kernel_module_vm_count = 2
filesystem_ops_vm_count = 2
vector_ops_vm_count = 1
performance_vm_count = 1
safety_vm_count = 2
integration_vm_count = 1

# Network Configuration
network_cidr = "${VEXFS_NETWORK_CIDR:-192.168.100.0/24}"
bridge_name = "vexfs-${ENVIRONMENT}-br0"

# Image Configuration
base_image_path = "${VEXFS_BASE_IMAGE_PATH:-/var/lib/libvirt/images/vexfs-base.qcow2}"
ssh_public_key = "$(cat ${VEXFS_SSH_KEY_PATH:-$HOME/.ssh/id_rsa}.pub 2>/dev/null || echo '')"

# Storage Configuration
storage_pool = "${VEXFS_STORAGE_POOL:-default}"

# Test Configuration
test_timeout_minutes = 30
parallel_test_execution = ${PARALLEL_EXECUTION}
max_parallel_tests = 4

# Monitoring Configuration
enable_prometheus = true
enable_grafana = true
enable_alerting = false

# Resource Limits
max_total_memory_gb = 16
max_total_cpus = 16
max_total_disk_gb = 100

# Additional tags
additional_tags = {
  "deployment_time" = "$(date -Iseconds)"
  "deployed_by" = "$(whoami)"
  "git_commit" = "$(git rev-parse HEAD 2>/dev/null || echo 'unknown')"
}
EOF
    
    log_success "Terraform variables file created"
}

# Apply Terraform deployment
apply_terraform() {
    log_info "Applying Terraform deployment..."
    
    cd "$TERRAFORM_DIR"
    
    if [[ "$DRY_RUN" == "true" ]]; then
        log_info "Dry run mode - skipping Terraform apply"
        return 0
    fi
    
    # Apply the plan
    terraform apply "tfplan-${ENVIRONMENT}"
    
    # Output important information
    terraform output -json > "outputs-${ENVIRONMENT}.json"
    
    log_success "Terraform deployment completed"
}

# Destroy Terraform infrastructure
destroy_terraform() {
    log_info "Destroying Terraform infrastructure..."
    
    cd "$TERRAFORM_DIR"
    
    if [[ "$FORCE_DESTROY" != "true" ]]; then
        read -p "Are you sure you want to destroy the infrastructure? (yes/no): " confirm
        if [[ "$confirm" != "yes" ]]; then
            log_info "Destruction cancelled"
            return 0
        fi
    fi
    
    terraform destroy -var-file="environments/${ENVIRONMENT}.tfvars" -auto-approve
    
    log_success "Terraform infrastructure destroyed"
}

# Setup Ansible inventory
setup_ansible_inventory() {
    log_info "Setting up Ansible inventory..."
    
    cd "$ANSIBLE_DIR"
    
    # Create inventory from Terraform outputs
    local terraform_outputs="$TERRAFORM_DIR/outputs-${ENVIRONMENT}.json"
    
    if [[ ! -f "$terraform_outputs" ]]; then
        log_error "Terraform outputs not found. Run Terraform deployment first."
        exit 1
    fi
    
    # Generate dynamic inventory
    python3 scripts/generate_inventory.py \
        --terraform-outputs "$terraform_outputs" \
        --environment "$ENVIRONMENT" \
        --output "inventory/${ENVIRONMENT}/hosts.yml"
    
    # Create group variables
    mkdir -p "inventory/${ENVIRONMENT}/group_vars"
    
    cat > "inventory/${ENVIRONMENT}/group_vars/all.yml" << EOF
---
# VexFS Ansible Configuration - ${ENVIRONMENT}

# Environment
vexfs_environment: ${ENVIRONMENT}
vexfs_project_root: ${PROJECT_ROOT}

# Test Configuration
vexfs_test_domains:
  - kernel_module
  - filesystem_operations
  - vector_operations
  - performance_metrics
  - safety_validation
  - integration_testing

# SSH Configuration
ansible_user: vexfs
ansible_ssh_private_key_file: ${VEXFS_SSH_KEY_PATH:-$HOME/.ssh/id_rsa}
ansible_ssh_common_args: '-o StrictHostKeyChecking=no'

# Test Execution
test_timeout: 1800
test_retry_count: 2
parallel_execution: ${PARALLEL_EXECUTION}
result_format: structured_json

# Paths
vexfs_source_path: /mnt/vexfs_source
test_artifacts_path: /home/vexfs/test_artifacts
result_output_path: /home/vexfs/test_results

# Monitoring
enable_monitoring: true
enable_logging: true
log_level: INFO
EOF
    
    log_success "Ansible inventory setup completed"
}

# Run Ansible playbooks
run_ansible_playbooks() {
    log_info "Running Ansible playbooks..."
    
    cd "$ANSIBLE_DIR"
    
    if [[ "$DRY_RUN" == "true" ]]; then
        log_info "Dry run mode - skipping Ansible execution"
        return 0
    fi
    
    # Run VM setup playbook
    log_info "Setting up VMs..."
    ansible-playbook \
        -i "inventory/${ENVIRONMENT}/hosts.yml" \
        playbooks/setup_vms.yml \
        --extra-vars "environment=${ENVIRONMENT}"
    
    # Run test execution playbook
    log_info "Executing domain tests..."
    ansible-playbook \
        -i "inventory/${ENVIRONMENT}/hosts.yml" \
        playbooks/run_domain_tests.yml \
        --extra-vars "environment=${ENVIRONMENT} domains=['kernel_module','filesystem_operations']"
    
    log_success "Ansible playbooks completed"
}

# Generate deployment report
generate_deployment_report() {
    log_info "Generating deployment report..."
    
    local report_file="$PROJECT_ROOT/deployment_report_${ENVIRONMENT}_$(date +%Y%m%d_%H%M%S).md"
    
    cat > "$report_file" << EOF
# VexFS Infrastructure Deployment Report

**Environment:** ${ENVIRONMENT}  
**Deployment Time:** $(date)  
**Deployed By:** $(whoami)  
**Git Commit:** $(git rev-parse HEAD 2>/dev/null || echo 'unknown')

## Infrastructure Overview

### Terraform Resources
$(cd "$TERRAFORM_DIR" && terraform show -json 2>/dev/null | jq -r '.values.root_module.resources[] | "- \(.type).\(.name)"' 2>/dev/null || echo "Unable to retrieve Terraform resources")

### VM Inventory
$(cd "$ANSIBLE_DIR" && ansible-inventory -i "inventory/${ENVIRONMENT}/hosts.yml" --list 2>/dev/null | jq -r '.test_vms.hosts[]? // empty | "- \(.)"' 2>/dev/null || echo "Unable to retrieve VM inventory")

## Test Domains Configured

- **Kernel Module**: Module loading, unloading, and stability tests
- **Filesystem Operations**: Mount, unmount, basic I/O operations  
- **Vector Operations**: Vector storage, search, and indexing
- **Performance Metrics**: Throughput, latency, and resource usage testing
- **Safety Validation**: Hang detection and crash recovery
- **Integration Testing**: End-to-end workflow validation

## Access Information

### SSH Access
\`\`\`bash
# Access VMs using:
ssh -i ${VEXFS_SSH_KEY_PATH:-$HOME/.ssh/id_rsa} vexfs@<vm_ip>
\`\`\`

### Test Execution
\`\`\`bash
# Run tests manually:
cd $ANSIBLE_DIR
ansible-playbook -i inventory/${ENVIRONMENT}/hosts.yml playbooks/run_domain_tests.yml
\`\`\`

### Monitoring
$(if [[ -f "$TERRAFORM_DIR/outputs-${ENVIRONMENT}.json" ]]; then
    echo "- Prometheus: http://$(jq -r '.prometheus_url.value // "not_configured"' "$TERRAFORM_DIR/outputs-${ENVIRONMENT}.json")"
    echo "- Grafana: http://$(jq -r '.grafana_url.value // "not_configured"' "$TERRAFORM_DIR/outputs-${ENVIRONMENT}.json")"
else
    echo "- Monitoring URLs not available"
fi)

## Next Steps

1. Verify VM connectivity: \`ansible -i inventory/${ENVIRONMENT}/hosts.yml test_vms -m ping\`
2. Run initial tests: \`ansible-playbook -i inventory/${ENVIRONMENT}/hosts.yml playbooks/run_domain_tests.yml\`
3. Monitor test results in the configured result storage
4. Access Grafana dashboards for real-time monitoring

## Cleanup

To destroy this infrastructure:
\`\`\`bash
$0 --environment ${ENVIRONMENT} --force-destroy
\`\`\`

---
*Report generated by VexFS Infrastructure-as-Code deployment script*
EOF
    
    log_success "Deployment report generated: $report_file"
}

# Main deployment function
main() {
    log_info "Starting VexFS Infrastructure-as-Code deployment"
    log_info "Environment: $ENVIRONMENT, Mode: $DEPLOY_MODE, Dry Run: $DRY_RUN"
    
    # Validation
    if [[ "$SKIP_VALIDATION" != "true" ]]; then
        validate_environment
    fi
    
    # Handle force destroy
    if [[ "$FORCE_DESTROY" == "true" ]]; then
        destroy_terraform
        return 0
    fi
    
    # Terraform deployment
    if [[ "$DEPLOY_MODE" == "full" || "$DEPLOY_MODE" == "terraform-only" ]]; then
        initialize_terraform
        plan_terraform
        apply_terraform
    fi
    
    # Ansible deployment
    if [[ "$DEPLOY_MODE" == "full" || "$DEPLOY_MODE" == "ansible-only" ]]; then
        setup_ansible_inventory
        run_ansible_playbooks
    fi
    
    # Generate report
    if [[ "$DRY_RUN" != "true" ]]; then
        generate_deployment_report
    fi
    
    log_success "VexFS Infrastructure-as-Code deployment completed successfully!"
    
    if [[ "$DRY_RUN" != "true" ]]; then
        log_info "Next steps:"
        log_info "1. Review the deployment report"
        log_info "2. Verify VM connectivity"
        log_info "3. Run test suites"
        log_info "4. Monitor results and performance"
    fi
}

# Script entry point
parse_arguments "$@"
main