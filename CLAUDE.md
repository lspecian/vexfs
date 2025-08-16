# VexFS Project Overview for Claude

## Executive Summary

VexFS (Vector Extended File System) is a sophisticated AI-native filesystem that integrates vector search and semantic capabilities at multiple layers. The project provides a complete ecosystem including a FUSE filesystem, multi-dialect API server, and React dashboard.

## Project Status

### Current Reality
- **Stage**: Alpha (v0.0.4-alpha) with functional components
- **Architecture**: Three-tier system (FUSE + API Server + Web Dashboard)
- **Vector Features**: Fully implemented in backend, partial filesystem exposure
- **API Compatibility**: ChromaDB, Qdrant, and Native VexFS APIs

### Key Components Status

1. **FUSE Filesystem** (🟡 Partially Working)
   - Location: `rust/src/bin/vexfs_fuse.rs`
   - Has complete vector backend (HNSW graph, vector storage)
   - Basic filesystem operations work
   - Vector features not exposed through filesystem interface
   
2. **Unified API Server** (✅ Working)
   - Location: `rust/src/bin/vexfs_unified_server.rs`
   - Port: 7680 (default)
   - Provides three API dialects:
     - ChromaDB API (`/api/v1/*`)
     - Qdrant API (`/collections/*`)
     - Native VexFS API (`/vexfs/v1/*`)
   - Full vector database functionality
   
3. **Web Dashboard** (✅ Working)
   - Location: `vexfs-dashboard/`
   - React 18 + Material-UI
   - Features:
     - Collection management
     - Document upload/management
     - Vector search interface
     - System metrics visualization
     - File browser with semantic search
   
4. **Kernel Module** (🔴 Unstable)
   - Location: `kernel_module/`
   - Issues: NULL pointer dereferences, system crashes
   - Module name: `vexfs_deadlock_fix.ko`
   
5. **Tools** (✅ Working)
   - `mkfs.vexfs`: Filesystem formatter
   - `vexctl`: Control utility

## Complete System Architecture

### Three-Tier Architecture

```
┌─────────────────────────────────────┐
│     Web Dashboard (React)           │
│  http://localhost:3000              │
│  - Collection Management            │
│  - Vector Search UI                 │
│  - System Metrics                   │
└────────────┬────────────────────────┘
             │ HTTP/REST
             ▼
┌─────────────────────────────────────┐
│   Unified API Server (Rust)        │
│   http://localhost:7680             │
│  ┌─────────────────────────────┐   │
│  │ ChromaDB API (/api/v1/*)    │   │
│  ├─────────────────────────────┤   │
│  │ Qdrant API (/collections/*) │   │
│  ├─────────────────────────────┤   │
│  │ Native API (/vexfs/v1/*)    │   │
│  └─────────────────────────────┘   │
│         VexFS Engine                │
└────────────┬────────────────────────┘
             │ Shared Backend
             ▼
┌─────────────────────────────────────┐
│    FUSE Filesystem (Rust)          │
│    Mount: /mnt/vexfs                │
│  - Vector Storage Manager           │
│  - HNSW Graph Implementation        │
│  - File Operations                  │
└─────────────────────────────────────┘
```

### Design Goals
- **Multi-Interface Access**: Same data accessible via filesystem, REST APIs, and web UI
- **Vector Database Compatibility**: Drop-in replacement for ChromaDB/Qdrant
- **High Performance**: Target 361,000+ ops/sec (claimed)
- **HNSW Indexing**: Efficient similarity search
- **Unified Backend**: Single engine serving multiple API dialects

### Technical Stack
- **Backend**: Rust (FUSE, API server, vector engine)
- **Frontend**: React 18, TypeScript, Material-UI
- **Vector Search**: HNSW graph, cosine/euclidean distance
- **APIs**: REST/JSON compatible with ChromaDB and Qdrant
- **Infrastructure**: Docker support, health checks, metrics
- **Languages**: Rust (core), TypeScript (dashboard), C (kernel module)

## Code Structure

### Core Components

#### 1. FUSE Implementation (`rust/src/`)
```
rust/src/
├── bin/
│   ├── vexfs_fuse.rs         # FUSE filesystem binary
│   └── vexfs_unified_server.rs # Multi-dialect API server
├── dialects/                  # API dialect implementations
│   ├── mod.rs                # Common engine and traits
│   ├── chromadb.rs           # ChromaDB compatibility
│   ├── qdrant.rs             # Qdrant compatibility
│   ├── native.rs             # Native VexFS API
│   └── router.rs             # Request routing
├── fuse_impl.rs              # FUSE operations
├── vector_storage.rs         # Vector storage backend
├── hnsw_graph.rs            # HNSW implementation
└── shared/                   # Shared types and utilities
```

