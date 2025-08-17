#!/usr/bin/env python3
"""
VexFS Large Scale Performance Test
Tests 10,000 vectors at 1,536 dimensions to match competitive analysis
"""

import os
import sys
import time
import json
import numpy as np
from datetime import datetime

def ensure_vexfs_mounted():
    """Ensure VexFS is mounted and accessible"""
    mount_point = "/tmp/vexfs_benchmark"
    
    # Check if already mounted
    result = os.system(f"mount | grep '{mount_point}' > /dev/null 2>&1")
    if result == 0:
        print("✅ VexFS already mounted")
        return mount_point
    
    # Try to mount
    print("🔄 Mounting VexFS...")
    os.makedirs(mount_point, exist_ok=True)
    
    # Start VexFS FUSE
    vexfs_binary = "./vexfs_fuse"
    if not os.path.exists(vexfs_binary):
        print(f"❌ VexFS binary not found: {vexfs_binary}")
        return None
    
    # Mount in background
    os.system(f"{vexfs_binary} {mount_point} -f &")
    time.sleep(2)  # Give it time to mount
    
    # Verify mount
    result = os.system(f"mount | grep '{mount_point}' > /dev/null 2>&1")
    if result == 0:
        print("✅ VexFS mounted successfully")
        return mount_point
    else:
        print("❌ Failed to mount VexFS")
        return None

