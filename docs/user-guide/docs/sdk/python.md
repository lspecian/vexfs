# Python SDK Documentation

Complete API reference and usage guide for the VexFS Python SDK - high-performance vector operations with Python simplicity.

## 🚀 Overview

The VexFS Python SDK provides native Rust-powered performance with a Pythonic API. Built with PyO3, it delivers zero-copy data handling and seamless integration with the Python ecosystem.

### Key Features

- **🔥 Native Performance**: Direct Rust integration with zero-copy operations
- **🐍 Pythonic API**: Clean, intuitive interface following Python conventions
- **📊 Data Science Ready**: Works seamlessly with NumPy, Pandas, and scikit-learn
- **🧠 AI/ML Optimized**: Perfect for RAG systems and ML pipelines
- **🔒 Memory Safe**: Rust's ownership system prevents vulnerabilities

## 📦 Installation

### From PyPI (Recommended)

```bash
pip install vexfs
```

### Development Installation

```bash
# Clone repository
git clone https://github.com/lspecian/vexfs.git
cd vexfs/bindings/python

# Install maturin
pip install maturin

# Build and install in development mode
maturin develop

# Or build wheel for distribution
maturin build --release
pip install target/wheels/vexfs-*.whl
```

### Virtual Environment Setup

```bash
# Create virtual environment
python -m venv vexfs-env
source vexfs-env/bin/activate  # Windows: vexfs-env\Scripts\activate

# Install VexFS with ML dependencies
pip install vexfs numpy pandas sentence-transformers scikit-learn
```

## 🎯 Quick Start

```python
import vexfs
import numpy as np

# Initialize VexFS
vexfs.init("/mnt/vexfs")

# Add document with metadata
doc_id = vexfs.add(
    "VexFS provides high-performance vector search",
    {"category": "technology", "type": "description"}
)

# Query with vector
query_vector = np.random.rand(384).tolist()
results = vexfs.query(query_vector, top_k=5)

# Process results
for doc_id, score, text in results:
    print(f"ID: {doc_id}, Score: {score:.4f}")
```

## 📚 API Reference

### Core Functions

#### `vexfs.init(mount_point: str) -> None`

Initialize VexFS with the specified mount point.

**Parameters:**
- `mount_point` (str): Path to the VexFS mount point

**Example:**
```python
import vexfs

# Initialize with mount point
vexfs.init("/mnt/vexfs")

# Or use environment variable
import os
vexfs.init(os.getenv("VEXFS_MOUNT_POINT", "/mnt/vexfs"))
```

#### `vexfs.add(text: str, metadata: Optional[Dict[str, str]] = None) -> str`

Add a text document to VexFS with optional metadata.

**Parameters:**
- `text` (str): The text content to store and index
- `metadata` (Dict[str, str], optional): Key-value pairs for document metadata

**Returns:**
- `str`: Unique document identifier

**Example:**
```python
# Simple addition
doc_id = vexfs.add("Machine learning is fascinating")

# With metadata
doc_id = vexfs.add(
    "Python is a versatile programming language",
    {
        "category": "programming",
        "language": "python",
        "difficulty": "beginner",
        "tags": "tutorial,basics",
        "author": "John Doe",
        "created_at": "2025-01-15"
    }
)
```

#### `vexfs.query(vector: List[float], top_k: int = 10, metric: str = "euclidean") -> List[Tuple[str, float, str]]`

Search for similar documents using vector similarity.

**Parameters:**
- `vector` (List[float]): Query vector for similarity search
- `top_k` (int, optional): Maximum number of results to return (default: 10)
- `metric` (str, optional): Similarity metric ("euclidean", "cosine", "inner_product")

**Returns:**
- `List[Tuple[str, float, str]]`: List of (document_id, score, text) tuples

**Example:**
```python
import numpy as np

# Create query vector
query_vector = np.random.rand(384).tolist()

# Basic search
results = vexfs.query(query_vector, top_k=5)

# Search with different metrics
euclidean_results = vexfs.query(query_vector, top_k=5, metric="euclidean")
cosine_results = vexfs.query(query_vector, top_k=5, metric="cosine")
inner_product_results = vexfs.query(query_vector, top_k=5, metric="inner_product")

# Process results
for doc_id, score, text in results:
    print(f"Document: {doc_id}")
    print(f"Similarity: {score:.4f}")
    print(f"Content: {text[:100]}...")
    print("---")
```

