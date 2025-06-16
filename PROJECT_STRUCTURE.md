# VexFS Project Structure

## Overview
VexFS is organized as a monorepo containing the kernel module, userspace tools, tests, and documentation for an AI-native semantic filesystem.

## Directory Structure

```
vexfs/
├── kernel_module/          # Linux kernel module implementation
│   ├── core/              # Core filesystem operations
│   ├── semantic/          # Vector and semantic operations
│   ├── include/           # Header files
│   ├── scripts/           # Module-specific scripts
│   ├── tests/             # Kernel module tests
│   └── Makefile          # Kernel module build configuration
│
├── rust/                  # Rust userspace components
│   ├── src/              # Source code
│   │   ├── bin/          # Binary targets (vexfs_fuse, vexctl)
│   │   ├── ffi/          # Foreign Function Interface
│   │   └── lib.rs        # Library root
│   ├── Cargo.toml        # Rust project configuration
│   └── target/           # Build outputs (gitignored)
│
├── tools/                 # Standalone tools
│   ├── mkfs.vexfs        # Filesystem creation tool
│   ├── vexctl            # Control utility
│   └── utilities/        # Helper utilities
│
├── tests/                 # All test files
│   ├── images/           # Test disk images (gitignored)
│   ├── scripts/          # Test scripts
│   │   ├── unit/         # Unit tests
│   │   ├── integration/  # Integration tests
│   │   └── stress/       # Stress tests
│   ├── data/             # Test data files
│   └── results/          # Test results (gitignored)
│
├── vm_testing/            # VM-based testing infrastructure
│   ├── images/           # VM disk images
│   ├── scripts/          # VM automation scripts
│   └── shared/           # Files shared with VM
│
├── benchmarks/            # Performance benchmarks
│   ├── scripts/          # Benchmark scripts
│   └── results/          # Benchmark results (gitignored)
│
├── docs/                  # Documentation
│   ├── api/              # API documentation
│   ├── architecture/     # Architecture docs
│   ├── implementation/   # Implementation details
│   ├── testing/          # Testing documentation
│   └── user/             # User guides
│
├── scripts/               # Development scripts
│   ├── build/            # Build automation
│   ├── testing/          # Test automation
│   └── cleanup/          # Cleanup utilities
│
├── deployment/            # Deployment configurations
│   ├── docker/           # Docker files
│   ├── kubernetes/       # K8s manifests
│   └── systemd/          # Systemd units
│
├── examples/              # Usage examples
│   ├── basic/            # Basic usage
│   └── advanced/         # Advanced features
│
└── .github/              # GitHub configuration
    ├── workflows/        # CI/CD workflows
    └── ISSUE_TEMPLATE/   # Issue templates
```

## Key Files

### Root Directory
- `README.md` - Main project documentation
- `LICENSE` - Project license (MIT)
- `.gitignore` - Git ignore configuration
- `Cargo.toml` - Workspace-level Rust configuration
- `Makefile` - Top-level build orchestration

### Configuration Files
- `.env.example` - Example environment configuration
- `docker-compose.yml` - Docker development setup
- `nginx.conf` - Nginx configuration for services

## Development Workflow

### Building
```bash
# Build everything
make all

# Build kernel module only
make -C kernel_module

# Build Rust components
cargo build --release
```

### Testing
```bash
# Run all tests
make test

# Run specific test suites
./scripts/testing/run_unit_tests.sh
./scripts/testing/run_integration_tests.sh
```

### VM Testing
```bash
# Start VM environment
cd vm_testing
./scripts/start_alpine_vm.sh

# Run tests in VM
./scripts/automated_vm_test_runner.sh
```

## Important Notes

1. **Kernel Module Safety**: Always test kernel modules in a VM first
2. **Build Artifacts**: All build outputs are gitignored
3. **Test Data**: Test images and results are not committed
4. **Documentation**: Keep docs up-to-date with implementation

## Contributing

When adding new features:
1. Place source code in appropriate directories
2. Add tests in `tests/` directory
3. Update documentation in `docs/`
4. Follow existing naming conventions
5. Keep the structure clean and organized