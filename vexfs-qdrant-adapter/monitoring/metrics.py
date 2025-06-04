"""
VexFS v2 Qdrant Adapter - Metrics Collection Module

Advanced metrics collection and aggregation for production monitoring.
"""

import time
import threading
from typing import Dict, List, Any, Optional, Callable
from dataclasses import dataclass, field
from collections import defaultdict, deque
import statistics
import logging
from datetime import datetime, timedelta
import psutil
import asyncio

logger = logging.getLogger(__name__)

@dataclass
class MetricPoint:
    """Individual metric data point"""
    timestamp: float
    value: float
    labels: Dict[str, str] = field(default_factory=dict)

@dataclass
class MetricSeries:
    """Time series of metric points"""
    name: str
    description: str
    metric_type: str  # "counter", "gauge", "histogram", "summary"
    points: deque = field(default_factory=lambda: deque(maxlen=1000))
    labels: Dict[str, str] = field(default_factory=dict)

class MetricsCollector:
    """
    Advanced metrics collector for VexFS v2 Qdrant adapter.
    
    Collects and aggregates metrics for:
    - Request performance
    - System resources
    - Business metrics
    - Error rates
    """
    
    def __init__(self, retention_seconds: int = 3600):
        self.retention_seconds = retention_seconds
        self.metrics: Dict[str, MetricSeries] = {}
        self.lock = threading.RLock()
        self.start_time = time.time()
        
        # Built-in metrics
        self._init_builtin_metrics()
        
        # Background cleanup
        self._cleanup_thread = threading.Thread(target=self._cleanup_loop, daemon=True)
        self._cleanup_thread.start()
    
    def _init_builtin_metrics(self):
        """Initialize built-in system metrics"""
        builtin_metrics = [
            ("http_requests_total", "Total HTTP requests", "counter"),
            ("http_request_duration_seconds", "HTTP request duration", "histogram"),
            ("http_requests_in_flight", "HTTP requests currently being processed", "gauge"),
            ("vector_search_operations_total", "Total vector search operations", "counter"),
            ("vector_search_duration_seconds", "Vector search operation duration", "histogram"),
            ("filter_operations_total", "Total filter operations", "counter"),
            ("filter_operation_duration_seconds", "Filter operation duration", "histogram"),
            ("recommendation_operations_total", "Total recommendation operations", "counter"),
            ("recommendation_duration_seconds", "Recommendation operation duration", "histogram"),
            ("scroll_operations_total", "Total scroll operations", "counter"),
            ("scroll_duration_seconds", "Scroll operation duration", "histogram"),
            ("batch_operations_total", "Total batch operations", "counter"),
            ("batch_operation_duration_seconds", "Batch operation duration", "histogram"),
            ("memory_usage_bytes", "Memory usage in bytes", "gauge"),
            ("cpu_usage_percent", "CPU usage percentage", "gauge"),
            ("active_connections", "Number of active connections", "gauge"),
            ("error_rate", "Error rate percentage", "gauge"),
            ("vexfs_operations_total", "Total VexFS operations", "counter"),
            ("vexfs_operation_duration_seconds", "VexFS operation duration", "histogram"),
        ]
        
        for name, description, metric_type in builtin_metrics:
            self.register_metric(name, description, metric_type)
    
    def register_metric(self, name: str, description: str, metric_type: str, labels: Dict[str, str] = None):
        """Register a new metric"""
        with self.lock:
            if name not in self.metrics:
                self.metrics[name] = MetricSeries(
                    name=name,
                    description=description,
                    metric_type=metric_type,
                    labels=labels or {}
                )
                logger.debug(f"Registered metric: {name}")
    
    def record_counter(self, name: str, value: float = 1.0, labels: Dict[str, str] = None):
        """Record a counter metric (monotonically increasing)"""
        self._record_metric(name, value, labels, "counter")
    
    def record_gauge(self, name: str, value: float, labels: Dict[str, str] = None):
        """Record a gauge metric (can go up or down)"""
        self._record_metric(name, value, labels, "gauge")
    
    def record_histogram(self, name: str, value: float, labels: Dict[str, str] = None):
        """Record a histogram metric (for measuring distributions)"""
        self._record_metric(name, value, labels, "histogram")
    
    def record_timing(self, name: str, duration_seconds: float, labels: Dict[str, str] = None):
        """Record timing information"""
        self.record_histogram(f"{name}_duration_seconds", duration_seconds, labels)
    
    def _record_metric(self, name: str, value: float, labels: Dict[str, str], expected_type: str):
        """Internal method to record metric"""
        with self.lock:
            if name not in self.metrics:
                self.register_metric(name, f"Auto-generated {expected_type}", expected_type, labels)
            
            metric = self.metrics[name]
            if metric.metric_type != expected_type:
                logger.warning(f"Metric type mismatch for {name}: expected {expected_type}, got {metric.metric_type}")
            
            point = MetricPoint(
                timestamp=time.time(),
                value=value,
                labels=labels or {}
            )
            
            metric.points.append(point)
    
    def get_metric_value(self, name: str, aggregation: str = "latest") -> Optional[float]:
        """Get aggregated metric value"""
        with self.lock:
            if name not in self.metrics:
                return None
            
            metric = self.metrics[name]
            if not metric.points:
                return None
            
            values = [p.value for p in metric.points]
            
            if aggregation == "latest":
                return values[-1]
            elif aggregation == "sum":
                return sum(values)
            elif aggregation == "avg":
                return statistics.mean(values)
            elif aggregation == "min":
                return min(values)
            elif aggregation == "max":
                return max(values)
            elif aggregation == "p95":
                return statistics.quantiles(values, n=20)[18] if len(values) > 1 else values[0]
            elif aggregation == "p99":
                return statistics.quantiles(values, n=100)[98] if len(values) > 1 else values[0]
            else:
                return values[-1]
    
    def get_metric_series(self, name: str, since_seconds: int = 300) -> List[MetricPoint]:
        """Get metric time series data"""
        with self.lock:
            if name not in self.metrics:
                return []
            
            cutoff_time = time.time() - since_seconds
            metric = self.metrics[name]
            
            return [p for p in metric.points if p.timestamp >= cutoff_time]
    
    def get_all_metrics(self) -> Dict[str, Any]:
        """Get all current metric values"""
        with self.lock:
            result = {}
            
            for name, metric in self.metrics.items():
                if metric.points:
                    latest_value = metric.points[-1].value
                    result[name] = {
                        "value": latest_value,
                        "type": metric.metric_type,
                        "description": metric.description,
                        "timestamp": metric.points[-1].timestamp,
                        "points_count": len(metric.points)
                    }
            
            return result
    
    def get_performance_summary(self) -> Dict[str, Any]:
        """Get performance summary with key metrics"""
        summary = {
            "uptime_seconds": time.time() - self.start_time,
            "timestamp": datetime.now().isoformat(),
            "metrics": {}
        }
        
        # Key performance metrics
        key_metrics = [
            ("http_requests_total", "sum"),
            ("http_request_duration_seconds", "p95"),
            ("vector_search_operations_total", "sum"),
            ("vector_search_duration_seconds", "avg"),
            ("memory_usage_bytes", "latest"),
            ("cpu_usage_percent", "latest"),
            ("active_connections", "latest"),
            ("error_rate", "latest")
        ]
        
        for metric_name, aggregation in key_metrics:
            value = self.get_metric_value(metric_name, aggregation)
            if value is not None:
                summary["metrics"][f"{metric_name}_{aggregation}"] = value
        
        return summary
    
    def _cleanup_loop(self):
        """Background cleanup of old metrics"""
        while True:
            try:
                self._cleanup_old_metrics()
                time.sleep(60)  # Cleanup every minute
            except Exception as e:
                logger.error(f"Metrics cleanup error: {e}")
                time.sleep(60)
    
    def _cleanup_old_metrics(self):
        """Remove old metric points beyond retention period"""
        cutoff_time = time.time() - self.retention_seconds
        
        with self.lock:
            for metric in self.metrics.values():
                # Remove old points
                while metric.points and metric.points[0].timestamp < cutoff_time:
                    metric.points.popleft()

