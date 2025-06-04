"""
VexFS v2 Qdrant Adapter - Connection Pool Management

Advanced connection pooling for production deployment.
"""

import asyncio
import time
import threading
from typing import Dict, List, Any, Optional, Callable
from dataclasses import dataclass
import logging
from collections import deque
import weakref

logger = logging.getLogger(__name__)

@dataclass
class ConnectionConfig:
    """Connection pool configuration"""
    max_size: int = 100
    min_size: int = 10
    timeout: int = 30
    keep_alive_timeout: int = 300
    max_idle_time: int = 600
    health_check_interval: int = 60

class ConnectionPoolManager:
    """
    Advanced connection pool management for VexFS v2 Qdrant adapter.
    
    Provides efficient connection pooling with:
    - Dynamic pool sizing
    - Connection health monitoring
    - Automatic cleanup
    - Load balancing
    """
    
    def __init__(self, config: ConnectionConfig = None):
        self.config = config or ConnectionConfig()
        self.connections = deque()
        self.active_connections = {}
        self.connection_stats = {}
        self.lock = threading.RLock()
        self.is_running = True
        
        # Start background tasks
        self._start_background_tasks()
        
        logger.info(f"🔗 Connection pool initialized: max_size={self.config.max_size}")
    
    def _start_background_tasks(self):
        """Start background maintenance tasks"""
        self.cleanup_thread = threading.Thread(target=self._cleanup_loop, daemon=True)
        self.cleanup_thread.start()
        
        self.health_check_thread = threading.Thread(target=self._health_check_loop, daemon=True)
        self.health_check_thread.start()
    
    async def get_connection(self, key: str = None) -> 'PooledConnection':
        """Get connection from pool"""
        with self.lock:
            # Try to get existing connection
            if self.connections:
                connection = self.connections.popleft()
                if self._is_connection_healthy(connection):
                    self.active_connections[connection.id] = connection
                    return connection
                else:
                    # Connection is unhealthy, create new one
                    await self._close_connection(connection)
            
            # Create new connection if pool not at max size
            if len(self.active_connections) < self.config.max_size:
                connection = await self._create_connection(key)
                self.active_connections[connection.id] = connection
                return connection
            
            # Pool is full, wait for available connection
            raise ConnectionPoolExhausted("Connection pool is full")
    
    async def return_connection(self, connection: 'PooledConnection'):
        """Return connection to pool"""
        with self.lock:
            if connection.id in self.active_connections:
                del self.active_connections[connection.id]
                
                if self._is_connection_healthy(connection) and len(self.connections) < self.config.max_size:
                    connection.last_used = time.time()
                    self.connections.append(connection)
                else:
                    await self._close_connection(connection)
    
    async def _create_connection(self, key: str = None) -> 'PooledConnection':
        """Create new pooled connection"""
        connection_id = f"conn_{int(time.time() * 1000)}_{id(self)}"
        
        connection = PooledConnection(
            id=connection_id,
            created_at=time.time(),
            last_used=time.time(),
            pool=weakref.ref(self)
        )
        
        # Initialize connection (placeholder for actual connection logic)
        await connection.initialize()
        
        self.connection_stats[connection_id] = {
            "created_at": time.time(),
            "total_uses": 0,
            "total_time": 0.0
        }
        
        logger.debug(f"Created new connection: {connection_id}")
        return connection
    
    async def _close_connection(self, connection: 'PooledConnection'):
        """Close and cleanup connection"""
        try:
            await connection.close()
            if connection.id in self.connection_stats:
                del self.connection_stats[connection.id]
            logger.debug(f"Closed connection: {connection.id}")
        except Exception as e:
            logger.error(f"Error closing connection {connection.id}: {e}")
    
    def _is_connection_healthy(self, connection: 'PooledConnection') -> bool:
        """Check if connection is healthy"""
        current_time = time.time()
        
        # Check if connection is too old
        if current_time - connection.created_at > self.config.keep_alive_timeout:
            return False
        
        # Check if connection has been idle too long
        if current_time - connection.last_used > self.config.max_idle_time:
            return False
        
        # Check connection-specific health
        return connection.is_healthy()
    
    def _cleanup_loop(self):
        """Background cleanup loop"""
        while self.is_running:
            try:
                self._cleanup_idle_connections()
                time.sleep(30)  # Cleanup every 30 seconds
            except Exception as e:
                logger.error(f"Connection cleanup error: {e}")
                time.sleep(60)
    
    def _cleanup_idle_connections(self):
        """Remove idle connections from pool"""
        with self.lock:
            current_time = time.time()
            connections_to_remove = []
            
            for connection in list(self.connections):
                if current_time - connection.last_used > self.config.max_idle_time:
                    connections_to_remove.append(connection)
            
            for connection in connections_to_remove:
                self.connections.remove(connection)
                asyncio.create_task(self._close_connection(connection))
            
            if connections_to_remove:
                logger.debug(f"Cleaned up {len(connections_to_remove)} idle connections")
    
    def _health_check_loop(self):
        """Background health check loop"""
        while self.is_running:
            try:
                self._perform_health_checks()
                time.sleep(self.config.health_check_interval)
            except Exception as e:
                logger.error(f"Health check error: {e}")
                time.sleep(self.config.health_check_interval)
    
    def _perform_health_checks(self):
        """Perform health checks on active connections"""
        with self.lock:
            unhealthy_connections = []
            
            for connection in list(self.active_connections.values()):
                if not self._is_connection_healthy(connection):
                    unhealthy_connections.append(connection)
            
            for connection in unhealthy_connections:
                if connection.id in self.active_connections:
                    del self.active_connections[connection.id]
                asyncio.create_task(self._close_connection(connection))
            
            if unhealthy_connections:
                logger.debug(f"Removed {len(unhealthy_connections)} unhealthy connections")
    
    def get_stats(self) -> Dict[str, Any]:
        """Get connection pool statistics"""
        with self.lock:
            return {
                "pool_size": len(self.connections),
                "active_connections": len(self.active_connections),
                "max_size": self.config.max_size,
                "total_connections_created": len(self.connection_stats),
                "average_connection_age": self._calculate_average_age(),
                "pool_utilization": len(self.active_connections) / self.config.max_size
            }
    
    def _calculate_average_age(self) -> float:
        """Calculate average age of connections"""
        if not self.connection_stats:
            return 0.0
        
        current_time = time.time()
        total_age = sum(
            current_time - stats["created_at"]
            for stats in self.connection_stats.values()
        )
        
        return total_age / len(self.connection_stats)
    
    async def close_all(self):
        """Close all connections and shutdown pool"""
        self.is_running = False
        
        with self.lock:
            # Close all pooled connections
            for connection in list(self.connections):
                await self._close_connection(connection)
            self.connections.clear()
            
            # Close all active connections
            for connection in list(self.active_connections.values()):
                await self._close_connection(connection)
            self.active_connections.clear()
        
        logger.info("Connection pool closed")

