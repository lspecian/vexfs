# VexFS v2.0 - Full Kernel-Native Vector Filesystem with Phase 3 Advanced Indexing
# Makefile for building the enhanced kernel module with all components

obj-m += vexfs_v2_phase3.o

# Phase 3 Advanced Indexing Components
vexfs_v2_phase3-objs := vexfs_v2_main.o \
                        vexfs_v2_search.o \
                        vexfs_v2_monitoring.o \
                        vexfs_v2_multi_model.o \
                        vexfs_v2_advanced_search.o \
                        vexfs_v2_hnsw.o \
                        vexfs_v2_lsh.o \
                        vexfs_v2_phase3_integration.o

# Kernel build directory
KDIR := /lib/modules/$(shell uname -r)/build

# Build flags for optimization and Phase 3 features
ccflags-y := -O2 -Wall -Wextra -DVEXFS_PHASE3_ENABLED

# Default target
all:
	$(MAKE) -C $(KDIR) M=$(PWD) modules

# Clean target
clean:
	$(MAKE) -C $(KDIR) M=$(PWD) clean

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

.PHONY: all clean install uninstall test untest cycle