#!/usr/bin/env python3
"""
VexFS Baseline Monitoring and Competitive Analysis Trigger
Monitors VexFS baseline completion and automatically triggers competitive analysis
"""

import time
import subprocess
import sys
import os
import json
from datetime import datetime

def check_vexfs_baseline_running():
    """Check if VexFS baseline test is still running"""
    try:
        result = subprocess.run(['ps', 'aux'], capture_output=True, text=True)
        return 'run_vexfs_baseline_only.py' in result.stdout
    except Exception as e:
        print(f"Error checking process status: {e}")
        return False

def check_vexfs_results_available():
    """Check if VexFS baseline results are available"""
    results_dir = "results"
    if not os.path.exists(results_dir):
        return False
    
    # Look for VexFS baseline results file
    for file in os.listdir(results_dir):
        if file.startswith('vexfs_baseline_') and file.endswith('.json'):
            return True
    return False

def run_competitive_analysis():
    """Execute the competitive analysis"""
    print("🚀 TRIGGERING COMPETITIVE ANALYSIS")
    print("=" * 50)
    
    try:
        # Activate virtual environment and run competitive benchmark
        cmd = ["python", "run_competitive_benchmark.py"]
        print(f"Executing: {' '.join(cmd)}")
        
        process = subprocess.Popen(
            cmd,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            universal_newlines=True,
            bufsize=1
        )
        
        # Stream output in real-time
        for line in iter(process.stdout.readline, ''):
            print(line.rstrip())
        
        process.wait()
        
        if process.returncode == 0:
            print("✅ Competitive analysis completed successfully!")
            return True
        else:
            print(f"❌ Competitive analysis failed with exit code: {process.returncode}")
            return False
            
    except Exception as e:
        print(f"❌ Error running competitive analysis: {e}")
        return False

def main():
    """Main monitoring loop"""
    print("🔍 VexFS BASELINE MONITORING & COMPETITIVE ANALYSIS TRIGGER")
    print("=" * 60)
    print(f"Started at: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}")
    print()
    
    # Initial status check
    if not check_vexfs_baseline_running():
        print("⚠️  VexFS baseline not detected as running")
        if check_vexfs_results_available():
            print("✅ VexFS results already available - triggering competitive analysis")
            run_competitive_analysis()
            return
        else:
            print("❌ No VexFS baseline running and no results available")
            print("   Please start VexFS baseline first: python run_vexfs_baseline_only.py")
            return
    
    print("✅ VexFS baseline detected as running")
    print("🔄 Monitoring for completion...")
    print()
    
    check_count = 0
    while True:
        check_count += 1
        current_time = datetime.now().strftime('%H:%M:%S')
        
        # Check if baseline is still running
        if not check_vexfs_baseline_running():
            print(f"[{current_time}] 🎯 VexFS baseline completed!")
            
            # Wait a moment for results to be written
            time.sleep(5)
            
            # Check if results are available
            if check_vexfs_results_available():
                print(f"[{current_time}] ✅ VexFS results detected - triggering competitive analysis")
                success = run_competitive_analysis()
                
                if success:
                    print()
                    print("🎉 CUSTOMER DELIVERABLE COMPLETE!")
                    print("=" * 40)
                    print("✅ VexFS FUSE baseline: COMPLETED")
                    print("✅ Competitive analysis: COMPLETED") 
                    print("✅ Executive summary: READY")
                    print()
                    print("📊 Customer-ready performance comparison available!")
                    print("   Check results/ directory for detailed reports")
                else:
                    print("❌ Competitive analysis failed - manual intervention required")
                
                break
            else:
                print(f"[{current_time}] ⚠️  VexFS baseline completed but no results found")
                print("   Waiting for results to be written...")
                time.sleep(10)
                continue
        
        # Status update every 5 checks (2.5 minutes)
        if check_count % 5 == 0:
            print(f"[{current_time}] 🔄 VexFS baseline still running (check #{check_count})")
        
        # Wait 30 seconds before next check
        time.sleep(30)

if __name__ == "__main__":
    try:
        main()
    except KeyboardInterrupt:
        print("\n⚠️  Monitoring interrupted by user")
        print("   VexFS baseline may still be running")
        print("   Run 'python run_competitive_benchmark.py' manually when baseline completes")
    except Exception as e:
        print(f"\n❌ Monitoring error: {e}")
        sys.exit(1)