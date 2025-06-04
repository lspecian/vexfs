# VexFS v2.0 Usage Guide

This comprehensive guide covers all aspects of using VexFS v2.0, from basic filesystem operations to advanced vector search capabilities.

## 🗂️ Basic Filesystem Operations

### File and Directory Management

VexFS v2.0 works like any standard filesystem while providing vector capabilities:

```bash
# Basic file operations
echo "Hello World" > /mnt/vexfs/hello.txt
cat /mnt/vexfs/hello.txt
cp /mnt/vexfs/hello.txt /mnt/vexfs/hello_copy.txt
mv /mnt/vexfs/hello_copy.txt /mnt/vexfs/renamed.txt
rm /mnt/vexfs/renamed.txt

# Directory operations
mkdir /mnt/vexfs/my_directory
ls -la /mnt/vexfs/
rmdir /mnt/vexfs/my_directory

# Permissions work normally
chmod 755 /mnt/vexfs/hello.txt
chown user:group /mnt/vexfs/hello.txt
```

### Extended Attributes for Vectors

VexFS v2.0 stores vector data as extended attributes:

```bash
# View vector attributes
getfattr -d /mnt/vexfs/document.txt

# Example output:
# user.vexfs.vector.embedding=[0.1,0.2,0.3,...]
# user.vexfs.vector.dimension=384
# user.vexfs.vector.algorithm=hnsw
```

## 🔍 Vector Operations

### Collection Management

Collections organize vectors with similar characteristics:

```python
import vexfs

# Connect to VexFS
client = vexfs.Client('/mnt/vexfs')

# Create a collection
collection = client.create_collection(
    name="documents",
    dimension=384,
    algorithm="hnsw",
    distance_metric="cosine",
    parameters={
        "m": 16,
        "ef_construction": 200,
        "ef_search": 100
    }
)

# List collections
collections = client.list_collections()
print(f"Available collections: {[c.name for c in collections]}")

# Get collection info
info = collection.info()
print(f"Collection: {info.name}")
print(f"Vectors: {info.vector_count}")
print(f"Dimension: {info.dimension}")
print(f"Algorithm: {info.algorithm}")
```

### Vector Insertion

#### Single Vector Insertion

```python
import numpy as np

# Insert a single vector
vector = np.random.random(384).astype(np.float32)
metadata = {
    "id": "doc_001",
    "title": "Introduction to VexFS",
    "category": "documentation",
    "timestamp": "2025-06-04T10:00:00Z"
}

result = collection.insert(
    vector=vector,
    metadata=metadata,
    file_path="/mnt/vexfs/documents/intro.txt"  # Optional: link to file
)

print(f"Inserted vector with ID: {result.id}")
```

#### Batch Vector Insertion

```python
# Insert multiple vectors efficiently
vectors = np.random.random((10000, 384)).astype(np.float32)
metadata_list = [
    {
        "id": f"doc_{i:06d}",
        "category": f"category_{i % 10}",
        "score": np.random.random()
    }
    for i in range(10000)
]

# Batch insert with progress tracking
results = collection.insert_batch(
    vectors=vectors,
    metadata=metadata_list,
    batch_size=1000,
    show_progress=True
)

print(f"Inserted {len(results)} vectors")
```

#### Streaming Insertion

```python
# For very large datasets
def vector_generator():
    for i in range(1000000):
        vector = np.random.random(384).astype(np.float32)
        metadata = {"id": f"stream_{i}", "batch": i // 10000}
        yield vector, metadata

# Stream vectors to collection
collection.insert_stream(
    vector_generator(),
    batch_size=5000,
    max_workers=4
)
```

### Vector Search

#### Basic Similarity Search

```python
# Search for similar vectors
query_vector = np.random.random(384).astype(np.float32)

results = collection.search(
    vector=query_vector,
    limit=10,
    distance_metric="cosine"
)

for result in results:
    print(f"ID: {result.metadata['id']}")
    print(f"Distance: {result.distance:.4f}")
    print(f"Metadata: {result.metadata}")
    print("---")
```

