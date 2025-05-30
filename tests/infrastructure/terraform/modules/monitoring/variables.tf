# Monitoring module variables

variable "project_name" {
  description = "Project name"
  type        = string
}

variable "environment" {
  description = "Environment name"
  type        = string
}

variable "enable_prometheus" {
  description = "Enable Prometheus monitoring"
  type        = bool
  default     = true
}

variable "prometheus_port" {
  description = "Prometheus port"
  type        = number
  default     = 9090
}

variable "enable_grafana" {
  description = "Enable Grafana dashboards"
  type        = bool
  default     = true
}

variable "grafana_port" {
  description = "Grafana port"
  type        = number
  default     = 3000
}

variable "enable_alerting" {
  description = "Enable alerting"
  type        = bool
  default     = false
}

variable "alert_webhook" {
  description = "Alert webhook URL"
  type        = string
  default     = ""
  sensitive   = true
}

variable "tags" {
  description = "Tags to apply to resources"
  type        = map(string)
  default     = {}
}