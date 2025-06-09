# VexFS Basic Usage Guide

This guide covers essential VexFS operations and common usage patterns for both FUSE and kernel module implementations.

## Quick Start

### 1. Mount VexFS (FUSE)
```bash
# Create mount point
mkdir /mnt/vexfs

# Mount VexFS
vexfs_fuse /mnt/vexfs

# Verify mount
mount | grep vexfs
ls -la /mnt/vexfs
```

### 2. Basic File Operations
```bash
# Create a file
echo "Hello VexFS" > /mnt/vexfs/hello.txt

# Read the file
cat /mnt/vexfs/hello.txt

# List files
ls -la /mnt/vexfs/

# Copy files
cp /home/user/document.pdf /mnt/vexfs/
```

### 3. Vector Operations
```bash
# Store a vector with metadata
vexctl vector store \
  --id "doc_001" \
  --vector "[0.1, 0.2, 0.3, 0.4]" \
  --metadata '{"title": "Example Document", "category": "text"}'

# Search for similar vectors
vexctl vector search \
  --vector "[0.1, 0.2, 0.3, 0.4]" \
  --k 10

# Retrieve a specific vector
vexctl vector get --id "doc_001"
```

## Core Concepts

### File System Operations
VexFS operates as a standard POSIX filesystem with enhanced vector search capabilities:

```bash
# Standard file operations work normally
touch /mnt/vexfs/example.txt
mkdir /mnt/vexfs/documents
chmod 755 /mnt/vexfs/documents
chown user:group /mnt/vexfs/documents
```

### Vector Storage
Files in VexFS can have associated vector embeddings for semantic search:

```bash
# Store file with automatic vector extraction
echo "This is a sample document" > /mnt/vexfs/sample.txt
vexctl vector extract --file /mnt/vexfs/sample.txt

# Store file with manual vector
vexctl vector associate \
  --file /mnt/vexfs/sample.txt \
  --vector "[0.1, 0.2, 0.3, 0.4]"
```

### Metadata Management
Rich metadata can be associated with files and vectors:

```bash
# Set file metadata
vexctl metadata set \
  --file /mnt/vexfs/sample.txt \
  --metadata '{"author": "John Doe", "created": "2025-01-08"}'

# Query by metadata
vexctl search metadata \
  --filter '{"author": "John Doe"}'
```

## Common Usage Patterns

### 1. Document Storage and Search

#### Store Documents
```bash
# Create document directory
mkdir /mnt/vexfs/documents

# Store documents with automatic vector extraction
for doc in *.pdf; do
  cp "$doc" /mnt/vexfs/documents/
  vexctl vector extract --file "/mnt/vexfs/documents/$doc"
done
```

#### Search Documents
```bash
# Semantic search
vexctl search semantic \
  --query "machine learning algorithms" \
  --k 10

# Hybrid search (semantic + metadata)
vexctl search hybrid \
  --query "machine learning" \
  --filter '{"category": "research"}' \
  --k 5
```

### 2. Image Storage and Search

#### Store Images
```bash
# Create image directory
mkdir /mnt/vexfs/images

# Store images with vector embeddings
for img in *.jpg; do
  cp "$img" /mnt/vexfs/images/
  vexctl vector extract \
    --file "/mnt/vexfs/images/$img" \
    --model "clip-vit-base"
done
```

#### Search Images
```bash
# Search by image similarity
vexctl search image \
  --query-image "/path/to/query.jpg" \
  --k 10

# Search by text description
vexctl search image \
  --query-text "sunset over mountains" \
  --k 5
```

### 3. Code Repository Search

#### Index Code Repository
```bash
# Create code directory
mkdir /mnt/vexfs/code

# Copy and index code files
rsync -av /path/to/repo/ /mnt/vexfs/code/
vexctl index code \
  --directory /mnt/vexfs/code \
  --language-model "codet5"
```

#### Search Code
```bash
# Semantic code search
vexctl search code \
  --query "function to sort array" \
  --language "python" \
  --k 10

# Search by functionality
vexctl search code \
  --query "authentication middleware" \
  --k 5
```

## API Usage

### REST API Examples

#### Store Vector via API
```bash
curl -X POST http://localhost:8080/api/v1/vectors \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -d '{
    "id": "doc_123",
    "vector": [0.1, 0.2, 0.3, 0.4],
    "metadata": {
      "title": "Example Document",
      "category": "text",
      "timestamp": "2025-01-08T12:00:00Z"
    }
  }'
```

#### Search Vectors via API
```bash
curl -X POST http://localhost:8080/api/v1/search \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -d '{
    "vector": [0.1, 0.2, 0.3, 0.4],
    "k": 10,
    "filters": {
      "category": "text"
    }
  }'
```

