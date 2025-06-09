# VexGraph API Reference

Complete API documentation for the VexGraph Dashboard backend services.

## Table of Contents

1. [Overview](#overview)
2. [Authentication](#authentication)
3. [Base URLs and Endpoints](#base-urls-and-endpoints)
4. [Node Operations](#node-operations)
5. [Edge Operations](#edge-operations)
6. [Graph Traversal](#graph-traversal)
7. [Search Operations](#search-operations)
8. [Analytics Operations](#analytics-operations)
9. [Schema Management](#schema-management)
10. [Real-time Operations](#real-time-operations)
11. [Batch Operations](#batch-operations)
12. [Error Handling](#error-handling)
13. [Rate Limiting](#rate-limiting)
14. [SDK and Client Libraries](#sdk-and-client-libraries)

## Overview

The VexGraph API provides comprehensive access to graph data stored in the VexFS filesystem. It supports RESTful operations for CRUD operations, advanced graph traversal algorithms, semantic search capabilities, and real-time collaboration features.

### API Versioning

- **Current Version**: v1
- **Base Path**: `/api/v1/vexgraph`
- **Content Type**: `application/json`
- **Response Format**: JSON

### Response Format

All API responses follow a consistent format:

```json
{
  "success": true,
  "data": { /* response data */ },
  "meta": { /* metadata like pagination */ },
  "timestamp": "2025-01-01T00:00:00Z"
}
```

Error responses:
```json
{
  "success": false,
  "error": "Error message",
  "code": "ERROR_CODE",
  "details": { /* additional error details */ },
  "timestamp": "2025-01-01T00:00:00Z"
}
```

## Authentication

### API Key Authentication

Include your API key in the request headers:

```http
Authorization: Bearer your-api-key-here
```

### Session-based Authentication

For web applications, use session cookies:

```http
Cookie: session=your-session-token
```

## Base URLs and Endpoints

### Production
- **REST API**: `https://api.vexfs.com/api/v1/vexgraph`
- **WebSocket**: `wss://ws.vexfs.com/vexgraph`

### Development
- **REST API**: `http://localhost:8080/api/v1/vexgraph`
- **WebSocket**: `ws://localhost:8080/vexgraph`

## Node Operations

### Create Node

Create a new node in the graph.

**Endpoint**: `POST /nodes`

**Request Body**:
```json
{
  "inode_number": 12345,
  "node_type": "File",
  "properties": {
    "name": "example.txt",
    "size": 1024,
    "created_at": "2025-01-01T00:00:00Z"
  }
}
```

**Response**:
```json
{
  "success": true,
  "data": {
    "id": "node-abc123",
    "inode_number": 12345,
    "node_type": "File",
    "properties": {
      "name": "example.txt",
      "size": 1024,
      "created_at": "2025-01-01T00:00:00Z"
    },
    "outgoing_edges": [],
    "incoming_edges": [],
    "created_at": "2025-01-01T00:00:00Z",
    "updated_at": "2025-01-01T00:00:00Z"
  }
}
```

### Get Node

Retrieve a specific node by ID.

**Endpoint**: `GET /nodes/{nodeId}`

**Parameters**:
- `nodeId` (path): Unique node identifier

**Response**:
```json
{
  "success": true,
  "data": {
    "id": "node-abc123",
    "inode_number": 12345,
    "node_type": "File",
    "properties": {
      "name": "example.txt",
      "size": 1024
    },
    "outgoing_edges": ["edge-def456"],
    "incoming_edges": ["edge-ghi789"],
    "created_at": "2025-01-01T00:00:00Z",
    "updated_at": "2025-01-01T00:00:00Z"
  }
}
```

### List Nodes

Retrieve a paginated list of nodes with optional filtering.

**Endpoint**: `GET /nodes`

**Query Parameters**:
- `limit` (integer, optional): Number of results per page (default: 100, max: 1000)
- `offset` (integer, optional): Number of results to skip (default: 0)
- `node_type` (string, optional): Filter by node type
- `property_filter` (string, optional): JSON-encoded property filters
- `sort_by` (string, optional): Sort field (default: created_at)
- `sort_order` (string, optional): Sort order (asc/desc, default: desc)

**Example Request**:
```http
GET /nodes?limit=50&offset=0&node_type=File&sort_by=name&sort_order=asc
```

**Response**:
```json
{
  "success": true,
  "data": {
    "items": [
      {
        "id": "node-abc123",
        "node_type": "File",
        "properties": { "name": "file1.txt" }
      }
    ],
    "total": 150,
    "page": 1,
    "pageSize": 50,
    "hasNext": true,
    "hasPrev": false
  }
}
```

### Update Node

Update an existing node's properties.

**Endpoint**: `PATCH /nodes/{nodeId}`

**Request Body**:
```json
{
  "properties": {
    "name": "updated-file.txt",
    "size": 2048
  }
}
```

**Response**:
```json
{
  "success": true,
  "data": {
    "id": "node-abc123",
    "inode_number": 12345,
    "node_type": "File",
    "properties": {
      "name": "updated-file.txt",
      "size": 2048
    },
    "updated_at": "2025-01-01T01:00:00Z"
  }
}
```

### Delete Node

Delete a node and all its associated edges.

**Endpoint**: `DELETE /nodes/{nodeId}`

**Response**:
```json
{
  "success": true,
  "data": {
    "deleted_node_id": "node-abc123",
    "deleted_edges": ["edge-def456", "edge-ghi789"]
  }
}
```

## Edge Operations

### Create Edge

Create a new edge between two nodes.

**Endpoint**: `POST /edges`

**Request Body**:
```json
{
  "source_id": "node-abc123",
  "target_id": "node-def456",
  "edge_type": "Contains",
  "weight": 1.0,
  "properties": {
    "relationship": "parent-child",
    "strength": 0.95
  }
}
```

**Response**:
```json
{
  "success": true,
  "data": {
    "id": "edge-xyz789",
    "source_id": "node-abc123",
    "target_id": "node-def456",
    "edge_type": "Contains",
    "weight": 1.0,
    "properties": {
      "relationship": "parent-child",
      "strength": 0.95
    },
    "created_at": "2025-01-01T00:00:00Z",
    "updated_at": "2025-01-01T00:00:00Z"
  }
}
```

### Get Edge

Retrieve a specific edge by ID.

**Endpoint**: `GET /edges/{edgeId}`

### List Edges

Retrieve a paginated list of edges with optional filtering.

**Endpoint**: `GET /edges`

**Query Parameters**:
- `limit`, `offset`: Pagination parameters
- `edge_type`: Filter by edge type
- `source_id`: Filter by source node
- `target_id`: Filter by target node
- `min_weight`, `max_weight`: Filter by weight range

### Update Edge

Update an existing edge's properties.

**Endpoint**: `PATCH /edges/{edgeId}`

### Delete Edge

Delete an edge.

**Endpoint**: `DELETE /edges/{edgeId}`

## Graph Traversal

### Execute Traversal

Perform graph traversal using various algorithms.

**Endpoint**: `POST /traversal`

**Request Body**:
```json
{
  "algorithm": "BreadthFirstSearch",
  "start_node": "node-abc123",
  "end_node": "node-def456",
  "max_depth": 5,
  "max_results": 100,
  "filters": {
    "node_types": ["File", "Directory"],
    "edge_types": ["Contains", "References"],
    "property_filters": {
      "size": { "min": 1024, "max": 1048576 }
    }
  },
  "include_paths": true,
  "include_weights": true
}
```

**Supported Algorithms**:
- `BreadthFirstSearch`: Explore nodes level by level
- `DepthFirstSearch`: Explore as far as possible along each branch
- `ShortestPath`: Find shortest path between nodes (Dijkstra's algorithm)
- `AllShortestPaths`: Find all shortest paths between nodes
- `PageRank`: Calculate PageRank scores
- `ConnectedComponents`: Find connected components
- `CommunityDetection`: Detect communities using modularity optimization

**Response**:
```json
{
  "success": true,
  "data": {
    "algorithm": "BreadthFirstSearch",
    "start_node": "node-abc123",
    "end_node": "node-def456",
    "path": ["node-abc123", "node-ghi789", "node-def456"],
    "visited_nodes": ["node-abc123", "node-ghi789", "node-def456", "node-jkl012"],
    "traversed_edges": ["edge-xyz789", "edge-uvw345"],
    "total_weight": 2.5,
    "execution_time_ms": 45,
    "success": true,
    "metadata": {
      "nodes_explored": 15,
      "edges_traversed": 8,
      "max_depth_reached": 3
    }
  }
}
```

### Get Node Neighbors

Get neighboring nodes of a specific node.

**Endpoint**: `GET /nodes/{nodeId}/neighbors`

**Query Parameters**:
- `direction`: `incoming`, `outgoing`, or `both` (default: both)
- `max_depth`: Maximum traversal depth (default: 1)
- `edge_types`: Comma-separated list of edge types to follow
- `limit`: Maximum number of neighbors to return

**Response**:
```json
{
  "success": true,
  "data": {
    "neighbors": [
      {
        "node": {
          "id": "node-def456",
          "node_type": "Directory",
          "properties": { "name": "documents" }
        },
        "edge": {
          "id": "edge-xyz789",
          "edge_type": "Contains",
          "weight": 1.0
        },
        "distance": 1,
        "path": ["node-abc123", "node-def456"]
      }
    ],
    "total_neighbors": 5,
    "max_depth_reached": 2
  }
}
```

## Search Operations

### Semantic Search

Perform AI-powered semantic search across graph nodes.

**Endpoint**: `POST /search`

**Request Body**:
```json
{
  "query": "configuration files and settings",
  "search_type": "semantic",
  "max_results": 20,
  "min_relevance": 0.7,
  "filters": {
    "node_types": ["File"],
    "property_filters": {
      "size": { "min": 100 }
    }
  },
  "include_embeddings": false,
  "include_explanations": true
}
```

**Response**:
```json
{
  "success": true,
  "data": {
    "nodes": [
      {
        "id": "node-abc123",
        "node_type": "File",
        "properties": { "name": "app.config" },
        "relevance_score": 0.95,
        "explanation": "Matches 'configuration' keyword and semantic context"
      }
    ],
    "edges": [],
    "relevance_scores": {
      "node-abc123": 0.95,
      "node-def456": 0.87
    },
    "execution_time_ms": 120,
    "total_results": 15,
    "query_embedding": [0.1, 0.2, 0.3, "..."],
    "search_metadata": {
      "model_version": "v2.1",
      "embedding_dimension": 768,
      "similarity_metric": "cosine"
    }
  }
}
```

### Property Search

Search nodes based on property values.

**Endpoint**: `POST /search`

**Request Body**:
```json
{
  "query": "*.conf",
  "search_type": "property",
  "property_name": "name",
  "match_type": "glob",
  "node_types": ["File"],
  "max_results": 50
}
```

**Match Types**:
- `exact`: Exact string match
- `contains`: Substring match
- `regex`: Regular expression match
- `glob`: Glob pattern match
- `fuzzy`: Fuzzy string matching

### Full-text Search

Search across all text properties of nodes.

**Endpoint**: `POST /search`

**Request Body**:
```json
{
  "query": "database configuration",
  "search_type": "fulltext",
  "max_results": 30,
  "highlight": true,
  "fuzzy": true,
  "boost_fields": {
    "name": 2.0,
    "description": 1.5,
    "content": 1.0
  }
}
```

## Analytics Operations

### Graph Statistics

Get comprehensive graph statistics.

**Endpoint**: `GET /stats`

**Response**:
```json
{
  "success": true,
  "data": {
    "node_count": 1500,
    "edge_count": 3200,
    "node_types": {
      "File": 1200,
      "Directory": 250,
      "Symlink": 45,
      "Device": 5
    },
    "edge_types": {
      "Contains": 1800,
      "References": 900,
      "DependsOn": 400,
      "SimilarTo": 100
    },
    "average_degree": 4.27,
    "density": 0.0014,
    "connected_components": 3,
    "largest_component_size": 1450,
    "clustering_coefficient": 0.23,
    "diameter": 12,
    "radius": 6,
    "assortativity": -0.15
  }
}
```

### Node Statistics

Get statistics for a specific node.

**Endpoint**: `GET /nodes/{nodeId}/stats`

**Response**:
```json
{
  "success": true,
  "data": {
    "id": "node-abc123",
    "degree": 8,
    "in_degree": 3,
    "out_degree": 5,
    "clustering_coefficient": 0.45,
    "betweenness_centrality": 0.023,
    "closeness_centrality": 0.67,
    "eigenvector_centrality": 0.12,
    "pagerank": 0.0015,
    "local_clustering": 0.33,
    "eccentricity": 8
  }
}
```

### Graph Analytics

Get advanced analytics including centrality measures and community detection.

**Endpoint**: `GET /analytics`

**Query Parameters**:
- `include_centrality`: Include centrality measures (default: true)
- `include_communities`: Include community detection (default: true)
- `include_paths`: Include path analysis (default: false)
- `algorithm`: Community detection algorithm (modularity, louvain, leiden)

**Response**:
```json
{
  "success": true,
  "data": {
    "degree_distribution": [
      { "degree": 1, "count": 150 },
      { "degree": 2, "count": 300 },
      { "degree": 3, "count": 250 }
    ],
    "centrality_measures": {
      "betweenness": {
        "node-abc123": 0.023,
        "node-def456": 0.045
      },
      "closeness": {
        "node-abc123": 0.67,
        "node-def456": 0.72
      },
      "eigenvector": {
        "node-abc123": 0.12,
        "node-def456": 0.18
      },
      "pagerank": {
        "node-abc123": 0.0015,
        "node-def456": 0.0023
      }
    },
    "clustering_coefficients": {
      "node-abc123": 0.45,
      "node-def456": 0.33
    },
    "shortest_paths_stats": {
      "average_path_length": 4.2,
      "diameter": 12,
      "radius": 6,
      "characteristic_path_length": 4.5
    },
    "community_detection": {
      "algorithm": "modularity",
      "communities": [
        ["node-abc123", "node-def456", "node-ghi789"],
        ["node-jkl012", "node-mno345"]
      ],
      "modularity": 0.42,
      "num_communities": 8,
      "community_sizes": [150, 200, 180, 120, 90, 80, 70, 60]
    }
  }
}
```

## Schema Management

### Get Schema

Retrieve the current graph schema.

**Endpoint**: `GET /schema`

**Response**:
```json
{
  "success": true,
  "data": {
    "version": "1.2.0",
    "node_types": [
      {
        "type": "File",
        "required_properties": ["name"],
        "optional_properties": ["size", "created_at", "modified_at"],
        "property_types": {
          "name": "String",
          "size": "Integer",
          "created_at": "DateTime",
          "modified_at": "DateTime"
        },
        "validation_rules": {
          "name": {
            "pattern": "^[a-zA-Z0-9._-]+$",
            "max_length": 255
          },
          "size": {
            "min_value": 0,
            "max_value": 1099511627776
          }
        }
      }
    ],
    "edge_types": [
      {
        "type": "Contains",
        "allowed_source_types": ["Directory"],
        "allowed_target_types": ["File", "Directory", "Symlink"],
        "required_properties": [],
        "optional_properties": ["relationship_strength"],
        "property_types": {
          "relationship_strength": "Float"
        },
        "directional": true,
        "allow_self_loops": false,
        "allow_multiple": false
      }
    ],
    "created_at": "2025-01-01T00:00:00Z",
    "updated_at": "2025-01-01T12:00:00Z"
  }
}
```

### Update Schema

Update the graph schema.

**Endpoint**: `PATCH /schema`

**Request Body**:
```json
{
  "version": "1.3.0",
  "node_types": [
    {
      "type": "ConfigFile",
      "required_properties": ["name", "format"],
      "optional_properties": ["encoding", "size"],
      "property_types": {
        "name": "String",
        "format": "String",
        "encoding": "String",
        "size": "Integer"
      }
    }
  ],
  "migration_strategy": "additive"
}
```

**Migration Strategies**:
- `additive`: Add new types without affecting existing data
- `strict`: Validate all existing data against new schema
- `graceful`: Apply schema to new data, leave existing data unchanged

### Validate Schema

Validate existing data against the current schema.

**Endpoint**: `POST /schema/validate`

**Response**:
```json
{
  "success": true,
  "data": {
    "valid": false,
    "validation_errors": [
      {
        "node_id": "node-abc123",
        "error_type": "missing_required_property",
        "property": "name",
        "message": "Required property 'name' is missing"
      },
      {
        "edge_id": "edge-def456",
        "error_type": "invalid_edge_type",
        "source_type": "File",
        "target_type": "Directory",
        "edge_type": "Contains",
        "message": "Edge type 'Contains' not allowed between File and Directory"
      }
    ],
    "total_errors": 15,
    "nodes_validated": 1500,
    "edges_validated": 3200,
    "validation_time_ms": 2300
  }
}
```

## Real-time Operations

### WebSocket Connection

Connect to the WebSocket endpoint for real-time updates.

**Connection URL**: `ws://localhost:8080/vexgraph`

**Authentication**: Include API key in connection query:
```
ws://localhost:8080/vexgraph?token=your-api-key
```

### Event Types

**Node Events**:
```json
{
  "type": "node_created",
  "data": {
    "node": { /* node object */ },
    "timestamp": "2025-01-01T00:00:00Z",
    "user_id": "user-123"
  }
}

{
  "type": "node_updated",
  "data": {
    "node_id": "node-abc123",
    "changes": {
      "properties": {
        "name": "new-name.txt"
      }
    },
    "timestamp": "2025-01-01T00:00:00Z",
    "user_id": "user-123"
  }
}

{
  "type": "node_deleted",
  "data": {
    "node_id": "node-abc123",
    "timestamp": "2025-01-01T00:00:00Z",
    "user_id": "user-123"
  }
}
```

**Edge Events**:
```json
{
  "type": "edge_created",
  "data": {
    "edge": { /* edge object */ },
    "timestamp": "2025-01-01T00:00:00Z",
    "user_id": "user-123"
  }
}
```

**System Events**:
```json
{
  "type": "user_joined",
  "data": {
    "user_id": "user-456",
    "user_name": "John Doe",
    "timestamp": "2025-01-01T00:00:00Z"
  }
}

{
  "type": "user_left",
  "data": {
    "user_id": "user-456",
    "timestamp": "2025-01-01T00:00:00Z"
  }
}
```

### Subscribing to Events

Send subscription messages to receive specific event types:

```json
{
  "action": "subscribe",
  "events": ["node_created", "node_updated", "edge_created"],
  "filters": {
    "node_types": ["File"],
    "user_id": "user-123"
  }
}
```

### Sending Commands

Send commands through WebSocket for real-time operations:

```json
{
  "action": "create_node",
  "data": {
    "node_type": "File",
    "properties": {
      "name": "realtime-file.txt"
    }
  },
  "request_id": "req-123"
}
```

## Batch Operations

### Batch Create Nodes

Create multiple nodes in a single request.

**Endpoint**: `POST /nodes/batch`

**Request Body**:
```json
{
  "nodes": [
    {
      "inode_number": 12345,
      "node_type": "File",
      "properties": { "name": "file1.txt" }
    },
    {
      "inode_number": 12346,
      "node_type": "File",
      "properties": { "name": "file2.txt" }
    }
  ]
}
```

**Response**:
```json
{
  "success": true,
  "data": {
    "created_nodes": [
      { "id": "node-abc123", /* ... */ },
      { "id": "node-def456", /* ... */ }
    ],
    "failed_nodes": [],
    "total_created": 2,
    "total_failed": 0
  }
}
```

### Batch Create Edges

Create multiple edges in a single request.

**Endpoint**: `POST /edges/batch`

### Batch Delete

Delete multiple nodes or edges.

**Endpoint**: `DELETE /nodes/batch`

**Request Body**:
```json
{
  "node_ids": ["node-abc123", "node-def456", "node-ghi789"]
}
```

**Endpoint**: `DELETE /edges/batch`

**Request Body**:
```json
{
  "edge_ids": ["edge-xyz789", "edge-uvw345"]
}
```

## Error Handling

### HTTP Status Codes

- `200 OK`: Successful request
- `201 Created`: Resource created successfully
- `400 Bad Request`: Invalid request parameters
- `401 Unauthorized`: Authentication required
- `403 Forbidden`: Insufficient permissions
- `404 Not Found`: Resource not found
- `409 Conflict`: Resource conflict (e.g., duplicate)
- `422 Unprocessable Entity`: Validation errors
- `429 Too Many Requests`: Rate limit exceeded
- `500 Internal Server Error`: Server error
- `503 Service Unavailable`: Service temporarily unavailable

### Error Response Format

```json
{
  "success": false,
  "error": "Validation failed",
  "code": "VALIDATION_ERROR",
  "details": {
    "field": "name",
    "message": "Name is required",
    "value": null
  },
  "timestamp": "2025-01-01T00:00:00Z",
  "request_id": "req-abc123"
}
```

### Common Error Codes

- `VALIDATION_ERROR`: Request validation failed
- `NOT_FOUND`: Resource not found
- `DUPLICATE_RESOURCE`: Resource already exists
- `SCHEMA_VIOLATION`: Data violates schema constraints
- `CIRCULAR_DEPENDENCY`: Operation would create circular dependency
- `INSUFFICIENT_PERMISSIONS`: User lacks required permissions
- `RATE_LIMIT_EXCEEDED`: Too many requests
- `SERVICE_UNAVAILABLE`: Backend service unavailable

## Rate Limiting

### Rate Limit Headers

All responses include rate limiting headers:

```http
X-RateLimit-Limit: 1000
X-RateLimit-Remaining: 999
X-RateLimit-Reset: 1640995200
X-RateLimit-Window: 3600
```

### Rate Limits

- **Standard API**: 1000 requests per hour
- **Search API**: 100 requests per hour
- **Batch Operations**: 50 requests per hour
- **WebSocket**: 10 connections per user

### Rate Limit Exceeded

When rate limit is exceeded:

```json
{
  "success": false,
  "error": "Rate limit exceeded",
  "code": "RATE_LIMIT_EXCEEDED",
  "details": {
    "limit": 1000,
    "window": 3600,
    "reset_at": "2025-01-01T01:00:00Z"
  }
}
```

## SDK and Client Libraries

### JavaScript/TypeScript

```bash
npm install @vexfs/graph-client
```

```typescript
import { VexGraphClient } from '@vexfs/graph-client';

const client = new VexGraphClient({
  baseURL: 'https://api.vexfs.com',
  apiKey: 'your-api-key'
});

// Create a node
const node = await client.nodes.create({
  node_type: 'File',
  properties: { name: 'example.txt' }
});

// Execute traversal
const result = await client.traversal.execute({
  algorithm: 'BreadthFirstSearch',
  start_node: node.id,
  max_depth: 3
});
```

### Python

```bash
pip install vexfs-graph-client
```

```python
from vexfs_graph import VexGraphClient

client = VexGraphClient(
    base_url='https://api.vexfs.com',
    api_key='your-api-key'
)

# Create a node
node = client.nodes.create(
    node_type='File',
    properties={'name': 'example.txt'}
)

# Execute traversal
result = client.traversal.execute(
    algorithm='BreadthFirstSearch',
    start_node=node['id'],
    max_depth=3
)
```

### cURL Examples

**Create Node**:
```bash
curl -X POST https://api.vexfs.com/api/v1/vexgraph/nodes \
  -H "Authorization: Bearer your-api-key" \
  -H "Content-Type: application/json" \
  -d '{
    "node_type": "File",
    "properties": {
      "name": "example.txt",
      "size": 1024
    }
  }'
```

**Execute Traversal**:
```bash
curl -X POST https://api.vexfs.com/api/v1/vexgraph/traversal \
  -H "Authorization: Bearer your-api-key" \
  -H "Content-Type: application/json" \
  -d '{
    "algorithm": "BreadthFirstSearch",
    "start_node": "node-abc123",
    "max_depth": 5
  }'
```

---

For additional support or questions about the API, please refer to the [Developer Guide](../developer/README.md) or contact our support team.