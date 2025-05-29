# VexFS SDK Examples

This directory contains comprehensive examples demonstrating how to use VexFS SDKs across different programming languages. These examples showcase real-world usage patterns, best practices, and integration scenarios for the VexFS vector-extended filesystem.

## 📁 **Directory Structure**

```
examples/
├── python/                 # Python SDK examples
│   ├── basic_usage.py      # Basic operations demo
│   ├── advanced_search.py  # Advanced search patterns
│   ├── batch_operations.py # Bulk data processing
│   └── ml_integration.py   # ML pipeline integration
├── typescript/             # TypeScript SDK examples
│   ├── basic_usage.ts      # Basic operations demo
│   ├── express_api.ts      # Express.js integration
│   ├── fastify_api.ts      # Fastify integration
│   └── real_time_search.ts # Real-time search implementation
├── benchmarks/             # Performance benchmarks
│   ├── vector_benchmark.rs
│   ├── vector_cache_benchmark.rs
│   └── cow_snapshot_benchmark.rs
└── README.md              # This file
```

## 🚀 **Getting Started**

### Prerequisites

Before running any examples, ensure you have:

1. **VexFS Core** installed and running
2. **Language-specific requirements** (see individual SDK documentation)
3. **Development environment** properly configured

### Quick Setup

```bash
# Clone the repository
git clone https://github.com/lspecian/vexfs.git
cd vexfs

# Build VexFS core
cargo build --release

# Navigate to examples
cd examples
```

## 🐍 **Python Examples**

### Prerequisites

- Python 3.8+
- VexFS Python SDK installed
- Optional: ML libraries for advanced examples

### Installation

```bash
# Install VexFS Python SDK
pip install vexfs

# Install optional dependencies for ML examples
pip install numpy pandas sentence-transformers scikit-learn
```

### Available Examples

#### 1. Basic Usage (`python/basic_usage.py`)

Demonstrates fundamental VexFS operations:
- Adding documents with metadata
- Performing vector queries
- Deleting documents
- Basic error handling

```bash
cd examples/python
python basic_usage.py
```

**Expected Output:**
```
Added document with ID: doc_12345
Query results: ['doc_12345', 'doc_67890']
Document deleted
```

#### 2. Advanced Search (`python/advanced_search.py`)

Shows sophisticated search patterns:
- Semantic search with embeddings
- Metadata filtering
- Result ranking and scoring
- Multi-metric similarity search

```bash
python advanced_search.py
```

#### 3. Batch Operations (`python/batch_operations.py`)

Demonstrates high-performance bulk operations:
- Batch document insertion
- Parallel processing
- Performance monitoring
- Memory-efficient processing

```bash
python batch_operations.py
```

#### 4. ML Integration (`python/ml_integration.py`)

Integration with machine learning workflows:
- Sentence transformer embeddings
- RAG (Retrieval-Augmented Generation) patterns
- Vector database integration
- Real-time inference pipelines

```bash
python ml_integration.py
```

### Running Python Examples

```bash
# Run all Python examples
cd examples/python
for example in *.py; do
    echo "Running $example..."
    python "$example"
    echo "---"
done
```

## 🔷 **TypeScript Examples**

### Prerequisites

- Node.js 16+
- TypeScript 5.0+
- VexFS TypeScript SDK installed

### Installation

```bash
# Install VexFS TypeScript SDK
npm install vexfs-sdk

# Install development dependencies
npm install -D typescript @types/node ts-node

# For web framework examples
npm install express fastify
npm install -D @types/express
```

### Available Examples

#### 1. Basic Usage (`typescript/basic_usage.ts`)

Fundamental TypeScript operations:
- Client initialization and configuration
- Async/await patterns
- Type-safe operations
- Error handling with try/catch

```bash
cd examples/typescript
npx ts-node basic_usage.ts
```

**Expected Output:**
```
Document added: doc_12345
Found 2 similar documents
Document deleted successfully
```

#### 2. Express API (`typescript/express_api.ts`)

REST API implementation with Express.js:
- RESTful endpoints for VexFS operations
- Request validation and error handling
- JSON API responses
- Middleware integration

```bash
npx ts-node express_api.ts
```

Test the API:
```bash
# Add document
curl -X POST http://localhost:3000/documents \
  -H "Content-Type: application/json" \
  -d '{"text": "Sample document", "metadata": {"type": "test"}}'

# Search
curl -X POST http://localhost:3000/search \
  -H "Content-Type: application/json" \
  -d '{"vector": [0.1, 0.2, 0.3], "topK": 5}'
```

#### 3. Fastify API (`typescript/fastify_api.ts`)

High-performance API with Fastify:
- Schema validation
- Type-safe request/response handling
- Performance optimizations
- Structured logging

```bash
npx ts-node fastify_api.ts
```

#### 4. Real-time Search (`typescript/real_time_search.ts`)

Real-time search implementation:
- Event-driven architecture
- Caching strategies
- WebSocket integration
- Performance monitoring

```bash
npx ts-node real_time_search.ts
```

### Running TypeScript Examples

```bash
# Compile and run all TypeScript examples
cd examples/typescript
for example in *.ts; do
    echo "Running $example..."
    npx ts-node "$example"
    echo "---"
done
```

## ⚡ **Performance Benchmarks**

### Rust Benchmarks

The `benchmarks/` directory contains Rust-based performance tests:

#### Vector Operations Benchmark
```bash
cd examples
cargo run --bin vector_benchmark --release
```

