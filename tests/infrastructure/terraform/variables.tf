# VexFS Infrastructure-as-Code Testing Framework
# Variable definitions for Terraform configuration

# Environment configuration
variable "environment" {
  description = "Environment name (dev, test, staging, prod)"
  type        = string
  default     = "test"

  validation {
    condition     = contains(["dev", "test", "staging", "prod"], var.environment)
    error_message = "Environment must be one of: dev, test, staging, prod."
  }
}

variable "libvirt_uri" {
  description = "Libvirt connection URI"
  type        = string
  default     = "qemu:///system"
}

variable "docker_host" {
  description = "Docker daemon host"
  type        = string
  default     = "unix:///var/run/docker.sock"
}

# VM count configuration for each test domain
variable "kernel_module_vm_count" {
  description = "Number of VMs for kernel module testing"
  type        = number
  default     = 2

  validation {
    condition     = var.kernel_module_vm_count >= 1 && var.kernel_module_vm_count <= 10
    error_message = "Kernel module VM count must be between 1 and 10."
  }
}

variable "filesystem_ops_vm_count" {
  description = "Number of VMs for filesystem operations testing"
  type        = number
  default     = 2
}

variable "vector_ops_vm_count" {
  description = "Number of VMs for vector operations testing"
  type        = number
  default     = 1
}

variable "performance_vm_count" {
  description = "Number of VMs for performance testing"
  type        = number
  default     = 1
}

variable "safety_vm_count" {
  description = "Number of VMs for safety validation testing"
  type        = number
  default     = 2
}

variable "integration_vm_count" {
  description = "Number of VMs for integration testing"
  type        = number
  default     = 1
}

# Network configuration
variable "network_cidr" {
  description = "CIDR block for test network"
  type        = string
  default     = "192.168.100.0/24"

  validation {
    condition     = can(cidrhost(var.network_cidr, 0))
    error_message = "Network CIDR must be a valid CIDR block."
  }
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

variable "debian_version" {
  description = "Debian version for base images"
  type        = string
  default     = "12"
}

variable "kernel_version" {
  description = "Kernel version to install"
  type        = string
  default     = "6.1"
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

# LXC configuration
variable "enable_lxc_testing" {
  description = "Enable LXC container testing alongside VMs"
  type        = bool
  default     = false
}

variable "lxc_template" {
  description = "LXC template to use"
  type        = string
  default     = "debian"
}

variable "lxc_privileged" {
  description = "Run LXC containers in privileged mode"
  type        = bool
  default     = true
}

variable "lxc_autostart" {
  description = "Auto-start LXC containers"
  type        = bool
  default     = false
}

# Build configuration
variable "enable_docker_in_vm" {
  description = "Install Docker in VMs"
  type        = bool
  default     = false
}

variable "packer_template_path" {
  description = "Path to Packer template"
  type        = string
  default     = "../legacy/packer/vexfs.pkr.hcl"
}

variable "image_output_directory" {
  description = "Directory for built images"
  type        = string
  default     = "/var/lib/libvirt/images"
}

# Result storage configuration
variable "result_storage_type" {
  description = "Type of result storage (filesystem, database, s3)"
  type        = string
  default     = "filesystem"

  validation {
    condition     = contains(["filesystem", "database", "s3"], var.result_storage_type)
    error_message = "Result storage type must be one of: filesystem, database, s3."
  }
}

variable "result_retention_days" {
  description = "Number of days to retain test results"
  type        = number
  default     = 30
}

variable "result_backup_enabled" {
  description = "Enable backup of test results"
  type        = bool
  default     = true
}

variable "result_db_engine" {
  description = "Database engine for result storage"
  type        = string
  default     = "postgresql"
}

variable "result_db_instance_type" {
  description = "Database instance type"
  type        = string
  default     = "small"
}

# Monitoring configuration
variable "enable_prometheus" {
  description = "Enable Prometheus monitoring"
  type        = bool
  default     = true
}

variable "prometheus_port" {
  description = "Prometheus server port"
  type        = number
  default     = 9090
}

variable "enable_grafana" {
  description = "Enable Grafana dashboards"
  type        = bool
  default     = true
}

variable "grafana_port" {
  description = "Grafana server port"
  type        = number
  default     = 3000
}

variable "enable_alerting" {
  description = "Enable alerting for test failures"
  type        = bool
  default     = false
}

variable "alert_webhook" {
  description = "Webhook URL for alerts"
  type        = string
  default     = ""
  sensitive   = true
}

# Test execution configuration
variable "test_timeout_minutes" {
  description = "Default timeout for test execution in minutes"
  type        = number
  default     = 30
}

variable "parallel_test_execution" {
  description = "Enable parallel test execution"
  type        = bool
  default     = true
}

variable "max_parallel_tests" {
  description = "Maximum number of parallel tests"
  type        = number
  default     = 4
}

variable "test_retry_count" {
  description = "Number of retries for failed tests"
  type        = number
  default     = 2
}

# Resource limits
variable "max_total_memory_gb" {
  description = "Maximum total memory allocation for all VMs (GB)"
  type        = number
  default     = 16
}

variable "max_total_cpus" {
  description = "Maximum total CPU allocation for all VMs"
  type        = number
  default     = 16
}

variable "max_total_disk_gb" {
  description = "Maximum total disk allocation for all VMs (GB)"
  type        = number
  default     = 100
}

# Tags
variable "additional_tags" {
  description = "Additional tags to apply to resources"
  type        = map(string)
  default     = {}
}