"""
Logging Setup

This module configures structured logging for the VexFS Qdrant adapter
with performance monitoring and request tracking capabilities.
"""

import logging
import logging.handlers
import sys
import time
from typing import Any, Dict, Optional
from functools import wraps
import structlog
from .config import get_config


def setup_logging():
    """Setup structured logging with performance monitoring"""
    config = get_config()
    
    # Configure standard library logging
    logging.basicConfig(
        level=getattr(logging, config.logging.level.upper()),
        format=config.logging.format,
        handlers=_get_handlers(config)
    )
    
    # Configure structlog
    structlog.configure(
        processors=[
            structlog.stdlib.filter_by_level,
            structlog.stdlib.add_logger_name,
            structlog.stdlib.add_log_level,
            structlog.stdlib.PositionalArgumentsFormatter(),
            structlog.processors.TimeStamper(fmt="iso"),
            structlog.processors.StackInfoRenderer(),
            structlog.processors.format_exc_info,
            structlog.processors.UnicodeDecoder(),
            structlog.processors.JSONRenderer()
        ],
        context_class=dict,
        logger_factory=structlog.stdlib.LoggerFactory(),
        wrapper_class=structlog.stdlib.BoundLogger,
        cache_logger_on_first_use=True,
    )


def _get_handlers(config):
    """Get logging handlers based on configuration"""
    handlers = []
    
    # Console handler
    console_handler = logging.StreamHandler(sys.stdout)
    console_handler.setFormatter(logging.Formatter(config.logging.format))
    handlers.append(console_handler)
    
    # File handler if specified
    if config.logging.file_path:
        file_handler = logging.handlers.RotatingFileHandler(
            config.logging.file_path,
            maxBytes=config.logging.max_file_size,
            backupCount=config.logging.backup_count
        )
        file_handler.setFormatter(logging.Formatter(config.logging.format))
        handlers.append(file_handler)
    
    return handlers


def get_logger(name: str) -> structlog.BoundLogger:
    """Get a structured logger instance"""
    return structlog.get_logger(name)


class PerformanceLogger:
    """Performance monitoring logger"""
    
    def __init__(self, name: str):
        self.logger = get_logger(f"perf.{name}")
        self.config = get_config()
    
    def log_operation(self, operation: str, duration: float, **kwargs):
        """Log an operation with performance metrics"""
        if not self.config.performance.enable_metrics:
            return
        
        self.logger.info(
            "operation_completed",
            operation=operation,
            duration_ms=duration * 1000,
            **kwargs
        )
    
    def log_vexfs_operation(self, operation: str, duration: float, 
                           vector_count: int = 0, dimensions: int = 0):
        """Log VexFS-specific operation metrics"""
        ops_per_sec = vector_count / duration if duration > 0 else 0
        
        self.logger.info(
            "vexfs_operation",
            operation=operation,
            duration_ms=duration * 1000,
            vector_count=vector_count,
            dimensions=dimensions,
            ops_per_sec=ops_per_sec
        )
    
    def log_api_request(self, method: str, path: str, duration: float, 
                       status_code: int, **kwargs):
        """Log API request metrics"""
        if not self.config.logging.log_requests:
            return
        
        self.logger.info(
            "api_request",
            method=method,
            path=path,
            duration_ms=duration * 1000,
            status_code=status_code,
            **kwargs
        )


def performance_monitor(operation_name: str):
    """Decorator for monitoring function performance"""
    def decorator(func):
        @wraps(func)
        def wrapper(*args, **kwargs):
            perf_logger = PerformanceLogger(func.__module__)
            start_time = time.time()
            
            try:
                result = func(*args, **kwargs)
                duration = time.time() - start_time
                
                perf_logger.log_operation(
                    operation=operation_name,
                    duration=duration,
                    success=True
                )
                
                return result
            except Exception as e:
                duration = time.time() - start_time
                
                perf_logger.log_operation(
                    operation=operation_name,
                    duration=duration,
                    success=False,
                    error=str(e)
                )
                
                raise
        
        return wrapper
    return decorator


def vexfs_operation_monitor(operation_name: str):
    """Decorator for monitoring VexFS operations specifically"""
    def decorator(func):
        @wraps(func)
        def wrapper(*args, **kwargs):
            perf_logger = PerformanceLogger("vexfs")
            start_time = time.time()
            
            try:
                result = func(*args, **kwargs)
                duration = time.time() - start_time
                
                # Extract vector metrics if available
                vector_count = 0
                dimensions = 0
                
                if hasattr(result, 'get'):
                    vector_count = result.get('vector_count', 0)
                    dimensions = result.get('dimensions', 0)
                elif 'points' in kwargs:
                    vector_count = len(kwargs['points'])
                    if kwargs['points'] and 'vector' in kwargs['points'][0]:
                        dimensions = len(kwargs['points'][0]['vector'])
                
                perf_logger.log_vexfs_operation(
                    operation=operation_name,
                    duration=duration,
                    vector_count=vector_count,
                    dimensions=dimensions
                )
                
                return result
            except Exception as e:
                duration = time.time() - start_time
                
                perf_logger.logger.error(
                    "vexfs_operation_failed",
                    operation=operation_name,
                    duration_ms=duration * 1000,
                    error=str(e)
                )
                
                raise
        
        return wrapper
    return decorator


class RequestLogger:
    """HTTP request logger with performance tracking"""
    
    def __init__(self):
        self.logger = get_logger("api.requests")
        self.perf_logger = PerformanceLogger("api")
    
    async def log_request(self, request, call_next):
        """Middleware function for logging requests"""
        start_time = time.time()
        
        # Log request start
        self.logger.info(
            "request_started",
            method=request.method,
            path=request.url.path,
            query_params=str(request.query_params),
            client_ip=request.client.host if request.client else None
        )
        
        try:
            response = await call_next(request)
            duration = time.time() - start_time
            
            # Log successful request
            self.perf_logger.log_api_request(
                method=request.method,
                path=request.url.path,
                duration=duration,
                status_code=response.status_code
            )
            
            return response
            
        except Exception as e:
            duration = time.time() - start_time
            
            # Log failed request
            self.logger.error(
                "request_failed",
                method=request.method,
                path=request.url.path,
                duration_ms=duration * 1000,
                error=str(e)
            )
            
            raise


def log_startup_info():
    """Log startup information"""
    logger = get_logger("startup")
    config = get_config()
    
    logger.info(
        "vexfs_qdrant_adapter_starting",
        vexfs_device=config.vexfs.device_path,
        api_host=config.api.host,
        api_port=config.api.port,
        log_level=config.logging.level,
        performance_monitoring=config.performance.enable_metrics
    )


def log_vexfs_stats(stats: Dict[str, Any]):
    """Log VexFS performance statistics"""
    logger = get_logger("vexfs.stats")
    
    logger.info(
        "vexfs_performance_stats",
        **stats
    )


def log_error(logger_name: str, error: Exception, context: Optional[Dict[str, Any]] = None):
    """Log an error with context"""
    logger = get_logger(logger_name)
    
    error_context = {
        "error_type": type(error).__name__,
        "error_message": str(error)
    }
    
    if context:
        error_context.update(context)
    
    logger.error("error_occurred", **error_context)


# Initialize logging on module import
setup_logging()