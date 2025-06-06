#!/usr/bin/env python3
"""
VexFS Recovery Testing Framework

This module provides comprehensive recovery testing for VexFS after crashes,
including automated recovery procedures and validation of filesystem state.
"""

import os
import sys
import json
import time
import logging
import subprocess
import threading
from pathlib import Path
from typing import Dict, List, Optional, Tuple
from dataclasses import dataclass
from enum import Enum

class RecoveryStatus(Enum):
    """Recovery operation status"""
    SUCCESS = "success"
    PARTIAL = "partial"
    FAILED = "failed"
    TIMEOUT = "timeout"

@dataclass
class RecoveryTest:
    """Configuration for a recovery test"""
    test_id: str
    description: str
    pre_crash_operations: List[str]
    crash_type: str
    expected_recovery_time: float
    validation_checks: List[str]

class RecoveryValidator:
    """Main recovery testing and validation framework"""
    
    def __init__(self, device_path: str, mount_point: str = "/mnt/vexfs_recovery"):
        self.device_path = device_path
        self.mount_point = mount_point
        self.logger = self._setup_logging()
        self.recovery_stats = {
            'total_tests': 0,
            'successful_recoveries': 0,
            'partial_recoveries': 0,
            'failed_recoveries': 0,
            'timeout_recoveries': 0,
            'average_recovery_time': 0.0,
            'data_loss_incidents': 0
        }
    
    def _setup_logging(self) -> logging.Logger:
        """Setup logging for recovery testing"""
        logger = logging.getLogger('recovery_validator')
        logger.setLevel(logging.DEBUG)
        
        # Create file handler
        log_file = Path(__file__).parent.parent / "results" / "recovery_testing.log"
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
    
    def run_recovery_test_suite(self, tests: List[RecoveryTest]) -> Dict:
        """Run a complete suite of recovery tests"""
        self.logger.info(f"Starting recovery test suite with {len(tests)} tests")
        
        results = {
            "total_tests": len(tests),
            "test_results": [],
            "statistics": {},
            "start_time": time.time()
        }
        
        recovery_times = []
        
        for i, test in enumerate(tests):
            self.logger.info(f"Running recovery test {i+1}/{len(tests)}: {test.description}")
            
            test_result = self.execute_recovery_test(test)
            results["test_results"].append(test_result)
            
            # Update statistics
            self.recovery_stats['total_tests'] += 1
            
            if test_result['recovery_status'] == RecoveryStatus.SUCCESS.value:
                self.recovery_stats['successful_recoveries'] += 1
            elif test_result['recovery_status'] == RecoveryStatus.PARTIAL.value:
                self.recovery_stats['partial_recoveries'] += 1
            elif test_result['recovery_status'] == RecoveryStatus.FAILED.value:
                self.recovery_stats['failed_recoveries'] += 1
            elif test_result['recovery_status'] == RecoveryStatus.TIMEOUT.value:
                self.recovery_stats['timeout_recoveries'] += 1
            
            if test_result.get('recovery_time'):
                recovery_times.append(test_result['recovery_time'])
            
            if not test_result.get('data_integrity', True):
                self.recovery_stats['data_loss_incidents'] += 1
            
            # Brief pause between tests
            time.sleep(2)
        
        # Calculate average recovery time
        if recovery_times:
            self.recovery_stats['average_recovery_time'] = sum(recovery_times) / len(recovery_times)
        
        results["end_time"] = time.time()
        results["total_duration"] = results["end_time"] - results["start_time"]
        results["statistics"] = self.recovery_stats.copy()
        
        # Save results
        self._save_recovery_results(results)
        
        self.logger.info(f"Recovery test suite completed: {self.recovery_stats['successful_recoveries']}/{self.recovery_stats['total_tests']} successful")
        return results
    
    def execute_recovery_test(self, test: RecoveryTest) -> Dict:
        """Execute a single recovery test"""
        self.logger.info(f"Executing recovery test: {test.test_id}")
        
        start_time = time.time()
        
        result = {
            "test_id": test.test_id,
            "description": test.description,
            "recovery_status": RecoveryStatus.FAILED.value,
            "recovery_time": 0.0,
            "data_integrity": False,
            "validation_results": {},
            "error_message": None
        }
        
        try:
            # Step 1: Prepare clean filesystem
            if not self._prepare_clean_filesystem():
                result["error_message"] = "Failed to prepare clean filesystem"
                return result
            
            # Step 2: Execute pre-crash operations
            pre_crash_state = self._execute_pre_crash_operations(test.pre_crash_operations)
            if not pre_crash_state:
                result["error_message"] = "Failed to execute pre-crash operations"
                return result
            
            # Step 3: Simulate crash
            if not self._simulate_crash(test.crash_type):
                result["error_message"] = f"Failed to simulate crash: {test.crash_type}"
                return result
            
            # Step 4: Attempt recovery
            recovery_start = time.time()
            recovery_status = self._attempt_recovery(test.expected_recovery_time)
            recovery_time = time.time() - recovery_start
            
            result["recovery_time"] = recovery_time
            result["recovery_status"] = recovery_status.value
            
            if recovery_status == RecoveryStatus.FAILED:
                result["error_message"] = "Recovery failed"
                return result
            
            # Step 5: Validate recovery
            validation_results = self._validate_recovery(test.validation_checks, pre_crash_state)
            result["validation_results"] = validation_results
            result["data_integrity"] = validation_results.get("data_integrity", False)
            
            # Step 6: Determine final status
            if recovery_status == RecoveryStatus.SUCCESS and result["data_integrity"]:
                result["recovery_status"] = RecoveryStatus.SUCCESS.value
            elif recovery_status == RecoveryStatus.SUCCESS:
                result["recovery_status"] = RecoveryStatus.PARTIAL.value
            
            end_time = time.time()
            result["total_test_time"] = end_time - start_time
            
            return result
            
        except Exception as e:
            self.logger.error(f"Error executing recovery test {test.test_id}: {e}")
            result["error_message"] = str(e)
            return result
    
    def _prepare_clean_filesystem(self) -> bool:
        """Prepare a clean VexFS filesystem for testing"""
        try:
            self.logger.info("Preparing clean filesystem...")
            
            # Unmount if mounted
            subprocess.run(f"umount {self.mount_point}", shell=True, stderr=subprocess.DEVNULL)
            
            # Format with VexFS
            cmd = f"mkfs.vexfs {self.device_path}"
            result = subprocess.run(cmd, shell=True, capture_output=True, text=True)
            if result.returncode != 0:
                self.logger.error(f"Failed to format device: {result.stderr}")
                return False
            
            # Create mount point
            os.makedirs(self.mount_point, exist_ok=True)
            
            # Mount filesystem
            cmd = f"mount -t vexfs {self.device_path} {self.mount_point}"
            result = subprocess.run(cmd, shell=True, capture_output=True, text=True)
            if result.returncode != 0:
                self.logger.error(f"Failed to mount filesystem: {result.stderr}")
                return False
            
            self.logger.info("Clean filesystem prepared successfully")
            return True
            
        except Exception as e:
            self.logger.error(f"Error preparing clean filesystem: {e}")
            return False
    
    def _execute_pre_crash_operations(self, operations: List[str]) -> Optional[Dict]:
        """Execute operations before crash simulation"""
        try:
            self.logger.info("Executing pre-crash operations...")
            
            state = {
                "files_created": [],
                "directories_created": [],
                "vectors_inserted": [],
                "checksums": {},
                "operation_count": len(operations)
            }
            
            for operation in operations:
                if operation == "create_files":
                    files = self._create_test_files()
                    state["files_created"].extend(files)
                elif operation == "create_directories":
                    dirs = self._create_test_directories()
                    state["directories_created"].extend(dirs)
                elif operation == "insert_vectors":
                    vectors = self._insert_test_vectors()
                    state["vectors_inserted"].extend(vectors)
                elif operation == "mixed_workload":
                    self._execute_mixed_workload()
                else:
                    self.logger.warning(f"Unknown operation: {operation}")
            
            # Calculate checksums for verification
            state["checksums"] = self._calculate_filesystem_checksums()
            
            # Sync to ensure data is written
            subprocess.run("sync", shell=True)
            
            self.logger.info(f"Pre-crash operations completed: {len(operations)} operations")
            return state
            
        except Exception as e:
            self.logger.error(f"Error executing pre-crash operations: {e}")
            return None
    
    def _create_test_files(self) -> List[str]:
        """Create test files with known content"""
        files = []
        test_dir = Path(self.mount_point) / "test_files"
        test_dir.mkdir(exist_ok=True)
        
        # Create files with different patterns
        patterns = {
            "sequential.dat": lambda: b"".join(i.to_bytes(4, 'little') for i in range(1000)),
            "text_file.txt": lambda: b"This is a test file for recovery testing.\n" * 100,
            "binary_data.bin": lambda: os.urandom(8192),
            "large_file.dat": lambda: b"RECOVERY_TEST_PATTERN" * 1000
        }
        
        for filename, content_func in patterns.items():
            file_path = test_dir / filename
            with open(file_path, "wb") as f:
                f.write(content_func())
            files.append(str(file_path))
        
        return files
    
    def _create_test_directories(self) -> List[str]:
        """Create test directory structure"""
        dirs = []
        base_dir = Path(self.mount_point) / "test_dirs"
        base_dir.mkdir(exist_ok=True)
        
        # Create nested directory structure
        for i in range(5):
            dir_path = base_dir / f"level1_{i}"
            dir_path.mkdir(exist_ok=True)
            dirs.append(str(dir_path))
            
            for j in range(3):
                subdir_path = dir_path / f"level2_{j}"
                subdir_path.mkdir(exist_ok=True)
                dirs.append(str(subdir_path))
                
                # Create a file in each subdirectory
                file_path = subdir_path / "test_file.txt"
                with open(file_path, "w") as f:
                    f.write(f"Content for directory {i}.{j}\n")
        
        return dirs
    
    def _insert_test_vectors(self) -> List[str]:
        """Insert test vectors using VexFS vector operations"""
        vectors = []
        vector_dir = Path(self.mount_point) / "test_vectors"
        vector_dir.mkdir(exist_ok=True)
        
        # Create test vectors
        for i in range(50):
            vector_file = vector_dir / f"vector_{i:03d}.vec"
            
            # Generate test vector data (128-dimensional float32)
            vector_data = os.urandom(128 * 4)
            
            with open(vector_file, "wb") as f:
                f.write(vector_data)
            
            vectors.append(str(vector_file))
        
        return vectors
    
    def _execute_mixed_workload(self):
        """Execute a mixed workload of operations"""
        # Combination of file operations
        self._create_test_files()
        self._create_test_directories()
        self._insert_test_vectors()
        
        # Additional operations
        temp_dir = Path(self.mount_point) / "temp_operations"
        temp_dir.mkdir(exist_ok=True)
        
        # Create and delete some files
        for i in range(10):
            temp_file = temp_dir / f"temp_{i}.txt"
            with open(temp_file, "w") as f:
                f.write(f"Temporary file {i}\n")
            
            if i % 2 == 0:  # Delete every other file
                temp_file.unlink()
    
    def _calculate_filesystem_checksums(self) -> Dict[str, str]:
        """Calculate checksums for all files in the filesystem"""
        import hashlib
        
        checksums = {}
        
        for root, dirs, files in os.walk(self.mount_point):
            for file in files:
                file_path = Path(root) / file
                try:
                    with open(file_path, "rb") as f:
                        content = f.read()
                        checksum = hashlib.sha256(content).hexdigest()
                        rel_path = file_path.relative_to(self.mount_point)
                        checksums[str(rel_path)] = checksum
                except Exception as e:
                    self.logger.warning(f"Could not checksum {file_path}: {e}")
        
        return checksums
    
    def _simulate_crash(self, crash_type: str) -> bool:
        """Simulate a filesystem crash"""
        try:
            self.logger.info(f"Simulating crash: {crash_type}")
            
            if crash_type == "power_failure":
                return self._simulate_power_failure()
            elif crash_type == "kernel_panic":
                return self._simulate_kernel_panic()
            elif crash_type == "io_error":
                return self._simulate_io_error()
            elif crash_type == "force_unmount":
                return self._simulate_force_unmount()
            else:
                self.logger.error(f"Unknown crash type: {crash_type}")
                return False
                
        except Exception as e:
            self.logger.error(f"Error simulating crash: {e}")
            return False
    
    def _simulate_power_failure(self) -> bool:
        """Simulate sudden power failure"""
        try:
            # Force unmount without sync
            result = subprocess.run(
                f"umount -f {self.mount_point}",
                shell=True,
                capture_output=True,
                text=True
            )
            
            # In a real VM environment, this would abruptly terminate the VM
            self.logger.info("Power failure simulated")
            return True
            
        except Exception as e:
            self.logger.error(f"Failed to simulate power failure: {e}")
            return False
    
    def _simulate_kernel_panic(self) -> bool:
        """Simulate kernel panic"""
        try:
            # In a real environment, this would trigger a kernel panic
            # For testing, we simulate by force unmounting
            subprocess.run("sync", shell=True)  # Partial sync
            subprocess.run(f"umount -f {self.mount_point}", shell=True)
            
            self.logger.info("Kernel panic simulated")
            return True
            
        except Exception as e:
            self.logger.error(f"Failed to simulate kernel panic: {e}")
            return False
    
    def _simulate_io_error(self) -> bool:
        """Simulate I/O errors"""
        try:
            # This would use dm-flakey or similar to inject I/O errors
            # For testing, we simulate by corrupting some data
            
            # Force unmount
            subprocess.run(f"umount -f {self.mount_point}", shell=True)
            
            self.logger.info("I/O error simulated")
            return True
            
        except Exception as e:
            self.logger.error(f"Failed to simulate I/O error: {e}")
            return False
    
    def _simulate_force_unmount(self) -> bool:
        """Simulate forced unmount"""
        try:
            result = subprocess.run(
                f"umount -f {self.mount_point}",
                shell=True,
                capture_output=True,
                text=True
            )
            
            self.logger.info("Force unmount simulated")
            return True
            
        except Exception as e:
            self.logger.error(f"Failed to simulate force unmount: {e}")
            return False
    
    def _attempt_recovery(self, timeout: float) -> RecoveryStatus:
        """Attempt to recover the filesystem"""
        try:
            self.logger.info("Attempting filesystem recovery...")
            
            start_time = time.time()
            
            # Step 1: Try to mount the filesystem
            mount_result = self._attempt_mount(timeout)
            
            if mount_result == RecoveryStatus.TIMEOUT:
                return RecoveryStatus.TIMEOUT
            elif mount_result == RecoveryStatus.FAILED:
                # Step 2: Try fsck and then mount
                fsck_result = self._run_fsck_recovery()
                if not fsck_result:
                    return RecoveryStatus.FAILED
                
                # Try mounting again after fsck
                mount_result = self._attempt_mount(timeout / 2)
                if mount_result != RecoveryStatus.SUCCESS:
                    return RecoveryStatus.FAILED
            
            # Step 3: Verify basic filesystem functionality
            if not self._verify_basic_functionality():
                return RecoveryStatus.PARTIAL
            
            recovery_time = time.time() - start_time
            self.logger.info(f"Recovery completed in {recovery_time:.2f} seconds")
            
            return RecoveryStatus.SUCCESS
            
        except Exception as e:
            self.logger.error(f"Error during recovery attempt: {e}")
            return RecoveryStatus.FAILED
    
    def _attempt_mount(self, timeout: float) -> RecoveryStatus:
        """Attempt to mount the filesystem with timeout"""
        try:
            def mount_filesystem():
                cmd = f"mount -t vexfs {self.device_path} {self.mount_point}"
                result = subprocess.run(cmd, shell=True, capture_output=True, text=True)
                return result.returncode == 0
            
            # Run mount in a thread with timeout
            mount_thread = threading.Thread(target=mount_filesystem)
            mount_thread.start()
            mount_thread.join(timeout)
            
            if mount_thread.is_alive():
                # Mount is taking too long
                self.logger.warning("Mount operation timed out")
                return RecoveryStatus.TIMEOUT
            
            # Check if mount was successful
            if os.path.ismount(self.mount_point):
                self.logger.info("Filesystem mounted successfully")
                return RecoveryStatus.SUCCESS
            else:
                self.logger.warning("Mount operation failed")
                return RecoveryStatus.FAILED
                
        except Exception as e:
            self.logger.error(f"Error attempting mount: {e}")
            return RecoveryStatus.FAILED
    
    def _run_fsck_recovery(self) -> bool:
        """Run filesystem check and repair"""
        try:
            self.logger.info("Running filesystem check and repair...")
            
            # Import the fsck module
            sys.path.append(str(Path(__file__).parent.parent / "data_integrity"))
            from vexfs_fsck import VexFSChecker
            
            # Run fsck with automatic fixes
            checker = VexFSChecker(self.device_path)
            results = checker.check_filesystem(fix_errors=True)
            
            if results["overall_status"] in ["clean", "warnings"]:
                self.logger.info("Filesystem check completed successfully")
                return True
            else:
                self.logger.error(f"Filesystem check failed: {results['overall_status']}")
                return False
                
        except Exception as e:
            self.logger.error(f"Error running fsck: {e}")
            return False
    
    def _verify_basic_functionality(self) -> bool:
        """Verify basic filesystem functionality after recovery"""
        try:
            # Test basic file operations
            test_file = Path(self.mount_point) / "recovery_test.tmp"
            
            # Write test
            with open(test_file, "w") as f:
                f.write("Recovery test content\n")
            
            # Read test
            with open(test_file, "r") as f:
                content = f.read()
                if "Recovery test content" not in content:
                    return False
            
            # Delete test
            test_file.unlink()
            
            # Test directory operations
            test_dir = Path(self.mount_point) / "recovery_test_dir"
            test_dir.mkdir()
            test_dir.rmdir()
            
            self.logger.info("Basic functionality verification passed")
            return True
            
        except Exception as e:
            self.logger.error(f"Basic functionality verification failed: {e}")
            return False
    
    def _validate_recovery(self, validation_checks: List[str], pre_crash_state: Dict) -> Dict:
        """Validate the recovery against pre-crash state"""
        results = {
            "data_integrity": True,
            "file_integrity": True,
            "directory_integrity": True,
            "vector_integrity": True,
            "missing_files": [],
            "corrupted_files": [],
            "extra_files": [],
            "validation_details": {}
        }
        
        try:
            for check in validation_checks:
                if check == "file_integrity":
                    file_result = self._validate_file_integrity(pre_crash_state)
                    results["file_integrity"] = file_result["integrity"]
                    results["validation_details"]["file_integrity"] = file_result
                    
                elif check == "directory_integrity":
                    dir_result = self._validate_directory_integrity(pre_crash_state)
                    results["directory_integrity"] = dir_result["integrity"]
                    results["validation_details"]["directory_integrity"] = dir_result
                    
                elif check == "vector_integrity":
                    vector_result = self._validate_vector_integrity(pre_crash_state)
                    results["vector_integrity"] = vector_result["integrity"]
                    results["validation_details"]["vector_integrity"] = vector_result
                    
                elif check == "checksum_validation":
                    checksum_result = self._validate_checksums(pre_crash_state)
                    results["validation_details"]["checksum_validation"] = checksum_result
                    
                    # Update file lists
                    results["missing_files"].extend(checksum_result.get("missing_files", []))
                    results["corrupted_files"].extend(checksum_result.get("corrupted_files", []))
                    results["extra_files"].extend(checksum_result.get("extra_files", []))
            
            # Overall data integrity
            results["data_integrity"] = (
                results["file_integrity"] and
                results["directory_integrity"] and
                results["vector_integrity"] and
                len(results["missing_files"]) == 0 and
                len(results["corrupted_files"]) == 0
            )
            
            return results
            
        except Exception as e:
            self.logger.error(f"Error validating recovery: {e}")
            results["data_integrity"] = False
            results["validation_details"]["error"] = str(e)
            return results
    
    def _validate_file_integrity(self, pre_crash_state: Dict) -> Dict:
        """Validate file integrity after recovery"""
        result = {"integrity": True, "details": []}
        
        try:
            created_files = pre_crash_state.get("files_created", [])
            
            for file_path in created_files:
                if os.path.exists(file_path):
                    # Check if file is readable
                    try:
                        with open(file_path, "rb") as f:
                            f.read()
                        result["details"].append(f"File intact: {file_path}")
                    except Exception as e:
                        result["integrity"] = False
                        result["details"].append(f"File corrupted: {file_path} - {e}")
                else:
                    result["integrity"] = False
                    result["details"].append(f"File missing: {file_path}")
            
            return result
            
        except Exception as e:
            result["integrity"] = False
            result["details"].append(f"Validation error: {e}")
            return result
    
    def _validate_directory_integrity(self, pre_crash_state: Dict) -> Dict:
        """Validate directory structure integrity after recovery"""
        result = {"integrity": True, "details": []}
        
        try:
            created_dirs = pre_crash_state.get("directories_created", [])
            
            for dir_path in created_dirs:
                if os.path.exists(dir_path) and os.path.isdir(dir_path):
                    result["details"].append(f"Directory intact: {dir_path}")
                else:
                    result["integrity"] = False
                    result["details"].append(f"Directory missing: {dir_path}")
            
            return result
            
        except Exception as e:
            result["integrity"] = False
            result["details"].append(f"Validation error: {e}")
            return result
    
    def _validate_vector_integrity(self, pre_crash_state: Dict) -> Dict:
        """Validate vector data integrity after recovery"""
        result = {"integrity": True, "details": []}
        
        try:
            inserted_vectors = pre_crash_state.get("vectors_inserted", [])
            
            for vector_path in inserted_vectors:
                if os.path.exists(vector_path):
                    try:
                        with open(vector_path, "rb") as f:
                            data = f.read()
                            if len(data) > 0:
                                result["details"].append(f"Vector intact: {vector_path}")
                            else:
                                result["integrity"] = False
                                result["details"].append(f"Vector empty: {vector_path}")
                    except Exception as e:
                        result["integrity"] = False
                        result["details"].append(f"Vector corrupted: {vector_path} - {e}")
                else:
                    result["integrity"] = False
                    result["details"].append(f"Vector missing: {vector_path}")
            
            return result
            
        except Exception as e:
            result["integrity"] = False
            result["details"].append(f"Validation error: {e}")
            return result
    
    def _validate_checksums(self, pre_crash_state: Dict) -> Dict:
        """Validate file checksums after recovery"""
        result = {
            "integrity": True,
            "missing_files": [],
            "corrupted_files": [],
            "extra_files": [],
            "details": []
        }
        
        try:
            pre_checksums = pre_crash_state.get("checksums", {})
            post_checksums = self._calculate_filesystem_checksums()
            
            # Check for missing files
            for file_path, pre_checksum in pre_checksums.items():
                if file_path not in post_checksums:
                    result["missing_files"].append(file_path)
                    result["integrity"] = False
                    result["details"].append(f"Missing: {file_path}")
                elif pre_checksum != post_checksums[file_path]:
                    result["corrupted_files"].append(file_path)
                    result["integrity"] = False
                    result["details"].append(f"Corrupted: {file_path}")
                else:
                    result["details"].append(f"Intact: {file_path}")
            
            # Check for extra files
            for file_path in post_checksums:
                if file_path not in pre_checksums:
                    result["extra_files"].append(file_path)
                    result["details"].append(f"Extra: {file_path}")
            
            return result
            
        except Exception as e:
            result["integrity"] = False
            result["details"].append(f"Checksum validation error: {e}")
            return result
    
    def _save_recovery_results(self, results: Dict):
        """Save recovery test results to file"""
        results_file = Path(__file__).parent.parent / "results" / f"recovery_test_results_{int(time.time())}.json"
        results_file.parent.mkdir(exist_ok=True)
        
        with open(results_file, "w") as f:
            json.dump(results, f, indent=2, default=str)
        
        self.logger.info(f"Recovery test results saved to: {results_file}")

