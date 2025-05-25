# Makefile for VexFS kernel module - VM Testing Only
# This Makefile is designed for VM testing environment only
# For host development, use: make syntax-check

# Variables
KERNEL_DIR ?= /lib/modules/$(shell uname -r)/build
MODULE_NAME = vexfs
RUST_TARGET = x86_64-unknown-linux-gnu

# Default target for VM testing
default: vm-build

# VM Testing: Full kernel module build
vm-build: clean rust-lib kernel-module
	@echo "✅ VM build complete: vexfs.ko ready"

# Build Rust static library  
rust-lib:
	@echo "🦀 Building Rust static library..."
	cargo build --release --target=$(RUST_TARGET)
	cp target/$(RUST_TARGET)/release/lib$(MODULE_NAME).a ./lib$(MODULE_NAME).a

# Build kernel module
kernel-module: lib$(MODULE_NAME).a
	@echo "🐧 Building kernel module..."
	$(MAKE) -C $(KERNEL_DIR) M=$(PWD) modules

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
	@echo "  vm-build      - Complete kernel module build"
	@echo "  clean         - Remove all build artifacts"
	@echo ""
	@echo "Strategy: Use syntax-check on host, vm-build in VM"

.PHONY: default vm-build rust-lib kernel-module syntax-check test-runner clean help
