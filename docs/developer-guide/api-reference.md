# VexFS v2.0 API Reference

Complete API reference for VexFS v2.0, covering all interfaces, data structures, and programming examples.

## 📚 API Overview

VexFS v2.0 provides multiple API layers:

1. **Kernel IOCTL Interface** - Direct kernel module communication
2. **Python SDK** - High-level Python bindings
3. **TypeScript SDK** - JavaScript/TypeScript bindings
4. **CLI Interface (vexctl)** - Command-line tool
5. **REST API** - HTTP interface (when available)

## 🔧 Kernel IOCTL Interface

### Core Data Structures

#### Vector Data Types

```c
// Vector data representation
typedef struct vexfs_vector {
    uint32_t dimension;
    uint32_t data_type;        // VEXFS_DTYPE_INT32, VEXFS_DTYPE_FLOAT32
    union {
        int32_t *int32_data;
        float *float32_data;
    } data;
} vexfs_vector_t;

// Vector metadata
typedef struct vexfs_vector_metadata {
    uint64_t id;
    uint64_t timestamp;
    uint32_t flags;
    char *json_metadata;       // JSON string
    size_t metadata_size;
} vexfs_vector_metadata_t;

// Search result
typedef struct vexfs_search_result {
    uint64_t vector_id;
    float distance;
    vexfs_vector_metadata_t metadata;
} vexfs_search_result_t;
```

#### Collection Management

```c
// Collection creation parameters
typedef struct vexfs_collection_create {
    char name[VEXFS_MAX_COLLECTION_NAME];
    uint32_t dimension;
    uint32_t algorithm;        // VEXFS_ALGO_HNSW, VEXFS_ALGO_LSH
    uint32_t distance_metric;  // VEXFS_DIST_COSINE, VEXFS_DIST_EUCLIDEAN
    
    union {
        struct {
            uint32_t m;                    // Number of connections
            uint32_t ef_construction;      // Construction parameter
            uint32_t ef_search;           // Search parameter
            uint32_t max_m;               // Maximum connections
            float ml;                     // Level generation factor
        } hnsw;
        
        struct {
            uint32_t num_tables;          // Number of hash tables
            uint32_t num_functions;       // Hash functions per table
            uint32_t bucket_size;         // Target bucket size
            uint32_t projection_type;     // Random or learned projections
        } lsh;
    } params;
} vexfs_collection_create_t;

// Collection information
typedef struct vexfs_collection_info {
    uint64_t collection_id;
    char name[VEXFS_MAX_COLLECTION_NAME];
    uint32_t dimension;
    uint32_t algorithm;
    uint32_t distance_metric;
    uint64_t vector_count;
    uint64_t created_timestamp;
    uint64_t modified_timestamp;
    size_t memory_usage;
} vexfs_collection_info_t;
```

### IOCTL Commands

#### Collection Operations

```c
// Create collection
#define VEXFS_IOC_CREATE_COLLECTION _IOW(VEXFS_IOC_MAGIC, 1, vexfs_collection_create_t)

// List collections
#define VEXFS_IOC_LIST_COLLECTIONS _IOR(VEXFS_IOC_MAGIC, 2, vexfs_collection_list_t)

// Get collection info
#define VEXFS_IOC_GET_COLLECTION_INFO _IOWR(VEXFS_IOC_MAGIC, 3, vexfs_collection_info_t)

// Delete collection
#define VEXFS_IOC_DELETE_COLLECTION _IOW(VEXFS_IOC_MAGIC, 4, uint64_t)

// Usage example
int fd = open("/mnt/vexfs/.vexfs_control", O_RDWR);

vexfs_collection_create_t create_params = {
    .name = "my_collection",
    .dimension = 384,
    .algorithm = VEXFS_ALGO_HNSW,
    .distance_metric = VEXFS_DIST_COSINE,
    .params.hnsw = {
        .m = 16,
        .ef_construction = 200,
        .ef_search = 100,
        .max_m = 32,
        .ml = 1.0 / log(2.0)
    }
};

if (ioctl(fd, VEXFS_IOC_CREATE_COLLECTION, &create_params) < 0) {
    perror("Failed to create collection");
}
```

