# VexFS Modular Kernel Module Makefile
# Phase 1: Minimal VFS-compliant Real Filesystem

# Main module object
obj-m += vexfs.o

# Core filesystem components (required for Phase 1)
vexfs-core-objs := core/main.o core/superblock.o core/block.o core/inode.o core/dir.o core/file.o

# Semantic components (optional for Phase 1, can be enabled later)
vexfs-semantic-objs := semantic/vector_ops.o semantic/search.o semantic/embedding.o

# Build only core components for Phase 1
vexfs-objs := $(vexfs-core-objs)

# Kernel build directory
KDIR := /lib/modules/$(shell uname -r)/build

# Build flags for optimization and debugging
ccflags-y := -O2 -Wall -Wextra -DDEBUG

# Default target
all:
	$(MAKE) -C $(KDIR) M=$(PWD) modules

# Clean target
clean:
	$(MAKE) -C $(KDIR) M=$(PWD) clean
	rm -f core/*.o semantic/*.o

# Install target
install: all
	sudo insmod vexfs.ko

# Uninstall target
uninstall:
	sudo rmmod vexfs || true

# Test target - mount the filesystem on a real block device
test: install
	@echo "VexFS Phase 1 loaded. To test with real block device:"
	@echo "1. Create a test file: sudo dd if=/dev/zero of=/tmp/vexfs_test.img bs=1M count=100"
	@echo "2. Setup loop device: sudo losetup /dev/loop0 /tmp/vexfs_test.img"
	@echo "3. Format with VexFS: sudo mkfs.vexfs /dev/loop0"
	@echo "4. Mount: sudo mkdir -p /mnt/vexfs && sudo mount -t vexfs /dev/loop0 /mnt/vexfs"

# Unmount test
untest:
	sudo umount /mnt/vexfs || true
	sudo losetup -d /dev/loop0 || true
	rm -f /tmp/vexfs_test.img || true

# Full cycle: clean, build, install, test
cycle: clean all install test

# Enable semantic components (for future phases)
semantic:
	$(eval vexfs-objs := $(vexfs-core-objs) $(vexfs-semantic-objs))
	$(MAKE) all

.PHONY: all clean install uninstall test untest cycle semantic