#### Filtered Search

```python
# Search with metadata filters
results = collection.search(
    vector=query_vector,
    limit=20,
    filter={
        "category": "documentation",
        "score": {"$gte": 0.5}
    }
)

# Complex filters
results = collection.search(
    vector=query_vector,
    limit=10,
    filter={
        "$and": [
            {"category": {"$in": ["docs", "tutorials"]}},
            {"timestamp": {"$gte": "2025-01-01"}},
            {"score": {"$between": [0.3, 0.9]}}
        ]
    }
)
```

#### Range Search

```python
# Find all vectors within a distance threshold
results = collection.search_range(
    vector=query_vector,
    max_distance=0.3,
    distance_metric="euclidean"
)

print(f"Found {len(results)} vectors within distance 0.3")
```

#### Hybrid Search

```python
# Combine vector similarity with text search
results = collection.hybrid_search(
    vector=query_vector,
    text_query="machine learning tutorial",
    vector_weight=0.7,
    text_weight=0.3,
    limit=15
)
```

## 🚀 Advanced Features

### Multi-Vector Documents

Store multiple vectors per document:

```python
# Document with multiple vector representations
document_vectors = {
    "title_embedding": np.random.random(384).astype(np.float32),
    "content_embedding": np.random.random(768).astype(np.float32),
    "summary_embedding": np.random.random(256).astype(np.float32)
}

collection.insert_multi_vector(
    vectors=document_vectors,
    metadata={"id": "multi_doc_001", "type": "article"},
    file_path="/mnt/vexfs/articles/article_001.txt"
)

# Search across multiple vector types
results = collection.search_multi_vector(
    vectors={
        "title_embedding": title_query_vector,
        "content_embedding": content_query_vector
    },
    weights={"title_embedding": 0.3, "content_embedding": 0.7},
    limit=10
)
```

### Vector Clustering

```python
# Cluster vectors for analysis
clusters = collection.cluster(
    algorithm="kmeans",
    num_clusters=50,
    sample_size=10000
)

# Get cluster information
for cluster in clusters:
    print(f"Cluster {cluster.id}: {cluster.size} vectors")
    print(f"Centroid: {cluster.centroid[:5]}...")
    print(f"Representative docs: {cluster.representative_ids}")
```

### Vector Analytics

```python
# Analyze vector distribution
stats = collection.analyze()
print(f"Vector count: {stats.count}")
print(f"Dimension: {stats.dimension}")
print(f"Average norm: {stats.avg_norm:.4f}")
print(f"Std deviation: {stats.std_norm:.4f}")

# Distance distribution
dist_stats = collection.distance_distribution(sample_size=1000)
print(f"Average pairwise distance: {dist_stats.mean:.4f}")
print(f"Distance std: {dist_stats.std:.4f}")
```

## 🔧 Performance Optimization

### Indexing Strategies

#### HNSW Configuration

```python
# High-performance configuration
collection = client.create_collection(
    name="high_perf",
    dimension=768,
    algorithm="hnsw",
    parameters={
        "m": 32,                    # More connections = better recall
        "ef_construction": 400,     # Higher = better index quality
        "ef_search": 200,          # Higher = better search quality
        "max_m": 64,               # Maximum connections
        "ml": 1/np.log(2.0)        # Level generation factor
    }
)
```

#### LSH Configuration

```python
# Memory-efficient configuration
collection = client.create_collection(
    name="memory_efficient",
    dimension=384,
    algorithm="lsh",
    parameters={
        "num_tables": 20,          # More tables = better recall
        "num_functions": 30,       # More functions = better precision
        "bucket_size": 100,        # Bucket size for efficiency
        "projection_type": "random" # or "learned"
    }
)
```

### Batch Operations

