# Makefile for VexFS kernel module - VM Testing Only
# This Makefile is designed for VM testing environment only
# For host development, use: make syntax-check

# Variables
KERNEL_DIR ?= /lib/modules/$(shell uname -r)/build
MODULE_NAME = vexfs
RUST_TARGET = x86_64-unknown-linux-gnu

# Default target for VM testing
default: vm-build

# Build everything (all targets)
all: clean rust-lib kernel-module c-kernel-module test-runner

# VM Testing: Full kernel module build
vm-build: clean rust-lib kernel-module

# C-only build for testing kernel module structure
c-only-build: clean c-kernel-module
	@echo "✅ C-only build complete: vexfs.ko ready (no Rust FFI)"
	@echo "✅ VM build complete: vexfs.ko ready"

# Build Rust static library
rust-lib:
	@echo "🦀 Building Rust static library..."
	cargo build --release --target=$(RUST_TARGET) --features=kernel-minimal --lib
	cp target/$(RUST_TARGET)/release/lib$(MODULE_NAME).a ./lib$(MODULE_NAME).a

# Build kernel module
kernel-module: lib$(MODULE_NAME).a vexfs_rust_combined.o
	@echo "🐧 Building kernel module..."
	$(MAKE) -C $(KERNEL_DIR) M=$(PWD) modules

# Create combined Rust object file
vexfs_rust_combined.o: lib$(MODULE_NAME).a
	@echo "🔗 Creating combined Rust object..."
	@mkdir -p .rust_extract
	@cd .rust_extract && ar x ../lib$(MODULE_NAME).a
	@echo "Extracted $$(ls .rust_extract/*.o | wc -l) object files"
	@ld -r -o vexfs_rust_combined_raw.o .rust_extract/*.o 2>/dev/null || \
	 ld -r -o vexfs_rust_combined_raw.o --whole-archive lib$(MODULE_NAME).a --no-whole-archive
	@echo "🔧 Stripping LLVM metadata sections..."
	@objcopy --remove-section=.llvmbc --remove-section=.llvmcmd vexfs_rust_combined_raw.o vexfs_rust_combined.o 2>/dev/null || \
	 cp vexfs_rust_combined_raw.o vexfs_rust_combined.o
	@rm -f vexfs_rust_combined_raw.o
	@rm -rf .rust_extract
	@# Create dependency file for kernel build system
	@touch .vexfs_rust_combined.o.cmd
	@echo "✅ Combined object created: vexfs_rust_combined.o"

# Build C-only kernel module (no Rust dependency)
c-kernel-module:
	@echo "🐧 Building C-only kernel module..."
	$(MAKE) -C $(KERNEL_DIR) M=$(PWD) VEXFS_C_ONLY=1 modules

# Host Development: Syntax check only (no kernel module build)
syntax-check:
	@echo "🔍 Host development syntax check..."
	cargo check --lib --target=$(RUST_TARGET)
	@echo "✅ Syntax check complete - use VM for full build"

# Test runner for development
test-runner:
	@echo "🧪 Building test runner..."
	cargo build --release --target=$(RUST_TARGET) --bin vector_test_runner
	./target/$(RUST_TARGET)/release/vector_test_runner

# Clean all artifacts
clean:
	@echo "🧹 Cleaning build artifacts..."
	cargo clean
	rm -f lib$(MODULE_NAME).a *.ko *.o *.mod *.mod.c .*.cmd
	rm -rf .tmp_versions Module.symvers modules.order

# Help
help:
	@echo "VexFS Build System - Two-Tier Strategy"
	@echo "======================================"
	@echo ""
	@echo "Host Development (fast iteration):"
	@echo "  syntax-check  - Rust syntax validation only"
	@echo "  test-runner   - Build and run userspace tests"
	@echo ""
	@echo "VM Testing (full validation):"
	@echo "  vm-build      - Complete kernel module build (default)"
	@echo "  c-only-build  - C kernel module only (no Rust FFI)"
	@echo "  all           - Build all targets (comprehensive test)"
	@echo "  clean         - Remove all build artifacts"
	@echo ""
	@echo "Individual Components:"
	@echo "  rust-lib      - Build Rust static library only"
	@echo "  kernel-module - Build kernel module (requires rust-lib)"
	@echo ""
	@echo "Strategy: Use syntax-check on host, vm-build in VM"

.PHONY: default all vm-build c-only-build rust-lib kernel-module c-kernel-module syntax-check test-runner clean help