#### `vexfs.delete(doc_id: str) -> None`

Remove a document from VexFS.

**Parameters:**
- `doc_id` (str): Document identifier to delete

**Example:**
```python
# Delete single document
vexfs.delete("doc_12345")

# Delete multiple documents
doc_ids = ["doc_1", "doc_2", "doc_3"]
for doc_id in doc_ids:
    vexfs.delete(doc_id)
```

### Advanced Functions

#### `vexfs.query_with_filter(vector: List[float], filters: Dict[str, Any], top_k: int = 10) -> List[Tuple[str, float, str]]`

Search with metadata filtering.

**Parameters:**
- `vector` (List[float]): Query vector
- `filters` (Dict[str, Any]): Metadata filters
- `top_k` (int, optional): Maximum results

**Example:**
```python
# Search with filters
results = vexfs.query_with_filter(
    vector=query_vector,
    filters={
        "category": "technology",
        "difficulty": ["beginner", "intermediate"],
        "created_after": "2025-01-01"
    },
    top_k=10
)
```

#### `vexfs.add_batch(documents: List[Tuple[str, Optional[Dict[str, str]]]]) -> List[str]`

Add multiple documents efficiently.

**Parameters:**
- `documents` (List[Tuple[str, Optional[Dict[str, str]]]]): List of (text, metadata) tuples

**Returns:**
- `List[str]`: List of document IDs

**Example:**
```python
# Batch addition
documents = [
    ("Document 1 content", {"type": "article", "topic": "AI"}),
    ("Document 2 content", {"type": "blog", "topic": "ML"}),
    ("Document 3 content", {"type": "paper", "topic": "NLP"}),
]

doc_ids = vexfs.add_batch(documents)
print(f"Added {len(doc_ids)} documents")
```

#### `vexfs.stats() -> Dict[str, Any]`

Get VexFS statistics and performance metrics.

**Returns:**
- `Dict[str, Any]`: Statistics dictionary

**Example:**
```python
stats = vexfs.stats()
print(f"Total documents: {stats['document_count']}")
print(f"Total vectors: {stats['vector_count']}")
print(f"Cache hit rate: {stats['cache_hit_rate']:.2%}")
print(f"Memory usage: {stats['memory_usage_mb']} MB")
print(f"Index size: {stats['index_size_mb']} MB")
```

### Client Class API

#### `VexFSClient`

Object-oriented interface for VexFS operations.

```python
from vexfs import VexFSClient

# Initialize client
client = VexFSClient(mount_point="/mnt/vexfs")

# Or with configuration
client = VexFSClient(
    mount_point="/mnt/vexfs",
    cache_size="2GB",
    vector_dimension=384
)
```

**Methods:**

##### `client.add(text: str, metadata: Optional[Dict[str, str]] = None) -> str`

```python
doc_id = client.add("Sample text", {"type": "example"})
```

##### `client.query(vector: List[float], top_k: int = 10) -> List[Tuple[str, float, str]]`

```python
results = client.query(query_vector, top_k=5)
```

##### `client.delete(doc_id: str) -> None`

```python
client.delete("doc_12345")
```

##### `client.stats() -> Dict[str, Any]`

```python
stats = client.stats()
```

##### `client.configure(config: Dict[str, Any]) -> None`

```python
client.configure({
    "cache_size": "4GB",
    "worker_threads": 8,
    "batch_size": 1000
})
```

## 🔧 Advanced Usage

### Working with Embeddings

#### Sentence Transformers Integration

```python
import vexfs
from sentence_transformers import SentenceTransformer

# Initialize embedding model
model = SentenceTransformer('all-MiniLM-L6-v2')

class VexFSEmbeddingStore:
    def __init__(self, mount_point: str):
        vexfs.init(mount_point)
        self.model = model
    
    def add_document(self, text: str, metadata: Optional[Dict[str, str]] = None) -> str:
        """Add document with automatic embedding generation"""
        # Generate embedding
        embedding = self.model.encode(text).tolist()
        
        # Store in VexFS (embedding handled internally)
        return vexfs.add(text, metadata)
    
    def semantic_search(self, query_text: str, top_k: int = 10) -> List[Tuple[str, float, str]]:
        """Perform semantic search using text query"""
        # Generate query embedding
        query_embedding = self.model.encode(query_text).tolist()
        
        # Search VexFS
        return vexfs.query(query_embedding, top_k=top_k)

# Usage
store = VexFSEmbeddingStore("/mnt/vexfs")

# Add documents
doc_id = store.add_document(
    "VexFS provides high-performance vector search",
    {"topic": "filesystem", "performance": "high"}
)

# Semantic search
results = store.semantic_search("fast vector database", top_k=5)
```

