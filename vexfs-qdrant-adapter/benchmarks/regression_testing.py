"""
VexFS v2 Qdrant Adapter - Regression Testing Module

Performance regression detection and baseline comparison for production deployment.
"""

import json
import time
import statistics
from typing import Dict, List, Any, Optional, Tuple
from dataclasses import dataclass, asdict
import logging
from pathlib import Path
import numpy as np
from datetime import datetime

from .performance_suite import BenchmarkResult

logger = logging.getLogger(__name__)

@dataclass
class RegressionThresholds:
    """Thresholds for regression detection"""
    throughput_degradation_percent: float = 10.0  # 10% degradation triggers alert
    latency_increase_percent: float = 20.0  # 20% latency increase triggers alert
    memory_increase_percent: float = 15.0  # 15% memory increase triggers alert
    success_rate_decrease_percent: float = 5.0  # 5% success rate decrease triggers alert

@dataclass
class RegressionResult:
    """Result of regression analysis"""
    metric_name: str
    baseline_value: float
    current_value: float
    change_percent: float
    is_regression: bool
    severity: str  # "low", "medium", "high", "critical"
    threshold_used: float

@dataclass
class BaselineComparison:
    """Complete baseline comparison result"""
    baseline_timestamp: str
    current_timestamp: str
    overall_regression_detected: bool
    regression_count: int
    improvement_count: int
    stable_count: int
    metric_comparisons: List[RegressionResult]
    summary: Dict[str, Any]

