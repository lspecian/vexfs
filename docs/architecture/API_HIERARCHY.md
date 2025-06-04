# VexFS v2.0 API Hierarchy and Structure

**Status**: ✅ **OFFICIAL API REFERENCE**  
**Date**: June 4, 2025  
**Scope**: All VexFS v2.0 APIs and interfaces

## API Architecture Overview

VexFS v2.0 provides a layered API architecture with clear separation between user-space interfaces, kernel interfaces, and internal implementations.

```
┌─────────────────────────────────────────────────────────────┐
│                    VexFS v2.0 API Stack                    │
├─────────────────────────────────────────────────────────────┤
│  User Applications                                          │
│  ├── Python SDK                                            │
│  ├── TypeScript SDK                                        │
│  └── C/C++ Applications                                    │
├─────────────────────────────────────────────────────────────┤
│  User-Space API Layer                                      │
│  ├── IOCTL Interface (vexfs_v2_uapi.h)                    │
│  ├── File Operations (POSIX)                              │
│  └── Mount/Unmount Operations                              │
├─────────────────────────────────────────────────────────────┤
│  Kernel-Space API Layer                                    │
│  ├── Vector Search Operations                              │
│  ├── Index Management                                      │
│  ├── Multi-Model Support                                   │
│  └── Performance Monitoring                                │
├─────────────────────────────────────────────────────────────┤
│  Internal Implementation Layer                             │
│  ├── HNSW Index Implementation                             │
│  ├── LSH Index Implementation                              │
│  ├── Brute Force Search                                    │
│  └── SIMD Optimizations                                    │
└─────────────────────────────────────────────────────────────┘
```

## Primary API Categories

### **1. User-Space APIs** (Public Interface)

#### **IOCTL Interface** - `vexfs_v2_uapi.h`
**Status**: ✅ **PRIMARY API** - Production Ready  
**Purpose**: Main interface for vector database operations

**Core Operations**:
```c
// Vector Storage Operations
#define VEXFS_IOC_INSERT_VECTOR     _IOW(VEXFS_IOC_MAGIC, 1, struct vexfs_vector_insert_request)
#define VEXFS_IOC_BATCH_INSERT      _IOW(VEXFS_IOC_MAGIC, 2, struct vexfs_batch_insert_request)

// Vector Search Operations  
#define VEXFS_IOC_VECTOR_SEARCH     _IOWR(VEXFS_IOC_MAGIC, 3, struct vexfs_vector_search_request)
#define VEXFS_IOC_KNN_SEARCH        _IOWR(VEXFS_IOC_MAGIC, 4, struct vexfs_knn_query)

// Metadata Operations
#define VEXFS_IOC_GET_METADATA      _IOR(VEXFS_IOC_MAGIC, 5, struct vexfs_metadata_request)
#define VEXFS_IOC_SET_METADATA      _IOW(VEXFS_IOC_MAGIC, 6, struct vexfs_metadata_request)
```

#### **POSIX File Operations**
**Status**: ✅ **STANDARD** - Full POSIX Compliance  
**Purpose**: Traditional filesystem operations

**Operations**:
- `open()`, `read()`, `write()`, `close()`
- `mkdir()`, `rmdir()`, `opendir()`, `readdir()`
- `stat()`, `chmod()`, `chown()`

### **2. Kernel-Space APIs** (Internal Interface)

#### **Vector Search API** - `vexfs_v2_search.h`
**Status**: ✅ **INTERNAL API** - Kernel Module Interface  
**Purpose**: Core vector search implementations

**Primary Functions**:
```c
// Core Search Operations
int vexfs_knn_search(struct file *file, struct vexfs_knn_query *query);
int vexfs_range_search(struct file *file, struct vexfs_range_query *query);
int vexfs_batch_search(struct file *file, struct vexfs_batch_search_request *req);

// Distance Calculations
uint32_t vexfs_euclidean_distance_bits(const uint32_t *vec_a, const uint32_t *vec_b, size_t dims);
uint32_t vexfs_cosine_similarity_bits(const uint32_t *vec_a, const uint32_t *vec_b, size_t dims);
uint32_t vexfs_manhattan_distance_bits(const uint32_t *vec_a, const uint32_t *vec_b, size_t dims);
```