#### Custom Embedding Models

```python
import vexfs
import numpy as np
from typing import List

class CustomEmbeddingModel:
    def __init__(self, model_path: str):
        # Load your custom model
        self.model = self.load_model(model_path)
    
    def encode(self, text: str) -> np.ndarray:
        # Your custom encoding logic
        return self.model.encode(text)
    
    def encode_batch(self, texts: List[str]) -> np.ndarray:
        # Batch encoding for efficiency
        return self.model.encode_batch(texts)

# Integration with VexFS
model = CustomEmbeddingModel("path/to/model")

def add_with_custom_embedding(text: str, metadata: Dict[str, str] = None) -> str:
    # Generate embedding with custom model
    embedding = model.encode(text).tolist()
    
    # Add to VexFS
    return vexfs.add(text, metadata)
```

### NumPy Integration

```python
import vexfs
import numpy as np

def numpy_vector_operations():
    """Demonstrate NumPy integration"""
    
    # Generate random vectors
    vectors = np.random.rand(1000, 384).astype(np.float32)
    
    # Add documents with NumPy vectors
    doc_ids = []
    for i, vector in enumerate(vectors):
        doc_id = vexfs.add(
            f"Document {i}",
            {"index": str(i), "vector_norm": str(np.linalg.norm(vector))}
        )
        doc_ids.append(doc_id)
    
    # Query with NumPy array
    query_vector = np.random.rand(384).astype(np.float32)
    query_list = query_vector.tolist()
    
    results = vexfs.query(query_list, top_k=10)
    
    # Analyze results with NumPy
    scores = np.array([score for _, score, _ in results])
    print(f"Mean score: {np.mean(scores):.4f}")
    print(f"Std score: {np.std(scores):.4f}")
    
    return results

# Vector similarity analysis
def analyze_vector_similarity(doc_ids: List[str], query_vector: np.ndarray):
    """Analyze vector similarities using NumPy"""
    
    # Get vectors for documents (if available)
    # This would require additional API to retrieve vectors
    
    # Compute similarities
    similarities = []
    for doc_id in doc_ids:
        # Retrieve document vector (hypothetical API)
        doc_vector = vexfs.get_vector(doc_id)  # Not implemented yet
        similarity = np.dot(query_vector, doc_vector)
        similarities.append(similarity)
    
    return np.array(similarities)
```

### Pandas Integration

```python
import vexfs
import pandas as pd
from sentence_transformers import SentenceTransformer

def index_dataframe(df: pd.DataFrame, text_column: str, metadata_columns: List[str] = None) -> pd.DataFrame:
    """Index a pandas DataFrame in VexFS"""
    
    model = SentenceTransformer('all-MiniLM-L6-v2')
    doc_ids = []
    
    for idx, row in df.iterrows():
        text = row[text_column]
        
        # Prepare metadata
        metadata = {"row_index": str(idx)}
        if metadata_columns:
            for col in metadata_columns:
                metadata[col] = str(row[col])
        
        # Add to VexFS
        doc_id = vexfs.add(text, metadata)
        doc_ids.append(doc_id)
    
    # Add doc_ids to DataFrame
    df['vexfs_id'] = doc_ids
    return df

def search_dataframe(df: pd.DataFrame, query: str, top_k: int = 10) -> pd.DataFrame:
    """Search DataFrame using VexFS"""
    
    model = SentenceTransformer('all-MiniLM-L6-v2')
    query_embedding = model.encode(query).tolist()
    
    # Search VexFS
    results = vexfs.query(query_embedding, top_k=top_k)
    
    # Convert to DataFrame
    result_df = pd.DataFrame(results, columns=['vexfs_id', 'score', 'text'])
    
    # Merge with original DataFrame
    merged_df = result_df.merge(df, on='vexfs_id', how='left')
    
    return merged_df.sort_values('score', ascending=False)

# Example usage
data = {
    'title': ['Article 1', 'Article 2', 'Article 3'],
    'content': ['Content about AI...', 'Content about ML...', 'Content about NLP...'],
    'category': ['tech', 'science', 'tech'],
    'author': ['Alice', 'Bob', 'Charlie']
}

df = pd.DataFrame(data)
df_indexed = index_dataframe(df, 'content', ['title', 'category', 'author'])

# Search
search_results = search_dataframe(df_indexed, "artificial intelligence", top_k=5)
print(search_results[['title', 'score', 'category']])
```

