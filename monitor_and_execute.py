#!/usr/bin/env python3
"""
Monitor VexFS baseline completion and automatically trigger competitive analysis
for immediate customer delivery
"""

import time
import subprocess
import sys
import os
from datetime import datetime

def log_message(msg):
    timestamp = datetime.now().strftime("%Y-%m-%d %H:%M:%S")
    print(f"[{timestamp}] {msg}")
    sys.stdout.flush()

def is_vexfs_baseline_running():
    """Check if VexFS baseline is still running"""
    try:
        result = subprocess.run(['ps', 'aux'], capture_output=True, text=True)
        return 'run_vexfs_baseline_only.py' in result.stdout
    except:
        return False

def check_competitive_readiness():
    """Verify competitive analysis environment is ready"""
    try:
        # Check Docker containers
        result = subprocess.run(['docker-compose', 'ps'], capture_output=True, text=True, cwd='/home/luis/Development/oss/vexfs/benchmarks')
        if 'benchmark_chromadb' not in result.stdout or 'benchmark_qdrant' not in result.stdout:
            return False, "Docker containers not running"
        
        # Check Python environment
        result = subprocess.run(['python3', '-c', 'import chromadb, qdrant_client; print("OK")'], 
                              capture_output=True, text=True, cwd='/home/luis/Development/oss/vexfs/benchmarks')
        if result.returncode != 0:
            return False, "Python dependencies not available"
        
        return True, "All systems ready"
    except Exception as e:
        return False, f"Error checking readiness: {e}"

def execute_competitive_analysis():
    """Execute competitive analysis immediately"""
    log_message("🚀 EXECUTING COMPETITIVE ANALYSIS")
    
    try:
        # Change to benchmarks directory
        os.chdir('/home/luis/Development/oss/vexfs/benchmarks')
        
        # Activate virtual environment and run competitive benchmark
        cmd = ['bash', '-c', 'source venv/bin/activate && python run_competitive_benchmark.py']
        
        log_message("Starting competitive benchmark execution...")
        process = subprocess.Popen(cmd, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)
        
        # Stream output in real-time
        for line in iter(process.stdout.readline, ''):
            print(line.rstrip())
            sys.stdout.flush()
        
        process.wait()
        
        if process.returncode == 0:
            log_message("✅ COMPETITIVE ANALYSIS COMPLETED SUCCESSFULLY")
            log_message("🎯 CUSTOMER RESULTS READY FOR DELIVERY")
        else:
            log_message(f"❌ Competitive analysis failed with exit code {process.returncode}")
            
    except Exception as e:
        log_message(f"❌ Error executing competitive analysis: {e}")

def main():
    log_message("🔍 MONITORING VEXFS BASELINE FOR COMPLETION")
    log_message("📊 Will automatically trigger competitive analysis when baseline completes")
    
    # Initial readiness check
    ready, msg = check_competitive_readiness()
    if ready:
        log_message(f"✅ Competitive environment ready: {msg}")
    else:
        log_message(f"❌ Competitive environment not ready: {msg}")
        return 1
    
    # Monitor VexFS baseline
    while True:
        if is_vexfs_baseline_running():
            log_message("⏳ VexFS baseline still running... (checking every 30 seconds)")
            time.sleep(30)
        else:
            log_message("🎉 VEXFS BASELINE COMPLETED!")
            log_message("🚀 TRIGGERING IMMEDIATE COMPETITIVE ANALYSIS")
            execute_competitive_analysis()
            break
    
    return 0

if __name__ == "__main__":
    sys.exit(main())
