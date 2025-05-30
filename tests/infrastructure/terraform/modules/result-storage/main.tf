# Result storage module for VexFS testing infrastructure

terraform {
  required_providers {
    local = {
      source  = "hashicorp/local"
      version = "~> 2.4"
    }
  }
}

# Create local directories for result storage
resource "local_file" "result_storage_config" {
  filename = "/tmp/vexfs-${var.environment}-results/config.json"
  content = jsonencode({
    storage_type = var.storage_type
    retention_days = var.retention_days
    backup_enabled = var.backup_enabled
    environment = var.environment
    project_name = var.project_name
  })
}

# Output storage information
output "storage_path" {
  value = "/tmp/vexfs-${var.environment}-results"
}

output "storage_config" {
  value = local_file.result_storage_config.filename
}