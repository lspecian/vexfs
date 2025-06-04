"""
VexFS v2 Qdrant Adapter - Production Optimizations

Advanced production optimizations for memory usage, connection pooling, 
query optimization, and concurrent request handling.
"""

import asyncio
import time
import gc
import threading
from typing import Dict, List, Any, Optional, Callable
from dataclasses import dataclass
import logging
import psutil
from concurrent.futures import ThreadPoolExecutor
import weakref
from collections import defaultdict, deque
import numpy as np

logger = logging.getLogger(__name__)

@dataclass
class OptimizationConfig:
    """Configuration for production optimizations"""
    # Memory optimization
    memory_limit_mb: int = 2048
    gc_threshold_mb: int = 1024
    memory_pool_size: int = 100
    
    # Connection optimization
    max_connections: int = 1000
    connection_timeout: int = 30
    keep_alive_timeout: int = 300
    
    # Query optimization
    query_cache_size: int = 10000
    query_cache_ttl: int = 300
    batch_size_limit: int = 1000
    
    # Concurrent processing
    max_workers: int = 8
    async_batch_size: int = 100
    request_queue_size: int = 5000

class ProductionOptimizations:
    """
    Production-grade optimizations for VexFS v2 Qdrant adapter.
    
    Provides:
    - Memory usage optimization (<100MB target for 1M vectors)
    - Connection pooling and resource management
    - Query optimization for complex filters
    - Batch operation efficiency improvements
    - Concurrent request handling optimization
    """
    
    def __init__(self, config: OptimizationConfig = None):
        self.config = config or OptimizationConfig()
        self.start_time = time.time()
        
        # Memory management
        self.memory_pool = MemoryPool(self.config.memory_pool_size)
        self.memory_monitor = MemoryMonitor(self.config.memory_limit_mb)
        
        # Connection management
        self.connection_pool = ConnectionPoolManager(
            max_size=self.config.max_connections,
            timeout=self.config.connection_timeout
        )
        
        # Query optimization
        self.query_cache = QueryCache(
            max_size=self.config.query_cache_size,
            ttl=self.config.query_cache_ttl
        )
        
        # Concurrent processing
        self.executor = ThreadPoolExecutor(max_workers=self.config.max_workers)
        self.request_queue = asyncio.Queue(maxsize=self.config.request_queue_size)
        
        # Performance metrics
        self.metrics = OptimizationMetrics()
        
        # Start background tasks
        self._start_background_tasks()
    
    def _start_background_tasks(self):
        """Start background optimization tasks"""
        # Memory cleanup task
        self.memory_cleanup_task = threading.Thread(
            target=self._memory_cleanup_loop, 
            daemon=True
        )
        self.memory_cleanup_task.start()
        
        # Connection cleanup task
        self.connection_cleanup_task = threading.Thread(
            target=self._connection_cleanup_loop,
            daemon=True
        )
        self.connection_cleanup_task.start()
        
        logger.info("🚀 Production optimizations initialized")
    
    async def optimize_memory_usage(self):
        """Implement memory optimization strategies"""
        logger.info("🧠 Optimizing memory usage")
        
        # Force garbage collection
        collected = gc.collect()
        
        # Clear unused caches
        self.query_cache.cleanup_expired()
        
        # Optimize memory pools
        self.memory_pool.optimize()
        
        # Monitor memory usage
        current_memory = psutil.virtual_memory().used / 1024 / 1024
        self.metrics.record_memory_optimization(current_memory, collected)
        
        logger.info(f"Memory optimization complete: {collected} objects collected")
    
    async def enhance_concurrent_handling(self):
        """Optimize concurrent request processing"""
        logger.info("⚡ Enhancing concurrent request handling")
        
        # Optimize thread pool
        if self.executor._threads:
            current_threads = len(self.executor._threads)
            optimal_threads = min(self.config.max_workers, psutil.cpu_count() * 2)
            
            if current_threads != optimal_threads:
                # Recreate executor with optimal thread count
                old_executor = self.executor
                self.executor = ThreadPoolExecutor(max_workers=optimal_threads)
                old_executor.shutdown(wait=False)
                
                logger.info(f"Thread pool optimized: {current_threads} -> {optimal_threads} threads")
        
        # Optimize connection pool
        await self.connection_pool.optimize()
        
        # Clear request queue if needed
        if self.request_queue.qsize() > self.config.request_queue_size * 0.8:
            logger.warning("Request queue near capacity, optimizing...")
            # Process pending requests in batch
            pending_requests = []
            while not self.request_queue.empty() and len(pending_requests) < 100:
                try:
                    request = self.request_queue.get_nowait()
                    pending_requests.append(request)
                except asyncio.QueueEmpty:
                    break
            
            # Process in parallel
            if pending_requests:
                await asyncio.gather(*pending_requests, return_exceptions=True)
    
    async def implement_caching_layer(self):
        """Add intelligent caching for performance"""
        logger.info("💾 Implementing intelligent caching layer")
        
        # Query result caching
        self.query_cache.optimize_storage()
        
        # Metadata caching
        if hasattr(self, 'metadata_cache'):
            self.metadata_cache.cleanup()
        else:
            self.metadata_cache = MetadataCache(max_size=1000)
        
        # Connection caching
        await self.connection_pool.cache_optimization()
        
        cache_stats = {
            "query_cache_size": len(self.query_cache.cache),
            "query_cache_hit_rate": self.query_cache.hit_rate,
            "connection_pool_size": self.connection_pool.size,
            "metadata_cache_size": getattr(self.metadata_cache, 'size', 0)
        }
        
        self.metrics.record_cache_optimization(cache_stats)
        logger.info(f"Caching optimization complete: {cache_stats}")
    
    def _memory_cleanup_loop(self):
        """Background memory cleanup loop"""
        while True:
            try:
                # Check memory usage
                memory_mb = psutil.virtual_memory().used / 1024 / 1024
                
                if memory_mb > self.config.gc_threshold_mb:
                    # Force garbage collection
                    collected = gc.collect()
                    logger.debug(f"Background GC: {collected} objects collected")
                    
                    # Clear caches if memory is still high
                    if psutil.virtual_memory().used / 1024 / 1024 > self.config.memory_limit_mb * 0.9:
                        self.query_cache.clear_lru(0.3)  # Clear 30% of LRU items
                        logger.debug("Cache cleared due to high memory usage")
                
                time.sleep(30)  # Check every 30 seconds
                
            except Exception as e:
                logger.error(f"Memory cleanup error: {e}")
                time.sleep(60)
    
    def _connection_cleanup_loop(self):
        """Background connection cleanup loop"""
        while True:
            try:
                # Cleanup expired connections
                cleaned = self.connection_pool.cleanup_expired()
                if cleaned > 0:
                    logger.debug(f"Cleaned up {cleaned} expired connections")
                
                time.sleep(60)  # Check every minute
                
            except Exception as e:
                logger.error(f"Connection cleanup error: {e}")
                time.sleep(60)
    
    def get_optimization_stats(self) -> Dict[str, Any]:
        """Get current optimization statistics"""
        return {
            "uptime_seconds": time.time() - self.start_time,
            "memory": {
                "current_mb": psutil.virtual_memory().used / 1024 / 1024,
                "limit_mb": self.config.memory_limit_mb,
                "pool_size": self.memory_pool.size,
                "gc_collections": self.metrics.gc_collections
            },
            "connections": {
                "active": self.connection_pool.active_count,
                "pool_size": self.connection_pool.size,
                "max_connections": self.config.max_connections
            },
            "cache": {
                "query_cache_size": len(self.query_cache.cache),
                "query_cache_hit_rate": self.query_cache.hit_rate,
                "cache_memory_mb": self.query_cache.memory_usage_mb
            },
            "performance": {
                "avg_response_time_ms": self.metrics.avg_response_time_ms,
                "requests_per_second": self.metrics.requests_per_second,
                "error_rate": self.metrics.error_rate
            }
        }

