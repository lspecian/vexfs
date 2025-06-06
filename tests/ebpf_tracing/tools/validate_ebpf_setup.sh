#!/bin/bash

# VexFS eBPF Tracing Setup Validation Script
# 
# Validates that the eBPF tracing infrastructure is properly installed
# and configured for VexFS kernel module debugging.
#
# Usage: ./validate_ebpf_setup.sh
#
# Author: VexFS Development Team
# Version: 1.0.0

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Counters
TESTS_PASSED=0
TESTS_FAILED=0
WARNINGS=0

# Print colored output
print_color() {
    local color="$1"
    shift
    echo -e "${color}$*${NC}"
}

# Test result functions
test_pass() {
    print_color "$GREEN" "✅ PASS: $1"
    ((TESTS_PASSED++))
}

test_fail() {
    print_color "$RED" "❌ FAIL: $1"
    ((TESTS_FAILED++))
}

test_warn() {
    print_color "$YELLOW" "⚠️  WARN: $1"
    ((WARNINGS++))
}

test_info() {
    print_color "$BLUE" "ℹ️  INFO: $1"
}

# Header
print_color "$CYAN" "=== VexFS eBPF Tracing Setup Validation ==="
print_color "$NC" "Validating eBPF tracing infrastructure..."
echo

# Test 1: Check directory structure
print_color "$PURPLE" "🔍 Testing Directory Structure..."

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
EBPF_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

required_dirs=(
    "scripts"
    "tools" 
    "configs"
    "results"
    "analysis"
)

for dir in "${required_dirs[@]}"; do
    if [[ -d "$EBPF_ROOT/$dir" ]]; then
        test_pass "Directory exists: $dir/"
    else
        test_fail "Missing directory: $dir/"
    fi
done

# Test 2: Check required files
print_color "$PURPLE" "🔍 Testing Required Files..."

required_files=(
    "scripts/vexfs_kernel_trace.bt"
    "scripts/vexfs_performance_trace.bt"
    "scripts/vexfs_memory_trace.bt"
    "tools/vexfs_trace_manager.sh"
    "configs/default_trace_config.yaml"
    "README.md"
)

for file in "${required_files[@]}"; do
    if [[ -f "$EBPF_ROOT/$file" ]]; then
        test_pass "File exists: $file"
    else
        test_fail "Missing file: $file"
    fi
done

# Test 3: Check file permissions
print_color "$PURPLE" "🔍 Testing File Permissions..."

executable_files=(
    "tools/vexfs_trace_manager.sh"
)

for file in "${executable_files[@]}"; do
    if [[ -x "$EBPF_ROOT/$file" ]]; then
        test_pass "Executable: $file"
    else
        test_fail "Not executable: $file"
    fi
done

# Test 4: Check bpftrace scripts syntax
print_color "$PURPLE" "🔍 Testing bpftrace Script Syntax..."

