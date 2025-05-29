# VexFS Repository Structure and Organization

## Overview

This document describes the organized structure of the VexFS repository after the cleanup and reorganization performed in subtask 30.1. The goal is to provide clear separation between C kernel code and Rust library code, with consistent error handling and build system validation.

## Directory Structure

```
vexfs/
├── kernel/                     # C kernel module code
│   ├── src/                   # Kernel module source files
│   │   ├── vexfs_module_entry.c      # Main kernel module (with FFI)
│   │   └── vexfs_module_entry_safe.c # Safe kernel module (no FFI)
│   ├── include/               # Kernel module headers
│   │   └── vexfs_ffi.h       # FFI interface header
│   ├── tests/                 # C FFI tests
│   │   ├── test_ffi.c
│   │   └── test_ffi_integration.c
│   └── build/                 # Kernel build configuration
│       ├── Kbuild            # Kernel build configuration
│       ├── Makefile          # Main kernel build system
│       ├── Makefile.safe     # Safe build system
│       └── Makefile.ffi      # FFI-specific build rules
├── rust/                      # Rust library code
│   ├── src/                   # Rust source files
│   │   ├── lib.rs            # Main library entry point
│   │   ├── ffi/              # FFI implementation
│   │   │   ├── mod.rs        # FFI module
│   │   │   ├── kernel.rs     # Kernel FFI functions
│   │   │   └── userspace.rs  # Userspace FFI functions
│   │   ├── shared/           # Shared domain components
│   │   ├── storage/          # Storage domain
│   │   ├── fs_core/          # Filesystem core domain
│   │   └── security/         # Security domain
│   ├── Cargo.toml            # Rust project configuration
│   └── build.rs              # Rust build script
├── build/                     # Build system and configuration
│   ├── scripts/              # Build scripts
│   ├── config/               # Build configuration
│   └── validation/           # FFI compatibility validation
├── docs/                      # Documentation
├── test_env/                  # Testing environment
├── examples/                  # Usage examples
└── bindings/                  # Language bindings
```

## Code Organization Principles

### 1. Clear Separation of Concerns

- **Kernel Code (`kernel/`)**: All C code related to the Linux kernel module
- **Rust Library (`rust/`)**: All Rust code implementing VexFS functionality
- **Build System (`build/`)**: Centralized build configuration and validation
- **Documentation (`docs/`)**: All project documentation

### 2. Consistent Error Handling

- **Kernel Module**: Uses Linux kernel error codes (-ENOMEM, -EINVAL, etc.)
- **Rust Library**: Uses VexfsError enum with proper error propagation
- **FFI Bridge**: Converts between Rust errors and C error codes consistently

### 3. Module Boundaries

- **FFI Interface**: Clean boundary between C and Rust code
- **Domain Architecture**: Clear separation between storage, filesystem, and security domains
- **Feature Flags**: Conditional compilation for kernel vs userspace

## Build System Architecture

### 1. Multi-Target Build Support

- **Host Development**: Syntax checking and userspace testing
- **VM Testing**: Full kernel module compilation
- **Safe Mode**: C-only builds for testing without FFI

### 2. FFI Compatibility Validation

- **Header Generation**: Automatic C header generation from Rust
- **Symbol Validation**: Verification of FFI function signatures
- **ABI Compatibility**: Ensures stable interface between C and Rust

### 3. Error Handling Validation

- **Error Code Mapping**: Validates error code consistency
- **Return Value Checking**: Ensures proper error propagation
- **Safety Checks**: Validates null pointer handling

## Safety Protocols

### 1. Development Safety

- **Safe Builds**: C-only builds that prevent system hangs
- **VM-Only Testing**: Kernel modules only tested in virtual machines
- **Incremental Development**: Step-by-step FFI implementation

### 2. Code Quality

- **Dead Code Removal**: Eliminated commented-out and unused code
- **Consistent Formatting**: Applied consistent code style
- **Documentation**: Comprehensive inline and external documentation

## Next Steps

This reorganization prepares the repository for:

1. **FFI Bridge Implementation** (subtask 30.2)
2. **Kernel Module Testing** (subtask 30.3)
3. **Production Deployment** (future tasks)

## Migration Notes

- Old file locations are documented for reference
- Build system maintains backward compatibility
- All existing functionality is preserved
- New structure enables safer development workflow