# VexFS Clean Project Structure

After the comprehensive codebase cleanup, VexFS now has a clean, focused architecture:

## Core Architecture

### Single Unified Server + Dashboard
- **Unified Server**: `src/bin/vexfs_unified_server.rs` - Single server supporting ChromaDB, Qdrant, and Native APIs
- **Dashboard**: `vexfs-dashboard/` - React/TypeScript dashboard served from `/ui/*`
- **Single Docker Config**: `Dockerfile` + `docker-compose.yml` for complete deployment

### Consolidated Rust Workspace
- **Main Library**: `src/lib.rs` - Core VexFS functionality
- **Binaries**: `src/bin/` - All CLI tools and servers
  - `vexfs_unified_server.rs` - Multi-dialect API server
  - `vexctl.rs` - CLI management tool (requires `--features vexctl`)
  - `vexfs_fuse.rs` - FUSE implementation (requires `--features fuse_support`)
  - `mkfs_vexfs.rs` - Filesystem creation tool
  - Test runners and benchmarks

### Domain-Driven Design Structure
- **`src/shared/`** - Foundational components (errors, types, constants)
- **`src/storage/`** - Block management, allocation, journaling
- **`src/fs_core/`** - File and directory operations
- **`src/security/`** - Encryption, ACL, capabilities
- **`src/ffi/`** - C integration for kernel module

### Language Bindings
- **`bindings/python/`** - Python client library
- **`bindings/typescript/`** - TypeScript/JavaScript client library

## Removed Complexity

### Eliminated Directories
- ❌ `core/` - Documentation moved to `docs/architecture/`
- ❌ `search/` - Documentation moved to `docs/architecture/`
- ❌ `vexctl/` - Integrated into main Rust workspace
- ❌ `rust/` - Moved to project root as main workspace

### Eliminated Files
- ❌ `Dockerfile.optimized`, `Dockerfile.server`, `Dockerfile.vexfs-server` - Consolidated to single `Dockerfile`
- ❌ `Makefile.unified` - Removed duplicate build system
- ❌ `vexfs_server.rs`, `vexfs_server_enhanced.rs` - Replaced by `vexfs_unified_server.rs`

## Development Workflow

### Building
```bash
# Build main library
cargo build

# Build unified server
cargo build --bin vexfs_unified_server --features server

# Build CLI tool
cargo build --bin vexctl --features vexctl

# Build FUSE implementation
cargo build --bin vexfs_fuse --features fuse_support
```

### Docker Deployment
```bash
# Build and run complete stack
docker-compose up --build

# Access dashboard at http://localhost:7680/ui/
# API available at http://localhost:7680/api/v1/ (ChromaDB)
# API available at http://localhost:7680/collections/ (Qdrant)
# API available at http://localhost:7680/vexfs/v1/ (Native)
```

### Testing
```bash
# Run comprehensive tests
cargo test

# Run specific test runners
cargo run --bin comprehensive_test_runner
cargo run --bin vector_test_runner
cargo run --bin anns_benchmark_test
```

## Key Benefits

1. **Single Port Architecture**: Eliminates CORS issues by serving everything on port 7680
2. **Unified Server**: One server binary supports all API dialects
3. **Clean Dependencies**: Feature-gated dependencies prevent bloat
4. **Clear Separation**: Domain-driven design with clear boundaries
5. **Simplified Deployment**: Single Docker configuration
6. **Maintainable Structure**: Logical organization and documentation

## Configuration

### Environment Variables
- `VEXFS_PORT` - Server port (default: 7680)
- `DASHBOARD_PATH` - Dashboard files location (default: `/app/dashboard`)

### Features
- `std` - Standard library support (default)
- `server` - Multi-dialect API server
- `vexctl` - CLI management tool
- `fuse_support` - FUSE userspace implementation
- `c_bindings` - C FFI for kernel module

This clean architecture provides a solid foundation for VexFS development and deployment.