# Basic Operations

Learn the fundamental operations of VexFS v1.0 - from storing your first document to performing advanced vector searches.

## 🎯 Core Concepts

### Documents and Vectors
- **Documents**: Text content with optional metadata
- **Vectors**: Numerical embeddings representing document semantics
- **Collections**: Logical groupings of related documents
- **Metadata**: Key-value pairs for filtering and organization

### Vector Embeddings
VexFS works with vector embeddings - numerical representations of text that capture semantic meaning. You can:
- Generate embeddings using models like Sentence Transformers
- Use pre-computed embeddings from your ML pipeline
- Let VexFS handle embedding generation (coming in v1.1)

## 📝 Document Operations

### Adding Documents

=== "Python"
    ```python
    import vexfs
    
    # Initialize VexFS
    vexfs.init("/mnt/vexfs")
    
    # Add document with metadata
    doc_id = vexfs.add(
        "VexFS provides high-performance vector search capabilities",
        {
            "category": "technology",
            "type": "description",
            "author": "VexFS Team",
            "created_at": "2025-01-15"
        }
    )
    print(f"Document added with ID: {doc_id}")
    ```

=== "TypeScript"
    ```typescript
    import VexFSClient from 'vexfs-sdk';
    
    const client = new VexFSClient({
        baseUrl: 'http://localhost:8000'
    });
    
    // Add document with metadata
    const docId = await client.add(
        "VexFS provides high-performance vector search capabilities",
        {
            category: "technology",
            type: "description",
            author: "VexFS Team",
            created_at: "2025-01-15"
        }
    );
    console.log(`Document added with ID: ${docId}`);
    ```

=== "REST API"
    ```bash
    curl -X POST http://localhost:8000/api/v1/collections/my_collection/add \
      -H "Content-Type: application/json" \
      -d '{
        "ids": ["doc_1"],
        "documents": ["VexFS provides high-performance vector search capabilities"],
        "metadatas": [{
          "category": "technology",
          "type": "description",
          "author": "VexFS Team"
        }],
        "embeddings": [[0.1, 0.2, 0.3, ...]]
      }'
    ```

=== "CLI"
    ```bash
    vexctl add \
      --text "VexFS provides high-performance vector search capabilities" \
      --metadata '{"category": "technology", "type": "description"}'
    ```

### Batch Document Addition

For high-performance bulk operations:

=== "Python"
    ```python
    import vexfs
    from concurrent.futures import ThreadPoolExecutor
    
    def add_documents_batch(documents):
        """Add multiple documents efficiently"""
        doc_ids = []
        
        # Sequential addition (fastest for VexFS)
        for text, metadata in documents:
            doc_id = vexfs.add(text, metadata)
            doc_ids.append(doc_id)
        
        return doc_ids
    
    # Example usage
    documents = [
        ("Document 1 content", {"type": "article", "topic": "AI"}),
        ("Document 2 content", {"type": "blog", "topic": "ML"}),
        ("Document 3 content", {"type": "paper", "topic": "NLP"}),
    ]
    
    doc_ids = add_documents_batch(documents)
    print(f"Added {len(doc_ids)} documents")
    ```

=== "TypeScript"
    ```typescript
    import VexFSClient from 'vexfs-sdk';
    
    class BatchProcessor {
        private client: VexFSClient;
        
        constructor(client: VexFSClient) {
            this.client = client;
        }
        
        async addDocumentsBatch(
            documents: Array<{text: string, metadata?: Record<string, string>}>
        ): Promise<string[]> {
            const docIds: string[] = [];
            
            // Process in parallel batches
            const batchSize = 10;
            for (let i = 0; i < documents.length; i += batchSize) {
                const batch = documents.slice(i, i + batchSize);
                const promises = batch.map(doc => 
                    this.client.add(doc.text, doc.metadata)
                );
                const batchIds = await Promise.all(promises);
                docIds.push(...batchIds);
            }
            
            return docIds;
        }
    }
    
    // Usage
    const processor = new BatchProcessor(client);
    const documents = [
        { text: "Document 1", metadata: { type: "article" } },
        { text: "Document 2", metadata: { type: "blog" } },
        { text: "Document 3", metadata: { type: "paper" } },
    ];
    
    const docIds = await processor.addDocumentsBatch(documents);
    console.log(`Added ${docIds.length} documents`);
    ```

