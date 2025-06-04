# Vector Search Tutorial

This comprehensive tutorial teaches you how to leverage VexFS v2.0's powerful vector search capabilities for building modern AI applications.

## 🎯 What You'll Learn

- Vector search fundamentals
- HNSW vs LSH algorithms
- Advanced search techniques
- Performance optimization
- Real-world applications

## 📚 Prerequisites

- VexFS v2.0 installed and running
- Basic understanding of vectors and embeddings
- Python or TypeScript development environment
- Sample dataset (we'll provide one)

## 🚀 Getting Started

### Setup Your Environment

```bash
# Ensure VexFS is mounted
mount | grep vexfs

# Install Python SDK
pip install vexfs-v2 numpy sentence-transformers

# Or TypeScript SDK
npm install @vexfs/sdk-v2
```

### Sample Dataset

We'll use a collection of Wikipedia articles for this tutorial:

```python
import vexfs
import numpy as np
from sentence_transformers import SentenceTransformer
import json

# Initialize embedding model
model = SentenceTransformer('all-MiniLM-L6-v2')

# Sample documents
documents = [
    {"id": 1, "title": "Machine Learning", "content": "Machine learning is a subset of artificial intelligence...", "category": "AI"},
    {"id": 2, "title": "Deep Learning", "content": "Deep learning is part of machine learning methods...", "category": "AI"},
    {"id": 3, "title": "Natural Language Processing", "content": "NLP is a subfield of linguistics and AI...", "category": "AI"},
    {"id": 4, "title": "Computer Vision", "content": "Computer vision is an interdisciplinary field...", "category": "AI"},
    {"id": 5, "title": "Quantum Computing", "content": "Quantum computing uses quantum mechanics...", "category": "Computing"},
    {"id": 6, "title": "Blockchain Technology", "content": "Blockchain is a distributed ledger technology...", "category": "Technology"},
    {"id": 7, "title": "Renewable Energy", "content": "Renewable energy comes from natural sources...", "category": "Environment"},
    {"id": 8, "title": "Climate Change", "content": "Climate change refers to long-term shifts...", "category": "Environment"},
    {"id": 9, "title": "Space Exploration", "content": "Space exploration is the investigation of space...", "category": "Science"},
    {"id": 10, "title": "Genetic Engineering", "content": "Genetic engineering involves direct manipulation...", "category": "Science"}
]

# Generate embeddings
embeddings = model.encode([doc["content"] for doc in documents])
print(f"Generated embeddings shape: {embeddings.shape}")
```

## 🔍 Basic Vector Search

### Creating Your First Collection

```python
# Connect to VexFS
client = vexfs.Client('/mnt/vexfs')

# Create collection with HNSW algorithm
collection = client.create_collection(
    name="wikipedia_articles",
    dimension=384,  # MiniLM model dimension
    algorithm="hnsw",
    distance_metric="cosine",
    # HNSW parameters for good recall
    m=16,
    ef_construction=200,
    ef_search=100
)

print(f"Created collection: {collection.info.name}")
print(f"Dimension: {collection.info.dimension}")
print(f"Algorithm: {collection.info.algorithm}")
```

### Inserting Vectors

```python
# Insert documents with embeddings
insert_results = []

for i, (doc, embedding) in enumerate(zip(documents, embeddings)):
    result = collection.insert(
        vector=embedding,
        metadata={
            "doc_id": doc["id"],
            "title": doc["title"],
            "category": doc["category"],
            "content_preview": doc["content"][:100] + "...",
            "word_count": len(doc["content"].split()),
            "inserted_at": "2025-06-04T10:00:00Z"
        }
    )
    insert_results.append(result)
    print(f"Inserted: {doc['title']} -> Vector ID: {result.id}")

print(f"\nInserted {len(insert_results)} documents")
```

### Basic Similarity Search

```python
# Search for documents similar to a query
query_text = "artificial intelligence and machine learning"
query_embedding = model.encode([query_text])[0]

# Perform search
results = collection.search(
    vector=query_embedding,
    limit=5,
    distance_metric="cosine"
)

print(f"\nSearch results for: '{query_text}'")
print("=" * 50)

for i, result in enumerate(results, 1):
    print(f"{i}. {result.metadata['title']}")
    print(f"   Distance: {result.distance:.4f}")
    print(f"   Category: {result.metadata['category']}")
    print(f"   Preview: {result.metadata['content_preview']}")
    print()
```

## 🎛️ Advanced Search Techniques

### Filtered Search

```python
# Search within specific categories
ai_results = collection.search(
    vector=query_embedding,
    limit=3,
    filter={"category": "AI"}
)

print("AI-related results:")
for result in ai_results:
    print(f"- {result.metadata['title']} (distance: {result.distance:.4f})")

# Complex filters
complex_results = collection.search(
    vector=query_embedding,
    limit=5,
    filter={
        "$and": [
            {"category": {"$in": ["AI", "Science"]}},
            {"word_count": {"$gte": 10}}
        ]
    }
)

print("\nComplex filter results:")
for result in complex_results:
    print(f"- {result.metadata['title']} ({result.metadata['category']})")
```

### Range Search

```python
# Find all documents within a distance threshold
similar_docs = collection.search_range(
    vector=query_embedding,
    max_distance=0.5,  # Cosine distance threshold
    filter={"category": "AI"}
)

print(f"\nFound {len(similar_docs)} documents within distance 0.5:")
for doc in similar_docs:
    print(f"- {doc.metadata['title']} (distance: {doc.distance:.4f})")
```

### Batch Search

```python
# Search multiple queries efficiently
queries = [
    "quantum computing and physics",
    "environmental sustainability",
    "space technology and exploration"
]

query_embeddings = model.encode(queries)

# Batch search
batch_results = collection.search_batch(
    vectors=query_embeddings,
    limit=3,
    max_workers=4
)

for i, (query, results) in enumerate(zip(queries, batch_results)):
    print(f"\nResults for: '{query}'")
    for result in results:
        print(f"  - {result.metadata['title']} ({result.distance:.4f})")
```

## ⚙️ Algorithm Comparison: HNSW vs LSH

### HNSW Collection (High Recall)

```python
# Create HNSW collection optimized for recall
hnsw_collection = client.create_collection(
    name="hnsw_demo",
    dimension=384,
    algorithm="hnsw",
    distance_metric="cosine",
    # High-quality parameters
    m=32,
    ef_construction=400,
    ef_search=200
)

# Insert same data
for doc, embedding in zip(documents, embeddings):
    hnsw_collection.insert(vector=embedding, metadata={"title": doc["title"]})
```

### LSH Collection (Memory Efficient)

```python
# Create LSH collection optimized for memory
lsh_collection = client.create_collection(
    name="lsh_demo",
    dimension=384,
    algorithm="lsh",
    distance_metric="cosine",
    # LSH parameters
    num_tables=20,
    num_functions=30,
    bucket_size=100
)

# Insert same data
for doc, embedding in zip(documents, embeddings):
    lsh_collection.insert(vector=embedding, metadata={"title": doc["title"]})
```

### Performance Comparison

```python
import time

def benchmark_search(collection, query_vector, iterations=100):
    """Benchmark search performance"""
    times = []
    
    for _ in range(iterations):
        start_time = time.time()
        results = collection.search(query_vector, limit=10)
        end_time = time.time()
        times.append((end_time - start_time) * 1000)  # Convert to ms
    
    return {
        'avg_time_ms': np.mean(times),
        'std_time_ms': np.std(times),
        'min_time_ms': np.min(times),
        'max_time_ms': np.max(times),
        'result_count': len(results)
    }

# Benchmark both algorithms
query_vector = model.encode(["machine learning algorithms"])[0]

print("Performance Comparison:")
print("=" * 40)

hnsw_perf = benchmark_search(hnsw_collection, query_vector)
print(f"HNSW - Avg: {hnsw_perf['avg_time_ms']:.2f}ms ± {hnsw_perf['std_time_ms']:.2f}ms")

lsh_perf = benchmark_search(lsh_collection, query_vector)
print(f"LSH  - Avg: {lsh_perf['avg_time_ms']:.2f}ms ± {lsh_perf['std_time_ms']:.2f}ms")

# Memory usage comparison
hnsw_stats = hnsw_collection.get_stats()
lsh_stats = lsh_collection.get_stats()

print(f"\nMemory Usage:")
print(f"HNSW: {hnsw_stats.index_size_mb:.1f}MB")
print(f"LSH:  {lsh_stats.index_size_mb:.1f}MB")
```

## 🚀 Performance Optimization

### Tuning HNSW Parameters

```python
def test_hnsw_parameters():
    """Test different HNSW parameter combinations"""
    
    parameter_sets = [
        {"m": 8, "ef_construction": 100, "ef_search": 50},    # Fast
        {"m": 16, "ef_construction": 200, "ef_search": 100},  # Balanced
        {"m": 32, "ef_construction": 400, "ef_search": 200},  # High quality
    ]
    
    results = []
    
    for i, params in enumerate(parameter_sets):
        # Create collection with specific parameters
        test_collection = client.create_collection(
            name=f"hnsw_test_{i}",
            dimension=384,
            algorithm="hnsw",
            **params
        )
        
        # Insert data
        start_time = time.time()
        for doc, embedding in zip(documents, embeddings):
            test_collection.insert(vector=embedding, metadata={"title": doc["title"]})
        insert_time = time.time() - start_time
        
        # Test search performance
        search_perf = benchmark_search(test_collection, query_vector, iterations=50)
        
        results.append({
            "params": params,
            "insert_time": insert_time,
            "search_time": search_perf['avg_time_ms'],
            "memory_mb": test_collection.get_stats().index_size_mb
        })
        
        # Cleanup
        client.delete_collection(f"hnsw_test_{i}")
    
    # Display results
    print("HNSW Parameter Tuning Results:")
    print("=" * 60)
    for result in results:
        print(f"Params: {result['params']}")
        print(f"  Insert time: {result['insert_time']:.2f}s")
        print(f"  Search time: {result['search_time']:.2f}ms")
        print(f"  Memory usage: {result['memory_mb']:.1f}MB")
        print()

test_hnsw_parameters()
```

### Batch Operations for Performance

```python
# Efficient batch insertion
def efficient_batch_insert():
    """Demonstrate efficient batch insertion"""
    
    # Generate larger dataset
    large_documents = []
    for i in range(1000):
        large_documents.append({
            "id": i,
            "content": f"Document {i} about various topics including AI, science, and technology.",
            "category": ["AI", "Science", "Technology"][i % 3]
        })
    
    # Generate embeddings in batches
    batch_size = 100
    all_embeddings = []
    
    for i in range(0, len(large_documents), batch_size):
        batch_docs = large_documents[i:i+batch_size]
        batch_embeddings = model.encode([doc["content"] for doc in batch_docs])
        all_embeddings.extend(batch_embeddings)
    
    # Create collection for batch test
    batch_collection = client.create_collection(
        name="batch_test",
        dimension=384,
        algorithm="hnsw"
    )
    
    # Method 1: Individual inserts (slow)
    start_time = time.time()
    for i in range(100):  # Just first 100 for comparison
        batch_collection.insert(
            vector=all_embeddings[i],
            metadata={"id": large_documents[i]["id"]}
        )
    individual_time = time.time() - start_time
    
    # Method 2: Batch insert (fast)
    start_time = time.time()
    vectors = np.array(all_embeddings[100:200])
    metadata = [{"id": doc["id"]} for doc in large_documents[100:200]]
    
    batch_collection.insert_batch(
        vectors=vectors,
        metadata=metadata,
        batch_size=50
    )
    batch_time = time.time() - start_time
    
    print(f"Individual inserts (100 vectors): {individual_time:.2f}s")
    print(f"Batch insert (100 vectors): {batch_time:.2f}s")
    print(f"Speedup: {individual_time/batch_time:.1f}x")
    
    # Cleanup
    client.delete_collection("batch_test")

efficient_batch_insert()
```

## 🎯 Real-World Applications

### Semantic Search Engine

```python
class SemanticSearchEngine:
    """A complete semantic search engine using VexFS"""
    
    def __init__(self, collection_name: str, model_name: str = 'all-MiniLM-L6-v2'):
        self.client = vexfs.Client('/mnt/vexfs')
        self.model = SentenceTransformer(model_name)
        self.collection_name = collection_name
        self.collection = None
    
    def create_index(self, documents: list, algorithm: str = "hnsw"):
        """Create search index from documents"""
        
        # Create collection
        self.collection = self.client.create_collection(
            name=self.collection_name,
            dimension=self.model.get_sentence_embedding_dimension(),
            algorithm=algorithm,
            distance_metric="cosine"
        )
        
        # Generate embeddings
        print(f"Generating embeddings for {len(documents)} documents...")
        embeddings = self.model.encode([doc["content"] for doc in documents])
        
        # Insert in batches
        batch_size = 100
        for i in range(0, len(documents), batch_size):
            batch_docs = documents[i:i+batch_size]
            batch_embeddings = embeddings[i:i+batch_size]
            
            metadata = [
                {
                    "doc_id": doc.get("id", i+j),
                    "title": doc.get("title", f"Document {i+j}"),
                    "url": doc.get("url", ""),
                    "category": doc.get("category", "general"),
                    "content_preview": doc["content"][:200] + "...",
                    "word_count": len(doc["content"].split())
                }
                for j, doc in enumerate(batch_docs)
            ]
            
            self.collection.insert_batch(
                vectors=batch_embeddings,
                metadata=metadata,
                batch_size=50
            )
        
        print(f"Indexed {len(documents)} documents")
    
    def search(self, query: str, limit: int = 10, filters: dict = None):
        """Search for relevant documents"""
        
        if not self.collection:
            self.collection = self.client.get_collection(self.collection_name)
        
        # Generate query embedding
        query_embedding = self.model.encode([query])[0]
        
        # Search
        results = self.collection.search(
            vector=query_embedding,
            limit=limit,
            filter=filters
        )
        
        return [
            {
                "title": result.metadata["title"],
                "content_preview": result.metadata["content_preview"],
                "category": result.metadata["category"],
                "relevance_score": 1 - result.distance,  # Convert distance to score
                "url": result.metadata.get("url", "")
            }
            for result in results
        ]
    
    def get_similar_documents(self, doc_id: int, limit: int = 5):
        """Find documents similar to a given document"""
        
        # Get the document vector
        doc_vector = self.collection.get_vector(doc_id)
        if not doc_vector:
            return []
        
        # Search for similar documents
        results = self.collection.search(
            vector=doc_vector.vector,
            limit=limit + 1,  # +1 because the document itself will be included
            filter={"doc_id": {"$ne": doc_id}}  # Exclude the document itself
        )
        
        return results[:limit]

# Example usage
search_engine = SemanticSearchEngine("semantic_search_demo")

# Create index
search_engine.create_index(documents)

# Search examples
print("Search Results:")
print("=" * 50)

queries = [
    "artificial intelligence and machine learning",
    "environmental protection and sustainability",
    "space exploration and astronomy"
]

for query in queries:
    print(f"\nQuery: '{query}'")
    results = search_engine.search(query, limit=3)
    
    for i, result in enumerate(results, 1):
        print(f"  {i}. {result['title']}")
        print(f"     Relevance: {result['relevance_score']:.3f}")
        print(f"     Category: {result['category']}")
```

### Recommendation System

```python
class DocumentRecommendationSystem:
    """Content-based recommendation system"""
    
    def __init__(self, collection_name: str):
        self.client = vexfs.Client('/mnt/vexfs')
        self.collection = self.client.get_collection(collection_name)
        self.user_profiles = {}
    
    def create_user_profile(self, user_id: str, liked_doc_ids: list):
        """Create user profile from liked documents"""
        
        # Get vectors for liked documents
        liked_vectors = []
        for doc_id in liked_doc_ids:
            vector_data = self.collection.get_vector(doc_id)
            if vector_data:
                liked_vectors.append(vector_data.vector)
        
        if not liked_vectors:
            return None
        
        # Create user profile as centroid of liked documents
        user_profile = np.mean(liked_vectors, axis=0)
        self.user_profiles[user_id] = user_profile
        
        return user_profile
    
    def get_recommendations(self, user_id: str, limit: int = 10, exclude_seen: list = None):
        """Get recommendations for a user"""
        
        if user_id not in self.user_profiles:
            return []
        
        user_profile = self.user_profiles[user_id]
        
        # Build filter to exclude seen documents
        filter_dict = {}
        if exclude_seen:
            filter_dict["doc_id"] = {"$nin": exclude_seen}
        
        # Search for similar documents
        results = self.collection.search(
            vector=user_profile,
            limit=limit,
            filter=filter_dict if filter_dict else None
        )
        
        return [
            {
                "doc_id": result.metadata["doc_id"],
                "title": result.metadata["title"],
                "category": result.metadata["category"],
                "recommendation_score": 1 - result.distance
            }
            for result in results
        ]
    
    def update_user_profile(self, user_id: str, new_liked_doc_id: int, weight: float = 0.1):
        """Update user profile with new interaction"""
        
        if user_id not in self.user_profiles:
            return False
        
        # Get new document vector
        vector_data = self.collection.get_vector(new_liked_doc_id)
        if not vector_data:
            return False
        
        # Update profile with weighted average
        current_profile = self.user_profiles[user_id]
        new_vector = vector_data.vector
        
        updated_profile = (1 - weight) * current_profile + weight * new_vector
        self.user_profiles[user_id] = updated_profile
        
        return True

# Example usage
recommender = DocumentRecommendationSystem("wikipedia_articles")

# Create user profile (user likes AI-related documents)
ai_doc_ids = [1, 2, 3]  # Machine Learning, Deep Learning, NLP
recommender.create_user_profile("user_123", ai_doc_ids)

# Get recommendations
recommendations = recommender.get_recommendations("user_123", limit=5, exclude_seen=ai_doc_ids)

print("Recommendations for user_123:")
for rec in recommendations:
    print(f"- {rec['title']} (score: {rec['recommendation_score']:.3f})")
```

## 📊 Monitoring and Analytics

### Search Analytics

```python
class SearchAnalytics:
    """Track and analyze search patterns"""
    
    def __init__(self, collection_name: str):
        self.client = vexfs.Client('/mnt/vexfs')
        self.collection = self.client.get_collection(collection_name)
        self.search_log = []
    
    def log_search(self, query: str, results: list, user_id: str = None):
        """Log search query and results"""
        
        self.search_log.append({
            "timestamp": time.time(),
            "query": query,
            "result_count": len(results),
            "user_id": user_id,
            "top_categories": self._get_top_categories(results),
            "avg_relevance": np.mean([1 - r.distance for r in results]) if results else 0
        })
    
    def _get_top_categories(self, results: list) -> list:
        """Get most common categories in results"""
        categories = [r.metadata.get("category", "unknown") for r in results]
        from collections import Counter
        return [cat for cat, count in Counter(categories).most_common(3)]
    
    def get_search_stats(self) -> dict:
        """Get search statistics"""
        
        if not self.search_log:
            return {}
        
        total_searches = len(self.search_log)
        avg_results = np.mean([log["result_count"] for log in self.search_log])
        avg_relevance = np.mean([log["avg_relevance"] for log in self.search_log])
        
        # Most common categories
        all_categories = []
        for log in self.search_log:
            all_categories.extend(log["top_categories"])
        
        from collections import Counter
        top_categories = Counter(all_categories).most_common(5)
        
        return {
            "total_searches": total_searches,
            "avg_results_per_search": avg_results,
            "avg_relevance_score": avg_relevance,
            "top_categories": top_categories
        }
    
    def get_performance_metrics(self) -> dict:
        """Get collection performance metrics"""
        
        stats = self.collection.get_stats()
        system_stats = self.client.get_stats()
        
        return {
            "collection_stats": {
                "vector_count": stats.vector_count,
                "search_count": stats.search_count,
                "insert_count": stats.insert_count,
                "index_size_mb": stats.index_size_mb
            },
            "system_stats": {
                "total_vectors": system_stats.total_vectors,
                "cache_hit_rate": system_stats.cache_hits / (system_stats.cache_hits + system_stats.cache_misses),
                "avg_search_time_ms": system_stats.avg_search_time_ms,
                "memory_usage_mb": system_stats.memory_usage_mb
            }
        }

# Example usage
analytics = SearchAnalytics("wikipedia_articles")

# Simulate some searches
test_queries = [
    "machine learning algorithms",
    "climate change effects",
    "quantum computing applications",
    "space exploration missions"
]

for query in test_queries:
    query_embedding = model.encode([query])[0]
    results = collection.search(query_embedding, limit=5)
    analytics.log_search(query, results, user_id="demo_user")

# Get analytics
search_stats = analytics.get_search_stats()
perf_metrics = analytics.get_performance_metrics()

print("Search Analytics:")
print(f"Total searches: {search_stats['total_searches']}")
print(f"Avg results per search: {search_stats['avg_results_per_search']:.1f}")
print(f"Avg relevance score: {search_stats['avg_relevance_score']:.3f}")
print(f"Top categories: {search_stats['top_categories']}")

print("\nPerformance Metrics:")
print(f"Cache hit rate: {perf_metrics['system_stats']['cache_hit_rate']:.2%}")
print(f"Avg search time: {perf_metrics['system_stats']['avg_search_time_ms']:.2f}ms")
```

## 🎓 Best Practices

### 1. Vector Quality

```python
def validate_and_normalize_vectors(vectors: np.ndarray) -> np.ndarray:
    """Ensure vector quality for optimal search performance"""
    
    # Check for NaN or infinite values
    if not np.isfinite(vectors).all():
        print("Warning: Vectors contain NaN or infinite values")
        vectors = np.nan_to_num(vectors, nan=0.0, posinf=1.0, neginf=-1.0)
    
    # Check for zero vectors
    norms = np.linalg.norm(vectors, axis=1)
    zero_vectors = np.sum(norms == 0)
    if zero_vectors > 0:
        print(f"Warning: {zero_vectors} zero vectors found")
    
    # Normalize vectors for cosine similarity
    normalized_vectors = vectors / norms[:, np.newaxis]
    normalized_vectors[norms == 0] = 0  # Handle zero vectors
    
    return normalized_vectors

# Example usage
raw_embeddings = model.encode([doc["content"] for doc in documents])
clean_embeddings = validate_and_normalize_vectors(raw_embeddings)
```

### 2. Metadata Design

```python
def design_efficient_metadata(document: dict) -> dict:
    """Design metadata for efficient filtering and retrieval"""
    
    return {
        # Identifiers
        "doc_id": document["id"],
        "source": document.get("source", "unknown"),
        
        # Categorical fields (good for filtering)
        "category": document.get("category", "general"),
        "language": document.get("language", "en"),
        "content_type": document.get("type", "text"),
        
        # Numerical fields (good for range queries)
        "word_count": len(document["content"].split()),
        "char_count": len(document["content"]),
        "quality_score": document.get("quality_score", 0.5),
        
        # Temporal fields
        "created_date": document.get("created_date", "2025-06-04"),
        "modified_date": document.get("modified_date", "2025-06-04"),
        
        # Searchable text fields
        "title": document.get("title", ""),
        "author": document.get("author", ""),
        "tags": document.get("tags", []),
        
        # Preview for display
        "content_preview": document["content"][:200] + "..." if len(document["content"]) > 200 else document["content"]
    }
```

### 3. Error Handling

```python
import vexfs.exceptions as vex

def robust_vector_search(collection, query_vector, **kwargs):
    """Robust search with comprehensive error handling"""
    
    try:
        results = collection.search(query_vector, **kwargs)
        return {"success": True, "results": results, "error": None}
        
    except vex.VectorDimensionError as e:
        return {"success": False, "results": [], "error": f"Dimension mismatch: {e}"}
        
    except vex.SearchTimeoutError as e:
        return {"success": False, "results": [], "error": f"Search timeout: {e}"}
        
    except vex.CollectionNotFoundError as e:
        return {"success": False, "results": [], "error": f"Collection not found: {e}"}
        
    except Exception as e:
        return {"success": False, "results": [], "error": f"Unexpected error: {e}"}

# Example usage
search_result = robust_vector_search(
    collection, 
    query_embedding, 
    limit=10, 
    filter={"category": "AI"}
)

if search_result["success"]:
    print(f"Found {len(search_result['results'])} results")
else:
    print(f"Search failed: {search_result['error']}")
```

## 🎯 Next Steps

Now that you've mastered vector search with VexFS v2.0:

1. **[Performance Tuning Guide](../reference/performance.md)** - Optimize for your specific use case
2. **[Integration Tutorial](integration.md)** - Integrate with existing systems
3. **[Production Deployment](../user-guide/installation.md)** - Deploy to production
4. **[API Reference](../developer-guide/api-reference.md)** - Complete API documentation

### Advanced Topics

- **Multi-modal search** - Combine text, image, and audio embeddings
- **Federated search** - Search across multiple VexFS instances
- **Real-time updates** - Handle streaming data and incremental updates
- **Custom algorithms** - Implement domain-specific search algorithms

---

**Congratulations!** 🎉 You've learned how to build powerful vector search applications with VexFS v2.0. The combination of filesystem semantics and advanced vector search opens up endless possibilities for AI-powered applications.

**Questions?** Check our [troubleshooting guide](../user-guide/troubleshooting.md) or [community discussions](https://github.com/lspecian/vexfs/discussions).