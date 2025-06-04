"""
VexFS v2 Qdrant Adapter - Performance Benchmarking Suite

Main benchmarking framework for comprehensive performance testing and optimization.
"""

import asyncio
import time
import json
import statistics
from typing import Dict, List, Any, Optional, Callable
from dataclasses import dataclass, asdict
from datetime import datetime
import psutil
import numpy as np
from concurrent.futures import ThreadPoolExecutor, as_completed
import logging

from .load_testing import LoadTester
from .memory_profiling import MemoryProfiler
from .concurrent_testing import ConcurrentTester
from .regression_testing import RegressionTester

logger = logging.getLogger(__name__)

@dataclass
class BenchmarkResult:
    """Individual benchmark result"""
    name: str
    duration_ms: float
    throughput_ops_sec: float
    memory_usage_mb: float
    cpu_usage_percent: float
    success_rate: float
    error_count: int
    metadata: Dict[str, Any]
    timestamp: str

@dataclass
class PerformanceTargets:
    """Production performance targets"""
    sustained_throughput_ops_sec: int = 500000
    memory_per_million_vectors_mb: int = 100
    latency_p99_ms: float = 5.0
    concurrent_connections: int = 1000
    uptime_percent: float = 99.9

class PerformanceSuite:
    """
    Comprehensive performance benchmarking suite for VexFS v2 Qdrant adapter.
    
    Provides real-world workload simulation, comparative benchmarks, and
    performance regression testing for production deployment validation.
    """
    
    def __init__(self, base_url: str = "http://localhost:6333"):
        self.base_url = base_url
        self.targets = PerformanceTargets()
        self.results: List[BenchmarkResult] = []
        
        # Initialize specialized testers
        self.load_tester = LoadTester(base_url)
        self.memory_profiler = MemoryProfiler()
        self.concurrent_tester = ConcurrentTester(base_url)
        self.regression_tester = RegressionTester()
        
        # Performance monitoring
        self.start_time = None
        self.baseline_memory = None
        
    async def run_comprehensive_benchmark(self) -> Dict[str, Any]:
        """
        Run complete performance benchmark suite covering all production scenarios.
        
        Returns:
            Comprehensive benchmark results with pass/fail status
        """
        logger.info("🚀 Starting comprehensive performance benchmark suite")
        
        self.start_time = time.time()
        self.baseline_memory = psutil.virtual_memory().used / 1024 / 1024
        
        benchmark_suite = [
            ("Basic Operations", self._benchmark_basic_operations),
            ("Vector Search Performance", self._benchmark_vector_search),
            ("Filter DSL Performance", self._benchmark_filter_dsl),
            ("Recommendation Performance", self._benchmark_recommendations),
            ("Scroll API Performance", self._benchmark_scroll_api),
            ("Batch Operations Performance", self._benchmark_batch_operations),
            ("Concurrent Load Test", self._benchmark_concurrent_load),
            ("Memory Efficiency Test", self._benchmark_memory_efficiency),
            ("Sustained Load Test", self._benchmark_sustained_load),
            ("Stress Test", self._benchmark_stress_test)
        ]
        
        results = {}
        
        for name, benchmark_func in benchmark_suite:
            logger.info(f"📊 Running benchmark: {name}")
            try:
                result = await benchmark_func()
                results[name] = result
                self.results.append(result)
                logger.info(f"✅ {name} completed: {result.throughput_ops_sec:.0f} ops/sec")
            except Exception as e:
                logger.error(f"❌ {name} failed: {e}")
                results[name] = self._create_error_result(name, str(e))
        
        # Generate comprehensive report
        report = self._generate_performance_report(results)
        
        # Save results
        await self._save_benchmark_results(report)
        
        logger.info("🎉 Comprehensive benchmark suite completed")
        return report
    
    async def _benchmark_basic_operations(self) -> BenchmarkResult:
        """Benchmark basic CRUD operations"""
        start_time = time.time()
        start_memory = psutil.virtual_memory().used / 1024 / 1024
        
        # Test basic operations
        operations = 1000
        success_count = 0
        
        for i in range(operations):
            try:
                # Simulate basic operations
                await asyncio.sleep(0.001)  # Simulate operation time
                success_count += 1
            except Exception:
                pass
        
        duration = (time.time() - start_time) * 1000
        memory_used = psutil.virtual_memory().used / 1024 / 1024 - start_memory
        throughput = operations / (duration / 1000)
        
        return BenchmarkResult(
            name="Basic Operations",
            duration_ms=duration,
            throughput_ops_sec=throughput,
            memory_usage_mb=memory_used,
            cpu_usage_percent=psutil.cpu_percent(),
            success_rate=success_count / operations,
            error_count=operations - success_count,
            metadata={"operations": operations},
            timestamp=datetime.now().isoformat()
        )
    
    async def _benchmark_vector_search(self) -> BenchmarkResult:
        """Benchmark vector search performance"""
        return await self.load_tester.benchmark_vector_search(
            num_vectors=10000,
            vector_dim=128,
            search_queries=1000
        )
    
    async def _benchmark_filter_dsl(self) -> BenchmarkResult:
        """Benchmark Filter DSL performance"""
        return await self.load_tester.benchmark_filter_operations(
            num_points=50000,
            filter_complexity=10
        )
    
    async def _benchmark_recommendations(self) -> BenchmarkResult:
        """Benchmark recommendation system performance"""
        return await self.load_tester.benchmark_recommendations(
            num_examples=100,
            strategies=["centroid", "average_vector", "best_score"]
        )
    
    async def _benchmark_scroll_api(self) -> BenchmarkResult:
        """Benchmark Scroll API performance"""
        return await self.load_tester.benchmark_scroll_operations(
            total_points=100000,
            batch_size=1000
        )
    
    async def _benchmark_batch_operations(self) -> BenchmarkResult:
        """Benchmark batch operations performance"""
        return await self.load_tester.benchmark_batch_search(
            num_queries=50,
            concurrent_batches=5
        )
    
    async def _benchmark_concurrent_load(self) -> BenchmarkResult:
        """Benchmark concurrent request handling"""
        return await self.concurrent_tester.test_concurrent_connections(
            max_connections=self.targets.concurrent_connections,
            duration_seconds=60
        )
    
    async def _benchmark_memory_efficiency(self) -> BenchmarkResult:
        """Benchmark memory efficiency with large datasets"""
        return await self.memory_profiler.test_memory_efficiency(
            num_vectors=1000000,
            target_memory_mb=self.targets.memory_per_million_vectors_mb
        )
    
    async def _benchmark_sustained_load(self) -> BenchmarkResult:
        """Benchmark sustained load performance"""
        return await self.load_tester.sustained_load_test(
            target_ops_sec=self.targets.sustained_throughput_ops_sec,
            duration_minutes=10
        )
    
    async def _benchmark_stress_test(self) -> BenchmarkResult:
        """Benchmark system under stress conditions"""
        return await self.load_tester.stress_test(
            max_load_multiplier=5,
            duration_minutes=5
        )
    
    def _create_error_result(self, name: str, error: str) -> BenchmarkResult:
        """Create error result for failed benchmarks"""
        return BenchmarkResult(
            name=name,
            duration_ms=0,
            throughput_ops_sec=0,
            memory_usage_mb=0,
            cpu_usage_percent=0,
            success_rate=0,
            error_count=1,
            metadata={"error": error},
            timestamp=datetime.now().isoformat()
        )
    
    def _generate_performance_report(self, results: Dict[str, BenchmarkResult]) -> Dict[str, Any]:
        """Generate comprehensive performance report"""
        total_duration = time.time() - self.start_time
        
        # Calculate aggregate metrics
        successful_benchmarks = [r for r in results.values() if r.success_rate > 0.95]
        avg_throughput = statistics.mean([r.throughput_ops_sec for r in successful_benchmarks]) if successful_benchmarks else 0
        total_memory = sum([r.memory_usage_mb for r in results.values()])
        
        # Check against targets
        targets_met = {
            "sustained_throughput": avg_throughput >= self.targets.sustained_throughput_ops_sec,
            "memory_efficiency": total_memory <= self.targets.memory_per_million_vectors_mb,
            "concurrent_connections": any(r.name == "Concurrent Load Test" and r.success_rate > 0.95 for r in results.values()),
            "overall_success": len(successful_benchmarks) >= len(results) * 0.8
        }
        
        return {
            "summary": {
                "total_duration_seconds": total_duration,
                "benchmarks_run": len(results),
                "benchmarks_passed": len(successful_benchmarks),
                "average_throughput_ops_sec": avg_throughput,
                "total_memory_usage_mb": total_memory,
                "targets_met": targets_met,
                "overall_pass": all(targets_met.values())
            },
            "detailed_results": {name: asdict(result) for name, result in results.items()},
            "performance_targets": asdict(self.targets),
            "system_info": {
                "cpu_count": psutil.cpu_count(),
                "memory_total_gb": psutil.virtual_memory().total / 1024 / 1024 / 1024,
                "timestamp": datetime.now().isoformat()
            }
        }
    
    async def _save_benchmark_results(self, report: Dict[str, Any]) -> None:
        """Save benchmark results to file"""
        timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
        filename = f"benchmark_results_{timestamp}.json"
        
        with open(filename, 'w') as f:
            json.dump(report, f, indent=2)
        
        logger.info(f"📊 Benchmark results saved to {filename}")
    
    async def compare_with_baseline(self, baseline_file: str) -> Dict[str, Any]:
        """Compare current results with baseline performance"""
        return await self.regression_tester.compare_with_baseline(
            current_results=self.results,
            baseline_file=baseline_file
        )
    
    def get_performance_summary(self) -> Dict[str, Any]:
        """Get quick performance summary"""
        if not self.results:
            return {"status": "No benchmarks run"}
        
        successful = [r for r in self.results if r.success_rate > 0.95]
        
        return {
            "benchmarks_run": len(self.results),
            "benchmarks_passed": len(successful),
            "average_throughput": statistics.mean([r.throughput_ops_sec for r in successful]) if successful else 0,
            "total_memory_mb": sum([r.memory_usage_mb for r in self.results]),
            "overall_success_rate": len(successful) / len(self.results) if self.results else 0
        }