#### Vector Operations

```c
// Insert vector
typedef struct vexfs_vector_insert {
    uint64_t collection_id;
    vexfs_vector_t vector;
    vexfs_vector_metadata_t metadata;
    uint64_t *result_id;       // Output: assigned vector ID
} vexfs_vector_insert_t;

#define VEXFS_IOC_INSERT_VECTOR _IOWR(VEXFS_IOC_MAGIC, 10, vexfs_vector_insert_t)

// Batch insert
typedef struct vexfs_vector_batch_insert {
    uint64_t collection_id;
    uint32_t count;
    vexfs_vector_t *vectors;
    vexfs_vector_metadata_t *metadata;
    uint64_t *result_ids;      // Output: assigned vector IDs
    uint32_t batch_size;       // Processing batch size
} vexfs_vector_batch_insert_t;

#define VEXFS_IOC_BATCH_INSERT _IOWR(VEXFS_IOC_MAGIC, 11, vexfs_vector_batch_insert_t)

// Search vectors
typedef struct vexfs_vector_search {
    uint64_t collection_id;
    vexfs_vector_t query_vector;
    uint32_t k;                // Number of results
    uint32_t ef_search;        // HNSW search parameter
    char *filter_json;         // JSON filter string
    size_t filter_size;
    vexfs_search_result_t *results;  // Output buffer
    uint32_t *result_count;    // Output: actual result count
    float max_distance;        // Maximum distance threshold
} vexfs_vector_search_t;

#define VEXFS_IOC_SEARCH_VECTORS _IOWR(VEXFS_IOC_MAGIC, 12, vexfs_vector_search_t)

// Usage example
vexfs_vector_search_t search_params = {
    .collection_id = collection_id,
    .query_vector = {
        .dimension = 384,
        .data_type = VEXFS_DTYPE_INT32,
        .data.int32_data = query_data
    },
    .k = 10,
    .ef_search = 100,
    .filter_json = "{\"category\": \"documents\"}",
    .filter_size = strlen(filter_json),
    .results = results_buffer,
    .result_count = &actual_count,
    .max_distance = 1.0
};

if (ioctl(fd, VEXFS_IOC_SEARCH_VECTORS, &search_params) < 0) {
    perror("Search failed");
}
```

#### Statistics and Monitoring

```c
// Performance statistics
typedef struct vexfs_performance_stats {
    uint64_t total_vectors;
    uint64_t total_searches;
    uint64_t total_insertions;
    uint64_t cache_hits;
    uint64_t cache_misses;
    
    // Timing statistics (nanoseconds)
    uint64_t avg_search_time_ns;
    uint64_t avg_insert_time_ns;
    uint64_t max_search_time_ns;
    uint64_t max_insert_time_ns;
    
    // Memory usage
    size_t total_memory_usage;
    size_t vector_memory_usage;
    size_t index_memory_usage;
    size_t cache_memory_usage;
} vexfs_performance_stats_t;

#define VEXFS_IOC_GET_STATS _IOR(VEXFS_IOC_MAGIC, 20, vexfs_performance_stats_t)

// Collection statistics
typedef struct vexfs_collection_stats {
    uint64_t collection_id;
    uint64_t vector_count;
    uint64_t search_count;
    uint64_t insert_count;
    float avg_vector_norm;
    float std_vector_norm;
    size_t index_size;
    uint32_t index_levels;     // For HNSW
    uint32_t avg_bucket_size;  // For LSH
} vexfs_collection_stats_t;

#define VEXFS_IOC_GET_COLLECTION_STATS _IOWR(VEXFS_IOC_MAGIC, 21, vexfs_collection_stats_t)
```

## 🐍 Python SDK