**Expected Metrics:**
- Vector insertion: ~263,852 vectors/second
- Search latency: 21.98-52.34µs
- Memory efficiency: 94.2%

#### Vector Cache Benchmark
```bash
cargo run --bin vector_cache_benchmark --release
```

**Expected Metrics:**
- Cache hit latency: ~2.18µs
- Cache miss latency: ~156.78µs
- Memory utilization: 100%

#### CoW Snapshot Benchmark
```bash
cargo run --bin cow_snapshot_benchmark --release
```

**Expected Metrics:**
- CoW reference creation: ~8.92µs
- Space efficiency: 89.94%
- Snapshot operations: ~12.35µs

### Running All Benchmarks

```bash
cd examples
./run_all_benchmarks.sh
```

## 🔄 **Cross-Language Examples**

### Data Interchange Patterns

Examples demonstrating data exchange between different SDK implementations:

#### Python to TypeScript Pipeline

1. **Data Ingestion (Python)**:
```python
# ingest_data.py
import vexfs
import json

# Process large dataset with Python's ML libraries
documents = process_ml_dataset()
doc_ids = []

for doc in documents:
    doc_id = vexfs.add(doc['text'], doc['metadata'])
    doc_ids.append(doc_id)

# Save document IDs for TypeScript processing
with open('doc_ids.json', 'w') as f:
    json.dump(doc_ids, f)
```

2. **API Service (TypeScript)**:
```typescript
// api_service.ts
import VexFSClient from 'vexfs-sdk';
import fs from 'fs';

const client = new VexFSClient();
const docIds = JSON.parse(fs.readFileSync('doc_ids.json', 'utf8'));

// Serve search API using pre-indexed documents
app.post('/search', async (req, res) => {
  const results = await client.query(req.body.vector, 10);
  res.json(results);
});
```

#### Shared Configuration

Both SDKs can share configuration and data:

```json
// vexfs_config.json
{
  "baseUrl": "http://localhost:8080",
  "timeout": 30000,
  "batchSize": 100,
  "vectorDimension": 384
}
```

## 🛠️ **Development Workflow**

### Setting Up Development Environment

1. **Install Dependencies**:
```bash
# Python environment
python -m venv venv
source venv/bin/activate  # On Windows: venv\Scripts\activate
pip install -r requirements.txt

# Node.js environment
npm install
```

2. **Configure VexFS**:
```bash
# Start VexFS server (if not running)
cargo run --bin vexfs_server

# Verify connection
curl http://localhost:8080/health
```

3. **Run Examples**:
```bash
# Python examples
cd examples/python
python basic_usage.py

# TypeScript examples
cd examples/typescript
npx ts-node basic_usage.ts
```

### Testing Examples

```bash
# Test Python examples
cd examples/python
python -m pytest test_examples.py

# Test TypeScript examples
cd examples/typescript
npm test
```

### Creating New Examples

1. **Follow naming convention**: `{language}/{purpose}.{ext}`
2. **Include comprehensive comments**
3. **Add error handling**
4. **Provide expected output**
5. **Update this README**

## 📊 **Performance Expectations**

Based on VexFS v1.0.0 production testing:

| Operation | Expected Performance | Notes |
|-----------|---------------------|-------|
| **Document Addition** | 263,852/second | Bulk operations |
| **Vector Search** | 21.98-52.34µs | Depends on metric |
| **Cache Hit** | 2.18µs | Optimal case |
| **Cache Miss** | 156.78µs | With disk I/O |
| **Memory Efficiency** | 94.2% | Optimal utilization |

### Optimization Tips

1. **Use batch operations** for multiple documents
2. **Implement caching** for frequently accessed data
3. **Monitor memory usage** with large datasets
4. **Use appropriate vector dimensions** (384 recommended)
5. **Configure timeouts** based on network conditions

## 🐛 **Troubleshooting**

### Common Issues

**VexFS server not running**:
```bash
# Check if server is running
curl http://localhost:8080/health

# Start server if needed
cargo run --bin vexfs_server
```

**Python import errors**:
```bash
# Reinstall VexFS Python SDK
pip uninstall vexfs
pip install vexfs
```

**TypeScript compilation errors**:
```bash
# Update TypeScript
npm install -g typescript@latest

# Check tsconfig.json
npx tsc --showConfig
```

**Vector dimension mismatch**:
- Ensure all vectors have the same dimension
- Use consistent embedding models
- Validate vector data before operations

### Getting Help

1. **Check SDK documentation**:
   - [Python SDK README](../bindings/python/README.md)
   - [TypeScript SDK README](../bindings/typescript/README.md)

2. **Review VexFS documentation**:
   - [Main README](../README.md)
   - [Architecture docs](../docs/architecture/)

3. **Report issues**:
   - [GitHub Issues](https://github.com/lspecian/vexfs/issues)

## 📄 **License**

All examples are licensed under the Apache License 2.0 - see the [LICENSE](../LICENSE) file for details.

## 🤝 **Contributing**

We welcome contributions to the examples collection:

1. **Fork the repository**
2. **Create example following our patterns**
3. **Add comprehensive documentation**
4. **Test thoroughly**
5. **Submit pull request**

### Example Contribution Guidelines

- **Clear purpose**: Each example should demonstrate specific functionality
- **Complete code**: Include all necessary imports and setup
- **Error handling**: Show proper error handling patterns
- **Documentation**: Explain what the example does and how to run it
- **Performance**: Include performance expectations where relevant

---

**VexFS Examples** - Learn by doing with comprehensive, real-world examples 🚀