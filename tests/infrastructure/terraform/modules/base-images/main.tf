# Base images module for VexFS testing infrastructure

terraform {
  required_providers {
    libvirt = {
      source  = "dmacvicar/libvirt"
      version = "~> 0.7"
    }
  }
}

# Create a base image volume (we'll use a simple approach for now)
resource "libvirt_volume" "base_image" {
  name   = "${var.project_name}-${var.environment}-base.qcow2"
  pool   = "default"
  source = "https://cloud.debian.org/images/cloud/bookworm/latest/debian-12-generic-amd64.qcow2"
  format = "qcow2"
}

# Output base image information
output "base_image_id" {
  value = libvirt_volume.base_image.id
}

output "base_image_path" {
  value = libvirt_volume.base_image.name
}