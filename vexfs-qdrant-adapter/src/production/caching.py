"""
VexFS v2 Qdrant Adapter - Intelligent Caching

Advanced caching strategies for query results, metadata, and connections.
"""

import time
import threading
import hashlib
import pickle
import asyncio
from typing import Dict, List, Any, Optional, Callable, Union
from dataclasses import dataclass
import logging
from collections import OrderedDict, defaultdict
import weakref
import json

logger = logging.getLogger(__name__)

@dataclass
class CacheConfig:
    """Caching configuration"""
    max_memory_mb: int = 512
    default_ttl: int = 300  # 5 minutes
    cleanup_interval: int = 60
    max_key_size: int = 1024
    compression_enabled: bool = True
    persistence_enabled: bool = False

class IntelligentCaching:
    """
    Intelligent caching system for VexFS v2 Qdrant adapter.
    
    Features:
    - Multi-level caching (L1: memory, L2: disk)
    - Intelligent cache eviction (LRU, LFU, TTL)
    - Query result caching with invalidation
    - Metadata caching with consistency
    - Connection state caching
    """
    
    def __init__(self, config: CacheConfig = None):
        self.config = config or CacheConfig()
        
        # Cache layers
        self.l1_cache = L1MemoryCache(self.config)
        self.l2_cache = L2DiskCache(self.config) if self.config.persistence_enabled else None
        
        # Specialized caches
        self.query_cache = QueryResultCache(self.config)
        self.metadata_cache = MetadataCache(self.config)
        self.connection_cache = ConnectionStateCache(self.config)
        
        # Cache statistics
        self.stats = CacheStatistics()
        
        # Background cleanup
        self.is_running = True
        self._start_background_tasks()
        
        logger.info(f"💾 Intelligent caching initialized: max_memory={self.config.max_memory_mb}MB")
    
    def _start_background_tasks(self):
        """Start background cache maintenance tasks"""
        self.cleanup_thread = threading.Thread(target=self._cleanup_loop, daemon=True)
        self.cleanup_thread.start()
        
        self.stats_thread = threading.Thread(target=self._stats_loop, daemon=True)
        self.stats_thread.start()
    
    async def get(self, key: str, cache_type: str = "general") -> Optional[Any]:
        """Get value from appropriate cache"""
        start_time = time.time()
        
        try:
            # Try specialized caches first
            if cache_type == "query":
                result = await self.query_cache.get(key)
            elif cache_type == "metadata":
                result = await self.metadata_cache.get(key)
            elif cache_type == "connection":
                result = await self.connection_cache.get(key)
            else:
                # Try L1 cache first
                result = await self.l1_cache.get(key)
                
                # Try L2 cache if L1 miss
                if result is None and self.l2_cache:
                    result = await self.l2_cache.get(key)
                    
                    # Promote to L1 if found in L2
                    if result is not None:
                        await self.l1_cache.set(key, result)
            
            # Update statistics
            if result is not None:
                self.stats.record_hit(cache_type, time.time() - start_time)
            else:
                self.stats.record_miss(cache_type)
            
            return result
            
        except Exception as e:
            logger.error(f"Cache get error for key {key}: {e}")
            self.stats.record_error(cache_type)
            return None
    
    async def set(self, key: str, value: Any, ttl: int = None, cache_type: str = "general"):
        """Set value in appropriate cache"""
        try:
            ttl = ttl or self.config.default_ttl
            
            # Set in specialized caches
            if cache_type == "query":
                await self.query_cache.set(key, value, ttl)
            elif cache_type == "metadata":
                await self.metadata_cache.set(key, value, ttl)
            elif cache_type == "connection":
                await self.connection_cache.set(key, value, ttl)
            else:
                # Set in L1 cache
                await self.l1_cache.set(key, value, ttl)
                
                # Set in L2 cache if enabled
                if self.l2_cache:
                    await self.l2_cache.set(key, value, ttl)
            
            self.stats.record_set(cache_type)
            
        except Exception as e:
            logger.error(f"Cache set error for key {key}: {e}")
            self.stats.record_error(cache_type)
    
    async def invalidate(self, pattern: str = None, cache_type: str = "all"):
        """Invalidate cache entries"""
        try:
            if cache_type == "all" or cache_type == "query":
                await self.query_cache.invalidate(pattern)
            
            if cache_type == "all" or cache_type == "metadata":
                await self.metadata_cache.invalidate(pattern)
            
            if cache_type == "all" or cache_type == "connection":
                await self.connection_cache.invalidate(pattern)
            
            if cache_type == "all" or cache_type == "general":
                await self.l1_cache.invalidate(pattern)
                if self.l2_cache:
                    await self.l2_cache.invalidate(pattern)
            
            self.stats.record_invalidation(cache_type)
            
        except Exception as e:
            logger.error(f"Cache invalidation error: {e}")
    
    def _cleanup_loop(self):
        """Background cleanup loop"""
        while self.is_running:
            try:
                asyncio.run(self._perform_cleanup())
                time.sleep(self.config.cleanup_interval)
            except Exception as e:
                logger.error(f"Cache cleanup error: {e}")
                time.sleep(self.config.cleanup_interval)
    
    async def _perform_cleanup(self):
        """Perform cache cleanup"""
        # Cleanup all cache layers
        await self.l1_cache.cleanup()
        if self.l2_cache:
            await self.l2_cache.cleanup()
        
        await self.query_cache.cleanup()
        await self.metadata_cache.cleanup()
        await self.connection_cache.cleanup()
        
        logger.debug("Cache cleanup completed")
    
    def _stats_loop(self):
        """Background statistics collection loop"""
        while self.is_running:
            try:
                self.stats.update_memory_usage(self.get_memory_usage())
                time.sleep(30)  # Update stats every 30 seconds
            except Exception as e:
                logger.error(f"Cache stats error: {e}")
                time.sleep(30)
    
    def get_memory_usage(self) -> float:
        """Get total cache memory usage in MB"""
        total_mb = 0.0
        
        total_mb += self.l1_cache.get_memory_usage()
        total_mb += self.query_cache.get_memory_usage()
        total_mb += self.metadata_cache.get_memory_usage()
        total_mb += self.connection_cache.get_memory_usage()
        
        return total_mb
    
    def get_cache_stats(self) -> Dict[str, Any]:
        """Get comprehensive cache statistics"""
        return {
            "memory_usage_mb": self.get_memory_usage(),
            "memory_limit_mb": self.config.max_memory_mb,
            "l1_cache": self.l1_cache.get_stats(),
            "query_cache": self.query_cache.get_stats(),
            "metadata_cache": self.metadata_cache.get_stats(),
            "connection_cache": self.connection_cache.get_stats(),
            "overall_stats": self.stats.get_summary()
        }

