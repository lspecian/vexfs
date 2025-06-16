# VexFS v2 API Compatibility Specification

**Date**: June 4, 2025  
**Version**: 1.0  
**Status**: Technical Specification  
**Related Documents**: 
- [`VEXFS_V2_VECTOR_DATABASE_COMPATIBILITY_ARCHITECTURE.md`](mdc:docs/architecture/VEXFS_V2_VECTOR_DATABASE_COMPATIBILITY_ARCHITECTURE.md)
- [`VEXFS_V2_VECTOR_DATABASE_COMPATIBILITY_IMPLEMENTATION_PLAN.md`](mdc:docs/implementation/VEXFS_V2_VECTOR_DATABASE_COMPATIBILITY_IMPLEMENTATION_PLAN.md)

## Executive Summary

This document provides detailed technical specifications for ChromaDB and Qdrant API compatibility layers in VexFS v2. It defines exact API endpoints, request/response formats, data models, and compatibility requirements to ensure drop-in replacement capability.

## ChromaDB API Compatibility Specification

### 1. ChromaDB API Endpoints

#### 1.1 System Endpoints

##### GET /api/v1/version
**Purpose**: Get server version information  
**ChromaDB Compatibility**: ✅ Full compatibility

**Request**: No parameters

**Response**:
```json
{
  "success": true,
  "data": "VexFS-ChromaDB-Adapter/1.0.0"
}
```

**VexFS Implementation**:
```python
@router.get("/api/v1/version")
async def get_version():
    return {
        "success": True,
        "data": f"VexFS-ChromaDB-Adapter/{VERSION}"
    }
```

##### GET /api/v1/heartbeat
**Purpose**: Health check endpoint  
**ChromaDB Compatibility**: ✅ Full compatibility

**Request**: No parameters

**Response**:
```json
{
  "nanosecond heartbeat": 1717459200000000000
}
```

**VexFS Implementation**:
```python
@router.get("/api/v1/heartbeat")
async def heartbeat():
    return {
        "nanosecond heartbeat": int(time.time_ns())
    }
```

##### POST /api/v1/reset
**Purpose**: Reset the database (development only)  
**ChromaDB Compatibility**: ✅ Full compatibility

**Request**: No parameters

**Response**:
```json
{
  "success": true,
  "data": "Reset successful"
}
```

#### 1.2 Collection Management Endpoints

##### POST /api/v1/collections
**Purpose**: Create a new collection  
**ChromaDB Compatibility**: ✅ Full compatibility

**Request**:
```json
{
  "name": "my_collection",
  "metadata": {
    "description": "My test collection",
    "custom_field": "custom_value"
  },
  "embedding_function": "default"
}
```

**Response**:
```json
{
  "success": true,
  "data": {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "name": "my_collection",
    "metadata": {
      "description": "My test collection",
      "custom_field": "custom_value"
    },
    "dimension": null,
    "tenant": "default_tenant",
    "database": "default_database"
  }
}
```

**VexFS Implementation**:
```python
class CreateCollectionRequest(BaseModel):
    name: str
    metadata: Optional[Dict[str, Any]] = {}
    embedding_function: Optional[str] = "default"

@router.post("/api/v1/collections")
async def create_collection(request: CreateCollectionRequest):
    # Validate collection name
    if not re.match(r'^[a-zA-Z0-9_-]+$', request.name):
        raise HTTPException(400, "Invalid collection name")
    
    # Create VexFS directory structure
    collection_path = COLLECTIONS_BASE_PATH / request.name
    collection_path.mkdir(parents=True, exist_ok=False)
    
    # Initialize VexFS vector file
    vexfs_file = collection_path / "vectors.vexfs"
    vexfs_client = VexFSClient(str(vexfs_file))
    
    # Create collection metadata
    collection = ChromaDBCollection(
        id=str(uuid.uuid4()),
        name=request.name,
        metadata=request.metadata,
        dimension=None,
        tenant="default_tenant",
        database="default_database"
    )
    
    # Save metadata
    metadata_file = collection_path / "collection_metadata.json"
    with open(metadata_file, 'w') as f:
        json.dump(asdict(collection), f, indent=2)
    
    return {"success": True, "data": asdict(collection)}
```

##### GET /api/v1/collections
**Purpose**: List all collections  
**ChromaDB Compatibility**: ✅ Full compatibility

**Request**: No parameters