### Async Operations

```python
import vexfs
import asyncio
from concurrent.futures import ThreadPoolExecutor
from typing import List, Tuple

class AsyncVexFS:
    def __init__(self, mount_point: str, max_workers: int = 4):
        vexfs.init(mount_point)
        self.executor = ThreadPoolExecutor(max_workers=max_workers)
    
    async def add_async(self, text: str, metadata: Dict[str, str] = None) -> str:
        """Add document asynchronously"""
        loop = asyncio.get_event_loop()
        return await loop.run_in_executor(
            self.executor, vexfs.add, text, metadata
        )
    
    async def query_async(self, vector: List[float], top_k: int = 10) -> List[Tuple[str, float, str]]:
        """Query documents asynchronously"""
        loop = asyncio.get_event_loop()
        return await loop.run_in_executor(
            self.executor, vexfs.query, vector, top_k
        )
    
    async def add_batch_async(self, documents: List[Tuple[str, Dict[str, str]]]) -> List[str]:
        """Add multiple documents asynchronously"""
        tasks = [
            self.add_async(text, metadata)
            for text, metadata in documents
        ]
        return await asyncio.gather(*tasks)
    
    async def parallel_search(self, vectors: List[List[float]], top_k: int = 10) -> List[List[Tuple[str, float, str]]]:
        """Perform multiple searches in parallel"""
        tasks = [
            self.query_async(vector, top_k)
            for vector in vectors
        ]
        return await asyncio.gather(*tasks)

# Usage example
async def main():
    async_vexfs = AsyncVexFS("/mnt/vexfs")
    
    # Add documents asynchronously
    documents = [
        ("Document 1", {"type": "article"}),
        ("Document 2", {"type": "blog"}),
        ("Document 3", {"type": "paper"}),
    ]
    
    doc_ids = await async_vexfs.add_batch_async(documents)
    print(f"Added {len(doc_ids)} documents")
    
    # Parallel searches
    query_vectors = [
        [0.1] * 384,
        [0.2] * 384,
        [0.3] * 384
    ]
    
    all_results = await async_vexfs.parallel_search(query_vectors, top_k=5)
    for i, results in enumerate(all_results):
        print(f"Query {i+1}: {len(results)} results")

# Run async example
# asyncio.run(main())
```

## 🔧 Configuration

### Environment Variables

```python
import os
import vexfs

# Configure via environment variables
os.environ['VEXFS_CACHE_SIZE'] = '4GB'
os.environ['VEXFS_WORKER_THREADS'] = '8'
os.environ['VEXFS_VECTOR_DIMENSION'] = '384'

# Initialize with environment configuration
vexfs.init(os.getenv('VEXFS_MOUNT_POINT', '/mnt/vexfs'))
```

### Programmatic Configuration

```python
import vexfs

# Configure VexFS settings
vexfs.configure({
    "cache_size": "4GB",
    "worker_threads": 8,
    "vector_dimension": 384,
    "batch_size": 1000,
    "index_type": "hnsw",
    "compression": "zstd",
    "sync_interval": 5000
})
```

### Performance Tuning

```python
# High-throughput configuration
vexfs.configure({
    "cache_size": "8GB",
    "worker_threads": 16,
    "batch_size": 2000,
    "async_writes": True,
    "compression": "lz4",  # Faster compression
    "memory_map": True
})

# Low-latency configuration
vexfs.configure({
    "cache_size": "16GB",
    "preload_index": True,
    "memory_map": True,
    "sync_interval": 1000,
    "cache_policy": "lru"
})
```

## 🐛 Error Handling

### Exception Types

