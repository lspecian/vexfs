#!/bin/bash

# VexFS Image Validation Script
# Validates VexFS images built with the QEMU-based pipeline

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
IMAGES_DIR="$SCRIPT_DIR/images"
TEMP_DIR="/tmp/vexfs-validation"

# Test configuration
BOOT_TIMEOUT=300  # 5 minutes
SSH_TIMEOUT=60    # 1 minute

# Helper functions
log_info() {
    echo -e "${BLUE}[VALIDATE]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

show_usage() {
    cat << EOF
VexFS Image Validation Script

Usage: $0 [OPTIONS] [IMAGE_FILE]

Options:
    -d, --images-dir DIR       Directory containing images (default: $IMAGES_DIR)
    --boot-timeout SECONDS     Boot timeout in seconds (default: $BOOT_TIMEOUT)
    --ssh-timeout SECONDS      SSH timeout in seconds (default: $SSH_TIMEOUT)
    --no-cleanup              Keep temporary files after validation
    -v, --verbose              Verbose output
    -h, --help                 Show this help message

Arguments:
    IMAGE_FILE                Specific image file to validate (optional)

Examples:
    $0                                    # Validate all images in images directory
    $0 vexfs-production-1.0.0.qcow2     # Validate specific image
    $0 --boot-timeout 600               # Use longer boot timeout

EOF
}

# Parse command line arguments
parse_arguments() {
    IMAGE_FILE=""
    CLEANUP=true
    VERBOSE=false
    
    while [[ $# -gt 0 ]]; do
        case $1 in
            -d|--images-dir)
                IMAGES_DIR="$2"
                shift 2
                ;;
            --boot-timeout)
                BOOT_TIMEOUT="$2"
                shift 2
                ;;
            --ssh-timeout)
                SSH_TIMEOUT="$2"
                shift 2
                ;;
            --no-cleanup)
                CLEANUP=false
                shift
                ;;
            -v|--verbose)
                VERBOSE=true
                shift
                ;;
            -h|--help)
                show_usage
                exit 0
                ;;
            -*)
                log_error "Unknown option: $1"
                ;;
            *)
                IMAGE_FILE="$1"
                shift
                ;;
        esac
    done
}

