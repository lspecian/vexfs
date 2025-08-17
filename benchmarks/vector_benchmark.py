#!/usr/bin/env python3
"""
VexFS Vector Operations Benchmark Suite

Tests performance of vector storage, retrieval, and search operations
across different APIs (ChromaDB, Qdrant, Native VexFS)
"""

import time
import json
import random
import numpy as np
import requests
from typing import List, Dict, Any, Tuple
from dataclasses import dataclass
from concurrent.futures import ThreadPoolExecutor, as_completed
import statistics
import argparse

@dataclass
class BenchmarkResult:
    operation: str
    api: str
    num_operations: int
    total_time: float
    avg_time: float
    min_time: float
    max_time: float
    std_dev: float
    throughput: float
    
    def to_dict(self):
        return {
            'operation': self.operation,
            'api': self.api,
            'num_operations': self.num_operations,
            'total_time': self.total_time,
            'avg_time': self.avg_time,
            'min_time': self.min_time,
            'max_time': self.max_time,
            'std_dev': self.std_dev,
            'throughput': self.throughput
        }

class VexFSBenchmark:
    def __init__(self, base_url: str = "http://localhost:7680", api_key: str = None):
        self.base_url = base_url
        self.api_key = api_key or "vexfs-default-key"
        self.token = None
        self.session = requests.Session()
        self._authenticate()
        
    def _authenticate(self):
        """Get JWT token for authenticated operations"""
        try:
            response = self.session.post(
                f"{self.base_url}/auth/login",
                json={"api_key": self.api_key}
            )
            if response.status_code == 200:
                self.token = response.json().get("token")
                self.session.headers.update({"Authorization": self.token})
        except:
            print("Warning: Authentication failed, continuing without auth")
    
    def generate_vectors(self, num_vectors: int, dimension: int = 384) -> List[List[float]]:
        """Generate random normalized vectors"""
        vectors = []
        for _ in range(num_vectors):
            vec = np.random.randn(dimension)
            vec = vec / np.linalg.norm(vec)  # Normalize
            vectors.append(vec.tolist())
        return vectors
    
    def generate_documents(self, num_docs: int) -> List[str]:
        """Generate sample documents"""
        return [f"Document {i}: " + " ".join([
            random.choice(["important", "critical", "normal", "low", "high"]),
            random.choice(["data", "information", "content", "text", "material"]),
            f"with id {i}"
        ]) for i in range(num_docs)]
    
    def benchmark_chromadb_insert(self, num_docs: int, dimension: int = 384) -> Tuple[BenchmarkResult, bool]:
        """Benchmark ChromaDB document insertion"""
        collection_name = f"bench_chromadb_{int(time.time())}"
        
        # Create collection
        self.session.post(
            f"{self.base_url}/api/v1/collections",
            json={"name": collection_name, "metadata": {"dimension": dimension}}
        )
        
        vectors = self.generate_vectors(num_docs, dimension)
        documents = self.generate_documents(num_docs)
        ids = [f"doc_{i}" for i in range(num_docs)]
        
        # Batch insert for efficiency
        batch_size = 100
        times = []
        
        for i in range(0, num_docs, batch_size):
            batch_end = min(i + batch_size, num_docs)
            batch_data = {
                "ids": ids[i:batch_end],
                "documents": documents[i:batch_end],
                "embeddings": vectors[i:batch_end]
            }
            
            start = time.perf_counter()
            response = self.session.post(
                f"{self.base_url}/api/v1/collections/{collection_name}/add",
                json=batch_data
            )
            elapsed = time.perf_counter() - start
            times.append(elapsed)
            
            if response.status_code != 200:
                return None, False
        
        total_time = sum(times)
        
        # Clean up
        self.session.delete(f"{self.base_url}/api/v1/collections/{collection_name}")
        
        return BenchmarkResult(
            operation="insert",
            api="ChromaDB",
            num_operations=num_docs,
            total_time=total_time,
            avg_time=total_time / num_docs,
            min_time=min(times) if times else 0,
            max_time=max(times) if times else 0,
            std_dev=statistics.stdev(times) if len(times) > 1 else 0,
            throughput=num_docs / total_time if total_time > 0 else 0
        ), True
    
    def benchmark_chromadb_search(self, num_queries: int, collection_size: int = 1000, k: int = 10) -> Tuple[BenchmarkResult, bool]:
        """Benchmark ChromaDB vector search"""
        collection_name = f"bench_search_{int(time.time())}"
        dimension = 384
        
        # Setup collection with data
        self.session.post(
            f"{self.base_url}/api/v1/collections",
            json={"name": collection_name, "metadata": {"dimension": dimension}}
        )
        
        # Insert test data
        vectors = self.generate_vectors(collection_size, dimension)
        documents = self.generate_documents(collection_size)
        ids = [f"doc_{i}" for i in range(collection_size)]
        
        self.session.post(
            f"{self.base_url}/api/v1/collections/{collection_name}/add",
            json={"ids": ids, "documents": documents, "embeddings": vectors}
        )
        
        # Generate query vectors
        query_vectors = self.generate_vectors(num_queries, dimension)
        
        # Perform searches
        times = []
        for query_vec in query_vectors:
            start = time.perf_counter()
            response = self.session.post(
                f"{self.base_url}/api/v1/collections/{collection_name}/query",
                json={"query_embeddings": [query_vec], "n_results": k}
            )
            elapsed = time.perf_counter() - start
            times.append(elapsed)
            
            if response.status_code != 200:
                return None, False
        
        total_time = sum(times)
        
        # Clean up
        self.session.delete(f"{self.base_url}/api/v1/collections/{collection_name}")
        
        return BenchmarkResult(
            operation=f"search_k{k}",
            api="ChromaDB",
            num_operations=num_queries,
            total_time=total_time,
            avg_time=statistics.mean(times),
            min_time=min(times),
            max_time=max(times),
            std_dev=statistics.stdev(times) if len(times) > 1 else 0,
            throughput=num_queries / total_time if total_time > 0 else 0
        ), True
    
    def benchmark_qdrant_insert(self, num_docs: int, dimension: int = 384) -> Tuple[BenchmarkResult, bool]:
        """Benchmark Qdrant point insertion"""
        collection_name = f"bench_qdrant_{int(time.time())}"
        
        # Create collection
        self.session.put(
            f"{self.base_url}/collections/{collection_name}",
            json={"vectors": {"size": dimension, "distance": "Cosine"}}
        )
        
        vectors = self.generate_vectors(num_docs, dimension)
        
        # Create points
        points = []
        for i, vec in enumerate(vectors):
            points.append({
                "id": i,
                "vector": vec,
                "payload": {"text": f"Document {i}"}
            })
        
        # Batch insert
        batch_size = 100
        times = []
        
        for i in range(0, num_docs, batch_size):
            batch_end = min(i + batch_size, num_docs)
            batch_points = points[i:batch_end]
            
            start = time.perf_counter()
            response = self.session.put(
                f"{self.base_url}/collections/{collection_name}/points",
                json={"points": batch_points}
            )
            elapsed = time.perf_counter() - start
            times.append(elapsed)
            
            if response.status_code != 200:
                return None, False
        
        total_time = sum(times)
        
        return BenchmarkResult(
            operation="insert",
            api="Qdrant",
            num_operations=num_docs,
            total_time=total_time,
            avg_time=total_time / num_docs,
            min_time=min(times) if times else 0,
            max_time=max(times) if times else 0,
            std_dev=statistics.stdev(times) if len(times) > 1 else 0,
            throughput=num_docs / total_time if total_time > 0 else 0
        ), True
    
    def benchmark_concurrent_operations(self, num_threads: int = 10, ops_per_thread: int = 100):
        """Benchmark concurrent operations"""
        collection_name = f"bench_concurrent_{int(time.time())}"
        dimension = 384
        
        # Setup collection
        self.session.post(
            f"{self.base_url}/api/v1/collections",
            json={"name": collection_name, "metadata": {"dimension": dimension}}
        )
        
        def worker(thread_id: int) -> List[float]:
            times = []
            vectors = self.generate_vectors(ops_per_thread, dimension)
            
            for i, vec in enumerate(vectors):
                doc_id = f"thread_{thread_id}_doc_{i}"
                start = time.perf_counter()
                
                response = self.session.post(
                    f"{self.base_url}/api/v1/collections/{collection_name}/add",
                    json={
                        "ids": [doc_id],
                        "documents": [f"Document from thread {thread_id}"],
                        "embeddings": [vec]
                    }
                )
                
                elapsed = time.perf_counter() - start
                times.append(elapsed)
            
            return times
        
        # Run concurrent operations
        all_times = []
        start_time = time.perf_counter()
        
        with ThreadPoolExecutor(max_workers=num_threads) as executor:
            futures = [executor.submit(worker, i) for i in range(num_threads)]
            for future in as_completed(futures):
                all_times.extend(future.result())
        
        total_time = time.perf_counter() - start_time
        total_ops = num_threads * ops_per_thread
        
        # Clean up
        self.session.delete(f"{self.base_url}/api/v1/collections/{collection_name}")
        
        return BenchmarkResult(
            operation=f"concurrent_{num_threads}threads",
            api="ChromaDB",
            num_operations=total_ops,
            total_time=total_time,
            avg_time=statistics.mean(all_times),
            min_time=min(all_times),
            max_time=max(all_times),
            std_dev=statistics.stdev(all_times) if len(all_times) > 1 else 0,
            throughput=total_ops / total_time
        ), True
    
    def run_all_benchmarks(self, sizes: List[int] = None) -> List[BenchmarkResult]:
        """Run complete benchmark suite"""
        if sizes is None:
            sizes = [100, 500, 1000, 5000]
        
        results = []
        
        print("Starting VexFS Performance Benchmarks")
        print("=" * 50)
        
        for size in sizes:
            print(f"\nBenchmarking with {size} documents/queries...")
            
            # ChromaDB benchmarks
            print("  - ChromaDB insert...", end=" ")
            result, success = self.benchmark_chromadb_insert(size)
            if success:
                results.append(result)
                print(f"✓ {result.throughput:.2f} ops/sec")
            else:
                print("✗ Failed")
            
            print("  - ChromaDB search...", end=" ")
            result, success = self.benchmark_chromadb_search(
                num_queries=min(100, size // 10),
                collection_size=size
            )
            if success:
                results.append(result)
                print(f"✓ {result.throughput:.2f} ops/sec")
            else:
                print("✗ Failed")
            
            # Qdrant benchmarks
            print("  - Qdrant insert...", end=" ")
            result, success = self.benchmark_qdrant_insert(size)
            if success:
                results.append(result)
                print(f"✓ {result.throughput:.2f} ops/sec")
            else:
                print("✗ Failed")
        
        # Concurrent operations benchmark
        print("\nBenchmarking concurrent operations...")
        result, success = self.benchmark_concurrent_operations(
            num_threads=10,
            ops_per_thread=100
        )
        if success:
            results.append(result)
            print(f"  ✓ {result.throughput:.2f} ops/sec with 10 threads")
        
        return results
    
    def print_results(self, results: List[BenchmarkResult]):
        """Print formatted benchmark results"""
        print("\n" + "=" * 80)
        print("BENCHMARK RESULTS SUMMARY")
        print("=" * 80)
        
        # Group by operation type
        by_operation = {}
        for r in results:
            if r.operation not in by_operation:
                by_operation[r.operation] = []
            by_operation[r.operation].append(r)
        
        for op, op_results in by_operation.items():
            print(f"\n{op.upper()} OPERATIONS:")
            print("-" * 40)
            
            for r in op_results:
                print(f"\n  API: {r.api}")
                print(f"  Documents: {r.num_operations}")
                print(f"  Total Time: {r.total_time:.3f}s")
                print(f"  Avg Latency: {r.avg_time * 1000:.2f}ms")
                print(f"  Min Latency: {r.min_time * 1000:.2f}ms")
                print(f"  Max Latency: {r.max_time * 1000:.2f}ms")
                print(f"  Std Dev: {r.std_dev * 1000:.2f}ms")
                print(f"  Throughput: {r.throughput:.2f} ops/sec")
        
        # Overall statistics
        print("\n" + "=" * 80)
        print("OVERALL PERFORMANCE METRICS:")
        print("-" * 40)
        
        total_ops = sum(r.num_operations for r in results)
        total_time = sum(r.total_time for r in results)
        avg_throughput = statistics.mean(r.throughput for r in results)
        
        print(f"Total Operations: {total_ops}")
        print(f"Total Time: {total_time:.2f}s")
        print(f"Average Throughput: {avg_throughput:.2f} ops/sec")
        
    def save_results(self, results: List[BenchmarkResult], filename: str = None):
        """Save results to JSON file"""
        if filename is None:
            filename = f"benchmark_results_{int(time.time())}.json"
        
        data = {
            "timestamp": time.time(),
            "results": [r.to_dict() for r in results]
        }
        
        with open(filename, 'w') as f:
            json.dump(data, f, indent=2)
        
        print(f"\nResults saved to {filename}")

def main():
    parser = argparse.ArgumentParser(description="VexFS Performance Benchmark Suite")
    parser.add_argument("--url", default="http://localhost:7680", help="VexFS server URL")
    parser.add_argument("--api-key", help="API key for authentication")
    parser.add_argument("--sizes", nargs="+", type=int, default=[100, 500, 1000],
                       help="Document sizes to benchmark")
    parser.add_argument("--output", help="Output JSON file for results")
    
    args = parser.parse_args()
    
    # Run benchmarks
    benchmark = VexFSBenchmark(args.url, args.api_key)
    results = benchmark.run_all_benchmarks(args.sizes)
    
    # Display results
    benchmark.print_results(results)
    
    # Save results
    if args.output:
        benchmark.save_results(results, args.output)
    
    print("\n✅ Benchmark complete!")

if __name__ == "__main__":
    main()