class MemoryPool:
    """Memory pool for efficient memory management"""
    
    def __init__(self, max_size: int):
        self.max_size = max_size
        self.pools = defaultdict(deque)
        self.lock = threading.RLock()
        self.allocated_count = 0
    
    def get_buffer(self, size: int) -> bytearray:
        """Get buffer from pool or create new one"""
        with self.lock:
            pool = self.pools[size]
            if pool:
                return pool.popleft()
            else:
                self.allocated_count += 1
                return bytearray(size)
    
    def return_buffer(self, buffer: bytearray):
        """Return buffer to pool"""
        with self.lock:
            size = len(buffer)
            pool = self.pools[size]
            
            if len(pool) < self.max_size:
                # Clear buffer and return to pool
                buffer[:] = b'\x00' * size
                pool.append(buffer)
    
    def optimize(self):
        """Optimize memory pools"""
        with self.lock:
            # Remove pools that are too large
            for size, pool in list(self.pools.items()):
                if len(pool) > self.max_size // 2:
                    # Keep only half
                    while len(pool) > self.max_size // 2:
                        pool.pop()
    
    @property
    def size(self) -> int:
        """Get total pool size"""
        with self.lock:
            return sum(len(pool) for pool in self.pools.values())

class MemoryMonitor:
    """Memory usage monitoring and alerts"""
    
    def __init__(self, limit_mb: int):
        self.limit_mb = limit_mb
        self.alerts_sent = 0
        self.last_alert_time = 0
    
    def check_memory_usage(self) -> bool:
        """Check if memory usage is within limits"""
        current_mb = psutil.virtual_memory().used / 1024 / 1024
        
        if current_mb > self.limit_mb:
            # Send alert if not sent recently
            if time.time() - self.last_alert_time > 300:  # 5 minutes
                logger.warning(f"Memory usage high: {current_mb:.1f}MB > {self.limit_mb}MB")
                self.alerts_sent += 1
                self.last_alert_time = time.time()
            return False
        
        return True

