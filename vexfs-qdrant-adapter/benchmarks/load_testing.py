"""
VexFS v2 Qdrant Adapter - Load Testing Module

Comprehensive load testing scenarios for production workload simulation.
"""

import asyncio
import aiohttp
import time
import random
import numpy as np
from typing import Dict, List, Any, Optional
from dataclasses import dataclass
import logging
from concurrent.futures import ThreadPoolExecutor
import json

from .performance_suite import BenchmarkResult

logger = logging.getLogger(__name__)

@dataclass
class LoadTestConfig:
    """Load test configuration"""
    base_url: str
    max_concurrent: int = 100
    ramp_up_seconds: int = 30
    steady_state_seconds: int = 300
    ramp_down_seconds: int = 30
    request_timeout: int = 30

class LoadTester:
    """
    Advanced load testing for VexFS v2 Qdrant adapter.
    
    Simulates real-world production workloads with various patterns:
    - Sustained high throughput
    - Burst traffic patterns
    - Mixed operation types
    - Concurrent user simulation
    """
    
    def __init__(self, base_url: str = "http://localhost:6333"):
        self.base_url = base_url
        self.session = None
        self.config = LoadTestConfig(base_url)
        
    async def __aenter__(self):
        """Async context manager entry"""
        self.session = aiohttp.ClientSession(
            timeout=aiohttp.ClientTimeout(total=self.config.request_timeout)
        )
        return self
    
    async def __aexit__(self, exc_type, exc_val, exc_tb):
        """Async context manager exit"""
        if self.session:
            await self.session.close()
    
    async def benchmark_vector_search(self, num_vectors: int, vector_dim: int, search_queries: int) -> BenchmarkResult:
        """
        Benchmark vector search performance with realistic workload.
        
        Args:
            num_vectors: Number of vectors to insert
            vector_dim: Vector dimensionality
            search_queries: Number of search queries to execute
        """
        logger.info(f"🔍 Benchmarking vector search: {num_vectors} vectors, {search_queries} queries")
        
        start_time = time.time()
        success_count = 0
        error_count = 0
        
        async with self:
            # Create collection
            collection_name = f"benchmark_vectors_{int(time.time())}"
            await self._create_test_collection(collection_name, vector_dim)
            
            # Insert vectors in batches
            batch_size = 1000
            for i in range(0, num_vectors, batch_size):
                batch_vectors = []
                for j in range(min(batch_size, num_vectors - i)):
                    vector = np.random.random(vector_dim).tolist()
                    batch_vectors.append({
                        "id": i + j,
                        "vector": vector,
                        "payload": {
                            "category": random.choice(["electronics", "books", "clothing"]),
                            "price": random.uniform(10, 1000),
                            "rating": random.uniform(1, 5)
                        }
                    })
                
                try:
                    await self._upsert_points(collection_name, batch_vectors)
                    success_count += len(batch_vectors)
                except Exception as e:
                    logger.error(f"Batch insert failed: {e}")
                    error_count += len(batch_vectors)
            
            # Execute search queries
            search_start = time.time()
            search_tasks = []
            
            for _ in range(search_queries):
                query_vector = np.random.random(vector_dim).tolist()
                task = self._search_vectors(collection_name, query_vector, limit=10)
                search_tasks.append(task)
            
            # Execute searches concurrently
            search_results = await asyncio.gather(*search_tasks, return_exceptions=True)
            search_success = sum(1 for r in search_results if not isinstance(r, Exception))
            search_errors = len(search_results) - search_success
            
            search_duration = time.time() - search_start
            
            # Cleanup
            await self._delete_collection(collection_name)
        
        total_duration = (time.time() - start_time) * 1000
        search_throughput = search_queries / search_duration if search_duration > 0 else 0
        
        return BenchmarkResult(
            name="Vector Search Performance",
            duration_ms=total_duration,
            throughput_ops_sec=search_throughput,
            memory_usage_mb=0,  # Will be measured by memory profiler
            cpu_usage_percent=0,
            success_rate=search_success / search_queries if search_queries > 0 else 0,
            error_count=search_errors,
            metadata={
                "num_vectors": num_vectors,
                "vector_dim": vector_dim,
                "search_queries": search_queries,
                "insert_success": success_count,
                "insert_errors": error_count
            },
            timestamp=time.time()
        )
    
    async def benchmark_filter_operations(self, num_points: int, filter_complexity: int) -> BenchmarkResult:
        """Benchmark Filter DSL performance"""
        logger.info(f"🔍 Benchmarking filter operations: {num_points} points, complexity {filter_complexity}")
        
        start_time = time.time()
        success_count = 0
        error_count = 0
        
        async with self:
            collection_name = f"benchmark_filters_{int(time.time())}"
            await self._create_test_collection(collection_name, 128)
            
            # Insert test data with filterable fields
            batch_size = 1000
            for i in range(0, num_points, batch_size):
                batch_points = []
                for j in range(min(batch_size, num_points - i)):
                    point = {
                        "id": i + j,
                        "vector": np.random.random(128).tolist(),
                        "payload": {
                            "category": random.choice(["electronics", "books", "clothing", "home", "sports"]),
                            "price": random.uniform(1, 1000),
                            "rating": random.uniform(1, 5),
                            "in_stock": random.choice([True, False]),
                            "brand": random.choice(["apple", "samsung", "sony", "nike", "adidas"]),
                            "year": random.randint(2020, 2024)
                        }
                    }
                    batch_points.append(point)
                
                await self._upsert_points(collection_name, batch_points)
            
            # Test various filter operations
            filter_tests = [
                # Simple filters
                {"key": "category", "match": {"value": "electronics"}},
                {"key": "price", "range": {"gte": 100, "lt": 500}},
                {"key": "in_stock", "match": {"value": True}},
                
                # Complex boolean filters
                {
                    "must": [
                        {"key": "category", "match": {"value": "electronics"}},
                        {"key": "price", "range": {"gte": 50, "lt": 200}}
                    ]
                },
                {
                    "should": [
                        {"key": "brand", "match": {"value": "apple"}},
                        {"key": "brand", "match": {"value": "samsung"}}
                    ]
                },
                {
                    "must": [
                        {"key": "in_stock", "match": {"value": True}},
                        {"key": "rating", "range": {"gte": 4.0}}
                    ],
                    "must_not": [
                        {"key": "category", "match": {"value": "books"}}
                    ]
                }
            ]
            
            # Execute filter tests
            for filter_query in filter_tests:
                try:
                    query_vector = np.random.random(128).tolist()
                    result = await self._search_with_filter(collection_name, query_vector, filter_query)
                    success_count += 1
                except Exception as e:
                    logger.error(f"Filter query failed: {e}")
                    error_count += 1
            
            await self._delete_collection(collection_name)
        
        duration = (time.time() - start_time) * 1000
        throughput = len(filter_tests) / (duration / 1000) if duration > 0 else 0
        
        return BenchmarkResult(
            name="Filter DSL Performance",
            duration_ms=duration,
            throughput_ops_sec=throughput,
            memory_usage_mb=0,
            cpu_usage_percent=0,
            success_rate=success_count / len(filter_tests) if filter_tests else 0,
            error_count=error_count,
            metadata={
                "num_points": num_points,
                "filter_complexity": filter_complexity,
                "filter_tests": len(filter_tests)
            },
            timestamp=time.time()
        )
    
    async def benchmark_recommendations(self, num_examples: int, strategies: List[str]) -> BenchmarkResult:
        """Benchmark recommendation system performance"""
        logger.info(f"🎯 Benchmarking recommendations: {num_examples} examples, {len(strategies)} strategies")
        
        start_time = time.time()
        success_count = 0
        error_count = 0
        
        async with self:
            collection_name = f"benchmark_recommendations_{int(time.time())}"
            await self._create_test_collection(collection_name, 128)
            
            # Insert test vectors
            test_vectors = []
            for i in range(1000):  # Base dataset
                vector = {
                    "id": i,
                    "vector": np.random.random(128).tolist(),
                    "payload": {"category": random.choice(["A", "B", "C"])}
                }
                test_vectors.append(vector)
            
            await self._upsert_points(collection_name, test_vectors)
            
            # Test recommendation strategies
            for strategy in strategies:
                try:
                    positive_examples = random.sample(range(1000), min(num_examples, 50))
                    negative_examples = random.sample(range(1000), min(num_examples // 2, 25))
                    
                    result = await self._get_recommendations(
                        collection_name,
                        positive_examples,
                        negative_examples,
                        strategy
                    )
                    success_count += 1
                except Exception as e:
                    logger.error(f"Recommendation failed for strategy {strategy}: {e}")
                    error_count += 1
            
            await self._delete_collection(collection_name)
        
        duration = (time.time() - start_time) * 1000
        throughput = len(strategies) / (duration / 1000) if duration > 0 else 0
        
        return BenchmarkResult(
            name="Recommendation Performance",
            duration_ms=duration,
            throughput_ops_sec=throughput,
            memory_usage_mb=0,
            cpu_usage_percent=0,
            success_rate=success_count / len(strategies) if strategies else 0,
            error_count=error_count,
            metadata={
                "num_examples": num_examples,
                "strategies": strategies,
                "test_vectors": len(test_vectors)
            },
            timestamp=time.time()
        )
    
    async def benchmark_scroll_operations(self, total_points: int, batch_size: int) -> BenchmarkResult:
        """Benchmark Scroll API performance"""
        logger.info(f"📜 Benchmarking scroll operations: {total_points} points, batch size {batch_size}")
        
        start_time = time.time()
        success_count = 0
        error_count = 0
        
        async with self:
            collection_name = f"benchmark_scroll_{int(time.time())}"
            await self._create_test_collection(collection_name, 128)
            
            # Insert test data
            insert_batch_size = 1000
            for i in range(0, total_points, insert_batch_size):
                batch_points = []
                for j in range(min(insert_batch_size, total_points - i)):
                    point = {
                        "id": i + j,
                        "vector": np.random.random(128).tolist(),
                        "payload": {"index": i + j}
                    }
                    batch_points.append(point)
                
                await self._upsert_points(collection_name, batch_points)
            
            # Test scroll operations
            scroll_start = time.time()
            offset = None
            points_retrieved = 0
            
            while True:
                try:
                    result = await self._scroll_points(collection_name, batch_size, offset)
                    if not result.get("points"):
                        break
                    
                    points_retrieved += len(result["points"])
                    success_count += 1
                    offset = result.get("next_page_offset")
                    
                    if not offset:
                        break
                        
                except Exception as e:
                    logger.error(f"Scroll operation failed: {e}")
                    error_count += 1
                    break
            
            scroll_duration = time.time() - scroll_start
            await self._delete_collection(collection_name)
        
        duration = (time.time() - start_time) * 1000
        scroll_throughput = points_retrieved / scroll_duration if scroll_duration > 0 else 0
        
        return BenchmarkResult(
            name="Scroll API Performance",
            duration_ms=duration,
            throughput_ops_sec=scroll_throughput,
            memory_usage_mb=0,
            cpu_usage_percent=0,
            success_rate=success_count / (success_count + error_count) if (success_count + error_count) > 0 else 0,
            error_count=error_count,
            metadata={
                "total_points": total_points,
                "batch_size": batch_size,
                "points_retrieved": points_retrieved,
                "scroll_operations": success_count
            },
            timestamp=time.time()
        )
    
    async def benchmark_batch_search(self, num_queries: int, concurrent_batches: int) -> BenchmarkResult:
        """Benchmark batch search operations"""
        logger.info(f"🔄 Benchmarking batch search: {num_queries} queries, {concurrent_batches} batches")
        
        start_time = time.time()
        success_count = 0
        error_count = 0
        
        async with self:
            collection_name = f"benchmark_batch_{int(time.time())}"
            await self._create_test_collection(collection_name, 128)
            
            # Insert test data
            test_vectors = []
            for i in range(1000):
                vector = {
                    "id": i,
                    "vector": np.random.random(128).tolist(),
                    "payload": {"category": random.choice(["A", "B", "C"])}
                }
                test_vectors.append(vector)
            
            await self._upsert_points(collection_name, test_vectors)
            
            # Create batch search requests
            batch_tasks = []
            for batch_id in range(concurrent_batches):
                searches = []
                for query_id in range(num_queries // concurrent_batches):
                    search = {
                        "vector": np.random.random(128).tolist(),
                        "limit": 10,
                        "filter": {"key": "category", "match": {"value": random.choice(["A", "B", "C"])}}
                    }
                    searches.append(search)
                
                task = self._batch_search(collection_name, searches)
                batch_tasks.append(task)
            
            # Execute batch searches concurrently
            batch_results = await asyncio.gather(*batch_tasks, return_exceptions=True)
            
            for result in batch_results:
                if isinstance(result, Exception):
                    error_count += 1
                else:
                    success_count += 1
            
            await self._delete_collection(collection_name)
        
        duration = (time.time() - start_time) * 1000
        throughput = num_queries / (duration / 1000) if duration > 0 else 0
        
        return BenchmarkResult(
            name="Batch Search Performance",
            duration_ms=duration,
            throughput_ops_sec=throughput,
            memory_usage_mb=0,
            cpu_usage_percent=0,
            success_rate=success_count / concurrent_batches if concurrent_batches > 0 else 0,
            error_count=error_count,
            metadata={
                "num_queries": num_queries,
                "concurrent_batches": concurrent_batches,
                "queries_per_batch": num_queries // concurrent_batches
            },
            timestamp=time.time()
        )
    
    async def sustained_load_test(self, target_ops_sec: int, duration_minutes: int) -> BenchmarkResult:
        """Test sustained load performance"""
        logger.info(f"⏱️ Sustained load test: {target_ops_sec} ops/sec for {duration_minutes} minutes")
        
        start_time = time.time()
        end_time = start_time + (duration_minutes * 60)
        success_count = 0
        error_count = 0
        
        async with self:
            collection_name = f"sustained_load_{int(time.time())}"
            await self._create_test_collection(collection_name, 128)
            
            # Insert base data
            base_vectors = []
            for i in range(10000):
                vector = {
                    "id": i,
                    "vector": np.random.random(128).tolist(),
                    "payload": {"index": i}
                }
                base_vectors.append(vector)
            
            await self._upsert_points(collection_name, base_vectors)
            
            # Sustained load loop
            operation_interval = 1.0 / target_ops_sec
            
            while time.time() < end_time:
                operation_start = time.time()
                
                try:
                    # Mix of operations
                    operation_type = random.choice(["search", "insert", "scroll"])
                    
                    if operation_type == "search":
                        query_vector = np.random.random(128).tolist()
                        await self._search_vectors(collection_name, query_vector, limit=10)
                    elif operation_type == "insert":
                        new_vector = {
                            "id": success_count + 10000,
                            "vector": np.random.random(128).tolist(),
                            "payload": {"index": success_count + 10000}
                        }
                        await self._upsert_points(collection_name, [new_vector])
                    elif operation_type == "scroll":
                        await self._scroll_points(collection_name, 100)
                    
                    success_count += 1
                    
                except Exception as e:
                    error_count += 1
                
                # Rate limiting
                operation_duration = time.time() - operation_start
                if operation_duration < operation_interval:
                    await asyncio.sleep(operation_interval - operation_duration)
            
            await self._delete_collection(collection_name)
        
        total_duration = (time.time() - start_time) * 1000
        actual_ops_sec = success_count / (total_duration / 1000) if total_duration > 0 else 0
        
        return BenchmarkResult(
            name="Sustained Load Test",
            duration_ms=total_duration,
            throughput_ops_sec=actual_ops_sec,
            memory_usage_mb=0,
            cpu_usage_percent=0,
            success_rate=success_count / (success_count + error_count) if (success_count + error_count) > 0 else 0,
            error_count=error_count,
            metadata={
                "target_ops_sec": target_ops_sec,
                "duration_minutes": duration_minutes,
                "total_operations": success_count + error_count
            },
            timestamp=time.time()
        )
    
    async def stress_test(self, max_load_multiplier: int, duration_minutes: int) -> BenchmarkResult:
        """Stress test with increasing load"""
        logger.info(f"💥 Stress test: {max_load_multiplier}x load for {duration_minutes} minutes")
        
        start_time = time.time()
        success_count = 0
        error_count = 0
        
        async with self:
            collection_name = f"stress_test_{int(time.time())}"
            await self._create_test_collection(collection_name, 128)
            
            # Gradually increase load
            base_ops_sec = 100
            duration_per_level = duration_minutes * 60 / max_load_multiplier
            
            for load_level in range(1, max_load_multiplier + 1):
                level_start = time.time()
                level_end = level_start + duration_per_level
                target_ops_sec = base_ops_sec * load_level
                operation_interval = 1.0 / target_ops_sec
                
                logger.info(f"Stress level {load_level}: {target_ops_sec} ops/sec")
                
                while time.time() < level_end:
                    operation_start = time.time()
                    
                    try:
                        query_vector = np.random.random(128).tolist()
                        await self._search_vectors(collection_name, query_vector, limit=10)
                        success_count += 1
                    except Exception:
                        error_count += 1
                    
                    operation_duration = time.time() - operation_start
                    if operation_duration < operation_interval:
                        await asyncio.sleep(operation_interval - operation_duration)
            
            await self._delete_collection(collection_name)
        
        total_duration = (time.time() - start_time) * 1000
        avg_ops_sec = success_count / (total_duration / 1000) if total_duration > 0 else 0
        
        return BenchmarkResult(
            name="Stress Test",
            duration_ms=total_duration,
            throughput_ops_sec=avg_ops_sec,
            memory_usage_mb=0,
            cpu_usage_percent=0,
            success_rate=success_count / (success_count + error_count) if (success_count + error_count) > 0 else 0,
            error_count=error_count,
            metadata={
                "max_load_multiplier": max_load_multiplier,
                "duration_minutes": duration_minutes,
                "total_operations": success_count + error_count
            },
            timestamp=time.time()
        )
    
    # Helper methods for API calls
    async def _create_test_collection(self, name: str, vector_size: int):
        """Create test collection"""
        url = f"{self.base_url}/collections/{name}"
        data = {
            "vectors": {
                "size": vector_size,
                "distance": "Cosine"
            }
        }
        async with self.session.put(url, json=data) as response:
            if response.status not in [200, 201]:
                raise Exception(f"Failed to create collection: {response.status}")
    
    async def _upsert_points(self, collection_name: str, points: List[Dict]):
        """Upsert points to collection"""
        url = f"{self.base_url}/collections/{collection_name}/points"
        data = {"points": points}
        async with self.session.put(url, json=data) as response:
            if response.status not in [200, 201]:
                raise Exception(f"Failed to upsert points: {response.status}")
    
    async def _search_vectors(self, collection_name: str, vector: List[float], limit: int = 10):
        """Search vectors in collection"""
        url = f"{self.base_url}/collections/{collection_name}/points/search"
        data = {"vector": vector, "limit": limit}
        async with self.session.post(url, json=data) as response:
            if response.status != 200:
                raise Exception(f"Search failed: {response.status}")
            return await response.json()
    
    async def _search_with_filter(self, collection_name: str, vector: List[float], filter_query: Dict):
        """Search with filter"""
        url = f"{self.base_url}/collections/{collection_name}/points/search"
        data = {"vector": vector, "limit": 10, "filter": filter_query}
        async with self.session.post(url, json=data) as response:
            if response.status != 200:
                raise Exception(f"Filtered search failed: {response.status}")
            return await response.json()
    
    async def _get_recommendations(self, collection_name: str, positive: List[int], negative: List[int], strategy: str):
        """Get recommendations"""
        url = f"{self.base_url}/collections/{collection_name}/points/recommend"
        data = {
            "positive": positive,
            "negative": negative,
            "strategy": strategy,
            "limit": 10
        }
        async with self.session.post(url, json=data) as response:
            if response.status != 200:
                raise Exception(f"Recommendation failed: {response.status}")
            return await response.json()
    
    async def _scroll_points(self, collection_name: str, limit: int, offset: Optional[str] = None):
        """Scroll through points"""
        url = f"{self.base_url}/collections/{collection_name}/points/scroll"
        data = {"limit": limit}
        if offset:
            data["offset"] = offset
        
        async with self.session.post(url, json=data) as response:
            if response.status != 200:
                raise Exception(f"Scroll failed: {response.status}")
            return await response.json()
    
    async def _batch_search(self, collection_name: str, searches: List[Dict]):
        """Execute batch search"""
        url = f"{self.base_url}/collections/{collection_name}/points/search/batch"
        data = {"searches": searches}
        async with self.session.post(url, json=data) as response:
            if response.status != 200:
                raise Exception(f"Batch search failed: {response.status}")
            return await response.json()
    
    async def _delete_collection(self, name: str):
        """Delete test collection"""
        url = f"{self.base_url}/collections/{name}"
        async with self.session.delete(url) as response:
            if response.status not in [200, 404]:  # 404 is OK if already deleted
                raise Exception(f"Failed to delete collection: {response.status}")