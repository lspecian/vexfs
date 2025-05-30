# QEMU MicroVM Module for VexFS Testing
# Provides lightweight, fast-booting VMs for test execution

terraform {
  required_providers {
    libvirt = {
      source  = "dmacvicar/libvirt"
      version = "~> 0.7"
    }
  }
}

# Local variables
locals {
  vm_name_prefix = "${var.project_name}-${var.environment}-${var.domain_name}"
  
  # Convert memory string to bytes for libvirt
  memory_bytes = tonumber(regex("([0-9]+)", var.memory)[0]) * (
    can(regex("G", var.memory)) ? 1073741824 :
    can(regex("M", var.memory)) ? 1048576 : 1
  )
  
  # Convert disk size to bytes
  disk_bytes = tonumber(regex("([0-9]+)", var.disk_size)[0]) * (
    can(regex("G", var.disk_size)) ? 1073741824 :
    can(regex("M", var.disk_size)) ? 1048576 : 1
  )
}

# Cloud-init configuration for VM initialization
data "template_file" "cloud_init_user_data" {
  count = var.vm_count
  
  template = file("${path.module}/templates/cloud-init-user-data.yaml")
  
  vars = {
    hostname        = "${local.vm_name_prefix}-${count.index + 1}"
    ssh_public_key  = var.ssh_public_key
    domain_name     = var.domain_name
    vm_index        = count.index + 1
    project_name    = var.project_name
    environment     = var.environment
    
    # Test-specific configuration
    enable_kernel_dev = var.domain_name == "kernel_module" ? "true" : "false"
    enable_vector_ops = var.domain_name == "vector_operations" ? "true" : "false"
    enable_perf_tools = var.domain_name == "performance_metrics" ? "true" : "false"
    
    # VexFS source mount configuration
    vexfs_source_path = "/mnt/vexfs_source"
    
    # Test execution configuration
    test_timeout     = var.test_timeout_minutes
    test_retry_count = var.test_retry_count
  }
}

# Cloud-init network configuration
data "template_file" "cloud_init_network_config" {
  count = var.vm_count
  
  template = file("${path.module}/templates/cloud-init-network-config.yaml")
  
  vars = {
    vm_index = count.index + 1
    # Calculate IP address based on domain and VM index
    ip_address = cidrhost(var.network_cidr, 10 + (count.index * 10) + count.index)
    gateway    = cidrhost(var.network_cidr, 1)
    netmask    = cidrnetmask(var.network_cidr)
  }
}

# Create cloud-init ISO for each VM
resource "libvirt_cloudinit_disk" "vm_cloudinit" {
  count = var.vm_count
  
  name           = "${local.vm_name_prefix}-${count.index + 1}-cloudinit.iso"
  user_data      = data.template_file.cloud_init_user_data[count.index].rendered
  network_config = data.template_file.cloud_init_network_config[count.index].rendered
  pool           = var.storage_pool
}

# Create disk volumes for VMs
resource "libvirt_volume" "vm_disk" {
  count = var.vm_count
  
  name             = "${local.vm_name_prefix}-${count.index + 1}-disk.qcow2"
  pool             = var.storage_pool
  size             = local.disk_bytes
  base_volume_name = basename(var.base_image_path)
  format           = "qcow2"
}

