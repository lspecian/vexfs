# VexFS v2.0 Unified Kernel Module Build System
# Organized Structure - Single Source of Truth

# Module name and version
MODULE_NAME := vexfs_v2_phase3
VERSION := 2.0.0

# Kernel build directory
KDIR := /lib/modules/$(shell uname -r)/build

# Source directories (organized structure)
SRC_CORE := src/core
SRC_SEARCH := src/search  
SRC_UTILS := src/utils
SRC_INCLUDE := src/include

# Object files from organized structure
obj-m += $(MODULE_NAME).o

$(MODULE_NAME)-objs := $(SRC_CORE)/vexfs_v2_main.o \
                       $(SRC_SEARCH)/vexfs_v2_search.o \
                       $(SRC_SEARCH)/vexfs_v2_advanced_search.o \
                       $(SRC_SEARCH)/vexfs_v2_hnsw.o \
                       $(SRC_SEARCH)/vexfs_v2_lsh.o \
                       $(SRC_SEARCH)/vexfs_v2_multi_model.o \
                       $(SRC_SEARCH)/vexfs_v2_phase3_integration.o \
                       $(SRC_UTILS)/vexfs_v2_monitoring.o \
                       $(SRC_UTILS)/vexfs_v2_memory_manager.o \
                       $(SRC_UTILS)/vexfs_v2_vector_cache.o \
                       $(SRC_UTILS)/vexfs_v2_vector_processing.o \
                       $(SRC_UTILS)/vexfs_v2_io_optimization.o \
                       $(SRC_UTILS)/vexfs_v2_comprehensive_monitoring.o \
                       $(SRC_UTILS)/vexfs_v2_ann_index_cache.o \
                       $(SRC_UTILS)/vexfs_v2_enhanced_file_ops.o \
                       $(SRC_UTILS)/vexfs_v2_enhanced_file_ops_part2.o \
                       $(SRC_UTILS)/vexfs_v2_enhanced_ioctl.o \
                       $(SRC_UTILS)/vexfs_v2_enhanced_ioctl_part2.o \
                       $(SRC_UTILS)/vexfs_v2_enhanced_ioctl_utils.o \
                       $(SRC_UTILS)/vexfs_v2_enhanced_registration.o \
                       $(SRC_UTILS)/vexfs_v2_enhanced_registration_part2.o \
                       $(SRC_UTILS)/vexfs_v2_locking.o \
                       $(SRC_UTILS)/vexfs_v2_locking_deadlock.o \
                       $(SRC_UTILS)/vexfs_v2_locking_rcu.o

# Build flags for optimization and Phase 3 features
ccflags-y := -O2 -Wall -Wextra -DVEXFS_PHASE3_ENABLED

# Include paths for organized headers
ccflags-y += -I$(src)/$(SRC_INCLUDE) -I$(src)/$(SRC_CORE) -I$(src)/$(SRC_SEARCH) -I$(src)/$(SRC_UTILS)

# Default target
all:
	$(MAKE) -C $(KDIR) M=$(PWD) modules
	@echo "✅ VexFS v2.0 kernel module built successfully"
	@echo "📁 Module: $(MODULE_NAME).ko"
	@echo "📊 Size: $$(du -h $(MODULE_NAME).ko | cut -f1)"

# Clean target
clean:
	$(MAKE) -C $(KDIR) M=$(PWD) clean
	find . -name "*.o" -delete
	find . -name "*.ko" -delete
	find . -name "*.mod*" -delete
	find . -name ".*.cmd" -delete
	find . -name "modules.order" -delete
	find . -name "Module.symvers" -delete
	@echo "✅ Cleaned build artifacts"

# Install target
install: all
	sudo insmod $(MODULE_NAME).ko
	@echo "✅ VexFS v2.0 kernel module loaded"

# Uninstall target
uninstall:
	sudo rmmod $(MODULE_NAME) || true
	@echo "✅ VexFS v2.0 kernel module unloaded"

# Test target - mount the filesystem
test: install
	mkdir -p /tmp/vexfs_v2_test
	sudo mount -t vexfs_v2_b62 none /tmp/vexfs_v2_test
	@echo "✅ VexFS v2.0 mounted at /tmp/vexfs_v2_test"
	@echo "🔍 Check dmesg for SIMD capabilities and performance info!"

# Unmount test
untest:
	sudo umount /tmp/vexfs_v2_test || true
	rmdir /tmp/vexfs_v2_test || true
	@echo "✅ VexFS v2.0 test unmounted"

# Full cycle: clean, build, install, test
cycle: clean all install test

# Show module info
info: all
	modinfo $(MODULE_NAME).ko

# Show organized structure
structure:
	@echo "📁 VexFS v2.0 Organized Structure:"
	@echo "├── src/"
	@echo "│   ├── core/     - Main kernel module ($(shell ls -1 $(SRC_CORE)/*.c | wc -l) files)"
	@echo "│   ├── search/   - Vector search algorithms ($(shell ls -1 $(SRC_SEARCH)/*.c | wc -l) files)"
	@echo "│   ├── utils/    - Utilities and enhancements ($(shell ls -1 $(SRC_UTILS)/*.c | wc -l) files)"
	@echo "│   └── include/  - Header files ($(shell ls -1 $(SRC_INCLUDE)/*.h | wc -l) files)"
	@echo "├── bin/          - Compiled binaries ($(shell ls -1 bin/ 2>/dev/null | wc -l) files)"
	@echo "├── tests_organized/ - Test programs ($(shell ls -1 tests_organized/*.c 2>/dev/null | wc -l) files)"
	@echo "└── archive/      - Legacy implementations"

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
	@echo "  info      - Show module information"
	@echo "  structure - Show organized directory structure"
	@echo ""
	@echo "🎯 Quick start: make cycle"
	@echo "📁 Structure: make structure"

.PHONY: all clean install uninstall test untest cycle info structure help