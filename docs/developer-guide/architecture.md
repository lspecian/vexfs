# VexFS v2.0 Architecture Overview

This document provides a comprehensive overview of VexFS v2.0's architecture, design principles, and implementation details for developers and contributors.

## 🏗️ High-Level Architecture

VexFS v2.0 implements a **dual-architecture approach** with both kernel-level and userspace implementations:

```
┌─────────────────────────────────────────────────────────────┐
│                    VexFS v2.0 Architecture                 │
├─────────────────────────────────────────────────────────────┤
│  Applications (Python, TypeScript, CLI, Direct FS Access)  │
├─────────────────────────────────────────────────────────────┤
│                    API Layer                               │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────┐ │
│  │   Python SDK    │  │ TypeScript SDK  │  │   vexctl    │ │
│  └─────────────────┘  └─────────────────┘  └─────────────┘ │
├─────────────────────────────────────────────────────────────┤
│                 VexFS Core Layer                           │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │              Kernel Module (Production)                │ │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────┐ │ │
│  │  │ Filesystem  │  │   Vector    │  │      ANNS       │ │ │
│  │  │   Layer     │  │   Engine    │  │   Algorithms    │ │ │
│  │  └─────────────┘  └─────────────┘  └─────────────────┘ │ │
│  └─────────────────────────────────────────────────────────┘ │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │            FUSE Implementation (Development)           │ │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────┐ │ │
│  │  │   Userspace │  │   Vector    │  │   Development   │ │ │
│  │  │ Filesystem  │  │   Engine    │  │    Testing      │ │ │
│  │  └─────────────┘  └─────────────┘  └─────────────────┘ │ │
│  └─────────────────────────────────────────────────────────┘ │
├─────────────────────────────────────────────────────────────┤
│                   Storage Layer                            │
│  ┌─────────────────┐  ┌─────────────┐  ┌─────────────────┐ │
│  │  Block Device   │  │   Vector    │  │    Metadata     │ │
│  │    Storage      │  │   Indices   │  │     Cache       │ │
│  └─────────────────┘  └─────────────┘  └─────────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

## 🔧 Core Components

### 1. Kernel Module Implementation

**Location**: [`kernel/vexfs_v2_build/`](../../kernel/vexfs_v2_build/)

The kernel module provides true filesystem semantics with vector capabilities:

#### Key Files:
- **[`vexfs_v2_main.c`](../../kernel/vexfs_v2_build/vexfs_v2_main.c)** - Main kernel module entry point
- **[`vexfs_v2_uapi.h`](../../kernel/vexfs_v2_build/vexfs_v2_uapi.h)** - User-space API definitions
- **[`vexfs_v2_hnsw.c`](../../kernel/vexfs_v2_build/vexfs_v2_hnsw.c)** - HNSW algorithm implementation
- **[`vexfs_v2_lsh.c`](../../kernel/vexfs_v2_build/vexfs_v2_lsh.c)** - LSH algorithm implementation
- **[`vexfs_v2_search.h`](../../kernel/vexfs_v2_build/vexfs_v2_search.h)** - Search interface definitions

#### Architecture Layers:

```c
// Kernel Module Architecture
┌─────────────────────────────────────┐
│         VFS Interface               │  // Standard Linux VFS
├─────────────────────────────────────┤
│      VexFS Filesystem Layer         │  // File operations, inodes
├─────────────────────────────────────┤
│       Vector Engine Core            │  // Vector operations
│  ┌─────────────┐  ┌─────────────────┐│
│  │    HNSW     │  │      LSH        ││  // ANNS algorithms
│  │  Algorithm  │  │   Algorithm     ││
│  └─────────────┘  └─────────────────┘│
├─────────────────────────────────────┤
│        Storage Backend              │  // Block device I/O
└─────────────────────────────────────┘
```

### 2. FUSE Implementation

**Location**: [`rust/src/fuse_impl.rs`](../../rust/src/fuse_impl.rs)

Userspace filesystem for development and cross-platform support:

```rust
// FUSE Architecture
pub struct VexFSFuse {
    vector_engine: VectorEngine,
    metadata_store: MetadataStore,
    file_operations: FileOperations,
    search_interface: SearchInterface,
}

