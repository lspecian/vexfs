# VexFS Kernel Module

## Overview
This is the Linux kernel module implementation of VexFS - an AI-native semantic filesystem.

## Directory Structure

```
kernel_module/
├── core/              # Core filesystem implementation
│   ├── main.c        # Module entry/exit and filesystem registration
│   ├── superblock.c  # Superblock operations
│   ├── block.c       # Block allocation and management
│   ├── inode.c       # Inode operations
│   ├── dir.c         # Directory operations
│   ├── dir_fix.c     # Directory operation fixes
│   ├── file.c        # File operations
│   └── file_enhanced.c # Enhanced file operations
│
├── semantic/          # Semantic/vector operations
│   ├── vector_ops.c  # Vector operations implementation
│   ├── search.c      # Search functionality (future)
│   └── embedding.c   # Embedding operations (future)
│
├── include/          # Header files
│   ├── vexfs_core.h  # Core filesystem structures
│   ├── vexfs_semantic.h # Semantic operations
│   └── vexfs_block.h # Block management
│
├── tests/            # Test files
├── scripts/          # Build and test scripts
├── docs/             # Module-specific documentation
└── archive/          # Old versions and patches

## Building

```bash
# Build the module
make

# Clean build artifacts
make clean

# Build with semantic features (future)
make semantic
```

## Files

### Active Files
- `Kbuild` - Kernel build configuration (active)
- `Makefile` - Alternative make configuration
- Source files in `core/` and `semantic/` directories

### Build Output
- `vexfs_deadlock_fix.ko` - The compiled kernel module

## Testing

Always test in a VM first! See `../vm_testing/` for VM test infrastructure.

```bash
# Load module
sudo insmod vexfs_deadlock_fix.ko

# Check if loaded
lsmod | grep vexfs

# Unload module
sudo rmmod vexfs_deadlock_fix
```

## Current Status
- Phase 1: Core filesystem implementation (active)
- Fixed: NULL pointer dereference in sync_fs
- Fixed: Buffer synchronization in put_super
- Requires: Filesystem formatting with mkfs.vexfs before mount