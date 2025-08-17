#!/usr/bin/env python3
"""
VexFS Integration Test for Competitive Benchmarking

This script tests the VexFS FUSE implementation to ensure it works
with our benchmarking infrastructure before running full competitive analysis.
"""

import os
import sys
import time
import tempfile
import subprocess
import logging
from pathlib import Path
from typing import Optional

# Configure logging
logging.basicConfig(level=logging.INFO, format='%(asctime)s - %(levelname)s - %(message)s')
logger = logging.getLogger(__name__)

class VexFSIntegrationTest:
    """Test VexFS FUSE integration for benchmarking"""
    
    def __init__(self):
        self.mount_point = None
        self.vexfs_process = None
        self.temp_dir = None
        
    def setup_test_environment(self) -> bool:
        """Setup test environment with temporary mount point"""
        try:
            # Create temporary directory for mount point
            self.temp_dir = tempfile.mkdtemp(prefix="vexfs_test_")
            self.mount_point = Path(self.temp_dir) / "mount"
            self.mount_point.mkdir()
            
            logger.info(f"Created test mount point: {self.mount_point}")
            return True
            
        except Exception as e:
            logger.error(f"Failed to setup test environment: {e}")
            return False
    
    def build_vexfs_fuse(self) -> bool:
        """Build VexFS FUSE binary"""
        try:
            logger.info("Building VexFS FUSE binary...")
            
            # Change to rust directory and build
            rust_dir = Path(__file__).parent.parent / "rust"
            
            result = subprocess.run(
                ["cargo", "build", "--release", "--bin", "vexfs_fuse", "--features=fuse_support"],
                cwd=rust_dir,
                capture_output=True,
                text=True
            )
            
            if result.returncode != 0:
                logger.error(f"Failed to build VexFS FUSE: {result.stderr}")
                return False
            
            # Check if binary exists (try both possible locations)
            binary_paths = [
                rust_dir / "target" / "release" / "vexfs_fuse",
                rust_dir / "target" / "x86_64-unknown-linux-gnu" / "release" / "vexfs_fuse"
            ]
            
            binary_path = None
            for path in binary_paths:
                if path.exists():
                    binary_path = path
                    break
            
            if not binary_path:
                logger.error(f"VexFS FUSE binary not found in any of: {binary_paths}")
                return False
            
            logger.info("✅ VexFS FUSE binary built successfully")
            return True
            
        except Exception as e:
            logger.error(f"Failed to build VexFS FUSE: {e}")
            return False
    
    def start_vexfs_fuse(self) -> bool:
        """Start VexFS FUSE filesystem"""
        try:
            if not self.mount_point:
                logger.error("Mount point not set")
                return False
            
            # Path to VexFS FUSE binary (try both possible locations)
            rust_dir = Path(__file__).parent.parent / "rust"
            binary_paths = [
                rust_dir / "target" / "release" / "vexfs_fuse",
                rust_dir / "target" / "x86_64-unknown-linux-gnu" / "release" / "vexfs_fuse"
            ]
            
            binary_path = None
            for path in binary_paths:
                if path.exists():
                    binary_path = path
                    break
            
            if not binary_path:
                logger.error(f"VexFS FUSE binary not found in any of: {binary_paths}")
                return False
            
            logger.info(f"Starting VexFS FUSE at {self.mount_point}")
            
            # Start VexFS FUSE in foreground mode for testing
            self.vexfs_process = subprocess.Popen(
                [str(binary_path), str(self.mount_point), "-f", "-d"],
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True
            )
            
            # Wait a moment for mount to complete
            time.sleep(2)
            
            # Check if process is still running
            if self.vexfs_process.poll() is not None:
                stdout, stderr = self.vexfs_process.communicate()
                logger.error(f"VexFS FUSE failed to start: {stderr}")
                return False
            
            # Verify mount point is accessible
            if not self.mount_point.exists():
                logger.error("Mount point not accessible after starting VexFS")
                return False
            
            logger.info("✅ VexFS FUSE started successfully")
            return True
            
        except Exception as e:
            logger.error(f"Failed to start VexFS FUSE: {e}")
            return False
    
    def test_basic_filesystem_operations(self) -> bool:
        """Test basic filesystem operations"""
        try:
            logger.info("Testing basic filesystem operations...")
            
            # Test 1: List directory
            files = list(self.mount_point.iterdir())
            logger.info(f"Mount point contents: {files}")
            
            # Test 2: Create a simple file
            test_file = self.mount_point / "test.txt"
            test_content = "Hello VexFS!"
            
            with open(test_file, 'w') as f:
                f.write(test_content)
            
            # Test 3: Read the file back
            with open(test_file, 'r') as f:
                read_content = f.read()
            
            if read_content != test_content:
                logger.error(f"File content mismatch: expected '{test_content}', got '{read_content}'")
                return False
            
            logger.info("✅ Basic filesystem operations work")
            return True
            
        except Exception as e:
            logger.error(f"Basic filesystem operations failed: {e}")
            return False
    
    def test_vector_file_operations(self) -> bool:
        """Test vector file operations"""
        try:
            logger.info("Testing vector file operations...")
            
            # Test 1: Create a vector file
            vector_file = self.mount_point / "test_vector.vec"
            vector_data = "0.1,0.2,0.3,0.4,0.5"
            
            with open(vector_file, 'w') as f:
                f.write(vector_data)
            
            # Test 2: Read vector file back
            with open(vector_file, 'r') as f:
                read_vector = f.read()
            
            if read_vector != vector_data:
                logger.error(f"Vector file content mismatch: expected '{vector_data}', got '{read_vector}'")
                return False
            
            # Test 3: Create multiple vector files
            for i in range(5):
                vec_file = self.mount_point / f"vector_{i}.vec"
                vec_content = ",".join([str(0.1 * j + i * 0.01) for j in range(10)])
                
                with open(vec_file, 'w') as f:
                    f.write(vec_content)
            
            # Test 4: List all vector files
            vector_files = list(self.mount_point.glob("*.vec"))
            logger.info(f"Created {len(vector_files)} vector files")
            
            if len(vector_files) < 6:  # Should have at least 6 vector files
                logger.error(f"Expected at least 6 vector files, found {len(vector_files)}")
                return False
            
            logger.info("✅ Vector file operations work")
            return True
            
        except Exception as e:
            logger.error(f"Vector file operations failed: {e}")
            return False
    
    def test_performance_baseline(self) -> bool:
        """Test basic performance characteristics"""
        try:
            logger.info("Testing performance baseline...")
            
            # Test write performance
            start_time = time.time()
            
            for i in range(100):
                test_file = self.mount_point / f"perf_test_{i}.txt"
                content = f"Performance test file {i} with some content to measure write speed"
                
                with open(test_file, 'w') as f:
                    f.write(content)
            
            write_time = time.time() - start_time
            write_throughput = 100 / write_time
            
            logger.info(f"Write performance: {write_throughput:.1f} files/second")
            
            # Test read performance
            start_time = time.time()
            
            for i in range(100):
                test_file = self.mount_point / f"perf_test_{i}.txt"
                with open(test_file, 'r') as f:
                    content = f.read()
            
            read_time = time.time() - start_time
            read_throughput = 100 / read_time
            
            logger.info(f"Read performance: {read_throughput:.1f} files/second")
            
            # Basic sanity check - should be able to do at least 10 ops/sec
            if write_throughput < 10 or read_throughput < 10:
                logger.warning("Performance seems low, but continuing...")
            
            logger.info("✅ Performance baseline test completed")
            return True
            
        except Exception as e:
            logger.error(f"Performance baseline test failed: {e}")
            return False
    
    def cleanup(self):
        """Cleanup test environment"""
        try:
            logger.info("Cleaning up test environment...")
            
            # Stop VexFS FUSE process
            if self.vexfs_process:
                self.vexfs_process.terminate()
                try:
                    self.vexfs_process.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    self.vexfs_process.kill()
                    self.vexfs_process.wait()
            
            # Unmount if still mounted
            if self.mount_point and self.mount_point.exists():
                try:
                    subprocess.run(["fusermount", "-u", str(self.mount_point)], 
                                 capture_output=True, timeout=10)
                except:
                    pass  # Ignore unmount errors
            
            # Remove temporary directory
            if self.temp_dir:
                import shutil
                shutil.rmtree(self.temp_dir, ignore_errors=True)
            
            logger.info("✅ Cleanup completed")
            
        except Exception as e:
            logger.error(f"Cleanup failed: {e}")
    
    def run_integration_test(self) -> bool:
        """Run complete integration test"""
        try:
            logger.info("🚀 Starting VexFS Integration Test")
            logger.info("=" * 50)
            
            # Setup
            if not self.setup_test_environment():
                return False
            
            # Build VexFS FUSE
            if not self.build_vexfs_fuse():
                return False
            
            # Start VexFS FUSE
            if not self.start_vexfs_fuse():
                return False
            
            # Run tests
            tests = [
                ("Basic Filesystem Operations", self.test_basic_filesystem_operations),
                ("Vector File Operations", self.test_vector_file_operations),
                ("Performance Baseline", self.test_performance_baseline),
            ]
            
            for test_name, test_func in tests:
                logger.info(f"Running: {test_name}")
                if not test_func():
                    logger.error(f"❌ Test failed: {test_name}")
                    return False
                logger.info(f"✅ Test passed: {test_name}")
            
            logger.info("=" * 50)
            logger.info("🎉 All integration tests passed!")
            logger.info("VexFS FUSE is ready for competitive benchmarking")
            return True
            
        except Exception as e:
            logger.error(f"Integration test failed: {e}")
            return False
        
        finally:
            self.cleanup()