impl Filesystem for VexFSFuse {
    // Standard FUSE operations
    fn lookup(&mut self, req: &Request, parent: u64, name: &OsStr, reply: ReplyEntry);
    fn getattr(&mut self, req: &Request, ino: u64, reply: ReplyAttr);
    fn read(&mut self, req: &Request, ino: u64, fh: u64, offset: i64, size: u32, reply: ReplyData);
    fn write(&mut self, req: &Request, ino: u64, fh: u64, offset: i64, data: &[u8], reply: ReplyWrite);
    
    // VexFS-specific operations
    fn setxattr(&mut self, req: &Request, ino: u64, name: &OsStr, value: &[u8], flags: u32, position: u32, reply: ReplyEmpty);
    fn getxattr(&mut self, req: &Request, ino: u64, name: &OsStr, size: u32, reply: ReplyXattr);
}
```

### 3. Vector Engine

The core vector processing engine shared between implementations:

#### HNSW (Hierarchical Navigable Small World)

```c
// HNSW Implementation Structure
typedef struct vexfs_hnsw_index {
    uint32_t dimension;
    uint32_t max_elements;
    uint32_t current_elements;
    uint32_t M;                    // Number of connections
    uint32_t max_M;               // Maximum connections
    uint32_t ef_construction;     // Construction parameter
    uint32_t ef_search;          // Search parameter
    
    struct vexfs_hnsw_node **levels;  // Multi-level graph
    uint32_t *level_counts;           // Nodes per level
    
    // Memory management
    struct kmem_cache *node_cache;
    struct kmem_cache *connection_cache;
} vexfs_hnsw_index_t;

// Core HNSW operations
int vexfs_hnsw_insert(vexfs_hnsw_index_t *index, 
                      const int32_t *vector, 
                      uint64_t id);

int vexfs_hnsw_search(vexfs_hnsw_index_t *index,
                      const int32_t *query,
                      uint32_t k,
                      struct vexfs_search_result *results);
```

#### LSH (Locality Sensitive Hashing)

```c
// LSH Implementation Structure
typedef struct vexfs_lsh_index {
    uint32_t dimension;
    uint32_t num_tables;
    uint32_t num_functions;
    uint32_t bucket_size;
    
    struct vexfs_lsh_table *tables;   // Hash tables
    struct vexfs_lsh_function *functions;  // Hash functions
    
    // Bucket management
    struct vexfs_lsh_bucket **buckets;
    uint32_t total_buckets;
    
    // Memory management
    struct kmem_cache *bucket_cache;
    struct kmem_cache *entry_cache;
} vexfs_lsh_index_t;

// Core LSH operations
int vexfs_lsh_insert(vexfs_lsh_index_t *index,
                     const int32_t *vector,
                     uint64_t id);

int vexfs_lsh_search(vexfs_lsh_index_t *index,
                     const int32_t *query,
                     uint32_t k,
                     struct vexfs_search_result *results);
```

## 🔄 Data Flow Architecture

### Vector Insertion Flow

```
User Application
       │
       ▼
┌─────────────────┐
│   SDK/API       │ ──► Validate vector dimensions
└─────────────────┘     Check metadata format
       │
       ▼
┌─────────────────┐
│ Kernel Module   │ ──► Allocate inode
│   or FUSE       │     Store file data
└─────────────────┘     Set extended attributes
       │
       ▼
┌─────────────────┐
│ Vector Engine   │ ──► Choose algorithm (HNSW/LSH)
└─────────────────┘     Insert into index
       │                Update statistics
       ▼
┌─────────────────┐
│ Storage Layer   │ ──► Write to block device
└─────────────────┘     Update metadata
                        Sync to disk
```

### Vector Search Flow

```
Search Query
       │
       ▼
┌─────────────────┐
│   SDK/API       │ ──► Parse query parameters
└─────────────────┘     Validate vector format
       │
       ▼
┌─────────────────┐
│ Query Planner   │ ──► Analyze filters
└─────────────────┘     Choose search strategy
       │                Optimize parameters
       ▼