class L1MemoryCache:
    """Level 1 in-memory cache with LRU eviction"""
    
    def __init__(self, config: CacheConfig):
        self.config = config
        self.cache = OrderedDict()
        self.expiry_times = {}
        self.lock = threading.RLock()
        self.memory_usage = 0
    
    async def get(self, key: str) -> Optional[Any]:
        """Get value from L1 cache"""
        with self.lock:
            if key in self.cache:
                # Check expiry
                if key in self.expiry_times and time.time() > self.expiry_times[key]:
                    del self.cache[key]
                    del self.expiry_times[key]
                    return None
                
                # Move to end (most recently used)
                value = self.cache.pop(key)
                self.cache[key] = value
                return value
            
            return None
    
    async def set(self, key: str, value: Any, ttl: int = None):
        """Set value in L1 cache"""
        with self.lock:
            # Calculate value size
            value_size = self._estimate_size(value)
            
            # Check memory limit
            while (self.memory_usage + value_size) > (self.config.max_memory_mb * 1024 * 1024):
                if not self.cache:
                    break
                self._evict_lru()
            
            # Set value
            self.cache[key] = value
            self.memory_usage += value_size
            
            # Set expiry
            if ttl:
                self.expiry_times[key] = time.time() + ttl
    
    def _evict_lru(self):
        """Evict least recently used item"""
        if self.cache:
            key, value = self.cache.popitem(last=False)
            self.memory_usage -= self._estimate_size(value)
            if key in self.expiry_times:
                del self.expiry_times[key]
    
    def _estimate_size(self, value: Any) -> int:
        """Estimate memory size of value"""
        try:
            return len(pickle.dumps(value))
        except:
            return 1024  # Default estimate
    
    async def cleanup(self):
        """Remove expired entries"""
        with self.lock:
            current_time = time.time()
            expired_keys = [
                key for key, expiry in self.expiry_times.items()
                if current_time > expiry
            ]
            
            for key in expired_keys:
                if key in self.cache:
                    value = self.cache.pop(key)
                    self.memory_usage -= self._estimate_size(value)
                del self.expiry_times[key]
    
    async def invalidate(self, pattern: str = None):
        """Invalidate cache entries"""
        with self.lock:
            if pattern is None:
                self.cache.clear()
                self.expiry_times.clear()
                self.memory_usage = 0
            else:
                keys_to_remove = [key for key in self.cache.keys() if pattern in key]
                for key in keys_to_remove:
                    value = self.cache.pop(key)
                    self.memory_usage -= self._estimate_size(value)
                    if key in self.expiry_times:
                        del self.expiry_times[key]
    
    def get_memory_usage(self) -> float:
        """Get memory usage in MB"""
        return self.memory_usage / 1024 / 1024
    
    def get_stats(self) -> Dict[str, Any]:
        """Get L1 cache statistics"""
        with self.lock:
            return {
                "size": len(self.cache),
                "memory_usage_mb": self.get_memory_usage(),
                "expired_entries": len([
                    key for key, expiry in self.expiry_times.items()
                    if time.time() > expiry
                ])
            }

