# VexFS Infrastructure-as-Code Testing Framework
# Simplified variable definitions for kernel module testing

# Environment configuration
variable "environment" {
  description = "Environment name (dev, test, staging, prod)"
  type        = string
  default     = "test"
}

variable "libvirt_uri" {
  description = "Libvirt connection URI"
  type        = string
  default     = "qemu:///system"
}

# VM count configuration
variable "kernel_module_vm_count" {
  description = "Number of VMs for kernel module testing"
  type        = number
  default     = 1
}

# Network configuration
variable "network_cidr" {
  description = "CIDR block for test network"
  type        = string
  default     = "192.168.100.0/24"
}

variable "bridge_name" {
  description = "Name of the bridge network interface"
  type        = string
  default     = "vexfs-test-br0"
}

variable "dhcp_range" {
  description = "DHCP range for test network"
  type = object({
    start = string
    end   = string
  })
  default = {
    start = "192.168.100.10"
    end   = "192.168.100.100"
  }
}

# Image configuration
variable "base_image_path" {
  description = "Path to base VM image"
  type        = string
  default     = "/var/lib/libvirt/images/vexfs-base.qcow2"
}

variable "ssh_public_key" {
  description = "SSH public key for VM access"
  type        = string
  default     = ""
}

variable "ssh_private_key_path" {
  description = "Path to SSH private key"
  type        = string
  default     = "~/.ssh/id_rsa"
}

# VM configuration
variable "enable_kvm" {
  description = "Enable KVM acceleration"
  type        = bool
  default     = true
}

variable "enable_nested_virt" {
  description = "Enable nested virtualization"
  type        = bool
  default     = false
}

variable "console_access" {
  description = "Enable console access to VMs"
  type        = bool
  default     = true
}

variable "vnc_access" {
  description = "Enable VNC access to VMs"
  type        = bool
  default     = false
}

variable "storage_pool" {
  description = "Libvirt storage pool name"
  type        = string
  default     = "default"
}