# Create VMs
resource "libvirt_domain" "test_vm" {
  count = var.vm_count
  
  name   = "${local.vm_name_prefix}-${count.index + 1}"
  memory = local.memory_bytes / 1048576  # Convert back to MB for libvirt
  vcpu   = var.cpus
  
  # Enable KVM acceleration if available
  type = var.enable_kvm ? "kvm" : "qemu"
  
  # CPU configuration
  cpu {
    mode = var.enable_kvm ? "host-passthrough" : "custom"
    
    dynamic "feature" {
      for_each = var.enable_nested_virt ? ["vmx", "svm"] : []
      content {
        policy = "optional"
        name   = feature.value
      }
    }
  }
  
  # Firmware configuration
  firmware = var.enable_uefi ? "/usr/share/OVMF/OVMF_CODE.fd" : null
  
  # Boot configuration
  boot_device {
    dev = ["hd", "cdrom", "network"]
  }
  
  # Disk configuration
  disk {
    volume_id = libvirt_volume.vm_disk[count.index].id
    scsi      = false
  }
  
  # Cloud-init disk
  disk {
    file = libvirt_cloudinit_disk.vm_cloudinit[count.index].id
  }
  
  # Network interface
  network_interface {
    network_name   = var.network_name
    wait_for_lease = true
    hostname       = "${local.vm_name_prefix}-${count.index + 1}"
  }
  
  # Console configuration
  console {
    type        = "pty"
    target_port = "0"
    target_type = var.console_access ? "serial" : null
  }
  
  # Graphics configuration
  dynamic "graphics" {
    for_each = var.vnc_access ? [1] : []
    content {
      type        = "vnc"
      listen_type = "address"
      listen_address = "0.0.0.0"
      autoport    = true
    }
  }
  
  # Filesystem mounts for VexFS source code
  filesystem {
    source   = var.vexfs_source_path
    target   = "vexfs_source"
    readonly = false
    accessmode = "mapped"
  }
  
  # Additional filesystems for test artifacts
  filesystem {
    source   = var.test_artifacts_path
    target   = "test_artifacts"
    readonly = false
    accessmode = "mapped"
  }
  
  # VM lifecycle management
  autostart = var.autostart
  
  # Resource constraints
  memory_backing {
    hugepages = var.enable_hugepages
  }
  
  # QEMU agent
  qemu_agent = true
  
  # Metadata
  xml {
    xslt = file("${path.module}/templates/domain-metadata.xsl")
  }
  
  # Tags as metadata
  metadata = jsonencode(merge(var.tags, {
    domain_name = var.domain_name
    vm_index    = count.index + 1
    priority    = var.priority
    description = var.description
  }))
  
  # Ensure proper shutdown
  on_poweroff = "destroy"
  on_reboot   = "restart"
  on_crash    = "restart"
  
  depends_on = [
    libvirt_volume.vm_disk,
    libvirt_cloudinit_disk.vm_cloudinit
  ]
}

# Wait for VMs to be ready
resource "null_resource" "wait_for_vm_ready" {
  count = var.vm_count
  
  provisioner "remote-exec" {
    inline = [
      "echo 'VM ${local.vm_name_prefix}-${count.index + 1} is ready'",
      "cloud-init status --wait",
      "systemctl is-active --quiet ssh",
      "test -d /mnt/vexfs_source || mkdir -p /mnt/vexfs_source"
    ]
    
    connection {
      type        = "ssh"
      host        = libvirt_domain.test_vm[count.index].network_interface[0].addresses[0]
      user        = var.ssh_user
      private_key = var.ssh_private_key
      timeout     = "5m"
    }
  }
  
  depends_on = [libvirt_domain.test_vm]
}

# Create test execution scripts
resource "local_file" "test_execution_script" {
  count = var.vm_count
  
  filename = "${var.test_artifacts_path}/${var.domain_name}/vm-${count.index + 1}/execute_tests.sh"
  
  content = templatefile("${path.module}/templates/execute_tests.sh.tpl", {
    domain_name   = var.domain_name
    vm_index      = count.index + 1
    vm_hostname   = "${local.vm_name_prefix}-${count.index + 1}"
    vm_ip         = libvirt_domain.test_vm[count.index].network_interface[0].addresses[0]
    ssh_user      = var.ssh_user
    ssh_key_path  = var.ssh_private_key_path
    test_timeout  = var.test_timeout_minutes
    retry_count   = var.test_retry_count
    project_name  = var.project_name
    environment   = var.environment
  })
  
  file_permission = "0755"
  
  depends_on = [libvirt_domain.test_vm]
}

# VM health check
resource "null_resource" "vm_health_check" {
  count = var.vm_count
  
  triggers = {
    vm_id = libvirt_domain.test_vm[count.index].id
  }
  
  provisioner "local-exec" {
    command = "${path.module}/scripts/health_check.sh"
    
    environment = {
      VM_NAME     = libvirt_domain.test_vm[count.index].name
      VM_IP       = libvirt_domain.test_vm[count.index].network_interface[0].addresses[0]
      SSH_USER    = var.ssh_user
      SSH_KEY     = var.ssh_private_key_path
      DOMAIN_NAME = var.domain_name
    }
  }
  
  depends_on = [null_resource.wait_for_vm_ready]
}