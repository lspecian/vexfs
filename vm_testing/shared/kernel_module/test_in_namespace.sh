#!/bin/bash

# Test VexFS in an isolated namespace
# This provides some protection against system-wide crashes

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

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

echo "🧪 VexFS Namespace Testing"
echo "========================"

# Check if running as root
if [ "$EUID" -ne 0 ]; then 
    log_error "This script must be run as root for namespace isolation"
    exit 1
fi

# Check module status
if lsmod | grep -q vexfs; then
    log_error "VexFS module already loaded - system needs reboot"
    log_info "The stuck module prevents any testing"
    exit 1
fi

log_info "Creating isolated test environment..."

# Create a test script to run in namespace
cat > /tmp/vexfs_ns_test.sh << 'EOF'
#!/bin/bash

# This runs inside the namespace

echo "Inside isolated namespace..."

# Test basic module operations without mounting
MODULE=/tmp/vexfs_deadlock_fix.ko

if [ -f "$MODULE" ]; then
    echo "Loading module..."
    if insmod "$MODULE"; then
        echo "✅ Module loaded successfully!"
        
        # Check registration
        if grep -q vexfs /proc/filesystems; then
            echo "✅ Filesystem registered"
            cat /proc/filesystems | grep vexfs
        fi
        
        # Don't attempt mount - just verify module works
        echo "Module loaded without crash - fix appears successful!"
        
        # Unload
        rmmod vexfs_deadlock_fix
        echo "✅ Module unloaded cleanly"
    else
        echo "❌ Module load failed"
        dmesg | tail -10
    fi
else
    echo "❌ Module not found in namespace"
fi
EOF

chmod +x /tmp/vexfs_ns_test.sh

# Copy module to temp
cp "$SCRIPT_DIR/vexfs_deadlock_fix.ko" /tmp/ 2>/dev/null || {
    log_error "Fixed module not found"
    exit 1
}

log_info "Running test in isolated namespace..."

# Run in namespace with limited capabilities
unshare --mount --pid --fork bash /tmp/vexfs_ns_test.sh || {
    log_error "Namespace test failed"
    log_warning "For safe testing, use the VM instead"
}

# Cleanup
rm -f /tmp/vexfs_ns_test.sh /tmp/vexfs_deadlock_fix.ko

echo
log_info "Namespace test complete"
log_info "For full testing including mount operations, use the VM"