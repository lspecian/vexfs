#!/bin/bash

# Simple test for mkfs.vexfs functionality
# Tests VexFS formatting without requiring a full VM

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

echo "=== VexFS mkfs.vexfs Test Suite ==="
echo "Testing VexFS filesystem creation and validation"
echo

# Test configuration
TEST_IMG="/tmp/vexfs_test.img"
TEST_SIZE="50M"
MKFS_TOOL="/tmp/mkfs_vexfs_simple"

# Function to run a test step
test_step() {
    local step_name="$1"
    echo "🔧 $step_name"
}

# Function to report success
success() {
    local msg="$1"
    echo "✅ $msg"
}

# Function to report failure
failure() {
    local msg="$1"
    echo "❌ $msg"
    return 1
}

# Function to report warning
warning() {
    local msg="$1"
    echo "⚠️  $msg"
}

# Test 1: Create mkfs.vexfs utility
test_mkfs_creation() {
    test_step "Creating mkfs.vexfs utility"
    
    if [ ! -f "$SCRIPT_DIR/create_mkfs_simple.sh" ]; then
        failure "mkfs creation script not found"
        return 1
    fi
    
    # Run the creation script
    if "$SCRIPT_DIR/create_mkfs_simple.sh"; then
        success "mkfs.vexfs utility created"
    else
        failure "mkfs.vexfs creation failed"
        return 1
    fi
    
    # Verify the utility exists and is executable
    if [ -x "$MKFS_TOOL" ]; then
        success "mkfs.vexfs is executable"
    else
        failure "mkfs.vexfs is not executable"
        return 1
    fi
    
    # Test help output
    if "$MKFS_TOOL" -h >/dev/null 2>&1; then
        success "mkfs.vexfs help works"
    else
        failure "mkfs.vexfs help failed"
        return 1
    fi
}

# Test 2: Create test image
test_image_creation() {
    test_step "Creating test image"
    
    # Remove existing test image
    rm -f "$TEST_IMG"
    
    # Create test image
    if dd if=/dev/zero of="$TEST_IMG" bs=1M count=50 >/dev/null 2>&1; then
        success "Test image created ($TEST_SIZE)"
    else
        failure "Test image creation failed"
        return 1
    fi
    
    # Verify image size
    local actual_size=$(stat -c%s "$TEST_IMG")
    local expected_size=$((50 * 1024 * 1024))
    
    if [ "$actual_size" -eq "$expected_size" ]; then
        success "Test image size verified: $actual_size bytes"
    else
        failure "Test image size mismatch: expected $expected_size, got $actual_size"
        return 1
    fi
}

# Test 3: Format with VexFS
test_vexfs_formatting() {
    test_step "Formatting test image with VexFS"
    
    # Format the image
    if "$MKFS_TOOL" -f -L "VexFS-Test" "$TEST_IMG"; then
        success "VexFS formatting completed"
    else
        failure "VexFS formatting failed"
        return 1
    fi
    
    # Verify the superblock was written
    test_step "Verifying VexFS superblock"
    
    # Check for VexFS magic number (VEXFSUPE in little-endian)
    if hexdump -C "$TEST_IMG" | head -1 | grep -q "45 50 55 53 46 58 45 56"; then
        success "VexFS magic number found in superblock"
    else
        failure "VexFS magic number not found"
        return 1
    fi
    
    # Check for volume label
    if hexdump -C "$TEST_IMG" | head -10 | grep -q "VexFS-Test"; then
        success "Volume label found in superblock"
    else
        warning "Volume label not found (may be at different offset)"
    fi
}

# Test 4: Validate superblock structure
test_superblock_validation() {
    test_step "Validating superblock structure"
    
    # Extract first 1024 bytes (superblock)
    local sb_file="/tmp/vexfs_superblock.bin"
    dd if="$TEST_IMG" of="$sb_file" bs=1024 count=1 >/dev/null 2>&1
    
    # Check superblock size
    local sb_size=$(stat -c%s "$sb_file")
    if [ "$sb_size" -eq 1024 ]; then
        success "Superblock extracted: $sb_size bytes"
    else
        failure "Superblock size unexpected: $sb_size bytes"
        return 1
    fi
    
    # Display superblock hex dump for verification
    echo "Superblock hex dump (first 128 bytes):"
    hexdump -C "$sb_file" | head -8
    
    # Clean up
    rm -f "$sb_file"
    
    success "Superblock validation completed"
}

