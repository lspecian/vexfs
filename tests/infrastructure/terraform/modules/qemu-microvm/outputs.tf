# QEMU MicroVM Module Outputs

output "vm_ips" {
  description = "IP addresses of the created VMs"
  value       = [for vm in libvirt_domain.test_vm : vm.network_interface[0].addresses[0]]
}

output "vm_names" {
  description = "Names of the created VMs"
  value       = [for vm in libvirt_domain.test_vm : vm.name]
}

output "vm_ids" {
  description = "IDs of the created VMs"
  value       = [for vm in libvirt_domain.test_vm : vm.id]
}

output "ssh_commands" {
  description = "SSH commands to access the VMs"
  value       = [for vm in libvirt_domain.test_vm : "ssh vexfs@${vm.network_interface[0].addresses[0]}"]
}

output "domain_name" {
  description = "Domain name for this module"
  value       = var.domain_name
}