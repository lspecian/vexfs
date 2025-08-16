# VexFS API Documentation

## Overview

VexFS provides a unified API server that supports three different API dialects, allowing it to serve as a drop-in replacement for ChromaDB, Qdrant, or use its native API for extended features.

**Base URL**: `http://localhost:7680`

## API Dialects

### 1. ChromaDB API (`/api/v1/*`)
Full compatibility with ChromaDB HTTP clients

### 2. Qdrant API (`/collections/*`)  
Full compatibility with Qdrant REST clients

### 3. Native VexFS API (`/vexfs/v1/*`)
Extended features and optimizations specific to VexFS

## ChromaDB API Reference

### Collections

#### List Collections
```http
GET /api/v1/collections
```

**Response:**
```json
{
  "collections": ["collection1", "collection2"]
}
```

#### Create Collection
```http
POST /api/v1/collections
```

**Request Body:**
```json
{
  "name": "my_collection",
  "metadata": {
    "distance": "cosine",  // "cosine", "euclidean", or "ip"
    "description": "Optional description"
  }
}
```

**Response:**
```json
{
  "name": "my_collection",
  "id": "uuid-string",
  "metadata": {...}
}
```

#### Delete Collection
```http
DELETE /api/v1/collections/{collection_name}
```

### Documents

#### Add Documents
```http
POST /api/v1/collections/{collection_name}/add
```

**Request Body:**
```json
{
  "ids": ["id1", "id2"],
  "embeddings": [[0.1, 0.2, ...], [0.3, 0.4, ...]],
  "metadatas": [{"key": "value"}, {"key": "value"}],
  "documents": ["document text 1", "document text 2"]
}
```

**Response:**
```json
{
  "success": true
}
```

#### Query Collection
```http
POST /api/v1/collections/{collection_name}/query
```

**Request Body:**
```json
{
  "query_embeddings": [[0.1, 0.2, ...]],
  "n_results": 5,
  "where": {"metadata_key": "filter_value"},
  "include": ["distances", "metadatas", "documents"]
}
```

**Response:**
```json
{
  "ids": [["id1", "id2", ...]],
  "distances": [[0.1, 0.2, ...]],
  "metadatas": [[{...}, {...}]],
  "documents": [["doc1", "doc2"]]
}
```

## Qdrant API Reference

### Collections

#### List Collections
```http
GET /collections
```

**Response:**
```json
{
  "result": {
    "collections": [
      {
        "name": "collection1"
      }
    ]
  },
  "status": "ok"
}
```

#### Create Collection
```http
PUT /collections/{collection_name}
```

**Request Body:**
```json
{
  "vectors": {
    "size": 384,
    "distance": "Cosine"
  }
}
```

#### Delete Collection
```http
DELETE /collections/{collection_name}
```

### Points (Vectors)

#### Upsert Points
```http
PUT /collections/{collection_name}/points
```

**Request Body:**
```json
{
  "points": [
    {
      "id": "point-id-1",
      "vector": [0.1, 0.2, ...],
      "payload": {"key": "value"}
    }
  ]
}
```

#### Search Points
```http
POST /collections/{collection_name}/points/search
```

**Request Body:**
```json
{
  "vector": [0.1, 0.2, ...],
  "limit": 5,
  "filter": {
    "must": [
      {"key": "field", "match": {"value": "value"}}
    ]
  },
  "with_payload": true,
  "with_vector": false
}
```

**Response:**
```json
{
  "result": [
    {
      "id": "point-id",
      "score": 0.95,
      "payload": {"key": "value"}
    }
  ],
  "status": "ok"
}
```

## Native VexFS API Reference

### Collections

#### List Collections with Details
```http
GET /vexfs/v1/collections
```

**Response:**
```json
{
  "collections": [
    {
      "name": "collection1",
      "document_count": 1000,
      "vector_dimension": 384,
      "metadata": {...}
    }
  ]
}
```

#### Create Collection
```http
POST /vexfs/v1/collections
```

**Request Body:**
```json
{
  "name": "my_collection",
  "dimension": 384,
  "distance_function": "cosine",
  "hnsw_config": {
    "m": 16,
    "ef_construction": 200,
    "ef_search": 50
  }
}
```

### Documents

#### Add Documents with Auto-Embedding
```http
POST /vexfs/v1/collections/{collection_name}/documents
```

**Request Body:**
```json
{
  "documents": [
    {
      "id": "doc1",
      "content": "Text to be embedded",
      "metadata": {"key": "value"},
      "embedding": [0.1, 0.2, ...]  // Optional
    }
  ],
  "auto_embed": true  // Use built-in embedding
}
```

#### Semantic Search
```http
POST /vexfs/v1/collections/{collection_name}/search
```

**Request Body:**
```json
{
  "query": "search text",  // Can use text
  "query_vector": [0.1, 0.2, ...],  // Or vector
  "limit": 10,
  "filters": {...},
  "include_score": true,
  "include_metadata": true
}
```

### System