┌─────────────────┐
│ Vector Engine   │ ──► Execute ANNS search
└─────────────────┘     Apply filters
       │                Rank results
       ▼
┌─────────────────┐
│ Result Builder  │ ──► Fetch metadata
└─────────────────┘     Format response
                        Return to user
```

## 🗄️ Storage Architecture

### On-Disk Layout

VexFS v2.0 uses a sophisticated on-disk layout optimized for vector operations:

```
VexFS v2.0 Disk Layout
┌─────────────────────────────────────────────────────────────┐
│                      Superblock                            │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────┐ │
│  │   Filesystem    │  │     Vector      │  │   Index     │ │
│  │    Metadata     │  │   Parameters    │  │  Metadata   │ │
│  └─────────────────┘  └─────────────────┘  └─────────────┘ │
├─────────────────────────────────────────────────────────────┤
│                    Inode Table                             │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │  Standard Inodes + Vector Extended Attributes          │ │
│  └─────────────────────────────────────────────────────────┘ │
├─────────────────────────────────────────────────────────────┤
│                   Data Blocks                              │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────┐ │
│  │   File Data     │  │   Vector Data   │  │  Metadata   │ │
│  │    Blocks       │  │     Blocks      │  │   Blocks    │ │
│  └─────────────────┘  └─────────────────┘  └─────────────┘ │
├─────────────────────────────────────────────────────────────┤
│                  Vector Indices                            │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────┐ │
│  │  HNSW Index     │  │   LSH Index     │  │   Auxiliary │ │
│  │    Blocks       │  │    Blocks       │  │   Indices   │ │
│  └─────────────────┘  └─────────────────┘  └─────────────┘ │
└─────────────────────────────────────────────────────────────┘
```

### Extended Attributes Schema

VexFS v2.0 stores vector metadata as extended attributes:

```c
// Extended Attribute Names
#define VEXFS_XATTR_VECTOR_DATA     "user.vexfs.vector.data"
#define VEXFS_XATTR_VECTOR_DIM      "user.vexfs.vector.dimension"
#define VEXFS_XATTR_VECTOR_ALGO     "user.vexfs.vector.algorithm"
#define VEXFS_XATTR_VECTOR_NORM     "user.vexfs.vector.norm"
#define VEXFS_XATTR_VECTOR_META     "user.vexfs.vector.metadata"

// Vector Data Structure
struct vexfs_vector_xattr {
    uint32_t magic;           // Magic number for validation
    uint32_t version;         // Format version
    uint32_t dimension;       // Vector dimension
    uint32_t algorithm;       // ANNS algorithm used
    uint32_t data_type;       // Data type (int32, float32, etc.)
    uint32_t flags;           // Various flags
    uint64_t timestamp;       // Creation timestamp
    uint32_t checksum;        // Data integrity checksum
    uint8_t data[];          // Vector data follows
} __packed;
```

## 🔌 API Architecture

### Kernel IOCTL Interface

The kernel module exposes functionality through IOCTL commands:

```c
// IOCTL Command Definitions
#define VEXFS_IOC_MAGIC 'V'

#define VEXFS_IOC_CREATE_COLLECTION    _IOW(VEXFS_IOC_MAGIC, 1, struct vexfs_collection_create)
#define VEXFS_IOC_INSERT_VECTOR        _IOW(VEXFS_IOC_MAGIC, 2, struct vexfs_vector_insert)
#define VEXFS_IOC_SEARCH_VECTORS       _IOWR(VEXFS_IOC_MAGIC, 3, struct vexfs_vector_search)
#define VEXFS_IOC_GET_STATS           _IOR(VEXFS_IOC_MAGIC, 4, struct vexfs_stats)

// IOCTL Data Structures
struct vexfs_collection_create {
    char name[256];
    uint32_t dimension;
    uint32_t algorithm;        // HNSW or LSH
    uint32_t distance_metric;  // Cosine, Euclidean, etc.
    union {
        struct vexfs_hnsw_params hnsw;
        struct vexfs_lsh_params lsh;
    } params;
};

struct vexfs_vector_insert {
    uint64_t collection_id;
    uint32_t dimension;
    uint32_t count;           // For batch operations
    const int32_t *vectors;   // Vector data
    const char *metadata;     // JSON metadata
    uint64_t *result_ids;     // Output: assigned IDs
};

