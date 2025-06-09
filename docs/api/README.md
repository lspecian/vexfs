# VexFS API Documentation

Complete API reference for VexFS REST API, WebSocket API, and integration libraries.

## Quick Navigation

### API References
- [REST API](rest-api.md) - Complete REST API reference
- [WebSocket API](websocket-api.md) - Real-time WebSocket API
- [GraphQL API](graphql-api.md) - GraphQL query interface
- [gRPC API](grpc-api.md) - High-performance gRPC interface

### Integration Guides
- [Python Integration](python-integration.md) - Python client library
- [JavaScript Integration](javascript-integration.md) - Node.js and browser clients
- [Go Integration](go-integration.md) - Go client library
- [Rust Integration](rust-integration.md) - Native Rust integration

### Authentication and Security
- [Authentication](authentication.md) - API authentication methods
- [Authorization](authorization.md) - Role-based access control
- [Rate Limiting](rate-limiting.md) - API rate limiting and quotas
- [Security Best Practices](security.md) - API security guidelines

### Advanced Topics
- [Batch Operations](batch-operations.md) - Bulk data operations
- [Streaming](streaming.md) - Real-time data streaming
- [Webhooks](webhooks.md) - Event-driven integrations
- [Error Handling](error-handling.md) - Comprehensive error handling

## API Overview

VexFS provides multiple API interfaces for different use cases:

### REST API (Port 8080)
- **Purpose**: Standard HTTP API for CRUD operations
- **Format**: JSON request/response
- **Authentication**: Bearer tokens, API keys
- **Use Cases**: Web applications, mobile apps, general integration

### WebSocket API (Port 8081)
- **Purpose**: Real-time bidirectional communication
- **Format**: JSON messages over WebSocket
- **Authentication**: Token-based authentication
- **Use Cases**: Real-time updates, live search, streaming data

### GraphQL API (Port 8082)
- **Purpose**: Flexible query interface with schema introspection
- **Format**: GraphQL queries and mutations
- **Authentication**: Bearer tokens
- **Use Cases**: Complex queries, mobile applications, API exploration

### gRPC API (Port 8083)
- **Purpose**: High-performance binary protocol
- **Format**: Protocol Buffers
- **Authentication**: mTLS, JWT tokens
- **Use Cases**: Microservices, high-throughput applications

## Core Concepts

### Vector Operations
```json
{
  "vector": [0.1, 0.2, 0.3, 0.4],
  "metadata": {
    "id": "doc_123",
    "category": "document",
    "timestamp": "2025-01-08T12:00:00Z"
  }
}
```

### Search Queries
```json
{
  "query": {
    "vector": [0.1, 0.2, 0.3, 0.4],
    "k": 10,
    "filters": {
      "category": "document",
      "timestamp": {
        "gte": "2025-01-01T00:00:00Z"
      }
    }
  }
}
```

### Graph Operations
```json
{
  "nodes": [
    {"id": "node1", "properties": {"type": "document"}},
    {"id": "node2", "properties": {"type": "user"}}
  ],
  "edges": [
    {"from": "node1", "to": "node2", "type": "authored_by"}
  ]
}
```

## Quick Start Examples

### REST API Example
```bash
# Store a vector
curl -X POST http://localhost:8080/api/v1/vectors \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -d '{
    "id": "doc_123",
    "vector": [0.1, 0.2, 0.3, 0.4],
    "metadata": {"title": "Example Document"}
  }'

# Search vectors
curl -X POST http://localhost:8080/api/v1/search \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -d '{
    "vector": [0.1, 0.2, 0.3, 0.4],
    "k": 10
  }'
```

### WebSocket API Example
```javascript
const ws = new WebSocket('ws://localhost:8081/api/v1/stream');

ws.onopen = function() {
  // Authenticate
  ws.send(JSON.stringify({
    type: 'auth',
    token: 'YOUR_TOKEN'
  }));
  
  // Subscribe to search results
  ws.send(JSON.stringify({
    type: 'subscribe',
    channel: 'search_results'
  }));
};

ws.onmessage = function(event) {
  const data = JSON.parse(event.data);
  console.log('Received:', data);
};
```

### Python Client Example
```python
from vexfs import VexFSClient

# Initialize client
client = VexFSClient(
    base_url='http://localhost:8080',
    api_key='YOUR_API_KEY'
)

# Store vector
result = client.vectors.create(
    id='doc_123',
    vector=[0.1, 0.2, 0.3, 0.4],
    metadata={'title': 'Example Document'}
)

# Search vectors
results = client.search.vector(
    vector=[0.1, 0.2, 0.3, 0.4],
    k=10
)
```

## API Endpoints Overview

