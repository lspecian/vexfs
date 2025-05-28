# VexFS: Vector-Native File System

[![Rust](https://img.shields.io/badge/rust-stable-brightgreen.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-Apache--2.0%20OR%20GPL--2.0-blue.svg)](LICENSE)
[![Build Status](https://img.shields.io/badge/build-passing-brightgreen.svg)](https://github.com/lspecian/vexfs)

**VexFS** is a revolutionary Linux kernel module that implements a POSIX-compliant file system with native vector search capabilities. By integrating vector embeddings directly into the file system layer, VexFS eliminates the impedance mismatch between traditional file storage and vector databases, providing unprecedented performance for AI/ML applications.

## 🎯 **Vision**

VexFS addresses the escalating demand for efficient, integrated storage solutions tailored to AI and Machine Learning applications. Instead of managing separate file systems and vector databases, VexFS provides a unified data substrate where semantic meaning (represented by vectors) is a first-class citizen of the file system itself.

## ✨ **Key Features**

### 🔬 **Vector-Native Integration**
- **Direct Vector Indexing**: Files can have their content automatically processed to generate vector embeddings
- **Hybrid Queries**: Combine traditional file metadata with semantic similarity searches
- **Vector-Aware Operations**: POSIX operations augmented with vector functionality via ioctls
- **Reduced Data Movement**: Co-locate vector embeddings with source data in a single storage system

### ⚡ **Performance & Scalability**
- **Kernel-Level Implementation**: Maximum performance through direct kernel integration
- **ANNS Algorithms**: Advanced Approximate Nearest Neighbor Search (HNSW, IVFADC, DiskANN)
- **Multiple Distance Metrics**: L2 Distance, Cosine Similarity, Inner Product
- **Optimized Storage**: Vector compression, quantization, and columnar layouts

### 🛠 **Developer Experience**
- **POSIX Compliance**: Seamless integration with existing Linux tools and applications
- **Intuitive APIs**: ioctl-based vector operations with planned client libraries
- **Command-Line Tools**: `vexctl` for management, indexing, and querying
- **Comprehensive Testing**: Two-tier development strategy (host + VM)

## 🚀 **Current Status**

### ✅ **What's Working**
- **Zero compilation errors** (resolved from 155 blocking errors)
- **Functional vector operations** (tested via a userspace harness)
- **Working C bindings** for userspace testing
- **Vector test runner** demonstrating end-to-end functionality

### 🔧 **In Development**
- VFS interface layer implementation
- Kernel module integration
- ioctl interface for vector operations
- Full system integration testing

## 📁 **Project Structure**

```
fs/
├── fs/                    # Core kernel module implementation
│   ├── src/                 # Rust source code
│   │   ├── lib.rs          # Main library entry point
│   │   ├── vector_*.rs     # Vector storage and search modules
│   │   ├── anns/           # ANNS algorithm implementations
│   │   └── file_ops.rs     # File system operations
│   ├── Cargo.toml         # Rust dependencies
│   └── Makefile           # Kernel module build system
├── vexctl/                 # Command-line interface tool
├── test_env/              # VM testing environment (Packer + QEMU)
├── docs/                  # Comprehensive documentation
└── scripts/               # Build and development scripts
```

## 🔧 **Quick Start**

### Prerequisites
- Rust (stable toolchain)
- Linux kernel headers
- QEMU (for kernel module testing)
- Standard development tools (make, gcc)

### Host Development (Userspace Testing)
```bash
# Clone the repository
git clone https://github.com/lspecian/vexfs.git
cd vexfs

# Build the project
cargo build

# Run vector operations test
cargo run --bin vector_test_runner

# Expected output: Vector operations with performance metrics
```

### Kernel Module Development
```bash
# Build kernel module (requires VM environment)
cd vexfs
make

# Test in QEMU VM
cd ../test_env
./run_qemu.sh
```

## 🏗 **Architecture**

VexFS implements a layered architecture optimized for both traditional file I/O and vector operations:

```
┌─────────────────────────────────────────────┐
│             VFS Interface Layer             │  ← POSIX compliance
├─────────────────────────────────────────────┤
│          Core File System Logic            │  ← File/directory ops
├─────────────────────────────────────────────┤
│       Vector Indexing & Search Module      │  ← ANNS algorithms
├─────────────────────────────────────────────┤
│         Storage Backend Interface          │  ← Block device abstraction
└─────────────────────────────────────────────┘
```

## 🧪 **Testing Strategy**

### Two-Tier Development Approach
1. **Host Development**: Fast iteration for userspace components and logic validation
2. **VM Testing**: Kernel module integration and full system testing

### Test Coverage
- Unit tests for vector operations
- Integration tests with VFS
- Preliminary performance benchmarks (userspace test harness)
- POSIX compliance validation
- Stress testing and data integrity checks

## Benchmarks

Preliminary benchmarks using a userspace test harness (`vector_test_runner` with its internal `TestVectorSearchEngine`):

### ✅ Functional Test
- 4 vectors added
- Cosine similarity and Euclidean search results returned correct top-K neighbors
- File paths resolved correctly (`/test/vec1.bin`, `/test/vec4.bin`, etc.)

### ⚡ Performance Test
The test harness performs these operations (1000 vector insertions, k-NN search for 3 metrics) in the order of milliseconds on typical desktop hardware. These figures serve as a baseline for the test harness itself.

The userspace test harness, by its nature, operates with low overhead. However, these results do not yet reflect the performance of the main VexFS kernel components or direct comparisons to production vector databases.

> Functional and preliminary performance tests for the userspace test harness completed successfully.

## 🛣 **Roadmap**

### Phase 1: MVP (Current)
- [x] Basic vector storage and search operations
- [x] C bindings for userspace testing
- [x] Core compilation and functionality
- [ ] VFS interface layer implementation
- [ ] Basic ioctl operations

### Phase 2: Integration
- [ ] Full kernel module functionality
- [ ] Complete ioctl API
- [ ] vexctl command-line tool
- [ ] Performance optimizations

### Phase 3: Advanced Features
- [ ] Security and access control
- [ ] Copy-on-Write and snapshots
- [ ] Hybrid query optimization
- [ ] Advanced ANNS algorithms

## 🔗 **Use Cases**

- **Retrieval-Augmented Generation (RAG)** for Large Language Models
- **AI Model Data Storage** and retrieval optimization
- **Semantic Search Engines** with file system integration
- **Multimedia Information Retrieval** (images, audio, video)
- **Anomaly Detection** systems
- **Personalized Recommendation** platforms

## 🤝 **Contributing**

We welcome contributions! Please see our development workflow:

1. Use TaskMaster for project management (`task-master list`)
2. Follow our two-tier development strategy
3. Ensure all tests pass before submitting PRs
4. Follow Rust best practices and kernel development guidelines

### Development Workflow
```bash
# Check current tasks
task-master list

# Get next task to work on
task-master next

# Run tests
cargo test
cargo run --bin vector_test_runner
```

## 📖 **Documentation**

- [Current Project Status](docs/CURRENT_PROJECT_STATUS.md)
- [Development Workflow](docs/DEVELOPMENT_WORKFLOW.md)
- [VM Testing Strategy](test_env/VM_TESTING_STRATEGY.md)
- [Implementation Plan](scripts/IMPLEMENTATION_PLAN.md)
- [Vector Storage Implementation](fs/VECTOR_STORAGE.md)

## 📝 **License**

This project is dual-licensed under:

- **Apache License 2.0** for userspace components (CLI, userland libraries)
- **GNU General Public License v2.0** for kernel module components

### License Details

- **Userspace Components**: Licensed under the [Apache License 2.0](LICENSE)
- **Kernel Module**: Licensed under the [GNU General Public License v2.0](LICENSE.kernel)

The userspace components (CLI tools, userland libraries, and testing infrastructure) are licensed under the Apache License 2.0 to provide maximum flexibility for integration and distribution. The kernel module components are licensed under GPL-2.0 to ensure compatibility with the Linux kernel licensing requirements.

See the respective license files for full terms and conditions:
- [LICENSE](LICENSE) - Apache 2.0 license text with dual license notice
- [LICENSE.kernel](LICENSE.kernel) - GPL-2.0 license text for kernel components

## 🙏 **Acknowledgments**

VexFS builds upon decades of file system research and modern vector database innovations, bringing them together in a novel kernel-level implementation optimized for the AI era.

---

**VexFS**: Where traditional file systems meet the vector age. 🚀