### Installation

```bash
pip install vexfs-v2
```

### Core Classes

#### VexFSClient

```python
import vexfs
import numpy as np
from typing import List, Dict, Optional, Union

class VexFSClient:
    """Main client for interacting with VexFS v2.0"""
    
    def __init__(self, mount_path: str, timeout: float = 30.0):
        """
        Initialize VexFS client
        
        Args:
            mount_path: Path to VexFS mount point
            timeout: Operation timeout in seconds
        """
        self.mount_path = mount_path
        self.timeout = timeout
        self._ioctl_fd = None
    
    def create_collection(
        self,
        name: str,
        dimension: int,
        algorithm: str = "hnsw",
        distance_metric: str = "cosine",
        **params
    ) -> 'Collection':
        """
        Create a new vector collection
        
        Args:
            name: Collection name
            dimension: Vector dimension
            algorithm: "hnsw" or "lsh"
            distance_metric: "cosine", "euclidean", "manhattan"
            **params: Algorithm-specific parameters
            
        Returns:
            Collection object
            
        Example:
            >>> client = vexfs.Client('/mnt/vexfs')
            >>> collection = client.create_collection(
            ...     name="documents",
            ...     dimension=384,
            ...     algorithm="hnsw",
            ...     m=16,
            ...     ef_construction=200
            ... )
        """
    
    def get_collection(self, name: str) -> 'Collection':
        """Get existing collection by name"""
    
    def list_collections(self) -> List['CollectionInfo']:
        """List all collections"""
    
    def delete_collection(self, name: str) -> bool:
        """Delete a collection"""
    
    def get_stats(self) -> 'PerformanceStats':
        """Get system performance statistics"""
```

#### Collection

```python
class Collection:
    """Vector collection interface"""
    
    def __init__(self, client: VexFSClient, collection_id: int, info: 'CollectionInfo'):
        self.client = client
        self.collection_id = collection_id
        self.info = info
    
    def insert(
        self,
        vector: Union[np.ndarray, List[float]],
        metadata: Optional[Dict] = None,
        file_path: Optional[str] = None
    ) -> 'InsertResult':
        """
        Insert a single vector
        
        Args:
            vector: Vector data (dimension must match collection)
            metadata: Optional metadata dictionary
            file_path: Optional path to associated file
            
        Returns:
            InsertResult with assigned vector ID
            
        Example:
            >>> vector = np.random.random(384).astype(np.float32)
            >>> result = collection.insert(
            ...     vector=vector,
            ...     metadata={"title": "Document 1", "category": "tech"}
            ... )
            >>> print(f"Inserted vector ID: {result.id}")
        """
    
    def insert_batch(
        self,
        vectors: Union[np.ndarray, List[List[float]]],
        metadata: Optional[List[Dict]] = None,
        batch_size: int = 1000,
        show_progress: bool = False
    ) -> List['InsertResult']:
        """
        Insert multiple vectors efficiently
        
        Args:
            vectors: Array of vectors (shape: [N, dimension])
            metadata: List of metadata dictionaries
            batch_size: Processing batch size
            show_progress: Show progress bar
            
        Returns:
            List of InsertResult objects
            
        Example:
            >>> vectors = np.random.random((10000, 384)).astype(np.float32)
            >>> metadata = [{"id": i} for i in range(10000)]
            >>> results = collection.insert_batch(
            ...     vectors=vectors,
            ...     metadata=metadata,
            ...     batch_size=1000,
            ...     show_progress=True
            ... )
        """
    
    def search(
        self,
        vector: Union[np.ndarray, List[float]],
        limit: int = 10,
        filter: Optional[Dict] = None,
        distance_metric: Optional[str] = None,
        ef_search: Optional[int] = None,
        max_distance: Optional[float] = None
    ) -> List['SearchResult']:
        """
        Search for similar vectors
        
        Args:
            vector: Query vector
            limit: Maximum number of results
            filter: Metadata filter (MongoDB-style)
            distance_metric: Override collection distance metric
            ef_search: HNSW search parameter
            max_distance: Maximum distance threshold
            
        Returns:
            List of SearchResult objects
            
        Example:
            >>> query = np.random.random(384).astype(np.float32)
            >>> results = collection.search(
            ...     vector=query,
            ...     limit=10,
            ...     filter={"category": "tech"},
            ...     ef_search=200
            ... )
            >>> for result in results:
            ...     print(f"ID: {result.id}, Distance: {result.distance:.4f}")
        """
    
    def search_range(
        self,
        vector: Union[np.ndarray, List[float]],
        max_distance: float,
        filter: Optional[Dict] = None
    ) -> List['SearchResult']:
        """Search for vectors within distance threshold"""
    
    def get_vector(self, vector_id: int) -> Optional['VectorData']:
        """Retrieve vector by ID"""
    
    def delete_vector(self, vector_id: int) -> bool:
        """Delete vector by ID"""
    
    def update_metadata(self, vector_id: int, metadata: Dict) -> bool:
        """Update vector metadata"""
    
    def get_stats(self) -> 'CollectionStats':
        """Get collection statistics"""
    
    def reindex(self, **params) -> bool:
        """Rebuild collection index with new parameters"""
```