```python
# Optimize batch sizes for your hardware
collection.configure_batching(
    insert_batch_size=10000,    # Larger batches for insertion
    search_batch_size=100,      # Smaller batches for search
    max_memory_usage="4GB"      # Memory limit
)

# Parallel processing
collection.set_parallelism(
    insert_workers=8,           # Parallel insertion threads
    search_workers=4,           # Parallel search threads
    io_workers=2               # I/O operation threads
)
```

### Caching

```python
# Configure vector caching
collection.configure_cache(
    vector_cache_size="2GB",    # Cache for vectors
    index_cache_size="1GB",     # Cache for index structures
    metadata_cache_size="512MB", # Cache for metadata
    cache_policy="lru"          # LRU, LFU, or adaptive
)

# Preload frequently accessed vectors
collection.preload_vectors(
    filter={"category": "frequently_accessed"},
    priority="high"
)
```

## 📊 Monitoring and Debugging

### Performance Metrics

```python
# Get collection performance metrics
metrics = collection.get_metrics()
print(f"Search latency (avg): {metrics.search_latency_ms:.2f}ms")
print(f"Insert throughput: {metrics.insert_throughput:.0f} vectors/sec")
print(f"Cache hit rate: {metrics.cache_hit_rate:.2%}")
print(f"Memory usage: {metrics.memory_usage_mb:.0f}MB")

# Real-time monitoring
def monitor_performance():
    while True:
        metrics = collection.get_metrics()
        print(f"QPS: {metrics.queries_per_second:.0f}, "
              f"Latency: {metrics.avg_latency_ms:.1f}ms")
        time.sleep(1)
```

### Query Analysis

```python
# Analyze query patterns
query_stats = collection.analyze_queries(time_window="1h")
print(f"Total queries: {query_stats.total_queries}")
print(f"Average results per query: {query_stats.avg_results:.1f}")
print(f"Most common filters: {query_stats.common_filters}")

# Query profiling
with collection.profile_query() as profiler:
    results = collection.search(query_vector, limit=10)

print(f"Query breakdown:")
print(f"  Index lookup: {profiler.index_time_ms:.2f}ms")
print(f"  Distance computation: {profiler.distance_time_ms:.2f}ms")
print(f"  Metadata retrieval: {profiler.metadata_time_ms:.2f}ms")
```

### Debugging

```python
# Enable debug logging
import logging
logging.basicConfig(level=logging.DEBUG)

# Validate vector data
validation_result = collection.validate()
if not validation_result.is_valid:
    print("Issues found:")
    for issue in validation_result.issues:
        print(f"  {issue.severity}: {issue.message}")

# Check index integrity
integrity_check = collection.check_integrity()
print(f"Index integrity: {'OK' if integrity_check.passed else 'FAILED'}")
```

## 🔄 Data Management

### Backup and Restore

```python
# Backup collection
backup_path = "/backup/vexfs/documents_backup"
collection.backup(
    path=backup_path,
    include_vectors=True,
    include_metadata=True,
    compression="gzip"
)

# Restore collection
restored_collection = client.restore_collection(
    backup_path=backup_path,
    new_name="documents_restored"
)
```

### Data Migration

```python
# Migrate from another vector database
from vexfs.migration import ChromaDBMigrator

migrator = ChromaDBMigrator(
    source_path="/path/to/chromadb",
    target_collection=collection
)

migrator.migrate(
    batch_size=5000,
    preserve_ids=True,
    show_progress=True
)
```

### Data Cleanup

```python
# Remove duplicate vectors
duplicates = collection.find_duplicates(threshold=0.01)
print(f"Found {len(duplicates)} duplicate groups")

# Remove duplicates (keep first occurrence)
collection.remove_duplicates(keep="first")

# Compact collection (reclaim space)
collection.compact()

# Reindex for better performance
collection.reindex(
    algorithm="hnsw",
    parameters={"m": 32, "ef_construction": 400}
)
```