class PooledConnection:
    """Individual pooled connection"""
    
    def __init__(self, id: str, created_at: float, last_used: float, pool: weakref.ref):
        self.id = id
        self.created_at = created_at
        self.last_used = last_used
        self.pool = pool
        self.is_closed = False
        self.connection_data = {}
        
    async def initialize(self):
        """Initialize the connection"""
        # Placeholder for actual connection initialization
        self.connection_data["initialized"] = True
        await asyncio.sleep(0.001)  # Simulate initialization time
    
    def is_healthy(self) -> bool:
        """Check if connection is healthy"""
        return not self.is_closed and self.connection_data.get("initialized", False)
    
    async def execute(self, operation: str, *args, **kwargs):
        """Execute operation using this connection"""
        if self.is_closed:
            raise ConnectionClosed("Connection is closed")
        
        start_time = time.time()
        try:
            # Placeholder for actual operation execution
            await asyncio.sleep(0.001)  # Simulate operation time
            result = f"Result for {operation}"
            
            # Update usage statistics
            pool_ref = self.pool()
            if pool_ref and self.id in pool_ref.connection_stats:
                stats = pool_ref.connection_stats[self.id]
                stats["total_uses"] += 1
                stats["total_time"] += time.time() - start_time
            
            self.last_used = time.time()
            return result
            
        except Exception as e:
            logger.error(f"Connection {self.id} operation failed: {e}")
            raise
    
    async def close(self):
        """Close the connection"""
        if not self.is_closed:
            self.is_closed = True
            # Placeholder for actual connection cleanup
            self.connection_data.clear()
    
    async def __aenter__(self):
        """Async context manager entry"""
        return self
    
    async def __aexit__(self, exc_type, exc_val, exc_tb):
        """Async context manager exit"""
        pool_ref = self.pool()
        if pool_ref:
            await pool_ref.return_connection(self)

class ConnectionPoolExhausted(Exception):
    """Exception raised when connection pool is exhausted"""
    pass

class ConnectionClosed(Exception):
    """Exception raised when trying to use a closed connection"""
    pass

# Connection pool context manager
class ConnectionPoolContext:
    """Context manager for connection pool operations"""
    
    def __init__(self, pool: ConnectionPoolManager, key: str = None):
        self.pool = pool
        self.key = key
        self.connection = None
    
    async def __aenter__(self) -> PooledConnection:
        """Get connection from pool"""
        self.connection = await self.pool.get_connection(self.key)
        return self.connection
    
    async def __aexit__(self, exc_type, exc_val, exc_tb):
        """Return connection to pool"""
        if self.connection:
            await self.pool.return_connection(self.connection)

# Global connection pool instance
_global_pool = None

def get_connection_pool() -> ConnectionPoolManager:
    """Get global connection pool instance"""
    global _global_pool
    if _global_pool is None:
        _global_pool = ConnectionPoolManager()
    return _global_pool

async def with_connection(operation: Callable, key: str = None):
    """Execute operation with pooled connection"""
    pool = get_connection_pool()
    async with ConnectionPoolContext(pool, key) as connection:
        return await operation(connection)