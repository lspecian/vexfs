# VexFS v2.0 - Organized Kernel Module Build System
# Unified Makefile for building the kernel module from organized structure

obj-m += vexfs_v2_phase3.o

# Phase 3 Advanced Indexing Components from organized directories
vexfs_v2_phase3-objs := core/vexfs_v2_main.o \
                        search/vexfs_v2_search.o \
                        search/vexfs_v2_advanced_search.o \
                        search/vexfs_v2_hnsw.o \
                        search/vexfs_v2_lsh.o \
                        search/vexfs_v2_multi_model.o \
                        search/vexfs_v2_phase3_integration.o \
                        utils/vexfs_v2_monitoring.o

# Kernel build directory
KDIR := /lib/modules/$(shell uname -r)/build

# Build flags for optimization and Phase 3 features
ccflags-y := -O2 -Wall -Wextra -DVEXFS_PHASE3_ENABLED

# Include paths for organized headers
ccflags-y += -I$(src)/core -I$(src)/search -I$(src)/uapi -I$(src)/utils

# Default target
all:
	$(MAKE) -C $(KDIR) M=$(PWD) modules

# Clean target
clean:
	$(MAKE) -C $(KDIR) M=$(PWD) clean
	find . -name "*.o" -delete
	find . -name "*.ko" -delete
	find . -name "*.mod*" -delete
	find . -name ".*.cmd" -delete
	find . -name "modules.order" -delete
	find . -name "Module.symvers" -delete

# Install target
install: all
	sudo insmod vexfs_v2_phase3.ko

# Uninstall target
uninstall:
	sudo rmmod vexfs_v2_phase3 || true

# Test target - mount the filesystem
test: install
	mkdir -p /tmp/vexfs_v2_test
	sudo mount -t vexfs_v2 none /tmp/vexfs_v2_test
	@echo "VexFS v2.0 mounted at /tmp/vexfs_v2_test"
	@echo "Check dmesg for SIMD capabilities and performance info!"

# Unmount test
untest:
	sudo umount /tmp/vexfs_v2_test || true
	rmdir /tmp/vexfs_v2_test || true

# Full cycle: clean, build, install, test
cycle: clean all install test

# Build tests (userspace)
tests:
	$(MAKE) -C tests -f ../build/Makefile.comparison_tests all

# Build utilities
utils:
	$(MAKE) -C utils -f ../build/Makefile.performance all

# Help target
help:
	@echo "VexFS v2.0 Organized Build System"
	@echo "=================================="
	@echo ""
	@echo "Main targets:"
	@echo "  all       - Build the kernel module"
	@echo "  clean     - Clean build artifacts"
	@echo "  install   - Install the kernel module"
	@echo "  uninstall - Remove the kernel module"
	@echo "  test      - Install and mount for testing"
	@echo "  untest    - Unmount and cleanup test"
	@echo "  cycle     - Full clean/build/install/test cycle"
	@echo ""
	@echo "Additional targets:"
	@echo "  tests     - Build userspace test programs"
	@echo "  utils     - Build utility programs"
	@echo "  help      - Show this help message"
	@echo ""
	@echo "Directory structure:"
	@echo "  core/     - Core filesystem functionality"
	@echo "  search/   - Vector search implementations"
	@echo "  uapi/     - User-space API headers"
	@echo "  tests/    - Test programs"
	@echo "  utils/    - Utilities and benchmarks"
	@echo "  build/    - Build configurations and scripts"

.PHONY: all clean install uninstall test untest cycle tests utils help