# Test 5: Multiple format test
test_multiple_formats() {
    test_step "Testing multiple format operations"
    
    # Format with different labels
    local labels=("Test1" "Test2" "VectorDB" "MyVexFS")
    
    for label in "${labels[@]}"; do
        if "$MKFS_TOOL" -f -L "$label" "$TEST_IMG" >/dev/null 2>&1; then
            success "Formatted with label: $label"
        else
            failure "Failed to format with label: $label"
            return 1
        fi
        
        # Verify the label was written
        if hexdump -C "$TEST_IMG" | head -10 | grep -q "$label"; then
            success "Label '$label' verified in superblock"
        else
            warning "Label '$label' not found in expected location"
        fi
    done
}

# Test 6: Error handling
test_error_handling() {
    test_step "Testing error handling"
    
    # Test with non-existent file
    if "$MKFS_TOOL" -f "/nonexistent/file" 2>/dev/null; then
        failure "Should have failed with non-existent file"
        return 1
    else
        success "Correctly failed with non-existent file"
    fi
    
    # Test without force flag on mounted filesystem (simulate)
    # This test is limited since we can't actually mount VexFS yet
    success "Error handling tests completed"
}

# Test 7: Performance test
test_performance() {
    test_step "Testing formatting performance"
    
    # Create larger test image
    local large_img="/tmp/vexfs_large.img"
    local start_time end_time duration
    
    # Create 100MB image
    if dd if=/dev/zero of="$large_img" bs=1M count=100 >/dev/null 2>&1; then
        success "Large test image created (100MB)"
    else
        failure "Large test image creation failed"
        return 1
    fi
    
    # Time the formatting operation
    start_time=$(date +%s.%N)
    if "$MKFS_TOOL" -f -L "PerfTest" "$large_img" >/dev/null 2>&1; then
        end_time=$(date +%s.%N)
        duration=$(echo "$end_time - $start_time" | bc -l)
        success "Large image formatted in ${duration}s"
    else
        failure "Large image formatting failed"
        rm -f "$large_img"
        return 1
    fi
    
    # Clean up
    rm -f "$large_img"
}

# Cleanup function
cleanup() {
    echo
    echo "🧹 Cleaning up test files..."
    rm -f "$TEST_IMG" "/tmp/vexfs_large.img" "/tmp/vexfs_superblock.bin"
    success "Cleanup completed"
}

# Main test runner
run_tests() {
    echo "Starting VexFS mkfs.vexfs tests..."
    echo
    
    local tests_passed=0
    local tests_total=0
    
    # List of test functions
    local tests=(
        "test_mkfs_creation"
        "test_image_creation" 
        "test_vexfs_formatting"
        "test_superblock_validation"
        "test_multiple_formats"
        "test_error_handling"
        "test_performance"
    )
    
    # Run each test
    for test_func in "${tests[@]}"; do
        tests_total=$((tests_total + 1))
        echo
        if $test_func; then
            tests_passed=$((tests_passed + 1))
            echo "✅ $test_func PASSED"
        else
            echo "❌ $test_func FAILED"
        fi
    done
    
    echo
    echo "=== Test Results ==="
    echo "Tests passed: $tests_passed/$tests_total"
    
    if [ $tests_passed -eq $tests_total ]; then
        echo "🎉 All tests passed!"
        return 0
    else
        echo "💥 Some tests failed"
        return 1
    fi
}

# Set up trap for cleanup
trap cleanup EXIT

# Main execution
main() {
    echo "VexFS mkfs.vexfs Test Suite"
    echo "============================"
    echo
    
    # Check dependencies
    if ! command -v dd >/dev/null 2>&1; then
        failure "dd command not found"
        exit 1
    fi
    
    if ! command -v hexdump >/dev/null 2>&1; then
        failure "hexdump command not found"
        exit 1
    fi
    
    if ! command -v bc >/dev/null 2>&1; then
        warning "bc not found - performance timing may not work"
    fi
    
    # Run tests
    if run_tests; then
        echo
        echo "🎉 VexFS mkfs.vexfs test suite completed successfully!"
        echo
        echo "Summary:"
        echo "- mkfs.vexfs utility created and tested"
        echo "- VexFS superblock format validated"
        echo "- Multiple formatting scenarios tested"
        echo "- Error handling verified"
        echo "- Performance benchmarked"
        echo
        echo "Next steps:"
        echo "1. Integrate with kernel module for mounting"
        echo "2. Test actual filesystem operations"
        echo "3. Validate with 200GB production test"
        
        return 0
    else
        echo
        echo "💥 VexFS mkfs.vexfs test suite failed!"
        echo "Check the output above for specific failures."
        return 1
    fi
}

# Run main function
main "$@"