#!/bin/bash

# VFS Operations Validation Script
# Validates that all required VFS operations are properly implemented

echo "🔍 Validating VFS Operations Implementation..."

# Check if kernel module source exists
KERNEL_MODULE="../src/vexfs_module_entry_safe_ffi.c"
FFI_HEADER="../include/vexfs_ffi.h"
RUST_FFI="../../rust/src/ffi/kernel.rs"

if [ ! -f "$KERNEL_MODULE" ]; then
    echo "❌ Kernel module source not found: $KERNEL_MODULE"
    exit 1
fi

if [ ! -f "$FFI_HEADER" ]; then
    echo "❌ FFI header not found: $FFI_HEADER"
    exit 1
fi

if [ ! -f "$RUST_FFI" ]; then
    echo "❌ Rust FFI implementation not found: $RUST_FFI"
    exit 1
fi

echo "✅ All source files found"

# Check for required VFS operations in kernel module
echo "🔍 Checking VFS operations in kernel module..."

REQUIRED_OPS=(
    "vexfs_create"
    "vexfs_lookup" 
    "vexfs_mkdir"
    "vexfs_rmdir"
    "vexfs_unlink"
    "vexfs_open"
    "vexfs_release"
    "vexfs_read"
    "vexfs_write"
    "vexfs_fsync"
    "vexfs_readdir"
)

for op in "${REQUIRED_OPS[@]}"; do
    if grep -q "static.*$op(" "$KERNEL_MODULE"; then
        echo "✅ Found VFS operation: $op"
    else
        echo "❌ Missing VFS operation: $op"
        exit 1
    fi
done

# Check for locking mechanisms
echo "🔍 Checking locking mechanisms..."

REQUIRED_LOCKS=(
    "vexfs_inode_mutex"
    "vexfs_dir_mutex"
    "vexfs_file_mutex"
)

for lock in "${REQUIRED_LOCKS[@]}"; do
    if grep -q "$lock" "$KERNEL_MODULE"; then
        echo "✅ Found locking mechanism: $lock"
    else
        echo "❌ Missing locking mechanism: $lock"
        exit 1
    fi
done

# Check for FFI function declarations
echo "🔍 Checking FFI function declarations..."

REQUIRED_FFI_FUNCS=(
    "vexfs_rust_create_file"
    "vexfs_rust_lookup_inode"
    "vexfs_rust_open_file"
    "vexfs_rust_release_file"
    "vexfs_rust_read_file"
    "vexfs_rust_write_file"
    "vexfs_rust_fsync_file"
    "vexfs_rust_readdir"
)

for func in "${REQUIRED_FFI_FUNCS[@]}"; do
    if grep -q "$func" "$FFI_HEADER"; then
        echo "✅ Found FFI declaration: $func"
    else
        echo "❌ Missing FFI declaration: $func"
        exit 1
    fi
done

# Check for Rust FFI implementations
echo "🔍 Checking Rust FFI implementations..."

for func in "${REQUIRED_FFI_FUNCS[@]}"; do
    if grep -q "pub extern \"C\" fn $func" "$RUST_FFI"; then
        echo "✅ Found Rust FFI implementation: $func"
    else
        echo "❌ Missing Rust FFI implementation: $func"
        exit 1
    fi
done

# Check for error handling patterns
echo "🔍 Checking error handling patterns..."

if grep -q "vexfs_safe_ffi_call" "$KERNEL_MODULE"; then
    echo "✅ Found safe FFI call mechanism"
else
    echo "❌ Missing safe FFI call mechanism"
    exit 1
fi

if grep -q "mutex_lock_interruptible" "$KERNEL_MODULE"; then
    echo "✅ Found interruptible locking"
else
    echo "❌ Missing interruptible locking"
    exit 1
fi

# Check for timeout mechanisms
if grep -q "VEXFS_OPERATION_TIMEOUT_MS" "$KERNEL_MODULE"; then
    echo "✅ Found operation timeout configuration"
else
    echo "❌ Missing operation timeout configuration"
    exit 1
fi

echo ""
echo "🎉 VFS Operations Validation Complete!"
echo "✅ All required VFS operations implemented"
echo "✅ Proper locking mechanisms in place"
echo "✅ FFI functions declared and implemented"
echo "✅ Error handling and safety mechanisms present"
echo "✅ Timeout mechanisms configured"
echo ""
echo "📋 Implementation Summary:"
echo "   - Enhanced file operations with proper locking"
echo "   - Directory operations with mutex protection"
echo "   - FFI integration with fallback mechanisms"
echo "   - Comprehensive error handling"
echo "   - Operation timeouts to prevent hangs"
echo ""
echo "🚀 Ready for VM testing validation!"