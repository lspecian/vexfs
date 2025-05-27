# VexFS Build System Documentation

## Overview

The VexFS build system is designed for a two-tier development strategy optimized for kernel module development with Rust FFI integration.

## Build Strategy

### Host Development (Fast Iteration)
- **Purpose**: Quick syntax checking and userspace testing
- **Environment**: Host system with standard Rust toolchain
- **Use Case**: Day-to-day development, code validation, unit testing

### VM Testing (Full Validation)
- **Purpose**: Complete kernel module compilation and testing
- **Environment**: Virtual machine with kernel headers
- **Use Case**: Integration testing, final validation before deployment

## Build Targets

### Development Targets (Host)

#### `make syntax-check`
- **Purpose**: Fast Rust syntax validation
- **Output**: Compilation warnings/errors only
- **Duration**: ~1-2 seconds
- **Use Case**: IDE integration, quick feedback during coding

#### `make test-runner`
- **Purpose**: Build and run userspace vector operation tests
- **Output**: Test executable with performance benchmarks
- **Duration**: ~3-5 seconds
- **Use Case**: Validate algorithm implementations

### Production Targets (VM)

#### `make vm-build` (default)
- **Purpose**: Complete kernel module with Rust FFI
- **Components**:
  - Rust static library (libvexfs.a)
  - Combined object file (vexfs_rust_combined.o)
  - Kernel module (vexfs.ko)
- **Duration**: ~10-15 seconds
- **Output Size**: ~2.6MB kernel module

#### `make c-only-build`
- **Purpose**: C-only kernel module (no Rust FFI)
- **Use Case**: Testing kernel module structure, fallback builds
- **Duration**: ~5-8 seconds
- **Output Size**: ~303KB kernel module

#### `make all`
- **Purpose**: Comprehensive build testing
- **Components**: All targets (rust-lib, kernel-module, c-kernel-module, test-runner)
- **Duration**: ~20-25 seconds
- **Use Case**: CI/CD validation, release preparation

### Component Targets

#### `make rust-lib`
- **Purpose**: Build only the Rust static library
- **Output**: libvexfs.a (4.3MB)
- **Features**: Kernel-compatible no_std build
- **Target**: x86_64-unknown-linux-gnu

#### `make kernel-module`
- **Purpose**: Build kernel module from existing Rust library
- **Prerequisites**: libvexfs.a must exist
- **Process**:
  1. Extract 316 object files from static library
  2. Combine into single object with `ld -r`
  3. Strip LLVM metadata sections
  4. Link with C kernel module code

### Maintenance Targets

#### `make clean`
- **Purpose**: Remove all build artifacts
- **Removes**:
  - Cargo build cache
  - Static libraries (*.a)
  - Kernel objects (*.ko, *.o)
  - Module metadata files

#### `make help`
- **Purpose**: Display available targets and usage

## Build Optimization Features

### Size Optimization
- **Release builds**: Optimized for size and performance
- **Metadata stripping**: Removes LLVM-specific sections
- **Dead code elimination**: Rust compiler removes unused code

### Cross-Compilation Support
- **Target**: x86_64-unknown-linux-gnu
- **Kernel compatibility**: Uses kernel headers from host system
- **Feature flags**: `--features=kernel` for no_std builds

### Development Workflow Integration
- **Fast feedback**: syntax-check provides immediate validation
- **Incremental builds**: Cargo caching for repeated builds
- **Parallel processing**: Make and Cargo handle parallelization

## Build Requirements

### Host System
- Rust toolchain (stable)
- Cargo build system
- Basic development tools (make, gcc)

### VM Environment
- Linux kernel headers matching target kernel
- GCC compiler compatible with kernel
- Standard build tools (make, ld, objcopy)

## Performance Characteristics

### Build Times (Approximate)
| Target | Clean Build | Incremental |
|--------|-------------|-------------|
| syntax-check | 1-2s | <1s |
| test-runner | 3-5s | 1-2s |
| rust-lib | 8-12s | 2-4s |
| vm-build | 15-20s | 5-8s |
| c-only-build | 5-8s | 2-3s |

### Output Sizes
| Component | Size | Description |
|-----------|------|-------------|
| libvexfs.a | 4.3MB | Rust static library |
| vexfs.ko (full) | 2.6MB | Kernel module with Rust |
| vexfs.ko (C-only) | 303KB | C-only kernel module |

## Troubleshooting

### Common Issues

#### "No kernel headers found"
- **Cause**: Missing kernel development packages
- **Solution**: Install linux-headers-$(uname -r)

#### "Compiler mismatch warning"
- **Cause**: Different GCC version than kernel build
- **Impact**: Usually harmless, module should work
- **Solution**: Install matching GCC version if needed

#### "BTF generation skipped"
- **Cause**: Missing vmlinux file
- **Impact**: No BTF debug info (usually acceptable)
- **Solution**: Install linux-image-$(uname -r)-dbgsym if needed

#### Large module size
- **Cause**: Debug symbols included
- **Solution**: Verify release build, check strip operations

### Build Verification

#### Quick Validation
```bash
# Test all development targets
make syntax-check
make test-runner

# Test production build
make vm-build
ls -lh vexfs.ko  # Should be ~2.6MB
```

#### Comprehensive Testing
```bash
# Test all targets
make all

# Verify clean builds
make clean && make vm-build
make clean && make c-only-build
```

## Integration with Development Workflow

### Recommended Development Cycle
1. **Code changes**: Edit Rust source files
2. **Quick validation**: `make syntax-check`
3. **Algorithm testing**: `make test-runner`
4. **Integration testing**: `make vm-build` (in VM)
5. **Module testing**: Load and test vexfs.ko

### CI/CD Integration
- **Stage 1**: Host syntax validation (`make syntax-check`)
- **Stage 2**: Userspace testing (`make test-runner`)
- **Stage 3**: VM build validation (`make vm-build`)
- **Stage 4**: Module loading tests (in VM)

This build system provides efficient development iteration while ensuring production builds are properly validated in the target environment.