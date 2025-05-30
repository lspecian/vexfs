# Network module variables

variable "project_name" {
  description = "Project name"
  type        = string
}

variable "environment" {
  description = "Environment name"
  type        = string
}

variable "network_cidr" {
  description = "CIDR block for the network"
  type        = string
  default     = "192.168.100.0/24"
}

variable "bridge_name" {
  description = "Bridge name"
  type        = string
  default     = "virbr0"
}

variable "dhcp_range" {
  description = "DHCP range"
  type = object({
    start = string
    end   = string
  })
  default = {
    start = "192.168.100.10"
    end   = "192.168.100.100"
  }
}

variable "tags" {
  description = "Tags to apply to resources"
  type        = map(string)
  default     = {}
}