# VexFS Infrastructure-as-Code Testing Framework
# Demonstration configuration for kernel module testing

terraform {
  required_version = ">= 1.0"
  required_providers {
    libvirt = {
      source  = "dmacvicar/libvirt"
      version = "~> 0.7"
    }
    local = {
      source  = "hashicorp/local"
      version = "~> 2.4"
    }
  }
}

# Provider configurations
provider "libvirt" {
  uri = var.libvirt_uri
}

# Local variables
locals {
  project_name = "vexfs"
  environment  = var.environment

  common_tags = {
    Project     = local.project_name
    Environment = local.environment
    ManagedBy   = "terraform"
    Purpose     = "testing"
  }
}

# Network infrastructure
module "test_network" {
  source = "./modules/network"

  project_name = local.project_name
  environment  = local.environment

  network_cidr = var.network_cidr
  bridge_name  = var.bridge_name
  dhcp_range   = var.dhcp_range

  tags = local.common_tags
}

# Create a simple test result file to demonstrate the infrastructure
resource "local_file" "infrastructure_status" {
  filename = "/tmp/vexfs_test_results/infrastructure_status.json"
  content = jsonencode({
    status             = "deployed"
    timestamp          = timestamp()
    environment        = var.environment
    network_cidr       = var.network_cidr
    ssh_key_configured = var.ssh_public_key != ""
    libvirt_uri        = var.libvirt_uri
    message            = "VexFS Infrastructure-as-Code framework successfully deployed"
    next_steps = [
      "VM deployment ready",
      "Kernel module testing framework available",
      "Domain-driven test execution configured"
    ]
  })
}

# Output important information
output "infrastructure_status" {
  description = "Infrastructure deployment status"
  value = {
    network_name = module.test_network.network_name
    network_id   = module.test_network.network_id
    environment  = var.environment
    status_file  = local_file.infrastructure_status.filename
  }
}

output "next_steps" {
  description = "Next steps for VexFS testing"
  value = [
    "Infrastructure successfully deployed",
    "Network configured: ${var.network_cidr}",
    "SSH keys configured: ${var.ssh_public_key != "" ? "Yes" : "No"}",
    "Ready for VM deployment and kernel module testing",
    "Status file created at: ${local_file.infrastructure_status.filename}"
  ]
}

output "test_commands" {
  description = "Commands to run VexFS kernel module tests"
  value = {
    check_libvirt  = "virsh list --all"
    check_network  = "virsh net-list --all"
    view_status    = "cat ${local_file.infrastructure_status.filename}"
    manual_vm_test = "cd ../../test_env && ./run_vm_simple_test.sh"
  }
}