**Response**:
```json
{
  "success": true,
  "data": [
    {
      "id": "550e8400-e29b-41d4-a716-446655440000",
      "name": "collection1",
      "metadata": {"description": "First collection"},
      "dimension": 768,
      "tenant": "default_tenant",
      "database": "default_database"
    },
    {
      "id": "550e8400-e29b-41d4-a716-446655440001",
      "name": "collection2",
      "metadata": {"description": "Second collection"},
      "dimension": 384,
      "tenant": "default_tenant",
      "database": "default_database"
    }
  ]
}
```

##### GET /api/v1/collections/{collection_name}
**Purpose**: Get collection information  
**ChromaDB Compatibility**: ✅ Full compatibility

**Request**: Collection name in URL path

**Response**:
```json
{
  "success": true,
  "data": {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "name": "my_collection",
    "metadata": {"description": "My test collection"},
    "dimension": 768,
    "tenant": "default_tenant",
    "database": "default_database"
  }
}
```

##### DELETE /api/v1/collections/{collection_name}
**Purpose**: Delete a collection  
**ChromaDB Compatibility**: ✅ Full compatibility

**Request**: Collection name in URL path

**Response**:
```json
{
  "success": true,
  "data": "Collection deleted successfully"
}
```

#### 1.3 Document Operations

##### POST /api/v1/collections/{collection_name}/add
**Purpose**: Add documents to collection  
**ChromaDB Compatibility**: ✅ Full compatibility

**Request**:
```json
{
  "ids": ["doc1", "doc2", "doc3"],
  "embeddings": [
    [0.1, 0.2, 0.3, 0.4],
    [0.2, 0.3, 0.4, 0.5],
    [0.3, 0.4, 0.5, 0.6]
  ],
  "metadatas": [
    {"category": "tech", "author": "john", "year": 2024},
    {"category": "science", "author": "jane", "year": 2023},
    {"category": "tech", "author": "bob", "year": 2024}
  ],
  "documents": [
    "VexFS is a vector-extended filesystem for high-performance vector operations",
    "Machine learning advances enable more sophisticated AI applications",
    "Database optimization techniques improve query performance significantly"
  ]
}
```

**Response**:
```json
{
  "success": true,
  "data": {
    "ids": ["doc1", "doc2", "doc3"],
    "inserted_count": 3
  }
}
```

**VexFS Implementation**:
```python
class AddDocumentsRequest(BaseModel):
    ids: List[str]
    embeddings: Optional[List[List[float]]] = None
    metadatas: Optional[List[Dict[str, Any]]] = None
    documents: Optional[List[str]] = None

@router.post("/api/v1/collections/{collection_name}/add")
async def add_documents(collection_name: str, request: AddDocumentsRequest):
    # Validate collection exists
    collection = await get_collection_or_404(collection_name)
    
    # Generate embeddings if not provided
    if request.embeddings is None and request.documents is not None:
        embeddings = await generate_embeddings(request.documents)
    else:
        embeddings = request.embeddings
    
    # Validate dimensions
    if embeddings and collection.dimension is None:
        collection.dimension = len(embeddings[0])
        await update_collection_metadata(collection)
    
    # Store documents and metadata
    documents_path = COLLECTIONS_BASE_PATH / collection_name / "documents"
    documents_path.mkdir(exist_ok=True)
    
    vector_ids = []
    for i, doc_id in enumerate(request.ids):
        # Store document
        doc_data = {
            "id": doc_id,
            "content": request.documents[i] if request.documents else None,
            "metadata": request.metadatas[i] if request.metadatas else {},
            "created_at": datetime.utcnow().isoformat(),
            "vector_id": len(vector_ids)
        }
        
        doc_file = documents_path / f"{doc_id}.json"
        with open(doc_file, 'w') as f:
            json.dump(doc_data, f, indent=2)
        
        vector_ids.append(len(vector_ids))
    
    # Store vectors in VexFS
    vexfs_file = COLLECTIONS_BASE_PATH / collection_name / "vectors.vexfs"
    vexfs_client = VexFSClient(str(vexfs_file))
    
    # Convert embeddings to IEEE 754 bits
    vectors_bits = []
    for embedding in embeddings:
        vector_bits = [vexfs_float_to_bits(f) for f in embedding]
        vectors_bits.extend(vector_bits)
    
    # Batch insert into VexFS
    success = vexfs_client.batch_insert_vectors(
        vectors_bits=vectors_bits,
        vector_count=len(embeddings),
        dimensions=len(embeddings[0]),
        vector_ids=vector_ids
    )
    
    if not success:
        raise HTTPException(500, "Failed to insert vectors into VexFS")
    
    return {
        "success": True,
        "data": {
            "ids": request.ids,
            "inserted_count": len(request.ids)
        }
    }
```

