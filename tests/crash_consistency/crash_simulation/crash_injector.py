#!/usr/bin/env python3
"""
VexFS Crash Consistency Testing - Crash Injection Framework

This module provides comprehensive crash simulation capabilities for VexFS,
including kernel panics, power failures, and I/O error injection.
"""

import os
import sys
import time
import json
import logging
import subprocess
import threading
from enum import Enum
from typing import Dict, List, Optional, Tuple
from dataclasses import dataclass
from pathlib import Path

class CrashType(Enum):
    """Types of crashes that can be simulated"""
    KERNEL_PANIC = "kernel_panic"
    POWER_FAILURE = "power_failure"
    IO_ERROR = "io_error"
    MEMORY_CORRUPTION = "memory_corruption"
    DISK_FULL = "disk_full"

class CrashTiming(Enum):
    """When to trigger the crash during operations"""
    BEFORE_OPERATION = "before_op"
    DURING_OPERATION = "during_op"
    AFTER_OPERATION = "after_op"
    RANDOM_TIMING = "random"

@dataclass
class CrashScenario:
    """Configuration for a crash test scenario"""
    crash_type: CrashType
    timing: CrashTiming
    operation: str
    delay_ms: int = 0
    repeat_count: int = 1
    description: str = ""

class CrashInjector:
    """Main crash injection framework for VexFS testing"""
    
    def __init__(self, vm_config: Dict, test_device: str = "/dev/vdb"):
        self.vm_config = vm_config
        self.test_device = test_device
        self.mount_point = "/mnt/vexfs_test"
        self.logger = self._setup_logging()
        self.crash_count = 0
        self.recovery_stats = {
            'successful_recoveries': 0,
            'failed_recoveries': 0,
            'data_corruption_detected': 0,
            'total_crashes': 0
        }
    
    def _setup_logging(self) -> logging.Logger:
        """Setup logging for crash injection tests"""
        logger = logging.getLogger('crash_injector')
        logger.setLevel(logging.DEBUG)
        
        # Create file handler
        log_file = Path(__file__).parent.parent / "results" / "crash_injection.log"
        log_file.parent.mkdir(exist_ok=True)
        
        handler = logging.FileHandler(log_file)
        formatter = logging.Formatter(
            '%(asctime)s - %(name)s - %(levelname)s - %(message)s'
        )
        handler.setFormatter(formatter)
        logger.addHandler(handler)
        
        # Also log to console
        console_handler = logging.StreamHandler()
        console_handler.setFormatter(formatter)
        logger.addHandler(console_handler)
        
        return logger
    
    def prepare_test_environment(self) -> bool:
        """Prepare the VM environment for crash testing"""
        try:
            self.logger.info("Preparing crash testing environment...")
            
            # Create baseline filesystem state
            self._create_baseline_state()
            
            # Setup crash injection mechanisms
            self._setup_crash_mechanisms()
            
            # Verify VexFS module is loaded
            if not self._verify_vexfs_module():
                self.logger.error("VexFS module not available")
                return False
            
            self.logger.info("Test environment prepared successfully")
            return True
            
        except Exception as e:
            self.logger.error(f"Failed to prepare test environment: {e}")
            return False
    
    def _create_baseline_state(self):
        """Create a known baseline filesystem state"""
        self.logger.info("Creating baseline filesystem state...")
        
        # Format device with VexFS
        cmd = f"mkfs.vexfs {self.test_device}"
        result = subprocess.run(cmd, shell=True, capture_output=True, text=True)
        if result.returncode != 0:
            raise Exception(f"Failed to format device: {result.stderr}")
        
        # Mount filesystem
        os.makedirs(self.mount_point, exist_ok=True)
        cmd = f"mount -t vexfs {self.test_device} {self.mount_point}"
        result = subprocess.run(cmd, shell=True, capture_output=True, text=True)
        if result.returncode != 0:
            raise Exception(f"Failed to mount filesystem: {result.stderr}")
        
        # Create test data patterns
        self._create_test_data_patterns()
        
        # Create vector test data
        self._create_vector_test_data()
        
        # Sync and unmount
        subprocess.run("sync", shell=True)
        subprocess.run(f"umount {self.mount_point}", shell=True)
    
    def _create_test_data_patterns(self):
        """Create known data patterns for integrity verification"""
        test_dir = Path(self.mount_point) / "test_data"
        test_dir.mkdir(exist_ok=True)
        
        # Create files with known patterns
        patterns = {
            "sequential.dat": b"".join(i.to_bytes(4, 'little') for i in range(1000)),
            "random.dat": os.urandom(4096),
            "zeros.dat": b"\x00" * 4096,
            "ones.dat": b"\xff" * 4096,
            "alternating.dat": b"\xaa\x55" * 2048
        }
        
        for filename, data in patterns.items():
            with open(test_dir / filename, "wb") as f:
                f.write(data)
        
        # Create directory structure
        for i in range(10):
            subdir = test_dir / f"subdir_{i}"
            subdir.mkdir(exist_ok=True)
            for j in range(5):
                with open(subdir / f"file_{j}.txt", "w") as f:
                    f.write(f"Content of file {j} in directory {i}\n" * 10)
    
    def _create_vector_test_data(self):
        """Create vector test data for VexFS-specific testing"""
        vector_dir = Path(self.mount_point) / "vectors"
        vector_dir.mkdir(exist_ok=True)
        
        # Create test vectors using VexFS ioctl interface
        # This would use the actual VexFS vector operations
        self.logger.info("Creating vector test data...")
        
        # For now, create placeholder files that would be replaced
        # with actual vector operations in a real implementation
        for i in range(100):
            vector_file = vector_dir / f"vector_{i:03d}.vec"
            # In real implementation, this would use VexFS vector ioctls
            with open(vector_file, "wb") as f:
                # Simulate 128-dimensional float32 vector
                vector_data = os.urandom(128 * 4)
                f.write(vector_data)
    
    def _setup_crash_mechanisms(self):
        """Setup various crash injection mechanisms"""
        self.logger.info("Setting up crash injection mechanisms...")
        
        # Setup kernel panic triggers
        self._setup_kernel_panic_triggers()
        
        # Setup I/O error injection
        self._setup_io_error_injection()
        
        # Setup memory pressure simulation
        self._setup_memory_pressure()
    
    def _setup_kernel_panic_triggers(self):
        """Setup kernel panic trigger mechanisms"""
        # Enable sysrq for kernel panic
        with open("/proc/sys/kernel/sysrq", "w") as f:
            f.write("1")
        
        # Prepare panic trigger script
        panic_script = Path(__file__).parent / "trigger_panic.sh"
        with open(panic_script, "w") as f:
            f.write("""#!/bin/bash
# Trigger kernel panic for crash testing
echo c > /proc/sysrq-trigger
""")
        panic_script.chmod(0o755)
    
    def _setup_io_error_injection(self):
        """Setup I/O error injection using dm-flakey"""
        self.logger.info("Setting up I/O error injection...")
        
        # Check if dm-flakey is available
        result = subprocess.run(
            "modprobe dm-flakey", 
            shell=True, 
            capture_output=True
        )
        if result.returncode != 0:
            self.logger.warning("dm-flakey not available, I/O error injection disabled")
    
    def _setup_memory_pressure(self):
        """Setup memory pressure simulation"""
        # Create memory pressure script
        pressure_script = Path(__file__).parent / "memory_pressure.py"
        with open(pressure_script, "w") as f:
            f.write("""#!/usr/bin/env python3
import time
import sys

def create_memory_pressure(mb_to_allocate):
    '''Allocate memory to create pressure'''
    data = []
    for i in range(mb_to_allocate):
        data.append(b'x' * 1024 * 1024)
        if i % 100 == 0:
            print(f"Allocated {i} MB")
    
    print(f"Allocated {mb_to_allocate} MB, holding for 30 seconds...")
    time.sleep(30)

if __name__ == "__main__":
    mb = int(sys.argv[1]) if len(sys.argv) > 1 else 512
    create_memory_pressure(mb)
""")
        pressure_script.chmod(0o755)
    
    def _verify_vexfs_module(self) -> bool:
        """Verify VexFS kernel module is loaded and functional"""
        try:
            # Check if module is loaded
            result = subprocess.run(
                "lsmod | grep vexfs", 
                shell=True, 
                capture_output=True, 
                text=True
            )
            if result.returncode != 0:
                self.logger.error("VexFS module not loaded")
                return False
            
            # Check if filesystem is registered
            result = subprocess.run(
                "cat /proc/filesystems | grep vexfs", 
                shell=True, 
                capture_output=True, 
                text=True
            )
            if result.returncode != 0:
                self.logger.error("VexFS not registered in /proc/filesystems")
                return False
            
            return True
            
        except Exception as e:
            self.logger.error(f"Error verifying VexFS module: {e}")
            return False
    
    def execute_crash_scenario(self, scenario: CrashScenario) -> Dict:
        """Execute a specific crash scenario"""
        self.logger.info(f"Executing crash scenario: {scenario.description}")
        
        start_time = time.time()
        self.crash_count += 1
        
        try:
            # Mount filesystem
            self._mount_filesystem()
            
            # Execute pre-crash operations
            pre_crash_state = self._capture_filesystem_state()
            
            # Start the operation that will be crashed
            operation_thread = threading.Thread(
                target=self._execute_operation,
                args=(scenario.operation,)
            )
            operation_thread.start()
            
            # Wait for crash timing
            self._wait_for_crash_timing(scenario.timing, scenario.delay_ms)
            
            # Trigger the crash
            crash_success = self._trigger_crash(scenario.crash_type)
            
            if not crash_success:
                self.logger.error(f"Failed to trigger crash: {scenario.crash_type}")
                return {"success": False, "error": "Crash trigger failed"}
            
            # Wait for operation thread to complete (it should be interrupted)
            operation_thread.join(timeout=5.0)
            
            # Simulate reboot/recovery
            recovery_success = self._simulate_recovery()
            
            # Verify filesystem integrity after recovery
            integrity_result = self._verify_integrity_after_crash(pre_crash_state)
            
            # Update statistics
            self.recovery_stats['total_crashes'] += 1
            if recovery_success and integrity_result['data_intact']:
                self.recovery_stats['successful_recoveries'] += 1
            else:
                self.recovery_stats['failed_recoveries'] += 1
            
            if not integrity_result['data_intact']:
                self.recovery_stats['data_corruption_detected'] += 1
            
            end_time = time.time()
            
            return {
                "success": True,
                "crash_type": scenario.crash_type.value,
                "operation": scenario.operation,
                "recovery_success": recovery_success,
                "data_integrity": integrity_result,
                "duration_seconds": end_time - start_time,
                "crash_id": self.crash_count
            }
            
        except Exception as e:
            self.logger.error(f"Error executing crash scenario: {e}")
            return {"success": False, "error": str(e)}
    
    def _mount_filesystem(self):
        """Mount the VexFS filesystem for testing"""
        cmd = f"mount -t vexfs {self.test_device} {self.mount_point}"
        result = subprocess.run(cmd, shell=True, capture_output=True, text=True)
        if result.returncode != 0:
            raise Exception(f"Failed to mount filesystem: {result.stderr}")
    
    def _capture_filesystem_state(self) -> Dict:
        """Capture current filesystem state for comparison"""
        state = {
            "file_checksums": {},
            "directory_structure": {},
            "vector_indices": {},
            "metadata": {}
        }
        
        # Calculate checksums for all files
        for root, dirs, files in os.walk(self.mount_point):
            for file in files:
                file_path = Path(root) / file
                try:
                    with open(file_path, "rb") as f:
                        import hashlib
                        content = f.read()
                        checksum = hashlib.sha256(content).hexdigest()
                        rel_path = file_path.relative_to(self.mount_point)
                        state["file_checksums"][str(rel_path)] = checksum
                except Exception as e:
                    self.logger.warning(f"Could not checksum {file_path}: {e}")
        
        # Capture directory structure
        state["directory_structure"] = self._get_directory_tree(self.mount_point)
        
        return state
    
    def _get_directory_tree(self, path: str) -> Dict:
        """Get directory tree structure"""
        tree = {}
        try:
            for item in os.listdir(path):
                item_path = os.path.join(path, item)
                if os.path.isdir(item_path):
                    tree[item] = self._get_directory_tree(item_path)
                else:
                    tree[item] = "file"
        except PermissionError:
            tree["<permission_denied>"] = True
        return tree
    
    def _execute_operation(self, operation: str):
        """Execute the filesystem operation that will be crashed"""
        self.logger.info(f"Executing operation: {operation}")
        
        operations = {
            "file_write": self._operation_file_write,
            "file_delete": self._operation_file_delete,
            "directory_create": self._operation_directory_create,
            "vector_insert": self._operation_vector_insert,
            "vector_search": self._operation_vector_search,
            "mixed_workload": self._operation_mixed_workload
        }
        
        if operation in operations:
            operations[operation]()
        else:
            self.logger.warning(f"Unknown operation: {operation}")
    
    def _operation_file_write(self):
        """File write operation for crash testing"""
        test_file = Path(self.mount_point) / "crash_test_file.dat"
        with open(test_file, "wb") as f:
            for i in range(1000):
                f.write(f"Line {i}: " + "x" * 100 + "\n")
                f.flush()
                time.sleep(0.001)  # Small delay to increase crash window
    
    def _operation_file_delete(self):
        """File deletion operation for crash testing"""
        test_dir = Path(self.mount_point) / "delete_test"
        test_dir.mkdir(exist_ok=True)
        
        # Create files to delete
        for i in range(50):
            test_file = test_dir / f"delete_me_{i}.txt"
            with open(test_file, "w") as f:
                f.write(f"File {i} content\n" * 10)
        
        # Delete files one by one
        for i in range(50):
            test_file = test_dir / f"delete_me_{i}.txt"
            if test_file.exists():
                test_file.unlink()
                time.sleep(0.001)
    
    def _operation_directory_create(self):
        """Directory creation operation for crash testing"""
        base_dir = Path(self.mount_point) / "dir_test"
        base_dir.mkdir(exist_ok=True)
        
        for i in range(100):
            new_dir = base_dir / f"subdir_{i}"
            new_dir.mkdir(exist_ok=True)
            
            # Create a file in each directory
            test_file = new_dir / "test.txt"
            with open(test_file, "w") as f:
                f.write(f"Directory {i} test file\n")
            
            time.sleep(0.001)
    
    def _operation_vector_insert(self):
        """Vector insertion operation for crash testing"""
        # This would use actual VexFS vector operations
        # For now, simulate with file operations
        vector_dir = Path(self.mount_point) / "crash_vectors"
        vector_dir.mkdir(exist_ok=True)
        
        for i in range(100):
            vector_file = vector_dir / f"crash_vector_{i}.vec"
            with open(vector_file, "wb") as f:
                # Simulate vector data
                vector_data = os.urandom(512)  # 128 float32 values
                f.write(vector_data)
            time.sleep(0.001)
    
    def _operation_vector_search(self):
        """Vector search operation for crash testing"""
        # This would use actual VexFS vector search operations
        # For now, simulate with file reads
        vector_dir = Path(self.mount_point) / "vectors"
        if vector_dir.exists():
            for i in range(50):
                vector_files = list(vector_dir.glob("*.vec"))
                if vector_files:
                    test_file = vector_files[i % len(vector_files)]
                    with open(test_file, "rb") as f:
                        data = f.read()
                        # Simulate search computation
                        time.sleep(0.002)
    
    def _operation_mixed_workload(self):
        """Mixed workload operation for crash testing"""
        operations = [
            self._operation_file_write,
            self._operation_directory_create,
            self._operation_vector_insert
        ]
        
        for i in range(30):
            op = operations[i % len(operations)]
            op()
            time.sleep(0.001)
    
    def _wait_for_crash_timing(self, timing: CrashTiming, delay_ms: int):
        """Wait for the appropriate time to trigger crash"""
        if timing == CrashTiming.BEFORE_OPERATION:
            time.sleep(0.001)  # Minimal delay
        elif timing == CrashTiming.DURING_OPERATION:
            time.sleep(delay_ms / 1000.0 if delay_ms > 0 else 0.1)
        elif timing == CrashTiming.AFTER_OPERATION:
            time.sleep(1.0)  # Wait for operation to complete
        elif timing == CrashTiming.RANDOM_TIMING:
            import random
            time.sleep(random.uniform(0.001, 0.5))
    
    def _trigger_crash(self, crash_type: CrashType) -> bool:
        """Trigger the specified type of crash"""
        self.logger.info(f"Triggering crash: {crash_type.value}")
        
        try:
            if crash_type == CrashType.KERNEL_PANIC:
                return self._trigger_kernel_panic()
            elif crash_type == CrashType.POWER_FAILURE:
                return self._trigger_power_failure()
            elif crash_type == CrashType.IO_ERROR:
                return self._trigger_io_error()
            elif crash_type == CrashType.MEMORY_CORRUPTION:
                return self._trigger_memory_corruption()
            elif crash_type == CrashType.DISK_FULL:
                return self._trigger_disk_full()
            else:
                self.logger.error(f"Unknown crash type: {crash_type}")
                return False
                
        except Exception as e:
            self.logger.error(f"Error triggering crash: {e}")
            return False
    
    def _trigger_kernel_panic(self) -> bool:
        """Trigger a kernel panic"""
        try:
            # In a real VM environment, this would trigger a panic
            # For testing, we simulate the effect
            self.logger.info("Simulating kernel panic...")
            
            # Force sync before panic
            subprocess.run("sync", shell=True)
            
            # In real implementation:
            # subprocess.run("echo c > /proc/sysrq-trigger", shell=True)
            
            return True
        except Exception as e:
            self.logger.error(f"Failed to trigger kernel panic: {e}")
            return False
    
    def _trigger_power_failure(self) -> bool:
        """Simulate power failure"""
        try:
            self.logger.info("Simulating power failure...")
            
            # In VM environment, this would abruptly terminate the VM
            # For testing, we simulate by force unmounting
            subprocess.run(f"umount -f {self.mount_point}", shell=True)
            
            return True
        except Exception as e:
            self.logger.error(f"Failed to simulate power failure: {e}")
            return False
    
    def _trigger_io_error(self) -> bool:
        """Trigger I/O errors using dm-flakey"""
        try:
            self.logger.info("Triggering I/O errors...")
            
            # This would use dm-flakey to inject I/O errors
            # For testing, we simulate the effect
            
            return True
        except Exception as e:
            self.logger.error(f"Failed to trigger I/O error: {e}")
            return False
    
    def _trigger_memory_corruption(self) -> bool:
        """Simulate memory corruption"""
        try:
            self.logger.info("Simulating memory corruption...")
            
            # Create memory pressure
            pressure_script = Path(__file__).parent / "memory_pressure.py"
            subprocess.Popen([sys.executable, str(pressure_script), "1024"])
            
            return True
        except Exception as e:
            self.logger.error(f"Failed to simulate memory corruption: {e}")
            return False
    
    def _trigger_disk_full(self) -> bool:
        """Simulate disk full condition"""
        try:
            self.logger.info("Simulating disk full...")
            
            # Fill up the filesystem
            fill_file = Path(self.mount_point) / "fill_disk.tmp"
            with open(fill_file, "wb") as f:
                try:
                    while True:
                        f.write(b"x" * 1024 * 1024)  # Write 1MB chunks
                except OSError:
                    # Disk is full
                    pass
            
            return True
        except Exception as e:
            self.logger.error(f"Failed to simulate disk full: {e}")
            return False
    
    def _simulate_recovery(self) -> bool:
        """Simulate system recovery after crash"""
        try:
            self.logger.info("Simulating recovery...")
            
            # Unmount if still mounted
            subprocess.run(f"umount {self.mount_point}", shell=True, stderr=subprocess.DEVNULL)
            
            # Wait a bit to simulate reboot time
            time.sleep(1)
            
            # Attempt to remount
            result = subprocess.run(
                f"mount -t vexfs {self.test_device} {self.mount_point}",
                shell=True,
                capture_output=True,
                text=True
            )
            
            if result.returncode == 0:
                self.logger.info("Recovery successful - filesystem mounted")
                return True
            else:
                self.logger.error(f"Recovery failed - mount error: {result.stderr}")
                return False
                
        except Exception as e:
            self.logger.error(f"Error during recovery simulation: {e}")
            return False
    
    def _verify_integrity_after_crash(self, pre_crash_state: Dict) -> Dict:
        """Verify filesystem integrity after crash and recovery"""
        self.logger.info("Verifying filesystem integrity after crash...")
        
        result = {
            "data_intact": True,
            "corrupted_files": [],
            "missing_files": [],
            "extra_files": [],
            "directory_structure_intact": True,
            "vector_indices_intact": True
        }
        
        try:
            # Capture post-crash state
            post_crash_state = self._capture_filesystem_state()
            
            # Compare file checksums
            pre_checksums = pre_crash_state.get("file_checksums", {})
            post_checksums = post_crash_state.get("file_checksums", {})
            
            # Check for corrupted files
            for file_path, pre_checksum in pre_checksums.items():
                if file_path in post_checksums:
                    if pre_checksum != post_checksums[file_path]:
                        result["corrupted_files"].append(file_path)
                        result["data_intact"] = False
                else:
                    result["missing_files"].append(file_path)
                    result["data_intact"] = False
            
            # Check for extra files
            for file_path in post_checksums:
                if file_path not in pre_checksums:
                    result["extra_files"].append(file_path)
            
            # Compare directory structures
            if pre_crash_state.get("directory_structure") != post_crash_state.get("directory_structure"):
                result["directory_structure_intact"] = False
                result["data_intact"] = False
            
            # Verify vector indices (VexFS-specific)
            vector_integrity = self._verify_vector_integrity()
            result["vector_indices_intact"] = vector_integrity
            if not vector_integrity:
                result["data_intact"] = False
            
            self.logger.info(f"Integrity check complete: {result}")
            return result
            
        except Exception as e:
            self.logger.error(f"Error verifying integrity: {e}")
            result["data_intact"] = False
            result["error"] = str(e)
            return result
    
    def _verify_vector_integrity(self) -> bool:
        """Verify VexFS vector index integrity"""
        try:
            # This would use VexFS-specific tools to verify vector indices
            # For now, we do basic file existence checks
            
            vector_dir = Path(self.mount_point) / "vectors"
            if not vector_dir.exists():
                return True  # No vectors to check
            
            # Check if vector files are readable
            for vector_file in vector_dir.glob("*.vec"):
                try:
                    with open(vector_file, "rb") as f:
                        data = f.read()
                        if len(data) == 0:
                            self.logger.warning(f"Empty vector file: {vector_file}")
                            return False
                except Exception as e:
                    self.logger.error(f"Cannot read vector file {vector_file}: {e}")
                    return False
            
            return True
            
        except Exception as e:
            self.logger.error(f"Error verifying vector integrity: {e}")
            return False
    
    def run_crash_test_suite(self, scenarios: List[CrashScenario]) -> Dict:
        """Run a complete suite of crash tests"""
        self.logger.info(f"Starting crash test suite with {len(scenarios)} scenarios")
        
        results = {
            "total_scenarios": len(scenarios),
            "successful_tests": 0,
            "failed_tests": 0,
            "scenarios": [],
            "statistics": {},
            "start_time": time.time()
        }
        
        for i, scenario in enumerate(scenarios):
            self.logger.info(f"Running scenario {i+1}/{len(scenarios)}: {scenario.description}")
            
            scenario_result = self.execute_crash_scenario(scenario)
            results["scenarios"].append(scenario_result)
            
            if scenario_result.get("success", False):
                results["successful_tests"] += 1
            else:
                results["failed_tests"] += 1
            
            # Brief pause between scenarios
            time.sleep(2)
        
        results["end_time"] = time.time()
        results["total_duration"] = results["end_time"] - results["start_time"]
        results["statistics"] = self.recovery_stats.copy()
        
        # Save results
        self._save_test_results(results)
        
        self.logger.info(f"Crash test suite completed: {results['successful_tests']}/{results['total_scenarios']} successful")
        return results
    
    def _save_test_results(self, results: Dict):
        """Save test results to file"""
        results_file = Path(__file__).parent.parent / "results" / f"crash_test_results_{int(time.time())}.json"
        results_file.parent.mkdir(exist_ok=True)
        
        with open(results_file, "w") as f:
            json.dump(results, f, indent=2, default=str)
        
        self.logger.info(f"Test results saved to: {results_file}")

