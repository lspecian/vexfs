# VexFS Infrastructure Configuration - Test Environment
# Generated on Fri 30 May 2025 01:10:56 AM CEST

environment = "test"

# VM Configuration (simplified for quick testing)
kernel_module_vm_count = 1

# Network Configuration
network_cidr = "192.168.100.0/24"
bridge_name  = "vexfs-test-br0"

# SSH Configuration
ssh_public_key = "ssh-rsa AAAAB3NzaC1yc2EAAAADAQABAAACAQDlrK4iYthkbVtIkAR2IWv1aBlJNWXYD2Uhy7begdPGHrjuK8VowTWthowEKBfkggxyLEm2GdpbB6WqUkeVruD8062nboSPR+XHHAQMQXXrGmG8hpKF4HC7X936erKW+wZkRfPqknwexRLqoIdKX1BnoNnKg2cYiZnjMLGmhwXnD9VuDEUm53mpL01oK5upwBKwo+VPmB+yqQdDZhqcxivO4XZ6U5iX6MCIiD5+yFAWeDvR73Hfg9x2X255adLPqwFZVU4h1lacmvChc7kHrXnXaU0hKwjUvoT8dXdlA2PY82asbLf/gVdGevI0GpfQqV27V8A8i0i14eRUqPyhqojcsl/nYBJx7AqtsYqM2tZOQBsIpZIy3HULbvbky5UR/ob5xImWDnqdCWWIG1Z8Jy4lHIcD61vLele1MwYumjP7Wf6BkutHxBC7dIeOLwf9NT5OPlJyz2CqAnV0vEEfGXJYf5Slz2SDlnDlE/PYUSZnuT6kUNxMlrNAW9MMtrTc/OfowTbTNyvi4IRiRUfz19agJffAUfnMa/s5Z7M2qTTIpaez/9ReJOrB1AXbXb6aRLcmpD73HfDIU26bztbmFYL8Dseq5Y2nhaTloYoxmahRhbXDewXGjF4RCjpkQWuX+6nkvhcaiYKrk61/wfm4NR0v0P/MKL4WYNTgNGQVxSeD3Q== vexfs-test-infrastructure"

# Storage Configuration
storage_pool = "default"

# Test Configuration
test_timeout_minutes    = 30
parallel_test_execution = false
max_parallel_tests      = 1

# Resource Limits (conservative for testing)
max_total_memory_gb = 4
max_total_cpus      = 4
max_total_disk_gb   = 20

# Additional tags
additional_tags = {
  "deployment_time" = "2025-05-30T01:10:56+02:00"
  "deployed_by"     = "luis"
  "setup_type"      = "quick_test"
}
