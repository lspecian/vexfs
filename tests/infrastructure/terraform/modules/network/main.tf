# Network module for VexFS testing infrastructure

terraform {
  required_providers {
    libvirt = {
      source  = "dmacvicar/libvirt"
      version = "~> 0.7"
    }
  }
}

# Create a network for test VMs
resource "libvirt_network" "test_network" {
  name      = "${var.project_name}-${var.environment}-network"
  mode      = "nat"
  domain    = "${var.project_name}-${var.environment}.local"
  addresses = [var.network_cidr]
  
  dhcp {
    enabled = true
  }
  
  dns {
    enabled = true
  }
  
  autostart = true
}

# Output network information
output "network_name" {
  value = libvirt_network.test_network.name
}

output "network_id" {
  value = libvirt_network.test_network.id
}