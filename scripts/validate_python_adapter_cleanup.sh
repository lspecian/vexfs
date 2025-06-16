#!/bin/bash
# VexFS Python Qdrant Adapter Cleanup Validation Script
# Task 70: Clean Up and Deprecate Python Qdrant Adapter

set -e

echo "VexFS Python Qdrant Adapter Cleanup Validation"
echo "=============================================="
echo ""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Counters
TESTS_PASSED=0
TESTS_FAILED=0
WARNINGS=0

# Function to print test results
print_result() {
    local test_name="$1"
    local result="$2"
    local message="$3"
    
    if [ "$result" = "PASS" ]; then
        echo -e "✅ ${GREEN}PASS${NC}: $test_name"
        [ -n "$message" ] && echo "   $message"
        ((TESTS_PASSED++))
    elif [ "$result" = "FAIL" ]; then
        echo -e "❌ ${RED}FAIL${NC}: $test_name"
        [ -n "$message" ] && echo "   $message"
        ((TESTS_FAILED++))
    elif [ "$result" = "WARN" ]; then
        echo -e "⚠️  ${YELLOW}WARN${NC}: $test_name"
        [ -n "$message" ] && echo "   $message"
        ((WARNINGS++))
    fi
}

# Test 1: Verify no Python Qdrant adapter code exists
echo "1. Checking for Python Qdrant adapter code..."
ADAPTER_FILES=$(find . -path ./archive -prune -o -name "*.py" -exec grep -l "vexfs.*qdrant.*adapter\|qdrant.*adapter.*vexfs\|python.*qdrant.*adapter" {} \; 2>/dev/null | grep -v "./archive")
if [ -n "$ADAPTER_FILES" ]; then
    print_result "Python adapter code removal" "FAIL" "Found Python Qdrant adapter code in codebase"
else
    print_result "Python adapter code removal" "PASS" "No Python Qdrant adapter code found"
fi

# Test 2: Verify Rust Qdrant implementation exists
echo ""
echo "2. Checking for Rust Qdrant implementation..."
if [ -f "rust/src/dialects/qdrant.rs" ] && [ -f "rust/src/dialects/qdrant_optimized.rs" ]; then
    print_result "Rust Qdrant implementation" "PASS" "Rust implementation found"
else
    print_result "Rust Qdrant implementation" "FAIL" "Rust implementation missing"
fi

# Test 3: Verify documentation exists
echo ""
echo "3. Checking for deprecation documentation..."
if [ -f "docs/deprecation/PYTHON_QDRANT_ADAPTER_DEPRECATION.md" ]; then
    print_result "Deprecation documentation" "PASS" "Documentation exists"
else
    print_result "Deprecation documentation" "FAIL" "Deprecation documentation missing"
fi

# Test 4: Verify README updates
echo ""
echo "4. Checking README updates..."
if grep -q "Qdrant API Compatibility" README.md && grep -q "Python Adapter Deprecation" README.md; then
    print_result "README updates" "PASS" "README contains Qdrant and deprecation information"
else
    print_result "README updates" "FAIL" "README missing Qdrant or deprecation information"
fi

# Test 5: Verify .gitignore protection
echo ""
echo "5. Checking .gitignore protection..."
if [ -f ".gitignore_python_adapter" ]; then
    print_result "Gitignore protection" "PASS" "Python adapter gitignore rules created"
else
    print_result "Gitignore protection" "WARN" "Consider adding .gitignore rules for Python adapter"
fi

# Test 6: Check for Python dependencies
echo ""
echo "6. Checking for Python dependencies..."
PYTHON_DEPS_FOUND=false
for file in requirements.txt setup.py pyproject.toml Pipfile; do
    if [ -f "$file" ] && grep -q "qdrant" "$file" 2>/dev/null; then
        PYTHON_DEPS_FOUND=true
        break
    fi
done

if [ "$PYTHON_DEPS_FOUND" = true ]; then
    print_result "Python dependencies cleanup" "WARN" "Found Python Qdrant dependencies in project files"
else
    print_result "Python dependencies cleanup" "PASS" "No Python Qdrant dependencies found"
fi

# Test 7: Verify Rust tests exist
echo ""
echo "7. Checking for Rust Qdrant tests..."
if [ -f "rust/tests/qdrant_api_test.rs" ] && [ -f "rust/tests/qdrant_performance_test.rs" ]; then
    print_result "Rust Qdrant tests" "PASS" "Rust test suite exists"
else
    print_result "Rust Qdrant tests" "FAIL" "Rust test suite missing"
fi

# Test 8: Check build system
echo ""
echo "8. Checking build system..."
if [ -f "rust/Cargo.toml" ] && grep -q "vexfs" "rust/Cargo.toml" 2>/dev/null; then
    print_result "Build system" "PASS" "Rust build system configured"
else
    print_result "Build system" "FAIL" "Rust build system not properly configured"
fi

# Test 9: Verify unified server binary
echo ""
echo "9. Checking for unified server..."
if [ -f "rust/src/bin/vexfs_unified_server.rs" ]; then
    print_result "Unified server" "PASS" "VexFS unified server exists"
else
    print_result "Unified server" "FAIL" "VexFS unified server missing"
fi

# Test 10: Check for archive directory (if Python code was archived)
echo ""
echo "10. Checking for archive directory..."
if [ -d "archive/python-qdrant-adapter" ]; then
    print_result "Archive directory" "PASS" "Python adapter archived for reference"
elif find . -name "*python*qdrant*" -o -name "*qdrant*python*" 2>/dev/null | head -1 > /dev/null; then
    print_result "Archive directory" "WARN" "Python adapter code found but not archived"
else
    print_result "Archive directory" "PASS" "No Python adapter code to archive"
fi

# Summary
echo ""
echo "=============================================="
echo "Validation Summary"
echo "=============================================="
echo -e "Tests Passed: ${GREEN}$TESTS_PASSED${NC}"
echo -e "Tests Failed: ${RED}$TESTS_FAILED${NC}"
echo -e "Warnings: ${YELLOW}$WARNINGS${NC}"
echo ""

if [ $TESTS_FAILED -eq 0 ]; then
    echo -e "${GREEN}✅ Python Qdrant Adapter Cleanup: SUCCESSFUL${NC}"
    echo ""
    echo "Key achievements:"
    echo "• Python adapter code removed/not present"
    echo "• Rust implementation verified"
    echo "• Documentation created"
    echo "• README updated"
    echo "• Build system validated"
    echo ""
    echo "VexFS now uses high-performance Rust Qdrant implementation!"
    exit 0
else
    echo -e "${RED}❌ Python Qdrant Adapter Cleanup: INCOMPLETE${NC}"
    echo ""
    echo "Issues found:"
    echo "• $TESTS_FAILED test(s) failed"
    echo "• $WARNINGS warning(s) issued"
    echo ""
    echo "Please address the failed tests before considering cleanup complete."
    exit 1
fi