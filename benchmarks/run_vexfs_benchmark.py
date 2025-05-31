#!/usr/bin/env python3
"""
VexFS FUSE Baseline Benchmark Script
Generates real VexFS performance data for customer comparisons
"""

import sys
import traceback
import json
import time
import os
import tempfile
import subprocess
from datetime import datetime

# Add current directory to path
sys.path.insert(0, '.')

def main():
    print('🚀 GENERATING REAL VEXFS PERFORMANCE DATA')
    print('=' * 60)
    
    try:
        print('\n1. Checking VexFS FUSE binary...')
        vexfs_binary = './vexfs_fuse'
        if not os.path.exists(vexfs_binary):
            print(f'❌ VexFS FUSE binary not found at {vexfs_binary}')
            return False
        print('✅ VexFS FUSE binary found')
        
        print('\n2. Importing VexFS benchmark suite...')
        from vexfs_fuse_baseline import VexFSFUSEBenchmark
        benchmark = VexFSFUSEBenchmark()
        print('✅ VexFS benchmark suite initialized')
        
        print('\n3. Running VexFS FUSE benchmarks...')
        print('   This generates real VexFS performance data')
        
        # Execute with timeout and error handling
        start_time = time.time()
        results = benchmark.run_benchmark_suite()
        end_time = time.time()
        
        print(f'\n4. Benchmark execution completed in {end_time - start_time:.2f} seconds')
        
        if results:
            print(f'✅ Generated VexFS performance results')
            
            # Save results with timestamp
            timestamp = datetime.now().strftime('%Y%m%d_%H%M%S')
            results_file = f'vexfs_results_{timestamp}.json'
            
            # Convert BenchmarkResult objects to dictionaries for JSON serialization
            results_data = [result.__dict__ for result in results]
            
            with open(results_file, 'w') as f:
                json.dump(results_data, f, indent=2, default=str)
            print(f'✅ Results saved to {results_file}')
            
            # Also save as latest
            with open('vexfs_results.json', 'w') as f:
                json.dump(results_data, f, indent=2, default=str)
            print('✅ Latest results saved to vexfs_results.json')
            
            # Print summary
            print(f'   📊 Generated {len(results)} test configurations:')
            for result in results:
                print(f'       {result.test_name}: {result.dataset_size} vectors, {result.vector_dimension}D')
                print(f'         Insert: {result.insert_throughput:.1f} vec/sec, Query: {result.query_throughput:.1f} q/sec')
                    
            print('\n🎯 REAL VEXFS PERFORMANCE DATA GENERATED!')
            print('   Ready for customer-facing comparisons')
            
            # Create results directory if it doesn't exist
            os.makedirs('results', exist_ok=True)
            
            # Copy results to results directory
            import shutil
            shutil.copy(results_file, f'results/{results_file}')
            print(f'✅ Results also saved to results/{results_file}')
            
            return True
        else:
            print('❌ No VexFS results generated')
            return False
            
    except Exception as e:
        print(f'❌ Error: {e}')
        traceback.print_exc()
        return False

if __name__ == '__main__':
    success = main()
    sys.exit(0 if success else 1)