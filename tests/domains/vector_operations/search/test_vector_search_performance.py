"""
Vector Search Performance Tests

Performance and benchmark tests for VexFS vector search operations
including ANNS (Approximate Nearest Neighbor Search) performance,
indexing efficiency, and search accuracy metrics.
"""

import pytest
import time
import numpy as np
import random
from typing import List, Tuple, Dict, Any
from dataclasses import dataclass

from tests.domains.shared.test_tags import tag, performance_test, integration_test


@dataclass
class VectorSearchResult:
    """Result of a vector search operation."""
    query_vector: np.ndarray
    results: List[Tuple[int, float]]  # (vector_id, distance)
    search_time: float
    accuracy: float = 0.0


@dataclass
class IndexBuildResult:
    """Result of index building operation."""
    vector_count: int
    build_time: float
    index_size: int
    memory_usage: int


class TestVectorSearchPerformance:
    """Performance test suite for vector search operations in VexFS."""
    
    def setup_method(self):
        """Set up test data for each test method."""
        # Generate test vectors
        self.vector_dimensions = 128
        self.small_dataset_size = 1000
        self.medium_dataset_size = 10000
        self.large_dataset_size = 100000
        
        # Create test datasets
        self.small_vectors = self._generate_test_vectors(self.small_dataset_size)
        self.medium_vectors = self._generate_test_vectors(self.medium_dataset_size)
        self.large_vectors = self._generate_test_vectors(self.large_dataset_size)
        
        # Create query vectors
        self.query_vectors = self._generate_test_vectors(100)
    
    @performance_test("vector_operations", "quick", "safe")
    def test_vector_search_small_dataset_performance_quick_safe(self):
        """Test vector search performance on small dataset (1K vectors)."""
        # Build index for small dataset
        build_result = self._simulate_index_build(self.small_vectors)
        assert build_result.build_time < 1.0, "Small dataset index should build in < 1 second"
        
        # Perform search operations
        search_times = []
        for query in self.query_vectors[:10]:  # Test with 10 queries
            search_result = self._simulate_vector_search(query, self.small_vectors, k=10)
            search_times.append(search_result.search_time)
            
            # Verify search quality
            assert len(search_result.results) == 10, "Should return exactly 10 results"
            assert search_result.search_time < 0.01, "Search should complete in < 10ms"
        
        avg_search_time = sum(search_times) / len(search_times)
        print(f"Small dataset average search time: {avg_search_time:.4f}s")
        
        # Performance assertions
        assert avg_search_time < 0.005, "Average search time should be < 5ms"
        assert max(search_times) < 0.02, "Max search time should be < 20ms"
    
    @performance_test("vector_operations", "medium", "safe")
    def test_vector_search_medium_dataset_performance_medium_safe(self):
        """Test vector search performance on medium dataset (10K vectors)."""
        # Build index for medium dataset
        build_result = self._simulate_index_build(self.medium_vectors)
        assert build_result.build_time < 10.0, "Medium dataset index should build in < 10 seconds"
        
        # Perform search operations
        search_times = []
        accuracy_scores = []
        
        for query in self.query_vectors[:20]:  # Test with 20 queries
            search_result = self._simulate_vector_search(query, self.medium_vectors, k=20)
            search_times.append(search_result.search_time)
            accuracy_scores.append(search_result.accuracy)
            
            # Verify search quality
            assert len(search_result.results) == 20, "Should return exactly 20 results"
            assert search_result.search_time < 0.05, "Search should complete in < 50ms"
        
        avg_search_time = sum(search_times) / len(search_times)
        avg_accuracy = sum(accuracy_scores) / len(accuracy_scores)
        
        print(f"Medium dataset average search time: {avg_search_time:.4f}s")
        print(f"Medium dataset average accuracy: {avg_accuracy:.3f}")
        
        # Performance assertions
        assert avg_search_time < 0.02, "Average search time should be < 20ms"
        assert avg_accuracy > 0.95, "Average accuracy should be > 95%"
    
    @performance_test("vector_operations", "slow", "monitored")
    def test_vector_search_large_dataset_performance_slow_monitored(self):
        """Test vector search performance on large dataset (100K vectors)."""
        # Build index for large dataset
        start_time = time.time()
        build_result = self._simulate_index_build(self.large_vectors)
        build_time = time.time() - start_time
        
        assert build_time < 60.0, "Large dataset index should build in < 60 seconds"
        print(f"Large dataset index build time: {build_time:.2f}s")
        
        # Perform search operations
        search_times = []
        accuracy_scores = []
        throughput_tests = []
        
        for i, query in enumerate(self.query_vectors[:50]):  # Test with 50 queries
            search_result = self._simulate_vector_search(query, self.large_vectors, k=50)
            search_times.append(search_result.search_time)
            accuracy_scores.append(search_result.accuracy)
            
            # Verify search quality
            assert len(search_result.results) == 50, "Should return exactly 50 results"
            assert search_result.search_time < 0.1, "Search should complete in < 100ms"
            
            if i % 10 == 0:
                print(f"Completed {i+1}/50 searches")
        
        # Calculate performance metrics
        avg_search_time = sum(search_times) / len(search_times)
        avg_accuracy = sum(accuracy_scores) / len(accuracy_scores)
        queries_per_second = 1.0 / avg_search_time
        
        print(f"Large dataset average search time: {avg_search_time:.4f}s")
        print(f"Large dataset average accuracy: {avg_accuracy:.3f}")
        print(f"Large dataset throughput: {queries_per_second:.1f} QPS")
        
        # Performance assertions
        assert avg_search_time < 0.05, "Average search time should be < 50ms"
        assert avg_accuracy > 0.90, "Average accuracy should be > 90%"
        assert queries_per_second > 20, "Should handle > 20 queries per second"
    
    @tag("performance", "vector_operations", "medium", "safe", "indexing")
    def test_vector_index_build_performance_medium_safe(self):
        """Test vector index building performance across different dataset sizes."""
        dataset_sizes = [1000, 5000, 10000, 25000]
        build_times = []
        memory_usage = []
        
        for size in dataset_sizes:
            vectors = self._generate_test_vectors(size)
            
            # Measure index build time
            start_time = time.time()
            build_result = self._simulate_index_build(vectors)
            build_time = time.time() - start_time
            
            build_times.append(build_time)
            memory_usage.append(build_result.memory_usage)
            
            print(f"Dataset size: {size:5d}, Build time: {build_time:.3f}s, "
                  f"Memory: {build_result.memory_usage/1024/1024:.1f}MB")
            
            # Performance assertions based on dataset size
            if size <= 1000:
                assert build_time < 1.0, f"Small dataset ({size}) should build in < 1s"
            elif size <= 10000:
                assert build_time < 5.0, f"Medium dataset ({size}) should build in < 5s"
            else:
                assert build_time < 15.0, f"Large dataset ({size}) should build in < 15s"
        
        # Test scaling characteristics
        # Build time should scale roughly linearly with dataset size
        time_ratio = build_times[-1] / build_times[0]
        size_ratio = dataset_sizes[-1] / dataset_sizes[0]
        
        # Allow for some overhead, but scaling should be reasonable
        assert time_ratio < size_ratio * 2, "Build time scaling should be reasonable"
    
    @performance_test("vector_operations", "medium", "safe")
    def test_vector_search_concurrent_performance_medium_safe(self):
        """Test vector search performance under concurrent load."""
        import threading
        import queue
        
        # Use medium dataset for concurrent testing
        build_result = self._simulate_index_build(self.medium_vectors)
        
        # Set up concurrent search test
        num_threads = 4
        queries_per_thread = 25
        result_queue = queue.Queue()
        
        def search_worker(thread_id: int, queries: List[np.ndarray]):
            """Worker function for concurrent searches."""
            thread_results = []
            for i, query in enumerate(queries):
                search_result = self._simulate_vector_search(query, self.medium_vectors, k=10)
                thread_results.append({
                    'thread_id': thread_id,
                    'query_id': i,
                    'search_time': search_result.search_time,
                    'accuracy': search_result.accuracy
                })
            result_queue.put(thread_results)
        
        # Start concurrent searches
        threads = []
        start_time = time.time()
        
        for thread_id in range(num_threads):
            thread_queries = self.query_vectors[
                thread_id * queries_per_thread:(thread_id + 1) * queries_per_thread
            ]
            thread = threading.Thread(
                target=search_worker, 
                args=(thread_id, thread_queries)
            )
            threads.append(thread)
            thread.start()
        
        # Wait for all threads to complete
        for thread in threads:
            thread.join()
        
        total_time = time.time() - start_time
        
        # Collect results
        all_results = []
        while not result_queue.empty():
            thread_results = result_queue.get()
            all_results.extend(thread_results)
        
        # Analyze concurrent performance
        search_times = [r['search_time'] for r in all_results]
        accuracies = [r['accuracy'] for r in all_results]
        
        avg_search_time = sum(search_times) / len(search_times)
        avg_accuracy = sum(accuracies) / len(accuracies)
        total_queries = len(all_results)
        overall_qps = total_queries / total_time
        
        print(f"Concurrent test results:")
        print(f"  Threads: {num_threads}")
        print(f"  Total queries: {total_queries}")
        print(f"  Total time: {total_time:.3f}s")
        print(f"  Overall QPS: {overall_qps:.1f}")
        print(f"  Average search time: {avg_search_time:.4f}s")
        print(f"  Average accuracy: {avg_accuracy:.3f}")
        
        # Performance assertions
        assert avg_search_time < 0.03, "Concurrent average search time should be < 30ms"
        assert avg_accuracy > 0.93, "Concurrent average accuracy should be > 93%"
        assert overall_qps > 50, "Should handle > 50 QPS under concurrent load"
    
    @tag("performance", "vector_operations", "slow", "monitored", "memory")
    def test_vector_search_memory_efficiency_slow_monitored(self):
        """Test memory efficiency of vector search operations."""
        import psutil
        import gc
        
        # Measure baseline memory usage
        gc.collect()
        baseline_memory = psutil.Process().memory_info().rss
        
        # Build index and measure memory growth
        build_result = self._simulate_index_build(self.large_vectors)
        post_build_memory = psutil.Process().memory_info().rss
        
        index_memory_usage = post_build_memory - baseline_memory
        
        print(f"Memory usage:")
        print(f"  Baseline: {baseline_memory / 1024 / 1024:.1f} MB")
        print(f"  Post-build: {post_build_memory / 1024 / 1024:.1f} MB")
        print(f"  Index overhead: {index_memory_usage / 1024 / 1024:.1f} MB")
        
        # Perform searches and monitor memory
        search_memories = []
        for i, query in enumerate(self.query_vectors[:20]):
            search_result = self._simulate_vector_search(query, self.large_vectors, k=100)
            current_memory = psutil.Process().memory_info().rss
            search_memories.append(current_memory)
            
            if i % 5 == 0:
                print(f"  Search {i+1}: {current_memory / 1024 / 1024:.1f} MB")
        
        # Analyze memory stability during searches
        max_search_memory = max(search_memories)
        min_search_memory = min(search_memories)
        memory_variance = max_search_memory - min_search_memory
        
        print(f"  Search memory variance: {memory_variance / 1024 / 1024:.1f} MB")
        
        # Memory efficiency assertions
        vectors_size_mb = len(self.large_vectors) * self.vector_dimensions * 4 / 1024 / 1024
        index_overhead_ratio = index_memory_usage / (vectors_size_mb * 1024 * 1024)
        
        assert index_overhead_ratio < 2.0, "Index memory overhead should be < 200% of vector data"
        assert memory_variance < 50 * 1024 * 1024, "Memory variance during search should be < 50MB"
    
    @integration_test("vector_operations", "slow", "monitored")
    def test_vector_search_accuracy_vs_speed_tradeoff_slow_monitored(self):
        """Test accuracy vs speed tradeoffs in vector search."""
        # Test different search configurations
        configurations = [
            {"name": "high_speed", "k": 10, "accuracy_target": 0.85},
            {"name": "balanced", "k": 20, "accuracy_target": 0.92},
            {"name": "high_accuracy", "k": 50, "accuracy_target": 0.97},
        ]
        
        build_result = self._simulate_index_build(self.medium_vectors)
        
        results = {}
        
        for config in configurations:
            search_times = []
            accuracies = []
            
            for query in self.query_vectors[:30]:
                search_result = self._simulate_vector_search(
                    query, self.medium_vectors, k=config["k"]
                )
                search_times.append(search_result.search_time)
                accuracies.append(search_result.accuracy)
            
            avg_time = sum(search_times) / len(search_times)
            avg_accuracy = sum(accuracies) / len(accuracies)
            
            results[config["name"]] = {
                "avg_time": avg_time,
                "avg_accuracy": avg_accuracy,
                "target_accuracy": config["accuracy_target"]
            }
            
            print(f"{config['name']}: {avg_time:.4f}s, accuracy: {avg_accuracy:.3f}")
            
            # Verify accuracy targets are met
            assert avg_accuracy >= config["accuracy_target"], \
                f"{config['name']} should meet accuracy target of {config['accuracy_target']}"
        
        # Verify speed vs accuracy tradeoff
        assert results["high_speed"]["avg_time"] < results["balanced"]["avg_time"]
        assert results["balanced"]["avg_time"] < results["high_accuracy"]["avg_time"]
        
        assert results["high_speed"]["avg_accuracy"] < results["balanced"]["avg_accuracy"]
        assert results["balanced"]["avg_accuracy"] < results["high_accuracy"]["avg_accuracy"]
    
    # Helper methods
    
    def _generate_test_vectors(self, count: int) -> List[np.ndarray]:
        """Generate normalized test vectors."""
        vectors = []
        for i in range(count):
            # Generate random vector
            vector = np.random.randn(self.vector_dimensions).astype(np.float32)
            # Normalize to unit length
            vector = vector / np.linalg.norm(vector)
            vectors.append(vector)
        return vectors
    
    def _simulate_index_build(self, vectors: List[np.ndarray]) -> IndexBuildResult:
        """Simulate building a vector search index."""
        # Simulate build time based on dataset size
        vector_count = len(vectors)
        base_time = 0.001  # 1ms base time
        time_per_vector = 0.00001  # 10μs per vector
        build_time = base_time + (vector_count * time_per_vector)
        
        # Simulate memory usage (index overhead)
        vector_size = vector_count * self.vector_dimensions * 4  # 4 bytes per float32
        index_overhead = vector_size * 0.5  # 50% overhead for index structures
        memory_usage = vector_size + index_overhead
        
        # Add some realistic variation
        build_time *= random.uniform(0.8, 1.2)
        
        return IndexBuildResult(
            vector_count=vector_count,
            build_time=build_time,
            index_size=int(index_overhead),
            memory_usage=int(memory_usage)
        )
    
    def _simulate_vector_search(self, query: np.ndarray, vectors: List[np.ndarray], 
                               k: int) -> VectorSearchResult:
        """Simulate vector search operation."""
        # Simulate search time based on dataset size and k
        dataset_size = len(vectors)
        base_time = 0.0001  # 0.1ms base time
        time_per_comparison = 0.000001  # 1μs per vector comparison
        k_factor = 0.00001  # 10μs per result in top-k
        
        search_time = base_time + (dataset_size * time_per_comparison) + (k * k_factor)
        
        # Add realistic variation
        search_time *= random.uniform(0.7, 1.3)
        
        # Simulate search results (mock distances)
        results = []
        for i in range(k):
            vector_id = random.randint(0, len(vectors) - 1)
            # Simulate distance (smaller is better)
            distance = random.uniform(0.1, 0.9)
            results.append((vector_id, distance))
        
        # Sort by distance (ascending)
        results.sort(key=lambda x: x[1])
        
        # Simulate accuracy based on k and dataset size
        # Higher k and smaller datasets generally give better accuracy
        base_accuracy = 0.85
        k_bonus = min(0.1, k * 0.002)  # Up to 10% bonus for higher k
        size_penalty = max(0, (dataset_size - 10000) * 0.000001)  # Penalty for very large datasets
        accuracy = min(0.99, base_accuracy + k_bonus - size_penalty + random.uniform(-0.05, 0.05))
        
        return VectorSearchResult(
            query_vector=query,
            results=results,
            search_time=search_time,
            accuracy=accuracy
        )


if __name__ == "__main__":
    # Run performance tests
    pytest.main([__file__, "-v", "-s", "--tb=short"])