## 🔍 Vector Search Operations

### Basic Vector Search

=== "Python"
    ```python
    import vexfs
    import numpy as np
    
    # Generate query vector (use your embedding model)
    query_vector = np.random.rand(384).tolist()  # 384-dimensional
    
    # Search for similar documents
    results = vexfs.query(query_vector, top_k=5)
    
    # Process results
    for doc_id, score, text in results:
        print(f"ID: {doc_id}")
        print(f"Score: {score:.4f}")
        print(f"Text: {text}")
        print("---")
    ```

=== "TypeScript"
    ```typescript
    import VexFSClient from 'vexfs-sdk';
    
    const client = new VexFSClient();
    
    // Generate query vector (use your embedding model)
    const queryVector = new Array(384).fill(0).map(() => Math.random());
    
    // Search for similar documents
    const results = await client.query(queryVector, 5);
    
    // Process results
    results.forEach((result, index) => {
        console.log(`${index + 1}. Score: ${result.score.toFixed(4)}`);
        console.log(`   ID: ${result.id}`);
        console.log(`   Text: ${result.document}`);
        console.log(`   Metadata:`, result.metadata);
    });
    ```

### Advanced Search with Filtering

=== "Python"
    ```python
    import vexfs
    
    # Search with metadata filtering
    results = vexfs.query_with_filter(
        vector=query_vector,
        top_k=10,
        filters={
            "category": "technology",
            "type": ["article", "paper"]  # Multiple values
        }
    )
    
    # Search with score threshold
    results = vexfs.query_with_threshold(
        vector=query_vector,
        threshold=0.8,  # Only return results with score > 0.8
        max_results=20
    )
    ```

=== "TypeScript"
    ```typescript
    // Search with metadata filtering
    const results = await client.queryWithFilter(queryVector, {
        topK: 10,
        filters: {
            category: "technology",
            type: ["article", "paper"]
        }
    });
    
    // Search with score threshold
    const highQualityResults = await client.queryWithThreshold(queryVector, {
        threshold: 0.8,
        maxResults: 20
    });
    ```

### Multi-Metric Search

VexFS supports multiple similarity metrics:

=== "Python"
    ```python
    import vexfs
    
    # Euclidean distance (default)
    euclidean_results = vexfs.query(
        vector=query_vector,
        top_k=5,
        metric="euclidean"
    )
    
    # Cosine similarity
    cosine_results = vexfs.query(
        vector=query_vector,
        top_k=5,
        metric="cosine"
    )
    
    # Inner product
    inner_product_results = vexfs.query(
        vector=query_vector,
        top_k=5,
        metric="inner_product"
    )
    ```

=== "CLI"
    ```bash
    # Search with different metrics
    vexctl search --vector "[0.1,0.2,0.3]" --metric euclidean --top-k 5
    vexctl search --vector "[0.1,0.2,0.3]" --metric cosine --top-k 5
    vexctl search --vector "[0.1,0.2,0.3]" --metric inner_product --top-k 5
    ```

## 🗑️ Document Management

### Deleting Documents

=== "Python"
    ```python
    import vexfs
    
    # Delete single document
    vexfs.delete("doc_12345")
    
    # Delete multiple documents
    doc_ids = ["doc_1", "doc_2", "doc_3"]
    for doc_id in doc_ids:
        vexfs.delete(doc_id)
    
    # Batch delete (more efficient)
    vexfs.delete_batch(doc_ids)
    ```

=== "TypeScript"
    ```typescript
    // Delete single document
    await client.delete("doc_12345");
    
    // Delete multiple documents
    const docIds = ["doc_1", "doc_2", "doc_3"];
    await Promise.all(docIds.map(id => client.delete(id)));
    
    // Batch delete
    await client.deleteBatch(docIds);
    ```

### Updating Documents