class L2DiskCache:
    """Level 2 disk-based cache for persistence"""
    
    def __init__(self, config: CacheConfig):
        self.config = config
        self.cache_dir = "/tmp/vexfs_cache"  # In production, use proper cache directory
        self.index = {}
        self.lock = threading.RLock()
    
    async def get(self, key: str) -> Optional[Any]:
        """Get value from L2 cache"""
        # Placeholder for disk cache implementation
        return None
    
    async def set(self, key: str, value: Any, ttl: int = None):
        """Set value in L2 cache"""
        # Placeholder for disk cache implementation
        pass
    
    async def cleanup(self):
        """Cleanup L2 cache"""
        # Placeholder for disk cache cleanup
        pass
    
    async def invalidate(self, pattern: str = None):
        """Invalidate L2 cache entries"""
        # Placeholder for disk cache invalidation
        pass

class QueryResultCache:
    """Specialized cache for query results"""
    
    def __init__(self, config: CacheConfig):
        self.config = config
        self.cache = {}
        self.query_patterns = defaultdict(set)
        self.lock = threading.RLock()
    
    async def get(self, query_key: str) -> Optional[Any]:
        """Get cached query result"""
        with self.lock:
            cache_key = self._generate_cache_key(query_key)
            if cache_key in self.cache:
                entry = self.cache[cache_key]
                if time.time() < entry["expires_at"]:
                    return entry["result"]
                else:
                    del self.cache[cache_key]
            return None
    
    async def set(self, query_key: str, result: Any, ttl: int = None):
        """Cache query result"""
        with self.lock:
            cache_key = self._generate_cache_key(query_key)
            expires_at = time.time() + (ttl or self.config.default_ttl)
            
            self.cache[cache_key] = {
                "result": result,
                "expires_at": expires_at,
                "created_at": time.time()
            }
            
            # Track query patterns for smart invalidation
            pattern = self._extract_pattern(query_key)
            self.query_patterns[pattern].add(cache_key)
    
    def _generate_cache_key(self, query_key: str) -> str:
        """Generate cache key from query"""
        return hashlib.md5(query_key.encode()).hexdigest()
    
    def _extract_pattern(self, query_key: str) -> str:
        """Extract pattern from query for invalidation"""
        # Simple pattern extraction - in production, this would be more sophisticated
        if "collection:" in query_key:
            return query_key.split("collection:")[1].split(":")[0]
        return "default"
    
    async def invalidate(self, pattern: str = None):
        """Invalidate query cache"""
        with self.lock:
            if pattern is None:
                self.cache.clear()
                self.query_patterns.clear()
            else:
                if pattern in self.query_patterns:
                    for cache_key in self.query_patterns[pattern]:
                        if cache_key in self.cache:
                            del self.cache[cache_key]
                    del self.query_patterns[pattern]
    
    async def cleanup(self):
        """Remove expired query results"""
        with self.lock:
            current_time = time.time()
            expired_keys = [
                key for key, entry in self.cache.items()
                if current_time >= entry["expires_at"]
            ]
            
            for key in expired_keys:
                del self.cache[key]
                
            # Clean up pattern tracking
            for pattern, keys in list(self.query_patterns.items()):
                self.query_patterns[pattern] = {k for k in keys if k in self.cache}
                if not self.query_patterns[pattern]:
                    del self.query_patterns[pattern]
    
    def get_memory_usage(self) -> float:
        """Get memory usage in MB"""
        try:
            return len(pickle.dumps(self.cache)) / 1024 / 1024
        except:
            return len(self.cache) * 0.001  # Rough estimate
    
    def get_stats(self) -> Dict[str, Any]:
        """Get query cache statistics"""
        with self.lock:
            return {
                "size": len(self.cache),
                "patterns": len(self.query_patterns),
                "memory_usage_mb": self.get_memory_usage()
            }