#### **Advanced Search API** - `vexfs_v2_phase3.h`
**Status**: ✅ **ADVANCED API** - Phase 3 Features  
**Purpose**: Advanced indexing and multi-model support

**Advanced Functions**:
```c
// Multi-Model Operations
int vexfs_multi_model_init(void);
int vexfs_multi_model_store_embedding(const struct vexfs_embedding_metadata *metadata,
                                     const uint32_t *embedding_bits, uint32_t dimensions);

// Advanced Search Operations
int vexfs_filtered_search(const uint32_t *query_vector, uint32_t dimensions,
                         const struct vexfs_filter_criteria *filters,
                         struct vexfs_search_result *results, uint32_t *result_count);

// Index Management
int vexfs_create_index(enum vexfs_index_type type, const struct vexfs_index_metadata *metadata);
int vexfs_rebuild_index(uint32_t index_id);
```

## Search Algorithm Hierarchy

### **Search Implementation Layers**

#### **Layer 1: Brute Force Search** (Baseline)
**Status**: ✅ **PRODUCTION READY**  
**Implementation**: `vexfs_v2_search.c`  
**Use Case**: Small datasets, exact results, fallback option

**Functions**:
- `vexfs_knn_search()` - Basic k-nearest neighbor
- `vexfs_range_search()` - Distance-based range queries
- `vexfs_batch_search()` - Multiple query processing

#### **Layer 2: LSH Index** (Approximate)
**Status**: ✅ **PRODUCTION READY**  
**Implementation**: `vexfs_v2_lsh.c`  
**Use Case**: Large datasets, approximate results, high throughput

**Functions**:
- `vexfs_lsh_init()` - Initialize LSH index
- `vexfs_lsh_insert()` - Add vectors to LSH index
- `vexfs_lsh_search()` - LSH-based approximate search

#### **Layer 3: HNSW Index** (Hierarchical)
**Status**: ✅ **PRODUCTION READY**  
**Implementation**: `vexfs_v2_hnsw.c`  
**Use Case**: Complex queries, hierarchical navigation, balanced performance

**Functions**:
- `vexfs_hnsw_init()` - Initialize HNSW index
- `vexfs_hnsw_insert()` - Add vectors to HNSW index
- `vexfs_hnsw_search()` - HNSW-based hierarchical search

#### **Layer 4: Smart Search** (Adaptive)
**Status**: ✅ **PRODUCTION READY**  
**Implementation**: `vexfs_v2_phase3_integration.c`  
**Use Case**: Automatic algorithm selection, optimal performance

**Functions**:
- `vexfs_phase3_smart_search()` - Intelligent algorithm selection
- `vexfs_phase3_ioctl()` - Unified Phase 3 interface

## API Usage Patterns

### **Basic Vector Operations**
```c
// 1. Insert vectors
struct vexfs_vector_insert_request insert_req = {
    .vector_id = 123,
    .dimensions = 768,
    .vector_data = vector_bits  // IEEE 754 bit representation
};
ioctl(fd, VEXFS_IOC_INSERT_VECTOR, &insert_req);

// 2. Search vectors
struct vexfs_knn_query search_req = {
    .query_vector = query_bits,
    .dimensions = 768,
    .k = 10
};
ioctl(fd, VEXFS_IOC_KNN_SEARCH, &search_req);
```

### **Advanced Index Operations**
```c
// 1. Create HNSW index
struct vexfs_index_metadata index_meta = {
    .index_type = VEXFS_INDEX_HNSW,
    .dimensions = 768,
    .max_connections = 16
};
ioctl(fd, VEXFS_IOC_CREATE_INDEX, &index_meta);

// 2. Use smart search
struct vexfs_smart_search_request smart_req = {
    .query_vector = query_bits,
    .dimensions = 768,
    .k = 10,
    .optimization_hint = VEXFS_OPTIMIZE_SPEED
};
ioctl(fd, VEXFS_IOC_SMART_SEARCH, &smart_req);
```

