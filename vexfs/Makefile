# Makefile for VexFS kernel module

# Default target: build the kernel module
default: all

# Variables
KERNEL_DIR ?= /lib/modules/$(shell uname -r)/build
MODULE_NAME = vexfs
RUST_TARGET = x86_64-unknown-linux-gnu # Adjust as needed
CARGO_TARGET_DIR = target # Cargo's root target directory
RUST_LIB_NAME = lib$(MODULE_NAME).a
RUST_STATIC_LIB = $(CARGO_TARGET_DIR)/$(RUST_TARGET)/release/$(RUST_LIB_NAME)

# This is the name Kbuild expects for the Rust static library in the current directory
KBUILD_RUST_LIB = $(RUST_LIB_NAME)

# Source files this Makefile directly depends on
C_SHIM_SRC = vexfs_module_entry.c
RUST_SRC_FILES = $(shell find src -name '*.rs') Cargo.toml Cargo.lock

# Default target
default: all

# Main target to build the kernel module
all: $(MODULE_NAME).ko

# Rule to build the Rust static library
$(RUST_STATIC_LIB): $(RUST_SRC_FILES)
	@echo "Building Rust static library $(RUST_LIB_NAME)..."
	cargo build --release --target=$(RUST_TARGET)

# Rule to copy the Rust static library to the location Kbuild expects
$(KBUILD_RUST_LIB): $(RUST_STATIC_LIB)
	@echo "Copying Rust static library to current directory..."
	cp $(RUST_STATIC_LIB) $(KBUILD_RUST_LIB)

# Rule to build the kernel module (.ko file)
# Depends on:
#   - The C shim source file (vexfs_module_entry.c)
#   - The Kbuild file itself
#   - The Rust static library being copied to the current directory
$(MODULE_NAME).ko: $(C_SHIM_SRC) Kbuild $(KBUILD_RUST_LIB)
	@echo "Building VexFS kernel module object $(MODULE_NAME).ko..."
	# Invoke kernel build system
	# The Kbuild file will handle compiling vexfs_module_entry.c
	# and linking it with $(KBUILD_RUST_LIB)
	$(MAKE) -C $(KERNEL_DIR) M=$(shell pwd) modules

clean:
	@echo "Cleaning VexFS build artifacts..."
	# Clean Rust build artifacts
	cargo clean
	# Clean kernel module artifacts
	# The `make -C ... clean` command handles Kbuild generated files like .o, .ko, .mod.c etc.
	if [ -d "$(KERNEL_DIR)" ]; then \
		$(MAKE) -C $(KERNEL_DIR) M=$(shell pwd) clean; \
	else \
		echo "Kernel directory $(KERNEL_DIR) not found, skipping kernel clean."; \
	fi
	# Remove the copied Rust static library
	rm -f $(KBUILD_RUST_LIB)
	# Explicitly remove other common artifacts not always caught by Kbuild's clean
	rm -f *.o *.ko *.mod.* modules.order Module.symvers .*.cmd .tmp_versions/ -rf

.PHONY: all clean default
