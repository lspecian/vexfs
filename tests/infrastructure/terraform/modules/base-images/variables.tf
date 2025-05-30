# Base images module variables

variable "project_name" {
  description = "Project name"
  type        = string
}

variable "environment" {
  description = "Environment name"
  type        = string
}

variable "debian_version" {
  description = "Debian version"
  type        = string
  default     = "12"
}

variable "kernel_version" {
  description = "Kernel version"
  type        = string
  default     = "6.1"
}

variable "enable_kernel_dev" {
  description = "Enable kernel development tools"
  type        = bool
  default     = true
}

variable "enable_rust" {
  description = "Enable Rust toolchain"
  type        = bool
  default     = true
}

variable "enable_docker" {
  description = "Enable Docker"
  type        = bool
  default     = false
}

variable "packer_template_path" {
  description = "Path to Packer template"
  type        = string
  default     = ""
}

variable "output_directory" {
  description = "Output directory for images"
  type        = string
  default     = "/var/lib/libvirt/images"
}

variable "tags" {
  description = "Tags to apply to resources"
  type        = map(string)
  default     = {}
}