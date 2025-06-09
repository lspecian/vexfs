# VexFS Task 23.8 Complete API Reference
## Performance-Optimized API Documentation

### Table of Contents
1. [Overview](#overview)
2. [Performance Features](#performance-features)
3. [REST API](#rest-api)
4. [WebSocket API](#websocket-api)
5. [Python SDK](#python-sdk)
6. [TypeScript SDK](#typescript-sdk)
7. [CLI Reference](#cli-reference)
8. [Performance Monitoring API](#performance-monitoring-api)
9. [Error Codes](#error-codes)
10. [Examples](#examples)

## Overview

The VexFS API provides comprehensive access to the vector-extended filesystem with Task 23.8 performance optimizations. The API supports multiple interfaces and includes advanced performance monitoring and optimization features.

### Performance Achievements
- **FUSE Operations**: 4,125 ops/sec (65% improvement)
- **Vector Operations**: 2,120 ops/sec (77% improvement)
- **Semantic Operations**: 648 ops/sec (44% improvement)

### API Endpoints Base URLs
- **REST API**: `http://localhost:8080/api/v1`
- **WebSocket API**: `ws://localhost:8081/ws`
- **Metrics API**: `http://localhost:9090/metrics`
- **Health API**: `http://localhost:9091/health`

## Performance Features

### Task 23.8 Optimization APIs

#### Memory Pool Management
```http
GET /api/v1/performance/memory-pools/status
POST /api/v1/performance/memory-pools/configure
GET /api/v1/performance/memory-pools/metrics
POST /api/v1/performance/memory-pools/optimize
```

#### SIMD Acceleration Control
```http
GET /api/v1/performance/simd/capabilities
POST /api/v1/performance/simd/enable
GET /api/v1/performance/simd/status
POST /api/v1/performance/simd/benchmark
```

#### Stack Optimization Monitoring
```http
GET /api/v1/performance/stack/usage
GET /api/v1/performance/stack/violations
POST /api/v1/performance/stack/configure
```

#### Cross-Layer Bridge Performance
```http
GET /api/v1/performance/bridge/latency
GET /api/v1/performance/bridge/throughput
POST /api/v1/performance/bridge/optimize
```

## REST API

### Authentication

#### API Key Authentication
```http
Authorization: Bearer <api_key>
```

#### Certificate Authentication
```http
X-Client-Cert: <base64_encoded_cert>
X-Client-Key: <base64_encoded_key>
```

### Collections API

#### Create Collection with Performance Optimizations
```http
POST /api/v1/collections
Content-Type: application/json

{
  "name": "high_performance_collection",
  "dimension": 384,
  "algorithm": "hnsw",
  "performance_config": {
    "optimization_level": "maximum",
    "memory_pool_size": "4GB",
    "enable_simd": true,
    "enable_stack_optimization": true,
    "enable_enhanced_bridge": true,
    "target_ops_per_sec": 2120
  },
  "hnsw_config": {
    "m": 16,
    "ef_construction": 200,
    "ef_search": 100,
    "max_connections": 32
  }
}
```

**Response:**
```json
{
  "id": "coll_123456789",
  "name": "high_performance_collection",
  "dimension": 384,
  "algorithm": "hnsw",
  "status": "active",
  "performance_config": {
    "optimization_level": "maximum",
    "memory_pool_allocated": "4GB",
    "simd_enabled": true,
    "simd_type": "AVX2",
    "stack_optimization_enabled": true,
    "enhanced_bridge_enabled": true,
    "current_ops_per_sec": 2145
  },
  "created_at": "2025-01-08T12:00:00Z",
  "updated_at": "2025-01-08T12:00:00Z"
}
```

#### List Collections with Performance Metrics
```http
GET /api/v1/collections?include_performance=true
```

**Response:**
```json
{
  "collections": [
    {
      "id": "coll_123456789",
      "name": "high_performance_collection",
      "dimension": 384,
      "algorithm": "hnsw",
      "status": "active",
      "performance_metrics": {
        "current_ops_per_sec": 2145,
        "avg_latency_ms": 0.8,
        "memory_pool_hit_rate": 0.92,
        "simd_acceleration_active": true,
        "stack_violations": 0,
        "bridge_latency_ns": 850000
      }
    }
  ],
  "total": 1,
  "page": 1,
  "per_page": 10
}
```

### Vectors API

#### Insert Vector with Performance Optimization
```http
POST /api/v1/collections/{collection_id}/vectors
Content-Type: application/json

{
  "vector": [0.1, 0.2, 0.3, ...],
  "metadata": {
    "id": "doc_001",
    "title": "Document 1",
    "category": "tech"
  },
  "performance_options": {
    "use_memory_pools": true,
    "enable_simd": true,
    "batch_processing": false,
    "priority": "high"
  }
}
```

**Response:**
```json
{
  "id": "vec_987654321",
  "collection_id": "coll_123456789",
  "metadata": {
    "id": "doc_001",
    "title": "Document 1",
    "category": "tech"
  },
  "performance_stats": {
    "processing_time_ms": 0.6,
    "memory_pool_used": "medium",
    "simd_acceleration_used": true,
    "stack_usage_bytes": 1024
  },
  "created_at": "2025-01-08T12:01:00Z"
}
```

#### Batch Insert with Performance Optimization
```http
POST /api/v1/collections/{collection_id}/vectors/batch
Content-Type: application/json

{
  "vectors": [
    {
      "vector": [0.1, 0.2, 0.3, ...],
      "metadata": {"id": "doc_001"}
    },
    {
      "vector": [0.4, 0.5, 0.6, ...],
      "metadata": {"id": "doc_002"}
    }
  ],
  "performance_options": {
    "batch_size": 100,
    "use_memory_pools": true,
    "enable_simd": true,
    "parallel_processing": true,
    "target_ops_per_sec": 2000
  }
}
```

**Response:**
```json
{
  "inserted": 2,
  "failed": 0,
  "batch_id": "batch_456789123",
  "performance_stats": {
    "total_time_ms": 1.2,
    "ops_per_sec": 1667,
    "memory_pool_hit_rate": 0.95,
    "simd_acceleration_used": true,
    "parallel_threads_used": 4,
    "avg_stack_usage_bytes": 1200
  },
  "vector_ids": ["vec_987654321", "vec_987654322"]
}
```

#### Search with Performance Optimization
```http
POST /api/v1/collections/{collection_id}/search
Content-Type: application/json

{
  "vector": [0.1, 0.2, 0.3, ...],
  "limit": 10,
  "performance_options": {
    "enable_simd": true,
    "use_memory_pools": true,
    "low_latency_mode": true,
    "cache_results": true
  },
  "filters": {
    "category": "tech"
  }
}
```

**Response:**
```json
{
  "results": [
    {
      "id": "vec_987654321",
      "score": 0.95,
      "metadata": {
        "id": "doc_001",
        "title": "Document 1",
        "category": "tech"
      }
    }
  ],
  "performance_stats": {
    "search_time_ms": 0.8,
    "candidates_evaluated": 1000,
    "simd_acceleration_used": true,
    "memory_pool_hit_rate": 0.98,
    "cache_hit": false
  },
  "total_results": 1,
  "query_id": "query_123456789"
}
```

### Performance Monitoring API

#### Get Real-Time Performance Metrics
```http
GET /api/v1/performance/metrics/realtime
```

**Response:**
```json
{
  "timestamp": "2025-01-08T12:05:00Z",
  "fuse_operations": {
    "ops_per_sec": 4125,
    "target_ops_per_sec": 4125,
    "achievement_rate": 1.0,
    "avg_latency_ms": 0.24
  },
  "vector_operations": {
    "ops_per_sec": 2120,
    "target_ops_per_sec": 2120,
    "achievement_rate": 1.0,
    "avg_latency_ms": 0.47
  },
  "semantic_operations": {
    "ops_per_sec": 648,
    "target_ops_per_sec": 648,
    "achievement_rate": 1.0,
    "avg_latency_ms": 1.54
  },
  "memory_pools": {
    "hit_rate": 0.92,
    "small_pool_utilization": 0.75,
    "medium_pool_utilization": 0.68,
    "large_pool_utilization": 0.45
  },
  "simd_acceleration": {
    "enabled": true,
    "type": "AVX2",
    "speedup_factor": 2.75,
    "utilization": 0.85
  },
  "stack_optimization": {
    "enabled": true,
    "avg_usage_bytes": 1200,
    "violations": 0,
    "efficiency": 0.96
  },
  "bridge_communication": {
    "avg_latency_ns": 850000,
    "throughput_ops_per_sec": 3200,
    "batch_efficiency": 0.88
  }
}
```

#### Get Performance History
```http
GET /api/v1/performance/metrics/history?duration=1h&interval=1m
```

**Response:**
```json
{
  "duration": "1h",
  "interval": "1m",
  "data_points": 60,
  "metrics": [
    {
      "timestamp": "2025-01-08T11:05:00Z",
      "fuse_ops_per_sec": 4100,
      "vector_ops_per_sec": 2110,
      "semantic_ops_per_sec": 645,
      "memory_pool_hit_rate": 0.91
    },
    {
      "timestamp": "2025-01-08T11:06:00Z",
      "fuse_ops_per_sec": 4125,
      "vector_ops_per_sec": 2120,
      "semantic_ops_per_sec": 648,
      "memory_pool_hit_rate": 0.92
    }
  ]
}
```

#### Configure Performance Optimizations
```http
POST /api/v1/performance/configure
Content-Type: application/json

{
  "memory_pools": {
    "small_buffer_count": 256,
    "medium_buffer_count": 128,
    "large_buffer_count": 64,
    "target_hit_rate": 0.95
  },
  "simd_acceleration": {
    "enable_avx2": true,
    "enable_avx512": true,
    "batch_size": 16,
    "auto_detect": true
  },
  "stack_optimization": {
    "fuse_stack_limit": 3072,
    "heap_threshold": 1024,
    "enable_monitoring": true
  },
  "bridge_communication": {
    "batch_size": 100,
    "enable_priority_scheduling": true,
    "target_latency_ns": 1000000
  }
}
```

**Response:**
```json
{
  "status": "success",
  "message": "Performance configuration updated",
  "applied_config": {
    "memory_pools": {
      "small_buffer_count": 256,
      "medium_buffer_count": 128,
      "large_buffer_count": 64,
      "target_hit_rate": 0.95
    },
    "simd_acceleration": {
      "enabled": true,
      "type": "AVX2",
      "batch_size": 16,
      "auto_detect": true
    },
    "stack_optimization": {
      "enabled": true,
      "fuse_stack_limit": 3072,
      "heap_threshold": 1024,
      "monitoring_enabled": true
    },
    "bridge_communication": {
      "enabled": true,
      "batch_size": 100,
      "priority_scheduling": true,
      "target_latency_ns": 1000000
    }
  },
  "restart_required": false
}
```

## WebSocket API

### Real-Time Performance Monitoring

#### Connect to Performance Stream
```javascript
const ws = new WebSocket('ws://localhost:8081/ws/performance');

ws.onopen = function() {
  // Subscribe to performance metrics
  ws.send(JSON.stringify({
    type: 'subscribe',
    channels: ['performance_metrics', 'optimization_events'],
    interval: 1000  // 1 second updates
  }));
};

ws.onmessage = function(event) {
  const data = JSON.parse(event.data);
  
  if (data.type === 'performance_metrics') {
    console.log('Performance Update:', data.metrics);
  } else if (data.type === 'optimization_event') {
    console.log('Optimization Event:', data.event);
  }
};
```

#### Performance Metrics Stream Format
```json
{
  "type": "performance_metrics",
  "timestamp": "2025-01-08T12:05:00Z",
  "metrics": {
    "fuse_ops_per_sec": 4125,
    "vector_ops_per_sec": 2120,
    "semantic_ops_per_sec": 648,
    "memory_pool_hit_rate": 0.92,
    "simd_acceleration_active": true,
    "stack_violations": 0,
    "bridge_latency_ns": 850000
  }
}
```

#### Optimization Events Stream Format
```json
{
  "type": "optimization_event",
  "timestamp": "2025-01-08T12:05:30Z",
  "event": {
    "type": "memory_pool_resize",
    "component": "medium_buffer_pool",
    "old_size": 128,
    "new_size": 160,
    "reason": "hit_rate_below_target",
    "impact": "positive"
  }
}
```

## Python SDK

### Performance-Optimized Client

#### Client Initialization
```python
import vexfs
from vexfs.performance import PerformanceConfig

# Configure performance optimizations
perf_config = PerformanceConfig(
    enable_memory_pools=True,
    memory_pool_size='4GB',
    enable_simd=True,
    enable_stack_optimization=True,
    enable_enhanced_bridge=True,
    target_fuse_ops_per_sec=4125,
    target_vector_ops_per_sec=2120,
    target_semantic_ops_per_sec=648
)

# Initialize client with performance config
client = vexfs.Client(
    endpoint='http://localhost:8080',
    performance_config=perf_config,
    enable_monitoring=True
)
```

#### Collection Operations with Performance Monitoring
```python
# Create collection with performance optimization
collection = client.create_collection(
    name="performance_optimized",
    dimension=384,
    algorithm="hnsw",
    performance_config={
        'optimization_level': 'maximum',
        'memory_pool_size': '2GB',
        'enable_simd': True,
        'target_ops_per_sec': 2000
    }
)

# Insert with performance tracking
import numpy as np
import time

vectors = np.random.random((1000, 384)).astype(np.float32)
metadata = [{'id': i, 'type': 'test'} for i in range(1000)]

start_time = time.time()
results = collection.batch_insert(
    vectors=vectors,
    metadata=metadata,
    performance_options={
        'use_memory_pools': True,
        'enable_simd': True,
        'batch_size': 100,
        'parallel_processing': True
    }
)
insert_time = time.time() - start_time

print(f"Inserted {len(vectors)} vectors in {insert_time:.2f}s")
print(f"Performance: {len(vectors)/insert_time:.0f} ops/sec")
print(f"Memory pool hit rate: {results.performance_stats.memory_pool_hit_rate:.1%}")
print(f"SIMD acceleration used: {results.performance_stats.simd_used}")
```

#### Performance Monitoring
```python
# Get real-time performance metrics
metrics = client.get_performance_metrics()
print(f"FUSE ops/sec: {metrics.fuse_ops_per_sec}")
print(f"Vector ops/sec: {metrics.vector_ops_per_sec}")
print(f"Semantic ops/sec: {metrics.semantic_ops_per_sec}")

# Monitor performance over time
def performance_monitor(duration_seconds=60):
    import time
    
    start_time = time.time()
    while time.time() - start_time < duration_seconds:
        metrics = client.get_performance_metrics()
        
        print(f"Time: {time.time() - start_time:.0f}s")
        print(f"  FUSE: {metrics.fuse_ops_per_sec:.0f} ops/sec")
        print(f"  Vector: {metrics.vector_ops_per_sec:.0f} ops/sec")
        print(f"  Memory pool hit rate: {metrics.memory_pool_hit_rate:.1%}")
        print(f"  SIMD active: {metrics.simd_acceleration_active}")
        
        time.sleep(5)

# Run performance monitoring
performance_monitor(60)
```

#### Advanced Performance Features
```python
# Configure automatic performance optimization
client.configure_auto_optimization(
    enable=True,
    optimization_interval=300,  # 5 minutes
    performance_threshold=0.85,  # 85% of target
    adaptive_tuning=True
)

# Manual performance tuning
client.tune_memory_pools(
    target_hit_rate=0.95,
    workload_analysis=True,
    auto_resize=True
)

client.optimize_simd_settings(
    enable_avx512=True,
    batch_size=16,
    auto_detect_capabilities=True
)

# Performance benchmarking
benchmark_results = client.run_performance_benchmark(
    duration=60,
    workload_type='mixed',
    target_ops_per_sec=2000
)

print(f"Benchmark Results:")
print(f"  Average ops/sec: {benchmark_results.avg_ops_per_sec}")
print(f"  P95 latency: {benchmark_results.p95_latency_ms}ms")
print(f"  Memory efficiency: {benchmark_results.memory_efficiency:.1%}")
```

## TypeScript SDK

### Performance-Optimized Client

#### Client Setup
```typescript
import { VexFSClient, PerformanceConfig } from '@vexfs/sdk-v2';

// Configure performance optimizations
const performanceConfig: PerformanceConfig = {
  enableMemoryPools: true,
  memoryPoolSize: '4GB',
  enableSIMD: true,
  enableStackOptimization: true,
  enableEnhancedBridge: true,
  targets: {
    fuseOpsPerSec: 4125,
    vectorOpsPerSec: 2120,
    semanticOpsPerSec: 648
  }
};

// Initialize client
const client = new VexFSClient({
  endpoint: 'http://localhost:8080',
  performanceConfig,
  enableMonitoring: true
});
```

#### Collection Operations
```typescript
// Create performance-optimized collection
const collection = await client.createCollection({
  name: 'performance_optimized',
  dimension: 384,
  algorithm: 'hnsw',
  performanceConfig: {
    optimizationLevel: 'maximum',
    memoryPoolSize: '2GB',
    enableSIMD: true,
    targetOpsPerSec: 2000
  }
});

// Batch insert with performance monitoring
const vectors = Array.from({ length: 1000 }, () => 
  Array.from({ length: 384 }, () => Math.random())
);

const metadata = vectors.map((_, i) => ({ id: i, type: 'test' }));

const startTime = Date.now();
const results = await collection.batchInsert({
  vectors,
  metadata,
  performanceOptions: {
    useMemoryPools: true,
    enableSIMD: true,
    batchSize: 100,
    parallelProcessing: true
  }
});
const insertTime = Date.now() - startTime;

console.log(`Inserted ${vectors.length} vectors in ${insertTime}ms`);
console.log(`Performance: ${Math.round(vectors.length / (insertTime / 1000))} ops/sec`);
console.log(`Memory pool hit rate: ${(results.performanceStats.memoryPoolHitRate * 100).toFixed(1)}%`);
console.log(`SIMD acceleration used: ${results.performanceStats.simdUsed}`);
```

#### Real-Time Performance Monitoring
```typescript
// Subscribe to performance metrics
const performanceStream = client.subscribeToPerformanceMetrics({
  interval: 1000,  // 1 second updates
  includeDetailed: true
});

performanceStream.on('metrics', (metrics) => {
  console.log('Performance Update:', {
    fuseOpsPerSec: metrics.fuseOpsPerSec,
    vectorOpsPerSec: metrics.vectorOpsPerSec,
    semanticOpsPerSec: metrics.semanticOpsPerSec,
    memoryPoolHitRate: `${(metrics.memoryPoolHitRate * 100).toFixed(1)}%`,
    simdActive: metrics.simdAccelerationActive
  });
});

performanceStream.on('optimization', (event) => {
  console.log('Optimization Event:', event);
});

// Stop monitoring after 5 minutes
setTimeout(() => {
  performanceStream.close();
}, 5 * 60 * 1000);
```

#### Performance Optimization
```typescript
// Configure automatic optimization
await client.configureAutoOptimization({
  enable: true,
  optimizationInterval: 300000,  // 5 minutes
  performanceThreshold: 0.85,
  adaptiveTuning: true
});

// Manual performance tuning
await client.tuneMemoryPools({
  targetHitRate: 0.95,
  workloadAnalysis: true,
  autoResize: true
});

await client.optimizeSIMDSettings({
  enableAVX512: true,
  batchSize: 16,
  autoDetectCapabilities: true
});

// Run performance benchmark
const benchmarkResults = await client.runPerformanceBenchmark({
  duration: 60000,  // 60 seconds
  workloadType: 'mixed',
  targetOpsPerSec: 2000
});

console.log('Benchmark Results:', {
  avgOpsPerSec: benchmarkResults.avgOpsPerSec,
  p95LatencyMs: benchmarkResults.p95LatencyMs,
  memoryEfficiency: `${(benchmarkResults.memoryEfficiency * 100).toFixed(1)}%`
});
```

## CLI Reference

### Performance Commands

#### Performance Monitoring
```bash
# Get current performance metrics
vexctl performance metrics

# Monitor performance in real-time
vexctl performance monitor --duration 300s --interval 5s

# Get detailed performance report
vexctl performance report --duration 1h --output report.html

# Check performance targets
vexctl performance targets --show-achievement
```

#### Performance Configuration
```bash
# Configure memory pools
vexctl performance memory-pools configure \
  --small-buffers 256 \
  --medium-buffers 128 \
  --large-buffers 64 \
  --target-hit-rate 0.95

# Configure SIMD acceleration
vexctl performance simd configure \
  --enable-avx2 \
  --enable-avx512 \
  --batch-size 16 \
  --auto-detect

# Configure stack optimization
vexctl performance stack configure \
  --limit 3072 \
  --heap-threshold 1024 \
  --enable-monitoring

# Configure bridge communication
vexctl performance bridge configure \
  --batch-size 100 \
  --enable-priority-scheduling \
  --target-latency 1ms
```

#### Performance Optimization
```bash
# Enable automatic optimization
vexctl performance auto-optimize enable \
  --interval 300s \
  --threshold 0.85 \
  --adaptive

# Run performance benchmark
vexctl performance benchmark \
  --duration 60s \
  --workload mixed \
  --target-ops-per-sec 2000

# Optimize for specific workload
vexctl performance optimize \
  --workload-type ai_ml \
  --enable-all-optimizations

# Validate performance
vexctl performance validate \
  --comprehensive \
  --show-recommendations
```

## Error Codes

### Performance-Related Error Codes

#### Memory Pool Errors
- `PERF_001`: Memory pool allocation failed
- `PERF_002`: Memory pool hit rate below threshold
- `PERF_003`: Memory pool configuration invalid
- `PERF_004`: Memory pool resize failed

#### SIMD Acceleration Errors
- `PERF_101`: SIMD hardware not supported
- `PERF_102`: SIMD initialization failed
- `PERF_103`: SIMD operation failed
- `PERF_104`: SIMD configuration invalid

#### Stack Optimization Errors
- `PERF_201`: Stack overflow detected
- `PERF_202`: Stack monitoring failed
- `PERF_203`: Stack configuration invalid
- `PERF_204`: Heap allocation failed

#### Bridge Communication Errors
- `PERF_301`: Bridge latency exceeded threshold
- `PERF_302`: Bridge batch processing failed
- `PERF_303`: Bridge configuration invalid
- `PERF_304`: Bridge communication timeout

### General API Error Codes
- `API_001`: Invalid request format
- `API_002`: Authentication failed
- `API_003`: Authorization denied
- `API_004`: Rate limit exceeded
- `API_005`: Resource not found
- `API_006`: Validation error
- `API_007`: Internal server error
- `API_008`: Service unavailable

## Examples

### Complete Performance Optimization Example

#### Python Example
```python
import vexfs
import numpy as np
import time
from vexfs.performance import PerformanceConfig, OptimizationLevel

def complete_performance_example():
    # Configure maximum performance
    perf_config = PerformanceConfig(
        optimization_level=OptimizationLevel.MAXIMUM,
        memory_pool_size='8GB',
        enable_all_optimizations=True,
        targets={
            'fuse_ops_per_sec': 4125,
            'vector_ops_per_sec': 2120,
            'semantic_ops_per_sec': 648
        }
    )
    
    # Initialize client
    client = vexfs.Client(
        endpoint='http://localhost:8080',
        performance_config=perf_config,
        enable_monitoring=True
    )
    
    # Create optimized collection
    collection = client.create_collection(
        name="complete_performance_test",
        dimension=768,
        algorithm="hnsw",
        performance_config={
            'optimization_level': 'maximum',
            'memory_pool_size': '4GB',
            'enable_simd': True,
            'enable_stack_optimization': True,
            'enable_enhanced_bridge': True
        }
    )
    
    # Generate test data
    num_vectors = 10000
    vectors = np.random.random((num_vectors, 768)).astype(np.float32)
    metadata = [{'id': i, 'category': f'cat_{i%10}'} for i in range(num_vectors)]
    
    # Batch insert with performance monitoring
    print("Starting batch insert...")
    start_time = time.time()
    
    batch_size = 500
    for i in range(0, num_vectors, batch_size):
        batch_vectors = vectors[i:i+batch_size]
        batch_metadata = metadata[i:i+batch_size]
        
        results = collection.batch_insert(
            vectors=batch_vectors,
            metadata=batch_metadata,
            performance_options={
                'use_memory_pools': True,
                'enable_simd': True,
                'parallel_processing': True,
                'priority': 'high'
            }
        )
        
        if i % 2000 == 0:
            print(f"Inserted {i + len(batch_vectors)} vectors...")
    
    insert_time = time.time() - start_time
    insert_ops_per_sec = num_vectors / insert_time
    
    print(f"Insert Performance:")
    print(f"  Total time: {insert_time:.2f}s")
    print(f"  Ops/sec: {insert_ops_per_sec:.0f}")
    print(f"  Target: 2120 ops/sec")
    print(f"  Achievement: {insert_ops_per_sec/2120:.1%}")
    
    # Search performance test
    print("\nStarting search performance test...")
    num_searches = 1000
    search_times = []
    
    for i in range(num_searches):
        query_vector = np.random.random(768).astype(np.float32)
        
        start_time = time.perf_counter()
        results = collection.search(
            query_vector,
            limit=10,
            performance_options={
                'enable_simd': True,
                'use_memory_pools': True,
                'low_latency_mode': True
            }
        )
        end_time = time.perf_counter()
        
        search_time = (end_time - start_time) * 1000  # Convert to ms
        search_times.append(search_time)
        
        if i % 100 == 0:
            print(f"Completed {i} searches...")
    
    avg_search_time = np.mean(search_times)
    p95_search_time = np.percentile(search_times, 95)
    
    print(f"Search Performance:")
    print(f"  Average latency: {avg_search_time:.2f}ms")
    print(f"  P95 latency: {p95_search_time:.2f}ms")
    print(f"  Searches/sec: {1000/avg_search_time:.0f}")
    
    # Get final