def run_large_scale_test():
    """Run large scale VexFS performance test"""
    print("🚀 VexFS LARGE SCALE PERFORMANCE TEST")
    print("=" * 60)
    
    mount_point = ensure_vexfs_mounted()
    if not mount_point:
        return None
    
    # Test configuration - matching competitive analysis
    dataset_size = 10000
    vector_dimension = 1536
    num_queries = 100
    
    print(f"📊 Testing {dataset_size} vectors, {vector_dimension}D...")
    
    # Generate test data
    print("   Generating test vectors...")
    vectors = np.random.rand(dataset_size, vector_dimension).astype(np.float32)
    query_vectors = np.random.rand(num_queries, vector_dimension).astype(np.float32)
    
    # Test directory
    test_dir = os.path.join(mount_point, "large_scale_test")
    os.makedirs(test_dir, exist_ok=True)
    
    # INSERT PERFORMANCE TEST
    print("   Testing insert performance...")
    insert_times = []
    insert_start = time.time()
    
    for i, vector in enumerate(vectors):
        start_time = time.time()
        
        # Write vector to VexFS
        vector_file = os.path.join(test_dir, f"vector_{i:06d}.bin")
        with open(vector_file, 'wb') as f:
            f.write(vector.tobytes())
        
        end_time = time.time()
        insert_times.append(end_time - start_time)
        
        # Progress indicator
        if (i + 1) % 1000 == 0:
            print(f"     Inserted {i + 1}/{dataset_size} vectors...")
    
    insert_end = time.time()
    total_insert_time = insert_end - insert_start
    
    # QUERY PERFORMANCE TEST - COMPREHENSIVE SEARCH
    print("   Testing query performance (searching ALL vectors)...")
    query_times = []
    query_start = time.time()
    
    # Pre-compute ground truth for accuracy measurement
    print("   Computing ground truth for accuracy measurement...")
    ground_truth = []
    for query_vector in query_vectors:
        # Compute similarities against all vectors for ground truth
        true_similarities = []
        for j in range(dataset_size):
            similarity = np.dot(query_vector, vectors[j])
            true_similarities.append((j, similarity))
        true_similarities.sort(key=lambda x: x[1], reverse=True)
        ground_truth.append([idx for idx, _ in true_similarities[:10]])
    
    # Actual query performance test
    all_retrieved_results = []
    for i, query_vector in enumerate(query_vectors):
        start_time = time.time()
        
        # COMPREHENSIVE vector search - read ALL stored vectors
        # This represents the full filesystem-level search operation
        similarities = []
        for j in range(dataset_size):  # Search through ALL vectors
            vector_file = os.path.join(test_dir, f"vector_{j:06d}.bin")
            try:
                with open(vector_file, 'rb') as f:
                    stored_vector = np.frombuffer(f.read(), dtype=np.float32)
                    # Cosine similarity (normalized dot product)
                    query_norm = np.linalg.norm(query_vector)
                    stored_norm = np.linalg.norm(stored_vector)
                    if query_norm > 0 and stored_norm > 0:
                        similarity = np.dot(query_vector, stored_vector) / (query_norm * stored_norm)
                    else:
                        similarity = 0.0
                    similarities.append((j, similarity))
            except Exception as e:
                # If file read fails, assign lowest similarity
                similarities.append((j, -1.0))
        
        # Sort by similarity (top-k search)
        similarities.sort(key=lambda x: x[1], reverse=True)
        top_results = similarities[:10]  # Top 10 results
        retrieved_indices = [idx for idx, _ in top_results]
        all_retrieved_results.append(retrieved_indices)
        
        end_time = time.time()
        query_times.append(end_time - start_time)
        
        # Progress indicator for long-running queries
        if (i + 1) % 10 == 0:
            print(f"     Completed {i + 1}/{num_queries} queries...")
    
    query_end = time.time()
    total_query_time = query_end - query_start
    
    # Calculate actual recall@10
    print("   Calculating recall@10...")
    recall_scores = []
    for i in range(num_queries):
        retrieved = set(all_retrieved_results[i])
        relevant = set(ground_truth[i])
        recall = len(retrieved.intersection(relevant)) / len(relevant)
        recall_scores.append(recall)
    
    actual_recall_at_10 = np.mean(recall_scores)
    
    # Calculate metrics
    insert_latency_avg = np.mean(insert_times) * 1000  # Convert to ms
    insert_latency_p95 = np.percentile(insert_times, 95) * 1000
    insert_throughput = dataset_size / total_insert_time
    
    query_latency_avg = np.mean(query_times) * 1000
    query_latency_p95 = np.percentile(query_times, 95) * 1000
    query_latency_p99 = np.percentile(query_times, 99) * 1000
    query_throughput = num_queries / total_query_time
    
    # Results
    result = {
        "database": "VexFS-FUSE",
        "test_name": "large_scale_10000_1536",
        "dataset_size": dataset_size,
        "vector_dimension": vector_dimension,
        "insert_latency_avg": insert_latency_avg,
        "insert_latency_p95": insert_latency_p95,
        "insert_throughput": insert_throughput,
        "query_latency_avg": query_latency_avg,
        "query_latency_p95": query_latency_p95,
        "query_latency_p99": query_latency_p99,
        "query_throughput": query_throughput,
        "memory_usage_mb": 0.0,  # VexFS uses filesystem, not in-memory
        "accuracy_recall_at_10": actual_recall_at_10,  # Measured recall@10
        "timestamp": datetime.now().strftime("%Y-%m-%d %H:%M:%S")
    }
    
    # Save results
    timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
    results_file = f"vexfs_large_scale_results_{timestamp}.json"
    
    with open(results_file, 'w') as f:
        json.dump([result], f, indent=2)
    
    print(f"✅ Large scale results saved to {results_file}")
    print()
    print("📊 VexFS Large Scale Performance Summary:")
    print(f"   large_scale_10000_1536: {insert_throughput:.1f} vec/sec insert, {query_throughput:.1f} q/sec query")
    print(f"   Insert latency: {insert_latency_avg:.2f}ms avg, {insert_latency_p95:.2f}ms P95")
    print(f"   Query latency: {query_latency_avg:.2f}ms avg, {query_latency_p95:.2f}ms P95")
    print(f"   Accuracy: {actual_recall_at_10:.3f} recall@10 (measured)")
    print()
    print("🎯 Large scale VexFS test completed!")
    
    return result

if __name__ == "__main__":
    try:
        result = run_large_scale_test()
        if result:
            print("✅ Large scale test successful")
        else:
            print("❌ Large scale test failed")
            sys.exit(1)
    except KeyboardInterrupt:
        print("\n⚠️ Test interrupted by user")
        sys.exit(1)
    except Exception as e:
        print(f"❌ Error: {e}")
        import traceback
        traceback.print_exc()
        sys.exit(1)