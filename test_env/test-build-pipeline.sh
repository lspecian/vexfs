#!/bin/bash

# VexFS Build Pipeline Integration Test
# Comprehensive test of the Packer-based image build pipeline

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TEST_OUTPUT_DIR="$SCRIPT_DIR/test-output"
TEST_LOG_DIR="$SCRIPT_DIR/test-logs"

# Test configuration
RUN_FULL_BUILD=false
RUN_VALIDATION=true
CLEANUP_AFTER_TEST=true

# Helper functions
log_info() {
    echo -e "${BLUE}[TEST-INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[TEST-SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[TEST-WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[TEST-ERROR]${NC} $1"
}

show_usage() {
    cat << EOF
VexFS Build Pipeline Integration Test

Usage: $0 [OPTIONS]

Options:
    --full-build           Run full image build test (time-consuming)
    --no-validation        Skip validation tests
    --no-cleanup           Keep test files after completion
    -h, --help             Show this help message

Test Modes:
    Default: Quick validation of pipeline components
    --full-build: Complete end-to-end build and validation test

Examples:
    $0                     # Quick pipeline validation
    $0 --full-build        # Full end-to-end test
    $0 --no-cleanup        # Keep test artifacts

EOF
}

# Parse command line arguments
parse_arguments() {
    while [[ $# -gt 0 ]]; do
        case $1 in
            --full-build)
                RUN_FULL_BUILD=true
                shift
                ;;
            --no-validation)
                RUN_VALIDATION=false
                shift
                ;;
            --no-cleanup)
                CLEANUP_AFTER_TEST=false
                shift
                ;;
            -h|--help)
                show_usage
                exit 0
                ;;
            *)
                log_error "Unknown option: $1"
                ;;
        esac
    done
}

# Setup test environment
setup_test_environment() {
    log_info "Setting up test environment..."
    
    mkdir -p "$TEST_OUTPUT_DIR"
    mkdir -p "$TEST_LOG_DIR"
    
    # Create test log
    cat > "$TEST_LOG_DIR/test-session.log" << EOF
VexFS Build Pipeline Integration Test
====================================

Test Session: $(date)
Test Mode: $([ "$RUN_FULL_BUILD" = true ] && echo "Full Build" || echo "Quick Validation")
Validation: $RUN_VALIDATION
Cleanup: $CLEANUP_AFTER_TEST

EOF
    
    log_success "Test environment ready"
}