=== "Python"
    ```python
    import vexfs
    
    # Update document content
    vexfs.update(
        doc_id="doc_12345",
        text="Updated document content",
        metadata={"updated_at": "2025-01-15", "version": "2"}
    )
    
    # Update only metadata
    vexfs.update_metadata(
        doc_id="doc_12345",
        metadata={"status": "reviewed", "reviewer": "John Doe"}
    )
    ```

=== "TypeScript"
    ```typescript
    // Update document content
    await client.update("doc_12345", {
        text: "Updated document content",
        metadata: { updated_at: "2025-01-15", version: "2" }
    });
    
    // Update only metadata
    await client.updateMetadata("doc_12345", {
        status: "reviewed",
        reviewer: "John Doe"
    });
    ```

## 📊 Collection Management

### Creating Collections

=== "Python"
    ```python
    import vexfs
    
    # Create collection with metadata
    collection = vexfs.create_collection(
        name="research_papers",
        metadata={
            "description": "Academic research papers",
            "created_by": "research_team",
            "vector_dimension": 384
        }
    )
    ```

=== "TypeScript"
    ```typescript
    // Create collection
    const collection = await client.createCollection("research_papers", {
        description: "Academic research papers",
        created_by: "research_team",
        vector_dimension: "384"
    });
    ```

### Listing Collections

=== "Python"
    ```python
    # List all collections
    collections = vexfs.list_collections()
    for collection in collections:
        print(f"Name: {collection.name}")
        print(f"ID: {collection.id}")
        print(f"Metadata: {collection.metadata}")
    ```

=== "TypeScript"
    ```typescript
    // List all collections
    const collections = await client.listCollections();
    collections.forEach(collection => {
        console.log(`Name: ${collection.name}`);
        console.log(`ID: ${collection.id}`);
        console.log(`Metadata:`, collection.metadata);
    });
    ```

## 📈 Performance Monitoring

### Getting Statistics

=== "Python"
    ```python
    import vexfs
    
    # Get filesystem statistics
    stats = vexfs.stats()
    print(f"Total documents: {stats['document_count']}")
    print(f"Total vectors: {stats['vector_count']}")
    print(f"Cache hit rate: {stats['cache_hit_rate']:.2%}")
    print(f"Memory usage: {stats['memory_usage_mb']} MB")
    ```

=== "TypeScript"
    ```typescript
    // Get server statistics
    const stats = await client.getStats();
    console.log(`Total documents: ${stats.document_count}`);
    console.log(`Total vectors: ${stats.vector_count}`);
    console.log(`Cache hit rate: ${(stats.cache_hit_rate * 100).toFixed(2)}%`);
    console.log(`Memory usage: ${stats.memory_usage_mb} MB`);
    ```

### Performance Benchmarking

=== "Python"
    ```python
    import vexfs
    import time
    import numpy as np
    
    def benchmark_operations():
        # Benchmark document addition
        start_time = time.time()
        doc_ids = []
        
        for i in range(1000):
            doc_id = vexfs.add(f"Document {i}", {"index": str(i)})
            doc_ids.append(doc_id)
        
        add_time = time.time() - start_time
        print(f"Added 1000 documents in {add_time:.2f}s")
        print(f"Rate: {1000/add_time:.0f} docs/second")
        
        # Benchmark search
        query_vector = np.random.rand(384).tolist()
        start_time = time.time()
        
        for _ in range(100):
            results = vexfs.query(query_vector, top_k=10)
        
        search_time = time.time() - start_time
        print(f"100 searches in {search_time:.2f}s")
        print(f"Average latency: {search_time*10:.2f}ms per search")
    
    benchmark_operations()
    ```

## 🔧 Configuration and Tuning

### Vector Dimension Configuration

```python
import vexfs

# Configure default vector dimension
vexfs.configure({
    "vector_dimension": 384,  # Match your embedding model
    "cache_size": "2GB",      # Adjust based on available memory
    "index_type": "hnsw",     # High-performance indexing
    "max_connections": 1000   # Concurrent connection limit
})
```

### Performance Tuning

