#!/bin/bash
# VexFS Kernel Module Build Script
# Copyright (C) 2025 VexFS Contributors
# Licensed under GPL v2

set -e  # Exit on any error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
KERNEL_DIR="$SCRIPT_DIR"
RUST_DIR="$SCRIPT_DIR/../rust"
PROJECT_ROOT="$SCRIPT_DIR/.."

echo -e "${BLUE}VexFS Kernel Module Build System${NC}"
echo -e "${BLUE}=================================${NC}"

# Function to print status messages
print_status() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check prerequisites
check_prerequisites() {
    print_status "Checking prerequisites..."
    
    # Check if we're in the right directory
    if [ ! -f "$KERNEL_DIR/src/vexfs_module_entry.c" ]; then
        print_error "vexfs_module_entry.c not found. Are you in the correct directory?"
        exit 1
    fi
    
    # Check for kernel headers
    KDIR="/lib/modules/$(uname -r)/build"
    if [ ! -d "$KDIR" ]; then
        print_error "Kernel headers not found at $KDIR"
        print_error "Please install kernel headers: sudo apt install linux-headers-$(uname -r)"
        exit 1
    fi
    
    # Check for Rust toolchain
    if ! command -v cargo &> /dev/null; then
        print_error "Rust toolchain not found. Please install Rust."
        exit 1
    fi
    
    # Check for required Rust target
    if ! rustup target list --installed | grep -q "x86_64-unknown-none"; then
        print_warning "Installing required Rust target..."
        rustup target add x86_64-unknown-none
    fi
    
    # Check for nightly toolchain (required for build-std)
    if ! rustup toolchain list | grep -q "nightly"; then
        print_warning "Installing nightly Rust toolchain..."
        rustup toolchain install nightly
    fi
    
    print_status "Prerequisites check completed."
}

# Build Rust static library
build_rust_library() {
    print_status "Building Rust static library for kernel integration..."
    
    cd "$RUST_DIR"
    
    # Clean previous builds
    cargo clean
    
    # Build with kernel-compatible flags using bare-metal target
    # Only build the library, not binaries (binaries require std and can't compile for kernel target)
    RUSTFLAGS="-C code-model=kernel -C relocation-model=static -C panic=abort" \
    cargo +nightly build \
        --release \
        --lib \
        --features kernel,c_bindings \
        --target x86_64-unknown-none \
        --no-default-features \
        -Z build-std=core,alloc \
        -Z build-std-features=compiler-builtins-mem
    
    if [ $? -ne 0 ]; then
        print_error "Rust library build failed"
        exit 1
    fi
    
    # Extract and combine Rust object files
    print_status "Extracting and combining Rust object files..."
    cd "$KERNEL_DIR"
    mkdir -p rust_objects
    cd rust_objects && ar x "$RUST_DIR/target/x86_64-unknown-none/release/libvexfs.a"
    ld -r -o ../vexfs_rust_combined.o *.o
    cd ..
    
    # Strip LLVM bitcode sections that cause kernel build issues
    objcopy --remove-section=.llvmbc --remove-section=.llvmcmd vexfs_rust_combined.o vexfs_rust_combined_clean.o
    mv vexfs_rust_combined_clean.o vexfs_rust_combined.o
    
    rm -rf rust_objects
    
    print_status "Rust static library built successfully"
    cd "$KERNEL_DIR"
}

# Build stub object files
build_stubs() {
    print_status "Building stub object files..."
    
    # Build unwind stub
    if [ -f "unwind_stub.c" ]; then
        gcc -c -o unwind_stub.o unwind_stub.c
        print_status "Built unwind_stub.o"
    else
        print_error "unwind_stub.c not found"
        exit 1
    fi
    
    # Build rust_eh_personality stub
    if [ -f "rust_eh_personality_stub.c" ]; then
        gcc -c -o rust_eh_personality_stub.o rust_eh_personality_stub.c
        print_status "Built rust_eh_personality_stub.o"
    else
        print_error "rust_eh_personality_stub.c not found"
        exit 1
    fi
}

# Build kernel module
build_kernel_module() {
    print_status "Building kernel module..."
    
    # Use the kernel build system
    make -C "/lib/modules/$(uname -r)/build" M="$KERNEL_DIR" modules
    
    if [ $? -ne 0 ]; then
        print_error "Kernel module build failed"
        exit 1
    fi
    
    if [ -f "vexfs.ko" ]; then
        print_status "Kernel module built successfully: vexfs.ko"
        
        # Show module information
        print_status "Module information:"
        modinfo vexfs.ko
        
        # Show file size
        ls -lh vexfs.ko
    else
        print_error "vexfs.ko was not created"
        exit 1
    fi
}

# Clean build artifacts
clean_build() {
    print_status "Cleaning build artifacts..."
    
    # Clean kernel module artifacts
    make -C "/lib/modules/$(uname -r)/build" M="$KERNEL_DIR" clean 2>/dev/null || true
    
    # Clean our artifacts
    rm -f vexfs_rust_combined.o unwind_stub.o rust_eh_personality_stub.o
    
    # Clean Rust artifacts
    cd "$RUST_DIR"
    cargo clean
    cd "$KERNEL_DIR"
    
    print_status "Clean completed"
}

# Main build function
main_build() {
    print_status "Starting VexFS kernel module build..."
    
    check_prerequisites
    build_rust_library
    build_stubs
    build_kernel_module
    
    print_status "Build completed successfully!"
    echo -e "${GREEN}✓ VexFS kernel module (vexfs.ko) is ready${NC}"
    echo ""
    echo "Next steps:"
    echo "  1. Test in VM: sudo insmod vexfs.ko"
    echo "  2. Check dmesg: dmesg | tail"
    echo "  3. Unload: sudo rmmod vexfs"
}

# Handle command line arguments
case "${1:-build}" in
    "build")
        main_build
        ;;
    "clean")
        clean_build
        ;;
    "help")
        echo "Usage: $0 [build|clean|help]"
        echo "  build  - Build the kernel module (default)"
        echo "  clean  - Clean build artifacts"
        echo "  help   - Show this help message"
        ;;
    *)
        print_error "Unknown command: $1"
        echo "Use '$0 help' for usage information"
        exit 1
        ;;
esac