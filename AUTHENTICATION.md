# VexFS API Authentication Guide

## Overview

The VexFS Unified API Server includes a comprehensive authentication and authorization system based on JWT tokens and API keys. This provides secure access control for all API endpoints across ChromaDB, Qdrant, and Native VexFS APIs.

## Features

- **JWT Token Authentication**: Secure token-based authentication
- **API Key Support**: Direct API key authentication for programmatic access
- **Role-Based Access Control (RBAC)**: Three levels of access (Reader, Writer, Admin)
- **Collection-Level Permissions**: Fine-grained access control per collection
- **Rate Limiting**: Configurable rate limits per API key
- **Anonymous Access**: Optional read-only anonymous access

## Authentication Methods

### 1. API Key Authentication

Use the `X-API-Key` header with your API key:

```bash
curl -H "X-API-Key: your-api-key-here" \
     http://localhost:7680/api/v1/collections
```

### 2. JWT Token Authentication

First, exchange your API key for a JWT token:

```bash
# Login to get JWT token
TOKEN=$(curl -s -X POST http://localhost:7680/auth/login \
  -H "Content-Type: application/json" \
  -d '{"api_key": "your-api-key-here"}' | jq -r '.token')

# Use the token in subsequent requests
curl -H "Authorization: $TOKEN" \
     http://localhost:7680/api/v1/collections
```

### 3. Anonymous Access

If enabled (default), read-only operations work without authentication:

```bash
curl http://localhost:7680/api/v1/collections
```

## User Roles

### Reader
- Can read collections and documents
- Can perform searches
- Cannot create, modify, or delete data

### Writer
- All Reader permissions
- Can create collections
- Can add/update/delete documents
- Can modify vectors

### Admin
- All Writer permissions
- Can manage API keys
- Can access all collections
- Can perform administrative operations

## Configuration

### Environment Variables

```bash
# JWT secret key (required in production)
JWT_SECRET=your-secret-key-here

# Allow anonymous read access (default: true)
ALLOW_ANONYMOUS=true

# Production mode (disables default keys)
PRODUCTION=true

# API Keys (format: key:role:collections:rate_limit)
API_KEY_1=mykey123:admin::1000
API_KEY_2=readkey456:reader:collection1,collection2:100
API_KEY_3=writekey789:writer::500
```

### API Key Format

API keys are configured using environment variables with the format:
```
API_KEY_N=key:role:collections:rate_limit
```

- `key`: The actual API key string
- `role`: One of `reader`, `writer`, or `admin`
- `collections`: Comma-separated list of allowed collections (empty = all)
- `rate_limit`: Requests per minute limit (optional)

Examples:
```bash
# Admin with no restrictions
API_KEY_1=admin-key-123:admin::

# Reader limited to specific collections
API_KEY_2=reader-key-456:reader:products,users:100

# Writer with rate limit
API_KEY_3=writer-key-789:writer::500
```

## API Endpoints

### Authentication Endpoints

#### POST /auth/login
Exchange API key for JWT token

Request:
```json
{
  "api_key": "your-api-key"
}
```

Response:
```json
{
  "token": "Bearer eyJ0eXAi...",
  "expires_at": 1234567890,
  "role": "writer"
}
```

#### GET /auth/verify
Verify token validity

Response:
```json
{
  "valid": true,
  "sub": "api_key_admin",
  "role": "admin",
  "exp": 1234567890
}
```

#### POST /auth/api-key
Create new API key (admin only)

Request:
```json
{
  "role": "writer",
  "collections": ["collection1", "collection2"],
  "rate_limit": 1000
}
```

Response:
```json
{
  "api_key": "generated-key-here",
  "role": "writer",
  "collections": ["collection1", "collection2"]
}
```

## Protected Endpoints

All data modification endpoints require authentication:

### ChromaDB API (Write Operations)
- `POST /api/v1/collections` - Create collection (Writer/Admin)
- `DELETE /api/v1/collections/{collection}` - Delete collection (Writer/Admin)
- `POST /api/v1/collections/{collection}/add` - Add documents (Writer/Admin)
- `POST /api/v1/collections/{collection}/vectors` - Update vectors (Writer/Admin)

### Qdrant API (Write Operations)
- `PUT /collections/{collection}` - Create collection (Writer/Admin)
- `PUT /collections/{collection}/points` - Upsert points (Writer/Admin)