class RegressionTester:
    """
    Advanced regression testing for VexFS v2 Qdrant adapter.
    
    Features:
    - Baseline performance tracking
    - Automated regression detection
    - Performance trend analysis
    - Alert generation for degradations
    """
    
    def __init__(self, baseline_dir: str = "benchmarks/baselines"):
        self.baseline_dir = Path(baseline_dir)
        self.baseline_dir.mkdir(parents=True, exist_ok=True)
        self.thresholds = RegressionThresholds()
        
    async def save_baseline(self, results: List[BenchmarkResult], baseline_name: str = None) -> str:
        """
        Save benchmark results as baseline for future comparisons.
        
        Args:
            results: List of benchmark results to save as baseline
            baseline_name: Optional name for baseline, defaults to timestamp
            
        Returns:
            Path to saved baseline file
        """
        if baseline_name is None:
            baseline_name = f"baseline_{datetime.now().strftime('%Y%m%d_%H%M%S')}"
        
        baseline_data = {
            "metadata": {
                "name": baseline_name,
                "timestamp": datetime.now().isoformat(),
                "version": "2.0.0-phase4",
                "result_count": len(results)
            },
            "results": [asdict(result) for result in results],
            "summary": self._calculate_baseline_summary(results)
        }
        
        baseline_file = self.baseline_dir / f"{baseline_name}.json"
        
        with open(baseline_file, 'w') as f:
            json.dump(baseline_data, f, indent=2)
        
        logger.info(f"📊 Baseline saved: {baseline_file}")
        return str(baseline_file)
    
    async def compare_with_baseline(self, current_results: List[BenchmarkResult], baseline_file: str) -> BaselineComparison:
        """
        Compare current results with baseline and detect regressions.
        
        Args:
            current_results: Current benchmark results
            baseline_file: Path to baseline file
            
        Returns:
            Detailed comparison results
        """
        logger.info(f"🔍 Comparing with baseline: {baseline_file}")
        
        # Load baseline
        baseline_data = self._load_baseline(baseline_file)
        if not baseline_data:
            raise ValueError(f"Could not load baseline from {baseline_file}")
        
        baseline_results = [BenchmarkResult(**r) for r in baseline_data["results"]]
        
        # Perform comparison
        metric_comparisons = []
        
        # Compare each benchmark by name
        for current_result in current_results:
            baseline_result = self._find_matching_baseline(current_result, baseline_results)
            if baseline_result:
                comparisons = self._compare_benchmark_results(current_result, baseline_result)
                metric_comparisons.extend(comparisons)
        
        # Analyze overall regression status
        regressions = [c for c in metric_comparisons if c.is_regression]
        improvements = [c for c in metric_comparisons if c.change_percent < -5.0]  # 5% improvement threshold
        stable = [c for c in metric_comparisons if not c.is_regression and c.change_percent >= -5.0]
        
        overall_regression = len(regressions) > 0
        
        # Generate summary
        summary = {
            "total_metrics_compared": len(metric_comparisons),
            "regressions_detected": len(regressions),
            "improvements_detected": len(improvements),
            "stable_metrics": len(stable),
            "regression_rate": len(regressions) / len(metric_comparisons) if metric_comparisons else 0,
            "critical_regressions": len([r for r in regressions if r.severity == "critical"]),
            "high_regressions": len([r for r in regressions if r.severity == "high"]),
            "medium_regressions": len([r for r in regressions if r.severity == "medium"]),
            "low_regressions": len([r for r in regressions if r.severity == "low"])
        }
        
        return BaselineComparison(
            baseline_timestamp=baseline_data["metadata"]["timestamp"],
            current_timestamp=datetime.now().isoformat(),
            overall_regression_detected=overall_regression,
            regression_count=len(regressions),
            improvement_count=len(improvements),
            stable_count=len(stable),
            metric_comparisons=metric_comparisons,
            summary=summary
        )
    
    def _load_baseline(self, baseline_file: str) -> Optional[Dict[str, Any]]:
        """Load baseline data from file"""
        try:
            with open(baseline_file, 'r') as f:
                return json.load(f)
        except Exception as e:
            logger.error(f"Failed to load baseline {baseline_file}: {e}")
            return None
    
    def _find_matching_baseline(self, current_result: BenchmarkResult, baseline_results: List[BenchmarkResult]) -> Optional[BenchmarkResult]:
        """Find matching baseline result by name"""
        for baseline_result in baseline_results:
            if baseline_result.name == current_result.name:
                return baseline_result
        return None
    
    def _compare_benchmark_results(self, current: BenchmarkResult, baseline: BenchmarkResult) -> List[RegressionResult]:
        """Compare two benchmark results and detect regressions"""
        comparisons = []
        
        # Compare throughput
        if baseline.throughput_ops_sec > 0:
            throughput_change = ((current.throughput_ops_sec - baseline.throughput_ops_sec) / baseline.throughput_ops_sec) * 100
            is_regression = throughput_change < -self.thresholds.throughput_degradation_percent
            severity = self._calculate_severity(abs(throughput_change), self.thresholds.throughput_degradation_percent)
            
            comparisons.append(RegressionResult(
                metric_name=f"{current.name} - Throughput",
                baseline_value=baseline.throughput_ops_sec,
                current_value=current.throughput_ops_sec,
                change_percent=throughput_change,
                is_regression=is_regression,
                severity=severity,
                threshold_used=self.thresholds.throughput_degradation_percent
            ))
        
        # Compare duration (latency)
        if baseline.duration_ms > 0:
            duration_change = ((current.duration_ms - baseline.duration_ms) / baseline.duration_ms) * 100
            is_regression = duration_change > self.thresholds.latency_increase_percent
            severity = self._calculate_severity(abs(duration_change), self.thresholds.latency_increase_percent)
            
            comparisons.append(RegressionResult(
                metric_name=f"{current.name} - Latency",
                baseline_value=baseline.duration_ms,
                current_value=current.duration_ms,
                change_percent=duration_change,
                is_regression=is_regression,
                severity=severity,
                threshold_used=self.thresholds.latency_increase_percent
            ))
        
        # Compare memory usage
        if baseline.memory_usage_mb > 0:
            memory_change = ((current.memory_usage_mb - baseline.memory_usage_mb) / baseline.memory_usage_mb) * 100
            is_regression = memory_change > self.thresholds.memory_increase_percent
            severity = self._calculate_severity(abs(memory_change), self.thresholds.memory_increase_percent)
            
            comparisons.append(RegressionResult(
                metric_name=f"{current.name} - Memory Usage",
                baseline_value=baseline.memory_usage_mb,
                current_value=current.memory_usage_mb,
                change_percent=memory_change,
                is_regression=is_regression,
                severity=severity,
                threshold_used=self.thresholds.memory_increase_percent
            ))
        
        # Compare success rate
        if baseline.success_rate > 0:
            success_rate_change = ((current.success_rate - baseline.success_rate) / baseline.success_rate) * 100
            is_regression = success_rate_change < -self.thresholds.success_rate_decrease_percent
            severity = self._calculate_severity(abs(success_rate_change), self.thresholds.success_rate_decrease_percent)
            
            comparisons.append(RegressionResult(
                metric_name=f"{current.name} - Success Rate",
                baseline_value=baseline.success_rate,
                current_value=current.success_rate,
                change_percent=success_rate_change,
                is_regression=is_regression,
                severity=severity,
                threshold_used=self.thresholds.success_rate_decrease_percent
            ))
        
        return comparisons
    
    def _calculate_severity(self, change_percent: float, threshold: float) -> str:
        """Calculate severity based on change percentage"""
        if change_percent >= threshold * 3:
            return "critical"
        elif change_percent >= threshold * 2:
            return "high"
        elif change_percent >= threshold * 1.5:
            return "medium"
        elif change_percent >= threshold:
            return "low"
        else:
            return "none"
    
    def _calculate_baseline_summary(self, results: List[BenchmarkResult]) -> Dict[str, Any]:
        """Calculate summary statistics for baseline"""
        if not results:
            return {}
        
        throughputs = [r.throughput_ops_sec for r in results if r.throughput_ops_sec > 0]
        durations = [r.duration_ms for r in results if r.duration_ms > 0]
        memory_usages = [r.memory_usage_mb for r in results if r.memory_usage_mb > 0]
        success_rates = [r.success_rate for r in results if r.success_rate > 0]
        
        return {
            "benchmark_count": len(results),
            "avg_throughput_ops_sec": statistics.mean(throughputs) if throughputs else 0,
            "max_throughput_ops_sec": max(throughputs) if throughputs else 0,
            "avg_duration_ms": statistics.mean(durations) if durations else 0,
            "max_memory_usage_mb": max(memory_usages) if memory_usages else 0,
            "avg_success_rate": statistics.mean(success_rates) if success_rates else 0,
            "min_success_rate": min(success_rates) if success_rates else 0
        }
    
    async def analyze_performance_trends(self, baseline_files: List[str]) -> Dict[str, Any]:
        """
        Analyze performance trends across multiple baselines.
        
        Args:
            baseline_files: List of baseline files in chronological order
            
        Returns:
            Trend analysis results
        """
        logger.info(f"📈 Analyzing performance trends across {len(baseline_files)} baselines")
        
        baselines = []
        for baseline_file in baseline_files:
            baseline_data = self._load_baseline(baseline_file)
            if baseline_data:
                baselines.append(baseline_data)
        
        if len(baselines) < 2:
            return {"error": "Need at least 2 baselines for trend analysis"}
        
        # Extract metrics over time
        trends = {}
        
        for baseline in baselines:
            timestamp = baseline["metadata"]["timestamp"]
            
            for result_data in baseline["results"]:
                result = BenchmarkResult(**result_data)
                benchmark_name = result.name
                
                if benchmark_name not in trends:
                    trends[benchmark_name] = {
                        "timestamps": [],
                        "throughput": [],
                        "duration": [],
                        "memory": [],
                        "success_rate": []
                    }
                
                trends[benchmark_name]["timestamps"].append(timestamp)
                trends[benchmark_name]["throughput"].append(result.throughput_ops_sec)
                trends[benchmark_name]["duration"].append(result.duration_ms)
                trends[benchmark_name]["memory"].append(result.memory_usage_mb)
                trends[benchmark_name]["success_rate"].append(result.success_rate)
        
        # Analyze trends
        trend_analysis = {}
        
        for benchmark_name, data in trends.items():
            if len(data["throughput"]) < 2:
                continue
            
            # Calculate trend slopes (simple linear regression)
            throughput_trend = self._calculate_trend_slope(data["throughput"])
            duration_trend = self._calculate_trend_slope(data["duration"])
            memory_trend = self._calculate_trend_slope(data["memory"])
            success_rate_trend = self._calculate_trend_slope(data["success_rate"])
            
            trend_analysis[benchmark_name] = {
                "data_points": len(data["throughput"]),
                "throughput_trend": throughput_trend,
                "duration_trend": duration_trend,
                "memory_trend": memory_trend,
                "success_rate_trend": success_rate_trend,
                "overall_health": self._assess_overall_health(
                    throughput_trend, duration_trend, memory_trend, success_rate_trend
                )
            }
        
        return {
            "baselines_analyzed": len(baselines),
            "benchmarks_analyzed": len(trend_analysis),
            "trend_analysis": trend_analysis,
            "summary": self._summarize_trends(trend_analysis)
        }
    
    def _calculate_trend_slope(self, values: List[float]) -> Dict[str, float]:
        """Calculate trend slope using simple linear regression"""
        if len(values) < 2:
            return {"slope": 0, "direction": "stable"}
        
        # Remove zeros and invalid values
        valid_values = [v for v in values if v > 0]
        if len(valid_values) < 2:
            return {"slope": 0, "direction": "stable"}
        
        x = list(range(len(valid_values)))
        y = valid_values
        
        # Simple linear regression
        n = len(x)
        sum_x = sum(x)
        sum_y = sum(y)
        sum_xy = sum(x[i] * y[i] for i in range(n))
        sum_x2 = sum(x[i] ** 2 for i in range(n))
        
        if n * sum_x2 - sum_x ** 2 == 0:
            slope = 0
        else:
            slope = (n * sum_xy - sum_x * sum_y) / (n * sum_x2 - sum_x ** 2)
        
        # Determine direction
        if abs(slope) < 0.01:  # Threshold for "stable"
            direction = "stable"
        elif slope > 0:
            direction = "improving" if "throughput" in str(values) or "success_rate" in str(values) else "degrading"
        else:
            direction = "degrading" if "throughput" in str(values) or "success_rate" in str(values) else "improving"
        
        return {"slope": slope, "direction": direction}
    
    def _assess_overall_health(self, throughput_trend: Dict, duration_trend: Dict, 
                              memory_trend: Dict, success_rate_trend: Dict) -> str:
        """Assess overall health based on all trends"""
        positive_trends = 0
        negative_trends = 0
        
        # Throughput: higher is better
        if throughput_trend["direction"] == "improving":
            positive_trends += 1
        elif throughput_trend["direction"] == "degrading":
            negative_trends += 1
        
        # Duration: lower is better
        if duration_trend["direction"] == "improving":  # This means decreasing duration
            positive_trends += 1
        elif duration_trend["direction"] == "degrading":  # This means increasing duration
            negative_trends += 1
        
        # Memory: lower is better
        if memory_trend["direction"] == "improving":  # This means decreasing memory
            positive_trends += 1
        elif memory_trend["direction"] == "degrading":  # This means increasing memory
            negative_trends += 1
        
        # Success rate: higher is better
        if success_rate_trend["direction"] == "improving":
            positive_trends += 1
        elif success_rate_trend["direction"] == "degrading":
            negative_trends += 1
        
        if positive_trends > negative_trends:
            return "improving"
        elif negative_trends > positive_trends:
            return "degrading"
        else:
            return "stable"
    
    def _summarize_trends(self, trend_analysis: Dict[str, Any]) -> Dict[str, Any]:
        """Summarize overall trends across all benchmarks"""
        if not trend_analysis:
            return {}
        
        health_counts = {"improving": 0, "stable": 0, "degrading": 0}
        
        for benchmark_data in trend_analysis.values():
            health = benchmark_data["overall_health"]
            health_counts[health] += 1
        
        total_benchmarks = len(trend_analysis)
        
        return {
            "total_benchmarks": total_benchmarks,
            "improving_benchmarks": health_counts["improving"],
            "stable_benchmarks": health_counts["stable"],
            "degrading_benchmarks": health_counts["degrading"],
            "improvement_rate": health_counts["improving"] / total_benchmarks if total_benchmarks > 0 else 0,
            "degradation_rate": health_counts["degrading"] / total_benchmarks if total_benchmarks > 0 else 0,
            "overall_trend": max(health_counts, key=health_counts.get)
        }
    
    def generate_regression_report(self, comparison: BaselineComparison) -> str:
        """Generate human-readable regression report"""
        report = []
        report.append("=" * 80)
        report.append("PERFORMANCE REGRESSION ANALYSIS REPORT")
        report.append("=" * 80)
        report.append(f"Baseline: {comparison.baseline_timestamp}")
        report.append(f"Current:  {comparison.current_timestamp}")
        report.append("")
        
        # Overall status
        status = "🚨 REGRESSION DETECTED" if comparison.overall_regression_detected else "✅ NO REGRESSIONS"
        report.append(f"Overall Status: {status}")
        report.append("")
        
        # Summary
        report.append("SUMMARY:")
        report.append(f"  Total Metrics Compared: {comparison.summary['total_metrics_compared']}")
        report.append(f"  Regressions Detected: {comparison.regression_count}")
        report.append(f"  Improvements Detected: {comparison.improvement_count}")
        report.append(f"  Stable Metrics: {comparison.stable_count}")
        report.append("")
        
        # Regression details
        if comparison.regression_count > 0:
            report.append("REGRESSIONS DETECTED:")
            
            critical = [r for r in comparison.metric_comparisons if r.is_regression and r.severity == "critical"]
            high = [r for r in comparison.metric_comparisons if r.is_regression and r.severity == "high"]
            medium = [r for r in comparison.metric_comparisons if r.is_regression and r.severity == "medium"]
            low = [r for r in comparison.metric_comparisons if r.is_regression and r.severity == "low"]
            
            for severity, regressions in [("CRITICAL", critical), ("HIGH", high), ("MEDIUM", medium), ("LOW", low)]:
                if regressions:
                    report.append(f"  {severity} ({len(regressions)}):")
                    for regression in regressions:
                        report.append(f"    - {regression.metric_name}: {regression.change_percent:+.1f}% "
                                    f"(was {regression.baseline_value:.2f}, now {regression.current_value:.2f})")
            report.append("")
        
        # Improvements
        improvements = [r for r in comparison.metric_comparisons if r.change_percent < -5.0]
        if improvements:
            report.append("IMPROVEMENTS DETECTED:")
            for improvement in improvements:
                report.append(f"  + {improvement.metric_name}: {improvement.change_percent:+.1f}% "
                            f"(was {improvement.baseline_value:.2f}, now {improvement.current_value:.2f})")
            report.append("")
        
        report.append("=" * 80)
        
        return "\n".join(report)
    
    def list_available_baselines(self) -> List[Dict[str, Any]]:
        """List all available baseline files"""
        baselines = []
        
        for baseline_file in self.baseline_dir.glob("*.json"):
            try:
                baseline_data = self._load_baseline(str(baseline_file))
                if baseline_data:
                    baselines.append({
                        "file": str(baseline_file),
                        "name": baseline_data["metadata"]["name"],
                        "timestamp": baseline_data["metadata"]["timestamp"],
                        "result_count": baseline_data["metadata"]["result_count"]
                    })
            except Exception as e:
                logger.warning(f"Could not load baseline {baseline_file}: {e}")
        
        # Sort by timestamp
        baselines.sort(key=lambda x: x["timestamp"])
        
        return baselines