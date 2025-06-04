# VexFS v2.0 Quick Start Guide

Get VexFS v2.0 up and running in 5 minutes! This guide covers the fastest path to experiencing the world's first production-ready vector-extended filesystem.

## 🚀 5-Minute Setup

### Option 1: Kernel Module (Production)

```bash
# 1. Clone and build
git clone https://github.com/lspecian/vexfs.git
cd vexfs/kernel/vexfs_v2_build
make

# 2. Load kernel module
sudo insmod vexfs_v2.ko

# 3. Format and mount (using loop device for demo)
sudo dd if=/dev/zero of=/tmp/vexfs_demo.img bs=1M count=100
sudo losetup /dev/loop0 /tmp/vexfs_demo.img
sudo mkfs.vexfs /dev/loop0
sudo mkdir -p /mnt/vexfs
sudo mount -t vexfs_v2 /dev/loop0 /mnt/vexfs

# 4. Test basic operations
echo "Hello VexFS v2.0!" | sudo tee /mnt/vexfs/hello.txt
cat /mnt/vexfs/hello.txt
```

### Option 2: FUSE (Development)

```bash
# 1. Build FUSE implementation
cd rust
cargo build --release --bin vexfs_fuse

# 2. Create mount point and start
mkdir /tmp/vexfs_mount
./target/release/vexfs_fuse /tmp/vexfs_mount

# 3. Test basic operations
echo "Hello VexFS v2.0!" > /tmp/vexfs_mount/hello.txt
cat /tmp/vexfs_mount/hello.txt
```

## 📊 Your First Vector Operations

### Using Python SDK

```python
#!/usr/bin/env python3
import numpy as np
import vexfs

# Connect to VexFS
client = vexfs.Client('/mnt/vexfs')  # or '/tmp/vexfs_mount' for FUSE

# Create a collection
collection = client.create_collection(
    name="demo_collection",
    dimension=384,
    algorithm="hnsw"
)

# Insert vectors with metadata
vectors = np.random.random((1000, 384)).astype(np.float32)
metadata = [{"id": i, "category": f"item_{i%10}"} for i in range(1000)]

collection.insert(vectors=vectors, metadata=metadata)

# Search for similar vectors
query_vector = np.random.random(384).astype(np.float32)
results = collection.search(
    vector=query_vector,
    limit=10,
    filter={"category": "item_5"}
)

print(f"Found {len(results)} similar vectors!")
for result in results:
    print(f"ID: {result.metadata['id']}, Distance: {result.distance:.4f}")
```

### Using TypeScript SDK

```typescript
import { VexFSClient } from '@vexfs/sdk-v2';

async function quickDemo() {
    // Connect to VexFS
    const client = new VexFSClient('/mnt/vexfs');
    
    // Create collection
    const collection = await client.createCollection({
        name: 'demo_collection',
        dimension: 384,
        algorithm: 'hnsw'
    });
    
    // Generate sample data
    const vectors = Array.from({ length: 1000 }, () => 
        Array.from({ length: 384 }, () => Math.random())
    );
    
    const metadata = Array.from({ length: 1000 }, (_, i) => ({
        id: i,
        category: `item_${i % 10}`
    }));
    
    // Insert vectors
    await collection.insert({ vectors, metadata });
    
    // Search
    const queryVector = Array.from({ length: 384 }, () => Math.random());
    const results = await collection.search({
        vector: queryVector,
        limit: 10,
        filter: { category: 'item_5' }
    });
    
    console.log(`Found ${results.length} similar vectors!`);
    results.forEach(result => {
        console.log(`ID: ${result.metadata.id}, Distance: ${result.distance.toFixed(4)}`);
    });
}

quickDemo().catch(console.error);
```

### Using CLI (vexctl)

```bash
# Create a collection
vexctl collection create demo_collection --dimension 384 --algorithm hnsw

# Insert vectors from file
echo '[0.1, 0.2, 0.3, ...]' > vector.json
vexctl vector insert demo_collection --file vector.json --metadata '{"id": 1}'

# Search for similar vectors
vexctl vector search demo_collection --vector '[0.1, 0.2, 0.3, ...]' --limit 10

# List collections
vexctl collection list

# Get collection info
vexctl collection info demo_collection
```

## 🔍 Exploring Vector Search Capabilities

### HNSW (Hierarchical Navigable Small World)

```python
# High-performance approximate search
collection = client.create_collection(
    name="hnsw_demo",
    dimension=768,
    algorithm="hnsw",
    parameters={
        "m": 16,                    # Number of connections
        "ef_construction": 200,     # Construction parameter
        "ef_search": 100           # Search parameter
    }
)

# Insert large dataset
vectors = np.random.random((100000, 768)).astype(np.float32)
collection.insert_batch(vectors, batch_size=10000)

# Fast approximate search
results = collection.search(query_vector, limit=100)
print(f"Search completed in {results.latency_ms}ms")
```

### LSH (Locality Sensitive Hashing)

```python
# Memory-efficient exact search
collection = client.create_collection(
    name="lsh_demo",
    dimension=384,
    algorithm="lsh",
    parameters={
        "num_tables": 10,      # Number of hash tables
        "num_functions": 20    # Hash functions per table
    }
)

# Insert and search
collection.insert(vectors)
results = collection.search(query_vector, exact=True)
```