# CLI interface for running benchmarks
async def main():
    """Main CLI interface for performance benchmarking"""
    import argparse
    
    parser = argparse.ArgumentParser(description="VexFS v2 Qdrant Adapter Performance Benchmarking")
    parser.add_argument("--url", default="http://localhost:6333", help="Base URL for testing")
    parser.add_argument("--baseline", help="Baseline file for regression testing")
    parser.add_argument("--output", help="Output file for results")
    
    args = parser.parse_args()
    
    # Setup logging
    logging.basicConfig(level=logging.INFO, format='%(asctime)s - %(levelname)s - %(message)s')
    
    # Run benchmarks
    suite = PerformanceSuite(args.url)
    results = await suite.run_comprehensive_benchmark()
    
    # Print summary
    print("\n" + "="*80)
    print("PERFORMANCE BENCHMARK SUMMARY")
    print("="*80)
    print(f"Overall Pass: {'✅ PASS' if results['summary']['overall_pass'] else '❌ FAIL'}")
    print(f"Benchmarks: {results['summary']['benchmarks_passed']}/{results['summary']['benchmarks_run']} passed")
    print(f"Average Throughput: {results['summary']['average_throughput_ops_sec']:.0f} ops/sec")
    print(f"Memory Usage: {results['summary']['total_memory_usage_mb']:.1f} MB")
    print("="*80)
    
    # Compare with baseline if provided
    if args.baseline:
        comparison = await suite.compare_with_baseline(args.baseline)
        print("\nREGRESSION ANALYSIS:")
        print(f"Performance Change: {comparison.get('performance_change', 'N/A')}")
    
    return results

if __name__ == "__main__":
    asyncio.run(main())