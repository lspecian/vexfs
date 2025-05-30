# Monitoring module for VexFS testing infrastructure

terraform {
  required_providers {
    local = {
      source  = "hashicorp/local"
      version = "~> 2.4"
    }
  }
}

# Create monitoring configuration
resource "local_file" "monitoring_config" {
  filename = "/tmp/vexfs-${var.environment}-monitoring/config.json"
  content = jsonencode({
    prometheus = {
      enabled = var.enable_prometheus
      port = var.prometheus_port
    }
    grafana = {
      enabled = var.enable_grafana
      port = var.grafana_port
    }
    alerting = {
      enabled = var.enable_alerting
      webhook = var.alert_webhook
    }
    environment = var.environment
    project_name = var.project_name
  })
}

# Output monitoring information
output "prometheus_url" {
  value = var.enable_prometheus ? "http://localhost:${var.prometheus_port}" : "disabled"
}

output "grafana_url" {
  value = var.enable_grafana ? "http://localhost:${var.grafana_port}" : "disabled"
}

output "monitoring_config" {
  value = local_file.monitoring_config.filename
}