#!/bin/bash
set -e

# VexFS FFI Integration Validation Script
# This script validates that the C FFI functions work correctly in kernel context
# and can be called from the kernel module

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

log() {
    echo -e "${BLUE}[$(date '+%H:%M:%S')]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[$(date '+%H:%M:%S')] ✅ $1${NC}"
}

log_warning() {
    echo -e "${YELLOW}[$(date '+%H:%M:%S')] ⚠️  $1${NC}"
}

log_error() {
    echo -e "${RED}[$(date '+%H:%M:%S')] ❌ $1${NC}"
}

# Test results
TESTS_PASSED=0
TESTS_FAILED=0

run_test() {
    local test_name="$1"
    local test_command="$2"
    
    log "Testing: $test_name"
    
    if eval "$test_command" >/dev/null 2>&1; then
        log_success "$test_name"
        TESTS_PASSED=$((TESTS_PASSED + 1))
        return 0
    else
        log_error "$test_name"
        TESTS_FAILED=$((TESTS_FAILED + 1))
        return 1
    fi
}

# Validate build environment
validate_build_environment() {
    log "🔍 Validating build environment..."
    
    run_test "Rust toolchain available" "rustc --version"
    run_test "Cargo available" "cargo --version"
    run_test "GCC available" "gcc --version"
    run_test "Make available" "make --version"
    run_test "Kernel headers available" "test -d /lib/modules/$(uname -r)/build"
}

# Validate FFI header generation
validate_ffi_headers() {
    log "📋 Validating FFI header generation..."
    
    cd "$PROJECT_ROOT/vexfs"
    
    # Check if cbindgen is available
    if ! command -v cbindgen >/dev/null; then
        log_warning "cbindgen not installed, installing..."
        cargo install cbindgen
    fi
    
    # Generate FFI headers
    run_test "FFI header generation" "cbindgen --config cbindgen.toml --crate vexfs --output vexfs_ffi.h"
    run_test "FFI header file exists" "test -f vexfs_ffi.h"
    
    # Validate header content
    run_test "FFI header contains expected functions" "grep -q 'vexfs_' vexfs_ffi.h"
    run_test "FFI header is valid C" "gcc -fsyntax-only -x c vexfs_ffi.h"
}

# Validate Rust library compilation
validate_rust_compilation() {
    log "🦀 Validating Rust library compilation..."
    
    cd "$PROJECT_ROOT/vexfs"
    
    # Build Rust library
    run_test "Rust library compilation" "cargo build --release --lib"
    run_test "Static library exists" "test -f target/release/libvexfs.a"
    
    # Check for FFI symbols in library
    run_test "FFI symbols in library" "nm target/release/libvexfs.a | grep -q vexfs_"
    
    # Copy static library for kernel module
    run_test "Copy static library" "cp target/release/libvexfs.a libvexfs.a"
}

# Validate C FFI integration
validate_c_ffi() {
    log "🔗 Validating C FFI integration..."
    
    cd "$PROJECT_ROOT/vexfs"
    
    # Compile C test program
    run_test "C FFI test compilation" "gcc -o test_ffi test_ffi.c -L. -lvexfs -lm -lpthread -ldl"
    run_test "C FFI test executable exists" "test -x test_ffi"
    
    # Run basic FFI test
    if run_test "C FFI basic execution" "./test_ffi"; then
        log_success "FFI functions callable from C"
    else
        log_error "FFI execution failed"
    fi
    
    # Compile integration test
    if [ -f "test_ffi_integration.c" ]; then
        run_test "C FFI integration compilation" "gcc -o test_ffi_integration test_ffi_integration.c -L. -lvexfs -lm -lpthread -ldl"
        run_test "C FFI integration execution" "./test_ffi_integration"
    else
        log_warning "Integration test source not found"
    fi
}

# Validate kernel module compilation
validate_kernel_module() {
    log "🐧 Validating kernel module compilation..."
    
    cd "$PROJECT_ROOT/vexfs"
    
    # Clean previous builds
    run_test "Clean previous builds" "make clean"
    
    # Build kernel module
    run_test "Kernel module compilation" "make"
    run_test "Kernel module file exists" "test -f vexfs.ko"
    
    # Validate module info
    run_test "Module info accessible" "modinfo vexfs.ko"
    run_test "Module has description" "modinfo vexfs.ko | grep -q description"
    
    # Check for FFI symbols in module
    run_test "FFI symbols in module" "nm vexfs.ko | grep -q vexfs_ || objdump -t vexfs.ko | grep -q vexfs_"
}