##### POST /api/v1/collections/{collection_name}/query
**Purpose**: Query collection for similar documents  
**ChromaDB Compatibility**: ✅ Full compatibility

**Request**:
```json
{
  "query_embeddings": [[0.15, 0.25, 0.35, 0.45]],
  "n_results": 5,
  "where": {
    "category": "tech",
    "year": {"$gte": 2024}
  },
  "where_document": {
    "$contains": "VexFS"
  },
  "include": ["documents", "metadatas", "distances"]
}
```

**Response**:
```json
{
  "success": true,
  "data": {
    "ids": [["doc1", "doc3"]],
    "distances": [[0.1234, 0.5678]],
    "metadatas": [[
      {"category": "tech", "author": "john", "year": 2024},
      {"category": "tech", "author": "bob", "year": 2024}
    ]],
    "documents": [[
      "VexFS is a vector-extended filesystem for high-performance vector operations",
      "Database optimization techniques improve query performance significantly"
    ]]
  }
}
```

**VexFS Implementation**:
```python
class QueryRequest(BaseModel):
    query_embeddings: List[List[float]]
    n_results: Optional[int] = 10
    where: Optional[Dict[str, Any]] = None
    where_document: Optional[Dict[str, Any]] = None
    include: Optional[List[str]] = ["documents", "metadatas", "distances"]

@router.post("/api/v1/collections/{collection_name}/query")
async def query_collection(collection_name: str, request: QueryRequest):
    # Validate collection exists
    collection = await get_collection_or_404(collection_name)
    
    # Perform vector search using VexFS
    vexfs_file = COLLECTIONS_BASE_PATH / collection_name / "vectors.vexfs"
    vexfs_client = VexFSClient(str(vexfs_file))
    
    results = []
    for query_embedding in request.query_embeddings:
        # Convert query to IEEE 754 bits
        query_bits = [vexfs_float_to_bits(f) for f in query_embedding]
        
        # Search vectors
        vector_ids, distances_bits = vexfs_client.search_vectors(
            query_vector_bits=query_bits,
            k=request.n_results,
            search_type=VEXFS_SEARCH_COSINE
        )
        
        # Convert distances back to floats
        distances = [vexfs_bits_to_float(bits) for bits in distances_bits]
        
        # Load documents and apply filters
        filtered_results = await apply_filters_and_load_documents(
            collection_name, vector_ids, request.where, request.where_document
        )
        
        # Format results according to include parameter
        formatted_result = await format_query_results(
            filtered_results, distances, request.include
        )
        
        results.append(formatted_result)
    
    return {"success": True, "data": results[0] if len(results) == 1 else results}
```

### 2. ChromaDB Data Model Mapping

#### 2.1 Collection Storage Structure
```
collections/{collection_name}/
├── collection_metadata.json    # Collection configuration
├── documents/                  # Document storage
│   ├── doc_id_1.json          # Document content + metadata
│   ├── doc_id_2.json
│   └── ...
├── vectors.vexfs              # VexFS vector file
├── indices/                   # Search indices (optional)
│   ├── metadata_index.json   # Metadata index for filtering
│   └── document_index.json   # Document content index
└── cache/                     # Caching (optional)
    ├── embedding_cache.json  # Cached embeddings
    └── query_cache.json      # Cached query results
```

#### 2.2 Document Schema
```json
{
  "id": "doc_unique_id",
  "content": "Document text content",
  "metadata": {
    "category": "tech",
    "author": "john",
    "year": 2024,
    "custom_field": "custom_value"
  },
  "vector_id": 12345,
  "created_at": "2025-06-04T02:39:00Z",
  "updated_at": "2025-06-04T02:39:00Z"
}
```

