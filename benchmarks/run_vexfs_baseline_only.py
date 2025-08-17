#!/usr/bin/env python3
"""
VexFS FUSE Baseline Performance Test
===================================

Runs standalone VexFS FUSE performance testing without Docker dependencies.
Provides immediate performance metrics for customer deliverable.
"""

import os
import sys
import time
import logging
import tempfile
import subprocess
from pathlib import Path

# Add current directory to Python path
sys.path.insert(0, '.')

from vexfs_fuse_baseline import VexFSFUSEBenchmark
from datasets.dataset_loader import BenchmarkDatasetManager
from generate_executive_summary import ExecutiveSummaryGenerator

def setup_logging():
    """Configure logging for the benchmark run."""
    logging.basicConfig(
        level=logging.INFO,
        format='%(asctime)s - %(levelname)s - %(message)s',
        handlers=[
            logging.FileHandler('vexfs_baseline_run.log'),
            logging.StreamHandler()
        ]
    )
    return logging.getLogger(__name__)

def find_vexfs_binary():
    """Find the VexFS FUSE binary."""
    possible_paths = [
        '../rust/target/release/vexfs_fuse',
        '../rust/target/x86_64-unknown-linux-gnu/release/vexfs_fuse'
    ]
    
    for path in possible_paths:
        if os.path.exists(path):
            return os.path.abspath(path)
    
    raise FileNotFoundError("VexFS FUSE binary not found. Please build it first with: cd ../rust && cargo build --release --bin vexfs_fuse --features=fuse_support")

def main():
    """Run VexFS FUSE baseline performance testing."""
    logger = setup_logging()
    
    logger.info("🚀 Starting VexFS FUSE Baseline Performance Test")
    logger.info("=" * 60)
    
    try:
        # Find VexFS binary
        logger.info("📍 Locating VexFS FUSE binary...")
        vexfs_binary = find_vexfs_binary()
        logger.info(f"✅ Found VexFS binary: {vexfs_binary}")
        
        # Create temporary mount point
        with tempfile.TemporaryDirectory(prefix='vexfs_benchmark_') as temp_dir:
            mount_point = os.path.join(temp_dir, 'vexfs_mount')
            os.makedirs(mount_point, exist_ok=True)
            
            logger.info(f"📁 Created mount point: {mount_point}")
            
            # Initialize dataset manager
            logger.info("📊 Initializing dataset manager...")
            dataset_manager = BenchmarkDatasetManager()
            
            # Initialize VexFS benchmark
            logger.info("🔧 Initializing VexFS FUSE benchmark...")
            vexfs_benchmark = VexFSFUSEBenchmark(mount_point=mount_point)
            
            # Update the binary path to the correct location
            vexfs_benchmark.vexfs_binary = Path(vexfs_binary)
            
            # Run comprehensive baseline test
            logger.info("🏃 Running VexFS FUSE baseline performance test...")
            logger.info("-" * 40)
            
            results = vexfs_benchmark.run_benchmark_suite()
            
            logger.info("✅ Baseline test completed!")
            logger.info("=" * 60)
            
            # Display results
            logger.info("📈 PERFORMANCE RESULTS:")
            logger.info("-" * 30)
            
            for result in results:
                logger.info(f"\n🔹 {result.test_name.upper()}:")
                logger.info(f"  • Dataset: {result.dataset_size:,} vectors × {result.vector_dimension}D")
                logger.info(f"  • Insert Throughput: {result.insert_throughput:.1f} vectors/sec")
                logger.info(f"  • Insert Avg Latency: {result.insert_latency_avg:.2f} ms")
                logger.info(f"  • Query Throughput: {result.query_throughput:.1f} queries/sec")
                logger.info(f"  • Query Avg Latency: {result.query_latency_avg:.2f} ms")
                logger.info(f"  • Memory Usage: {result.memory_usage_mb:.1f} MB")
                logger.info(f"  • Accuracy: {result.accuracy_recall_at_10:.1%}")
            
            # Generate executive summary
            logger.info("\n📋 Generating executive summary...")
            
            # Create results directory
            results_dir = Path('results')
            results_dir.mkdir(exist_ok=True)
            
            # Save raw results
            import json
            from dataclasses import asdict
            results_data = [asdict(result) for result in results]
            with open(results_dir / 'vexfs_baseline_results.json', 'w') as f:
                json.dump(results_data, f, indent=2)
            
            # Generate executive summary (VexFS only)
            summary_generator = ExecutiveSummaryGenerator()
            
            # Create a simplified competitive results structure for VexFS only
            competitive_results = {
                'vexfs': results,
                'metadata': {
                    'test_date': time.strftime('%Y-%m-%d %H:%M:%S'),
                    'test_type': 'VexFS FUSE Baseline Only',
                    'note': 'Competitive comparison requires Docker environment setup'
                }
            }
            
            summary_path = summary_generator.generate_summary(
                competitive_results, 
                str(results_dir / 'vexfs_baseline_executive_summary.md')
            )
            
            logger.info(f"✅ Executive summary saved: {summary_path}")
            
            # Final summary
            logger.info("\n" + "=" * 60)
            logger.info("🎯 VEXFS FUSE BASELINE TEST COMPLETED")
            logger.info("=" * 60)
            logger.info(f"📊 Results saved to: {results_dir}")
            logger.info(f"📋 Executive summary: {summary_path}")
            logger.info("\n💡 Next steps:")
            logger.info("   • Review executive summary for customer presentation")
            logger.info("   • Set up Docker environment for competitive comparison")
            logger.info("   • Consider kernel module testing in VM environment")
            
            return True
            
    except Exception as e:
        logger.error(f"❌ Benchmark failed: {e}")
        logger.exception("Full error details:")
        return False

if __name__ == '__main__':
    success = main()
    sys.exit(0 if success else 1)