### Data Classes

```python
from dataclasses import dataclass
from typing import Any, Dict, Optional
import numpy as np

@dataclass
class CollectionInfo:
    """Collection information"""
    id: int
    name: str
    dimension: int
    algorithm: str
    distance_metric: str
    vector_count: int
    created_timestamp: int
    modified_timestamp: int
    memory_usage: int

@dataclass
class InsertResult:
    """Vector insertion result"""
    id: int
    success: bool
    error_message: Optional[str] = None

@dataclass
class SearchResult:
    """Vector search result"""
    id: int
    distance: float
    metadata: Dict[str, Any]
    vector: Optional[np.ndarray] = None

@dataclass
class VectorData:
    """Complete vector data"""
    id: int
    vector: np.ndarray
    metadata: Dict[str, Any]
    timestamp: int

@dataclass
class PerformanceStats:
    """System performance statistics"""
    total_vectors: int
    total_searches: int
    total_insertions: int
    cache_hits: int
    cache_misses: int
    avg_search_time_ms: float
    avg_insert_time_ms: float
    memory_usage_mb: int

@dataclass
class CollectionStats:
    """Collection-specific statistics"""
    collection_id: int
    vector_count: int
    search_count: int
    insert_count: int
    avg_vector_norm: float
    std_vector_norm: float
    index_size_mb: int
```

### Advanced Features

#### Filtering

```python
# MongoDB-style filtering
results = collection.search(
    vector=query_vector,
    filter={
        "category": "documents",
        "score": {"$gte": 0.5},
        "tags": {"$in": ["important", "urgent"]},
        "$and": [
            {"created_date": {"$gte": "2025-01-01"}},
            {"author": {"$ne": "anonymous"}}
        ]
    }
)

# Supported operators
filter_examples = {
    # Equality
    "field": "value",
    
    # Comparison
    "score": {"$gt": 0.5},      # Greater than
    "score": {"$gte": 0.5},     # Greater than or equal
    "score": {"$lt": 0.9},      # Less than
    "score": {"$lte": 0.9},     # Less than or equal
    "score": {"$ne": 0.0},      # Not equal
    
    # Array operations
    "tags": {"$in": ["tag1", "tag2"]},        # In array
    "tags": {"$nin": ["exclude1", "exclude2"]}, # Not in array
    
    # Logical operations
    "$and": [{"field1": "value1"}, {"field2": "value2"}],
    "$or": [{"field1": "value1"}, {"field2": "value2"}],
    "$not": {"field": "value"},
    
    # Range operations
    "score": {"$between": [0.3, 0.8]},
    
    # Existence
    "field": {"$exists": True},
    "field": {"$exists": False}
}
```

