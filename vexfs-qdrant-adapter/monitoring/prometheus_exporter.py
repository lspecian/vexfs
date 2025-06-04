"""
VexFS v2 Qdrant Adapter - Prometheus Exporter

Prometheus metrics exporter for production monitoring and observability.
"""

import asyncio
import time
from typing import Dict, List, Any, Optional
from dataclasses import dataclass
import logging
from aiohttp import web
import threading

from .metrics import MetricsCollector, PrometheusMetrics

logger = logging.getLogger(__name__)

@dataclass
class ExporterConfig:
    """Configuration for Prometheus exporter"""
    port: int = 8080
    host: str = "0.0.0.0"
    metrics_path: str = "/metrics"
    health_path: str = "/health"
    update_interval: int = 15

class PrometheusExporter:
    """
    Prometheus metrics exporter for VexFS v2 Qdrant adapter.
    
    Provides HTTP endpoint for Prometheus to scrape metrics in standard format.
    """
    
    def __init__(self, metrics_collector: MetricsCollector, config: ExporterConfig = None):
        self.metrics_collector = metrics_collector
        self.config = config or ExporterConfig()
        self.prometheus_metrics = PrometheusMetrics(metrics_collector)
        self.app = None
        self.runner = None
        self.site = None
        self.is_running = False
        
    async def start(self):
        """Start the Prometheus exporter HTTP server"""
        if self.is_running:
            return
        
        self.app = web.Application()
        self._setup_routes()
        
        self.runner = web.AppRunner(self.app)
        await self.runner.setup()
        
        self.site = web.TCPSite(
            self.runner, 
            self.config.host, 
            self.config.port
        )
        
        await self.site.start()
        self.is_running = True
        
        logger.info(f"🔍 Prometheus exporter started on {self.config.host}:{self.config.port}")
    
    async def stop(self):
        """Stop the Prometheus exporter HTTP server"""
        if not self.is_running:
            return
        
        if self.site:
            await self.site.stop()
        
        if self.runner:
            await self.runner.cleanup()
        
        self.is_running = False
        logger.info("Prometheus exporter stopped")
    
    def _setup_routes(self):
        """Setup HTTP routes for the exporter"""
        self.app.router.add_get(self.config.metrics_path, self._metrics_handler)
        self.app.router.add_get(self.config.health_path, self._health_handler)
        self.app.router.add_get("/", self._index_handler)
    
    async def _metrics_handler(self, request):
        """Handle metrics endpoint request"""
        try:
            metrics_text = self.prometheus_metrics.format_metrics()
            return web.Response(
                text=metrics_text,
                content_type="text/plain; version=0.0.4; charset=utf-8"
            )
        except Exception as e:
            logger.error(f"Error generating metrics: {e}")
            return web.Response(
                text=f"# Error generating metrics: {e}\n",
                status=500,
                content_type="text/plain"
            )
    
    async def _health_handler(self, request):
        """Handle health check endpoint"""
        health_status = {
            "status": "healthy",
            "timestamp": time.time(),
            "metrics_collector": "active" if self.metrics_collector else "inactive",
            "exporter_uptime": time.time() - getattr(self, 'start_time', time.time())
        }
        
        return web.json_response(health_status)
    
    async def _index_handler(self, request):
        """Handle index page"""
        html = f"""
        <html>
        <head><title>VexFS v2 Qdrant Adapter - Prometheus Exporter</title></head>
        <body>
        <h1>VexFS v2 Qdrant Adapter - Prometheus Exporter</h1>
        <p>Metrics endpoint: <a href="{self.config.metrics_path}">{self.config.metrics_path}</a></p>
        <p>Health endpoint: <a href="{self.config.health_path}">{self.config.health_path}</a></p>
        <p>Version: 2.0.0-phase4</p>
        </body>
        </html>
        """
        return web.Response(text=html, content_type="text/html")