## 🔗 Integration Patterns

### File System Integration

```python
# Automatically index files as they're added
import os
from watchdog.observers import Observer
from watchdog.events import FileSystemEventHandler

class VexFSHandler(FileSystemEventHandler):
    def __init__(self, collection):
        self.collection = collection
    
    def on_created(self, event):
        if event.is_file and event.src_path.endswith('.txt'):
            # Extract text and generate embedding
            with open(event.src_path, 'r') as f:
                text = f.read()
            
            # Generate embedding (using your preferred model)
            vector = generate_embedding(text)
            
            # Insert into VexFS
            self.collection.insert(
                vector=vector,
                metadata={"file_path": event.src_path, "type": "text"},
                file_path=event.src_path
            )

# Watch directory for changes
observer = Observer()
observer.schedule(VexFSHandler(collection), "/mnt/vexfs/documents", recursive=True)
observer.start()
```

### API Integration

```python
# RESTful API wrapper
from flask import Flask, request, jsonify

app = Flask(__name__)

@app.route('/search', methods=['POST'])
def search_vectors():
    data = request.json
    query_vector = np.array(data['vector'])
    
    results = collection.search(
        vector=query_vector,
        limit=data.get('limit', 10),
        filter=data.get('filter', {})
    )
    
    return jsonify([{
        'id': r.metadata['id'],
        'distance': float(r.distance),
        'metadata': r.metadata
    } for r in results])

@app.route('/insert', methods=['POST'])
def insert_vector():
    data = request.json
    vector = np.array(data['vector'])
    
    result = collection.insert(
        vector=vector,
        metadata=data['metadata']
    )
    
    return jsonify({'id': result.id, 'status': 'success'})
```

## 🎯 Best Practices

### Vector Quality

```python
# Normalize vectors for consistent similarity computation
def normalize_vector(vector):
    norm = np.linalg.norm(vector)
    return vector / norm if norm > 0 else vector

# Validate vector dimensions
def validate_vector(vector, expected_dim):
    if len(vector) != expected_dim:
        raise ValueError(f"Expected {expected_dim} dimensions, got {len(vector)}")
    
    if not np.isfinite(vector).all():
        raise ValueError("Vector contains NaN or infinite values")

# Use consistent data types
vectors = vectors.astype(np.float32)  # Use float32 for memory efficiency
```

### Metadata Design

```python
# Design efficient metadata schemas
metadata_schema = {
    "id": str,              # Unique identifier
    "category": str,        # For filtering
    "timestamp": datetime,  # For temporal queries
    "score": float,         # For range queries
    "tags": list,          # For multi-value filtering
    "embedding_model": str  # Track model versions
}

# Index frequently filtered fields
collection.create_metadata_index(["category", "timestamp"])
```

### Error Handling

```python
import vexfs.exceptions as vex

try:
    results = collection.search(query_vector, limit=10)
except vex.VectorDimensionError as e:
    print(f"Dimension mismatch: {e}")
except vex.CollectionNotFoundError as e:
    print(f"Collection not found: {e}")
except vex.SearchTimeoutError as e:
    print(f"Search timed out: {e}")
except vex.VexFSError as e:
    print(f"General VexFS error: {e}")
```

## 📚 Next Steps

Now that you understand VexFS v2.0 usage:

1. **[Vector Search Tutorial](../tutorials/vector-search.md)** - Advanced search techniques
2. **[Performance Tuning](../reference/performance.md)** - Optimize for your workload
3. **[API Reference](../developer-guide/api-reference.md)** - Complete API documentation
4. **[Integration Examples](../tutorials/integration.md)** - Real-world integration patterns
5. **[Troubleshooting](troubleshooting.md)** - Common issues and solutions

**Ready to build?** Explore our [tutorial collection](../tutorials/) for hands-on examples!

---

**VexFS v2.0** - Powerful vector operations with familiar filesystem semantics! 🚀