#### Batch Operations

```python
# Streaming insertion for large datasets
def vector_generator():
    for i in range(1000000):
        vector = generate_embedding(f"document_{i}")
        metadata = {"id": i, "batch": i // 10000}
        yield vector, metadata

# Stream vectors with automatic batching
collection.insert_stream(
    vector_generator(),
    batch_size=5000,
    max_workers=4,
    show_progress=True
)

# Parallel batch search
queries = [np.random.random(384) for _ in range(100)]
results = collection.search_batch(
    vectors=queries,
    limit=10,
    max_workers=8
)
```

#### Configuration

```python
# Configure collection behavior
collection.configure(
    cache_size="2GB",
    batch_size=10000,
    ef_search=200,
    prefetch_enabled=True,
    compression_enabled=False
)

# Monitor performance
with collection.performance_monitor() as monitor:
    results = collection.search(query_vector, limit=100)
    
print(f"Search took {monitor.elapsed_ms:.2f}ms")
print(f"Cache hit rate: {monitor.cache_hit_rate:.2%}")
```

## 🔷 TypeScript SDK

### Installation

```bash
npm install @vexfs/sdk-v2
```

### Core Interfaces

```typescript
// Type definitions
export interface VectorData {
    id: number;
    vector: number[];
    metadata: Record<string, any>;
    timestamp: number;
}

export interface SearchResult {
    id: number;
    distance: number;
    metadata: Record<string, any>;
    vector?: number[];
}

export interface CollectionInfo {
    id: number;
    name: string;
    dimension: number;
    algorithm: 'hnsw' | 'lsh';
    distanceMetric: 'cosine' | 'euclidean' | 'manhattan';
    vectorCount: number;
    createdTimestamp: number;
    modifiedTimestamp: number;
    memoryUsage: number;
}

export interface CollectionOptions {
    name: string;
    dimension: number;
    algorithm?: 'hnsw' | 'lsh';
    distanceMetric?: 'cosine' | 'euclidean' | 'manhattan';
    parameters?: HNSWParameters | LSHParameters;
}

export interface HNSWParameters {
    m?: number;
    efConstruction?: number;
    efSearch?: number;
    maxM?: number;
    ml?: number;
}

export interface LSHParameters {
    numTables?: number;
    numFunctions?: number;
    bucketSize?: number;
    projectionType?: 'random' | 'learned';
}

export interface SearchOptions {
    limit?: number;
    filter?: Record<string, any>;
    distanceMetric?: string;
    efSearch?: number;
    maxDistance?: number;
}

export interface InsertOptions {
    metadata?: Record<string, any>;
    filePath?: string;
}

export interface BatchInsertOptions {
    batchSize?: number;
    showProgress?: boolean;
    maxWorkers?: number;
}
```

### VexFSClient Class

```typescript
export class VexFSClient {
    private mountPath: string;
    private timeout: number;
    
    constructor(mountPath: string, timeout: number = 30000) {
        this.mountPath = mountPath;
        this.timeout = timeout;
    }
    
    /**
     * Create a new vector collection
     */
    async createCollection(options: CollectionOptions): Promise<Collection> {
        // Implementation
    }
    
    /**
     * Get existing collection by name
     */
    async getCollection(name: string): Promise<Collection> {
        // Implementation
    }
    
    /**
     * List all collections
     */
    async listCollections(): Promise<CollectionInfo[]> {
        // Implementation
    }
    
    /**
     * Delete a collection
     */
    async deleteCollection(name: string): Promise<boolean> {
        // Implementation
    }
    
    /**
     * Get system performance statistics
     */
    async getStats(): Promise<PerformanceStats> {
        // Implementation
    }
}
```

### Collection Class