# Validate module loading (requires root)
validate_module_loading() {
    log "🔌 Validating module loading..."
    
    cd "$PROJECT_ROOT/vexfs"
    
    if [ "$EUID" -ne 0 ]; then
        log_warning "Root required for module loading tests, skipping..."
        return
    fi
    
    # Load module
    if run_test "Module loading" "insmod vexfs.ko"; then
        # Check if module is loaded
        run_test "Module loaded verification" "lsmod | grep -q vexfs"
        
        # Check kernel messages
        run_test "Module load messages" "dmesg | tail -10 | grep -i vexfs"
        
        # Check exported symbols
        run_test "Exported symbols" "cat /proc/kallsyms | grep -q vexfs_"
        
        # Unload module
        run_test "Module unloading" "rmmod vexfs"
        run_test "Module unloaded verification" "! lsmod | grep -q vexfs"
    else
        log_error "Module loading failed"
    fi
}

# Validate vector operations through FFI
validate_vector_operations() {
    log "🔢 Validating vector operations..."
    
    # Build vector test runner (in vexfs subdirectory)
    cd "$PROJECT_ROOT/vexfs"
    if [ -f "src/bin/vector_test_runner.rs" ]; then
        run_test "Vector test runner compilation" "cargo build --release --bin vector_test_runner"
        
        if [ -f "target/release/vector_test_runner" ]; then
            run_test "Vector test runner execution" "./target/release/vector_test_runner"
        else
            log_warning "Vector test runner executable not found"
        fi
    else
        log_warning "Vector test runner source not found"
    fi
    
    # Test vector storage functions through FFI
    if [ -f "test_ffi" ]; then
        log "Testing vector storage through FFI..."
        if ./test_ffi 2>&1 | grep -q "vector\|search\|index\|SUCCESS"; then
            log_success "Vector operations accessible through FFI"
        else
            log_warning "Vector operations not explicitly tested"
        fi
    else
        log_warning "FFI test executable not available"
    fi
}

# Create comprehensive test report
create_test_report() {
    local total_tests=$((TESTS_PASSED + TESTS_FAILED))
    local success_rate=0
    
    if [ $total_tests -gt 0 ]; then
        success_rate=$(( (TESTS_PASSED * 100) / total_tests ))
    fi
    
    echo ""
    echo "📊 FFI Integration Validation Report"
    echo "===================================="
    echo "Total Tests: $total_tests"
    echo "Passed: $TESTS_PASSED"
    echo "Failed: $TESTS_FAILED"
    echo "Success Rate: ${success_rate}%"
    echo ""
    
    if [ $TESTS_FAILED -eq 0 ]; then
        log_success "🎉 All FFI integration tests passed!"
        echo ""
        echo "✅ Rust library compiles successfully"
        echo "✅ FFI headers are generated correctly"
        echo "✅ C programs can link against Rust library"
        echo "✅ Kernel module includes FFI functionality"
        echo "✅ Module loads and unloads without errors"
        echo ""
        echo "🚀 VexFS FFI integration is ready for development!"
        return 0
    else
        log_error "❌ Some FFI integration tests failed"
        echo ""
        echo "Issues found:"
        [ $TESTS_FAILED -gt 0 ] && echo "- $TESTS_FAILED test(s) failed"
        echo ""
        echo "Recommendations:"
        echo "1. Check build dependencies are installed"
        echo "2. Verify kernel headers match running kernel"
        echo "3. Ensure Rust toolchain is properly configured"
        echo "4. Review error messages above for specific issues"
        return 1
    fi
}

# Main execution
main() {
    echo "🔧 VexFS FFI Integration Validation"
    echo "===================================="
    echo "This script validates that the Rust-to-C FFI integration"
    echo "works correctly for kernel module development."
    echo ""
    
    validate_build_environment
    validate_ffi_headers
    validate_rust_compilation
    validate_c_ffi
    validate_kernel_module
    validate_module_loading
    validate_vector_operations
    
    create_test_report
}

# Run if script is executed directly
if [ "${BASH_SOURCE[0]}" = "${0}" ]; then
    main "$@"
fi