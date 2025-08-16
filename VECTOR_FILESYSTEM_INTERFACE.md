# VexFS Vector Filesystem Interface Design

## Overview
This document describes how vector operations are exposed through the filesystem interface in VexFS.

## File Types and Conventions

### 1. Vector Files (.vec)
Store vectors as comma-separated float values in `.vec` files.

**Format:**
```
0.1,0.2,0.3,0.4,0.5
```

**Operations:**
- **Write**: `echo "0.1,0.2,0.3" > /mnt/vexfs/my_vector.vec`
- **Read**: `cat /mnt/vexfs/my_vector.vec`
- **Update**: Overwrite the file with new values

### 2. Search Query Files (.search)
Perform vector similarity search by writing to `.search` files.

**Format:**
```
vector: 0.1,0.2,0.3
k: 5
threshold: 0.8
```

**Usage:**
```bash
# Create search query
cat > /mnt/vexfs/query.search << EOF
vector: 0.1,0.2,0.3
k: 10
EOF

# Read results
cat /mnt/vexfs/query.search.results
```

### 3. Collection Directories
Organize vectors into collections using directories.

```
/mnt/vexfs/
├── documents/          # Collection of document embeddings
│   ├── doc1.vec
│   ├── doc2.vec
│   └── metadata.json
├── images/            # Collection of image embeddings
│   ├── img1.vec
│   └── img2.vec
└── _index/            # Special directory for index control
    ├── status         # Index status
    └── rebuild        # Trigger index rebuild
```

## Extended Attributes (xattr)

Use filesystem extended attributes to store vector metadata:

```bash
# Set metadata
setfattr -n user.dimension -v "384" /mnt/vexfs/doc.vec
setfattr -n user.model -v "bert-base" /mnt/vexfs/doc.vec
setfattr -n user.timestamp -v "2025-08-16" /mnt/vexfs/doc.vec

# Get metadata
getfattr -n user.dimension /mnt/vexfs/doc.vec
getfattr -d /mnt/vexfs/doc.vec  # Get all attributes
```

## Special Control Files

### /_vexfs/control
System control and configuration file.

**Read**: Get current configuration
```bash
cat /mnt/vexfs/_vexfs/control
```

**Write**: Update configuration
```bash
echo "index.auto_rebuild=true" >> /mnt/vexfs/_vexfs/control
echo "search.algorithm=hnsw" >> /mnt/vexfs/_vexfs/control
```

### /_vexfs/stats
Real-time statistics about the vector storage.

```bash
cat /mnt/vexfs/_vexfs/stats
# Output:
# vectors_stored: 10000
# index_size: 45MB
# dimensions: 384
# search_operations: 1523
# avg_search_time_ms: 2.3
```

### /_vexfs/search
Global search interface.

```bash
# Perform search
echo "0.1,0.2,0.3" > /mnt/vexfs/_vexfs/search

# Read results (returns top-k similar vectors)
cat /mnt/vexfs/_vexfs/search
# Output:
# 1. /documents/doc5.vec (distance: 0.02)
# 2. /documents/doc2.vec (distance: 0.05)
# 3. /images/img1.vec (distance: 0.08)
```

## Operations Examples

### 1. Store Document Embedding
```bash
# Generate embedding (example)
python -c "import numpy as np; print(','.join(map(str, np.random.rand(384))))" > doc.vec

# Copy to VexFS
cp doc.vec /mnt/vexfs/documents/

# Add metadata
setfattr -n user.source -v "research_paper.pdf" /mnt/vexfs/documents/doc.vec
```

### 2. Similarity Search
```bash
# Method 1: Using search file
echo "vector: 0.1,0.2,0.3,0.4" > /mnt/vexfs/query.search
cat /mnt/vexfs/query.search.results

# Method 2: Using control interface
echo "0.1,0.2,0.3,0.4" > /mnt/vexfs/_vexfs/search
cat /mnt/vexfs/_vexfs/search
```

### 3. Batch Operations
```bash
# Store multiple vectors
for i in {1..100}; do
    python -c "import numpy as np; print(','.join(map(str, np.random.rand(128))))" > /mnt/vexfs/batch/vec_$i.vec
done

# Trigger index rebuild
echo "rebuild" > /mnt/vexfs/_vexfs/index/rebuild

# Check status
cat /mnt/vexfs/_vexfs/index/status
```

### 4. Collection Management
```bash
# Create collection
mkdir /mnt/vexfs/my_collection

# Add vectors to collection
cp *.vec /mnt/vexfs/my_collection/

# Set collection metadata
echo '{"name": "My Collection", "model": "sentence-transformers"}' > /mnt/vexfs/my_collection/metadata.json

# Search within collection
echo "collection: my_collection" > /mnt/vexfs/_vexfs/search.config
echo "0.1,0.2,0.3" > /mnt/vexfs/_vexfs/search
```

## Implementation Status

### ✅ Currently Implemented
- Basic .vec file detection and parsing
- Vector storage in backend (HNSW graph)
- Internal vector operations

### 🚧 To Be Implemented
- [ ] Search file interface (.search files)
- [ ] Extended attributes support
- [ ] Special control files (_vexfs/ directory)
- [ ] Search results formatting
- [ ] Collection-based operations
- [ ] Batch processing optimization
- [ ] Index management interface

## Performance Considerations

1. **Lazy Indexing**: Vectors are added to HNSW index on first search or after batch threshold
2. **Memory Mapping**: Large vector files are memory-mapped for efficiency
3. **Caching**: Recent search results are cached for repeated queries
4. **Async Operations**: Index rebuilds happen asynchronously

## Security Notes

- Vector data is stored unencrypted
- No access control on vector operations (inherits filesystem permissions)
- Search queries are logged for debugging (disable in production)

---

*This interface design provides intuitive filesystem-based access to VexFS's vector capabilities while maintaining compatibility with standard Unix tools.*