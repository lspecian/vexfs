Sorry - couldn't find any contact info. I'm currently looking for a solution to deploy a remote POSIX filesystem scalably for an AI agent. Curious if you've tried using this solution for multiple end users - does it require one docker instance per user?

# VexFS - AI-Native Semantic Filesystem

## Complete Vector Database System with Multi-Interface Access

VexFS is a sophisticated three-tier vector database system that provides filesystem, REST API, and web UI access to semantic search capabilities. While still in alpha, it includes a functional unified API server compatible with ChromaDB/Qdrant and a comprehensive React dashboard.

### Version Status
- **Current Version**: v0.0.4-alpha
- **Architecture**: Three-tier (FUSE + API Server + Dashboard)
- **API Compatibility**: ChromaDB, Qdrant, Native VexFS
- **Status**: Functional but not production-ready

## System Architecture

```
┌─────────────────────────────────────┐
│     Web Dashboard (React)           │
│     http://localhost:3000           │
└────────────┬────────────────────────┘
             │ REST API
             ▼
┌─────────────────────────────────────┐
│   Unified API Server (Rust)        │
│   http://localhost:7680             │
│   • ChromaDB API (/api/v1/*)       │
│   • Qdrant API (/collections/*)    │
│   • Native API (/vexfs/v1/*)       │
└────────────┬────────────────────────┘
             │ Shared Backend
             ▼
┌─────────────────────────────────────┐
│    FUSE Filesystem (Optional)       │
│    Mount: /mnt/vexfs                │
└─────────────────────────────────────┘
```

## Project Components

### 1. Unified API Server ✅ Working
- **Location**: `rust/src/bin/vexfs_unified_server.rs`
- **Port**: 7680 (default)
- **Features**:
  - ChromaDB-compatible API
  - Qdrant-compatible API  
  - Native VexFS API
  - Health checks and metrics

### 2. Web Dashboard ✅ Working
- **Location**: `vexfs-dashboard/`
- **Stack**: React 18, TypeScript, Material-UI
- **Features**:
  - Collection management
  - Document upload
  - Vector search interface
  - System metrics
  - File browser

### 3. FUSE Filesystem 🟡 Partially Working
- **Location**: `rust/src/bin/vexfs_fuse.rs`
- **Status**: Basic operations work (except delete/rmdir)
- **Backend**: Complete vector infrastructure (HNSW, storage)
- **Issue**: Vector features not exposed via filesystem

### 4. Kernel Module 🔴 Unstable
- **Location**: `kernel_module/`
- **Status**: Critical bugs (NULL pointer dereferences)
- **Warning**: VM testing only!

### 5. Tools ✅ Working
- `mkfs.vexfs` - Filesystem formatter
- `vexctl` - Control utility

## Quick Start

### Docker Deployment (Recommended)
```bash
# Build and run complete system
docker-compose up --build

# Access components:
# - API Server: http://localhost:7680
# - Dashboard: http://localhost:3000
# - Health Check: http://localhost:7680/health
```

### Manual Building

#### Prerequisites
- Rust 1.70+
- Node.js 18+
- Docker 20.10+ (for containerized deployment)
- Linux kernel headers (for kernel module)
- FUSE 3 development libraries

#### Build Commands
```bash
# Unified API Server
cd rust
cargo build --release --features server --bin vexfs_unified_server

# Web Dashboard
cd vexfs-dashboard
npm install
npm run build

# FUSE Filesystem (optional)
cd rust
cargo build --release --features fuse_support

# Kernel Module (VM only!)
cd kernel_module
make clean && make
```

## API Usage Examples

### ChromaDB Client
```python
import chromadb

client = chromadb.HttpClient(host="localhost", port=7680)
collection = client.create_collection("my_docs")

collection.add(
    documents=["Doc 1", "Doc 2"],
    embeddings=[[0.1, 0.2], [0.3, 0.4]],
    ids=["id1", "id2"]
)

results = collection.query(
    query_embeddings=[[0.1, 0.2]],
    n_results=5
)
```

### Qdrant Client
```python
from qdrant_client import QdrantClient

client = QdrantClient(host="localhost", port=7680)

client.recreate_collection(
    collection_name="my_docs",
    vectors_config={"size": 384, "distance": "Cosine"}
)
```

### Web Dashboard
Access http://localhost:3000 for the full UI with collection management, document upload, and vector search.

## Current Limitations & Roadmap

### Working Features ✅
- Multi-dialect API server (ChromaDB/Qdrant compatible)
- Web dashboard for management
- Basic FUSE filesystem operations
- Vector backend with HNSW indexing

### Known Issues 🔧
1. **FUSE**: Delete/rmdir operations fail
2. **Kernel Module**: NULL pointer crashes (use VM only)
3. **Integration**: Vector features not exposed via filesystem
4. **Security**: No authentication/authorization
5. **Performance**: 361K ops/sec claim unverified

### Roadmap 🚀
- [ ] Fix FUSE deletion operations
- [ ] Add filesystem vector operations via xattr
- [ ] Implement authentication for API
- [ ] Create comprehensive benchmarks
- [ ] Stabilize kernel module
- [ ] Add Pinecone/Weaviate API compatibility

## Documentation

### Accurate Documentation
- [`CLAUDE.md`](CLAUDE.md) - Complete architectural overview
- [`ACTUAL_PROJECT_STATUS.md`](ACTUAL_PROJECT_STATUS.md) - Current project reality
- [`TEST_RESULTS_2025_08_16.md`](TEST_RESULTS_2025_08_16.md) - Latest test results
- [`API_DOCUMENTATION.md`](API_DOCUMENTATION.md) - API reference (to be created)

⚠️ **Note**: The `docs/` folder contains outdated information. Trust only the files listed above.

## Development Setup

### Running the System
```bash
# Start API Server
VEXFS_PORT=7680 ./target/release/vexfs_unified_server

# Start Dashboard (development)
cd vexfs-dashboard && npm start

# Mount FUSE (optional)
sudo mkdir -p /mnt/vexfs
./target/release/vexfs_fuse /mnt/vexfs -f
```

### Testing
```bash
# API Health Check
curl http://localhost:7680/health

# List Collections
curl http://localhost:7680/api/v1/collections  # ChromaDB style
curl http://localhost:7680/collections         # Qdrant style
```

## Safety Warning

⚠️ **Kernel Module**: DO NOT load on host system! Use VM only. Can cause:
- System crashes and kernel panics
- Data corruption
- Stuck modules requiring reboot

## Contributing

Contributions welcome! Priority areas:

1. **Bug Fixes**:
   - FUSE delete/rmdir operations
   - Kernel module stability

2. **Features**:
   - Authentication/authorization
   - Filesystem vector operations
   - Additional API compatibility (Pinecone, Weaviate)

3. **Testing**:
   - Integration tests
   - Performance benchmarks
   - Security audit

See [`CONTRIBUTING.md`](CONTRIBUTING.md) for guidelines.

## Performance

- **Claimed**: 361,000+ ops/sec (unverified)
- **Expected**: 10-50K API ops/sec, 1-10K FUSE ops/sec
- **Optimization**: HNSW indexing, memory caching, async I/O

## License

MIT License - See LICENSE file

## Support

- **Issues**: [GitHub Issues](https://github.com/yourusername/vexfs/issues)
- **Discussions**: [GitHub Discussions](https://github.com/yourusername/vexfs/discussions)
- **Documentation**: See [`CLAUDE.md`](CLAUDE.md) for detailed architecture

---

*VexFS is an experimental project. Use at your own risk. Not suitable for production.*