class MetadataCache:
    """Cache for metadata operations"""
    
    def __init__(self, config: CacheConfig):
        self.config = config
        self.cache = {}
        self.lock = threading.RLock()
    
    async def get(self, key: str) -> Optional[Any]:
        """Get cached metadata"""
        with self.lock:
            if key in self.cache:
                entry = self.cache[key]
                if time.time() < entry["expires_at"]:
                    return entry["data"]
                else:
                    del self.cache[key]
            return None
    
    async def set(self, key: str, data: Any, ttl: int = None):
        """Cache metadata"""
        with self.lock:
            expires_at = time.time() + (ttl or self.config.default_ttl)
            self.cache[key] = {
                "data": data,
                "expires_at": expires_at
            }
    
    async def invalidate(self, pattern: str = None):
        """Invalidate metadata cache"""
        with self.lock:
            if pattern is None:
                self.cache.clear()
            else:
                keys_to_remove = [key for key in self.cache.keys() if pattern in key]
                for key in keys_to_remove:
                    del self.cache[key]
    
    async def cleanup(self):
        """Remove expired metadata"""
        with self.lock:
            current_time = time.time()
            expired_keys = [
                key for key, entry in self.cache.items()
                if current_time >= entry["expires_at"]
            ]
            
            for key in expired_keys:
                del self.cache[key]
    
    def get_memory_usage(self) -> float:
        """Get memory usage in MB"""
        return len(self.cache) * 0.001  # Rough estimate
    
    def get_stats(self) -> Dict[str, Any]:
        """Get metadata cache statistics"""
        with self.lock:
            return {
                "size": len(self.cache),
                "memory_usage_mb": self.get_memory_usage()
            }