if command -v bpftrace &> /dev/null; then
    for script in "$EBPF_ROOT"/scripts/*.bt; do
        if [[ -f "$script" ]]; then
            script_name=$(basename "$script")
            if bpftrace -d "$script" &> /dev/null; then
                test_pass "Valid syntax: $script_name"
            else
                test_fail "Invalid syntax: $script_name"
            fi
        fi
    done
else
    test_warn "bpftrace not available - skipping syntax validation"
fi

# Test 5: Check system requirements
print_color "$PURPLE" "🔍 Testing System Requirements..."

# Check kernel version
kernel_version=$(uname -r)
kernel_major=$(echo "$kernel_version" | cut -d. -f1)
kernel_minor=$(echo "$kernel_version" | cut -d. -f2)

if [[ $kernel_major -gt 6 || ($kernel_major -eq 6 && $kernel_minor -ge 11) ]]; then
    test_pass "Kernel version: $kernel_version (>= 6.11)"
else
    test_warn "Kernel version: $kernel_version (recommended >= 6.11)"
fi

# Check bpftrace availability
if command -v bpftrace &> /dev/null; then
    bpftrace_version=$(bpftrace --version 2>&1 | head -n1 || echo "unknown")
    test_pass "bpftrace available: $bpftrace_version"
else
    test_fail "bpftrace not found - install with: sudo apt-get install bpftrace"
fi

# Check if running as root (for actual tracing)
if [[ $EUID -eq 0 ]]; then
    test_pass "Running as root (required for kernel tracing)"
else
    test_info "Not running as root (sudo required for actual tracing)"
fi

# Test 6: Check VexFS module status
print_color "$PURPLE" "🔍 Testing VexFS Module Status..."

if lsmod | grep -q "vexfs"; then
    test_pass "VexFS kernel module is loaded"
else
    test_warn "VexFS kernel module not loaded (some traces may not capture events)"
fi

# Test 7: Test trace manager functionality
print_color "$PURPLE" "🔍 Testing Trace Manager Functionality..."

trace_manager="$EBPF_ROOT/tools/vexfs_trace_manager.sh"

if [[ -x "$trace_manager" ]]; then
    # Test help command
    if "$trace_manager" help &> /dev/null; then
        test_pass "Trace manager help command works"
    else
        test_fail "Trace manager help command failed"
    fi
    
    # Test list command
    if "$trace_manager" list &> /dev/null; then
        test_pass "Trace manager list command works"
    else
        test_fail "Trace manager list command failed"
    fi
    
    # Test status command
    if "$trace_manager" status &> /dev/null; then
        test_pass "Trace manager status command works"
    else
        test_fail "Trace manager status command failed"
    fi
else
    test_fail "Trace manager not executable"
fi

# Test 8: Check configuration file validity
print_color "$PURPLE" "🔍 Testing Configuration File..."

config_file="$EBPF_ROOT/configs/default_trace_config.yaml"

if [[ -f "$config_file" ]]; then
    # Basic YAML syntax check (if yq is available)
    if command -v yq &> /dev/null; then
        if yq eval '.' "$config_file" &> /dev/null; then
            test_pass "Configuration file has valid YAML syntax"
        else
            test_fail "Configuration file has invalid YAML syntax"
        fi
    else
        test_info "yq not available - skipping YAML syntax validation"
    fi
    
    # Check for required sections
    required_sections=(
        "general"
        "kernel_trace"
        "performance_trace"
        "memory_trace"
    )
    
    for section in "${required_sections[@]}"; do
        if grep -q "^${section}:" "$config_file"; then
            test_pass "Configuration section exists: $section"
        else
            test_fail "Missing configuration section: $section"
        fi
    done
else
    test_fail "Configuration file not found"
fi

# Test 9: Check documentation
print_color "$PURPLE" "🔍 Testing Documentation..."

readme_file="$EBPF_ROOT/README.md"

if [[ -f "$readme_file" ]]; then
    # Check for required sections
    required_sections=(
        "Overview"
        "Quick Start"
        "Tracing Scripts"
        "Trace Manager Tool"
        "Configuration"
    )
    
    for section in "${required_sections[@]}"; do
        if grep -q "## $section" "$readme_file"; then
            test_pass "Documentation section exists: $section"
        else
            test_warn "Missing documentation section: $section"
        fi
    done
    
    # Check file size (should be comprehensive)
    file_size=$(wc -c < "$readme_file")
    if [[ $file_size -gt 5000 ]]; then
        test_pass "Documentation is comprehensive (${file_size} bytes)"
    else
        test_warn "Documentation may be incomplete (${file_size} bytes)"
    fi
else
    test_fail "README.md not found"
fi

# Test 10: Environment validation
print_color "$PURPLE" "🔍 Testing Environment..."

# Check available disk space for results
available_space=$(df "$EBPF_ROOT" | tail -1 | awk '{print $4}')
if [[ $available_space -gt 1048576 ]]; then  # > 1GB
    test_pass "Sufficient disk space available ($(($available_space / 1024 / 1024)) GB)"
else
    test_warn "Limited disk space available ($(($available_space / 1024)) MB)"
fi

# Check if debugfs is mounted (helpful for some eBPF features)
if mount | grep -q debugfs; then
    test_pass "debugfs is mounted"
else
    test_warn "debugfs not mounted (may limit some eBPF features)"
fi

# Summary
echo
print_color "$CYAN" "=== Validation Summary ==="
print_color "$GREEN" "Tests Passed: $TESTS_PASSED"
print_color "$RED" "Tests Failed: $TESTS_FAILED"
print_color "$YELLOW" "Warnings: $WARNINGS"

if [[ $TESTS_FAILED -eq 0 ]]; then
    print_color "$GREEN" "🎉 VexFS eBPF Tracing Setup: READY"
    echo
    print_color "$BLUE" "Next Steps:"
    print_color "$NC" "1. Load VexFS kernel module (if not already loaded)"
    print_color "$NC" "2. Run: sudo $EBPF_ROOT/tools/vexfs_trace_manager.sh list"
    print_color "$NC" "3. Start tracing: sudo $EBPF_ROOT/tools/vexfs_trace_manager.sh run kernel"
    echo
    exit 0
else
    print_color "$RED" "❌ VexFS eBPF Tracing Setup: ISSUES FOUND"
    echo
    print_color "$YELLOW" "Please fix the failed tests before using the tracing infrastructure."
    echo
    exit 1
fi