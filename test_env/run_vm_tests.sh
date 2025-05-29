#!/bin/bash

# Script to run comprehensive VexFS tests in VM
# This script waits for VM to be ready and then runs the test suite

set -e

echo "🚀 VexFS VM Test Runner"
echo "======================"

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

# Configuration
VM_HOST="localhost"
VM_PORT="2222"
VM_USER="vexfs"
VM_KEY="test_env/vm/keys/vexfs_vm_key"
MAX_WAIT=300  # 5 minutes max wait
WAIT_INTERVAL=10

echo -e "${YELLOW}Waiting for VM to be ready...${NC}"

# Function to check if VM is ready
check_vm_ready() {
    ssh -p "$VM_PORT" -i "$VM_KEY" -o StrictHostKeyChecking=no -o ConnectTimeout=5 \
        "$VM_USER@$VM_HOST" "echo 'ready'" 2>/dev/null
}

# Wait for VM to be ready
waited=0
while [ $waited -lt $MAX_WAIT ]; do
    if check_vm_ready >/dev/null 2>&1; then
        echo -e "${GREEN}✅ VM is ready!${NC}"
        break
    fi
    
    echo -e "${YELLOW}⏳ Waiting for VM... (${waited}s/${MAX_WAIT}s)${NC}"
    sleep $WAIT_INTERVAL
    waited=$((waited + WAIT_INTERVAL))
done

if [ $waited -ge $MAX_WAIT ]; then
    echo -e "${RED}❌ VM failed to become ready within ${MAX_WAIT} seconds${NC}"
    exit 1
fi

# Run the comprehensive test suite
echo -e "${YELLOW}🧪 Running comprehensive VexFS test suite...${NC}"

ssh -p "$VM_PORT" -i "$VM_KEY" -o StrictHostKeyChecking=no \
    "$VM_USER@$VM_HOST" << 'EOF'
    
    echo "🔧 Setting up test environment in VM..."
    
    # Ensure we're in the right directory
    cd /mnt/vexfs_source
    
    # Make sure test script is executable
    chmod +x test_env/comprehensive_vexfs_test.sh
    
    # Run the comprehensive test suite
    echo "🧪 Starting VexFS comprehensive test suite..."
    ./test_env/comprehensive_vexfs_test.sh
    
    # Copy test results to a location we can access
    echo "📋 Test completed. Results:"
    if [ -f /tmp/vexfs_test_results.log ]; then
        echo "=== TEST RESULTS ==="
        cat /tmp/vexfs_test_results.log
        echo "===================="
    else
        echo "❌ Test results file not found"
    fi
EOF

test_exit_code=$?

if [ $test_exit_code -eq 0 ]; then
    echo -e "${GREEN}🎉 All VM tests completed successfully!${NC}"
    echo -e "${GREEN}✅ VexFS kernel module is ready for production testing${NC}"
else
    echo -e "${RED}❌ Some VM tests failed${NC}"
    echo -e "${RED}🚫 VexFS kernel module is NOT ready for production testing${NC}"
fi

exit $test_exit_code