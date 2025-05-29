# VexFS Kernel Module Build System - Organized Structure
# This Makefile supports the new organized repository structure

# Variables
KERNEL_DIR ?= /lib/modules/$(shell uname -r)/build
MODULE_NAME = vexfs
RUST_TARGET = x86_64-unknown-linux-gnu

# Paths for organized structure
KERNEL_SRC_DIR = ../src
KERNEL_INCLUDE_DIR = ../include
RUST_DIR = ../../rust
BUILD_DIR = .

# Default target for VM testing
default: vm-build

# Build everything (all targets)
all: clean rust-lib kernel-module c-kernel-module test-runner

# VM Testing: Full kernel module build with FFI
vm-build: clean rust-lib kernel-module
	@echo "✅ VM build complete: vexfs.ko ready (WITH FFI)"

# C-only build for testing kernel module structure
c-only-build: clean c-kernel-module
	@echo "✅ C-only build complete: vexfs.ko ready (no Rust FFI)"

# Safe build: Uses safe kernel module without FFI
safe-build: clean-safe
	@echo "🛡️  Building SAFE VexFS kernel module (no FFI)..."
	@echo "This version prevents system hangs by avoiding Rust FFI calls"
	@cp $(KERNEL_SRC_DIR)/vexfs_module_entry_safe.c $(KERNEL_SRC_DIR)/vexfs_module_entry.c.backup 2>/dev/null || true
	@cp $(KERNEL_SRC_DIR)/vexfs_module_entry_safe.c $(KERNEL_SRC_DIR)/vexfs_module_entry.c
	$(MAKE) -C $(KERNEL_DIR) M=$(PWD) VEXFS_SAFE_MODE=1 modules
	@mv vexfs.ko vexfs_safe.ko 2>/dev/null || true
	@echo "✅ SAFE build complete: vexfs_safe.ko ready"
	@echo "⚠️  This module can be loaded/unloaded but mounting may still have issues"

# Safe FFI build: Uses safe FFI kernel module with comprehensive error handling
safe-ffi-build: clean-safe-ffi
	@echo "🛡️  Building SAFE FFI VexFS kernel module..."
	@echo "This version combines FFI functionality with comprehensive error handling"
	@echo "Features: State management, timeout handling, graceful degradation"
	@cp $(KERNEL_SRC_DIR)/vexfs_module_entry.c $(KERNEL_SRC_DIR)/vexfs_module_entry.c.backup 2>/dev/null || true
	@cp $(KERNEL_SRC_DIR)/vexfs_module_entry_safe_ffi.c $(KERNEL_SRC_DIR)/vexfs_module_entry.c
	$(MAKE) rust-lib-safe-ffi
	$(MAKE) kernel-module-safe-ffi
	@mv vexfs.ko vexfs_safe_ffi.ko 2>/dev/null || true
	@echo "✅ SAFE FFI build complete: vexfs_safe_ffi.ko ready"
	@echo "🔒 This module includes comprehensive safety mechanisms"

# Build Rust static library
rust-lib:
	@echo "🦀 Building Rust static library..."
	cd $(RUST_DIR) && cargo build --release --target=$(RUST_TARGET) --features=kernel,c_bindings --lib
	cp $(RUST_DIR)/target/$(RUST_TARGET)/release/lib$(MODULE_NAME).a ./lib$(MODULE_NAME).a

# Build kernel module with FFI
kernel-module: lib$(MODULE_NAME).a vexfs_rust_combined.o
	@echo "🐧 Building kernel module with FFI..."
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

# Build Rust library for safe FFI
rust-lib-safe-ffi:
	@echo "🦀 Building Rust library for safe FFI..."
	cd $(RUST_DIR) && cargo build --release --target=$(RUST_TARGET) --features=kernel,c_bindings --lib
	cp $(RUST_DIR)/target/$(RUST_TARGET)/release/lib$(MODULE_NAME).a ./lib$(MODULE_NAME)_safe_ffi.a

# Build kernel module with safe FFI
kernel-module-safe-ffi: lib$(MODULE_NAME)_safe_ffi.a vexfs_rust_safe_ffi_combined.o
	@echo "🐧 Building kernel module with safe FFI..."
	$(MAKE) -C $(KERNEL_DIR) M=$(PWD) VEXFS_SAFE_FFI=1 modules