### WebSocket API Examples

#### Real-time Search Updates
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
    channel: 'search_results',
    query: {
      vector: [0.1, 0.2, 0.3, 0.4],
      k: 10
    }
  }));
};

ws.onmessage = function(event) {
  const data = JSON.parse(event.data);
  console.log('Search result:', data);
};
```

## Configuration

### Basic Configuration
Edit `/etc/vexfs/vexfs.conf`:

```toml
[general]
log_level = "info"
data_dir = "/var/lib/vexfs"

[fuse]
mount_options = "allow_other,default_permissions"
cache_size = "1GB"

[api]
enable_rest = true
rest_port = 8080
enable_websocket = true
websocket_port = 8081

[search]
default_k = 10
max_k = 100
similarity_threshold = 0.7
```

### Performance Configuration
```toml
[performance]
memory_pool_size = "2GB"
enable_simd = true
max_concurrent_operations = 100
io_threads = 8

[cache]
vector_cache_size = "512MB"
metadata_cache_size = "256MB"
query_cache_size = "128MB"
```

## Monitoring and Maintenance

### Check System Status
```bash
# Overall system status
vexctl status

# Performance metrics
vexctl metrics show

# Cache statistics
vexctl cache stats

# Storage usage
vexctl storage usage
```

### Maintenance Operations
```bash
# Optimize indexes
vexctl index optimize

# Compact storage
vexctl storage compact

# Verify data integrity
vexctl fsck --verify

# Backup data
vexctl backup create --output /backup/vexfs_backup.tar.gz
```

## Troubleshooting

### Common Issues

#### Mount Fails
```bash
# Check FUSE availability
ls -l /dev/fuse

# Check permissions
groups $USER

# Check mount point
ls -ld /mnt/vexfs

# Debug mount
vexfs_fuse /mnt/vexfs -d -f
```

#### Slow Performance
```bash
# Check system resources
htop
iostat -x 1

# Analyze performance
cargo run --bin performance_benchmark

# Check configuration
vexctl config validate-performance
```

#### API Connection Issues
```bash
# Check service status
systemctl status vexfs

# Test API connectivity
curl -f http://localhost:8080/api/v1/health

# Check logs
journalctl -u vexfs -f
```

## Best Practices

### File Organization
```bash
# Organize by content type
/mnt/vexfs/
├── documents/          # Text documents
├── images/            # Image files
├── code/              # Source code
├── datasets/          # Data files
└── archives/          # Archived content
```

### Vector Management
```bash
# Use consistent vector dimensions
vexctl config set vector.default_dimension 512

# Batch operations for better performance
vexctl vector store-batch --input vectors.jsonl

# Regular index optimization
vexctl index optimize --schedule weekly
```

### Security
```bash
# Use authentication
export VEXFS_API_KEY="your-api-key"

# Set appropriate permissions
chmod 700 /mnt/vexfs/private/
chmod 755 /mnt/vexfs/public/

# Enable audit logging
vexctl audit enable
```

## Integration Examples

### Python Integration
```python
from vexfs import VexFSClient

# Initialize client
client = VexFSClient(
    base_url='http://localhost:8080',
    api_key='your-api-key'
)

# Store vector
result = client.vectors.create(
    id='doc_123',
    vector=[0.1, 0.2, 0.3, 0.4],
    metadata={'title': 'Example'}
)

# Search vectors
results = client.search.vector(
    vector=[0.1, 0.2, 0.3, 0.4],
    k=10
)
```

### JavaScript Integration
```javascript
import { VexFSClient } from 'vexfs-js';

const client = new VexFSClient({
  baseUrl: 'http://localhost:8080',
  apiKey: 'your-api-key'
});

// Store vector
const result = await client.vectors.create({
  id: 'doc_123',
  vector: [0.1, 0.2, 0.3, 0.4],
  metadata: { title: 'Example' }
});

// Search vectors
const results = await client.search.vector({
  vector: [0.1, 0.2, 0.3, 0.4],
  k: 10
});
```

## Next Steps

1. **[Configuration Guide](configuration.md)** - Detailed configuration options
2. **[API Integration](api-integration.md)** - Complete API documentation
3. **[Performance Tuning](../performance/README.md)** - Optimize for your workload
4. **[Advanced Features](advanced-features.md)** - Explore advanced capabilities

## Getting Help

- **Documentation**: [docs.vexfs.io](https://docs.vexfs.io)
- **Community**: [community.vexfs.io](https://community.vexfs.io)
- **Issues**: [github.com/vexfs/vexfs/issues](https://github.com/vexfs/vexfs/issues)
- **Support**: [support@vexfs.io](mailto:support@vexfs.io)