```python
from vexfs.exceptions import (
    VexFSError,
    DocumentNotFoundError,
    VectorDimensionError,
    ConfigurationError,
    IndexError,
    CacheError
)

def robust_operations():
    try:
        # VexFS operations
        vexfs.init("/mnt/vexfs")
        doc_id = vexfs.add("Sample text", {"type": "test"})
        results = vexfs.query([0.1] * 384, top_k=10)
        
    except DocumentNotFoundError as e:
        print(f"Document not found: {e}")
        # Handle missing document
        
    except VectorDimensionError as e:
        print(f"Vector dimension mismatch: {e}")
        # Handle dimension error
        
    except ConfigurationError as e:
        print(f"Configuration error: {e}")
        # Handle config issues
        
    except IndexError as e:
        print(f"Index error: {e}")
        # Handle index problems
        
    except CacheError as e:
        print(f"Cache error: {e}")
        # Handle cache issues
        
    except VexFSError as e:
        print(f"General VexFS error: {e}")
        # Handle general errors
        
    except Exception as e:
        print(f"Unexpected error: {e}")
        # Handle unexpected errors
```

### Retry Logic

```python
import time
from functools import wraps

def retry_on_error(max_retries: int = 3, delay: float = 1.0):
    """Decorator for retrying VexFS operations"""
    def decorator(func):
        @wraps(func)
        def wrapper(*args, **kwargs):
            last_exception = None
            
            for attempt in range(max_retries):
                try:
                    return func(*args, **kwargs)
                except VexFSError as e:
                    last_exception = e
                    if attempt < max_retries - 1:
                        time.sleep(delay * (2 ** attempt))  # Exponential backoff
                    else:
                        raise last_exception
                        
            return None
        return wrapper
    return decorator

# Usage
@retry_on_error(max_retries=3, delay=1.0)
def reliable_add(text: str, metadata: Dict[str, str] = None) -> str:
    return vexfs.add(text, metadata)

@retry_on_error(max_retries=3, delay=0.5)
def reliable_query(vector: List[float], top_k: int = 10) -> List[Tuple[str, float, str]]:
    return vexfs.query(vector, top_k)
```

## 📊 Performance Monitoring

### Metrics Collection

```python
import vexfs
import time
from dataclasses import dataclass
from typing import List

@dataclass
class PerformanceMetrics:
    operation: str
    duration: float
    throughput: float
    memory_usage: float
    cache_hit_rate: float

class VexFSMonitor:
    def __init__(self):
        self.metrics: List[PerformanceMetrics] = []
    
    def benchmark_add(self, documents: List[Tuple[str, Dict[str, str]]], batch_size: int = 100) -> PerformanceMetrics:
        """Benchmark document addition"""
        start_time = time.time()
        start_stats = vexfs.stats()
        
        # Add documents in batches
        doc_ids = []
        for i in range(0, len(documents), batch_size):
            batch = documents[i:i + batch_size]
            batch_ids = vexfs.add_batch(batch)
            doc_ids.extend(batch_ids)
        
        end_time = time.time()
        end_stats = vexfs.stats()
        
        duration = end_time - start_time
        throughput = len(documents) / duration
        
        metrics = PerformanceMetrics(
            operation="add",
            duration=duration,
            throughput=throughput,
            memory_usage=end_stats['memory_usage_mb'],
            cache_hit_rate=end_stats['cache_hit_rate']
        )
        
        self.metrics.append(metrics)
        return metrics
    
    def benchmark_query(self, vectors: List[List[float]], top_k: int = 10) -> PerformanceMetrics:
        """Benchmark query operations"""
        start_time = time.time()
        start_stats = vexfs.stats()
        
        all_results = []
        for vector in vectors:
            results = vexfs.query(vector, top_k=top_k)
            all_results.append(results)
        
        end_time = time.time()
        end_stats = vexfs.stats()
        
        duration = end_time - start_time
        throughput = len(vectors) / duration
        
        metrics = PerformanceMetrics(
            operation="query",
            duration=duration,
            throughput=throughput,
            memory_usage=end_stats['memory_usage_mb'],
            cache_hit_rate=end_stats['cache_hit_rate']
        )
        
        self.metrics.append(metrics)
        return metrics
    
    def report(self):
        """Generate performance report"""
        if not self.metrics:
            print("No metrics collected")
            return
        
        print("VexFS Performance Report")
        print("=" * 40)
        
        for metric in self.metrics:
            print(f"\nOperation: {metric.operation}")
            print(f"Duration: {metric.duration:.2f}s")
            print(f"Throughput: {metric.throughput:.0f} ops/sec")
            print(f"Memory Usage: {metric.memory_usage:.1f} MB")
            print(f"Cache Hit Rate: {metric.cache_hit_rate:.2%}")

# Usage
monitor = VexFSMonitor()

# Benchmark document addition
documents = [
    (f"Document {i}", {"index": str(i), "type": "test"})
    for i in range(1000)
]

add_metrics = monitor.benchmark_add(documents)
print(f"Added 1000 documents at {add_metrics.throughput:.0f} docs/sec")

# Benchmark queries
import numpy as np
query_vectors = [np.random.rand(384).tolist() for _ in range(100)]

query_metrics = monitor.benchmark_query(query_vectors)
print(f"Performed 100 queries at {query_metrics.throughput:.0f} queries/sec")

# Generate report
monitor.report()
```

