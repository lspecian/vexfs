#!/bin/bash

# VexFS FFI Compatibility Validation Script
# This script validates that the C FFI header matches the Rust FFI exports

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
KERNEL_INCLUDE_DIR="$PROJECT_ROOT/kernel/include"
RUST_SRC_DIR="$PROJECT_ROOT/rust/src"

echo "🔍 VexFS FFI Compatibility Check"
echo "================================"
echo "Project Root: $PROJECT_ROOT"
echo "Kernel Include: $KERNEL_INCLUDE_DIR"
echo "Rust Source: $RUST_SRC_DIR"
echo ""

# Check if required files exist
echo "📁 Checking required files..."
if [ ! -f "$KERNEL_INCLUDE_DIR/vexfs_ffi.h" ]; then
    echo "❌ Error: vexfs_ffi.h not found in $KERNEL_INCLUDE_DIR"
    exit 1
fi

if [ ! -f "$RUST_SRC_DIR/ffi/kernel.rs" ]; then
    echo "❌ Error: kernel.rs not found in $RUST_SRC_DIR/ffi/"
    exit 1
fi

echo "✅ Required files found"
echo ""

# Extract function declarations from C header (improved regex to handle void *)
echo "🔍 Extracting C function declarations..."
C_FUNCTIONS=$(grep -E "^(int|void|void\s*\*)\s+vexfs_rust_|^\s*void\s*\*\s*vexfs_rust_" "$KERNEL_INCLUDE_DIR/vexfs_ffi.h" | \
              sed 's/;//' | \
              sed 's/^\s*//' | \
              sort)

echo "Found C functions:"
echo "$C_FUNCTIONS" | sed 's/^/  /'
echo ""

# Extract function declarations from Rust FFI
echo "🔍 Extracting Rust FFI exports..."
RUST_FUNCTIONS=$(grep -E "pub extern \"C\" fn vexfs_rust_" "$RUST_SRC_DIR/ffi/kernel.rs" | \
                 sed 's/#\[no_mangle\]//' | \
                 sed 's/pub extern "C" fn //' | \
                 sed 's/{.*//' | \
                 sed 's/^\s*//' | \
                 sort)

echo "Found Rust functions:"
echo "$RUST_FUNCTIONS" | sed 's/^/  /'
echo ""

# Extract function names for comparison
echo "🔍 Comparing function signatures..."
C_FUNC_NAMES=$(echo "$C_FUNCTIONS" | sed -E 's/.*(vexfs_rust_[^(]*).*/\1/' | sort)
RUST_FUNC_NAMES=$(echo "$RUST_FUNCTIONS" | sed 's/\(vexfs_rust_[^(]*\).*/\1/' | sort)

echo "C function names:"
echo "$C_FUNC_NAMES" | sed 's/^/  /'
echo ""

echo "Rust function names:"
echo "$RUST_FUNC_NAMES" | sed 's/^/  /'
echo ""

# Check for missing functions
MISSING_IN_RUST=$(comm -23 <(echo "$C_FUNC_NAMES") <(echo "$RUST_FUNC_NAMES"))
MISSING_IN_C=$(comm -13 <(echo "$C_FUNC_NAMES") <(echo "$RUST_FUNC_NAMES"))

if [ -n "$MISSING_IN_RUST" ]; then
    echo "❌ Functions declared in C but missing in Rust:"
    echo "$MISSING_IN_RUST" | sed 's/^/  /'
    echo ""
fi

if [ -n "$MISSING_IN_C" ]; then
    echo "❌ Functions exported by Rust but missing in C:"
    echo "$MISSING_IN_C" | sed 's/^/  /'
    echo ""
fi

# Check error code constants
echo "🔍 Checking error code constants..."
C_ERROR_CODES=$(grep -E "#define VEXFS_ERROR_" "$KERNEL_INCLUDE_DIR/vexfs_ffi.h" | \
                awk '{print $2 " " $3}' | sort)
RUST_ERROR_CODES=$(grep -E "pub const VEXFS_ERROR_" "$RUST_SRC_DIR/ffi/mod.rs" | \
                   sed 's/pub const //' | \
                   sed 's/: c_int = / /' | \
                   sed 's/;//' | \
                   sed 's/\s*\/\/.*$//' | \
                   sort)

echo "C error codes:"
echo "$C_ERROR_CODES" | sed 's/^/  /'
echo ""

echo "Rust error codes:"
echo "$RUST_ERROR_CODES" | sed 's/^/  /'
echo ""

# Final validation
ERRORS=0

if [ -n "$MISSING_IN_RUST" ] || [ -n "$MISSING_IN_C" ]; then
    echo "❌ FFI function signature mismatch detected"
    ERRORS=$((ERRORS + 1))
else
    echo "✅ FFI function signatures match"
fi

# Check if error codes match
if ! diff -q <(echo "$C_ERROR_CODES") <(echo "$RUST_ERROR_CODES") > /dev/null 2>&1; then
    echo "❌ Error code constants mismatch detected"
    echo ""
    echo "🔍 Detailed comparison:"
    echo "C codes not in Rust:"
    comm -23 <(echo "$C_ERROR_CODES") <(echo "$RUST_ERROR_CODES") | sed 's/^/  /'
    echo "Rust codes not in C:"
    comm -13 <(echo "$C_ERROR_CODES") <(echo "$RUST_ERROR_CODES") | sed 's/^/  /'
    ERRORS=$((ERRORS + 1))
else
    echo "✅ Error code constants match"
fi

echo ""
if [ $ERRORS -eq 0 ]; then
    echo "🎉 FFI compatibility validation PASSED"
    echo "✅ C header and Rust FFI exports are compatible"
    echo ""
    echo "📋 Summary:"
    echo "  ✅ All FFI function signatures match"
    echo "  ✅ All error code constants match"
    echo "  ✅ Repository is ready for FFI bridge implementation"
    exit 0
else
    echo "💥 FFI compatibility validation FAILED"
    echo "❌ Found $ERRORS compatibility issues"
    echo ""
    echo "🔧 To fix these issues:"
    echo "1. Update the C header to match Rust exports, or"
    echo "2. Update the Rust FFI to match C declarations, or"
    echo "3. Add missing functions to maintain compatibility"
    exit 1
fi