#### 2.3 Collection Metadata Schema
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "name": "my_collection",
  "metadata": {
    "description": "Collection description",
    "custom_field": "custom_value"
  },
  "dimension": 768,
  "tenant": "default_tenant",
  "database": "default_database",
  "created_at": "2025-06-04T02:39:00Z",
  "updated_at": "2025-06-04T02:39:00Z",
  "vector_count": 1000,
  "embedding_function": "nomic-embed-text"
}
```

## Qdrant API Compatibility Specification

### 1. Qdrant API Endpoints

#### 1.1 Collection Management

##### PUT /collections/{collection_name}
**Purpose**: Create a new collection  
**Qdrant Compatibility**: ✅ Full compatibility

**Request**:
```json
{
  "vectors": {
    "size": 768,
    "distance": "Cosine"
  },
  "optimizers_config": {
    "default_segment_number": 2,
    "max_segment_size": 20000,
    "memmap_threshold": 50000,
    "indexing_threshold": 20000,
    "flush_interval_sec": 5,
    "max_optimization_threads": 1
  },
  "replication_factor": 1,
  "write_consistency_factor": 1,
  "on_disk_payload": true
}
```

**Response**:
```json
{
  "result": true,
  "status": "ok",
  "time": 0.031
}
```

**VexFS Implementation**:
```python
class VectorConfig(BaseModel):
    size: int
    distance: str = "Cosine"

class OptimizersConfig(BaseModel):
    default_segment_number: Optional[int] = 2
    max_segment_size: Optional[int] = 20000
    memmap_threshold: Optional[int] = 50000
    indexing_threshold: Optional[int] = 20000
    flush_interval_sec: Optional[int] = 5
    max_optimization_threads: Optional[int] = 1

class CreateCollectionRequest(BaseModel):
    vectors: VectorConfig
    optimizers_config: Optional[OptimizersConfig] = None
    replication_factor: Optional[int] = 1
    write_consistency_factor: Optional[int] = 1
    on_disk_payload: Optional[bool] = True

@router.put("/collections/{collection_name}")
async def create_collection(collection_name: str, request: CreateCollectionRequest):
    start_time = time.time()
    
    # Create VexFS directory structure
    collection_path = COLLECTIONS_BASE_PATH / collection_name
    collection_path.mkdir(parents=True, exist_ok=False)
    
    # Initialize VexFS vector file with proper dimensions
    vexfs_file = collection_path / "vectors.vexfs"
    vexfs_client = VexFSClient(str(vexfs_file))
    
    # Set vector metadata
    vector_info = VexFSVectorFileInfo(
        dimensions=request.vectors.size,
        element_type=VEXFS_VECTOR_FLOAT32,
        vector_count=0,
        storage_format=VEXFS_STORAGE_DENSE,
        data_offset=0,
        index_offset=0,
        compression_type=VEXFS_COMPRESS_NONE,
        alignment_bytes=32
    )
    
    success = vexfs_client.set_vector_metadata(vector_info)
    if not success:
        raise HTTPException(500, "Failed to initialize VexFS vector file")
    
    # Save collection configuration
    config = {
        "name": collection_name,
        "vectors": request.vectors.dict(),
        "optimizers_config": request.optimizers_config.dict() if request.optimizers_config else {},
        "replication_factor": request.replication_factor,
        "write_consistency_factor": request.write_consistency_factor,
        "on_disk_payload": request.on_disk_payload,
        "created_at": datetime.utcnow().isoformat()
    }
    
    config_file = collection_path / "collection_config.json"
    with open(config_file, 'w') as f:
        json.dump(config, f, indent=2)
    
    elapsed_time = time.time() - start_time
    
    return {
        "result": True,
        "status": "ok",
        "time": elapsed_time
    }
