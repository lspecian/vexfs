#!/bin/bash

# Safe module testing script
# Tests the module without mounting to avoid crashes

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MODULE_PATH="$SCRIPT_DIR/vexfs_deadlock_fix.ko"

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

echo "🧪 VexFS Module Safety Test"
echo "=========================="

# Check if module is already loaded
if lsmod | grep -q vexfs; then
    log_error "VexFS module is already loaded!"
    log_warning "The module has refcount=2 from previous crash"
    log_info "System needs reboot to clear the stuck module"
    echo
    echo "After reboot, you can:"
    echo "1. Run this test script"
    echo "2. Or use the VM for safe testing"
    exit 1
fi

# Check if we have the fixed module
if [ ! -f "$MODULE_PATH" ]; then
    log_error "Fixed module not found at: $MODULE_PATH"
    exit 1
fi

log_info "Fixed module found at: $MODULE_PATH"

# Show module info
log_info "Module information:"
modinfo "$MODULE_PATH" | grep -E "filename:|description:|license:|vermagic:"

# Verify the fix is in place
log_info "Verifying spinlock fix..."
if strings "$MODULE_PATH" | grep -q "spin_lock_irqsave.*bitmap_lock"; then
    log_warning "Module may still contain problematic spinlock code"
else
    log_success "Spinlock appears to be removed from module"
fi

# Check kernel readiness
log_info "Current kernel: $(uname -r)"
log_info "Module expects: $(modinfo "$MODULE_PATH" | grep vermagic | awk '{print $2}')"

echo
log_info "Module appears ready for testing"
log_warning "For safety, use the VM to test mounting:"
echo
echo "1. Start VM: ./vm_testing/scripts/start_alpine_vm.sh"
echo "2. Login as root and run: /mnt/shared/setup_alpine_auto.sh"
echo "3. After reboot, SSH: ssh -p 2222 root@localhost"
echo "4. Run tests: /mnt/shared/run_all_tests.sh"
echo
log_info "Or if you want to test on host (risky):"
echo "sudo insmod $MODULE_PATH"
echo "# Create test image and mount..."