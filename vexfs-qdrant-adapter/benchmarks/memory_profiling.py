"""
VexFS v2 Qdrant Adapter - Memory Profiling Module

Advanced memory usage analysis and optimization for production deployment.
"""

import asyncio
import psutil
import gc
import tracemalloc
import time
import threading
from typing import Dict, List, Any, Optional, Tuple
from dataclasses import dataclass
import logging
import numpy as np
from concurrent.futures import ThreadPoolExecutor

from .performance_suite import BenchmarkResult

logger = logging.getLogger(__name__)

@dataclass
class MemorySnapshot:
    """Memory usage snapshot"""
    timestamp: float
    total_mb: float
    available_mb: float
    used_mb: float
    percent: float
    process_mb: float
    heap_mb: Optional[float] = None

@dataclass
class MemoryProfile:
    """Complete memory profile"""
    baseline: MemorySnapshot
    peak: MemorySnapshot
    final: MemorySnapshot
    snapshots: List[MemorySnapshot]
    leak_detected: bool
    efficiency_score: float

class MemoryProfiler:
    """
    Advanced memory profiling for VexFS v2 Qdrant adapter.
    
    Provides:
    - Real-time memory monitoring
    - Memory leak detection
    - Efficiency analysis
    - Resource optimization recommendations
    """
    
    def __init__(self, monitoring_interval: float = 1.0):
        self.monitoring_interval = monitoring_interval
        self.process = psutil.Process()
        self.monitoring_active = False
        self.snapshots: List[MemorySnapshot] = []
        self.baseline_memory = None
        
    def start_monitoring(self):
        """Start continuous memory monitoring"""
        if self.monitoring_active:
            return
        
        self.monitoring_active = True
        self.snapshots.clear()
        
        # Start tracemalloc for detailed Python memory tracking
        tracemalloc.start()
        
        # Record baseline
        self.baseline_memory = self._take_snapshot()
        self.snapshots.append(self.baseline_memory)
        
        # Start monitoring thread
        self.monitor_thread = threading.Thread(target=self._monitor_loop, daemon=True)
        self.monitor_thread.start()
        
        logger.info("🔍 Memory monitoring started")
    
    def stop_monitoring(self) -> MemoryProfile:
        """Stop monitoring and return profile"""
        if not self.monitoring_active:
            return None
        
        self.monitoring_active = False
        
        # Wait for monitor thread to finish
        if hasattr(self, 'monitor_thread'):
            self.monitor_thread.join(timeout=5.0)
        
        # Take final snapshot
        final_snapshot = self._take_snapshot()
        self.snapshots.append(final_snapshot)
        
        # Stop tracemalloc
        tracemalloc.stop()
        
        # Analyze profile
        profile = self._analyze_memory_profile()
        
        logger.info("📊 Memory monitoring stopped")
        return profile
    
    def _monitor_loop(self):
        """Continuous monitoring loop"""
        while self.monitoring_active:
            try:
                snapshot = self._take_snapshot()
                self.snapshots.append(snapshot)
                time.sleep(self.monitoring_interval)
            except Exception as e:
                logger.error(f"Memory monitoring error: {e}")
                break
    
    def _take_snapshot(self) -> MemorySnapshot:
        """Take memory usage snapshot"""
        # System memory
        system_memory = psutil.virtual_memory()
        
        # Process memory
        process_memory = self.process.memory_info()
        process_mb = process_memory.rss / 1024 / 1024
        
        # Python heap memory (if tracemalloc is active)
        heap_mb = None
        if tracemalloc.is_tracing():
            current, peak = tracemalloc.get_traced_memory()
            heap_mb = current / 1024 / 1024
        
        return MemorySnapshot(
            timestamp=time.time(),
            total_mb=system_memory.total / 1024 / 1024,
            available_mb=system_memory.available / 1024 / 1024,
            used_mb=system_memory.used / 1024 / 1024,
            percent=system_memory.percent,
            process_mb=process_mb,
            heap_mb=heap_mb
        )
    
    def _analyze_memory_profile(self) -> MemoryProfile:
        """Analyze complete memory profile"""
        if len(self.snapshots) < 2:
            return None
        
        baseline = self.snapshots[0]
        final = self.snapshots[-1]
        peak = max(self.snapshots, key=lambda s: s.process_mb)
        
        # Detect memory leaks
        memory_growth = final.process_mb - baseline.process_mb
        leak_threshold = 50  # MB
        leak_detected = memory_growth > leak_threshold
        
        # Calculate efficiency score
        peak_usage = peak.process_mb - baseline.process_mb
        efficiency_score = max(0, 100 - (peak_usage / 10))  # Penalize high memory usage
        
        return MemoryProfile(
            baseline=baseline,
            peak=peak,
            final=final,
            snapshots=self.snapshots.copy(),
            leak_detected=leak_detected,
            efficiency_score=efficiency_score
        )
    
    async def test_memory_efficiency(self, num_vectors: int, target_memory_mb: int) -> BenchmarkResult:
        """
        Test memory efficiency with large datasets.
        
        Args:
            num_vectors: Number of vectors to process
            target_memory_mb: Target memory usage limit
        """
        logger.info(f"🧠 Testing memory efficiency: {num_vectors} vectors, target {target_memory_mb}MB")
        
        start_time = time.time()
        self.start_monitoring()
        
        try:
            # Simulate vector processing workload
            success_count = 0
            error_count = 0
            
            # Process vectors in batches to simulate real workload
            batch_size = 10000
            for i in range(0, num_vectors, batch_size):
                try:
                    # Simulate vector operations
                    current_batch_size = min(batch_size, num_vectors - i)
                    
                    # Create vectors (simulating insertion)
                    vectors = np.random.random((current_batch_size, 128)).astype(np.float32)
                    
                    # Simulate processing
                    await asyncio.sleep(0.1)
                    
                    # Simulate search operations
                    query_vector = np.random.random(128).astype(np.float32)
                    similarities = np.dot(vectors, query_vector)
                    
                    # Clean up batch
                    del vectors, similarities
                    gc.collect()
                    
                    success_count += current_batch_size
                    
                    # Check memory usage
                    current_snapshot = self._take_snapshot()
                    if current_snapshot.process_mb > target_memory_mb * 2:  # Allow 2x target as warning
                        logger.warning(f"Memory usage high: {current_snapshot.process_mb:.1f}MB")
                    
                except Exception as e:
                    logger.error(f"Batch processing error: {e}")
                    error_count += current_batch_size
            
            # Final cleanup
            gc.collect()
            await asyncio.sleep(1)  # Allow cleanup to complete
            
        finally:
            profile = self.stop_monitoring()
        
        duration = (time.time() - start_time) * 1000
        throughput = success_count / (duration / 1000) if duration > 0 else 0
        
        # Analyze memory efficiency
        peak_memory = profile.peak.process_mb - profile.baseline.process_mb
        memory_per_vector = peak_memory / num_vectors * 1000000  # MB per million vectors
        memory_target_met = peak_memory <= target_memory_mb
        
        return BenchmarkResult(
            name="Memory Efficiency Test",
            duration_ms=duration,
            throughput_ops_sec=throughput,
            memory_usage_mb=peak_memory,
            cpu_usage_percent=0,
            success_rate=success_count / num_vectors if num_vectors > 0 else 0,
            error_count=error_count,
            metadata={
                "num_vectors": num_vectors,
                "target_memory_mb": target_memory_mb,
                "peak_memory_mb": peak_memory,
                "memory_per_million_vectors": memory_per_vector,
                "memory_target_met": memory_target_met,
                "leak_detected": profile.leak_detected,
                "efficiency_score": profile.efficiency_score,
                "baseline_memory_mb": profile.baseline.process_mb,
                "final_memory_mb": profile.final.process_mb
            },
            timestamp=time.time()
        )
    
    async def test_memory_leak_detection(self, iterations: int, operation_type: str = "vector_ops") -> BenchmarkResult:
        """
        Test for memory leaks over multiple iterations.
        
        Args:
            iterations: Number of iterations to run
            operation_type: Type of operations to test
        """
        logger.info(f"🔍 Testing memory leak detection: {iterations} iterations, {operation_type}")
        
        start_time = time.time()
        self.start_monitoring()
        
        try:
            success_count = 0
            error_count = 0
            
            for i in range(iterations):
                try:
                    if operation_type == "vector_ops":
                        await self._simulate_vector_operations()
                    elif operation_type == "search_ops":
                        await self._simulate_search_operations()
                    elif operation_type == "filter_ops":
                        await self._simulate_filter_operations()
                    else:
                        await self._simulate_mixed_operations()
                    
                    success_count += 1
                    
                    # Periodic cleanup
                    if i % 100 == 0:
                        gc.collect()
                        
                except Exception as e:
                    logger.error(f"Operation {i} failed: {e}")
                    error_count += 1
            
            # Final cleanup and analysis
            gc.collect()
            await asyncio.sleep(2)  # Allow cleanup
            
        finally:
            profile = self.stop_monitoring()
        
        duration = (time.time() - start_time) * 1000
        throughput = success_count / (duration / 1000) if duration > 0 else 0
        
        # Analyze for memory leaks
        memory_growth = profile.final.process_mb - profile.baseline.process_mb
        growth_per_iteration = memory_growth / iterations if iterations > 0 else 0
        
        return BenchmarkResult(
            name="Memory Leak Detection",
            duration_ms=duration,
            throughput_ops_sec=throughput,
            memory_usage_mb=memory_growth,
            cpu_usage_percent=0,
            success_rate=success_count / iterations if iterations > 0 else 0,
            error_count=error_count,
            metadata={
                "iterations": iterations,
                "operation_type": operation_type,
                "memory_growth_mb": memory_growth,
                "growth_per_iteration_kb": growth_per_iteration * 1024,
                "leak_detected": profile.leak_detected,
                "efficiency_score": profile.efficiency_score,
                "baseline_memory_mb": profile.baseline.process_mb,
                "peak_memory_mb": profile.peak.process_mb,
                "final_memory_mb": profile.final.process_mb
            },
            timestamp=time.time()
        )
    
    async def profile_operation(self, operation_func, *args, **kwargs) -> Tuple[Any, MemoryProfile]:
        """
        Profile memory usage of a specific operation.
        
        Args:
            operation_func: Function to profile
            *args, **kwargs: Arguments for the function
            
        Returns:
            Tuple of (operation_result, memory_profile)
        """
        self.start_monitoring()
        
        try:
            result = await operation_func(*args, **kwargs)
        finally:
            profile = self.stop_monitoring()
        
        return result, profile
    
    def get_memory_recommendations(self, profile: MemoryProfile) -> List[str]:
        """Get memory optimization recommendations"""
        recommendations = []
        
        if profile.leak_detected:
            recommendations.append("🚨 Memory leak detected - review object lifecycle management")
        
        peak_usage = profile.peak.process_mb - profile.baseline.process_mb
        if peak_usage > 500:  # MB
            recommendations.append("💾 High memory usage - consider batch processing or streaming")
        
        if profile.efficiency_score < 70:
            recommendations.append("⚡ Low efficiency score - optimize data structures and algorithms")
        
        # Check for memory fragmentation
        snapshots_count = len(profile.snapshots)
        if snapshots_count > 10:
            memory_variance = np.var([s.process_mb for s in profile.snapshots])
            if memory_variance > 100:  # High variance indicates fragmentation
                recommendations.append("🔧 High memory variance - consider memory pooling")
        
        # Check growth rate
        if len(profile.snapshots) > 2:
            growth_rate = (profile.final.process_mb - profile.baseline.process_mb) / len(profile.snapshots)
            if growth_rate > 1:  # MB per snapshot
                recommendations.append("📈 High memory growth rate - review allocation patterns")
        
        if not recommendations:
            recommendations.append("✅ Memory usage appears optimal")
        
        return recommendations
    
    # Simulation methods for testing
    async def _simulate_vector_operations(self):
        """Simulate vector operations for testing"""
        vectors = np.random.random((1000, 128)).astype(np.float32)
        query = np.random.random(128).astype(np.float32)
        similarities = np.dot(vectors, query)
        await asyncio.sleep(0.01)
        del vectors, query, similarities
    
    async def _simulate_search_operations(self):
        """Simulate search operations for testing"""
        # Simulate index lookup
        index_data = {f"key_{i}": np.random.random(10) for i in range(100)}
        query_results = [index_data[f"key_{i}"] for i in range(0, 100, 10)]
        await asyncio.sleep(0.01)
        del index_data, query_results
    
    async def _simulate_filter_operations(self):
        """Simulate filter operations for testing"""
        # Simulate metadata filtering
        metadata = [{"category": f"cat_{i%5}", "value": i} for i in range(1000)]
        filtered = [m for m in metadata if m["category"] == "cat_1" and m["value"] > 500]
        await asyncio.sleep(0.01)
        del metadata, filtered
    
    async def _simulate_mixed_operations(self):
        """Simulate mixed operations for testing"""
        await self._simulate_vector_operations()
        await self._simulate_search_operations()
        await self._simulate_filter_operations()