def main():
    """Main execution function"""
    import argparse
    
    parser = argparse.ArgumentParser(description="VexFS Integration Test")
    parser.add_argument("--verbose", "-v", action="store_true", help="Enable verbose logging")
    
    args = parser.parse_args()
    
    if args.verbose:
        logging.getLogger().setLevel(logging.DEBUG)
    
    # Check prerequisites
    if os.geteuid() == 0:
        logger.warning("Running as root - this may cause permission issues")
    
    # Check if FUSE is available
    try:
        subprocess.run(["fusermount", "--version"], capture_output=True, check=True)
    except (subprocess.CalledProcessError, FileNotFoundError):
        logger.error("FUSE not available. Install with: sudo apt-get install fuse")
        return 1
    
    # Run integration test
    test = VexFSIntegrationTest()
    
    try:
        success = test.run_integration_test()
        
        if success:
            print("\n✅ VexFS integration test PASSED")
            print("🚀 Ready to run competitive benchmarks!")
            return 0
        else:
            print("\n❌ VexFS integration test FAILED")
            print("🔧 Check the logs above for details")
            return 1
            
    except KeyboardInterrupt:
        print("\n⚠️ Test interrupted by user")
        test.cleanup()
        return 1
    except Exception as e:
        print(f"\n💥 Unexpected error: {e}")
        test.cleanup()
        return 1

if __name__ == "__main__":
    exit(main())