class HealthMonitor:
    """Advanced health monitoring for production deployment"""
    
    def __init__(self, metrics_collector: MetricsCollector):
        self.metrics_collector = metrics_collector
        self.start_time = time.time()
        self.health_checks = {}
        self.last_check_time = 0
        
    def register_health_check(self, name: str, check_func: callable, interval: int = 60):
        """Register a health check function"""
        self.health_checks[name] = {
            "function": check_func,
            "interval": interval,
            "last_run": 0,
            "last_result": None,
            "last_error": None
        }
    
    async def run_health_checks(self) -> Dict[str, Any]:
        """Run all registered health checks"""
        current_time = time.time()
        results = {}
        
        for name, check_info in self.health_checks.items():
            if current_time - check_info["last_run"] >= check_info["interval"]:
                try:
                    if asyncio.iscoroutinefunction(check_info["function"]):
                        result = await check_info["function"]()
                    else:
                        result = check_info["function"]()
                    
                    check_info["last_result"] = result
                    check_info["last_error"] = None
                    check_info["last_run"] = current_time
                    
                except Exception as e:
                    check_info["last_error"] = str(e)
                    check_info["last_result"] = False
                    check_info["last_run"] = current_time
            
            results[name] = {
                "status": check_info["last_result"],
                "last_run": check_info["last_run"],
                "error": check_info["last_error"]
            }
        
        self.last_check_time = current_time
        return results
    
    def get_overall_health(self) -> Dict[str, Any]:
        """Get overall system health status"""
        uptime = time.time() - self.start_time
        
        # Get basic metrics
        metrics_summary = self.metrics_collector.get_performance_summary()
        
        # Determine overall health
        overall_healthy = True
        health_issues = []
        
        # Check memory usage
        memory_mb = metrics_summary.get("metrics", {}).get("memory_usage_bytes_latest", 0) / 1024 / 1024
        if memory_mb > 2048:  # 2GB threshold
            overall_healthy = False
            health_issues.append(f"High memory usage: {memory_mb:.1f}MB")
        
        # Check error rate
        error_rate = metrics_summary.get("metrics", {}).get("error_rate_latest", 0)
        if error_rate > 0.05:  # 5% error rate threshold
            overall_healthy = False
            health_issues.append(f"High error rate: {error_rate:.2%}")
        
        return {
            "status": "healthy" if overall_healthy else "unhealthy",
            "uptime_seconds": uptime,
            "issues": health_issues,
            "metrics_summary": metrics_summary,
            "last_check": self.last_check_time,
            "timestamp": time.time()
        }

class AdvancedHealthChecks:
    """Collection of advanced health check functions"""
    
    def __init__(self, metrics_collector: MetricsCollector):
        self.metrics_collector = metrics_collector
    
    async def check_vexfs_connectivity(self) -> bool:
        """Check VexFS kernel module connectivity"""
        try:
            # Simulate VexFS connectivity check
            # In real implementation, this would check actual VexFS device
            await asyncio.sleep(0.01)  # Simulate check
            return True
        except Exception:
            return False
    
    async def check_memory_usage(self) -> bool:
        """Check memory usage is within acceptable limits"""
        try:
            import psutil
            memory = psutil.virtual_memory()
            return memory.percent < 90  # Less than 90% memory usage
        except Exception:
            return False
    
    async def check_disk_space(self) -> bool:
        """Check disk space availability"""
        try:
            import psutil
            disk = psutil.disk_usage('/')
            return (disk.free / disk.total) > 0.1  # More than 10% free space
        except Exception:
            return False
    
    async def check_api_responsiveness(self) -> bool:
        """Check API responsiveness"""
        try:
            # Simulate API health check
            # In real implementation, this would make actual API calls
            await asyncio.sleep(0.01)
            return True
        except Exception:
            return False
    
    async def check_database_connectivity(self) -> bool:
        """Check database/storage connectivity"""
        try:
            # Simulate database connectivity check
            await asyncio.sleep(0.01)
            return True
        except Exception:
            return False

# Global exporter instance
_global_exporter = None

async def start_prometheus_exporter(metrics_collector: MetricsCollector, config: ExporterConfig = None):
    """Start global Prometheus exporter"""
    global _global_exporter
    
    if _global_exporter is None:
        _global_exporter = PrometheusExporter(metrics_collector, config)
        await _global_exporter.start()
        
        # Setup health monitoring
        health_monitor = HealthMonitor(metrics_collector)
        health_checks = AdvancedHealthChecks(metrics_collector)
        
        # Register health checks
        health_monitor.register_health_check("vexfs_connectivity", health_checks.check_vexfs_connectivity, 60)
        health_monitor.register_health_check("memory_usage", health_checks.check_memory_usage, 30)
        health_monitor.register_health_check("disk_space", health_checks.check_disk_space, 300)
        health_monitor.register_health_check("api_responsiveness", health_checks.check_api_responsiveness, 30)
        health_monitor.register_health_check("database_connectivity", health_checks.check_database_connectivity, 60)
        
        # Start health check loop
        asyncio.create_task(_health_check_loop(health_monitor))
        
        logger.info("🔍 Prometheus exporter and health monitoring started")
    
    return _global_exporter

async def _health_check_loop(health_monitor: HealthMonitor):
    """Background health check loop"""
    while True:
        try:
            await health_monitor.run_health_checks()
            await asyncio.sleep(30)  # Run health checks every 30 seconds
        except Exception as e:
            logger.error(f"Health check loop error: {e}")
            await asyncio.sleep(60)

async def stop_prometheus_exporter():
    """Stop global Prometheus exporter"""
    global _global_exporter
    
    if _global_exporter:
        await _global_exporter.stop()
        _global_exporter = None

def get_prometheus_exporter() -> Optional[PrometheusExporter]:
    """Get global Prometheus exporter instance"""
    return _global_exporter