### Vector Operations
- `POST /api/v1/vectors` - Store vector
- `GET /api/v1/vectors/{id}` - Retrieve vector
- `PUT /api/v1/vectors/{id}` - Update vector
- `DELETE /api/v1/vectors/{id}` - Delete vector
- `POST /api/v1/vectors/batch` - Batch operations

### Search Operations
- `POST /api/v1/search` - Vector similarity search
- `POST /api/v1/search/hybrid` - Hybrid search (vector + text)
- `POST /api/v1/search/graph` - Graph traversal search
- `GET /api/v1/search/suggestions` - Search suggestions

### Graph Operations
- `POST /api/v1/graph/nodes` - Create nodes
- `POST /api/v1/graph/edges` - Create edges
- `GET /api/v1/graph/traverse` - Graph traversal
- `POST /api/v1/graph/query` - Graph queries

### System Operations
- `GET /api/v1/status` - System status
- `GET /api/v1/health` - Health check
- `GET /api/v1/metrics` - Performance metrics
- `GET /api/v1/info` - System information

## Authentication

### API Key Authentication
```bash
curl -H "X-API-Key: YOUR_API_KEY" http://localhost:8080/api/v1/status
```

### Bearer Token Authentication
```bash
curl -H "Authorization: Bearer YOUR_JWT_TOKEN" http://localhost:8080/api/v1/status
```

### OAuth2 Authentication
```bash
# Get access token
curl -X POST http://localhost:8080/oauth/token \
  -d "grant_type=client_credentials" \
  -d "client_id=YOUR_CLIENT_ID" \
  -d "client_secret=YOUR_CLIENT_SECRET"

# Use access token
curl -H "Authorization: Bearer ACCESS_TOKEN" http://localhost:8080/api/v1/status
```

## Rate Limiting

### Default Limits
- **Authenticated requests**: 1000 requests/minute
- **Unauthenticated requests**: 100 requests/minute
- **Batch operations**: 10 requests/minute
- **Search operations**: 500 requests/minute

### Rate Limit Headers
```http
X-RateLimit-Limit: 1000
X-RateLimit-Remaining: 999
X-RateLimit-Reset: 1641024000
```

## Error Handling

### Standard Error Response
```json
{
  "error": {
    "code": "INVALID_REQUEST",
    "message": "Vector dimension mismatch",
    "details": {
      "expected_dimension": 512,
      "provided_dimension": 256
    },
    "request_id": "req_123456789"
  }
}
```

### HTTP Status Codes
- `200` - Success
- `201` - Created
- `400` - Bad Request
- `401` - Unauthorized
- `403` - Forbidden
- `404` - Not Found
- `429` - Rate Limited
- `500` - Internal Server Error

## SDKs and Libraries

### Official SDKs
- **Python**: `pip install vexfs-python`
- **JavaScript**: `npm install vexfs-js`
- **Go**: `go get github.com/vexfs/vexfs-go`
- **Rust**: `cargo add vexfs`

### Community SDKs
- **Java**: `vexfs-java` (Maven Central)
- **C#**: `VexFS.NET` (NuGet)
- **PHP**: `vexfs/php-client` (Packagist)
- **Ruby**: `vexfs-ruby` (RubyGems)

## API Versioning

### Version Strategy
- **Current Version**: v1
- **Versioning**: URL path versioning (`/api/v1/`)
- **Backward Compatibility**: Maintained for 2 major versions
- **Deprecation Notice**: 6 months advance notice

### Version Headers
```http
API-Version: v1
API-Supported-Versions: v1, v2-beta
```

## Performance Considerations

### Optimization Tips
1. **Batch Operations**: Use batch endpoints for multiple operations
2. **Pagination**: Use pagination for large result sets
3. **Compression**: Enable gzip compression for large payloads
4. **Connection Pooling**: Reuse HTTP connections
5. **Caching**: Implement client-side caching where appropriate

### Performance Metrics
- **Latency**: P50 < 10ms, P99 < 100ms
- **Throughput**: 10,000+ requests/second
- **Availability**: 99.9% uptime SLA

## Getting Started

1. **[Authentication Setup](authentication.md)** - Configure API access
2. **[REST API Guide](rest-api.md)** - Learn the REST API
3. **[Client Libraries](python-integration.md)** - Use official SDKs
4. **[Examples Repository](https://github.com/vexfs/examples)** - Code examples

## Support and Resources

- **API Documentation**: [docs.vexfs.io/api](https://docs.vexfs.io/api)
- **Interactive API Explorer**: [api.vexfs.io](https://api.vexfs.io)
- **Community Forum**: [community.vexfs.io](https://community.vexfs.io)
- **GitHub Issues**: [github.com/vexfs/vexfs/issues](https://github.com/vexfs/vexfs/issues)