# Utility functions for memory analysis
def analyze_memory_usage(snapshots: List[MemorySnapshot]) -> Dict[str, Any]:
    """Analyze memory usage patterns from snapshots"""
    if not snapshots:
        return {}
    
    process_memory = [s.process_mb for s in snapshots]
    system_memory = [s.percent for s in snapshots]
    
    return {
        "process_memory": {
            "min_mb": min(process_memory),
            "max_mb": max(process_memory),
            "avg_mb": sum(process_memory) / len(process_memory),
            "growth_mb": process_memory[-1] - process_memory[0]
        },
        "system_memory": {
            "min_percent": min(system_memory),
            "max_percent": max(system_memory),
            "avg_percent": sum(system_memory) / len(system_memory)
        },
        "timeline": {
            "duration_seconds": snapshots[-1].timestamp - snapshots[0].timestamp,
            "sample_count": len(snapshots)
        }
    }

def detect_memory_patterns(snapshots: List[MemorySnapshot]) -> Dict[str, Any]:
    """Detect memory usage patterns"""
    if len(snapshots) < 3:
        return {"status": "insufficient_data"}
    
    memory_values = [s.process_mb for s in snapshots]
    
    # Detect trends
    differences = [memory_values[i+1] - memory_values[i] for i in range(len(memory_values)-1)]
    avg_change = sum(differences) / len(differences)
    
    # Detect spikes
    threshold = np.std(memory_values) * 2
    spikes = [i for i, val in enumerate(memory_values) if abs(val - np.mean(memory_values)) > threshold]
    
    # Detect cycles
    has_cycles = len(set(differences)) < len(differences) * 0.7  # Rough heuristic
    
    return {
        "trend": "increasing" if avg_change > 1 else "decreasing" if avg_change < -1 else "stable",
        "avg_change_mb": avg_change,
        "spikes_detected": len(spikes),
        "spike_indices": spikes,
        "has_cycles": has_cycles,
        "volatility": np.std(memory_values)
    }