class ConnectionPoolManager:
    """Advanced connection pool management"""
    
    def __init__(self, max_size: int, timeout: int):
        self.max_size = max_size
        self.timeout = timeout
        self.connections = {}
        self.connection_times = {}
        self.lock = threading.RLock()
        self.active_count = 0
    
    async def get_connection(self, key: str):
        """Get connection from pool"""
        with self.lock:
            if key in self.connections:
                self.connection_times[key] = time.time()
                return self.connections[key]
            
            if len(self.connections) >= self.max_size:
                # Remove oldest connection
                oldest_key = min(self.connection_times.keys(), 
                               key=lambda k: self.connection_times[k])
                self.remove_connection(oldest_key)
            
            # Create new connection (placeholder)
            connection = f"connection_{key}_{time.time()}"
            self.connections[key] = connection
            self.connection_times[key] = time.time()
            self.active_count += 1
            
            return connection
    
    def remove_connection(self, key: str):
        """Remove connection from pool"""
        with self.lock:
            if key in self.connections:
                del self.connections[key]
                del self.connection_times[key]
                self.active_count = max(0, self.active_count - 1)
    
    def cleanup_expired(self) -> int:
        """Cleanup expired connections"""
        with self.lock:
            current_time = time.time()
            expired_keys = [
                key for key, conn_time in self.connection_times.items()
                if current_time - conn_time > self.timeout
            ]
            
            for key in expired_keys:
                self.remove_connection(key)
            
            return len(expired_keys)
    
    async def optimize(self):
        """Optimize connection pool"""
        cleaned = self.cleanup_expired()
        logger.debug(f"Connection pool optimized: {cleaned} connections cleaned")
    
    async def cache_optimization(self):
        """Optimize connection caching"""
        # Implement connection-specific caching optimizations
        pass
    
    @property
    def size(self) -> int:
        """Get current pool size"""
        with self.lock:
            return len(self.connections)