class PrometheusMetrics:
    """
    Prometheus-compatible metrics formatter.
    
    Formats metrics in Prometheus exposition format for scraping.
    """
    
    def __init__(self, metrics_collector: MetricsCollector):
        self.collector = metrics_collector
    
    def format_metrics(self) -> str:
        """Format all metrics in Prometheus exposition format"""
        lines = []
        
        with self.collector.lock:
            for name, metric in self.collector.metrics.items():
                if not metric.points:
                    continue
                
                # Add HELP and TYPE comments
                lines.append(f"# HELP {name} {metric.description}")
                lines.append(f"# TYPE {name} {metric.metric_type}")
                
                if metric.metric_type == "histogram":
                    lines.extend(self._format_histogram(name, metric))
                else:
                    lines.extend(self._format_simple_metric(name, metric))
                
                lines.append("")  # Empty line between metrics
        
        return "\n".join(lines)
    
    def _format_simple_metric(self, name: str, metric: MetricSeries) -> List[str]:
        """Format simple metric (counter, gauge)"""
        lines = []
        
        if metric.metric_type == "counter":
            # For counters, use the sum of all values
            total_value = sum(p.value for p in metric.points)
            lines.append(f"{name}_total {total_value}")
        else:
            # For gauges, use the latest value
            latest_point = metric.points[-1]
            label_str = self._format_labels(latest_point.labels)
            lines.append(f"{name}{label_str} {latest_point.value}")
        
        return lines
    
    def _format_histogram(self, name: str, metric: MetricSeries) -> List[str]:
        """Format histogram metric with buckets"""
        lines = []
        
        # Get all values for histogram calculation
        values = [p.value for p in metric.points]
        
        if not values:
            return lines
        
        # Define histogram buckets
        buckets = [0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0, float('inf')]
        
        # Calculate bucket counts
        bucket_counts = {}
        for bucket in buckets:
            count = sum(1 for v in values if v <= bucket)
            bucket_counts[bucket] = count
        
        # Format bucket metrics
        for bucket in buckets:
            bucket_label = "+Inf" if bucket == float('inf') else str(bucket)
            lines.append(f"{name}_bucket{{le=\"{bucket_label}\"}} {bucket_counts[bucket]}")
        
        # Add sum and count
        lines.append(f"{name}_sum {sum(values)}")
        lines.append(f"{name}_count {len(values)}")
        
        return lines
    
    def _format_labels(self, labels: Dict[str, str]) -> str:
        """Format labels for Prometheus"""
        if not labels:
            return ""
        
        label_pairs = [f'{k}="{v}"' for k, v in labels.items()]
        return "{" + ",".join(label_pairs) + "}"

