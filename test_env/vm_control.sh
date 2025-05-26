#!/bin/bash

# VexFS VM Control Script
# Manages the VexFS development VM

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
VM_DIR="$SCRIPT_DIR/vm"
VM_IMAGE="$VM_DIR/images/vexfs-dev.qcow2"
CLOUD_INIT_ISO="$VM_DIR/config/cloud-init.iso"
SSH_KEY="$VM_DIR/keys/vexfs_vm_key"

VM_MEMORY="2G"
VM_CPUS="2"
SSH_PORT="2222"
VNC_PORT="5900"
MONITOR_PORT="55555"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log() { echo -e "${BLUE}[INFO]${NC} $1"; }
warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
error() { echo -e "${RED}[ERROR]${NC} $1" >&2; }
success() { echo -e "${GREEN}[SUCCESS]${NC} $1"; }

check_vm_exists() {
    if [ ! -f "$VM_IMAGE" ]; then
        error "VM image not found. Run './setup_vm.sh' first."
        exit 1
    fi
}

get_vm_pid() {
    pgrep -f "qemu-system-x86_64.*vexfs-dev" || true
}

is_vm_running() {
    [ -n "$(get_vm_pid)" ]
}

wait_for_ssh() {
    local max_attempts=30
    local attempt=1
    
    log "Waiting for VM to be ready..."
    
    while [ $attempt -le $max_attempts ]; do
        if ssh -o ConnectTimeout=5 -o StrictHostKeyChecking=no -p $SSH_PORT -i "$SSH_KEY" vexfs@localhost "echo 'VM ready'" &>/dev/null; then
            success "VM is ready for SSH connections"
            return 0
        fi
        
        echo -n "."
        sleep 2
        ((attempt++))
    done
    
    error "VM did not become ready within expected time"
    return 1
}

start_vm() {
    local headless=${1:-false}
    local background=${2:-false}
    
    check_vm_exists
    
    if is_vm_running; then
        warn "VM is already running (PID: $(get_vm_pid))"
        return 0
    fi
    
    log "Starting VexFS development VM..."
    
    # Build QEMU command
    local qemu_cmd="qemu-system-x86_64"
    qemu_cmd="$qemu_cmd -name vexfs-dev"
    
    # Check if KVM is available and accessible
    if [ -w /dev/kvm ] 2>/dev/null; then
        qemu_cmd="$qemu_cmd -enable-kvm"
        log "Using KVM acceleration"
    else
        log "KVM not available, using software emulation (slower)"
    fi
    
    qemu_cmd="$qemu_cmd -m $VM_MEMORY"
    qemu_cmd="$qemu_cmd -smp $VM_CPUS"
    qemu_cmd="$qemu_cmd -drive file=$VM_IMAGE,format=qcow2,if=virtio"
    qemu_cmd="$qemu_cmd -drive file=$CLOUD_INIT_ISO,format=raw,if=virtio,readonly=on"
    qemu_cmd="$qemu_cmd -netdev user,id=net0,hostfwd=tcp::$SSH_PORT-:22"
    qemu_cmd="$qemu_cmd -device virtio-net,netdev=net0"
    qemu_cmd="$qemu_cmd -virtfs local,path=$PROJECT_ROOT,mount_tag=vexfs_source,security_model=passthrough,id=vexfs_source"
    qemu_cmd="$qemu_cmd -monitor telnet:localhost:$MONITOR_PORT,server,nowait"
    
    if [ "$headless" = "true" ]; then
        qemu_cmd="$qemu_cmd -display none -vnc :0"
        log "Starting in headless mode (VNC on port $VNC_PORT)"
    else
        qemu_cmd="$qemu_cmd -display gtk,grab-on-hover=on"
    fi
    
    if [ "$background" = "true" ]; then
        # Run QEMU with clean environment to avoid snap library conflicts
        env -i PATH="/usr/bin:/bin:/usr/sbin:/sbin" $qemu_cmd &
        local vm_pid=$!
        log "VM started in background (PID: $vm_pid)"
        wait_for_ssh
    else
        # Run QEMU with clean environment to avoid snap library conflicts
        env -i PATH="/usr/bin:/bin:/usr/sbin:/sbin" $qemu_cmd
    fi
}

stop_vm() {
    if ! is_vm_running; then
        warn "VM is not running"
        return 0
    fi
    
    log "Stopping VM gracefully..."
    
    # Try graceful shutdown via SSH first
    if ssh -o ConnectTimeout=5 -p $SSH_PORT -i "$SSH_KEY" vexfs@localhost "sudo poweroff" &>/dev/null; then
        # Wait for graceful shutdown
        local attempts=0
        while is_vm_running && [ $attempts -lt 30 ]; do
            sleep 1
            ((attempts++))
        done
    fi
    
    # Force kill if still running
    if is_vm_running; then
        warn "Forcing VM shutdown..."
        kill $(get_vm_pid)
    fi
    
    success "VM stopped"
}

ssh_vm() {
    check_vm_exists
    
    if ! is_vm_running; then
        error "VM is not running. Start it first with: $0 start"
        exit 1
    fi
    
    if [ $# -eq 0 ]; then
        log "Connecting to VM..."
        ssh -p $SSH_PORT -i "$SSH_KEY" vexfs@localhost
    else
        # Execute command in VM
        ssh -p $SSH_PORT -i "$SSH_KEY" vexfs@localhost "$@"
    fi
}

status_vm() {
    if is_vm_running; then
        success "VM is running (PID: $(get_vm_pid))"
        log "SSH: ssh -p $SSH_PORT -i $SSH_KEY vexfs@localhost"
        if netstat -tln 2>/dev/null | grep -q ":$VNC_PORT"; then
            log "VNC: localhost:$VNC_PORT"
        fi
        log "Monitor: telnet localhost $MONITOR_PORT"
    else
        warn "VM is not running"
    fi
}

monitor_vm() {
    if ! is_vm_running; then
        error "VM is not running"
        exit 1
    fi
    
    log "Connecting to QEMU monitor..."
    log "Type 'help' for available commands, 'quit' to exit"
    telnet localhost $MONITOR_PORT
}

show_help() {
    echo "VexFS VM Control Script"
    echo
    echo "Usage: $0 <command> [options]"
    echo
    echo "Commands:"
    echo "  start          Start VM with GUI"
    echo "  start-headless Start VM without GUI (VNC on port $VNC_PORT)"
    echo "  stop           Stop VM gracefully"
    echo "  ssh [command]  SSH into VM or run command"
    echo "  status         Show VM status"
    echo "  monitor        Access QEMU monitor"
    echo "  help           Show this help"
    echo
    echo "Examples:"
    echo "  $0 start                    # Start VM with GUI"
    echo "  $0 start-headless          # Start VM headless"
    echo "  $0 ssh                      # Interactive SSH session"
    echo "  $0 ssh 'ls -la'             # Run command via SSH"
    echo "  $0 ssh vexfs-quick-test     # Run VexFS test suite"
}

case "${1:-help}" in
    start)
        start_vm false true
        ;;
    start-headless)
        start_vm true true
        ;;
    stop)
        stop_vm
        ;;
    ssh)
        shift
        ssh_vm "$@"
        ;;
    status)
        status_vm
        ;;
    monitor)
        monitor_vm
        ;;
    help|--help|-h)
        show_help
        ;;
    *)
        error "Unknown command: $1"
        echo
        show_help
        exit 1
        ;;
esac