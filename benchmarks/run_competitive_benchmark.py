#!/usr/bin/env python3
"""
Real Competitive Benchmark Script
Generates actual performance data for customer comparisons
"""

import sys
import traceback
import json
import time
import os
from datetime import datetime

# Add current directory to path
sys.path.insert(0, '.')

def main():
    print('🚀 GENERATING REAL COMPETITIVE PERFORMANCE DATA')
    print('=' * 60)
    
    try:
        print('\n1. Testing database connections...')
        import chromadb
        from qdrant_client import QdrantClient
        
        # Test ChromaDB
        chroma_client = chromadb.HttpClient(host='localhost', port=8000)
        chroma_client.heartbeat()
        print('✅ ChromaDB: Connected')
        
        # Test Qdrant
        qdrant_client = QdrantClient(host='localhost', port=6333)
        collections = qdrant_client.get_collections()
        print('✅ Qdrant: Connected')
        
        print('\n2. Importing competitive analysis suite...')
        from competitive_analysis import CompetitiveBenchmarkSuite
        suite = CompetitiveBenchmarkSuite()
        print('✅ Suite initialized')
        
        print('\n3. Running competitive benchmarks...')
        print('   This generates real performance data for customer comparisons')
        
        # Execute with timeout and error handling
        start_time = time.time()
        results = suite.run_competitive_analysis()
        end_time = time.time()
        
        print(f'\n4. Benchmark execution completed in {end_time - start_time:.2f} seconds')
        
        if results:
            print(f'✅ Generated results for {len(results)} databases')
            
            # Save results with timestamp
            timestamp = datetime.now().strftime('%Y%m%d_%H%M%S')
            results_file = f'competitive_results_{timestamp}.json'
            
            # Convert results to serializable format
            results_data = []
            for result in results:
                if hasattr(result, '__dict__'):
                    results_data.append(result.__dict__)
                else:
                    results_data.append(str(result))
            
            with open(results_file, 'w') as f:
                json.dump(results_data, f, indent=2, default=str)
            print(f'✅ Results saved to {results_file}')
            
            # Also save as latest
            with open('competitive_results.json', 'w') as f:
                json.dump(results_data, f, indent=2, default=str)
            print('✅ Latest results saved to competitive_results.json')
            
            # Print summary
            databases = set()
            for result in results:
                if hasattr(result, 'database'):
                    databases.add(result.database)
            
            print(f'   📊 Generated results for {len(databases)} databases: {", ".join(databases)}')
            for result in results:
                if hasattr(result, 'database'):
                    print(f'   📊 {result.database}: {result.test_name}')
                    
            print('\n🎯 REAL PERFORMANCE DATA GENERATED!')
            print('   Ready for customer-facing executive summary')
            
            # Create results directory if it doesn't exist
            os.makedirs('results', exist_ok=True)
            
            # Copy results to results directory
            import shutil
            shutil.copy(results_file, f'results/{results_file}')
            print(f'✅ Results also saved to results/{results_file}')
            
            return True
        else:
            print('❌ No competitive results generated')
            return False
            
    except Exception as e:
        print(f'❌ Error: {e}')
        traceback.print_exc()
        return False

if __name__ == '__main__':
    success = main()
    sys.exit(0 if success else 1)