class TimingContext:
    """Context manager for timing operations"""
    
    def __init__(self, metrics_collector: MetricsCollector, metric_name: str, labels: Dict[str, str] = None):
        self.collector = metrics_collector
        self.metric_name = metric_name
        self.labels = labels
        self.start_time = None
    
    def __enter__(self):
        self.start_time = time.time()
        return self
    
    def __exit__(self, exc_type, exc_val, exc_tb):
        if self.start_time:
            duration = time.time() - self.start_time
            self.collector.record_timing(self.metric_name, duration, self.labels)

def timing_decorator(metrics_collector: MetricsCollector, metric_name: str = None, labels: Dict[str, str] = None):
    """Decorator for timing function execution"""
    def decorator(func):
        def wrapper(*args, **kwargs):
            name = metric_name or f"{func.__module__}.{func.__name__}"
            with TimingContext(metrics_collector, name, labels):
                return func(*args, **kwargs)
        return wrapper
    return decorator

async def async_timing_decorator(metrics_collector: MetricsCollector, metric_name: str = None, labels: Dict[str, str] = None):
    """Decorator for timing async function execution"""
    def decorator(func):
        async def wrapper(*args, **kwargs):
            name = metric_name or f"{func.__module__}.{func.__name__}"
            start_time = time.time()
            try:
                result = await func(*args, **kwargs)
                return result
            finally:
                duration = time.time() - start_time
                metrics_collector.record_timing(name, duration, labels)
        return wrapper
    return decorator

# Global metrics collector instance
_global_metrics_collector = None

def get_metrics_collector() -> MetricsCollector:
    """Get global metrics collector instance"""
    global _global_metrics_collector
    if _global_metrics_collector is None:
        _global_metrics_collector = MetricsCollector()
    return _global_metrics_collector

def record_system_metrics():
    """Record current system metrics"""
    collector = get_metrics_collector()
    
    # Memory usage
    memory = psutil.virtual_memory()
    collector.record_gauge("memory_usage_bytes", memory.used)
    collector.record_gauge("memory_usage_percent", memory.percent)
    
    # CPU usage
    cpu_percent = psutil.cpu_percent(interval=None)
    collector.record_gauge("cpu_usage_percent", cpu_percent)
    
    # Disk usage (if VexFS device is available)
    try:
        disk = psutil.disk_usage('/')
        collector.record_gauge("disk_usage_bytes", disk.used)
        collector.record_gauge("disk_usage_percent", (disk.used / disk.total) * 100)
    except Exception:
        pass  # Ignore disk errors

async def start_system_metrics_collection(interval_seconds: int = 30):
    """Start background system metrics collection"""
    while True:
        try:
            record_system_metrics()
            await asyncio.sleep(interval_seconds)
        except Exception as e:
            logger.error(f"System metrics collection error: {e}")
            await asyncio.sleep(interval_seconds)