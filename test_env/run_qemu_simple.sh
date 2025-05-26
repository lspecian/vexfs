#!/bin/bash
set -e

# VexFS Simple QEMU Runner - Fast Development Environment
# This script starts a VM with live source mounting for rapid iteration

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

# Configuration
VM_DIR="$SCRIPT_DIR/vm"
VM_IMAGE="$VM_DIR/vexfs-vm-base.qcow2"
CLOUD_INIT_ISO="$VM_DIR/cloud-init.iso"
EXTRA_DISK="$VM_DIR/vexfs_test_disk.img"

# VM Settings
VM_MEMORY="${VEXFS_VM_MEMORY:-4G}"
VM_CPUS="${VEXFS_VM_CPUS:-4}"
VM_SSH_PORT="${VEXFS_VM_SSH_PORT:-2222}"

# Modes
TEST_MODE="${1:-interactive}"
SOURCE_PATH="${2:-$PROJECT_ROOT}"

echo "🚀 VexFS Simple VM Runner"
echo "========================="
echo "Mode: $TEST_MODE"
echo "Source path: $SOURCE_PATH"
echo "VM Memory: $VM_MEMORY"
echo "VM CPUs: $VM_CPUS"
echo "SSH Port: $VM_SSH_PORT"

# Check if VM image exists
if [ ! -f "$VM_IMAGE" ]; then
    echo "❌ VM image not found: $VM_IMAGE"
    echo "Run './test_env/setup_vm.sh' first to create the VM image."
    exit 1
fi

# Check if cloud-init ISO exists
if [ ! -f "$CLOUD_INIT_ISO" ]; then
    echo "❌ Cloud-init ISO not found: $CLOUD_INIT_ISO"
    echo "Run './test_env/setup_vm.sh' first to create the cloud-init configuration."
    exit 1
fi

# Create extra disk for VexFS testing if it doesn't exist
if [ ! -f "$EXTRA_DISK" ]; then
    echo "💾 Creating test disk for VexFS..."
    qemu-img create -f raw "$EXTRA_DISK" 1G
fi

# Build QEMU command
QEMU_CMD=(
    qemu-system-x86_64
    -enable-kvm
    -m "$VM_MEMORY"
    -smp "$VM_CPUS"
    -drive "file=$VM_IMAGE,if=virtio,format=qcow2,index=0,media=disk"
    -drive "file=$CLOUD_INIT_ISO,if=virtio,format=raw,index=1,media=cdrom"
    -drive "file=$EXTRA_DISK,if=virtio,format=raw,index=2,media=disk"
    -netdev "user,id=net0,hostfwd=tcp::$VM_SSH_PORT-:22"
    -device "virtio-net-pci,netdev=net0"
    -virtfs "local,path=$SOURCE_PATH,mount_tag=vexfs-src,security_model=passthrough,id=vexfs-src"
    -device "virtio-rng-pci"
)

# Configure display based on mode
case "$TEST_MODE" in
    "test"|"headless")
        QEMU_CMD+=(-nographic -serial mon:stdio)
        echo "🖥️  Starting VM in headless mode..."
        ;;
    "interactive"|*)
        QEMU_CMD+=(-display gtk -serial pty)
        echo "🖥️  Starting VM in interactive mode..."
        echo "💡 Connect via SSH: ssh vexfs@localhost -p $VM_SSH_PORT"
        echo "🔑 Password: vexfs123"
        ;;
esac

# Function to wait for SSH to be available
wait_for_ssh() {
    local max_attempts=60
    local attempt=1
    
    echo "⏳ Waiting for SSH to be available..."
    while [ $attempt -le $max_attempts ]; do
        if ssh -o ConnectTimeout=2 -o StrictHostKeyChecking=no -p "$VM_SSH_PORT" vexfs@localhost 'echo "SSH ready"' >/dev/null 2>&1; then
            echo "✅ SSH connection established"
            return 0
        fi
        echo "   Attempt $attempt/$max_attempts..."
        sleep 2
        ((attempt++))
    done
    
    echo "❌ SSH connection timeout"
    return 1
}

# Function to run automated tests
run_automated_tests() {
    echo "🧪 Running automated test suite..."
    
    if ! wait_for_ssh; then
        echo "❌ Cannot connect to VM for testing"
        return 1
    fi
    
    echo "📋 Executing test suite..."
    ssh -o StrictHostKeyChecking=no -p "$VM_SSH_PORT" vexfs@localhost << 'TEST_SCRIPT'
set -e

echo "🔄 Mounting VexFS source..."
if ! mountpoint -q /mnt/vexfs-src; then
    sudo mount /mnt/vexfs-src
fi

echo "📂 Source directory contents:"
ls -la /mnt/vexfs-src/

echo "🧪 Running VexFS test suite..."
/usr/local/bin/vexfs-test.sh

echo "🎉 Test suite completed successfully!"
TEST_SCRIPT
    
    local test_result=$?
    if [ $test_result -eq 0 ]; then
        echo "✅ All tests passed!"
    else
        echo "❌ Tests failed with exit code $test_result"
    fi
    
    return $test_result
}

# Function to show development tips
show_dev_tips() {
    cat << 'EOF'

🛠️  VexFS Development Tips:
===========================

Quick Commands (in VM):
  vs      - Change to VexFS source directory
  vt      - Run full test suite
  build-kernel - Build kernel module only
  build-vexctl - Build userspace tool only

Development Workflow:
  1. Edit code on host (changes are immediately visible in VM)
  2. SSH into VM: ssh vexfs@localhost -p 2222
  3. Run tests: vt
  4. Debug: dmesg | tail -20

Debugging:
  - View kernel logs: dmesg | tail -50
  - Check module status: lsmod | grep vexfs
  - Monitor system: htop
  - Trace syscalls: strace -p <pid>

Performance Testing:
  - Stress test: stress-ng --vm 2 --vm-bytes 1G --timeout 60s
  - I/O benchmark: fio --name=test --ioengine=libaio --rw=write --bs=4k --size=100M

EOF
}

# Start VM based on mode
case "$TEST_MODE" in
    "test")
        echo "🤖 Starting automated test mode..."
        "${QEMU_CMD[@]}" &
        VM_PID=$!
        
        # Wait a bit for boot, then run tests
        if run_automated_tests; then
            echo "✅ Automated testing completed successfully"
            exit_code=0
        else
            echo "❌ Automated testing failed"
            exit_code=1
        fi
        
        # Shutdown VM
        echo "🛑 Shutting down VM..."
        ssh -o StrictHostKeyChecking=no -p "$VM_SSH_PORT" vexfs@localhost 'sudo shutdown -h now' >/dev/null 2>&1 || true
        wait $VM_PID 2>/dev/null || true
        
        exit $exit_code
        ;;
        
    "interactive"|*)
        show_dev_tips
        echo ""
        echo "🚀 Starting VM..."
        echo "   Press Ctrl+C to stop"
        echo ""
        
        # Handle Ctrl+C gracefully
        trap 'echo ""; echo "🛑 Shutting down VM..."; kill $VM_PID 2>/dev/null; exit 0' INT
        
        "${QEMU_CMD[@]}" &
        VM_PID=$!
        
        # Show connection info once SSH is ready
        if wait_for_ssh; then
            echo ""
            echo "🎯 VM is ready for development!"
            echo "   SSH: ssh vexfs@localhost -p $VM_SSH_PORT"
            echo "   Password: vexfs123"
            echo ""
            echo "   Try: ssh vexfs@localhost -p $VM_SSH_PORT -t 'vt'"
            echo ""
        fi
        
        # Wait for VM to finish
        wait $VM_PID
        ;;
esac