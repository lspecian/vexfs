# Result storage module variables

variable "project_name" {
  description = "Project name"
  type        = string
}

variable "environment" {
  description = "Environment name"
  type        = string
}

variable "storage_type" {
  description = "Storage type"
  type        = string
  default     = "filesystem"
}

variable "retention_days" {
  description = "Retention days"
  type        = number
  default     = 30
}

variable "backup_enabled" {
  description = "Backup enabled"
  type        = bool
  default     = true
}

variable "db_engine" {
  description = "Database engine"
  type        = string
  default     = "postgresql"
}

variable "db_instance_type" {
  description = "Database instance type"
  type        = string
  default     = "small"
}

variable "tags" {
  description = "Tags to apply to resources"
  type        = map(string)
  default     = {}
}