#!/bin/sh
# Alpine Linux automated setup script

echo "Starting Alpine Linux automated setup..."

# Answer setup-alpine questions automatically
cat > /tmp/answerfile << EOF
KEYMAPOPTS="us us"
HOSTNAMEOPTS="-n vexfs-test"
INTERFACESOPTS="auto lo
iface lo inet loopback

auto eth0
iface eth0 inet dhcp
"
DNSOPTS="-n 8.8.8.8"
TIMEZONEOPTS="-z UTC"
PROXYOPTS="none"
APKREPOSOPTS="-1"
SSHDOPTS="-c openssh"
NTPOPTS="-c openntpd"
DISKOPTS="-m sys /dev/vda"
EOF

# Set root password to 'vexfs'
echo -e "vexfs\nvexfs" | passwd root

# Run setup with answerfile
setup-alpine -f /tmp/answerfile

# Install additional packages
apk add --no-cache \
    build-base \
    linux-headers \
    linux-virt-dev \
    bash \
    sudo \
    util-linux \
    e2fsprogs \
    git

# Create vexfs user
adduser -D -s /bin/bash vexfs
echo -e "vexfs\nvexfs" | passwd vexfs
echo "vexfs ALL=(ALL) NOPASSWD: ALL" >> /etc/sudoers

# Enable SSH root login (for testing only)
sed -i 's/^#PermitRootLogin.*/PermitRootLogin yes/' /etc/ssh/sshd_config
rc-service sshd restart

# Setup 9p mount for shared directory
mkdir -p /mnt/shared
echo "shared /mnt/shared 9p trans=virtio,version=9p2000.L,rw,_netdev 0 0" >> /etc/fstab

echo "Alpine setup complete! Rebooting..."
reboot
