packer {
  required_plugins {
    qemu = {
      version = ">= 1.0.0"
      source  = "github.com/hashicorp/qemu"
    }
  }
}

variable "vm_name" {
  type    = string
  default = "vexfs-dev-vm"
}

variable "output_dir" {
  type    = string
  default = "./packer_output"
}

variable "disk_size" {
  type    = string
  default = "10G" # Increased for kernel headers and build tools
}

variable "memory" {
  type    = string
  default = "2048"
}

variable "cpus" {
  type    = string
  default = "2"
}

source "qemu" "vexfs" {
  vm_name               = var.vm_name
  iso_url               = "https://cdimage.debian.org/debian-cd/current/amd64/iso-cd/debian-12.5.0-amd64-netinst.iso"
  iso_checksum          = "sha256:a221b567e568273b535842919e10f0f03990332f52996a910904136508084ecc" # For debian-12.5.0-amd64-netinst.iso
  output_directory      = var.output_dir
  disk_size             = var.disk_size
  format                = "qcow2"
  accelerator           = "kvm" # Use KVM for better performance
  headless              = false # Set to true for CI/headless environments initially false for easier setup
  memory                = var.memory
  cpus                  = var.cpus

  # Boot command for Debian preseed
  # This is a simplified preseed example for non-interactive installation.
  # A full preseed file would be more robust.
  boot_command = [
    "<esc><wait>",
    "install <wait>",
    "preseed/url=http://{{ .HTTPIP }}:{{ .HTTPPort }}/preseed.cfg <wait>",
    "debian-installer/locale=en_US <wait>",
    "debian-installer/country=US <wait>",
    "debian-installer/keymap=us <wait>",
    "netcfg/get_hostname=vexfs-vm <wait>",
    "netcfg/get_domain=localdomain <wait>",
    "fb=false <wait>",
    "debconf/frontend=noninteractive <wait>",
    "console-setup/ask_detect=false <wait>",
    "console-keymaps-at/keymap=us <wait>",
    "keyboard-configuration/xkb-keymap=us <wait>",
    "<enter><wait>"
  ]

  # HTTP server to serve preseed file
  http_directory = "http"
  http_port_min  = 8080
  http_port_max  = 8080


  # QEMU connection options
  ssh_username         = "root" # Debian installer default is root with empty pass for rescue/auto mode, or user created by preseed
  ssh_password         = "password" # Replace with a secure password or use SSH key (password set in preseed)
  ssh_timeout          = "20m"    # Increased timeout for OS installation
  shutdown_command     = "sudo /sbin/halt -p"
  
  # qemuargs allows specifying additional QEMU parameters
  # Example: qemuargs = [ [ "-m", "2048M" ], [ "-smp", "2" ] ]
  # These are already covered by memory and cpus fields above.
}

build {
  sources = ["source.qemu.vexfs"]

  provisioner "shell" {
    inline = [
      "echo 'Waiting for cloud-init to finish...'",
      "while [ ! -f /var/lib/cloud/instance/boot-finished ]; do echo 'Waiting...'; sleep 1; done",
      "echo 'Cloud-init finished.'",
      "apt-get update",
      # Install sudo if not present (common in minimal installs if not using root directly)
      "DEBIAN_FRONTEND=noninteractive apt-get install -y sudo",
      "DEBIAN_FRONTEND=noninteractive apt-get install -y make gcc curl git",
      # Install kernel headers for the currently running kernel (Debian 6.1 default)
      "DEBIAN_FRONTEND=noninteractive apt-get install -y linux-headers-$(uname -r)",
      # Install Rust via rustup.sh
      "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain nightly",
      "echo 'source $HOME/.cargo/env' >> /root/.bashrc", # Assuming SSH as root
      "source $HOME/.cargo/env",
      # Verify cargo installation
      "/root/.cargo/bin/cargo --version",
      "/root/.cargo/bin/rustc --version",
    ]
    execute_command = "sudo sh -c '{{ .Vars }} {{ .Path }}'"
  }
  
  provisioner "shell" {
    inline = [
      "mkdir -p /usr/src/vexfs",
    ]
    execute_command = "sudo sh -c '{{ .Vars }} {{ .Path }}'"
  }

  provisioner "file" {
    source      = "../../vexfs/" # Path to the vexfs directory relative to test_env directory
    destination = "/usr/src/vexfs_temp" # Temporary location
    direction   = "upload"
  }

  provisioner "shell" {
    inline = [
      # Move from temp to final to handle potential ownership/permission issues with 'file' provisioner directly into /usr/src
      "mv /usr/src/vexfs_temp/* /usr/src/vexfs/",
      "rmdir /usr/src/vexfs_temp",
      "ls -la /usr/src/vexfs", # Verify files are copied
    ]
    execute_command = "sudo sh -c '{{ .Vars }} {{ .Path }}'"
  }

  provisioner "shell" {
    environment_vars = ["HOME=/root", "PATH=/root/.cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"]
    inline = [
      "cd /usr/src/vexfs",
      "echo 'Current directory:' $(pwd)",
      "echo 'Listing files:' $(ls -la)",
      "echo 'Checking PATH:' $PATH",
      "echo 'Checking cargo version from provisioner:'",
      "cargo --version", # Test if cargo is in PATH
      "echo 'Building VexFS kernel module...'",
      "make",
      "echo 'Build complete. Listing files:'",
      "ls -la *.ko",
    ]
    execute_command = "sudo sh -c '{{ .Vars }} {{ .Path }}'"
  }

  # Provision vexctl
  provisioner "shell" {
    inline = [
      "mkdir -p /usr/src/vexctl",
    ]
    execute_command = "sudo sh -c '{{ .Vars }} {{ .Path }}'"
  }

  provisioner "file" {
    source      = "../../vexctl/" # Path to the vexctl directory relative to test_env
    destination = "/usr/src/vexctl_temp"
    direction   = "upload"
  }

  provisioner "shell" {
    inline = [
      "mv /usr/src/vexctl_temp/* /usr/src/vexctl/",
      "rmdir /usr/src/vexctl_temp",
      "ls -la /usr/src/vexctl",
    ]
    execute_command = "sudo sh -c '{{ .Vars }} {{ .Path }}'"
  }

  provisioner "shell" {
    environment_vars = ["HOME=/root", "PATH=/root/.cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"]
    inline = [
      "cd /usr/src/vexctl",
      "echo 'Current directory for vexctl build:' $(pwd)",
      "echo 'Listing files in vexctl src:' $(ls -la .)",
      "echo 'Building vexctl...'",
      "cargo build --release",
      "echo 'vexctl build complete.'",
      "echo 'Copying vexctl to /usr/local/bin'",
      "cp target/release/vexctl /usr/local/bin/",
      "ls -la /usr/local/bin/vexctl",
      "vexctl --version", # Test if vexctl is accessible and runs
    ]
    execute_command = "sudo sh -c '{{ .Vars }} {{ .Path }}'"
  }
}
