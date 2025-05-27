# Kbuild file for VexFS kernel module
# Supports both C-only builds and full C+Rust builds

# The final module will be named vexfs.ko
obj-m += vexfs.o

# Name of the C object file compiled from vexfs_module_entry.c
VEXFS_C_OBJ := vexfs_module_entry.o

# Check if we're doing a C-only build or full build
# The Makefile will set VEXFS_C_ONLY=1 for C-only builds
ifndef VEXFS_C_ONLY
    # Full build: Include pre-created combined Rust object
    ifneq (,$(wildcard $(src)/vexfs_rust_combined.o))
        ccflags-y += -DVEXFS_RUST_FFI_ENABLED
        vexfs-objs := $(VEXFS_C_OBJ) vexfs_rust_combined.o
        clean-files += vexfs_rust_combined.o
    else
        $(warning VexFS: vexfs_rust_combined.o not found, building C-only version)
        ccflags-y += -DVEXFS_C_ONLY_BUILD
        vexfs-objs := $(VEXFS_C_OBJ)
    endif
else
    # C-only build: Set compile flag and only use C object
    ccflags-y += -DVEXFS_C_ONLY_BUILD
    vexfs-objs := $(VEXFS_C_OBJ)
    $(info VexFS: Building C-only version without Rust FFI)
endif

# Optional: If there are C header files specific to this module in an 'include' directory
# ccflags-y += -I$(src)/include

# This Kbuild setup expects:
# 1. vexfs_module_entry.c to be present in $(src) (current directory).
#    Kbuild will compile it to vexfs_module_entry.o.
# 2. For full builds: libvexfs.a to be present in $(src).
#    The main Makefile is responsible for building this with cargo
#    and copying it here before invoking the kernel build.
# 3. For C-only builds: VEXFS_C_ONLY=1 environment variable set.

# To ensure the linker finds the Rust standard libraries (core, alloc, etc.)
# and any other crates that libvexfs.a depends on, the RUSTFLAGS or linker
# arguments might need to be adjusted in the main Makefile when cargo build
# is invoked, or by passing appropriate LDFLAGS to the kernel's `make` command.
# However, for a static library `libfoo.a`, the symbols it needs should ideally
# be self-contained or resolved from other libraries provided to the linker.
# The `rust-kernel-rdma` example uses `cargo rustc ... -- -C link-arg=...`
# This is handled by cargo when building libvexfs.a as a staticlib.
# The kernel linker just needs to link libvexfs.a itself.

# No specific clean actions needed here beyond what `make clean` in kernel dir does for .o files.
# The main Makefile handles `cargo clean`.
