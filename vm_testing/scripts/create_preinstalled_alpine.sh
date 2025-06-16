#!/bin/bash

# Create a pre-installed Alpine Linux image for VexFS testing
# This bypasses the manual installation step

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
VM_DIR="$(dirname "$SCRIPT_DIR")"
PROJECT_ROOT="$(dirname "$VM_DIR")"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info() { echo -e "${BLUE}ℹ️  $1${NC}"; }
log_success() { echo -e "${GREEN}✅ $1${NC}"; }
log_warning() { echo -e "${YELLOW}⚠️  $1${NC}"; }
log_error() { echo -e "${RED}❌ $1${NC}"; }

# Alpine configuration
ALPINE_VERSION="3.19"
ALPINE_MIRROR="https://dl-cdn.alpinelinux.org/alpine/v${ALPINE_VERSION}"
WORK_DIR="/tmp/alpine_vexfs_build"
DISK_IMAGE="$VM_DIR/images/vexfs_alpine_preinstalled.qcow2"

log_info "Creating pre-installed Alpine Linux image..."

# Clean up previous attempts
rm -rf "$WORK_DIR"
mkdir -p "$WORK_DIR"

# Create disk image
log_info "Creating 2GB disk image..."
qemu-img create -f qcow2 "$DISK_IMAGE" 2G

# Download Alpine mini root filesystem
log_info "Downloading Alpine mini root filesystem..."
ROOTFS_URL="${ALPINE_MIRROR}/releases/x86_64/alpine-minirootfs-${ALPINE_VERSION}.0-x86_64.tar.gz"
wget -q -O "$WORK_DIR/alpine-minirootfs.tar.gz" "$ROOTFS_URL" || {
    log_error "Failed to download Alpine rootfs"
    exit 1
}

# Create a script to build the image using Docker/Podman
cat > "$WORK_DIR/build_alpine_image.sh" << 'EOF'
#!/bin/sh
# This runs inside an Alpine container to build our image

set -e

# Setup Alpine
setup-apkrepos -1
apk update
apk add --no-cache \
    linux-virt \
    openrc \
    openssh \
    e2fsprogs \
    util-linux \
    bash \
    sudo \
    build-base \
    linux-headers

# Configure system
rc-update add devfs sysinit
rc-update add dmesg sysinit
rc-update add mdev sysinit
rc-update add hwclock boot
rc-update add modules boot
rc-update add sysctl boot
rc-update add hostname boot
rc-update add bootmisc boot
rc-update add syslog boot
rc-update add networking boot
rc-update add sshd default
rc-update add local default

# Set hostname
echo "vexfs-test" > /etc/hostname

# Configure networking
cat > /etc/network/interfaces << NETEOF
auto lo
iface lo inet loopback

auto eth0
iface eth0 inet dhcp
NETEOF

# Set root password
echo "root:vexfs" | chpasswd

# Create vexfs user
adduser -D -s /bin/bash vexfs
echo "vexfs:vexfs" | chpasswd
echo "vexfs ALL=(ALL) NOPASSWD: ALL" >> /etc/sudoers

# Configure SSH
sed -i 's/^#PermitRootLogin.*/PermitRootLogin yes/' /etc/ssh/sshd_config
sed -i 's/^#PasswordAuthentication.*/PasswordAuthentication yes/' /etc/ssh/sshd_config

# Create mount point for shared directory
mkdir -p /mnt/shared
echo "shared /mnt/shared 9p trans=virtio,version=9p2000.L,rw,_netdev 0 0" >> /etc/fstab

# Configure serial console
sed -i 's/^tty1::respawn/# tty1::respawn/' /etc/inittab
echo "ttyS0::respawn:/sbin/getty -L ttyS0 115200 vt100" >> /etc/inittab

# Create auto-login script
cat > /etc/profile.d/vexfs_welcome.sh << WELCOMEEOF
#!/bin/sh
if [ "\$(tty)" = "/dev/ttyS0" ]; then
    echo "Welcome to VexFS Alpine Test VM!"
    echo "Root password: vexfs"
    echo "Shared directory: /mnt/shared"
fi
WELCOMEEOF
chmod +x /etc/profile.d/vexfs_welcome.sh
EOF

log_info "Building Alpine image (this may take a moment)..."

# Use a simple chroot approach instead
MOUNT_DIR="$WORK_DIR/mount"
mkdir -p "$MOUNT_DIR"

# Mount the disk image
LOOP_DEV=$(sudo losetup --find --show "$DISK_IMAGE")
sudo parted -s "$LOOP_DEV" mklabel msdos
sudo parted -s "$LOOP_DEV" mkpart primary ext4 1M 100%
sudo partprobe "$LOOP_DEV"
sudo mkfs.ext4 "${LOOP_DEV}p1"
sudo mount "${LOOP_DEV}p1" "$MOUNT_DIR"