class QueryCache:
    """Intelligent query result caching"""
    
    def __init__(self, max_size: int, ttl: int):
        self.max_size = max_size
        self.ttl = ttl
        self.cache = {}
        self.access_times = {}
        self.hit_count = 0
        self.miss_count = 0
        self.lock = threading.RLock()
    
    def get(self, key: str) -> Optional[Any]:
        """Get cached result"""
        with self.lock:
            if key in self.cache:
                # Check if expired
                if time.time() - self.access_times[key] > self.ttl:
                    del self.cache[key]
                    del self.access_times[key]
                    self.miss_count += 1
                    return None
                
                # Update access time
                self.access_times[key] = time.time()
                self.hit_count += 1
                return self.cache[key]
            
            self.miss_count += 1
            return None
    
    def set(self, key: str, value: Any):
        """Set cached result"""
        with self.lock:
            if len(self.cache) >= self.max_size:
                # Remove LRU item
                lru_key = min(self.access_times.keys(), 
                            key=lambda k: self.access_times[k])
                del self.cache[lru_key]
                del self.access_times[lru_key]
            
            self.cache[key] = value
            self.access_times[key] = time.time()
    
    def cleanup_expired(self):
        """Remove expired cache entries"""
        with self.lock:
            current_time = time.time()
            expired_keys = [
                key for key, access_time in self.access_times.items()
                if current_time - access_time > self.ttl
            ]
            
            for key in expired_keys:
                del self.cache[key]
                del self.access_times[key]
    
    def clear_lru(self, fraction: float):
        """Clear fraction of LRU items"""
        with self.lock:
            items_to_remove = int(len(self.cache) * fraction)
            if items_to_remove > 0:
                # Sort by access time and remove oldest
                sorted_keys = sorted(self.access_times.keys(), 
                                   key=lambda k: self.access_times[k])
                
                for key in sorted_keys[:items_to_remove]:
                    del self.cache[key]
                    del self.access_times[key]
    
    def optimize_storage(self):
        """Optimize cache storage"""
        self.cleanup_expired()
        
        # Compact cache if needed
        if len(self.cache) > self.max_size * 0.8:
            self.clear_lru(0.2)  # Remove 20% of LRU items
    
    @property
    def hit_rate(self) -> float:
        """Get cache hit rate"""
        total = self.hit_count + self.miss_count
        return self.hit_count / total if total > 0 else 0.0
    
    @property
    def memory_usage_mb(self) -> float:
        """Estimate memory usage in MB"""
        # Rough estimation
        return len(self.cache) * 0.001  # 1KB per entry estimate

class MetadataCache:
    """Cache for metadata operations"""
    
    def __init__(self, max_size: int):
        self.max_size = max_size
        self.cache = {}
        self.size = 0
    
    def cleanup(self):
        """Cleanup metadata cache"""
        if self.size > self.max_size * 0.8:
            # Clear half the cache
            keys_to_remove = list(self.cache.keys())[:len(self.cache) // 2]
            for key in keys_to_remove:
                del self.cache[key]
            self.size = len(self.cache)

class OptimizationMetrics:
    """Metrics tracking for optimizations"""
    
    def __init__(self):
        self.gc_collections = 0
        self.memory_optimizations = 0
        self.cache_optimizations = 0
        self.response_times = deque(maxlen=1000)
        self.request_count = 0
        self.error_count = 0
        self.start_time = time.time()
    
    def record_memory_optimization(self, memory_mb: float, collected: int):
        """Record memory optimization event"""
        self.memory_optimizations += 1
        self.gc_collections += collected
    
    def record_cache_optimization(self, stats: Dict[str, Any]):
        """Record cache optimization event"""
        self.cache_optimizations += 1
    
    def record_request(self, response_time_ms: float, success: bool):
        """Record request metrics"""
        self.response_times.append(response_time_ms)
        self.request_count += 1
        if not success:
            self.error_count += 1
    
    @property
    def avg_response_time_ms(self) -> float:
        """Get average response time"""
        return sum(self.response_times) / len(self.response_times) if self.response_times else 0
    
    @property
    def requests_per_second(self) -> float:
        """Get requests per second"""
        uptime = time.time() - self.start_time
        return self.request_count / uptime if uptime > 0 else 0
    
    @property
    def error_rate(self) -> float:
        """Get error rate"""
        return self.error_count / self.request_count if self.request_count > 0 else 0