# Test 1: Validate dependencies
test_dependencies() {
    log_info "Test 1: Validating dependencies..."
    
    local missing_deps=()
    local required_tools=("packer" "qemu-system-x86_64" "qemu-img" "ssh")
    
    for tool in "${required_tools[@]}"; do
        if command -v "$tool" >/dev/null 2>&1; then
            log_success "  ✓ $tool found"
        else
            missing_deps+=("$tool")
            log_error "  ✗ $tool missing"
        fi
    done
    
    if [ ${#missing_deps[@]} -eq 0 ]; then
        log_success "Test 1: All dependencies satisfied"
        return 0
    else
        log_error "Test 1: Missing dependencies: ${missing_deps[*]}"
        return 1
    fi
}

# Test 2: Validate Packer configuration
test_packer_configuration() {
    log_info "Test 2: Validating Packer configuration..."
    
    local packer_config="$SCRIPT_DIR/packer/vexfs-production.pkr.hcl"
    
    if [ ! -f "$packer_config" ]; then
        log_error "Test 2: Packer configuration not found: $packer_config"
        return 1
    fi
    
    log_info "  Validating Packer syntax..."
    if packer validate "$packer_config" > "$TEST_LOG_DIR/packer-validate.log" 2>&1; then
        log_success "  ✓ Packer configuration is valid"
    else
        log_error "  ✗ Packer configuration validation failed"
        log_error "    Check log: $TEST_LOG_DIR/packer-validate.log"
        return 1
    fi
    
    log_success "Test 2: Packer configuration validation passed"
    return 0
}

# Test 3: Validate preseed files
test_preseed_files() {
    log_info "Test 3: Validating preseed files..."
    
    local variants=("minimal" "development" "testing" "production")
    local all_valid=true
    
    for variant in "${variants[@]}"; do
        local preseed_file="$SCRIPT_DIR/http/preseed-${variant}.cfg"
        
        if [ -f "$preseed_file" ]; then
            log_success "  ✓ $variant preseed file found"
            
            # Basic syntax validation
            if grep -q "d-i debian-installer/locale" "$preseed_file"; then
                log_success "    ✓ Contains required debian-installer directives"
            else
                log_error "    ✗ Missing required debian-installer directives"
                all_valid=false
            fi
        else
            log_error "  ✗ $variant preseed file missing: $preseed_file"
            all_valid=false
        fi
    done
    
    if [ "$all_valid" = true ]; then
        log_success "Test 3: All preseed files validation passed"
        return 0
    else
        log_error "Test 3: Preseed files validation failed"
        return 1
    fi
}

# Test 4: Validate build scripts
test_build_scripts() {
    log_info "Test 4: Validating build scripts..."
    
    local scripts=(
        "$SCRIPT_DIR/build-images.sh"
        "$SCRIPT_DIR/validate-images.sh"
        "$SCRIPT_DIR/ci-build-pipeline.sh"
    )
    
    local all_valid=true
    
    for script in "${scripts[@]}"; do
        local script_name=$(basename "$script")
        
        if [ -f "$script" ]; then
            log_success "  ✓ $script_name found"
            
            if [ -x "$script" ]; then
                log_success "    ✓ $script_name is executable"
            else
                log_error "    ✗ $script_name is not executable"
                all_valid=false
            fi
            
            # Basic syntax check
            if bash -n "$script"; then
                log_success "    ✓ $script_name syntax is valid"
            else
                log_error "    ✗ $script_name has syntax errors"
                all_valid=false
            fi
        else
            log_error "  ✗ $script_name missing: $script"
            all_valid=false
        fi
    done
    
    if [ "$all_valid" = true ]; then
        log_success "Test 4: Build scripts validation passed"
        return 0
    else
        log_error "Test 4: Build scripts validation failed"
        return 1
    fi
}

# Test 5: Validate VexFS source
test_vexfs_source() {
    log_info "Test 5: Validating VexFS source..."
    
    local project_root="$(dirname "$SCRIPT_DIR")"
    
    # Check main Cargo.toml
    if [ -f "$project_root/Cargo.toml" ]; then
        log_success "  ✓ Main Cargo.toml found"
    else
        log_error "  ✗ Main Cargo.toml missing"
        return 1
    fi
    
    # Check Makefile
    if [ -f "$project_root/Makefile" ]; then
        log_success "  ✓ Makefile found"
    else
        log_error "  ✗ Makefile missing"
        return 1
    fi
    
    # Check vexctl
    if [ -d "$project_root/vexctl" ] && [ -f "$project_root/vexctl/Cargo.toml" ]; then
        log_success "  ✓ vexctl source found"
    else
        log_error "  ✗ vexctl source missing"
        return 1
    fi
    
    # Quick syntax check
    log_info "  Running Rust syntax check..."
    cd "$project_root"
    if cargo check --lib --target=x86_64-unknown-linux-gnu > "$TEST_LOG_DIR/rust-check.log" 2>&1; then
        log_success "  ✓ Rust syntax check passed"
    else
        log_error "  ✗ Rust syntax check failed"
        log_error "    Check log: $TEST_LOG_DIR/rust-check.log"
        return 1
    fi
    
    log_success "Test 5: VexFS source validation passed"
    return 0
}

# Test 6: Test build script functionality (dry run)
test_build_script_dry_run() {
    log_info "Test 6: Testing build script functionality (dry run)..."
    
    # Test help output
    if "$SCRIPT_DIR/build-images.sh" --help > "$TEST_LOG_DIR/build-help.log" 2>&1; then
        log_success "  ✓ Build script help works"
    else
        log_error "  ✗ Build script help failed"
        return 1
    fi
    
    # Test validation script help
    if "$SCRIPT_DIR/validate-images.sh" --help > "$TEST_LOG_DIR/validate-help.log" 2>&1; then
        log_success "  ✓ Validation script help works"
    else
        log_error "  ✗ Validation script help failed"
        return 1
    fi
    
    # Test CI script help
    if "$SCRIPT_DIR/ci-build-pipeline.sh" --help > "$TEST_LOG_DIR/ci-help.log" 2>&1; then
        log_success "  ✓ CI script help works"
    else
        log_error "  ✗ CI script help failed"
        return 1
    fi
    
    log_success "Test 6: Build script functionality test passed"
    return 0
}

# Test 7: Full build test (optional)
test_full_build() {
    if [ "$RUN_FULL_BUILD" = false ]; then
        log_info "Test 7: Skipping full build test (use --full-build to enable)"
        return 0
    fi
    
    log_info "Test 7: Running full build test..."
    log_warning "This test will take 15-30 minutes to complete"
    
    # Run minimal build only for testing
    local build_cmd=(
        "$SCRIPT_DIR/build-images.sh"
        --variants minimal
        --vexfs-version "test-$(date +%Y%m%d%H%M%S)"
        --output-dir "$TEST_OUTPUT_DIR"
        --log-dir "$TEST_LOG_DIR"
        --no-validation
    )
    
    log_info "  Starting minimal image build..."
    if "${build_cmd[@]}" > "$TEST_LOG_DIR/full-build.log" 2>&1; then
        log_success "  ✓ Full build test completed successfully"
        
        # Check if image was created
        if find "$TEST_OUTPUT_DIR" -name "*.qcow2" | grep -q .; then
            log_success "  ✓ Image file created successfully"
        else
            log_error "  ✗ No image file found after build"
            return 1
        fi
        
        return 0
    else
        log_error "  ✗ Full build test failed"
        log_error "    Check log: $TEST_LOG_DIR/full-build.log"
        return 1
    fi
}

# Test 8: Validation test (if images exist)
test_validation() {
    if [ "$RUN_VALIDATION" = false ]; then
        log_info "Test 8: Skipping validation test (disabled)"
        return 0
    fi
    
    log_info "Test 8: Testing validation functionality..."
    
    # Check if we have any images to validate
    local test_images=$(find "$TEST_OUTPUT_DIR" -name "*.qcow2" 2>/dev/null || true)
    
    if [ -z "$test_images" ]; then
        log_info "  No test images found, skipping validation test"
        log_info "  (Run with --full-build to create test images)"
        return 0
    fi
    
    log_info "  Found test images, running validation..."
    
    # Run validation with boot-only test (faster)
    local validation_cmd=(
        "$SCRIPT_DIR/validate-images.sh"
        --images-dir "$TEST_OUTPUT_DIR"
        --test-type boot-only
        --boot-timeout 180
        --no-cleanup
    )
    
    if "${validation_cmd[@]}" > "$TEST_LOG_DIR/validation-test.log" 2>&1; then
        log_success "  ✓ Validation test completed successfully"
        return 0
    else
        log_error "  ✗ Validation test failed"
        log_error "    Check log: $TEST_LOG_DIR/validation-test.log"
        return 1
    fi
}

# Generate test report
generate_test_report() {
    log_info "Generating test report..."
    
    local report_file="$TEST_LOG_DIR/integration-test-report.txt"
    
    cat > "$report_file" << EOF
VexFS Build Pipeline Integration Test Report
===========================================

Test Date: $(date)
Test Mode: $([ "$RUN_FULL_BUILD" = true ] && echo "Full Build" || echo "Quick Validation")

Test Results:
EOF
    
    # Add test results to report
    echo "  1. Dependencies: $TEST_1_RESULT" >> "$report_file"
    echo "  2. Packer Configuration: $TEST_2_RESULT" >> "$report_file"
    echo "  3. Preseed Files: $TEST_3_RESULT" >> "$report_file"
    echo "  4. Build Scripts: $TEST_4_RESULT" >> "$report_file"
    echo "  5. VexFS Source: $TEST_5_RESULT" >> "$report_file"
    echo "  6. Script Functionality: $TEST_6_RESULT" >> "$report_file"
    echo "  7. Full Build: $TEST_7_RESULT" >> "$report_file"
    echo "  8. Validation: $TEST_8_RESULT" >> "$report_file"
    
    cat >> "$report_file" << EOF

Summary:
  Total Tests: 8
  Passed: $TESTS_PASSED
  Failed: $TESTS_FAILED
  Skipped: $TESTS_SKIPPED

Logs Directory: $TEST_LOG_DIR
Output Directory: $TEST_OUTPUT_DIR

EOF
    
    log_success "Test report generated: $report_file"
}

# Cleanup test environment
cleanup_test() {
    if [ "$CLEANUP_AFTER_TEST" = false ]; then
        log_info "Skipping cleanup (disabled)"
        return
    fi
    
    log_info "Cleaning up test environment..."
    
    # Clean up test images (they can be large)
    find "$TEST_OUTPUT_DIR" -name "*.qcow2" -delete 2>/dev/null || true
    
    # Keep logs but clean up large temporary files
    find "$TEST_OUTPUT_DIR" -type f -size +100M -delete 2>/dev/null || true
    
    log_success "Cleanup completed"
}

# Main test execution
main() {
    echo "🧪 VexFS Build Pipeline Integration Test"
    echo "========================================"
    echo "Test Mode: $([ "$RUN_FULL_BUILD" = true ] && echo "Full Build" || echo "Quick Validation")"
    echo ""
    
    parse_arguments "$@"
    setup_test_environment
    
    # Initialize test counters
    TESTS_PASSED=0
    TESTS_FAILED=0
    TESTS_SKIPPED=0
    
    # Run tests
    if test_dependencies; then
        TEST_1_RESULT="PASSED"
        ((TESTS_PASSED++))
    else
        TEST_1_RESULT="FAILED"
        ((TESTS_FAILED++))
    fi
    
    if test_packer_configuration; then
        TEST_2_RESULT="PASSED"
        ((TESTS_PASSED++))
    else
        TEST_2_RESULT="FAILED"
        ((TESTS_FAILED++))
    fi
    
    if test_preseed_files; then
        TEST_3_RESULT="PASSED"
        ((TESTS_PASSED++))
    else
        TEST_3_RESULT="FAILED"
        ((TESTS_FAILED++))
    fi
    
    if test_build_scripts; then
        TEST_4_RESULT="PASSED"
        ((TESTS_PASSED++))
    else
        TEST_4_RESULT="FAILED"
        ((TESTS_FAILED++))
    fi
    
    if test_vexfs_source; then
        TEST_5_RESULT="PASSED"
        ((TESTS_PASSED++))
    else
        TEST_5_RESULT="FAILED"
        ((TESTS_FAILED++))
    fi
    
    if test_build_script_dry_run; then
        TEST_6_RESULT="PASSED"
        ((TESTS_PASSED++))
    else
        TEST_6_RESULT="FAILED"
        ((TESTS_FAILED++))
    fi
    
    if test_full_build; then
        if [ "$RUN_FULL_BUILD" = true ]; then
            TEST_7_RESULT="PASSED"
            ((TESTS_PASSED++))
        else
            TEST_7_RESULT="SKIPPED"
            ((TESTS_SKIPPED++))
        fi
    else
        TEST_7_RESULT="FAILED"
        ((TESTS_FAILED++))
    fi
    
    if test_validation; then
        if [ "$RUN_VALIDATION" = true ]; then
            TEST_8_RESULT="PASSED"
            ((TESTS_PASSED++))
        else
            TEST_8_RESULT="SKIPPED"
            ((TESTS_SKIPPED++))
        fi
    else
        TEST_8_RESULT="FAILED"
        ((TESTS_FAILED++))
    fi
    
    echo ""
    generate_test_report
    cleanup_test
    
    echo ""
    log_info "Test Summary:"
    log_info "  Total Tests: 8"
    log_info "  Passed: $TESTS_PASSED"
    log_info "  Failed: $TESTS_FAILED"
    log_info "  Skipped: $TESTS_SKIPPED"
    
    if [ $TESTS_FAILED -eq 0 ]; then
        echo ""
        log_success "🎉 All tests passed! Build pipeline is ready for use."
        exit 0
    else
        echo ""
        log_error "❌ $TESTS_FAILED test(s) failed. Check logs for details."
        exit 1
    fi
}

# Run main function with all arguments
main "$@"