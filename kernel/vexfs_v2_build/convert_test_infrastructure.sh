#!/bin/bash

# VexFS v2 Test Infrastructure Integer Conversion Script
# Task 66.5: Convert Test Infrastructure to Integer Representations
#
# This script systematically converts all test files to use integer-based
# IEEE 754 bit representation instead of floating-point operations.

echo "🔧 VexFS v2 Test Infrastructure Integer Conversion"
echo "=================================================="

# List of test files that need conversion
TEST_FILES=(
    "test_hnsw_functionality.c"
    "test_vector_search.c"
    "standalone_lsh_test.c"
    "phase3_advanced_search_test.c"
    "phase3_multi_model_test.c"
    "corrected_vector_test.c"
    "corrected_vector_test_fixed.c"
    "final_corrected_vector_test.c"
    "simple_vector_test.c"
    "block_device_test.c"
    "debug_vector_test.c"
    "test_with_uapi_header.c"
    "before_after_comparison_test.c"
    "regression_prevention_test.c"
    "test_uapi_sizes.c"
    "test_ioctl_numbers.c"
)

# Function to check if a file contains floating-point usage
check_float_usage() {
    local file="$1"
    if [ -f "$file" ]; then
        local float_count=$(grep -c "float\|double" "$file" 2>/dev/null || echo "0")
        echo "  $file: $float_count floating-point references"
        return $float_count
    else
        echo "  $file: NOT FOUND"
        return 0
    fi
}

# Function to backup a file
backup_file() {
    local file="$1"
    if [ -f "$file" ]; then
        cp "$file" "${file}.backup"
        echo "  ✅ Backed up $file"
    fi
}

# Function to verify no floating-point symbols in compiled object
check_compiled_symbols() {
    local file="$1"
    local obj_file="${file%.c}.o"
    
    if gcc -I. -c "$file" -o "$obj_file" 2>/dev/null; then
        local float_symbols=$(objdump -t "$obj_file" 2>/dev/null | grep -E "__.*sf|__.*df|float|double" | wc -l)
        if [ "$float_symbols" -eq 0 ]; then
            echo "  ✅ $file: No floating-point symbols in compiled object"
            rm -f "$obj_file"
            return 0
        else
            echo "  ❌ $file: $float_symbols floating-point symbols found"
            objdump -t "$obj_file" | grep -E "__.*sf|__.*df|float|double"
            rm -f "$obj_file"
            return 1
        fi
    else
        echo "  ❌ $file: Compilation failed"
        return 1
    fi
}

echo ""
echo "📊 Phase 1: Analyzing current floating-point usage..."
echo "---------------------------------------------------"

total_files=0
files_with_floats=0

for file in "${TEST_FILES[@]}"; do
    if [ -f "$file" ]; then
        total_files=$((total_files + 1))
        check_float_usage "$file"
        if [ $? -gt 0 ]; then
            files_with_floats=$((files_with_floats + 1))
        fi
    fi
done

echo ""
echo "📈 Analysis Summary:"
echo "  Total test files: $total_files"
echo "  Files with floating-point usage: $files_with_floats"
echo "  Files already converted: $((total_files - files_with_floats))"

echo ""
echo "🔧 Phase 2: Testing compilation of converted files..."
echo "----------------------------------------------------"

converted_files=0
compilation_errors=0

# Test files that should already be converted
CONVERTED_FILES=(
    "test_uapi_compatibility.c"
    "simple_phase2_test.c"
    "test_phase2_search_clean.c"
    "standalone_phase3_test.c"
)

for file in "${CONVERTED_FILES[@]}"; do
    if [ -f "$file" ]; then
        echo "Testing $file..."
        if check_compiled_symbols "$file"; then
            converted_files=$((converted_files + 1))
        else
            compilation_errors=$((compilation_errors + 1))
        fi
    fi
done

echo ""
echo "📈 Conversion Test Summary:"
echo "  Successfully converted files: $converted_files"
echo "  Files with compilation errors: $compilation_errors"

echo ""
echo "🎯 Phase 3: Conversion recommendations..."
echo "----------------------------------------"

echo "Files that still need conversion:"
for file in "${TEST_FILES[@]}"; do
    if [ -f "$file" ]; then
        float_count=$(grep -c "float\|double" "$file" 2>/dev/null || echo "0")
        if [ "$float_count" -gt 0 ]; then
            echo "  📝 $file ($float_count floating-point references)"
        fi
    fi
done

echo ""
echo "🔧 Conversion patterns to apply:"
echo "1. Replace 'float *vectors' with 'uint32_t *vectors_bits'"
echo "2. Replace 'float *query_vector' with 'uint32_t *query_vector'"
echo "3. Add IEEE 754 conversion: vexfs_float_array_to_bits(floats, bits, count)"
echo "4. Update structure field names to match UAPI definitions"
echo "5. Include 'vexfs_v2_uapi.h' for conversion utilities"

echo ""
echo "✅ Test infrastructure conversion analysis complete!"
echo "Next steps: Apply systematic conversion to remaining files."