```typescript
export class Collection {
    private client: VexFSClient;
    private collectionId: number;
    private info: CollectionInfo;
    
    constructor(client: VexFSClient, collectionId: number, info: CollectionInfo) {
        this.client = client;
        this.collectionId = collectionId;
        this.info = info;
    }
    
    /**
     * Insert a single vector
     */
    async insert(vector: number[], options?: InsertOptions): Promise<InsertResult> {
        // Validate vector dimension
        if (vector.length !== this.info.dimension) {
            throw new Error(`Vector dimension ${vector.length} doesn't match collection dimension ${this.info.dimension}`);
        }
        
        // Implementation
    }
    
    /**
     * Insert multiple vectors efficiently
     */
    async insertBatch(
        vectors: number[][],
        metadata?: Record<string, any>[],
        options?: BatchInsertOptions
    ): Promise<InsertResult[]> {
        // Implementation
    }
    
    /**
     * Search for similar vectors
     */
    async search(vector: number[], options?: SearchOptions): Promise<SearchResult[]> {
        // Implementation
    }
    
    /**
     * Search for vectors within distance threshold
     */
    async searchRange(vector: number[], maxDistance: number, filter?: Record<string, any>): Promise<SearchResult[]> {
        // Implementation
    }
    
    /**
     * Get vector by ID
     */
    async getVector(vectorId: number): Promise<VectorData | null> {
        // Implementation
    }
    
    /**
     * Delete vector by ID
     */
    async deleteVector(vectorId: number): Promise<boolean> {
        // Implementation
    }
    
    /**
     * Update vector metadata
     */
    async updateMetadata(vectorId: number, metadata: Record<string, any>): Promise<boolean> {
        // Implementation
    }
    
    /**
     * Get collection statistics
     */
    async getStats(): Promise<CollectionStats> {
        // Implementation
    }
    
    /**
     * Rebuild collection index
     */
    async reindex(parameters?: HNSWParameters | LSHParameters): Promise<boolean> {
        // Implementation
    }
}
```

### Usage Examples

```typescript
import { VexFSClient } from '@vexfs/sdk-v2';

async function example() {
    // Connect to VexFS
    const client = new VexFSClient('/mnt/vexfs');
    
    // Create collection
    const collection = await client.createCollection({
        name: 'documents',
        dimension: 384,
        algorithm: 'hnsw',
        parameters: {
            m: 16,
            efConstruction: 200,
            efSearch: 100
        }
    });
    
    // Insert vectors
    const vectors = Array.from({ length: 1000 }, () => 
        Array.from({ length: 384 }, () => Math.random())
    );
    
    const metadata = Array.from({ length: 1000 }, (_, i) => ({
        id: i,
        category: `category_${i % 10}`,
        timestamp: Date.now()
    }));
    
    await collection.insertBatch(vectors, metadata, {
        batchSize: 100,
        showProgress: true
    });
    
    // Search
    const queryVector = Array.from({ length: 384 }, () => Math.random());
    const results = await collection.search(queryVector, {
        limit: 10,
        filter: { category: 'category_5' },
        efSearch: 200
    });
    
    console.log(`Found ${results.length} similar vectors`);
    results.forEach(result => {
        console.log(`ID: ${result.id}, Distance: ${result.distance.toFixed(4)}`);
    });
}
```

## 🖥️ CLI Interface (vexctl)

### Installation

```bash
# Install from source
cd vexctl
cargo build --release
sudo cp target/release/vexctl /usr/local/bin/

# Or use pre-built binary
wget https://github.com/lspecian/vexfs/releases/latest/download/vexctl
chmod +x vexctl
sudo mv vexctl /usr/local/bin/
```

### Commands

#### Collection Management

```bash
# Create collection
vexctl collection create my_collection \
    --dimension 384 \
    --algorithm hnsw \
    --distance-metric cosine \
    --hnsw-m 16 \
    --hnsw-ef-construction 200

# List collections
vexctl collection list

# Get collection info
vexctl collection info my_collection

