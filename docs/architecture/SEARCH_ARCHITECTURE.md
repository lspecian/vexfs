# VexFS v2.0 Search Module

## Overview

The `search/` directory contains all vector search implementations for VexFS v2.0, including core search algorithms, advanced indexing methods, and multi-model search capabilities.

## Files

### Core Search Infrastructure

#### `vexfs_v2_search.h`
**Purpose**: Search function declarations and core structures

**Key Components**:
- Search algorithm interfaces
- Distance calculation function prototypes
- Search result structures
- Configuration constants

#### `vexfs_v2_search.c`
**Purpose**: Core search algorithms and distance calculations

**Key Functions**:
- `vexfs_euclidean_distance_scaled()` - Integer-based Euclidean distance
- `vexfs_cosine_similarity_scaled()` - Integer-based cosine similarity
- `vexfs_knn_search()` - K-nearest neighbor search
- `vexfs_search_init()` - Search subsystem initialization
- `vexfs_search_exit()` - Search subsystem cleanup

**Features**:
- Integer-only arithmetic (no floating-point)
- SIMD-optimized distance calculations
- Scalable search algorithms
- Memory-efficient implementations

### Advanced Search Operations

#### `vexfs_v2_advanced_search.c`
**Purpose**: Advanced search operations and IOCTL handlers

**Key Functions**:
- `vexfs_advanced_search_ioctl()` - Main IOCTL dispatcher
- `vexfs_batch_search()` - Batch search operations
- `vexfs_similarity_search()` - Similarity-based search
- `vexfs_advanced_search_cleanup()` - Resource cleanup

**IOCTL Commands**:
- `VEXFS_IOCTL_BATCH_SEARCH` - Batch vector search
- `VEXFS_IOCTL_SIMILARITY_SEARCH` - Similarity search
- `VEXFS_IOCTL_ADVANCED_STATS` - Advanced statistics

### Indexing Algorithms

#### `vexfs_v2_lsh.c`
**Purpose**: Locality-Sensitive Hashing (LSH) implementation

**Key Features**:
- **Fast Approximate Search**: O(1) average lookup time
- **Configurable Hash Tables**: Multiple hash tables for accuracy
- **Integer-Only Operations**: No floating-point arithmetic
- **Memory Efficient**: Optimized hash table storage

**Key Functions**:
- `vexfs_lsh_init()` - Initialize LSH structures
- `vexfs_lsh_insert()` - Insert vector into LSH index
- `vexfs_lsh_search()` - Search using LSH index
- `vexfs_lsh_get_stats()` - Get LSH statistics
- `vexfs_lsh_cleanup()` - Cleanup LSH resources

**Configuration**:
- Hash table count: Configurable for accuracy vs. speed
- Hash function parameters: Tunable for different data distributions
- Bucket size limits: Memory usage control

#### `vexfs_v2_hnsw.c`
**Purpose**: Hierarchical Navigable Small World (HNSW) implementation

**Key Features**:
- **High Accuracy**: Near-optimal search results
- **Logarithmic Search Time**: O(log n) search complexity
- **Dynamic Updates**: Support for insertions and deletions
- **Layer-Based Structure**: Hierarchical graph organization

**Key Functions**:
- `vexfs_hnsw_init()` - Initialize HNSW graph
- `vexfs_hnsw_insert()` - Insert vector into HNSW graph
- `vexfs_hnsw_search()` - Search using HNSW graph
- `vexfs_hnsw_get_stats()` - Get HNSW statistics
- `vexfs_hnsw_cleanup()` - Cleanup HNSW resources

**Configuration**:
- Maximum connections per layer: Accuracy vs. memory trade-off
- Layer probability: Controls graph structure
- Search beam width: Search accuracy vs. speed

### Integration and Coordination

#### `vexfs_v2_multi_model.c`
**Purpose**: Multi-model search support

**Key Features**:
- **Algorithm Selection**: Choose optimal algorithm per query
- **Performance Monitoring**: Track algorithm performance
- **Adaptive Switching**: Dynamic algorithm selection
- **Unified Interface**: Single interface for multiple algorithms

**Key Functions**:
- `vexfs_multi_model_ioctl()` - Multi-model IOCTL handler
- `vexfs_multi_model_init()` - Initialize multi-model support
- `vexfs_multi_model_cleanup()` - Cleanup multi-model resources

