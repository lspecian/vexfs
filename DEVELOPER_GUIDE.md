# VexFS Developer Guide

## Project Overview

VexFS is a three-tier vector database system with filesystem integration. This guide will help you set up your development environment and understand the codebase.

## Development Environment Setup

### Prerequisites

- **Operating System**: Linux (Ubuntu 20.04+ or similar)
- **Rust**: 1.70+ (install via [rustup](https://rustup.rs/))
- **Node.js**: 18+ (for dashboard development)
- **Docker**: 20.10+ (optional, for containerized development)
- **Git**: 2.25+
- **Build Tools**: gcc, make, pkg-config
- **Libraries**: libfuse3-dev, libssl-dev

### Initial Setup

```bash
# Clone the repository
git clone https://github.com/yourusername/vexfs.git
cd vexfs

# Install Rust if not already installed
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Install system dependencies (Ubuntu/Debian)
sudo apt-get update
sudo apt-get install -y \
    build-essential \
    pkg-config \
    libfuse3-dev \
    libssl-dev \
    linux-headers-$(uname -r)

# Install Node.js (using NodeSource repository)
curl -fsSL https://deb.nodesource.com/setup_18.x | sudo -E bash -
sudo apt-get install -y nodejs
```

## Project Structure

```
vexfs/
├── rust/                    # Rust backend code
│   ├── src/
│   │   ├── bin/            # Binary executables
│   │   │   ├── vexfs_fuse.rs          # FUSE filesystem
│   │   │   └── vexfs_unified_server.rs # API server
│   │   ├── dialects/       # API dialect implementations
│   │   ├── fuse_impl.rs    # FUSE operations
│   │   └── lib.rs          # Library exports
│   └── Cargo.toml          # Rust dependencies
├── vexfs-dashboard/         # React web dashboard
│   ├── src/
│   │   ├── components/     # React components
│   │   ├── services/       # API clients
│   │   └── App.tsx         # Main app
│   └── package.json        # Node dependencies
├── kernel_module/           # Linux kernel module (unstable)
│   ├── core/               # Core filesystem code
│   └── Makefile            # Kernel build
├── tools/                   # Utility programs
│   └── mkfs.vexfs.c        # Filesystem formatter
└── docs/                    # Documentation (outdated)
```

## Building Components

### 1. API Server

```bash
cd rust

# Build debug version (faster compilation)
cargo build --features server --bin vexfs_unified_server

# Build release version (optimized)
cargo build --release --features server --bin vexfs_unified_server

# Run directly
cargo run --features server --bin vexfs_unified_server
```

### 2. FUSE Filesystem

```bash
cd rust

# Build FUSE implementation
cargo build --release --features fuse_support

# The binary will be at:
# target/release/vexfs_fuse
```

### 3. Web Dashboard

```bash
cd vexfs-dashboard

# Install dependencies
npm install

# Development server (hot reload)
npm start
# Opens at http://localhost:3000

# Production build
npm run build
# Creates optimized build in dist/
```

### 4. Tools

```bash
cd tools

# Build mkfs.vexfs
make

# Clean and rebuild
make clean && make
```

### 5. Kernel Module (VM Only!)

```bash
cd kernel_module

# Clean previous builds
make clean

# Build module
make

# The module will be at:
# vexfs_deadlock_fix.ko
```

⚠️ **WARNING**: Only load kernel module in a VM! It will crash your system.

## Development Workflow

### Running the Complete System

```bash
# Terminal 1: Start API Server
cd rust
RUST_LOG=debug cargo run --features server --bin vexfs_unified_server

# Terminal 2: Start Dashboard (development)
cd vexfs-dashboard
npm start

# Terminal 3: Mount FUSE (optional)
sudo mkdir -p /mnt/vexfs
./rust/target/release/vexfs_fuse /mnt/vexfs -f -d
```

### Using Docker for Development

```bash
# Build and run everything
docker-compose up --build

# Rebuild after changes
docker-compose build --no-cache
docker-compose up
```

## Code Organization

### Rust Code Structure

#### API Server (`rust/src/bin/vexfs_unified_server.rs`)
- Entry point for the unified server
- Sets up Axum web framework
- Configures routing for all dialects

#### Dialects (`rust/src/dialects/`)
- `mod.rs` - Common traits and engine
- `chromadb.rs` - ChromaDB API implementation
- `qdrant.rs` - Qdrant API implementation
- `native.rs` - Native VexFS API
- `router.rs` - Request routing logic

#### FUSE Implementation (`rust/src/fuse_impl.rs`)
- Implements FUSE filesystem operations
- Contains vector storage backend
- HNSW graph for similarity search

### Dashboard Code Structure

#### Components (`vexfs-dashboard/src/components/`)
- `CollectionManager.tsx` - CRUD for collections
- `DocumentUpload.tsx` - Document management
- `VectorSearch.tsx` - Search interface
- `SystemMetrics.tsx` - Performance monitoring

#### Services (`vexfs-dashboard/src/services/`)
- `api.ts` - API client for backend communication
- Axios-based HTTP client
- Type definitions for API responses

## Testing

### Unit Tests

```bash
# Run all Rust tests
cd rust
cargo test --all-features

# Run specific test
cargo test test_name

# Run with output
cargo test -- --nocapture
```

### Integration Tests

```bash
# Test API endpoints
cd tests
./test_api.sh

# Test FUSE operations
./test_fuse.sh
```

### Dashboard Tests

```bash
cd vexfs-dashboard

# Run tests
npm test

# Run with coverage
npm run test:coverage
```

## Debugging

### Rust Debugging

#### Using println! debugging
```rust
println!("Debug: variable = {:?}", variable);
eprintln!("Error: {}", error_message);
```

#### Using env_logger
```rust
// In your code
use log::{debug, info, warn, error};

debug!("Debug message");
info!("Info message");

// Run with logging
RUST_LOG=debug cargo run
```

#### Using lldb/gdb
```bash
# Build with debug symbols
cargo build

# Debug with lldb
lldb target/debug/vexfs_unified_server
(lldb) breakpoint set --name main
(lldb) run
```

### Dashboard Debugging

#### Browser DevTools
1. Open Chrome/Firefox DevTools (F12)
2. Use Sources tab for breakpoints
3. Check Network tab for API calls
4. Console for error messages

#### React Developer Tools
```bash
# Install browser extension
# Chrome: React Developer Tools
# Firefox: React Developer Tools
```

## Common Development Tasks

### Adding a New API Endpoint

1. **Define the endpoint in dialect** (`rust/src/dialects/native.rs`):
```rust
match (method, path) {
    ("GET", "/vexfs/v1/new-endpoint") => {
        // Implementation
    }
}
```

2. **Update the router** if needed (`rust/src/dialects/router.rs`)

3. **Add to API documentation** (`API_DOCUMENTATION.md`)

### Adding a Dashboard Feature

1. **Create component** (`vexfs-dashboard/src/components/NewFeature.tsx`):
```typescript
import React from 'react';

export const NewFeature: React.FC = () => {
    // Component logic
    return <div>New Feature</div>;
};
```

2. **Add to App.tsx**:
```typescript
import { NewFeature } from './components/NewFeature';
```

3. **Update API client** if needed (`vexfs-dashboard/src/services/api.ts`)

### Fixing FUSE Operations

Current issues with delete/rmdir in `rust/src/fuse_impl.rs`:

```rust
fn unlink(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEmpty) {
    // TODO: Fix implementation
    // Current issue: Returns ENOENT even when file exists
}
```

## Performance Profiling

### Rust Profiling

```bash
# Build with profiling
cargo build --release

# Use perf (Linux)
perf record ./target/release/vexfs_unified_server
perf report

# Use flamegraph
cargo install flamegraph
cargo flamegraph --bin vexfs_unified_server
```

### Dashboard Profiling

```bash
# React Profiler
# Use React DevTools Profiler tab

# Bundle analysis
npm run build
npm run analyze
```

## Git Workflow

### Branch Strategy

```bash
# Create feature branch
git checkout -b feature/new-feature

# Make changes and commit
git add .
git commit -m "feat: Add new feature"

# Push to remote
git push origin feature/new-feature

# Create pull request on GitHub
```

### Commit Message Convention

```
type(scope): subject

body

footer
```

Types:
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation
- `style`: Formatting
- `refactor`: Code restructuring
- `test`: Tests
- `chore`: Maintenance

Example:
```bash
git commit -m "fix(fuse): Resolve delete operation error

The delete operation was returning ENOENT even when the file existed.
This was due to incorrect inode lookup logic.

Fixes #123"
```

## Troubleshooting

### Common Build Issues

#### Rust compilation errors
```bash
# Clear cargo cache
cargo clean

# Update dependencies
cargo update

# Check for breaking changes
cargo check --all-features
```

#### Dashboard build issues
```bash
# Clear npm cache
npm cache clean --force

# Reinstall dependencies
rm -rf node_modules package-lock.json
npm install
```

#### FUSE mount issues
```bash
# Check if FUSE is installed
fusermount3 --version

# Check permissions
ls -la /dev/fuse

# Debug mount
./vexfs_fuse /mnt/vexfs -f -d
```

## Resources

### Documentation
- [Rust Book](https://doc.rust-lang.org/book/)
- [FUSE Documentation](https://libfuse.github.io/doxygen/)
- [React Documentation](https://react.dev/)
- [Axum Web Framework](https://github.com/tokio-rs/axum)

### VexFS Specific
- `CLAUDE.md` - Complete architecture overview
- `API_DOCUMENTATION.md` - API reference
- `ACTUAL_PROJECT_STATUS.md` - Current status
- `DOCKER_DEPLOYMENT.md` - Container deployment

## Contributing

1. Read `ACTUAL_PROJECT_STATUS.md` for current state
2. Check existing issues on GitHub
3. Create feature branch
4. Write tests for new code
5. Ensure all tests pass
6. Submit pull request

## Getting Help

- GitHub Issues: Report bugs and request features
- GitHub Discussions: Ask questions and share ideas
- Documentation: Check docs first for common issues

---

*Developer Guide for VexFS v0.0.4-alpha*