```

##### GET /collections/{collection_name}
**Purpose**: Get collection information  
**Qdrant Compatibility**: ✅ Full compatibility

**Response**:
```json
{
  "result": {
    "status": "green",
    "optimizer_status": "ok",
    "vectors_count": 1000,
    "indexed_vectors_count": 1000,
    "points_count": 1000,
    "segments_count": 2,
    "config": {
      "params": {
        "vectors": {
          "size": 768,
          "distance": "Cosine"
        },
        "replication_factor": 1,
        "write_consistency_factor": 1,
        "on_disk_payload": true
      },
      "optimizer_config": {
        "deleted_threshold": 0.2,
        "vacuum_min_vector_number": 1000,
        "default_segment_number": 2,
        "max_segment_size": 20000,
        "memmap_threshold": 50000,
        "indexing_threshold": 20000,
        "flush_interval_sec": 5,
        "max_optimization_threads": 1
      }
    }
  },
  "status": "ok",
  "time": 0.002
}
```

#### 1.2 Point Operations

##### PUT /collections/{collection_name}/points
**Purpose**: Insert or update points  
**Qdrant Compatibility**: ✅ Full compatibility

**Request**:
```json
{
  "points": [
    {
      "id": 1,
      "vector": [0.1, 0.2, 0.3, 0.4],
      "payload": {
        "category": "tech",
        "title": "VexFS Introduction",
        "author": "john",
        "year": 2024
      }
    },
    {
      "id": 2,
      "vector": [0.5, 0.6, 0.7, 0.8],
      "payload": {
        "category": "science",
        "title": "ML Advances",
        "author": "jane",
        "year": 2023
      }
    }
  ]
}
```

**Response**:
```json
{
  "result": {
    "operation_id": 123,
    "status": "completed"
  },
  "status": "ok",
  "time": 0.042
}
```

**VexFS Implementation**:
```python
class QdrantPoint(BaseModel):
    id: Union[int, str]
    vector: List[float]
    payload: Optional[Dict[str, Any]] = {}

class UpsertPointsRequest(BaseModel):
    points: List[QdrantPoint]

@router.put("/collections/{collection_name}/points")
async def upsert_points(collection_name: str, request: UpsertPointsRequest):
    start_time = time.time()
    
    # Validate collection exists
    collection_config = await get_collection_config_or_404(collection_name)
    
    # Prepare vectors for VexFS
    vectors_bits = []
    vector_ids = []
    
    for point in request.points:
        # Convert vector to IEEE 754 bits
        vector_bits = [vexfs_float_to_bits(f) for f in point.vector]
        vectors_bits.extend(vector_bits)
        vector_ids.append(point.id)
        
        # Store payload
        payload_path = COLLECTIONS_BASE_PATH / collection_name / "points"
        payload_path.mkdir(exist_ok=True)
        
        payload_data = {
            "id": point.id,
            "payload": point.payload,
            "created_at": datetime.utcnow().isoformat(),
            "updated_at": datetime.utcnow().isoformat()
        }
        
        payload_file = payload_path / f"{point.id}.json"
        with open(payload_file, 'w') as f:
            json.dump(payload_data, f, indent=2)
    
    # Batch insert into VexFS
    vexfs_file = COLLECTIONS_BASE_PATH / collection_name / "vectors.vexfs"
    vexfs_client = VexFSClient(str(vexfs_file))
    
    success = vexfs_client.batch_insert_vectors(
        vectors_bits=vectors_bits,
        vector_count=len(request.points),
        dimensions=len(request.points[0].vector),
        vector_ids=vector_ids
    )
    
    if not success:
        raise HTTPException(500, "Failed to insert vectors into VexFS")
    
    elapsed_time = time.time() - start_time
    operation_id = int(time.time() * 1000)  # Simple operation ID
    
    return {
        "result": {
            "operation_id": operation_id,
            "status": "completed"
        },
        "status": "ok",
        "time": elapsed_time
    }
```

##### POST /collections/{collection_name}/points/search
**Purpose**: Search for similar points  
**Qdrant Compatibility**: ✅ Full compatibility

**Request**:
```json
{
  "vector": [0.15, 0.25, 0.35, 0.45],
  "limit": 5,
  "filter": {
    "must": [
      {
        "key": "category",
        "match": {
          "value": "tech"
        }
      },
      {
        "key": "year",
        "range": {
          "gte": 2024
        }
      }
    ]
  },
  "with_payload": true,
  "with_vector": false,
  "score_threshold": 0.5
}
```

**Response**:
```json
{
  "result": [
    {
      "id": 1,
      "score": 0.9876,
      "payload": {
        "category": "tech",
        "title": "VexFS Introduction",
        "author": "john",
        "year": 2024
      }
    },
    {
      "id": 3,
      "score": 0.8765,
      "payload": {
        "category": "tech",
        "title": "Database Optimization",
        "author": "bob",
        "year": 2024
      }
    }
  ],
  "status": "ok",
  "time": 0.018
}
```

### 2. Qdrant Data Model Mapping

#### 2.1 Collection Storage Structure
```
collections/{collection_name}/
├── collection_config.json     # Collection configuration
├── points/                    # Point payload storage
│   ├── 1.json                # Point 1 payload
│   ├── 2.json                # Point 2 payload
│   └── ...
├── vectors.vexfs             # VexFS vector file
├── indices/                  # Search indices
│   ├── payload_index.json   # Payload index for filtering
│   └── vector_index.vexfs   # Vector index
└── operations/               # Operation logs (optional)
    └── operations.log       # Operation history