#### 2. Web Dashboard (`vexfs-dashboard/`)
```
vexfs-dashboard/
├── src/
│   ├── App.tsx               # Main application
│   ├── components/           # React components
│   │   ├── CollectionManager.tsx
│   │   ├── DocumentUpload.tsx
│   │   ├── VectorSearch.tsx
│   │   ├── SystemMetrics.tsx
│   │   └── FileBrowser.tsx
│   ├── services/
│   │   └── api.ts           # API client (port 7680)
│   └── hooks/               # Custom React hooks
└── package.json             # Dependencies
```

#### 3. Kernel Module (`kernel_module/`)
```
kernel_module/
├── core/              # Core filesystem implementation
│   ├── main.c        # Module entry/exit
│   ├── superblock.c  # Superblock operations
│   ├── inode.c       # Inode operations
│   └── file.c        # File operations
└── semantic/         # Vector operations (incomplete)
    └── vector_ops.c  # IOCTL-based vector operations
```

### Key Data Structures

#### FUSE Backend
```rust
pub struct VexFSFuse {
    vector_storage: Arc<OptimizedVectorStorageManager>,
    hnsw_graph: Arc<Mutex<OptimizedHnswGraph>>,
    storage_hnsw_bridge: Arc<Mutex<StorageHnswBridge>>,
    memory_cache: Arc<MemoryCache>,
}
```

#### API Engine
```rust
pub struct VexFSEngine {
    collections: Arc<Mutex<HashMap<String, Collection>>>,
}

pub struct Collection {
    name: String,
    metadata: CollectionMetadata,
    documents: HashMap<String, Document>,
}
```

#### Dashboard API Client
```typescript
interface Collection {
  name: string;
  metadata: CollectionMetadata;
  document_count: number;
  vector_dimension?: number;
}

interface SearchResult {
  id: string;
  score: number;
  metadata?: Record<string, any>;
  content?: string;
}
```

## Development Guidelines

### Building

#### Complete System (Docker)
```bash
# Build and run all components
docker-compose up --build

# Builds:
# - Unified server on port 7680
# - Dashboard on port 3000 (if exposed)
# - Includes health checks and auto-restart
```

#### Individual Components
```bash
# FUSE implementation
cd rust && cargo build --release --features fuse_support

# Unified API server
cd rust && cargo build --release --features server --bin vexfs_unified_server

# Dashboard
cd vexfs-dashboard && npm install && npm run build

# Kernel module (VM only!)
cd kernel_module && make

# Tools
cd tools && make
```

### Deployment Options

#### 1. Docker (Recommended)
```bash
# Production deployment
docker run -d \
  -p 7680:7680 \
  -v vexfs_data:/app/data \
  --name vexfs \
  vexfs:latest
```

#### 2. Standalone Services
```bash
# Start API server
VEXFS_PORT=7680 ./vexfs_unified_server

# Start dashboard (development)
cd vexfs-dashboard && npm start

# Mount FUSE filesystem
./vexfs_fuse /mnt/vexfs -f
```

### Testing Safety
⚠️ **CRITICAL**: Kernel module requires VM testing
- Can cause kernel panics
- May corrupt data
- Module can get stuck requiring reboot

### Current Issues
1. **Kernel module**: Crashes with NULL pointer dereferences
2. **FUSE**: File deletion and rmdir operations fail
3. **Documentation**: Outdated, doesn't reflect actual implementation
4. **Performance**: 361K ops/sec claim unverified
5. **Integration**: Vector features not exposed through filesystem interface

## Key Files to Understand

### Core Implementation
1. `/rust/src/bin/vexfs_unified_server.rs` - Multi-dialect API server
2. `/rust/src/dialects/mod.rs` - Shared vector engine
3. `/rust/src/fuse_impl.rs` - FUSE filesystem operations
4. `/vexfs-dashboard/src/App.tsx` - Dashboard main component
5. `/vexfs-dashboard/src/services/api.ts` - API client

### Configuration
1. `/Dockerfile` - Production container setup
2. `/docker-compose.yml` - Full system deployment
3. `/docker-entrypoint.sh` - Container initialization

### Documentation
1. `/ACTUAL_PROJECT_STATUS.md` - Current project reality
2. `/TEST_RESULTS_2025_08_16.md` - Latest test results