# Extract Alpine rootfs
log_info "Extracting Alpine rootfs..."
sudo tar -xzf "$WORK_DIR/alpine-minirootfs.tar.gz" -C "$MOUNT_DIR"

# Configure the system
log_info "Configuring Alpine system..."
sudo cp /etc/resolv.conf "$MOUNT_DIR/etc/"

# Install packages and configure
sudo chroot "$MOUNT_DIR" /bin/sh << 'CHROOT_EOF'
# Configure APK repositories
cat > /etc/apk/repositories << EOF
https://dl-cdn.alpinelinux.org/alpine/v3.19/main
https://dl-cdn.alpinelinux.org/alpine/v3.19/community
EOF

# Update and install packages
apk update
apk add --no-cache \
    linux-virt \
    openrc \
    openssh \
    e2fsprogs \
    util-linux \
    bash \
    sudo \
    build-base \
    linux-headers \
    syslinux

# Configure boot
dd if=/usr/share/syslinux/mbr.bin of=/dev/loop0 bs=440 count=1
extlinux --install /boot

cat > /boot/extlinux.conf << EOF
DEFAULT vexfs
PROMPT 0
TIMEOUT 10

LABEL vexfs
    LINUX /boot/vmlinuz-virt
    INITRD /boot/initramfs-virt
    APPEND root=/dev/vda1 rw console=ttyS0,115200
EOF

# Set up services
rc-update add devfs sysinit
rc-update add dmesg sysinit
rc-update add hwclock boot
rc-update add modules boot
rc-update add sysctl boot
rc-update add hostname boot
rc-update add bootmisc boot
rc-update add networking boot
rc-update add sshd default

# Configure system
echo "vexfs-test" > /etc/hostname
echo "root:vexfs" | chpasswd

# Configure networking
cat > /etc/network/interfaces << EOF
auto lo
iface lo inet loopback

auto eth0
iface eth0 inet dhcp
EOF

# Configure SSH
sed -i 's/^#PermitRootLogin.*/PermitRootLogin yes/' /etc/ssh/sshd_config

# Create mount point
mkdir -p /mnt/shared
echo "shared /mnt/shared 9p trans=virtio,version=9p2000.L,rw,_netdev 0 0" >> /etc/fstab

# Serial console
echo "ttyS0::respawn:/sbin/getty -L ttyS0 115200 vt100" >> /etc/inittab
CHROOT_EOF

# Clean up
sudo umount "$MOUNT_DIR"
sudo losetup -d "$LOOP_DEV"

log_success "Pre-installed Alpine image created!"

# Create start script for pre-installed image
cat > "$VM_DIR/scripts/start_preinstalled_alpine.sh" << 'EOF'
#!/bin/bash

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
VM_DIR="$(dirname "$SCRIPT_DIR")"
PROJECT_ROOT="$(dirname "$VM_DIR")"

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m'

echo -e "${BLUE}Starting pre-installed Alpine VM...${NC}"

# Copy kernel module to shared
if [ -f "$PROJECT_ROOT/kernel_module/vexfs_deadlock_fix.ko" ]; then
    cp "$PROJECT_ROOT/kernel_module/vexfs_deadlock_fix.ko" "$VM_DIR/shared/"
fi

# Start VM
qemu-system-x86_64 \
    -name "vexfs-alpine-test" \
    -machine type=pc,accel=kvm \
    -cpu host \
    -smp 2 \
    -m 2048 \
    -drive file="$VM_DIR/images/vexfs_alpine_preinstalled.qcow2",format=qcow2,if=virtio \
    -netdev user,id=net0,hostfwd=tcp::2222-:22 \
    -device virtio-net-pci,netdev=net0 \
    -virtfs local,path="$VM_DIR/shared",mount_tag=shared,security_model=passthrough,id=shared \
    -nographic \
    -serial mon:stdio \
    -daemonize \
    -pidfile "$VM_DIR/qemu.pid"

echo -e "${GREEN}✅ VM started!${NC}"
echo "SSH: ssh -p 2222 root@localhost (password: vexfs)"
echo "Stop: kill \$(cat $VM_DIR/qemu.pid)"
EOF

chmod +x "$VM_DIR/scripts/start_preinstalled_alpine.sh"

log_info "To use the pre-installed image:"
log_info "1. Start VM: $VM_DIR/scripts/start_preinstalled_alpine.sh"
log_info "2. SSH directly: ssh -p 2222 root@localhost"
log_info "3. Run tests: /mnt/shared/run_all_tests.sh"