# Delete collection
vexctl collection delete my_collection
```

#### Vector Operations

```bash
# Insert single vector
vexctl vector insert my_collection \
    --vector '[0.1, 0.2, 0.3, ...]' \
    --metadata '{"title": "Document 1", "category": "tech"}'

# Insert from file
vexctl vector insert my_collection \
    --file vectors.json \
    --batch-size 1000

# Search vectors
vexctl vector search my_collection \
    --vector '[0.1, 0.2, 0.3, ...]' \
    --limit 10 \
    --filter '{"category": "tech"}' \
    --ef-search 200

# Get vector by ID
vexctl vector get my_collection --id 12345

# Delete vector
vexctl vector delete my_collection --id 12345
```

#### System Operations

```bash
# Get system statistics
vexctl stats

# Get collection statistics
vexctl stats --collection my_collection

# Monitor performance
vexctl monitor --interval 1s

# Health check
vexctl health
```

### Configuration

```bash
# Set default mount path
vexctl config set mount-path /mnt/vexfs

# Set default timeout
vexctl config set timeout 30s

# View configuration
vexctl config show
```

## 🌐 REST API (Future)

### Endpoints

```http
# Collection management
POST   /api/v2/collections
GET    /api/v2/collections
GET    /api/v2/collections/{name}
DELETE /api/v2/collections/{name}

# Vector operations
POST   /api/v2/collections/{name}/vectors
POST   /api/v2/collections/{name}/vectors/batch
POST   /api/v2/collections/{name}/search
GET    /api/v2/collections/{name}/vectors/{id}
PUT    /api/v2/collections/{name}/vectors/{id}
DELETE /api/v2/collections/{name}/vectors/{id}

# Statistics
GET    /api/v2/stats
GET    /api/v2/collections/{name}/stats
```

### Example Requests

```http
# Create collection
POST /api/v2/collections
Content-Type: application/json

{
    "name": "documents",
    "dimension": 384,
    "algorithm": "hnsw",
    "distance_metric": "cosine",
    "parameters": {
        "m": 16,
        "ef_construction": 200,
        "ef_search": 100
    }
}

# Insert vector
POST /api/v2/collections/documents/vectors
Content-Type: application/json

{
    "vector": [0.1, 0.2, 0.3, ...],
    "metadata": {
        "title": "Document 1",
        "category": "tech"
    }
}

# Search vectors
POST /api/v2/collections/documents/search
Content-Type: application/json

{
    "vector": [0.1, 0.2, 0.3, ...],
    "limit": 10,
    "filter": {
        "category": "tech"
    },
    "ef_search": 200
}
```

## 🔧 Error Handling

### Error Codes

```c
// VexFS error codes
#define VEXFS_SUCCESS                0
#define VEXFS_ERROR_INVALID_PARAM   -1
#define VEXFS_ERROR_NO_MEMORY       -2
#define VEXFS_ERROR_NOT_FOUND       -3
#define VEXFS_ERROR_EXISTS          -4
#define VEXFS_ERROR_PERMISSION      -5
#define VEXFS_ERROR_DIMENSION       -6
#define VEXFS_ERROR_ALGORITHM       -7
#define VEXFS_ERROR_TIMEOUT         -8
#define VEXFS_ERROR_CORRUPTION      -9
#define VEXFS_ERROR_FULL           -10
```

### Python Exceptions

```python
# VexFS exception hierarchy
class VexFSError(Exception):
    """Base VexFS exception"""
    pass

class VectorDimensionError(VexFSError):
    """Vector dimension mismatch"""
    pass

class CollectionNotFoundError(VexFSError):
    """Collection not found"""
    pass

class SearchTimeoutError(VexFSError):
    """Search operation timed out"""
    pass

class InsufficientMemoryError(VexFSError):
    """Insufficient memory for operation"""
    pass

# Usage
try:
    results = collection.search(query_vector, limit=10)
except VectorDimensionError as e:
    print(f"Dimension error: {e}")
except SearchTimeoutError as e:
    print