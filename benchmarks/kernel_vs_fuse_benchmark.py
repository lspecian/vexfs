#!/usr/bin/env python3
"""
VexFS Kernel Module vs FUSE Performance Benchmark
Comprehensive comparison between kernel module and FUSE implementations
"""

import os
import sys
import time
import json
import subprocess
import tempfile
import shutil
from pathlib import Path

class VexFSBenchmark:
    def __init__(self):
        self.results = []
        self.temp_dirs = []
        
    def cleanup(self):
        """Clean up temporary directories and mounts"""
        for temp_dir in self.temp_dirs:
            try:
                if os.path.ismount(temp_dir):
                    subprocess.run(['sudo', 'umount', temp_dir], check=False)
                if os.path.exists(temp_dir):
                    shutil.rmtree(temp_dir)
            except Exception as e:
                print(f"Cleanup warning: {e}")
    
    def test_kernel_module_mount(self):
        """Test if kernel module can mount successfully"""
        try:
            # Check if vexfs is registered
            result = subprocess.run(['cat', '/proc/filesystems'], capture_output=True, text=True)
            if 'vexfs' not in result.stdout:
                return False, "VexFS not registered in /proc/filesystems"
            
            # Create temporary mount point
            mount_point = tempfile.mkdtemp(prefix='vexfs_kernel_test_')
            self.temp_dirs.append(mount_point)
            
            # Create loop device for testing
            loop_file = f"{mount_point}_loop.img"
            subprocess.run(['dd', 'if=/dev/zero', f'of={loop_file}', 'bs=1M', 'count=100'], 
                         check=True, capture_output=True)
            
            # Setup loop device
            loop_result = subprocess.run(['sudo', 'losetup', '-f', '--show', loop_file], 
                                       capture_output=True, text=True, check=True)
            loop_device = loop_result.stdout.strip()
            
            try:
                # Attempt to mount
                mount_result = subprocess.run(['sudo', 'mount', '-t', 'vexfs_fixed', loop_device, mount_point],
                                            capture_output=True, text=True, timeout=10)
                
                if mount_result.returncode == 0:
                    # Successfully mounted, test basic operations
                    test_file = os.path.join(mount_point, 'test.txt')
                    subprocess.run(['sudo', 'touch', test_file], check=True)
                    subprocess.run(['sudo', 'umount', mount_point], check=True)
                    return True, "Kernel module mount successful"
                else:
                    return False, f"Mount failed: {mount_result.stderr}"
                    
            finally:
                # Cleanup loop device
                subprocess.run(['sudo', 'losetup', '-d', loop_device], check=False)
                os.unlink(loop_file)
                
        except Exception as e:
            return False, f"Kernel module test error: {e}"
    
    def benchmark_fuse_implementation(self):
        """Benchmark FUSE implementation"""
        print("🔍 Testing FUSE implementation...")
        
        # Check if FUSE binary exists
        fuse_binary = "benchmarks/vexfs_fuse"
        if not os.path.exists(fuse_binary):
            return {"error": "FUSE binary not found"}
        
        # Create mount point
        mount_point = tempfile.mkdtemp(prefix='vexfs_fuse_bench_')
        self.temp_dirs.append(mount_point)
        
        try:
            # Start FUSE filesystem
            fuse_process = subprocess.Popen([fuse_binary, mount_point], 
                                          stdout=subprocess.PIPE, stderr=subprocess.PIPE)
            time.sleep(2)  # Allow mount to complete
            
            # Test basic file operations
            start_time = time.time()
            
            # File creation benchmark
            file_ops = 1000
            create_times = []
            
            for i in range(file_ops):
                file_start = time.time()
                test_file = os.path.join(mount_point, f'test_{i}.txt')
                with open(test_file, 'w') as f:
                    f.write(f'test data {i}' * 100)  # ~1KB per file
                create_times.append(time.time() - file_start)
            
            # Read benchmark
            read_times = []
            for i in range(min(100, file_ops)):
                read_start = time.time()
                test_file = os.path.join(mount_point, f'test_{i}.txt')
                with open(test_file, 'r') as f:
                    content = f.read()
                read_times.append(time.time() - read_start)
            
            total_time = time.time() - start_time
            
            # Calculate metrics
            avg_create_latency = sum(create_times) / len(create_times)
            avg_read_latency = sum(read_times) / len(read_times)
            create_throughput = file_ops / sum(create_times)
            read_throughput = len(read_times) / sum(read_times)
            
            return {
                "implementation": "FUSE",
                "file_operations": file_ops,
                "avg_create_latency_ms": avg_create_latency * 1000,
                "avg_read_latency_ms": avg_read_latency * 1000,
                "create_throughput_ops_sec": create_throughput,
                "read_throughput_ops_sec": read_throughput,
                "total_time_sec": total_time,
                "status": "success"
            }
            
        except Exception as e:
            return {"error": f"FUSE benchmark failed: {e}"}
        finally:
            try:
                fuse_process.terminate()
                fuse_process.wait(timeout=5)
            except:
                fuse_process.kill()
            subprocess.run(['fusermount', '-u', mount_point], check=False)
    
    def benchmark_kernel_module(self):
        """Benchmark kernel module implementation"""
        print("🔍 Testing kernel module implementation...")
        
        # First check if kernel module can mount
        can_mount, mount_msg = self.test_kernel_module_mount()
        
        if not can_mount:
            return {
                "implementation": "Kernel Module",
                "status": "failed",
                "error": mount_msg,
                "note": "Kernel module cannot mount - likely needs reboot with fixed module"
            }
        
        # If mount works, run performance benchmark
        mount_point = tempfile.mkdtemp(prefix='vexfs_kernel_bench_')
        self.temp_dirs.append(mount_point)
        
        try:
            # Create loop device for testing
            loop_file = f"{mount_point}_loop.img"
            subprocess.run(['dd', 'if=/dev/zero', f'of={loop_file}', 'bs=1M', 'count=100'], 
                         check=True, capture_output=True)
            
            loop_result = subprocess.run(['sudo', 'losetup', '-f', '--show', loop_file], 
                                       capture_output=True, text=True, check=True)
            loop_device = loop_result.stdout.strip()
            
            try:
                # Mount filesystem
                subprocess.run(['sudo', 'mount', '-t', 'vexfs_fixed', loop_device, mount_point],
                             check=True, capture_output=True)
                
                # File operations benchmark
                start_time = time.time()
                file_ops = 1000
                create_times = []
                
                for i in range(file_ops):
                    file_start = time.time()
                    test_file = os.path.join(mount_point, f'test_{i}.txt')
                    subprocess.run(['sudo', 'bash', '-c', f'echo "test data {i}" > {test_file}'], 
                                 check=True, capture_output=True)
                    create_times.append(time.time() - file_start)
                
                # Read benchmark
                read_times = []
                for i in range(min(100, file_ops)):
                    read_start = time.time()
                    test_file = os.path.join(mount_point, f'test_{i}.txt')
                    subprocess.run(['sudo', 'cat', test_file], 
                                 check=True, capture_output=True)
                    read_times.append(time.time() - read_start)
                
                total_time = time.time() - start_time
                
                # Calculate metrics
                avg_create_latency = sum(create_times) / len(create_times)
                avg_read_latency = sum(read_times) / len(read_times)
                create_throughput = file_ops / sum(create_times)
                read_throughput = len(read_times) / sum(read_times)
                
                # Unmount
                subprocess.run(['sudo', 'umount', mount_point], check=True)
                
                return {
                    "implementation": "Kernel Module",
                    "file_operations": file_ops,
                    "avg_create_latency_ms": avg_create_latency * 1000,
                    "avg_read_latency_ms": avg_read_latency * 1000,
                    "create_throughput_ops_sec": create_throughput,
                    "read_throughput_ops_sec": read_throughput,
                    "total_time_sec": total_time,
                    "status": "success"
                }
                
            finally:
                subprocess.run(['sudo', 'losetup', '-d', loop_device], check=False)
                os.unlink(loop_file)
                
        except Exception as e:
            return {
                "implementation": "Kernel Module",
                "status": "failed",
                "error": f"Kernel benchmark failed: {e}"
            }
    
    def run_comprehensive_benchmark(self):
        """Run comprehensive benchmark comparing both implementations"""
        print("🚀 VexFS Kernel Module vs FUSE Performance Benchmark")
        print("=" * 60)
        
        results = {
            "timestamp": time.strftime("%Y-%m-%d %H:%M:%S"),
            "benchmarks": []
        }
        
        # Test FUSE implementation
        fuse_result = self.benchmark_fuse_implementation()
        results["benchmarks"].append(fuse_result)
        
        # Test kernel module implementation
        kernel_result = self.benchmark_kernel_module()
        results["benchmarks"].append(kernel_result)
        
        # Print results
        print("\n📊 BENCHMARK RESULTS")
        print("=" * 40)
        
        for result in results["benchmarks"]:
            impl_name = result.get('implementation', 'Unknown')
            print(f"\n🔧 {impl_name}:")
            if result.get("status") == "success":
                print(f"  ✅ Create Throughput: {result['create_throughput_ops_sec']:.1f} ops/sec")
                print(f"  ✅ Read Throughput: {result['read_throughput_ops_sec']:.1f} ops/sec")
                print(f"  ⏱️  Avg Create Latency: {result['avg_create_latency_ms']:.2f} ms")
                print(f"  ⏱️  Avg Read Latency: {result['avg_read_latency_ms']:.2f} ms")
            else:
                print(f"  ❌ Status: {result.get('error', 'Failed')}")
                if result.get('note'):
                    print(f"  📝 Note: {result['note']}")
        
        # Save results
        output_file = f"benchmarks/kernel_vs_fuse_results_{int(time.time())}.json"
        with open(output_file, 'w') as f:
            json.dump(results, f, indent=2)
        
        print(f"\n💾 Results saved to: {output_file}")
        
        return results