# Setup validation environment
setup_environment() {
    log_info "Setting up validation environment..."
    
    mkdir -p "$TEMP_DIR"
    
    # Check dependencies
    local missing_deps=()
    
    if ! command -v qemu-system-x86_64 >/dev/null 2>&1; then
        missing_deps+=("qemu-system-x86_64")
    fi
    
    if ! command -v ssh >/dev/null 2>&1; then
        missing_deps+=("ssh")
    fi
    
    if [ ${#missing_deps[@]} -ne 0 ]; then
        log_error "Missing dependencies: ${missing_deps[*]}"
    fi
    
    log_success "Validation environment ready"
}

# Find images to validate
find_images() {
    log_info "Finding images to validate..."
    
    local images=()
    
    if [ -n "$IMAGE_FILE" ]; then
        # Validate specific image
        if [ -f "$IMAGES_DIR/$IMAGE_FILE" ]; then
            images=("$IMAGES_DIR/$IMAGE_FILE")
        elif [ -f "$IMAGE_FILE" ]; then
            images=("$IMAGE_FILE")
        else
            log_error "Image file not found: $IMAGE_FILE"
        fi
    else
        # Find all qcow2 files
        while IFS= read -r -d '' image; do
            images+=("$image")
        done < <(find "$IMAGES_DIR" -name "*.qcow2" -print0 2>/dev/null)
    fi
    
    if [ ${#images[@]} -eq 0 ]; then
        log_error "No images found to validate"
    fi
    
    log_info "Found ${#images[@]} images to validate:"
    for image in "${images[@]}"; do
        log_info "  $(basename "$image")"
    done
    
    echo "${images[@]}"
}

# Boot image and wait for SSH
boot_image() {
    local image_path="$1"
    local image_name="$(basename "$image_path" .qcow2)"
    local ssh_port="$2"
    
    log_info "Booting image: $image_name"
    
    # Start QEMU in background
    local qemu_cmd=(
        qemu-system-x86_64
        -name "$image_name-validation"
        -m 2048
        -smp 2
        -drive "file=$image_path,format=qcow2,if=virtio"
        -netdev "user,id=net0,hostfwd=tcp::${ssh_port}-:22"
        -device "virtio-net,netdev=net0"
        -display none
        -daemonize
        -pidfile "$TEMP_DIR/${image_name}.pid"
    )
    
    if [ "$VERBOSE" = true ]; then
        log_info "QEMU command: ${qemu_cmd[*]}"
    fi
    
    "${qemu_cmd[@]}" || {
        log_error "Failed to start QEMU for $image_name"
        return 1
    }
    
    local qemu_pid=$(cat "$TEMP_DIR/${image_name}.pid")
    log_info "QEMU started with PID: $qemu_pid"
    
    # Wait for SSH to become available
    log_info "Waiting for SSH to become available (timeout: ${BOOT_TIMEOUT}s)..."
    
    local start_time=$(date +%s)
    local ssh_ready=false
    
    while [ $(($(date +%s) - start_time)) -lt $BOOT_TIMEOUT ]; do
        if timeout $SSH_TIMEOUT ssh -o ConnectTimeout=5 -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null \
           -p "$ssh_port" vexfs@localhost "echo 'SSH ready'" >/dev/null 2>&1; then
            ssh_ready=true
            break
        fi
        sleep 5
    done
    
    if [ "$ssh_ready" = true ]; then
        log_success "SSH connection established for $image_name"
        echo "$qemu_pid"
        return 0
    else
        log_error "SSH connection timeout for $image_name"
        kill "$qemu_pid" 2>/dev/null || true
        return 1
    fi
}

# Run SSH command on image
run_ssh_command() {
    local ssh_port="$1"
    local command="$2"
    local timeout_duration="${3:-30}"
    
    timeout "$timeout_duration" ssh -o ConnectTimeout=5 -o StrictHostKeyChecking=no -o UserKnownHostsFile=/dev/null \
        -p "$ssh_port" vexfs@localhost "$command" 2>/dev/null
}

# Test VexFS functionality
test_vexfs_functionality() {
    local ssh_port="$1"
    local image_name="$2"
    
    log_info "Testing VexFS functionality on $image_name..."
    
    # Test 1: Check if VexFS module is available
    log_info "  Checking VexFS module availability..."
    if run_ssh_command "$ssh_port" "modinfo vexfs" 30; then
        log_success "  ✓ VexFS module found"
    else
        log_error "  ✗ VexFS module not found"
        return 1
    fi
    
    # Test 2: Load VexFS module
    log_info "  Loading VexFS module..."
    if run_ssh_command "$ssh_port" "sudo modprobe vexfs" 30; then
        log_success "  ✓ VexFS module loaded successfully"
    else
        log_error "  ✗ Failed to load VexFS module"
        return 1
    fi
    
    # Test 3: Check if module is loaded
    log_info "  Verifying module is loaded..."
    if run_ssh_command "$ssh_port" "lsmod | grep vexfs" 30; then
        log_success "  ✓ VexFS module is active"
    else
        log_error "  ✗ VexFS module not active"
        return 1
    fi
    
    # Test 4: Check vexctl availability
    log_info "  Testing vexctl availability..."
    if run_ssh_command "$ssh_port" "vexctl --version" 30; then
        log_success "  ✓ vexctl is available and working"
    else
        log_error "  ✗ vexctl not available or not working"
        return 1
    fi
    
    # Test 5: Test basic vexctl functionality
    log_info "  Testing vexctl status command..."
    if run_ssh_command "$ssh_port" "vexctl status" 30; then
        log_success "  ✓ vexctl status command works"
    else
        log_warning "  ⚠ vexctl status command failed (may be expected if no filesystem mounted)"
    fi
    
    # Test 6: Check VexFS configuration
    log_info "  Checking VexFS configuration..."
    if run_ssh_command "$ssh_port" "test -f /etc/vexfs/vexfs.conf" 30; then
        log_success "  ✓ VexFS configuration file exists"
    else
        log_error "  ✗ VexFS configuration file missing"
        return 1
    fi
    
    # Test 7: Check systemd service
    log_info "  Checking VexFS systemd service..."
    if run_ssh_command "$ssh_port" "systemctl is-enabled vexfs" 30; then
        log_success "  ✓ VexFS systemd service is enabled"
    else
        log_warning "  ⚠ VexFS systemd service not enabled"
    fi
    
    # Test 8: Test module unloading
    log_info "  Testing module unloading..."
    if run_ssh_command "$ssh_port" "sudo rmmod vexfs" 30; then
        log_success "  ✓ VexFS module unloaded successfully"
    else
        log_error "  ✗ Failed to unload VexFS module"
        return 1
    fi
    
    log_success "All VexFS functionality tests passed for $image_name"
    return 0
}

# Test system integration
test_system_integration() {
    local ssh_port="$1"
    local image_name="$2"
    
    log_info "Testing system integration on $image_name..."
    
    # Test 1: Check mount helpers
    log_info "  Checking mount helpers..."
    if run_ssh_command "$ssh_port" "test -x /usr/local/bin/vexfs-mount-helper" 30; then
        log_success "  ✓ VexFS mount helper exists and is executable"
    else
        log_error "  ✗ VexFS mount helper missing or not executable"
        return 1
    fi
    
    if run_ssh_command "$ssh_port" "test -x /usr/local/bin/vexfs-umount-helper" 30; then
        log_success "  ✓ VexFS unmount helper exists and is executable"
    else
        log_error "  ✗ VexFS unmount helper missing or not executable"
        return 1
    fi
    
    # Test 2: Check log file
    log_info "  Checking log file..."
    if run_ssh_command "$ssh_port" "test -f /var/log/vexfs.log" 30; then
        log_success "  ✓ VexFS log file exists"
    else
        log_warning "  ⚠ VexFS log file missing"
    fi
    
    # Test 3: Check modules load configuration
    log_info "  Checking modules load configuration..."
    if run_ssh_command "$ssh_port" "test -f /etc/modules-load.d/vexfs.conf" 30; then
        log_success "  ✓ VexFS modules load configuration exists"
    else
        log_error "  ✗ VexFS modules load configuration missing"
        return 1
    fi
    
    # Test 4: Test basic system functionality
    log_info "  Testing basic system functionality..."
    if run_ssh_command "$ssh_port" "uname -a" 30; then
        log_success "  ✓ System is responsive"
    else
        log_error "  ✗ System not responsive"
        return 1
    fi
    
    log_success "System integration tests passed for $image_name"
    return 0
}

# Validate a single image
validate_image() {
    local image_path="$1"
    local image_name="$(basename "$image_path" .qcow2)"
    local ssh_port=$((2222 + RANDOM % 1000))  # Random port to avoid conflicts
    
    log_info "Validating image: $image_name"
    
    # Check image integrity first
    log_info "Checking image integrity..."
    if qemu-img check "$image_path" >/dev/null 2>&1; then
        log_success "Image integrity check passed"
    else
        log_error "Image integrity check failed"
        return 1
    fi
    
    # Boot the image
    local qemu_pid
    if qemu_pid=$(boot_image "$image_path" "$ssh_port"); then
        log_success "Image booted successfully"
    else
        log_error "Failed to boot image"
        return 1
    fi
    
    local validation_result=0
    
    # Run VexFS functionality tests
    if ! test_vexfs_functionality "$ssh_port" "$image_name"; then
        validation_result=1
    fi
    
    # Run system integration tests
    if ! test_system_integration "$ssh_port" "$image_name"; then
        validation_result=1
    fi
    
    # Shutdown QEMU
    log_info "Shutting down QEMU..."
    if run_ssh_command "$ssh_port" "sudo shutdown -h now" 30 2>/dev/null || true; then
        sleep 10  # Give it time to shutdown gracefully
    fi
    
    # Force kill if still running
    if kill -0 "$qemu_pid" 2>/dev/null; then
        log_warning "Force killing QEMU process"
        kill "$qemu_pid" 2>/dev/null || true
    fi
    
    # Clean up PID file
    rm -f "$TEMP_DIR/${image_name}.pid"
    
    if [ $validation_result -eq 0 ]; then
        log_success "Image validation passed: $image_name"
    else
        log_error "Image validation failed: $image_name"
    fi
    
    return $validation_result
}

# Generate validation report
generate_report() {
    local results=("$@")
    local report_file="$SCRIPT_DIR/validation-report-$(date +%Y%m%d-%H%M%S).txt"
    
    log_info "Generating validation report..."
    
    local total_images=${#results[@]}
    local passed_images=0
    local failed_images=0
    
    cat > "$report_file" << EOF
VexFS Image Validation Report
=============================

Validation Date: $(date)
Boot Timeout: ${BOOT_TIMEOUT}s
SSH Timeout: ${SSH_TIMEOUT}s

Results:
EOF
    
    for result in "${results[@]}"; do
        local image_name="${result%:*}"
        local status="${result#*:}"
        
        echo "  $image_name: $status" >> "$report_file"
        
        if [ "$status" = "PASSED" ]; then
            ((passed_images++))
        else
            ((failed_images++))
        fi
    done
    
    cat >> "$report_file" << EOF

Summary:
  Total Images: $total_images
  Passed: $passed_images
  Failed: $failed_images
  Success Rate: $(( passed_images * 100 / total_images ))%

EOF
    
    log_success "Validation report generated: $report_file"
    
    # Display summary
    echo ""
    log_info "Validation Summary:"
    log_info "  Total Images: $total_images"
    log_info "  Passed: $passed_images"
    log_info "  Failed: $failed_images"
    log_info "  Success Rate: $(( passed_images * 100 / total_images ))%"
}

# Cleanup function
cleanup() {
    if [ "$CLEANUP" = true ]; then
        log_info "Cleaning up temporary files..."
        rm -rf "$TEMP_DIR"
        log_success "Cleanup completed"
    else
        log_info "Skipping cleanup (disabled)"
    fi
}

# Main execution
main() {
    echo "🧪 VexFS Image Validation"
    echo "========================="
    
    parse_arguments "$@"
    setup_environment
    
    local images
    read -ra images <<< "$(find_images)"
    
    log_info "Starting validation..."
    echo ""
    
    local results=()
    local failed_count=0
    
    for image in "${images[@]}"; do
        local image_name="$(basename "$image" .qcow2)"
        
        if validate_image "$image"; then
            results+=("$image_name:PASSED")
        else
            results+=("$image_name:FAILED")
            ((failed_count++))
        fi
        
        echo ""
    done
    
    generate_report "${results[@]}"
    cleanup
    
    echo ""
    if [ $failed_count -eq 0 ]; then
        log_success "🎉 All image validations passed!"
        exit 0
    else
        log_error "❌ $failed_count image validation(s) failed"
        exit 1
    fi
}

# Trap cleanup on exit
trap cleanup EXIT

# Run main function with all arguments
main "$@"