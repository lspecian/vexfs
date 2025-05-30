# QEMU MicroVM Module Variables
# Variable definitions for the QEMU MicroVM testing module

# Basic configuration
variable "project_name" {
  description = "Name of the project"
  type        = string
}

variable "environment" {
  description = "Environment name (dev, test, staging, prod)"
  type        = string
}

variable "domain_name" {
  description = "Test domain name (kernel_module, filesystem_operations, etc.)"
  type        = string
}

variable "description" {
  description = "Description of the test domain"
  type        = string
  default     = ""
}

# VM configuration
variable "vm_count" {
  description = "Number of VMs to create for this domain"
  type        = number
  default     = 1
  
  validation {
    condition     = var.vm_count >= 1 && var.vm_count <= 20
    error_message = "VM count must be between 1 and 20."
  }
}

variable "memory" {
  description = "Memory allocation for each VM (e.g., '1024M', '2G')"
  type        = string
  default     = "1024M"
  
  validation {
    condition     = can(regex("^[0-9]+[MG]$", var.memory))
    error_message = "Memory must be specified with M or G suffix (e.g., '1024M', '2G')."
  }
}

variable "cpus" {
  description = "Number of CPUs for each VM"
  type        = number
  default     = 2
  
  validation {
    condition     = var.cpus >= 1 && var.cpus <= 16
    error_message = "CPU count must be between 1 and 16."
  }
}

variable "disk_size" {
  description = "Disk size for each VM (e.g., '10G', '5120M')"
  type        = string
  default     = "8G"
  
  validation {
    condition     = can(regex("^[0-9]+[MG]$", var.disk_size))
    error_message = "Disk size must be specified with M or G suffix (e.g., '10G', '5120M')."
  }
}

variable "priority" {
  description = "Test priority level (high, medium, low)"
  type        = string
  default     = "medium"
  
  validation {
    condition     = contains(["high", "medium", "low"], var.priority)
    error_message = "Priority must be one of: high, medium, low."
  }
}

# Image and storage configuration
variable "base_image_path" {
  description = "Path to the base VM image"
  type        = string
}

variable "storage_pool" {
  description = "Libvirt storage pool name"
  type        = string
  default     = "default"
}

# Network configuration
variable "network_name" {
  description = "Name of the libvirt network"
  type        = string
  default     = "default"
}

variable "network_cidr" {
  description = "CIDR block for the test network"
  type        = string
  default     = "192.168.100.0/24"
}

# SSH configuration
variable "ssh_public_key" {
  description = "SSH public key for VM access"
  type        = string
}

variable "ssh_private_key" {
  description = "SSH private key content for VM access"
  type        = string
  sensitive   = true
  default     = ""
}

variable "ssh_private_key_path" {
  description = "Path to SSH private key file"
  type        = string
  default     = "~/.ssh/id_rsa"
}

variable "ssh_user" {
  description = "SSH username for VM access"
  type        = string
  default     = "vexfs"
}

# VM features
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

variable "enable_uefi" {
  description = "Enable UEFI firmware"
  type        = bool
  default     = false
}

variable "enable_hugepages" {
  description = "Enable hugepages for memory backing"
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

variable "autostart" {
  description = "Auto-start VMs on host boot"
  type        = bool
  default     = false
}

# Filesystem mounts
variable "vexfs_source_path" {
  description = "Host path to VexFS source code"
  type        = string
  default     = "/home/user/vexfs"
}

variable "test_artifacts_path" {
  description = "Host path for test artifacts"
  type        = string
  default     = "/tmp/vexfs_test_artifacts"
}

# Test execution configuration
variable "test_timeout_minutes" {
  description = "Timeout for test execution in minutes"
  type        = number
  default     = 30
}

variable "test_retry_count" {
  description = "Number of retries for failed tests"
  type        = number
  default     = 2
}

# Tags and metadata
variable "tags" {
  description = "Tags to apply to resources"
  type        = map(string)
  default     = {}
}

# Advanced configuration
variable "qemu_args" {
  description = "Additional QEMU arguments"
  type        = list(string)
  default     = []
}

variable "kernel_args" {
  description = "Additional kernel boot arguments"
  type        = string
  default     = ""
}

variable "machine_type" {
  description = "QEMU machine type"
  type        = string
  default     = "pc"
}

variable "cpu_model" {
  description = "CPU model for VMs"
  type        = string
  default     = "host"
}

# Resource limits
variable "memory_max" {
  description = "Maximum memory allocation (for ballooning)"
  type        = string
  default     = ""
}

variable "cpu_shares" {
  description = "CPU shares for scheduling priority"
  type        = number
  default     = 1024
}

# Monitoring and health checks
variable "enable_monitoring" {
  description = "Enable VM monitoring"
  type        = bool
  default     = true
}

variable "health_check_interval" {
  description = "Health check interval in seconds"
  type        = number
  default     = 60
}

variable "health_check_timeout" {
  description = "Health check timeout in seconds"
  type        = number
  default     = 30
}

# Backup and snapshots
variable "enable_snapshots" {
  description = "Enable VM snapshots"
  type        = bool
  default     = false
}

variable "snapshot_schedule" {
  description = "Snapshot schedule (cron format)"
  type        = string
  default     = ""
}

# Security configuration
variable "enable_secure_boot" {
  description = "Enable secure boot"
  type        = bool
  default     = false
}

variable "enable_tpm" {
  description = "Enable TPM emulation"
  type        = bool
  default     = false
}

# Performance tuning
variable "io_threads" {
  description = "Number of I/O threads"
  type        = number
  default     = 1
}

variable "disk_cache" {
  description = "Disk cache mode (none, writethrough, writeback)"
  type        = string
  default     = "writethrough"
  
  validation {
    condition     = contains(["none", "writethrough", "writeback"], var.disk_cache)
    error_message = "Disk cache must be one of: none, writethrough, writeback."
  }
}

variable "disk_io" {
  description = "Disk I/O mode (threads, native)"
  type        = string
  default     = "threads"
  
  validation {
    condition     = contains(["threads", "native"], var.disk_io)
    error_message = "Disk I/O mode must be one of: threads, native."
  }
}