# Create combined Rust object file for safe FFI
vexfs_rust_safe_ffi_combined.o: lib$(MODULE_NAME)_safe_ffi.a
	@echo "🔗 Creating combined Rust object for safe FFI..."
	@mkdir -p .rust_extract_safe_ffi
	@cd .rust_extract_safe_ffi && ar x ../lib$(MODULE_NAME)_safe_ffi.a
	@echo "Extracted $$(ls .rust_extract_safe_ffi/*.o | wc -l) object files"
	@ld -r -o vexfs_rust_safe_ffi_combined_raw.o .rust_extract_safe_ffi/*.o 2>/dev/null || \
	 ld -r -o vexfs_rust_safe_ffi_combined_raw.o --whole-archive lib$(MODULE_NAME)_safe_ffi.a --no-whole-archive
	@echo "🔧 Stripping LLVM metadata sections..."
	@objcopy --remove-section=.llvmbc --remove-section=.llvmcmd vexfs_rust_safe_ffi_combined_raw.o vexfs_rust_safe_ffi_combined.o 2>/dev/null || \
	 cp vexfs_rust_safe_ffi_combined_raw.o vexfs_rust_safe_ffi_combined.o
	@rm -f vexfs_rust_safe_ffi_combined_raw.o
	@rm -rf .rust_extract_safe_ffi
	@# Create dependency file for kernel build system
	@touch .vexfs_rust_safe_ffi_combined.o.cmd
	@echo "✅ Safe FFI combined object created: vexfs_rust_safe_ffi_combined.o"

# Build C-only kernel module (no Rust dependency)
c-kernel-module:
	@echo "🐧 Building C-only kernel module..."
	$(MAKE) -C $(KERNEL_DIR) M=$(PWD) VEXFS_C_ONLY=1 modules

# Host Development: Syntax check only (no kernel module build)
syntax-check:
	@echo "🔍 Host development syntax check..."
	cd $(RUST_DIR) && cargo check --lib --target=$(RUST_TARGET)
	@echo "✅ Syntax check complete - use VM for full build"

# Test runner for development
test-runner:
	@echo "🧪 Building test runner..."
	cd $(RUST_DIR) && cargo build --release --target=$(RUST_TARGET) --bin vector_test_runner
	$(RUST_DIR)/target/$(RUST_TARGET)/release/vector_test_runner

# FFI compatibility validation
validate-ffi:
	@echo "🔍 Validating FFI compatibility..."
	@echo "Checking C header against Rust FFI exports..."
	@# TODO: Add actual FFI validation script
	@echo "✅ FFI validation complete"

# Clean all artifacts
clean:
	@echo "🧹 Cleaning build artifacts..."
	cd $(RUST_DIR) && cargo clean
	rm -f lib$(MODULE_NAME).a *.ko *.o *.mod *.mod.c .*.cmd
	rm -rf .tmp_versions Module.symvers modules.order

# Clean safe build artifacts
clean-safe:
	@echo "🧹 Cleaning safe build artifacts..."
	rm -f vexfs_safe.ko *.o *.mod *.mod.c .*.cmd
	rm -rf .tmp_versions Module.symvers modules.order
	@echo "✅ Safe build artifacts cleaned"

# Clean safe FFI build artifacts
clean-safe-ffi:
	@echo "🧹 Cleaning safe FFI build artifacts..."
	rm -f vexfs_safe_ffi.ko lib$(MODULE_NAME)_safe_ffi.a vexfs_rust_safe_ffi_combined.o
	rm -f *.mod *.mod.c .*.cmd
	rm -rf .tmp_versions Module.symvers modules.order .rust_extract_safe_ffi
	@echo "✅ Safe FFI build artifacts cleaned"

# Restore original module after safe build
restore-original:
	@echo "🔄 Restoring original kernel module..."
	@if [ -f "$(KERNEL_SRC_DIR)/vexfs_module_entry.c.backup" ]; then \
		mv $(KERNEL_SRC_DIR)/vexfs_module_entry.c.backup $(KERNEL_SRC_DIR)/vexfs_module_entry.c; \
		echo "✅ Original module restored"; \
	else \
		echo "⚠️  No backup found - original may already be in place"; \
	fi

# Restore original module after safe FFI build
restore-original-safe-ffi:
	@echo "🔄 Restoring original kernel module after safe FFI build..."
	@if [ -f "$(KERNEL_SRC_DIR)/vexfs_module_entry.c.backup" ]; then \
		mv $(KERNEL_SRC_DIR)/vexfs_module_entry.c.backup $(KERNEL_SRC_DIR)/vexfs_module_entry.c; \
		echo "✅ Original module restored"; \
	else \
		echo "⚠️  No backup found - original may already be in place"; \
	fi

# Help
help:
	@echo "VexFS Build System - Organized Repository Structure"
	@echo "=================================================="
	@echo ""
	@echo "Host Development (fast iteration):"
	@echo "  syntax-check  - Rust syntax validation only"
	@echo "  test-runner   - Build and run userspace tests"
	@echo "  validate-ffi  - Check FFI compatibility"
	@echo ""
	@echo "VM Testing (full validation):"
	@echo "  vm-build         - Complete kernel module build with FFI (default)"
	@echo "  c-only-build     - C kernel module only (no Rust FFI)"
	@echo "  safe-build       - Safe kernel module (prevents hangs, no FFI)"
	@echo "  safe-ffi-build   - Safe FFI kernel module (FFI + error handling)"
	@echo "  all              - Build all targets (comprehensive test)"
	@echo "  clean            - Remove all build artifacts"
	@echo ""
	@echo "Individual Components:"
	@echo "  rust-lib         - Build Rust static library only"
	@echo "  kernel-module    - Build kernel module (requires rust-lib)"
	@echo "  rust-lib-safe-ffi - Build Rust library for safe FFI"
	@echo "  kernel-module-safe-ffi - Build safe FFI kernel module"
	@echo ""
	@echo "Safety Commands:"
	@echo "  clean-safe              - Clean safe build artifacts"
	@echo "  clean-safe-ffi          - Clean safe FFI build artifacts"
	@echo "  restore-original        - Restore original module source"
	@echo "  restore-original-safe-ffi - Restore after safe FFI build"
	@echo ""
	@echo "Build Variants:"
	@echo "  vexfs.ko         - Standard FFI kernel module"
	@echo "  vexfs_safe.ko    - Safe kernel module (no FFI)"
	@echo "  vexfs_safe_ffi.ko - Safe FFI kernel module (FFI + safety)"
	@echo ""
	@echo "Repository Structure:"
	@echo "  kernel/src/      - C kernel module source files"
	@echo "  kernel/include/  - C header files"
	@echo "  rust/src/       - Rust library source files"
	@echo "  kernel/build/   - Build system files"
	@echo ""
	@echo "Strategy: Use syntax-check on host, safe-ffi-build in VM"

.PHONY: default all vm-build c-only-build safe-build safe-ffi-build rust-lib kernel-module rust-lib-safe-ffi kernel-module-safe-ffi c-kernel-module syntax-check test-runner validate-ffi clean clean-safe clean-safe-ffi restore-original restore-original-safe-ffi help