```python
# Optimize for high-throughput scenarios
vexfs.configure({
    "batch_size": 1000,           # Larger batches for bulk operations
    "worker_threads": 8,          # Match CPU cores
    "cache_policy": "lru",        # Least Recently Used cache
    "compression": "zstd",        # Enable compression
    "sync_interval": 5000         # Sync to disk every 5000 operations
})

# Optimize for low-latency scenarios
vexfs.configure({
    "cache_size": "4GB",          # Larger cache for faster access
    "preload_index": True,        # Load index into memory at startup
    "async_writes": True,         # Non-blocking write operations
    "memory_map": True            # Use memory mapping for large files
})
```

## 🛡️ Error Handling

### Robust Error Handling

=== "Python"
    ```python
    import vexfs
    from vexfs.exceptions import VexFSError, DocumentNotFoundError, VectorDimensionError
    
    def safe_operations():
        try:
            # Initialize VexFS
            vexfs.init("/mnt/vexfs")
            
            # Add document
            doc_id = vexfs.add("Sample text", {"type": "test"})
            
            # Query with error handling
            query_vector = [0.1] * 384  # Ensure correct dimension
            results = vexfs.query(query_vector, top_k=10)
            
        except DocumentNotFoundError as e:
            print(f"Document not found: {e}")
        except VectorDimensionError as e:
            print(f"Vector dimension mismatch: {e}")
        except VexFSError as e:
            print(f"VexFS error: {e}")
        except Exception as e:
            print(f"Unexpected error: {e}")
    ```

=== "TypeScript"
    ```typescript
    import VexFSClient, { VexFSError } from 'vexfs-sdk';
    
    async function safeOperations() {
        const client = new VexFSClient();
        
        try {
            // Add document
            const docId = await client.add("Sample text", { type: "test" });
            
            // Query with error handling
            const queryVector = new Array(384).fill(0.1);
            const results = await client.query(queryVector, 10);
            
        } catch (error) {
            if (error instanceof VexFSError) {
                console.error(`VexFS error: ${error.message}`);
            } else {
                console.error(`Unexpected error: ${error}`);
            }
        }
    }
    ```

## 🎯 Best Practices

### 1. Vector Dimension Consistency
Always use the same vector dimension across your application:

```python
# Set dimension once and stick to it
VECTOR_DIMENSION = 384

# Validate vectors before operations
def validate_vector(vector):
    if len(vector) != VECTOR_DIMENSION:
        raise ValueError(f"Expected {VECTOR_DIMENSION} dimensions, got {len(vector)}")
    return vector
```

### 2. Efficient Batch Operations
Use batch operations for better performance:

```python
# Good: Batch operations
doc_ids = vexfs.add_batch([
    ("Text 1", {"type": "article"}),
    ("Text 2", {"type": "blog"}),
    ("Text 3", {"type": "paper"})
])

# Avoid: Individual operations in loops
for text, metadata in documents:
    vexfs.add(text, metadata)  # Slower
```

### 3. Metadata Design
Design metadata for efficient filtering:

```python
# Good: Structured metadata
metadata = {
    "category": "technology",      # Single value for exact match
    "tags": "ai,ml,nlp",          # Comma-separated for multiple values
    "created_date": "2025-01-15", # ISO format for date comparisons
    "priority": "high"            # Enumerated values
}

# Avoid: Unstructured metadata
metadata = {
    "info": "This is a technology article about AI and ML from 2025"  # Hard to filter
}
```

### 4. Memory Management
Monitor and optimize memory usage:

```python
# Check memory usage regularly
stats = vexfs.stats()
if stats['memory_usage_mb'] > 8000:  # 8GB threshold
    vexfs.clear_cache()  # Clear cache if needed
    vexfs.optimize_index()  # Optimize index structure
```

## 🚀 Next Steps

Now that you understand basic operations:

1. **[Vector Search Guide](vector-search.md)** - Advanced search techniques
2. **[Hybrid Queries](hybrid-queries.md)** - Combine vector and metadata search
3. **[Performance Optimization](performance.md)** - Tune for your use case
4. **[Python Examples](../examples/python.md)** - Real-world Python examples
5. **[TypeScript Examples](../examples/typescript.md)** - Real-world TypeScript examples

Ready to build something amazing with VexFS? 🎉