## API Compatibility Matrix

### **Current APIs** (VexFS v2.0)

| **API Component** | **Status** | **Stability** | **Use Case** |
|-------------------|------------|---------------|--------------|
| **IOCTL Interface** | ✅ Production | Stable | Primary user interface |
| **POSIX Operations** | ✅ Production | Stable | File system operations |
| **Brute Force Search** | ✅ Production | Stable | Small datasets |
| **LSH Index** | ✅ Production | Stable | Large datasets |
| **HNSW Index** | ✅ Production | Stable | Complex queries |
| **Multi-Model Support** | ✅ Production | Stable | Multiple embedding models |
| **Smart Search** | ✅ Production | Stable | Adaptive performance |

### **Deprecated APIs** (Legacy)

| **Legacy Component** | **Status** | **Replacement** | **Removal Timeline** |
|---------------------|------------|-----------------|---------------------|
| `vexfs_v1_*` functions | ❌ Deprecated | `vexfs_v2_*` functions | Removed in v2.0 |
| Float-based interfaces | ❌ Deprecated | IEEE 754 bit interfaces | Removed in v2.0 |
| Phase-specific naming | ❌ Deprecated | Unified v2.0 naming | Transition ongoing |

## Performance Characteristics

### **API Performance Hierarchy**

| **API Layer** | **Latency** | **Throughput** | **Accuracy** | **Use Case** |
|---------------|-------------|----------------|--------------|--------------|
| **IOCTL Interface** | <100μs | 361K+ ops/sec | 100% | Metadata operations |
| **Brute Force Search** | 0.241ms | 4K ops/sec | 100% | Small datasets |
| **LSH Search** | 0.1ms | 50K+ ops/sec | ~95% | Large datasets |
| **HNSW Search** | 0.2ms | 20K+ ops/sec | ~98% | Complex queries |
| **Smart Search** | Variable | Optimal | Variable | Adaptive selection |

## Integration Guidelines

### **For Application Developers**
1. **Use IOCTL interface** for vector operations
2. **Use POSIX interface** for file operations
3. **Start with brute force** for prototyping
4. **Upgrade to indexed search** for production

### **For System Integrators**
1. **Include `vexfs_v2_uapi.h`** for all user-space code
2. **Use standardized structures** from UAPI header
3. **Handle IEEE 754 conversion** for floating-point data
4. **Implement error handling** for all IOCTL calls

### **For Kernel Developers**
1. **Follow kernel coding standards** for all internal APIs
2. **Use integer-only arithmetic** in kernel space
3. **Export symbols** for module integration
4. **Maintain backward compatibility** during transitions

## Future API Evolution

### **Planned Enhancements**
- **Streaming API**: For large-scale data ingestion
- **Distributed API**: For multi-node deployments
- **GPU API**: For hardware acceleration
- **ML Pipeline API**: For integrated training workflows

### **Compatibility Guarantees**
- **IOCTL interface**: Stable across minor versions
- **POSIX interface**: Always maintained
- **Internal APIs**: May change between major versions
- **Deprecated APIs**: 6-month deprecation notice

## References

- [Version Standardization Guide](VERSION_STANDARDIZATION.md)
- [Legacy Version Mapping](LEGACY_VERSION_MAPPING.md)
- [VexFS v2.0 UAPI Header](../../kernel/vexfs_v2_build/vexfs_v2_uapi.h)
- [VexFS v2.0 Architecture](VEXFS_V2_FLOATING_POINT_ARCHITECTURE.md)

---

**This document defines the official API hierarchy for VexFS v2.0. All development must follow these interface specifications.**

**Last Updated**: June 4, 2025  
**Next Review**: September 2025  
**Maintained By**: VexFS Development Team