# VexFS Kernel Module Makefile
# Copyright (C) 2025 VexFS Contributors
# Licensed under GPL v2

# Module name and object files
obj-m := vexfs.o
vexfs-objs := src/vexfs_module_entry.o vexfs_rust_combined.o unwind_stub.o rust_eh_personality_stub.o

# Kernel build directory (auto-detected)
KDIR := /lib/modules/$(shell uname -r)/build

# Current directory
PWD := $(shell pwd)

# Rust project directory
RUST_DIR := ../rust

# Build targets
all: rust_static_lib
	$(MAKE) -C $(KDIR) M=$(PWD) modules

# Build Rust static library for kernel integration
rust_static_lib:
	@echo "Checking for existing Rust static library..."
	@if [ ! -f vexfs_rust_combined.o ]; then \
		echo "Building Rust static library for kernel integration..."; \
		cd $(RUST_DIR) && cargo +nightly build --release --no-default-features --features kernel,c_bindings --target x86_64-unknown-none; \
		echo "Extracting and combining Rust object files..."; \
		mkdir -p rust_objects; \
		cd rust_objects && ar x ../$(RUST_DIR)/target/x86_64-unknown-none/release/libvexfs.a; \
		ld -r -o ../vexfs_rust_combined.o *.o; \
		cd .. && rm -rf rust_objects; \
		echo "Rust static library ready: vexfs_rust_combined.o"; \
	else \
		echo "Rust static library already exists: vexfs_rust_combined.o"; \
	fi

# Create stub files if they don't exist
stubs: unwind_stub.o rust_eh_personality_stub.o

unwind_stub.o: unwind_stub.c
	$(CC) -c -o $@ $<

rust_eh_personality_stub.o: rust_eh_personality_stub.c
	$(CC) -c -o $@ $<

# Clean build artifacts
clean:
	$(MAKE) -C $(KDIR) M=$(PWD) clean
	rm -f *.o *.ko *.mod.c .*.cmd Module.symvers modules.order
	rm -f vexfs_rust_combined.o
	cd $(RUST_DIR) && cargo clean

# Install module (requires root)
install:
	$(MAKE) -C $(KDIR) M=$(PWD) modules_install
	depmod -a

# Uninstall module (requires root)
uninstall:
	rm -f /lib/modules/$(shell uname -r)/extra/vexfs.ko
	depmod -a

# Load module (requires root)
load:
	insmod vexfs.ko

# Unload module (requires root)
unload:
	rmmod vexfs

# Test module info
info:
	modinfo vexfs.ko

# Help target
help:
	@echo "VexFS Kernel Module Build System"
	@echo "================================"
	@echo "Available targets:"
	@echo "  all       - Build the kernel module (default)"
	@echo "  clean     - Clean build artifacts"
	@echo "  install   - Install module to system (requires root)"
	@echo "  uninstall - Remove module from system (requires root)"
	@echo "  load      - Load module into kernel (requires root)"
	@echo "  unload    - Unload module from kernel (requires root)"
	@echo "  info      - Show module information"
	@echo "  stubs     - Create stub object files"
	@echo "  help      - Show this help message"
	@echo ""
	@echo "Prerequisites:"
	@echo "  - Linux kernel headers installed"
	@echo "  - Rust toolchain with kernel target support"
	@echo "  - GCC compiler"

.PHONY: all clean install uninstall load unload info stubs help rust_static_lib