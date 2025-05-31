#!/usr/bin/env python3
"""
Simple VexFS FUSE test to isolate the hanging issue
"""

import os
import subprocess
import time
import tempfile
from pathlib import Path

def test_vexfs_simple():
    """Test basic VexFS operations to identify hanging point"""
    
    mount_point = Path("/tmp/vexfs_simple_test")
    vexfs_binary = Path("./vexfs_fuse")
    
    # Ensure mount point exists
    mount_point.mkdir(exist_ok=True)
    
    print("🧪 Starting VexFS FUSE simple test...")
    
    try:
        # 1. Start VexFS FUSE
        print("1. Starting VexFS FUSE process...")
        process = subprocess.Popen(
            [str(vexfs_binary.absolute()), str(mount_point), "-f"],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True
        )
        
        # Wait for mount
        time.sleep(2)
        print("✅ VexFS FUSE process started")
        
        # 2. Test basic ls (this might hang)
        print("2. Testing basic ls operation...")
        try:
            result = subprocess.run(
                ["ls", "-la", str(mount_point)],
                capture_output=True,
                text=True,
                timeout=5  # 5 second timeout
            )
            print(f"✅ ls completed: {result.stdout}")
        except subprocess.TimeoutExpired:
            print("❌ ls operation HUNG - this is the issue!")
            return False
        
        # 3. Test file creation (if ls worked)
        print("3. Testing file creation...")
        try:
            test_file = mount_point / "test.txt"
            # Use subprocess with timeout for file operations
            result = subprocess.run(
                ["touch", str(test_file)],
                capture_output=True,
                text=True,
                timeout=10
            )
            if result.returncode == 0:
                print("✅ File creation completed")
            else:
                print(f"❌ File creation failed: {result.stderr}")
                return False
        except subprocess.TimeoutExpired:
            print("❌ File creation HUNG - this is the issue!")
            return False
        except Exception as e:
            print(f"❌ File creation failed: {e}")
            return False
        
        # 4. Test file writing
        print("4. Testing file writing...")
        try:
            result = subprocess.run(
                ["sh", "-c", f"echo 'Hello VexFS' > {test_file}"],
                capture_output=True,
                text=True,
                timeout=10
            )
            if result.returncode == 0:
                print("✅ File writing completed")
            else:
                print(f"❌ File writing failed: {result.stderr}")
                return False
        except subprocess.TimeoutExpired:
            print("❌ File writing HUNG - this is the issue!")
            return False
        except Exception as e:
            print(f"❌ File writing failed: {e}")
            return False
        
        # 5. Test file reading
        print("5. Testing file reading...")
        try:
            result = subprocess.run(
                ["cat", str(test_file)],
                capture_output=True,
                text=True,
                timeout=10
            )
            if result.returncode == 0:
                print(f"✅ File reading completed: {result.stdout.strip()}")
            else:
                print(f"❌ File reading failed: {result.stderr}")
                return False
        except subprocess.TimeoutExpired:
            print("❌ File reading HUNG - this is the issue!")
            return False
        except Exception as e:
            print(f"❌ File reading failed: {e}")
            return False
        
        print("🎯 All basic operations completed successfully!")
        return True
        
    except Exception as e:
        print(f"❌ Test failed: {e}")
        return False
        
    finally:
        # Cleanup
        try:
            if 'process' in locals():
                process.terminate()
                process.wait(timeout=5)
        except:
            pass
        
        try:
            subprocess.run(["fusermount", "-u", str(mount_point)], 
                         capture_output=True, timeout=10)
        except:
            pass

if __name__ == "__main__":
    success = test_vexfs_simple()
    exit(0 if success else 1)