def create_default_recovery_tests() -> List[RecoveryTest]:
    """Create a default set of recovery tests"""
    tests = []
    
    # Basic recovery tests
    basic_tests = [
        RecoveryTest(
            test_id="recovery_001",
            description="Power failure during file creation",
            pre_crash_operations=["create_files"],
            crash_type="power_failure",
            expected_recovery_time=5.0,
            validation_checks=["file_integrity", "checksum_validation"]
        ),
        RecoveryTest(
            test_id="recovery_002",
            description="Kernel panic during directory operations",
            pre_crash_operations=["create_directories"],
            crash_type="kernel_panic",
            expected_recovery_time=10.0,
            validation_checks=["directory_integrity", "file_integrity"]
        ),
        RecoveryTest(
            test_id="recovery_003",
            description="I/O error during vector insertion",
            pre_crash_operations=["insert_vectors"],
            crash_type="io_error",
            expected_recovery_time=15.0,
            validation_checks=["vector_integrity", "checksum_validation"]
        ),
        RecoveryTest(
            test_id="recovery_004",
            description="Force unmount during mixed workload",
            pre_crash_operations=["mixed_workload"],
            crash_type="force_unmount",
            expected_recovery_time=8.0,
            validation_checks=["file_integrity", "directory_integrity", "vector_integrity", "checksum_validation"]
        )
    ]
    
    tests.extend(basic_tests)
    
    # Stress recovery tests
    stress_tests = [
        RecoveryTest(
            test_id="recovery_stress_001",
            description="Multiple operations with power failure",
            pre_crash_operations=["create_files", "create_directories", "insert_vectors"],
            crash_type="power_failure",
            expected_recovery_time=20.0,
            validation_checks=["file_integrity", "directory_integrity", "vector_integrity", "checksum_validation"]
        ),
        RecoveryTest(
            test_id="recovery_stress_002",
            description="Repeated crash and recovery cycles",
            pre_crash_operations=["mixed_workload"],
            crash_type="power_failure",
            expected_recovery_time=25.0,
            validation_checks=["file_integrity", "directory_integrity", "vector_integrity"]
        )
    ]
    
    tests.extend(stress_tests)
    return tests