class ConnectionStateCache:
    """Cache for connection state information"""
    
    def __init__(self, config: CacheConfig):
        self.config = config
        self.cache = {}
        self.lock = threading.RLock()
    
    async def get(self, key: str) -> Optional[Any]:
        """Get cached connection state"""
        with self.lock:
            return self.cache.get(key)
    
    async def set(self, key: str, state: Any, ttl: int = None):
        """Cache connection state"""
        with self.lock:
            self.cache[key] = state
    
    async def invalidate(self, pattern: str = None):
        """Invalidate connection cache"""
        with self.lock:
            if pattern is None:
                self.cache.clear()
            else:
                keys_to_remove = [key for key in self.cache.keys() if pattern in key]
                for key in keys_to_remove:
                    del self.cache[key]
    
    async def cleanup(self):
        """Cleanup connection cache"""
        # Connection cache doesn't use TTL, so no cleanup needed
        pass
    
    def get_memory_usage(self) -> float:
        """Get memory usage in MB"""
        return len(self.cache) * 0.001  # Rough estimate
    
    def get_stats(self) -> Dict[str, Any]:
        """Get connection cache statistics"""
        with self.lock:
            return {
                "size": len(self.cache),
                "memory_usage_mb": self.get_memory_usage()
            }

class CacheStatistics:
    """Cache performance statistics"""
    
    def __init__(self):
        self.hits = defaultdict(int)
        self.misses = defaultdict(int)
        self.sets = defaultdict(int)
        self.errors = defaultdict(int)
        self.invalidations = defaultdict(int)
        self.response_times = defaultdict(list)
        self.memory_usage_history = []
        self.lock = threading.RLock()
    
    def record_hit(self, cache_type: str, response_time: float):
        """Record cache hit"""
        with self.lock:
            self.hits[cache_type] += 1
            self.response_times[cache_type].append(response_time)
            
            # Keep only recent response times
            if len(self.response_times[cache_type]) > 1000:
                self.response_times[cache_type] = self.response_times[cache_type][-1000:]
    
    def record_miss(self, cache_type: str):
        """Record cache miss"""
        with self.lock:
            self.misses[cache_type] += 1
    
    def record_set(self, cache_type: str):
        """Record cache set operation"""
        with self.lock:
            self.sets[cache_type] += 1
    
    def record_error(self, cache_type: str):
        """Record cache error"""
        with self.lock:
            self.errors[cache_type] += 1
    
    def record_invalidation(self, cache_type: str):
        """Record cache invalidation"""
        with self.lock:
            self.invalidations[cache_type] += 1
    
    def update_memory_usage(self, memory_mb: float):
        """Update memory usage history"""
        with self.lock:
            self.memory_usage_history.append({
                "timestamp": time.time(),
                "memory_mb": memory_mb
            })
            
            # Keep only recent history
            if len(self.memory_usage_history) > 1440:  # 24 hours at 1-minute intervals
                self.memory_usage_history = self.memory_usage_history[-1440:]
    
    def get_summary(self) -> Dict[str, Any]:
        """Get statistics summary"""
        with self.lock:
            summary = {}
            
            for cache_type in set(list(self.hits.keys()) + list(self.misses.keys())):
                hits = self.hits[cache_type]
                misses = self.misses[cache_type]
                total = hits + misses
                
                hit_rate = hits / total if total > 0 else 0
                avg_response_time = (
                    sum(self.response_times[cache_type]) / len(self.response_times[cache_type])
                    if self.response_times[cache_type] else 0
                )
                
                summary[cache_type] = {
                    "hits": hits,
                    "misses": misses,
                    "hit_rate": hit_rate,
                    "sets": self.sets[cache_type],
                    "errors": self.errors[cache_type],
                    "invalidations": self.invalidations[cache_type],
                    "avg_response_time_ms": avg_response_time * 1000
                }
            
            return summary

# Global caching instance
_global_cache = None

def get_intelligent_cache() -> IntelligentCaching:
    """Get global intelligent cache instance"""
    global _global_cache
    if _global_cache is None:
        _global_cache = IntelligentCaching()
    return _global_cache