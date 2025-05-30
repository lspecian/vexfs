# VexFS Infrastructure-as-Code Testing Framework
# Simplified main configuration for kernel module testing

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

# QEMU MicroVM for kernel module testing only
module "kernel_module_vms" {
  source = "./modules/qemu-microvm"

  domain_name = "kernel_module"
  description = "Kernel module loading, unloading, and stability tests"
  vm_count    = var.kernel_module_vm_count
  memory      = "1024M"
  cpus        = 2
  disk_size   = "8G"
  priority    = "high"

  base_image_path = var.base_image_path
  ssh_public_key  = var.ssh_public_key

  project_name = local.project_name
  environment  = local.environment
  tags         = local.common_tags

  # Network configuration
  network_name = module.test_network.network_name
  network_cidr = var.network_cidr

  # Test-specific configuration
  enable_kvm         = var.enable_kvm
  enable_nested_virt = var.enable_nested_virt
  console_access     = var.console_access
  vnc_access         = var.vnc_access

  # Storage configuration
  storage_pool = var.storage_pool

  # VexFS source mount
  vexfs_source_path   = "/home/luis/Development/oss/vexfs"
  test_artifacts_path = "/tmp/vexfs_test_artifacts"

  depends_on = [module.test_network]
}

# Output important information
output "kernel_module_vm_ips" {
  description = "IP addresses of kernel module test VMs"
  value       = module.kernel_module_vms.vm_ips
}

output "ssh_command" {
  description = "SSH command to access VMs"
  value       = "ssh -i ${var.ssh_private_key_path} vexfs@<vm_ip>"
}

output "test_execution_command" {
  description = "Command to run kernel module tests"
  value       = "cd ../ansible && ansible-playbook -i inventory/test/hosts.yml playbooks/run_domain_tests.yml --extra-vars 'domains=[\"kernel_module\"]'"
}