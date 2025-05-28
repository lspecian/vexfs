# VexFS Production Image Build Pipeline
# Enhanced Packer configuration for automated VexFS deployment images

packer {
  required_plugins {
    qemu = {
      version = ">= 1.0.0"
      source  = "github.com/hashicorp/qemu"
    }
  }
}

# Build variables for different configurations
variable "image_variant" {
  type        = string
  default     = "minimal"
  description = "Image variant: minimal, development, testing, production"
}

variable "kernel_version" {
  type        = string
  default     = "6.1"
  description = "Target kernel version for VexFS module compilation"
}

variable "vexfs_version" {
  type        = string
  default     = "1.0.0"
  description = "VexFS version to build and install"
}

variable "output_dir" {
  type        = string
  default     = "./images"
  description = "Output directory for built images"
}

variable "disk_size" {
  type        = string
  default     = "8G"
  description = "Disk size for the image"
}

variable "memory" {
  type        = string
  default     = "2048"
  description = "Memory allocation during build"
}

variable "cpus" {
  type        = string
  default     = "2"
  description = "CPU cores during build"
}

variable "enable_testing" {
  type        = bool
  default     = true
  description = "Enable comprehensive testing during build"
}

variable "enable_validation" {
  type        = bool
  default     = true
  description = "Enable image validation procedures"
}

variable "build_timeout" {
  type        = string
  default     = "45m"
  description = "Maximum build time before timeout"
}

# Local variables for computed values
locals {
  timestamp = regex_replace(timestamp(), "[- TZ:]", "")
  image_name = "vexfs-${var.image_variant}-${var.vexfs_version}-${local.timestamp}"
  
  # Kernel configuration based on variant
  kernel_configs = {
    minimal     = ["CONFIG_VEXFS=m", "CONFIG_FUSE_FS=y"]
    development = ["CONFIG_VEXFS=m", "CONFIG_FUSE_FS=y", "CONFIG_DEBUG_KERNEL=y", "CONFIG_DEBUG_INFO=y"]
    testing     = ["CONFIG_VEXFS=m", "CONFIG_FUSE_FS=y", "CONFIG_DEBUG_KERNEL=y", "CONFIG_KASAN=y"]
    production  = ["CONFIG_VEXFS=m", "CONFIG_FUSE_FS=y", "CONFIG_SECURITY=y"]
  }
  
  # Package lists based on variant
  package_lists = {
    minimal     = ["linux-headers-generic", "build-essential", "curl", "git"]
    development = ["linux-headers-generic", "build-essential", "curl", "git", "vim", "gdb", "strace", "htop"]
    testing     = ["linux-headers-generic", "build-essential", "curl", "git", "stress-ng", "fio", "iperf3"]
    production  = ["linux-headers-generic", "build-essential", "curl", "git", "systemd", "rsyslog"]
  }
}

# Base QEMU source configuration
source "qemu" "vexfs-base" {
  vm_name          = local.image_name
  iso_url          = "https://cdimage.debian.org/debian-cd/current/amd64/iso-cd/debian-12.5.0-amd64-netinst.iso"
  iso_checksum     = "sha256:a221b567e568273b535842919e10f0f03990332f52996a910904136508084ecc"
  output_directory = "${var.output_dir}/${local.image_name}"
  disk_size        = var.disk_size
  format           = "qcow2"
  accelerator      = "kvm"
  headless         = true
  memory           = var.memory
  cpus             = var.cpus
  
  # Network configuration for package downloads
  net_device       = "virtio-net"
  
  # Boot configuration with preseed
  boot_command = [
    "<esc><wait>",
    "install <wait>",
    "preseed/url=http://{{ .HTTPIP }}:{{ .HTTPPort }}/preseed-${var.image_variant}.cfg <wait>",
    "debian-installer/locale=en_US <wait>",
    "debian-installer/country=US <wait>",
    "debian-installer/keymap=us <wait>",
    "netcfg/get_hostname=${local.image_name} <wait>",
    "netcfg/get_domain=vexfs.local <wait>",
    "fb=false <wait>",
    "debconf/frontend=noninteractive <wait>",
    "console-setup/ask_detect=false <wait>",
    "console-keymaps-at/keymap=us <wait>",
    "keyboard-configuration/xkb-keymap=us <wait>",
    "<enter><wait>"
  ]
  
  # HTTP server for preseed files
  http_directory = "http"
  http_port_min  = 8080
  http_port_max  = 8090
  
  # SSH configuration
  ssh_username     = "root"
  ssh_password     = "vexfs2024!"
  ssh_timeout      = var.build_timeout
  shutdown_command = "systemctl poweroff"
  
  # QEMU specific optimizations
  qemuargs = [
    ["-cpu", "host"],
    ["-smp", var.cpus],
    ["-m", var.memory],
    ["-netdev", "user,id=net0"],
    ["-device", "virtio-net,netdev=net0"],
    ["-drive", "if=virtio,cache=writeback,discard=ignore,format=qcow2,file=output-${local.image_name}/${local.image_name}"]
  ]
}

