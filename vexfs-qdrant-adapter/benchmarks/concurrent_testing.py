"""
VexFS v2 Qdrant Adapter - Concurrent Testing Module

Advanced concurrent request handling and connection testing for production deployment.
"""

import asyncio
import aiohttp
import time
import random
import statistics
from typing import Dict, List, Any, Optional, Tuple
from dataclasses import dataclass
import logging
from concurrent.futures import ThreadPoolExecutor, as_completed
import numpy as np
import threading
from collections import defaultdict

from .performance_suite import BenchmarkResult

logger = logging.getLogger(__name__)

@dataclass
class ConnectionMetrics:
    """Metrics for individual connection"""
    connection_id: int
    start_time: float
    end_time: float
    requests_sent: int
    requests_successful: int
    requests_failed: int
    avg_response_time_ms: float
    errors: List[str]

@dataclass
class ConcurrencyTestResult:
    """Results from concurrency testing"""
    max_concurrent_connections: int
    total_requests: int
    successful_requests: int
    failed_requests: int
    avg_response_time_ms: float
    p95_response_time_ms: float
    p99_response_time_ms: float
    throughput_rps: float
    connection_metrics: List[ConnectionMetrics]
    error_distribution: Dict[str, int]

class ConcurrentTester:
    """
    Advanced concurrent testing for VexFS v2 Qdrant adapter.
    
    Tests:
    - Maximum concurrent connections
    - Request throughput under load
    - Response time distribution
    - Error handling under stress
    - Connection pooling efficiency
    """
    
    def __init__(self, base_url: str = "http://localhost:6333"):
        self.base_url = base_url
        self.active_connections = 0
        self.connection_lock = threading.Lock()
        self.results_lock = threading.Lock()
        
    async def test_concurrent_connections(self, max_connections: int, duration_seconds: int) -> BenchmarkResult:
        """
        Test concurrent connection handling.
        
        Args:
            max_connections: Maximum number of concurrent connections to test
            duration_seconds: Duration to maintain connections
        """
        logger.info(f"🔗 Testing concurrent connections: {max_connections} connections for {duration_seconds}s")
        
        start_time = time.time()
        connection_metrics = []
        error_counts = defaultdict(int)
        response_times = []
        
        # Create test collection
        collection_name = f"concurrent_test_{int(time.time())}"
        await self._setup_test_collection(collection_name)
        
        try:
            # Launch concurrent connections
            tasks = []
            for conn_id in range(max_connections):
                task = asyncio.create_task(
                    self._run_connection_test(
                        conn_id, 
                        collection_name, 
                        duration_seconds
                    )
                )
                tasks.append(task)
            
            # Wait for all connections to complete
            results = await asyncio.gather(*tasks, return_exceptions=True)
            
            # Process results
            successful_connections = 0
            total_requests = 0
            successful_requests = 0
            failed_requests = 0
            
            for result in results:
                if isinstance(result, Exception):
                    error_counts[str(type(result).__name__)] += 1
                    continue
                
                if isinstance(result, ConnectionMetrics):
                    connection_metrics.append(result)
                    successful_connections += 1
                    total_requests += result.requests_sent
                    successful_requests += result.requests_successful
                    failed_requests += result.requests_failed
                    
                    # Collect response times (simulated for now)
                    if result.avg_response_time_ms > 0:
                        response_times.extend([result.avg_response_time_ms] * result.requests_successful)
                    
                    for error in result.errors:
                        error_counts[error] += 1
        
        finally:
            # Cleanup
            await self._cleanup_test_collection(collection_name)
        
        # Calculate metrics
        total_duration = time.time() - start_time
        throughput = successful_requests / total_duration if total_duration > 0 else 0
        
        avg_response_time = statistics.mean(response_times) if response_times else 0
        p95_response_time = np.percentile(response_times, 95) if response_times else 0
        p99_response_time = np.percentile(response_times, 99) if response_times else 0
        
        success_rate = successful_requests / total_requests if total_requests > 0 else 0
        
        return BenchmarkResult(
            name="Concurrent Connections Test",
            duration_ms=total_duration * 1000,
            throughput_ops_sec=throughput,
            memory_usage_mb=0,  # Will be measured separately
            cpu_usage_percent=0,
            success_rate=success_rate,
            error_count=failed_requests,
            metadata={
                "max_connections": max_connections,
                "successful_connections": successful_connections,
                "total_requests": total_requests,
                "successful_requests": successful_requests,
                "failed_requests": failed_requests,
                "avg_response_time_ms": avg_response_time,
                "p95_response_time_ms": p95_response_time,
                "p99_response_time_ms": p99_response_time,
                "error_distribution": dict(error_counts),
                "connection_success_rate": successful_connections / max_connections
            },
            timestamp=time.time()
        )
    
    async def _run_connection_test(self, conn_id: int, collection_name: str, duration_seconds: int) -> ConnectionMetrics:
        """Run test for individual connection"""
        start_time = time.time()
        end_time = start_time + duration_seconds
        
        requests_sent = 0
        requests_successful = 0
        requests_failed = 0
        response_times = []
        errors = []
        
        # Create session for this connection
        timeout = aiohttp.ClientTimeout(total=30)
        async with aiohttp.ClientSession(timeout=timeout) as session:
            
            with self.connection_lock:
                self.active_connections += 1
            
            try:
                while time.time() < end_time:
                    request_start = time.time()
                    
                    try:
                        # Mix of different operations
                        operation = random.choice(["search", "get", "scroll"])
                        
                        if operation == "search":
                            await self._perform_search(session, collection_name)
                        elif operation == "get":
                            await self._perform_get(session, collection_name)
                        elif operation == "scroll":
                            await self._perform_scroll(session, collection_name)
                        
                        request_time = (time.time() - request_start) * 1000
                        response_times.append(request_time)
                        requests_successful += 1
                        
                    except Exception as e:
                        errors.append(str(type(e).__name__))
                        requests_failed += 1
                    
                    requests_sent += 1
                    
                    # Small delay to prevent overwhelming
                    await asyncio.sleep(0.01)
            
            finally:
                with self.connection_lock:
                    self.active_connections -= 1
        
        avg_response_time = statistics.mean(response_times) if response_times else 0
        
        return ConnectionMetrics(
            connection_id=conn_id,
            start_time=start_time,
            end_time=time.time(),
            requests_sent=requests_sent,
            requests_successful=requests_successful,
            requests_failed=requests_failed,
            avg_response_time_ms=avg_response_time,
            errors=errors
        )
    
    async def test_connection_pooling(self, pool_sizes: List[int], requests_per_pool: int) -> BenchmarkResult:
        """
        Test connection pooling efficiency.
        
        Args:
            pool_sizes: List of pool sizes to test
            requests_per_pool: Number of requests to send per pool size
        """
        logger.info(f"🏊 Testing connection pooling: {pool_sizes} pool sizes, {requests_per_pool} requests each")
        
        start_time = time.time()
        results = {}
        
        collection_name = f"pool_test_{int(time.time())}"
        await self._setup_test_collection(collection_name)
        
        try:
            for pool_size in pool_sizes:
                pool_start = time.time()
                
                # Create connector with specific pool size
                connector = aiohttp.TCPConnector(limit=pool_size, limit_per_host=pool_size)
                timeout = aiohttp.ClientTimeout(total=30)
                
                async with aiohttp.ClientSession(connector=connector, timeout=timeout) as session:
                    # Send concurrent requests
                    tasks = []
                    for _ in range(requests_per_pool):
                        task = self._perform_search(session, collection_name)
                        tasks.append(task)
                    
                    # Execute all requests
                    pool_results = await asyncio.gather(*tasks, return_exceptions=True)
                    
                    successful = sum(1 for r in pool_results if not isinstance(r, Exception))
                    failed = len(pool_results) - successful
                    
                    pool_duration = time.time() - pool_start
                    pool_throughput = successful / pool_duration if pool_duration > 0 else 0
                    
                    results[pool_size] = {
                        "successful": successful,
                        "failed": failed,
                        "duration": pool_duration,
                        "throughput": pool_throughput
                    }
        
        finally:
            await self._cleanup_test_collection(collection_name)
        
        # Find optimal pool size
        optimal_pool_size = max(results.keys(), key=lambda k: results[k]["throughput"])
        total_duration = time.time() - start_time
        
        total_successful = sum(r["successful"] for r in results.values())
        total_requests = len(pool_sizes) * requests_per_pool
        
        return BenchmarkResult(
            name="Connection Pooling Test",
            duration_ms=total_duration * 1000,
            throughput_ops_sec=total_successful / total_duration if total_duration > 0 else 0,
            memory_usage_mb=0,
            cpu_usage_percent=0,
            success_rate=total_successful / total_requests if total_requests > 0 else 0,
            error_count=total_requests - total_successful,
            metadata={
                "pool_sizes_tested": pool_sizes,
                "requests_per_pool": requests_per_pool,
                "optimal_pool_size": optimal_pool_size,
                "pool_results": results,
                "total_requests": total_requests,
                "total_successful": total_successful
            },
            timestamp=time.time()
        )
    
    async def test_burst_traffic(self, burst_size: int, burst_interval: float, num_bursts: int) -> BenchmarkResult:
        """
        Test handling of burst traffic patterns.
        
        Args:
            burst_size: Number of requests per burst
            burst_interval: Time between bursts in seconds
            num_bursts: Number of bursts to send
        """
        logger.info(f"💥 Testing burst traffic: {burst_size} requests/burst, {burst_interval}s interval, {num_bursts} bursts")
        
        start_time = time.time()
        burst_results = []
        
        collection_name = f"burst_test_{int(time.time())}"
        await self._setup_test_collection(collection_name)
        
        try:
            for burst_num in range(num_bursts):
                burst_start = time.time()
                
                # Create burst of concurrent requests
                tasks = []
                for _ in range(burst_size):
                    task = self._perform_search_with_session(collection_name)
                    tasks.append(task)
                
                # Execute burst
                burst_responses = await asyncio.gather(*tasks, return_exceptions=True)
                
                burst_duration = time.time() - burst_start
                burst_successful = sum(1 for r in burst_responses if not isinstance(r, Exception))
                burst_failed = len(burst_responses) - burst_successful
                
                burst_results.append({
                    "burst_num": burst_num,
                    "duration": burst_duration,
                    "successful": burst_successful,
                    "failed": burst_failed,
                    "throughput": burst_successful / burst_duration if burst_duration > 0 else 0
                })
                
                # Wait for next burst
                if burst_num < num_bursts - 1:
                    await asyncio.sleep(burst_interval)
        
        finally:
            await self._cleanup_test_collection(collection_name)
        
        # Calculate overall metrics
        total_duration = time.time() - start_time
        total_successful = sum(b["successful"] for b in burst_results)
        total_requests = num_bursts * burst_size
        
        avg_burst_throughput = statistics.mean([b["throughput"] for b in burst_results])
        min_burst_throughput = min([b["throughput"] for b in burst_results])
        max_burst_throughput = max([b["throughput"] for b in burst_results])
        
        return BenchmarkResult(
            name="Burst Traffic Test",
            duration_ms=total_duration * 1000,
            throughput_ops_sec=total_successful / total_duration if total_duration > 0 else 0,
            memory_usage_mb=0,
            cpu_usage_percent=0,
            success_rate=total_successful / total_requests if total_requests > 0 else 0,
            error_count=total_requests - total_successful,
            metadata={
                "burst_size": burst_size,
                "burst_interval": burst_interval,
                "num_bursts": num_bursts,
                "avg_burst_throughput": avg_burst_throughput,
                "min_burst_throughput": min_burst_throughput,
                "max_burst_throughput": max_burst_throughput,
                "burst_results": burst_results,
                "total_requests": total_requests,
                "total_successful": total_successful
            },
            timestamp=time.time()
        )
    
    async def test_gradual_ramp_up(self, max_connections: int, ramp_duration: int, steady_duration: int) -> BenchmarkResult:
        """
        Test gradual connection ramp-up to maximum load.
        
        Args:
            max_connections: Maximum number of connections to reach
            ramp_duration: Time to ramp up to max connections (seconds)
            steady_duration: Time to maintain max connections (seconds)
        """
        logger.info(f"📈 Testing gradual ramp-up: {max_connections} max connections, {ramp_duration}s ramp, {steady_duration}s steady")
        
        start_time = time.time()
        connection_history = []
        active_tasks = []
        
        collection_name = f"ramp_test_{int(time.time())}"
        await self._setup_test_collection(collection_name)
        
        try:
            # Ramp-up phase
            ramp_start = time.time()
            connections_per_second = max_connections / ramp_duration
            
            while time.time() - ramp_start < ramp_duration:
                current_time = time.time() - ramp_start
                target_connections = int(current_time * connections_per_second)
                
                # Add connections if needed
                while len(active_tasks) < target_connections:
                    task = asyncio.create_task(
                        self._run_sustained_connection(collection_name, steady_duration + ramp_duration)
                    )
                    active_tasks.append(task)
                
                connection_history.append({
                    "timestamp": current_time,
                    "active_connections": len(active_tasks),
                    "target_connections": target_connections
                })
                
                await asyncio.sleep(1)  # Check every second
            
            # Steady state phase
            logger.info(f"Steady state: {len(active_tasks)} connections")
            steady_start = time.time()
            
            while time.time() - steady_start < steady_duration:
                connection_history.append({
                    "timestamp": time.time() - start_time,
                    "active_connections": len(active_tasks),
                    "target_connections": max_connections
                })
                await asyncio.sleep(1)
            
            # Wait for all connections to complete
            results = await asyncio.gather(*active_tasks, return_exceptions=True)
            
            # Process results
            successful_connections = sum(1 for r in results if not isinstance(r, Exception))
            total_requests = 0
            successful_requests = 0
            
            for result in results:
                if isinstance(result, dict):
                    total_requests += result.get("requests_sent", 0)
                    successful_requests += result.get("requests_successful", 0)
        
        finally:
            await self._cleanup_test_collection(collection_name)
        
        total_duration = time.time() - start_time
        
        return BenchmarkResult(
            name="Gradual Ramp-up Test",
            duration_ms=total_duration * 1000,
            throughput_ops_sec=successful_requests / total_duration if total_duration > 0 else 0,
            memory_usage_mb=0,
            cpu_usage_percent=0,
            success_rate=successful_requests / total_requests if total_requests > 0 else 0,
            error_count=total_requests - successful_requests,
            metadata={
                "max_connections": max_connections,
                "ramp_duration": ramp_duration,
                "steady_duration": steady_duration,
                "successful_connections": successful_connections,
                "total_requests": total_requests,
                "successful_requests": successful_requests,
                "connection_history": connection_history
            },
            timestamp=time.time()
        )
    
    # Helper methods
    async def _setup_test_collection(self, collection_name: str):
        """Setup test collection with sample data"""
        timeout = aiohttp.ClientTimeout(total=30)
        async with aiohttp.ClientSession(timeout=timeout) as session:
            # Create collection
            url = f"{self.base_url}/collections/{collection_name}"
            data = {
                "vectors": {
                    "size": 128,
                    "distance": "Cosine"
                }
            }
            async with session.put(url, json=data) as response:
                if response.status not in [200, 201]:
                    raise Exception(f"Failed to create collection: {response.status}")
            
            # Insert sample data
            points = []
            for i in range(100):
                points.append({
                    "id": i,
                    "vector": np.random.random(128).tolist(),
                    "payload": {"category": f"cat_{i % 5}", "value": i}
                })
            
            url = f"{self.base_url}/collections/{collection_name}/points"
            data = {"points": points}
            async with session.put(url, json=data) as response:
                if response.status not in [200, 201]:
                    raise Exception(f"Failed to insert points: {response.status}")
    
    async def _cleanup_test_collection(self, collection_name: str):
        """Cleanup test collection"""
        timeout = aiohttp.ClientTimeout(total=30)
        async with aiohttp.ClientSession(timeout=timeout) as session:
            url = f"{self.base_url}/collections/{collection_name}"
            async with session.delete(url) as response:
                pass  # Ignore errors during cleanup
    
    async def _perform_search(self, session: aiohttp.ClientSession, collection_name: str):
        """Perform search operation"""
        url = f"{self.base_url}/collections/{collection_name}/points/search"
        data = {
            "vector": np.random.random(128).tolist(),
            "limit": 10
        }
        async with session.post(url, json=data) as response:
            if response.status != 200:
                raise Exception(f"Search failed: {response.status}")
            return await response.json()
    
    async def _perform_get(self, session: aiohttp.ClientSession, collection_name: str):
        """Perform get operation"""
        point_id = random.randint(0, 99)
        url = f"{self.base_url}/collections/{collection_name}/points/{point_id}"
        async with session.get(url) as response:
            if response.status != 200:
                raise Exception(f"Get failed: {response.status}")
            return await response.json()
    
    async def _perform_scroll(self, session: aiohttp.ClientSession, collection_name: str):
        """Perform scroll operation"""
        url = f"{self.base_url}/collections/{collection_name}/points/scroll"
        data = {"limit": 10}
        async with session.post(url, json=data) as response:
            if response.status != 200:
                raise Exception(f"Scroll failed: {response.status}")
            return await response.json()
    
    async def _perform_search_with_session(self, collection_name: str):
        """Perform search with new session"""
        timeout = aiohttp.ClientTimeout(total=30)
        async with aiohttp.ClientSession(timeout=timeout) as session:
            return await self._perform_search(session, collection_name)
    
    async def _run_sustained_connection(self, collection_name: str, duration: int) -> Dict[str, int]:
        """Run sustained connection for specified duration"""
        start_time = time.time()
        end_time = start_time + duration
        
        requests_sent = 0
        requests_successful = 0
        
        timeout = aiohttp.ClientTimeout(total=30)
        async with aiohttp.ClientSession(timeout=timeout) as session:
            while time.time() < end_time:
                try:
                    await self._perform_search(session, collection_name)
                    requests_successful += 1
                except Exception:
                    pass
                
                requests_sent += 1
                await asyncio.sleep(0.1)  # 10 requests per second per connection
        
        return {
            "requests_sent": requests_sent,
            "requests_successful": requests_successful
        }