struct vexfs_vector_search {
    uint64_t collection_id;
    uint32_t dimension;
    const int32_t *query_vector;
    uint32_t k;               // Number of results
    uint32_t ef_search;       // HNSW parameter
    const char *filter;       // JSON filter
    struct vexfs_search_result *results;  // Output buffer
    uint32_t *result_count;   // Output: actual count
};
```

### Language Bindings Architecture

#### Python SDK

```python
# Python SDK Architecture
class VexFSClient:
    def __init__(self, mount_path: str):
        self._mount_path = mount_path
        self._collections = {}
        self._ioctl_fd = None
    
    def _open_ioctl(self) -> int:
        """Open IOCTL interface to kernel module"""
        control_path = os.path.join(self._mount_path, ".vexfs_control")
        return os.open(control_path, os.O_RDWR)
    
    def create_collection(self, name: str, dimension: int, 
                         algorithm: str = "hnsw", **params) -> Collection:
        """Create a new vector collection"""
        # Implementation details...

class Collection:
    def insert(self, vector: np.ndarray, metadata: dict = None) -> InsertResult:
        """Insert a vector into the collection"""
        # Validate vector format
        # Convert to kernel format
        # Execute IOCTL call
        
    def search(self, vector: np.ndarray, limit: int = 10, 
               filter: dict = None) -> List[SearchResult]:
        """Search for similar vectors"""
        # Prepare search parameters
        # Execute IOCTL call
        # Parse results
```

#### TypeScript SDK

```typescript
// TypeScript SDK Architecture
export class VexFSClient {
    private mountPath: string;
    private ioctlFd: number | null = null;
    
    constructor(mountPath: string) {
        this.mountPath = mountPath;
    }
    
    async createCollection(options: CollectionOptions): Promise<Collection> {
        // Implementation using Node.js native bindings
    }
}

export class Collection {
    async insert(vector: number[], metadata?: object): Promise<InsertResult> {
        // Vector validation and insertion
    }
    
    async search(vector: number[], options?: SearchOptions): Promise<SearchResult[]> {
        // Vector search implementation
    }
}
```

## 🧠 Memory Management

### Kernel Memory Architecture

```c
// Memory Pool Management
struct vexfs_memory_pool {
    struct kmem_cache *vector_cache;      // Vector data cache
    struct kmem_cache *node_cache;        // HNSW node cache
    struct kmem_cache *bucket_cache;      // LSH bucket cache
    struct kmem_cache *result_cache;      // Search result cache
    
    // Memory limits
    size_t max_vector_memory;
    size_t max_index_memory;
    size_t current_usage;
    
    // Statistics
    atomic64_t allocations;
    atomic64_t deallocations;
    atomic64_t cache_hits;
    atomic64_t cache_misses;
};

// Memory allocation functions
void *vexfs_alloc_vector(size_t size);
void *vexfs_alloc_node(void);
void *vexfs_alloc_bucket(void);
void vexfs_free_vector(void *ptr);
void vexfs_free_node(void *ptr);
void vexfs_free_bucket(void *ptr);
```

### Cache Management

```c
// Multi-level caching system
struct vexfs_cache_system {
    // L1: Vector data cache (hot vectors)
    struct vexfs_lru_cache *vector_cache;
    
    // L2: Index structure cache (HNSW nodes, LSH buckets)
    struct vexfs_lru_cache *index_cache;
    
    // L3: Metadata cache (search results, statistics)
    struct vexfs_lru_cache *metadata_cache;
    
    // Cache policies
    enum vexfs_cache_policy policy;
    uint32_t max_entries;
    size_t max_memory;
};
```

## 🔒 Security Architecture

### Access Control

```c
// VexFS Security Model
struct vexfs_security_context {
    uid_t uid;                    // User ID
    gid_t gid;                    // Group ID
    uint32_t capabilities;        // Capability flags
    uint32_t collection_perms;    // Collection permissions
    uint32_t vector_perms;        // Vector operation permissions
};