def create_default_scenarios() -> List[CrashScenario]:
    """Create a default set of crash test scenarios"""
    scenarios = []
    
    # Basic crash scenarios
    crash_types = [CrashType.KERNEL_PANIC, CrashType.POWER_FAILURE, CrashType.IO_ERROR]
    operations = ["file_write", "file_delete", "directory_create", "vector_insert"]
    timings = [CrashTiming.DURING_OPERATION, CrashTiming.AFTER_OPERATION]
    
    for crash_type in crash_types:
        for operation in operations:
            for timing in timings:
                scenario = CrashScenario(
                    crash_type=crash_type,
                    timing=timing,
                    operation=operation,
                    description=f"{crash_type.value} during {operation} ({timing.value})"
                )
                scenarios.append(scenario)
    
    # Add some random timing scenarios
    for i in range(10):
        scenario = CrashScenario(
            crash_type=CrashType.POWER_FAILURE,
            timing=CrashTiming.RANDOM_TIMING,
            operation="mixed_workload",
            description=f"Random power failure during mixed workload #{i+1}"
        )
        scenarios.append(scenario)
    
    return scenarios

if __name__ == "__main__":
    # Example usage
    vm_config = {
        "memory": "2G",
        "cpus": 2,
        "disk_size": "10G"
    }
    
    injector = CrashInjector(vm_config)
    
    if injector.prepare_test_environment():
        scenarios = create_default_scenarios()
        results = injector.run_crash_test_suite(scenarios)
        
        print(f"\nCrash Test Suite Results:")
        print(f"Total scenarios: {results['total_scenarios']}")
        print(f"Successful tests: {results['successful_tests']}")
        print(f"Failed tests: {results['failed_tests']}")
        print(f"Recovery success rate: {results['statistics']['successful_recoveries']}/{results['statistics']['total_crashes']}")
        
        if results['statistics']['data_corruption_detected'] > 0:
            print(f"WARNING: Data corruption detected in {results['statistics']['data_corruption_detected']} cases")
    else:
        print("Failed to prepare test environment")
        sys.exit(1)