def main():
    benchmark = VexFSBenchmark()
    try:
        results = benchmark.run_comprehensive_benchmark()
        
        # Print summary
        print("\n🎯 PERFORMANCE SUMMARY")
        print("=" * 30)
        
        fuse_success = any(r.get("status") == "success" and r.get("implementation") == "FUSE" 
                          for r in results["benchmarks"])
        kernel_success = any(r.get("status") == "success" and r.get("implementation") == "Kernel Module" 
                           for r in results["benchmarks"])
        
        if fuse_success and kernel_success:
            fuse_result = next(r for r in results["benchmarks"] 
                             if r.get("implementation") == "FUSE" and r.get("status") == "success")
            kernel_result = next(r for r in results["benchmarks"] 
                               if r.get("implementation") == "Kernel Module" and r.get("status") == "success")
            
            speedup = kernel_result["create_throughput_ops_sec"] / fuse_result["create_throughput_ops_sec"]
            print(f"🏆 Kernel Module Speedup: {speedup:.2f}x faster than FUSE")
        elif fuse_success:
            print("✅ FUSE implementation working")
            print("⚠️  Kernel module needs reboot with fixed version")
        else:
            print("❌ Both implementations need attention")
            
    finally:
        benchmark.cleanup()

if __name__ == "__main__":
    main()