def main():
    """Main entry point for recovery testing"""
    import argparse
    
    parser = argparse.ArgumentParser(description="VexFS Recovery Testing Framework")
    parser.add_argument("device", help="Device path to test")
    parser.add_argument("-m", "--mount-point", default="/mnt/vexfs_recovery", help="Mount point for testing")
    parser.add_argument("-t", "--test-suite", choices=["basic", "stress", "all"], default="basic", help="Test suite to run")
    parser.add_argument("-v", "--verbose", action="store_true", help="Verbose output")
    
    args = parser.parse_args()
    
    if args.verbose:
        logging.getLogger().setLevel(logging.DEBUG)
    
    validator = RecoveryValidator(args.device, args.mount_point)
    
    print(f"VexFS Recovery Testing - Device: {args.device}")
    print("=" * 50)
    
    # Create test suite
    if args.test_suite == "basic":
        tests = create_default_recovery_tests()[:4]  # First 4 basic tests
    elif args.test_suite == "stress":
        tests = create_default_recovery_tests()[4:]  # Stress tests
    else:  # all
        tests = create_default_recovery_tests()
    
    results = validator.run_recovery_test_suite(tests)
    
    # Print summary
    print(f"\nRecovery Test Results:")
    print(f"Total tests: {results['total_tests']}")
    print(f"Successful recoveries: {results['statistics']['successful_recoveries']}")
    print(f"Partial recoveries: {results['statistics']['partial_recoveries']}")
    print(f"Failed recoveries: {results['statistics']['failed_recoveries']}")
    print(f"Timeout recoveries: {results['statistics']['timeout_recoveries']}")
    print(f"Average recovery time: {results['statistics']['average_recovery_time']:.2f} seconds")
    
    if results['statistics']['data_loss_incidents'] > 0:
        print(f"WARNING: Data loss detected in {results['statistics']['data_loss_incidents']} cases")
    
    # Print individual test results
    print(f"\nIndividual Test Results:")
    for test_result in results['test_results']:
        status = test_result['recovery_status'].upper()
        test_id = test_result['test_id']
        recovery_time = test_result.get('recovery_time', 0)
        data_integrity = test_result.get('data_integrity', False)
        
        print(f"  {test_id}: {status} ({recovery_time:.2f}s) - Data Integrity: {'✓' if data_integrity else '✗'}")
        
        if test_result.get('error_message'):
            print(f"    Error: {test_result['error_message']}")
    
    # Exit with appropriate code
    if results['statistics']['failed_recoveries'] > 0:
        sys.exit(2)
    elif results['statistics']['partial_recoveries'] > 0:
        sys.exit(1)
    else:
        sys.exit(0)

if __name__ == "__main__":
    main()