#### `vexfs_v2_phase3_integration.c`
**Purpose**: Phase 3 integration and coordination

**Key Features**:
- **Algorithm Coordination**: Coordinate between LSH and HNSW
- **Performance Optimization**: Optimize based on query patterns
- **Statistics Collection**: Comprehensive performance metrics
- **Smart Search**: Intelligent algorithm selection

**Key Functions**:
- `vexfs_phase3_init()` - Initialize Phase 3 features
- `vexfs_phase3_cleanup()` - Cleanup Phase 3 resources
- `vexfs_phase3_ioctl()` - Phase 3 IOCTL handler
- `vexfs_phase3_smart_search()` - Intelligent search routing

## Architecture

### Search Algorithm Hierarchy

```
Core Search (vexfs_v2_search.c)
├── Distance Calculations
├── Basic K-NN Search
└── Search Infrastructure

Advanced Search (vexfs_v2_advanced_search.c)
├── Batch Operations
├── Similarity Search
└── Advanced Statistics

Indexing Algorithms
├── LSH (vexfs_v2_lsh.c)
│   ├── Hash Tables
│   ├── Hash Functions
│   └── Bucket Management
└── HNSW (vexfs_v2_hnsw.c)
    ├── Graph Structure
    ├── Layer Management
    └── Navigation Logic

Integration Layer
├── Multi-Model (vexfs_v2_multi_model.c)
│   ├── Algorithm Selection
│   └── Performance Monitoring
└── Phase 3 (vexfs_v2_phase3_integration.c)
    ├── Coordination
    └── Smart Routing
```

### Data Flow

1. **Query Reception**: IOCTL commands received from userspace
2. **Algorithm Selection**: Choose optimal algorithm based on query type
3. **Search Execution**: Execute search using selected algorithm
4. **Result Processing**: Process and format search results
5. **Response**: Return results to userspace

## Performance Characteristics

### LSH (Locality-Sensitive Hashing)
- **Search Time**: O(1) average, O(n) worst case
- **Memory Usage**: O(n) for hash tables
- **Accuracy**: Configurable, typically 80-95%
- **Best For**: Large datasets, approximate results acceptable

### HNSW (Hierarchical Navigable Small World)
- **Search Time**: O(log n) average
- **Memory Usage**: O(n) for graph structure
- **Accuracy**: Very high, typically 95-99%
- **Best For**: High accuracy requirements, moderate datasets

### Multi-Model Selection
- **Automatic**: Based on dataset size and accuracy requirements
- **Manual**: User can specify preferred algorithm
- **Adaptive**: Learns from query patterns and performance

## Configuration

### LSH Configuration
```c
// Number of hash tables (more = higher accuracy, more memory)
#define VEXFS_LSH_NUM_TABLES 8

// Hash function parameters
#define VEXFS_LSH_HASH_BITS 16
#define VEXFS_LSH_BUCKET_SIZE 64
```

### HNSW Configuration
```c
// Maximum connections per layer
#define VEXFS_HNSW_MAX_CONNECTIONS 16

// Layer probability factor
#define VEXFS_HNSW_LEVEL_FACTOR 2

// Search beam width
#define VEXFS_HNSW_SEARCH_BEAM 32
```

## Development Guidelines

### Adding New Search Algorithms
1. Create new `.c` file in `search/` directory
2. Implement standard interface functions:
   - `algorithm_init()`
   - `algorithm_insert()`
   - `algorithm_search()`
   - `algorithm_cleanup()`
3. Add integration to multi-model system
4. Update Phase 3 coordination logic

### Performance Optimization
- Use integer-only arithmetic
- Implement SIMD optimizations where possible
- Minimize memory allocations in search paths
- Cache frequently accessed data structures

### Testing New Algorithms
1. Add unit tests in `../tests/`
2. Implement performance benchmarks
3. Verify accuracy against ground truth
4. Test memory usage and cleanup

## Debugging and Monitoring

### Performance Statistics
- Search latency per algorithm
- Memory usage tracking
- Cache hit/miss ratios
- Algorithm selection statistics

### Debug Features
- Detailed logging for search operations
- Performance counters for each algorithm
- Memory leak detection
- Search result validation

---

The search module provides the core vector search capabilities for VexFS v2.0, implementing state-of-the-art algorithms with optimal performance and accuracy characteristics.