#### Health Check
```http
GET /vexfs/v1/health
```

**Response:**
```json
{
  "status": "healthy",
  "version": "0.0.4-alpha",
  "uptime": 3600,
  "collections_count": 5
}
```

#### Metrics
```http
GET /vexfs/v1/metrics
```

**Response:**
```json
{
  "operations_per_second": 15000,
  "memory_usage_mb": 512,
  "vector_count": 100000,
  "index_stats": {...}
}
```

## Server Information

#### Get Server Info
```http
GET /
```

**Response:**
```json
{
  "name": "VexFS Unified Server",
  "version": "0.0.4-alpha",
  "supported_apis": ["chromadb", "qdrant", "native"],
  "features": {
    "vector_search": true,
    "auto_embedding": false,
    "distributed": false
  }
}
```

#### Health Check
```http
GET /health
```

**Response:**
```json
{
  "status": "ok",
  "timestamp": "2025-08-16T10:00:00Z"
}
```

## Client Examples

### Python - ChromaDB Client
```python
import chromadb

# Connect to VexFS
client = chromadb.HttpClient(host="localhost", port=7680)

# Create collection
collection = client.create_collection(
    name="my_docs",
    metadata={"distance": "cosine"}
)

# Add documents
collection.add(
    documents=["Doc 1", "Doc 2"],
    embeddings=[[0.1, 0.2], [0.3, 0.4]],
    ids=["id1", "id2"],
    metadatas=[{"source": "file1"}, {"source": "file2"}]
)

# Query
results = collection.query(
    query_embeddings=[[0.1, 0.2]],
    n_results=5
)
```

### Python - Qdrant Client
```python
from qdrant_client import QdrantClient
from qdrant_client.models import Distance, VectorParams

# Connect to VexFS
client = QdrantClient(host="localhost", port=7680)

# Create collection
client.recreate_collection(
    collection_name="my_docs",
    vectors_config=VectorParams(size=384, distance=Distance.COSINE)
)

# Add vectors
client.upsert(
    collection_name="my_docs",
    points=[
        {"id": 1, "vector": [0.1, 0.2, ...], "payload": {"text": "doc1"}}
    ]
)

# Search
results = client.search(
    collection_name="my_docs",
    query_vector=[0.1, 0.2, ...],
    limit=5
)
```

### JavaScript/TypeScript
```typescript
// Using the dashboard API client
import { VexFSClient } from './api';

const client = new VexFSClient('http://localhost:7680');

// Create collection
await client.createCollection({
  name: 'my_docs',
  dimension: 384,
  distance: 'cosine'
});

// Add documents
await client.addDocuments('my_docs', [
  {
    id: 'doc1',
    embedding: [0.1, 0.2, ...],
    metadata: { source: 'file1' }
  }
]);

// Search
const results = await client.search('my_docs', {
  vector: [0.1, 0.2, ...],
  limit: 5
});
```

### cURL Examples
```bash
# List collections (ChromaDB style)
curl http://localhost:7680/api/v1/collections

# Create collection (Qdrant style)
curl -X PUT http://localhost:7680/collections/my_collection \
  -H "Content-Type: application/json" \
  -d '{"vectors": {"size": 384, "distance": "Cosine"}}'

# Health check
curl http://localhost:7680/health

# Add documents (Native API)
curl -X POST http://localhost:7680/vexfs/v1/collections/my_collection/documents \
  -H "Content-Type: application/json" \
  -d '{
    "documents": [{
      "id": "doc1",
      "content": "Sample text",
      "metadata": {"key": "value"}
    }]
  }'
```

## Error Responses

All APIs return consistent error responses:

```json
{
  "error": {
    "code": "NOT_FOUND",
    "message": "Collection not found",
    "details": {...}
  },
  "status": "error"
}
```

**Common Error Codes:**
- `NOT_FOUND` - Resource not found
- `INVALID_ARGUMENT` - Invalid request parameters
- `ALREADY_EXISTS` - Resource already exists
- `INTERNAL_ERROR` - Server error
- `LOCK_ERROR` - Concurrency issue

## Rate Limiting

Currently no rate limiting is implemented. This is planned for future releases.

## Authentication

Currently no authentication is required. This is a critical gap that will be addressed in the next release.

## Performance Considerations

- **Batch Operations**: Use batch add/upsert for better performance
- **Vector Dimensions**: Keep dimensions reasonable (< 2048) for optimal HNSW performance
- **Collection Size**: Performance may degrade with > 1M vectors per collection
- **Concurrent Requests**: Server handles concurrent requests well due to Rust async runtime

## Migration Guide

### From ChromaDB
1. Change connection URL to `http://localhost:7680`
2. No other code changes needed - full compatibility

### From Qdrant
1. Change connection URL to `http://localhost:7680`
2. No other code changes needed - full compatibility

### From Pinecone/Weaviate
Native support coming soon. Currently requires using Native VexFS API.

---

*API Documentation for VexFS v0.0.4-alpha - Subject to change in future releases*