// Permission checking
int vexfs_check_collection_access(struct vexfs_security_context *ctx,
                                  uint64_t collection_id,
                                  uint32_t requested_perms);

int vexfs_check_vector_access(struct vexfs_security_context *ctx,
                              uint64_t vector_id,
                              uint32_t requested_perms);
```

### Data Integrity

```c
// Integrity verification
struct vexfs_integrity_check {
    uint32_t checksum_algorithm;  // CRC32, SHA256, etc.
    uint64_t last_check_time;
    uint32_t error_count;
    uint32_t repair_count;
};

// Integrity functions
int vexfs_verify_vector_integrity(uint64_t vector_id);
int vexfs_verify_index_integrity(uint64_t collection_id);
int vexfs_repair_corruption(uint64_t object_id);
```

## 📊 Performance Architecture

### Parallel Processing

```c
// Work queue system for parallel operations
struct vexfs_work_queue {
    struct workqueue_struct *insert_wq;   // Vector insertion queue
    struct workqueue_struct *search_wq;   // Search operation queue
    struct workqueue_struct *maint_wq;    // Maintenance queue
    
    // Worker configuration
    uint32_t insert_workers;
    uint32_t search_workers;
    uint32_t maint_workers;
};

// Batch processing
struct vexfs_batch_operation {
    uint32_t operation_type;
    uint32_t batch_size;
    void **batch_data;
    struct completion *completion;
    atomic_t remaining_ops;
};
```

### Performance Monitoring

```c
// Performance metrics collection
struct vexfs_performance_metrics {
    // Operation counters
    atomic64_t vector_insertions;
    atomic64_t vector_searches;
    atomic64_t cache_hits;
    atomic64_t cache_misses;
    
    // Timing statistics
    uint64_t avg_insert_time_ns;
    uint64_t avg_search_time_ns;
    uint64_t max_insert_time_ns;
    uint64_t max_search_time_ns;
    
    // Resource usage
    size_t memory_usage;
    uint32_t active_collections;
    uint64_t total_vectors;
};
```

## 🔧 Build System Architecture

### Kernel Module Build

```makefile
# Kernel module Makefile structure
obj-m := vexfs_v2.o

vexfs_v2-objs := vexfs_v2_main.o \
                 vexfs_v2_hnsw.o \
                 vexfs_v2_lsh.o \
                 vexfs_v2_search.o \
                 vexfs_v2_advanced_search.o

# Compiler flags
ccflags-y := -DVEXFS_VERSION=\"2.0.0\" \
             -DVEXFS_DEBUG \
             -O2 \
             -Wall \
             -Wextra

# Kernel build integration
all:
	$(MAKE) -C /lib/modules/$(shell uname -r)/build M=$(PWD) modules

clean:
	$(MAKE) -C /lib/modules/$(shell uname -r)/build M=$(PWD) clean
```

### Rust Build Integration

```toml
# Cargo.toml for Rust components
[package]
name = "vexfs"
version = "2.0.0"
edition = "2021"

[lib]
name = "vexfs"
crate-type = ["cdylib", "rlib"]

[[bin]]
name = "vexfs_fuse"
path = "src/bin/vexfs_fuse.rs"

[dependencies]
fuse = "0.13"
libc = "0.2"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1.0", features = ["full"] }

[build-dependencies]
cbindgen = "0.24"
```

## 🚀 Future Architecture Considerations

### Scalability Enhancements

1. **Distributed Architecture**: Multi-node vector search
2. **GPU Acceleration**: CUDA/OpenCL integration
3. **Advanced Algorithms**: Learned indices, quantum-inspired search
4. **Cloud Integration**: S3-compatible storage backend

### Performance Optimizations

1. **SIMD Vectorization**: AVX-512 optimized distance calculations
2. **Memory Mapping**: Zero-copy vector operations
3. **Async I/O**: io_uring integration for high-performance I/O
4. **Compression**: Vector compression for storage efficiency

---

This architecture provides the foundation for VexFS v2.0's high-performance vector operations while maintaining filesystem compatibility and extensibility for future enhancements.

**Next**: [API Reference](api-reference.md) | [Contributing Guide](contributing.md) | [Testing Guide](testing.md)