### Native VexFS API (Write Operations)
- `POST /vexfs/v1/collections` - Create collection (Writer/Admin)
- `POST /vexfs/v1/collections/{collection}/documents` - Add documents (Writer/Admin)

## Read Operations

Read operations support optional authentication. With authentication, you get:
- Higher rate limits
- Access to restricted collections
- Audit logging of access

Without authentication (anonymous):
- Limited to 10 requests per minute
- Read-only access to public collections
- Basic access only

## Security Best Practices

1. **Always use HTTPS in production** to protect tokens and API keys
2. **Rotate API keys regularly** using the admin API
3. **Use JWT tokens for browser-based applications** instead of embedding API keys
4. **Set appropriate rate limits** to prevent abuse
5. **Disable anonymous access** in production if not needed
6. **Use strong JWT secrets** - at least 32 characters
7. **Implement collection-level permissions** for multi-tenant scenarios

## Example Usage

### Python Client

```python
import requests
import json

class VexFSClient:
    def __init__(self, api_key, base_url="http://localhost:7680"):
        self.base_url = base_url
        self.api_key = api_key
        self.token = None
        self._login()
    
    def _login(self):
        response = requests.post(
            f"{self.base_url}/auth/login",
            json={"api_key": self.api_key}
        )
        response.raise_for_status()
        self.token = response.json()["token"]
    
    def create_collection(self, name, dimension=384):
        response = requests.post(
            f"{self.base_url}/api/v1/collections",
            headers={"Authorization": self.token},
            json={"name": name, "dimension": dimension}
        )
        response.raise_for_status()
        return response.json()
    
    def add_documents(self, collection, documents, embeddings):
        response = requests.post(
            f"{self.base_url}/api/v1/collections/{collection}/add",
            headers={"Authorization": self.token},
            json={
                "documents": documents,
                "embeddings": embeddings
            }
        )
        response.raise_for_status()
        return response.json()

# Usage
client = VexFSClient("your-api-key-here")
client.create_collection("products", dimension=768)
client.add_documents(
    "products",
    ["Product 1", "Product 2"],
    [[0.1, 0.2, ...], [0.3, 0.4, ...]]
)
```

### JavaScript/TypeScript Client

```typescript
class VexFSClient {
  private token: string | null = null;
  
  constructor(
    private apiKey: string,
    private baseUrl: string = "http://localhost:7680"
  ) {}
  
  async login(): Promise<void> {
    const response = await fetch(`${this.baseUrl}/auth/login`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ api_key: this.apiKey })
    });
    
    const data = await response.json();
    this.token = data.token;
  }
  
  async query(collection: string, vector: number[], k: number = 10) {
    if (!this.token) await this.login();
    
    const response = await fetch(
      `${this.baseUrl}/api/v1/collections/${collection}/query`,
      {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'Authorization': this.token!
        },
        body: JSON.stringify({
          query_embeddings: [vector],
          n_results: k
        })
      }
    );
    
    return response.json();
  }
}
```

## Troubleshooting

### Common Issues

1. **401 Unauthorized**: Check that your API key is correct and properly configured
2. **403 Forbidden**: Your role doesn't have permission for this operation
3. **Token expired**: Tokens expire after 24 hours by default, re-login to get a new one
4. **Rate limit exceeded**: You've exceeded your configured rate limit, wait before retrying

### Debug Mode

Enable debug logging to troubleshoot authentication issues:

```bash
VEXFS_LOG_LEVEL=debug ./vexfs_unified_server
```

## Migration from Unauthenticated API

To migrate from an unauthenticated setup:

1. Start with `ALLOW_ANONYMOUS=true` to maintain compatibility
2. Generate API keys for your applications
3. Update applications to use authentication
4. Once all applications are updated, set `ALLOW_ANONYMOUS=false`

## Security Considerations

The current implementation uses a simplified JWT mechanism for demonstration purposes. In production:

1. Use proper JWT libraries with RS256 or ES256 algorithms
2. Implement token refresh mechanisms
3. Add token revocation support
4. Use secure session management
5. Implement audit logging
6. Add IP-based rate limiting
7. Use HTTPS exclusively
8. Implement CORS properly for browser clients

## Future Enhancements

Planned improvements to the authentication system:

- OAuth 2.0 / OpenID Connect support
- Multi-factor authentication (MFA)
- Token refresh endpoints
- Session management
- Audit logging
- IP allowlisting
- Dynamic rate limiting
- API key rotation policies
- Webhook authentication events