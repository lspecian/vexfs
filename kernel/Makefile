# Simple VexFS Minimal C Stub Makefile
obj-m := vexfs_minimal.o
vexfs_minimal-objs := vexfs_minimal_stub.o

KDIR := /lib/modules/$(shell uname -r)/build

all:
	@echo "Building VexFS minimal C stub..."
	$(MAKE) -C $(KDIR) M=$(PWD) modules
	@echo "Build complete: vexfs_minimal.ko"

clean:
	$(MAKE) -C $(KDIR) M=$(PWD) clean

.PHONY: all clean