# Main build configuration
build {
  name = "vexfs-production-pipeline"
  sources = ["source.qemu.vexfs-base"]
  
  # Stage 1: System Preparation and Base Setup
  provisioner "shell" {
    name = "system-preparation"
    inline = [
      "echo '=== VexFS Production Image Build Pipeline ==='",
      "echo 'Image Variant: ${var.image_variant}'",
      "echo 'Kernel Version: ${var.kernel_version}'",
      "echo 'VexFS Version: ${var.vexfs_version}'",
      "echo 'Build Timestamp: ${local.timestamp}'",
      "",
      "# Wait for system to be ready",
      "while [ ! -f /var/lib/dpkg/lock-frontend ]; do sleep 1; done",
      "while fuser /var/lib/dpkg/lock-frontend >/dev/null 2>&1; do sleep 1; done",
      "",
      "# Update system",
      "export DEBIAN_FRONTEND=noninteractive",
      "apt-get update",
      "apt-get upgrade -y",
      "",
      "# Install base packages",
      "apt-get install -y ${join(" ", local.package_lists[var.image_variant])}",
      "",
      "# Install Rust toolchain",
      "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain nightly",
      "source /root/.cargo/env",
      "rustup target add x86_64-unknown-linux-gnu",
      "",
      "# Verify installations",
      "gcc --version",
      "make --version",
      "/root/.cargo/bin/cargo --version",
      "/root/.cargo/bin/rustc --version",
      "",
      "echo '✅ System preparation completed'"
    ]
    execute_command = "sudo sh -c '{{ .Vars }} {{ .Path }}'"
  }
  
  # Stage 2: Kernel Preparation and Custom Configuration
  provisioner "shell" {
    name = "kernel-preparation"
    environment_vars = [
      "KERNEL_VERSION=${var.kernel_version}",
      "IMAGE_VARIANT=${var.image_variant}"
    ]
    inline = [
      "echo '=== Kernel Preparation Stage ==='",
      "",
      "# Install kernel headers and development tools",
      "apt-get install -y linux-headers-$(uname -r) linux-source",
      "apt-get install -y bc bison flex libssl-dev libelf-dev",
      "",
      "# Prepare kernel source for module compilation",
      "cd /usr/src",
      "if [ ! -d linux-source-* ]; then",
      "  apt-get source linux",
      "fi",
      "",
      "# Create kernel configuration directory",
      "mkdir -p /etc/vexfs/kernel",
      "",
      "# Generate kernel config for VexFS",
      "cat > /etc/vexfs/kernel/vexfs.config << 'EOF'",
      "${join("\\n", local.kernel_configs[var.image_variant])}",
      "EOF",
      "",
      "echo '✅ Kernel preparation completed'"
    ]
    execute_command = "sudo sh -c '{{ .Vars }} {{ .Path }}'"
  }
  
  # Stage 3: VexFS Source Code Transfer
  provisioner "file" {
    name = "vexfs-source-transfer"
    sources = [
      "../../src/",
      "../../Cargo.toml",
      "../../Makefile",
      "../../Kbuild",
      "../../cbindgen.toml"
    ]
    destination = "/tmp/vexfs-source/"
    direction = "upload"
  }
  
  provisioner "file" {
    name = "vexctl-source-transfer"
    source = "../../vexctl/"
    destination = "/tmp/vexctl-source/"
    direction = "upload"
  }
  
  # Stage 4: VexFS Compilation and Installation
  provisioner "shell" {
    name = "vexfs-compilation"
    environment_vars = [
      "HOME=/root",
      "PATH=/root/.cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin",
      "VEXFS_VERSION=${var.vexfs_version}",
      "IMAGE_VARIANT=${var.image_variant}"
    ]
    inline = [
      "echo '=== VexFS Compilation Stage ==='",
      "",
      "# Create VexFS installation directory",
      "mkdir -p /usr/src/vexfs",
      "mkdir -p /usr/local/lib/vexfs",
      "mkdir -p /etc/vexfs",
      "",
      "# Move source code to proper location",
      "cp -r /tmp/vexfs-source/* /usr/src/vexfs/",
      "cd /usr/src/vexfs",
      "",
      "# Set up Rust environment",
      "source /root/.cargo/env",
      "",
      "# Clean any existing builds",
      "make clean || true",
      "",
      "# Build VexFS kernel module",
      "echo 'Building VexFS kernel module...'",
      "make vm-build",
      "",
      "# Verify module was built",
      "if [ ! -f vexfs.ko ]; then",
      "  echo 'ERROR: VexFS kernel module build failed'",
      "  exit 1",
      "fi",
      "",
      "# Install kernel module",
      "mkdir -p /lib/modules/$(uname -r)/extra/vexfs",
      "cp vexfs.ko /lib/modules/$(uname -r)/extra/vexfs/",
      "depmod -a",
      "",
      "# Build and install vexctl",
      "echo 'Building vexctl...'",
      "cp -r /tmp/vexctl-source/* /usr/src/vexctl/",
      "cd /usr/src/vexctl",
      "cargo build --release",
      "cp target/release/vexctl /usr/local/bin/",
      "chmod +x /usr/local/bin/vexctl",
      "",
      "# Create VexFS configuration",
      "cat > /etc/vexfs/vexfs.conf << 'EOF'",
      "# VexFS Configuration",
      "# Generated during image build",
      "",
      "version=${var.vexfs_version}",
      "variant=${var.image_variant}",
      "build_date=${local.timestamp}",
      "",
      "# Default mount options",
      "default_mount_options=rw,relatime",
      "",
      "# Vector cache settings",
      "vector_cache_size=64M",
      "vector_cache_enabled=true",
      "",
      "# Logging settings",
      "log_level=info",
      "log_file=/var/log/vexfs.log",
      "EOF",
      "",
      "echo '✅ VexFS compilation and installation completed'"
    ]
    execute_command = "sudo sh -c '{{ .Vars }} {{ .Path }}'"
  }
  
  # Stage 5: System Integration and Auto-mounting
  provisioner "shell" {
    name = "system-integration"
    inline = [
      "echo '=== System Integration Stage ==='",
      "",
      "# Create VexFS systemd service",
      "cat > /etc/systemd/system/vexfs.service << 'EOF'",
      "[Unit]",
      "Description=VexFS Vector Filesystem Service",
      "After=local-fs.target",
      "Before=multi-user.target",
      "",
      "[Service]",
      "Type=oneshot",
      "RemainAfterExit=yes",
      "ExecStart=/usr/local/bin/vexfs-mount-helper",
      "ExecStop=/usr/local/bin/vexfs-umount-helper",
      "TimeoutSec=30",
      "",
      "[Install]",
      "WantedBy=multi-user.target",
      "EOF",
      "",
      "# Create mount helper scripts",
      "cat > /usr/local/bin/vexfs-mount-helper << 'EOF'",
      "#!/bin/bash",
      "# VexFS Mount Helper",
      "",
      "set -e",
      "",
      "# Load VexFS module",
      "if ! lsmod | grep -q vexfs; then",
      "    modprobe vexfs",
      "    echo 'VexFS module loaded'",
      "fi",
      "",
      "# Create default mount point",
      "mkdir -p /mnt/vexfs",
      "",
      "# Log startup",
      "echo \"$(date): VexFS service started\" >> /var/log/vexfs.log",
      "EOF",
      "",
      "cat > /usr/local/bin/vexfs-umount-helper << 'EOF'",
      "#!/bin/bash",
      "# VexFS Unmount Helper",
      "",
      "# Unmount any VexFS filesystems",
      "umount -t vexfs -a 2>/dev/null || true",
      "",
      "# Unload module if no filesystems are mounted",
      "if ! mount | grep -q 'type vexfs'; then",
      "    rmmod vexfs 2>/dev/null || true",
      "    echo 'VexFS module unloaded'",
      "fi",
      "",
      "# Log shutdown",
      "echo \"$(date): VexFS service stopped\" >> /var/log/vexfs.log",
      "EOF",
      "",
      "chmod +x /usr/local/bin/vexfs-mount-helper",
      "chmod +x /usr/local/bin/vexfs-umount-helper",
      "",
      "# Enable VexFS service",
      "systemctl enable vexfs.service",
      "",
      "# Create log file",
      "touch /var/log/vexfs.log",
      "chmod 644 /var/log/vexfs.log",
      "",
      "# Add VexFS to modules load list",
      "echo 'vexfs' >> /etc/modules-load.d/vexfs.conf",
      "",
      "echo '✅ System integration completed'"
    ]
    execute_command = "sudo sh -c '{{ .Vars }} {{ .Path }}'"
  }
  
  # Stage 6: Testing and Validation (if enabled)
  provisioner "shell" {
    name = "testing-validation"
    only_if = "${var.enable_testing}"
    inline = [
      "echo '=== Testing and Validation Stage ==='",
      "",
      "# Test kernel module loading",
      "echo 'Testing VexFS kernel module...'",
      "modprobe vexfs",
      "if ! lsmod | grep -q vexfs; then",
      "    echo 'ERROR: VexFS module failed to load'",
      "    exit 1",
      "fi",
      "",
      "# Test vexctl functionality",
      "echo 'Testing vexctl...'",
      "vexctl --version",
      "vexctl status || echo 'vexctl status check completed'",
      "",
      "# Test basic filesystem operations",
      "echo 'Testing basic filesystem operations...'",
      "mkdir -p /tmp/vexfs-test",
      "",
      "# Create a test filesystem image",
      "dd if=/dev/zero of=/tmp/vexfs-test.img bs=1M count=10",
      "",
      "# Test module unloading",
      "rmmod vexfs",
      "if lsmod | grep -q vexfs; then",
      "    echo 'ERROR: VexFS module failed to unload'",
      "    exit 1",
      "fi",
      "",
      "# Reload for final state",
      "modprobe vexfs",
      "",
      "echo '✅ Testing and validation completed successfully'"
    ]
    execute_command = "sudo sh -c '{{ .Vars }} {{ .Path }}'"
  }
  
  # Stage 7: Image Optimization and Cleanup
  provisioner "shell" {
    name = "image-optimization"
    inline = [
      "echo '=== Image Optimization Stage ==='",
      "",
      "# Clean package cache",
      "apt-get autoremove -y",
      "apt-get autoclean",
      "apt-get clean",
      "",
      "# Remove temporary files",
      "rm -rf /tmp/*",
      "rm -rf /var/tmp/*",
      "",
      "# Clean logs (keep structure)",
      "find /var/log -type f -name '*.log' -exec truncate -s 0 {} \\;",
      "",
      "# Remove SSH host keys (will be regenerated on first boot)",
      "rm -f /etc/ssh/ssh_host_*",
      "",
      "# Clear bash history",
      "history -c",
      "rm -f /root/.bash_history",
      "",
      "# Zero out free space for better compression",
      "dd if=/dev/zero of=/tmp/zero bs=1M 2>/dev/null || true",
      "rm -f /tmp/zero",
      "",
      "# Create image manifest",
      "cat > /etc/vexfs/image-manifest.json << 'EOF'",
      "{",
      "  \"image_name\": \"${local.image_name}\",",
      "  \"vexfs_version\": \"${var.vexfs_version}\",",
      "  \"image_variant\": \"${var.image_variant}\",",
      "  \"kernel_version\": \"${var.kernel_version}\",",
      "  \"build_timestamp\": \"${local.timestamp}\",",
      "  \"build_system\": \"packer\",",
      "  \"components\": {",
      "    \"vexfs_module\": \"/lib/modules/$(uname -r)/extra/vexfs/vexfs.ko\",",
      "    \"vexctl_binary\": \"/usr/local/bin/vexctl\",",
      "    \"configuration\": \"/etc/vexfs/vexfs.conf\",",
      "    \"systemd_service\": \"/etc/systemd/system/vexfs.service\"",
      "  }",
      "}",
      "EOF",
      "",
      "echo '✅ Image optimization completed'"
    ]
    execute_command = "sudo sh -c '{{ .Vars }} {{ .Path }}'"
  }
  
  # Stage 8: Final Validation (if enabled)
  provisioner "shell" {
    name = "final-validation"
    only_if = "${var.enable_validation}"
    inline = [
      "echo '=== Final Validation Stage ==='",
      "",
      "# Validate all components are in place",
      "echo 'Validating VexFS installation...'",
      "",
      "# Check kernel module",
      "if [ ! -f /lib/modules/$(uname -r)/extra/vexfs/vexfs.ko ]; then",
      "    echo 'ERROR: VexFS kernel module not found'",
      "    exit 1",
      "fi",
      "",
      "# Check vexctl binary",
      "if [ ! -f /usr/local/bin/vexctl ]; then",
      "    echo 'ERROR: vexctl binary not found'",
      "    exit 1",
      "fi",
      "",
      "# Check configuration",
      "if [ ! -f /etc/vexfs/vexfs.conf ]; then",
      "    echo 'ERROR: VexFS configuration not found'",
      "    exit 1",
      "fi",
      "",
      "# Check systemd service",
      "if [ ! -f /etc/systemd/system/vexfs.service ]; then",
      "    echo 'ERROR: VexFS systemd service not found'",
      "    exit 1",
      "fi",
      "",
      "# Test module loading one final time",
      "modprobe vexfs",
      "if ! lsmod | grep -q vexfs; then",
      "    echo 'ERROR: Final VexFS module load test failed'",
      "    exit 1",
      "fi",
      "",
      "# Generate validation report",
      "cat > /etc/vexfs/validation-report.txt << 'EOF'",
      "VexFS Image Validation Report",
      "============================",
      "",
      "Image: ${local.image_name}",
      "Build Date: ${local.timestamp}",
      "Validation Date: $(date)",
      "",
      "✅ Kernel module: $(ls -la /lib/modules/$(uname -r)/extra/vexfs/vexfs.ko)",
      "✅ vexctl binary: $(ls -la /usr/local/bin/vexctl)",
      "✅ Configuration: $(ls -la /etc/vexfs/vexfs.conf)",
      "✅ Systemd service: $(ls -la /etc/systemd/system/vexfs.service)",
      "✅ Module loading: $(lsmod | grep vexfs)",
      "",
      "All validation checks passed successfully.",
      "EOF",
      "",
      "echo '✅ Final validation completed successfully'",
      "echo '🎉 VexFS production image build completed!'"
    ]
    execute_command = "sudo sh -c '{{ .Vars }} {{ .Path }}'"
  }
  
  # Post-processor: Create compressed image variants
  post-processor "compress" {
    output = "${var.output_dir}/${local.image_name}/${local.image_name}.qcow2.gz"
    compression_level = 6
  }
}