```

#### 2.2 Point Payload Schema
```json
{
  "id": 1,
  "payload": {
    "category": "tech",
    "title": "VexFS Introduction",
    "author": "john",
    "year": 2024,
    "tags": ["filesystem", "vector", "database"],
    "metadata": {
      "source": "documentation",
      "version": "1.0"
    }
  },
  "created_at": "2025-06-04T02:39:00Z",
  "updated_at": "2025-06-04T02:39:00Z"
}
```

#### 2.3 Collection Configuration Schema
```json
{
  "name": "my_collection",
  "vectors": {
    "size": 768,
    "distance": "Cosine"
  },
  "optimizers_config": {
    "deleted_threshold": 0.2,
    "vacuum_min_vector_number": 1000,
    "default_segment_number": 2,
    "max_segment_size": 20000,
    "memmap_threshold": 50000,
    "indexing_threshold": 20000,
    "flush_interval_sec": 5,
    "max_optimization_threads": 1
  },
  "replication_factor": 1,
  "write_consistency_factor": 1,
  "on_disk_payload": true,
  "created_at": "2025-06-04T02:39:00Z",
  "point_count": 1000,
  "vector_count": 1000
}
```

## Performance Requirements

### 1. Latency Requirements

| Operation | Target Latency | VexFS Baseline | Implementation Strategy |
|-----------|---------------|----------------|------------------------|
| **Collection Create** | <100ms | N/A | Direct filesystem operations |
| **Document/Point Insert** | <10ms | 0.03ms | Leverage VexFS batch insert |
| **Vector Search** | <10ms | TBD | Optimize IOCTL calls |
| **Metadata Query** | <5ms | N/A | In-memory indexing |
| **Batch Operations** | <1ms per item | 0.00003ms | VexFS batch optimization |

### 2. Throughput Requirements

| Operation | Target Throughput | VexFS Baseline | Implementation Strategy |
|-----------|------------------|----------------|------------------------|
| **Vector Insert** | 100K ops/sec | 3.27M ops/sec | ✅ Exceeds target |
| **Search Queries** | 10K queries/sec | TBD | Parallel processing |
| **Document Retrieval** | 50K docs/sec | TBD | Efficient file I/O |
| **Concurrent Connections** | 1000 connections | TBD | Async FastAPI |

### 3. Compatibility Requirements

| Database | API Coverage | Client Compatibility | Migration Effort |
|----------|--------------|---------------------|------------------|
| **ChromaDB** | 95%+ core APIs | Python, JS clients | Zero code changes |
| **Qdrant** | 90%+ core APIs | Python, JS, Rust clients | Zero code changes |

## Error Handling and Status Codes

### 1. ChromaDB Error Format
```json
{
  "success": false,
  "error": "Collection not found",
  "error_code": "COLLECTION_NOT_FOUND",
  "details": {
    "collection_name": "nonexistent_collection",
    "available_collections": ["collection1", "collection2"]
  }
}
```

### 2. Qdrant Error Format
```json
{
  "status": "error",
  "error": "Collection not found",
  "time": 0.001
}
```

### 3. HTTP Status Code Mapping

| Error Type | ChromaDB Status | Qdrant Status | Description |
|------------|----------------|---------------|-------------|
| **Collection Not Found** | 404 | 404 | Collection does not exist |
| **Invalid Request** | 400 | 400 | Malformed request data |
| **Dimension Mismatch** | 400 | 400 | Vector dimension mismatch |
| **Server Error** | 500 | 500 | Internal server error |
| **Rate Limited** | 429 | 429 | Too many requests |

## Testing and Validation

### 1. Compatibility Test Suite

#### ChromaDB Compatibility Tests
```python
# Test with official ChromaDB client
def test_chromadb_client_compatibility():
    import chromadb
    
    client = chromadb.HttpClient(host="localhost", port=8000)
    
    # Test collection operations
    collection = client.create_collection("test_collection")
    assert collection.name == "test_collection"
    
    # Test document operations
    collection.add(
        ids=["