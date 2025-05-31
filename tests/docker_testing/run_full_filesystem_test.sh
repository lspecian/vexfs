#!/bin/bash

set -e

echo "🚀 VexFS Complete Filesystem Test Runner"
echo "======================================="

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

# Check if we're in the right directory
if [ ! -f "tests/docker_testing/full_filesystem_test.sh" ]; then
    log_error "Please run this script from the VexFS project root directory"
    exit 1
fi

# Make sure the test script is executable
chmod +x tests/docker_testing/full_filesystem_test.sh

log_info "Building Docker container for complete filesystem testing..."

# Build the Docker container
if docker build -f tests/docker_testing/Dockerfile.memory_test -t vexfs-full-test . 2>&1; then
    log_success "Docker container built successfully"
else
    log_error "Failed to build Docker container"
    exit 1
fi

log_info "Running complete VexFS filesystem test in Docker container..."
log_info "This will test:"
log_info "  🔧 Module loading"
log_info "  🗂️  Filesystem mounting"
log_info "  📁 File operations (create, read, write)"
log_info "  📂 Directory operations"
log_info "  🔄 Unmounting and cleanup"

echo ""
log_info "Starting test execution..."
echo "======================================="

# Run the complete filesystem test
if docker run --rm --privileged \
    -v /lib/modules:/lib/modules:ro \
    -v /usr/src:/usr/src:ro \
    vexfs-full-test \
    /vexfs/tests/docker_testing/full_filesystem_test.sh; then
    
    echo ""
    echo "======================================="
    log_success "🎉 COMPLETE FILESYSTEM TEST COMPLETED!"
    log_success "Check the output above for detailed results"
else
    echo ""
    echo "======================================="
    log_error "❌ FILESYSTEM TEST FAILED"
    log_error "Check the output above for error details"
    exit 1
fi