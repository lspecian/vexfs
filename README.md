# VexFS - Vector-Enhanced Filesystem

## ⚠️ HIGHLY EXPERIMENTAL - ALPHA SOFTWARE

**⚠️ WARNING: This is extremely experimental alpha software. Expect crashes, data loss, and system instability.**

VexFS is an experimental research project exploring the integration of vector search capabilities directly into a Linux filesystem. This is pre-alpha educational software with significant stability issues.

### Version Status
- **Current Version**: v0.0.4-alpha
- **Status**: Early experimental prototype
- **Stability**: Unstable - crashes expected
- **Data Safety**: Not safe - can cause data corruption

## Project Components

### 1. Kernel Module (`kernel_module/`)
**Status**: 🔴 Unstable - Development Only
- Linux kernel module implementation
- Currently has critical bugs (NULL pointer dereferences)
- Requires VM testing for safety
- See [`ACTUAL_PROJECT_STATUS.md`](ACTUAL_PROJECT_STATUS.md) for details

### 2. FUSE Implementation (`rust/`)
**Status**: 🟡 Partially Working
- Userspace filesystem via FUSE
- Basic filesystem operations work
- Vector features not yet implemented
- Builds with warnings

### 3. Tools
- `mkfs.vexfs` - Filesystem formatter ✅ Working
- `vexctl` - Control utility (status unknown)

## Building

### Prerequisites
- Linux kernel headers (for kernel module)
- Rust toolchain (for FUSE)
- FUSE development libraries

### Kernel Module
```bash
cd kernel_module
make clean && make
# WARNING: Do not load on host system - use VM only!
```

### FUSE Implementation
```bash
cd rust
cargo build --release --features fuse_support
```

## Current Limitations

1. **No Vector Search**: Despite the name, vector search is not implemented
2. **No API Compatibility**: Qdrant/ChromaDB APIs not available
3. **No Performance Benchmarks**: All performance claims in docs are unverified
4. **Stability Issues**: Kernel module crashes, FUSE has not been thoroughly tested

## Documentation Status

⚠️ **Warning**: The `docs/` folder contains 300+ files with outdated, conflicting, or aspirational information. For accurate project status, see:
- [`ACTUAL_PROJECT_STATUS.md`](ACTUAL_PROJECT_STATUS.md) - Current reality
- [`kernel_module/README.md`](kernel_module/README.md) - Kernel module specifics

## Safety Warning

**DO NOT** load the kernel module on your host system. It can cause:
- System crashes
- Kernel panics
- Data corruption
- Stuck modules requiring reboot

Always use a VM for kernel module testing.

## Contributing

Before contributing, please read [`ACTUAL_PROJECT_STATUS.md`](ACTUAL_PROJECT_STATUS.md) to understand the current state. Priority areas:
1. Fixing kernel module stability
2. Implementing basic vector operations
3. Creating real benchmarks
4. Cleaning up documentation

## License

MIT License - See LICENSE file

---

*This README reflects the honest state of the project. Previous READMEs contained aspirational claims that have been removed for accuracy.*