## API Usage Examples

### ChromaDB Compatible API
```python
import chromadb

# Connect to VexFS instead of ChromaDB
client = chromadb.HttpClient(host="localhost", port=7680)
collection = client.create_collection("my_collection")

# Add documents with embeddings
collection.add(
    documents=["Document 1", "Document 2"],
    embeddings=[[0.1, 0.2, ...], [0.3, 0.4, ...]],
    ids=["id1", "id2"]
)

# Search
results = collection.query(
    query_embeddings=[[0.1, 0.2, ...]],
    n_results=5
)
```

### Qdrant Compatible API
```python
from qdrant_client import QdrantClient

# Connect to VexFS instead of Qdrant
client = QdrantClient(host="localhost", port=7680)

# Create collection
client.recreate_collection(
    collection_name="my_collection",
    vectors_config={"size": 384, "distance": "Cosine"}
)

# Search
client.search(
    collection_name="my_collection",
    query_vector=[0.1, 0.2, ...],
    limit=5
)
```

### Dashboard Access
```bash
# Access web UI
open http://localhost:3000

# Features available:
# - Create/delete collections
# - Upload documents
# - Perform vector searches
# - View system metrics
# - Browse files with semantic search
```

## Common Tasks

### Start Complete System
```bash
# Using Docker Compose
docker-compose up -d

# Access dashboard
open http://localhost:3000

# Check API health
curl http://localhost:7680/health
```

### Mount FUSE Filesystem
```bash
# Create mount point
sudo mkdir -p /mnt/vexfs

# Mount with debug output
./rust/target/release/vexfs_fuse /mnt/vexfs -f -d

# Verify mount
df -h | grep vexfs
```

### API Operations
```bash
# List collections (ChromaDB style)
curl http://localhost:7680/api/v1/collections

# List collections (Qdrant style)
curl http://localhost:7680/collections

# List collections (Native VexFS)
curl http://localhost:7680/vexfs/v1/collections

# Server info
curl http://localhost:7680/
```

## Architecture Decisions

1. **Three-Tier Architecture**: Separation of concerns (Storage/API/UI)
2. **Multi-Dialect API**: Single backend serving ChromaDB/Qdrant/Native APIs
3. **Rust Core**: Memory safety and performance for backend
4. **React Dashboard**: Modern web UI for management
5. **HNSW Algorithm**: Implemented for efficient similarity search
6. **Docker Deployment**: Containerized for easy deployment
7. **Unified Server**: Single binary serving both API and static dashboard files

## Security Considerations

- No authentication/authorization implemented
- Vector data stored unencrypted
- IOCTL interface needs security review
- Potential for kernel exploits in current state

## Performance

### Claimed Performance
- **Target**: 361,000+ operations/second
- **Source**: Server startup logs and documentation
- **Status**: Unverified, no benchmarks available

### Expected Reality
- **FUSE Operations**: 1-10K ops/sec (typical FUSE overhead)
- **API Operations**: 10-50K ops/sec (depending on hardware)
- **Vector Search**: Depends on collection size and HNSW parameters
- **Dashboard**: Standard React app performance

### Optimization Features
- Memory caching in FUSE implementation
- Optimized HNSW graph traversal
- Async I/O in API server
- Connection pooling planned

## Contributing Guidelines

1. Test all kernel changes in VM first
2. Focus on stability over features
3. Document actual behavior, not aspirations
4. Add comprehensive error handling
5. Write tests for new functionality

## System Requirements

### Minimum Requirements
- Linux kernel 5.4+ (for kernel module)
- Rust 1.70+ (for building)
- Node.js 18+ (for dashboard)
- 2GB RAM
- 10GB disk space

### Docker Requirements
- Docker 20.10+
- Docker Compose 2.0+
- 4GB RAM allocated to Docker

## Resources

### Libraries Used
- **FUSE**: rust-fuse for filesystem operations
- **Web Framework**: Axum for API server
- **UI Framework**: React 18 + Material-UI
- **Vector Search**: Custom HNSW implementation
- **Serialization**: Serde for JSON handling

### Compatibility
- **ChromaDB Clients**: Any ChromaDB HTTP client
- **Qdrant Clients**: Any Qdrant HTTP client  
- **Languages**: Python, JavaScript, Go, Rust clients supported

---

*This document provides a complete architectural overview of VexFS based on deep source code analysis. The project is more complete than documentation suggests, with a functional three-tier architecture including FUSE filesystem, multi-dialect API server, and web dashboard.*