## 📈 Performance Testing

### Benchmark Your Setup

```bash
# Run built-in benchmarks
cd kernel/vexfs_v2_build

# Test HNSW performance
./test_hnsw_functionality

# Test comprehensive functionality
./standalone_phase3_test

# Expected output:
# ✅ Vector insertion: >100,000 vectors/second
# ✅ Search latency: <1ms for 10-NN
# ✅ Memory efficiency: >90%
# ✅ Concurrent operations: 1000+ ops/sec
```

### Custom Performance Test

```python
import time
import numpy as np
import vexfs

def benchmark_vexfs():
    client = vexfs.Client('/mnt/vexfs')
    collection = client.create_collection("benchmark", dimension=384)
    
    # Insertion benchmark
    vectors = np.random.random((10000, 384)).astype(np.float32)
    start_time = time.time()
    collection.insert_batch(vectors, batch_size=1000)
    insert_time = time.time() - start_time
    
    print(f"Insertion: {len(vectors)/insert_time:.0f} vectors/second")
    
    # Search benchmark
    query_vector = np.random.random(384).astype(np.float32)
    search_times = []
    
    for _ in range(100):
        start_time = time.time()
        results = collection.search(query_vector, limit=10)
        search_times.append((time.time() - start_time) * 1000)
    
    avg_latency = np.mean(search_times)
    print(f"Search latency: {avg_latency:.2f}ms average")

benchmark_vexfs()
```

## 🔧 Configuration Quick Tweaks

### Optimize for Your Use Case

```bash
# High-throughput configuration
export VEXFS_BATCH_SIZE=50000
export VEXFS_WORKER_THREADS=16
export VEXFS_CACHE_SIZE=8GB

# Low-latency configuration
export VEXFS_CACHE_SIZE=16GB
export VEXFS_MAX_OPERATIONS=10000
export VEXFS_PREFETCH_ENABLED=true

# Memory-constrained configuration
export VEXFS_CACHE_SIZE=512MB
export VEXFS_BATCH_SIZE=1000
export VEXFS_COMPRESSION_ENABLED=true
```

### Kernel Module Parameters

```bash
# Load module with custom parameters
sudo insmod vexfs_v2.ko \
    default_dimension=768 \
    cache_size_mb=4096 \
    max_concurrent_ops=2000
```

## 🧪 Testing Your Installation

### Verification Script

```bash
#!/bin/bash
# VexFS v2.0 Quick Verification

echo "🔍 Testing VexFS v2.0 installation..."

# Check kernel module
if lsmod | grep -q vexfs_v2; then
    echo "✅ Kernel module loaded"
else
    echo "❌ Kernel module not loaded"
fi

# Check filesystem registration
if cat /proc/filesystems | grep -q vexfs; then
    echo "✅ Filesystem registered"
else
    echo "❌ Filesystem not registered"
fi

# Check mount
if mount | grep -q vexfs; then
    echo "✅ Filesystem mounted"
    mount | grep vexfs
else
    echo "❌ Filesystem not mounted"
fi

# Test basic file operations
if [ -w /mnt/vexfs ]; then
    echo "test" > /mnt/vexfs/test_file
    if [ -f /mnt/vexfs/test_file ]; then
        echo "✅ Basic file operations working"
        rm /mnt/vexfs/test_file
    else
        echo "❌ File operations failed"
    fi
else
    echo "❌ Mount point not writable"
fi

echo "🎉 VexFS v2.0 verification complete!"
```

## 🚨 Common Quick Fixes

### Module Won't Load

```bash
# Check kernel headers
sudo apt install linux-headers-$(uname -r)

# Rebuild module
cd kernel/vexfs_v2_build
make clean && make

# Check for errors
dmesg | tail -20
```

### Mount Fails

```bash
# Check device permissions
sudo chmod 666 /dev/loop0

# Try different mount options
sudo mount -t vexfs_v2 -o defaults /dev/loop0 /mnt/vexfs
```

### FUSE Issues

```bash
# Unmount and retry
fusermount -u /tmp/vexfs_mount
./target/release/vexfs_fuse /tmp/vexfs_mount

# Check FUSE permissions
sudo usermod -a -G fuse $USER
```

## 🎯 Next Steps

Now that you have VexFS v2.0 running:

1. **[Complete Installation Guide](installation.md)** - Detailed setup options
2. **[Basic Usage Guide](usage.md)** - Learn core concepts
3. **[Vector Search Tutorial](../tutorials/vector-search.md)** - Advanced search techniques
4. **[Performance Tuning](../reference/performance.md)** - Optimize for your workload
5. **[API Reference](../developer-guide/api-reference.md)** - Complete API documentation

### Example Projects

- **[Semantic Search Engine](../tutorials/semantic-search.md)** - Build a semantic search system
- **[RAG Pipeline](../tutorials/rag-pipeline.md)** - Retrieval-Augmented Generation
- **[Image Similarity](../tutorials/image-similarity.md)** - Visual search application
- **[Recommendation System](../tutorials/recommendations.md)** - Vector-based recommendations

**Questions?** Check our [troubleshooting guide](troubleshooting.md) or [community discussions](https://github.com/lspecian/vexfs/discussions).

---

**Welcome to VexFS v2.0!** 🚀 You're now ready to build the next generation of vector-powered applications.