## 🎯 Best Practices

### 1. Vector Dimension Management

```python
# Define constants
VECTOR_DIMENSION = 384

def validate_vector(vector: List[float]) -> List[float]:
    """Validate vector dimension"""
    if len(vector) != VECTOR_DIMENSION:
        raise ValueError(f"Expected {VECTOR_DIMENSION} dimensions, got {len(vector)}")
    return vector

def normalize_vector(vector: List[float]) -> List[float]:
    """Normalize vector for cosine similarity"""
    import math
    magnitude = math.sqrt(sum(x * x for x in vector))
    if magnitude == 0:
        return vector
    return [x / magnitude for x in vector]
```

### 2. Efficient Batch Processing

```python
def efficient_batch_processing(documents: List[Tuple[str, Dict[str, str]]], batch_size: int = 1000):
    """Process documents in optimal batches"""
    
    total_docs = len(documents)
    processed = 0
    
    for i in range(0, total_docs, batch_size):
        batch = documents[i:i + batch_size]
        
        # Process batch
        doc_ids = vexfs.add_batch(batch)
        processed += len(doc_ids)
        
        # Progress reporting
        progress = (processed / total_docs) * 100
        print(f"Processed {processed}/{total_docs} documents ({progress:.1f}%)")
        
        # Optional: Clear cache periodically for large datasets
        if processed % 10000 == 0:
            vexfs.optimize_cache()
    
    return processed
```

### 3. Memory Management

```python
def memory_aware_operations():
    """Monitor and manage memory usage"""
    
    # Check memory before large operations
    stats = vexfs.stats()
    if stats['memory_usage_mb'] > 8000:  # 8GB threshold
        print("High memory usage detected, optimizing...")
        vexfs.clear_cache()
        vexfs.optimize_index()
    
    # Configure for available memory
    import psutil
    available_memory_gb = psutil.virtual_memory().available / (1024**3)
    
    if available_memory_gb > 16:
        cache_size = "8GB"
    elif available_memory_gb > 8:
        cache_size = "4GB"
    else:
        cache_size = "2GB"
    
    vexfs.configure({"cache_size": cache_size})
```

### 4. Error Recovery

```python
def robust_document_processing(documents: List[Tuple[str, Dict[str, str]]]):
    """Process documents with error recovery"""
    
    successful = []
    failed = []
    
    for i, (text, metadata) in enumerate(documents):
        try:
            doc_id = vexfs.add(text, metadata)
            successful.append((i, doc_id))
            
        except Exception as e:
            failed.append((i, text, metadata, str(e)))
            print(f"Failed to add document {i}: {e}")
    
    print(f"Successfully processed: {len(successful)}")
    print(f"Failed: {len(failed)}")
    
    # Retry failed documents with different strategy
    if failed:
        print("Retrying failed documents...")
        for i, text, metadata, error in failed:
            try:
                # Simplify metadata or truncate text if needed
                simplified_metadata = {k: v for k, v in metadata.items() if len(v) < 100}
                doc_id = vexfs.add(text[:1000], simplified_metadata)
                successful.append((i, doc_id))
                print(f"Retry successful for document {i}")
            except Exception as e:
                print(f"Retry failed for document {i}: {e}")
    
    return successful, failed
```

## 🚀 Next Steps

- **[TypeScript SDK](typescript.md)** - TypeScript/JavaScript integration
- **[REST API](rest-api.md)** - HTTP API reference
- **[CLI Tool](vexctl.md)** - Command-line interface
- **[Examples](../examples/python.md)** - Real-world Python examples
- **[Performance Guide](../user-guide/performance.md)** - Optimization techniques

Ready to build amazing applications with VexFS Python SDK! 🎉