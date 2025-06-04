#!/usr/bin/env python3
"""
VexFS v2.0 Performance Results Analysis Script

This script analyzes the output from the VexFS v2.0 performance validation
framework and generates detailed reports, charts, and comparisons.

Features:
- Parse performance test logs
- Generate statistical summaries
- Create performance charts
- Compare against targets
- Identify performance bottlenecks
- Generate recommendations

Usage:
    python3 analyze_performance_results.py <log_file>
    python3 analyze_performance_results.py performance_results_*.log
"""

import sys
import re
import json
import argparse
from datetime import datetime
from typing import Dict, List, Tuple, Optional

class PerformanceAnalyzer:
    def __init__(self):
        self.test_results = []
        self.summary_stats = {}
        self.targets = {
            'ops_per_second': 100000.0,
            'avg_latency_us': 1000.0,
            'error_rate': 0.0
        }
    
    def parse_log_file(self, log_file: str) -> bool:
        """Parse performance test log file and extract results."""
        try:
            with open(log_file, 'r') as f:
                content = f.read()
            
            # Extract test results using regex patterns
            test_pattern = r'📊 Performance Results for (.+?):\n(.*?)(?=📊|🎉|$)'
            matches = re.findall(test_pattern, content, re.DOTALL)
            
            for test_name, result_block in matches:
                result = self._parse_result_block(test_name.strip(), result_block)
                if result:
                    self.test_results.append(result)
            
            # Extract summary statistics
            self._parse_summary_stats(content)
            
            return len(self.test_results) > 0
            
        except Exception as e:
            print(f"❌ Error parsing log file {log_file}: {e}")
            return False
    
    def _parse_result_block(self, test_name: str, block: str) -> Optional[Dict]:
        """Parse individual test result block."""
        try:
            result = {'test_name': test_name}
            
            # Extract configuration
            config_match = re.search(r'Configuration: (\d+) dimensions, (\d+) batch size, (\d+) iterations', block)
            if config_match:
                result['dimensions'] = int(config_match.group(1))
                result['batch_size'] = int(config_match.group(2))
                result['iterations'] = int(config_match.group(3))
            
            # Extract performance metrics
            metrics = {
                'ops_per_second': r'Operations per second: ([\d.]+) ops/sec',
                'avg_latency_us': r'Average latency: ([\d.]+) μs',
                'p95_latency_us': r'P95 latency: ([\d.]+) μs',
                'p99_latency_us': r'P99 latency: ([\d.]+) μs',
                'min_latency_us': r'Min latency: ([\d.]+) μs',
                'max_latency_us': r'Max latency: ([\d.]+) μs',
                'successful_operations': r'Successful operations: (\d+)',
                'failed_operations': r'Failed operations: (\d+)',
                'error_rate': r'Error rate: ([\d.]+)%',
                'memory_usage_kb': r'Peak memory usage: (\d+) KB'
            }
            
            for key, pattern in metrics.items():
                match = re.search(pattern, block)
                if match:
                    if key in ['successful_operations', 'failed_operations', 'memory_usage_kb']:
                        result[key] = int(match.group(1))
                    else:
                        result[key] = float(match.group(1))
            
            # Extract target validation
            target_patterns = {
                'ops_target_achieved': r'Ops/sec target.*?ACHIEVED',
                'latency_target_achieved': r'Latency target.*?ACHIEVED',
                'error_target_achieved': r'Error rate target.*?ACHIEVED'
            }
            
            for key, pattern in target_patterns.items():
                result[key] = bool(re.search(pattern, block))
            
            return result
            
        except Exception as e:
            print(f"⚠️  Warning: Failed to parse result block for {test_name}: {e}")
            return None
    
    def _parse_summary_stats(self, content: str) -> None:
        """Parse overall summary statistics."""
        try:
            summary_pattern = r'📊 Overall Statistics:\n(.*?)(?=\n🎉|\n📝|$)'
            match = re.search(summary_pattern, content, re.DOTALL)
            
            if match:
                summary_block = match.group(1)
                
                stats_patterns = {
                    'tests_completed': r'Tests completed: (\d+)/(\d+)',
                    'tests_achieving_targets': r'Tests achieving all targets: (\d+)/(\d+)',
                    'avg_ops_per_sec': r'Average ops/sec across tests: ([\d.]+)',
                    'avg_error_rate': r'Average error rate: ([\d.]+)%'
                }
                
                for key, pattern in stats_patterns.items():
                    match = re.search(pattern, summary_block)
                    if match:
                        if key in ['tests_completed', 'tests_achieving_targets']:
                            self.summary_stats[key] = (int(match.group(1)), int(match.group(2)))
                        else:
                            self.summary_stats[key] = float(match.group(1))
                            
        except Exception as e:
            print(f"⚠️  Warning: Failed to parse summary stats: {e}")
    
    def generate_analysis_report(self) -> str:
        """Generate comprehensive analysis report."""
        report = []
        report.append("🔍 VexFS v2.0 Performance Analysis Report")
        report.append("=" * 50)
        report.append(f"📅 Generated: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}")
        report.append(f"📊 Total tests analyzed: {len(self.test_results)}")
        report.append("")
        
        if not self.test_results:
            report.append("❌ No test results found to analyze")
            return "\n".join(report)
        
        # Performance summary
        report.append("📈 Performance Summary")
        report.append("-" * 25)
        
        ops_values = [r.get('ops_per_second', 0) for r in self.test_results if 'ops_per_second' in r]
        latency_values = [r.get('avg_latency_us', 0) for r in self.test_results if 'avg_latency_us' in r]
        error_values = [r.get('error_rate', 0) for r in self.test_results if 'error_rate' in r]
        
        if ops_values:
            report.append(f"⚡ Operations/sec - Min: {min(ops_values):.0f}, Max: {max(ops_values):.0f}, Avg: {sum(ops_values)/len(ops_values):.0f}")
        if latency_values:
            report.append(f"⏱️  Latency (μs) - Min: {min(latency_values):.2f}, Max: {max(latency_values):.2f}, Avg: {sum(latency_values)/len(latency_values):.2f}")
        if error_values:
            report.append(f"❌ Error rate (%) - Min: {min(error_values):.2f}, Max: {max(error_values):.2f}, Avg: {sum(error_values)/len(error_values):.2f}")
        
        report.append("")
        
        # Target achievement analysis
        report.append("🎯 Target Achievement Analysis")
        report.append("-" * 30)
        
        ops_achieved = sum(1 for r in self.test_results if r.get('ops_target_achieved', False))
        latency_achieved = sum(1 for r in self.test_results if r.get('latency_target_achieved', False))
        error_achieved = sum(1 for r in self.test_results if r.get('error_target_achieved', False))
        total_tests = len(self.test_results)
        
        report.append(f"✅ Ops/sec target (≥100K): {ops_achieved}/{total_tests} ({ops_achieved/total_tests*100:.1f}%)")
        report.append(f"✅ Latency target (<1ms): {latency_achieved}/{total_tests} ({latency_achieved/total_tests*100:.1f}%)")
        report.append(f"✅ Error rate target (0%): {error_achieved}/{total_tests} ({error_achieved/total_tests*100:.1f}%)")
        
        all_targets_achieved = sum(1 for r in self.test_results 
                                 if r.get('ops_target_achieved', False) and 
                                    r.get('latency_target_achieved', False) and 
                                    r.get('error_target_achieved', False))
        report.append(f"🏆 All targets achieved: {all_targets_achieved}/{total_tests} ({all_targets_achieved/total_tests*100:.1f}%)")
        report.append("")
        
        # Dimensional analysis
        report.append("📐 Performance by Dimensions")
        report.append("-" * 28)
        
        dim_groups = {}
        for result in self.test_results:
            dim = result.get('dimensions', 0)
            if dim not in dim_groups:
                dim_groups[dim] = []
            dim_groups[dim].append(result)
        
        for dim in sorted(dim_groups.keys()):
            results = dim_groups[dim]
            avg_ops = sum(r.get('ops_per_second', 0) for r in results) / len(results)
            avg_latency = sum(r.get('avg_latency_us', 0) for r in results) / len(results)
            avg_error = sum(r.get('error_rate', 0) for r in results) / len(results)
            
            report.append(f"📊 {dim}D vectors: {avg_ops:.0f} ops/sec, {avg_latency:.2f}μs latency, {avg_error:.2f}% errors ({len(results)} tests)")
        
        report.append("")
        
        # Batch size analysis
        report.append("📦 Performance by Batch Size")
        report.append("-" * 28)
        
        batch_groups = {}
        for result in self.test_results:
            batch = result.get('batch_size', 0)
            if batch not in batch_groups:
                batch_groups[batch] = []
            batch_groups[batch].append(result)
        
        for batch in sorted(batch_groups.keys()):
            results = batch_groups[batch]
            avg_ops = sum(r.get('ops_per_second', 0) for r in results) / len(results)
            avg_latency = sum(r.get('avg_latency_us', 0) for r in results) / len(results)
            
            report.append(f"📦 Batch size {batch}: {avg_ops:.0f} ops/sec, {avg_latency:.2f}μs latency ({len(results)} tests)")
        
        report.append("")
        
        # Performance bottlenecks
        report.append("🔍 Performance Bottlenecks & Recommendations")
        report.append("-" * 42)
        
        # Identify slow tests
        slow_tests = [r for r in self.test_results if r.get('avg_latency_us', 0) > 1000]
        if slow_tests:
            report.append(f"⚠️  {len(slow_tests)} tests exceeded 1ms latency target:")
            for test in slow_tests[:5]:  # Show top 5
                report.append(f"   • {test['test_name']}: {test.get('avg_latency_us', 0):.2f}μs")
        
        # Identify low throughput tests
        low_throughput = [r for r in self.test_results if r.get('ops_per_second', 0) < 100000]
        if low_throughput:
            report.append(f"⚠️  {len(low_throughput)} tests below 100K ops/sec target:")
            for test in low_throughput[:5]:  # Show top 5
                report.append(f"   • {test['test_name']}: {test.get('ops_per_second', 0):.0f} ops/sec")
        
        # Error analysis
        error_tests = [r for r in self.test_results if r.get('error_rate', 0) > 0]
        if error_tests:
            report.append(f"❌ {len(error_tests)} tests had errors:")
            for test in error_tests:
                report.append(f"   • {test['test_name']}: {test.get('error_rate', 0):.2f}% error rate")
        
        report.append("")
        
        # Recommendations
        report.append("💡 Recommendations")
        report.append("-" * 16)
        
        if all_targets_achieved == total_tests:
            report.append("🎉 Excellent! All performance targets achieved.")
            report.append("   Consider stress testing with larger datasets.")
        elif all_targets_achieved > total_tests * 0.8:
            report.append("✅ Good performance overall.")
            report.append("   Focus on optimizing high-dimensional operations.")
        else:
            report.append("⚠️  Performance improvements needed:")
            if ops_achieved < total_tests * 0.8:
                report.append("   • Optimize throughput for vector operations")
            if latency_achieved < total_tests * 0.8:
                report.append("   • Reduce latency in IOCTL processing")
            if error_achieved < total_tests * 0.8:
                report.append("   • Investigate and fix error conditions")
        
        return "\n".join(report)
    
    def export_json(self, filename: str) -> bool:
        """Export results to JSON format."""
        try:
            data = {
                'timestamp': datetime.now().isoformat(),
                'summary_stats': self.summary_stats,
                'test_results': self.test_results,
                'targets': self.targets
            }
            
            with open(filename, 'w') as f:
                json.dump(data, f, indent=2)
            
            return True
            
        except Exception as e:
            print(f"❌ Error exporting to JSON: {e}")
            return False

def main():
    parser = argparse.ArgumentParser(description='Analyze VexFS v2.0 performance test results')
    parser.add_argument('log_file', help='Performance test log file to analyze')
    parser.add_argument('--output', '-o', help='Output file for analysis report')
    parser.add_argument('--json', help='Export results to JSON file')
    
    args = parser.parse_args()
    
    analyzer = PerformanceAnalyzer()
    
    print(f"🔍 Analyzing performance results from: {args.log_file}")
    
    if not analyzer.parse_log_file(args.log_file):
        print("❌ Failed to parse log file")
        return 1
    
    print(f"✅ Successfully parsed {len(analyzer.test_results)} test results")
    
    # Generate analysis report
    report = analyzer.generate_analysis_report()
    
    if args.output:
        try:
            with open(args.output, 'w') as f:
                f.write(report)
            print(f"📄 Analysis report saved to: {args.output}")
        except Exception as e:
            print(f"❌ Error saving report: {e}")
    else:
        print("\n" + report)
    
    # Export to JSON if requested
    if args.json:
        if analyzer.export_json(args.json):
            print(f"📊 Results exported to JSON: {args.json}")
    
    return 0

if __name__ == '__main__':
    sys.exit(main())