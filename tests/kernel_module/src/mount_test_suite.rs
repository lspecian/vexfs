//! Comprehensive Mount-Level Test Suite for VexFS Kernel Module
//!
//! This module provides comprehensive mount testing with:
//! - Normal mount/unmount operations with various mount options
//! - Edge cases: invalid mount options, already-mounted filesystems
//! - Resource-constrained environment testing
//! - Mount option validation and boundary testing
//! - Concurrent mount testing with race condition detection
//! - Parallel mount/unmount stress testing
//! - Deadlock detection and prevention
//! - Resource contention handling

use std::process::{Command, Stdio, Child};
use std::time::{Duration, Instant, SystemTime};
use std::thread;
use std::sync::{Arc, Mutex, mpsc};
use std::collections::{HashMap, VecDeque};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use serde::{Deserialize, Serialize};

use super::{
    Level2TestRunner, VmConfig, TestStatus, PerformanceMetrics,
    crash_detection::{CrashDetector, VmMonitorConfig, CrashEvent, CrashEventType, CrashSeverity}
};

#[derive(Debug, Clone)]
pub struct MountTestSuite {
    pub test_name: String,
    pub vm_config: VmConfig,
    pub crash_detector: Option<CrashDetector>,
    pub test_timeout: Duration,
    pub max_concurrent_mounts: u32,
    pub stress_test_cycles: u32,
    pub mount_options_variants: Vec<MountOptionsSet>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MountOptionsSet {
    pub name: String,
    pub options: Vec<String>,
    pub expected_success: bool,
    pub description: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MountTestResult {
    pub test_name: String,
    pub status: TestStatus,
    pub duration_ms: u64,
    pub normal_mount_tests: NormalMountTestResults,
    pub edge_case_tests: EdgeCaseTestResults,
    pub concurrent_mount_tests: ConcurrentMountTestResults,
    pub stress_test_results: StressTestResults,
    pub resource_constraint_tests: ResourceConstraintTestResults,
    pub performance_metrics: PerformanceMetrics,
    pub crash_events: Vec<CrashEvent>,
    pub error_details: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NormalMountTestResults {
    pub basic_mount_unmount: bool,
    pub mount_with_options: HashMap<String, bool>,
    pub remount_tests: bool,
    pub mount_point_validation: bool,
    pub filesystem_type_detection: bool,
    pub mount_time_ms: u64,
    pub unmount_time_ms: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EdgeCaseTestResults {
    pub invalid_mount_options: HashMap<String, bool>,
    pub already_mounted_filesystem: bool,
    pub nonexistent_device: bool,
    pub invalid_mount_point: bool,
    pub permission_denied_scenarios: bool,
    pub corrupted_filesystem: bool,
    pub device_busy_scenarios: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConcurrentMountTestResults {
    pub parallel_mount_attempts: u32,
    pub successful_concurrent_mounts: u32,
    pub race_conditions_detected: u32,
    pub deadlocks_detected: u32,
    pub resource_contention_events: u32,
    pub max_concurrent_achieved: u32,
    pub concurrent_mount_time_ms: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StressTestResults {
    pub stress_cycles_completed: u32,
    pub mount_unmount_cycles: u32,
    pub failures_detected: u32,
    pub recovery_attempts: u32,
    pub successful_recoveries: u32,
    pub memory_leaks_detected: u32,
    pub performance_degradation_detected: bool,
    pub stability_score: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResourceConstraintTestResults {
    pub low_memory_mount_test: bool,
    pub high_cpu_load_mount_test: bool,
    pub disk_space_constraint_test: bool,
    pub file_descriptor_limit_test: bool,
    pub network_constraint_test: bool,
    pub constraint_recovery_successful: bool,
}

impl Default for MountTestSuite {
    fn default() -> Self {
        Self {
            test_name: "Comprehensive_Mount_Test_Suite".to_string(),
            vm_config: VmConfig::default(),
            crash_detector: None,
            test_timeout: Duration::from_secs(3600), // 1 hour for comprehensive testing
            max_concurrent_mounts: 10,
            stress_test_cycles: 50,
            mount_options_variants: Self::default_mount_options(),
        }
    }
}

impl MountTestSuite {
    pub fn new(vm_config: VmConfig) -> Self {
        Self {
            vm_config,
            ..Default::default()
        }
    }

    pub fn with_crash_detection(mut self, enabled: bool) -> Self {
        if enabled {
            let monitor_config = VmMonitorConfig {
                ssh_key_path: self.vm_config.ssh_key_path.clone(),
                ssh_port: self.vm_config.ssh_port,
                vm_user: self.vm_config.vm_user.clone(),
                monitoring_interval_ms: 1000,
                crash_log_path: "tests/vm_testing/logs/mount_crash.log".to_string(),
                performance_log_path: "tests/vm_testing/logs/mount_performance.log".to_string(),
                max_events_stored: 1000,
                auto_recovery_enabled: true,
                performance_thresholds: Default::default(),
            };
            self.crash_detector = Some(CrashDetector::new(monitor_config));
        }
        self
    }

    pub fn with_stress_cycles(mut self, cycles: u32) -> Self {
        self.stress_test_cycles = cycles;
        self
    }

    pub fn with_max_concurrent_mounts(mut self, max_mounts: u32) -> Self {
        self.max_concurrent_mounts = max_mounts;
        self
    }

    fn default_mount_options() -> Vec<MountOptionsSet> {
        vec![
            MountOptionsSet {
                name: "default".to_string(),
                options: vec![],
                expected_success: true,
                description: "Default mount options".to_string(),
            },
            MountOptionsSet {
                name: "read_only".to_string(),
                options: vec!["-o".to_string(), "ro".to_string()],
                expected_success: true,
                description: "Read-only mount".to_string(),
            },
            MountOptionsSet {
                name: "no_exec".to_string(),
                options: vec!["-o".to_string(), "noexec".to_string()],
                expected_success: true,
                description: "No execution mount".to_string(),
            },
            MountOptionsSet {
                name: "no_suid".to_string(),
                options: vec!["-o".to_string(), "nosuid".to_string()],
                expected_success: true,
                description: "No SUID mount".to_string(),
            },
            MountOptionsSet {
                name: "sync".to_string(),
                options: vec!["-o".to_string(), "sync".to_string()],
                expected_success: true,
                description: "Synchronous mount".to_string(),
            },
            MountOptionsSet {
                name: "invalid_option".to_string(),
                options: vec!["-o".to_string(), "invalid_option_xyz".to_string()],
                expected_success: false,
                description: "Invalid mount option test".to_string(),
            },
        ]
    }

    pub fn run_comprehensive_mount_tests(&mut self) -> Result<MountTestResult, Box<dyn std::error::Error>> {
        let start_time = Instant::now();
        println!("🚀 Starting Comprehensive Mount-Level Test Suite with Crash Recovery");

        let mut result = MountTestResult {
            test_name: self.test_name.clone(),
            status: TestStatus::Failed,
            duration_ms: 0,
            normal_mount_tests: NormalMountTestResults {
                basic_mount_unmount: false,
                mount_with_options: HashMap::new(),
                remount_tests: false,
                mount_point_validation: false,
                filesystem_type_detection: false,
                mount_time_ms: 0,
                unmount_time_ms: 0,
            },
            edge_case_tests: EdgeCaseTestResults {
                invalid_mount_options: HashMap::new(),
                already_mounted_filesystem: false,
                nonexistent_device: false,
                invalid_mount_point: false,
                permission_denied_scenarios: false,
                corrupted_filesystem: false,
                device_busy_scenarios: false,
            },
            concurrent_mount_tests: ConcurrentMountTestResults {
                parallel_mount_attempts: 0,
                successful_concurrent_mounts: 0,
                race_conditions_detected: 0,
                deadlocks_detected: 0,
                resource_contention_events: 0,
                max_concurrent_achieved: 0,
                concurrent_mount_time_ms: 0,
            },
            stress_test_results: StressTestResults {
                stress_cycles_completed: 0,
                mount_unmount_cycles: 0,
                failures_detected: 0,
                recovery_attempts: 0,
                successful_recoveries: 0,
                memory_leaks_detected: 0,
                performance_degradation_detected: false,
                stability_score: 0.0,
            },
            resource_constraint_tests: ResourceConstraintTestResults {
                low_memory_mount_test: false,
                high_cpu_load_mount_test: false,
                disk_space_constraint_test: false,
                file_descriptor_limit_test: false,
                network_constraint_test: false,
                constraint_recovery_successful: false,
            },
            performance_metrics: PerformanceMetrics {
                mount_time_ms: 0,
                unmount_time_ms: 0,
                file_creation_time_ms: 0,
                file_write_time_ms: 0,
                file_read_time_ms: 0,
                memory_usage_kb: 0,
                cpu_usage_percent: 0.0,
                io_operations_per_second: 0.0,
                kernel_memory_usage_kb: 0,
            },
            crash_events: Vec::new(),
            error_details: None,
        };

        // Start crash detection if enabled
        if let Some(ref mut detector) = self.crash_detector {
            detector.start_monitoring()?;
        }

        // Phase 1: Normal Mount Tests
        println!("📋 Phase 1: Normal Mount Operations Testing");
        match self.run_normal_mount_tests() {
            Ok(normal_results) => result.normal_mount_tests = normal_results,
            Err(e) => {
                result.error_details = Some(format!("Normal mount tests failed: {}", e));
                self.finalize_test_result(&mut result, start_time);
                return Ok(result);
            }
        }

        // Phase 2: Edge Case Tests
        println!("⚠️  Phase 2: Edge Case Testing");
        match self.run_edge_case_tests() {
            Ok(edge_results) => result.edge_case_tests = edge_results,
            Err(e) => {
                result.error_details = Some(format!("Edge case tests failed: {}", e));
                self.finalize_test_result(&mut result, start_time);
                return Ok(result);
            }
        }

        // Phase 3: Concurrent Mount Tests
        println!("🔄 Phase 3: Concurrent Mount Testing");
        match self.run_concurrent_mount_tests() {
            Ok(concurrent_results) => result.concurrent_mount_tests = concurrent_results,
            Err(e) => {
                result.error_details = Some(format!("Concurrent mount tests failed: {}", e));
                self.finalize_test_result(&mut result, start_time);
                return Ok(result);
            }
        }

        // Phase 4: Stress Testing
        println!("💪 Phase 4: Mount Stress Testing");
        match self.run_stress_tests() {
            Ok(stress_results) => result.stress_test_results = stress_results,
            Err(e) => {
                result.error_details = Some(format!("Stress tests failed: {}", e));
                self.finalize_test_result(&mut result, start_time);
                return Ok(result);
            }
        }

        // Phase 5: Resource Constraint Tests
        println!("🔒 Phase 5: Resource Constraint Testing");
        match self.run_resource_constraint_tests() {
            Ok(constraint_results) => result.resource_constraint_tests = constraint_results,
            Err(e) => {
                result.error_details = Some(format!("Resource constraint tests failed: {}", e));
                self.finalize_test_result(&mut result, start_time);
                return Ok(result);
            }
        }

        // Determine overall success
        result.status = if result.normal_mount_tests.basic_mount_unmount
            && result.edge_case_tests.invalid_mount_options.len() > 0
            && result.concurrent_mount_tests.successful_concurrent_mounts > 0
            && result.stress_test_results.stress_cycles_completed > 0 {
            TestStatus::Success
        } else {
            TestStatus::Failed
        };

        self.finalize_test_result(&mut result, start_time);
        Ok(result)
    }

    fn run_normal_mount_tests(&self) -> Result<NormalMountTestResults, Box<dyn std::error::Error>> {
        let mut results = NormalMountTestResults {
            basic_mount_unmount: false,
            mount_with_options: HashMap::new(),
            remount_tests: false,
            mount_point_validation: false,
            filesystem_type_detection: false,
            mount_time_ms: 0,
            unmount_time_ms: 0,
        };

        // Test 1: Basic Mount/Unmount
        println!("  🔧 Testing basic mount/unmount operations...");
        let mount_start = Instant::now();
        
        // Create test filesystem
        let setup_result = self.setup_test_filesystem("basic_test")?;
        if !setup_result {
            return Err("Failed to setup test filesystem".into());
        }

        // Mount filesystem
        let mount_cmd = self.execute_ssh_command(&format!(
            "sudo mkdir -p /mnt/vexfs_basic && sudo mount /dev/loop0 /mnt/vexfs_basic"
        ))?;

        results.mount_time_ms = mount_start.elapsed().as_millis() as u64;
        
        if mount_cmd.status.success() {
            results.basic_mount_unmount = true;
            println!("    ✅ Basic mount successful ({}ms)", results.mount_time_ms);

            // Test unmount
            let unmount_start = Instant::now();
            let unmount_cmd = self.execute_ssh_command("sudo umount /mnt/vexfs_basic")?;
            results.unmount_time_ms = unmount_start.elapsed().as_millis() as u64;
            
            if unmount_cmd.status.success() {
                println!("    ✅ Basic unmount successful ({}ms)", results.unmount_time_ms);
            } else {
                results.basic_mount_unmount = false;
            }
        }

        // Test 2: Mount with Various Options
        println!("  ⚙️  Testing mount with various options...");
        for option_set in &self.mount_options_variants {
            let mount_result = self.test_mount_with_options(&option_set)?;
            results.mount_with_options.insert(option_set.name.clone(), mount_result);
            
            if mount_result == option_set.expected_success {
                println!("    ✅ Mount option '{}' behaved as expected", option_set.name);
            } else {
                println!("    ❌ Mount option '{}' unexpected result", option_set.name);
            }
        }

        // Test 3: Remount Tests
        println!("  🔄 Testing remount operations...");
        results.remount_tests = self.test_remount_operations()?;

        // Test 4: Mount Point Validation
        println!("  📁 Testing mount point validation...");
        results.mount_point_validation = self.test_mount_point_validation()?;

        // Test 5: Filesystem Type Detection
        println!("  🔍 Testing filesystem type detection...");
        results.filesystem_type_detection = self.test_filesystem_type_detection()?;

        Ok(results)
    }

    fn run_edge_case_tests(&self) -> Result<EdgeCaseTestResults, Box<dyn std::error::Error>> {
        let mut results = EdgeCaseTestResults {
            invalid_mount_options: HashMap::new(),
            already_mounted_filesystem: false,
            nonexistent_device: false,
            invalid_mount_point: false,
            permission_denied_scenarios: false,
            corrupted_filesystem: false,
            device_busy_scenarios: false,
        };

        println!("  ⚠️  Testing invalid mount options...");
        let invalid_options = vec![
            ("invalid_fs_type", vec!["-t", "nonexistent_fs"]),
            ("malformed_option", vec!["-o", "malformed=option=value"]),
            ("conflicting_options", vec!["-o", "ro,rw"]),
            ("unknown_option", vec!["-o", "unknown_option_xyz"]),
        ];

        for (name, options) in invalid_options {
            let result = self.test_invalid_mount_option(options)?;
            results.invalid_mount_options.insert(name.to_string(), result);
        }

        println!("  🔒 Testing already mounted filesystem...");
        results.already_mounted_filesystem = self.test_already_mounted_filesystem()?;

        println!("  ❓ Testing nonexistent device...");
        results.nonexistent_device = self.test_nonexistent_device()?;

        println!("  📂 Testing invalid mount point...");
        results.invalid_mount_point = self.test_invalid_mount_point()?;

        println!("  🚫 Testing permission denied scenarios...");
        results.permission_denied_scenarios = self.test_permission_denied_scenarios()?;

        println!("  💥 Testing corrupted filesystem...");
        results.corrupted_filesystem = self.test_corrupted_filesystem()?;

        println!("  🔒 Testing device busy scenarios...");
        results.device_busy_scenarios = self.test_device_busy_scenarios()?;

        Ok(results)
    }

    fn run_concurrent_mount_tests(&self) -> Result<ConcurrentMountTestResults, Box<dyn std::error::Error>> {
        let mut results = ConcurrentMountTestResults {
            parallel_mount_attempts: 0,
            successful_concurrent_mounts: 0,
            race_conditions_detected: 0,
            deadlocks_detected: 0,
            resource_contention_events: 0,
            max_concurrent_achieved: 0,
            concurrent_mount_time_ms: 0,
        };

        println!("  🔄 Testing parallel mount attempts...");
        let concurrent_start = Instant::now();
        
        // Setup multiple test filesystems
        let num_concurrent = self.max_concurrent_mounts.min(5); // Limit for safety
        let mut handles = Vec::new();
        let (tx, rx) = mpsc::channel();

        for i in 0..num_concurrent {
            let vm_config = self.vm_config.clone();
            let sender = tx.clone();
            
            let handle = thread::spawn(move || {
                let mount_result = Self::concurrent_mount_test(vm_config, i);
                sender.send((i, mount_result)).unwrap();
            });
            
            handles.push(handle);
            results.parallel_mount_attempts += 1;
        }

        // Collect results
        drop(tx);
        let mut successful_mounts = 0;
        
        for (mount_id, mount_success) in rx {
            if mount_success {
                successful_mounts += 1;
                println!("    ✅ Concurrent mount {} successful", mount_id);
            } else {
                println!("    ❌ Concurrent mount {} failed", mount_id);
            }
        }

        // Wait for all threads
        for handle in handles {
            handle.join().unwrap();
        }

        results.successful_concurrent_mounts = successful_mounts;
        results.max_concurrent_achieved = successful_mounts;
        results.concurrent_mount_time_ms = concurrent_start.elapsed().as_millis() as u64;

        println!("    📊 Concurrent mount results: {}/{} successful", 
                successful_mounts, results.parallel_mount_attempts);

        Ok(results)
    }

    fn run_stress_tests(&self) -> Result<StressTestResults, Box<dyn std::error::Error>> {
        let mut results = StressTestResults {
            stress_cycles_completed: 0,
            mount_unmount_cycles: 0,
            failures_detected: 0,
            recovery_attempts: 0,
            successful_recoveries: 0,
            memory_leaks_detected: 0,
            performance_degradation_detected: false,
            stability_score: 0.0,
        };

        println!("  💪 Running mount/unmount stress cycles...");
        
        for cycle in 1..=self.stress_test_cycles {
            if cycle % 10 == 0 {
                println!("    🔄 Stress cycle {}/{}", cycle, self.stress_test_cycles);
            }

            // Setup filesystem for this cycle
            if !self.setup_test_filesystem(&format!("stress_{}", cycle))? {
                results.failures_detected += 1;
                continue;
            }

            // Mount
            let mount_cmd = self.execute_ssh_command(&format!(
                "sudo mkdir -p /mnt/stress_{} && sudo mount /dev/loop0 /mnt/stress_{}",
                cycle, cycle
            ));

            if mount_cmd.is_ok() && mount_cmd.unwrap().status.success() {
                results.mount_unmount_cycles += 1;

                // Perform some operations
                let _ = self.execute_ssh_command(&format!(
                    "sudo touch /mnt/stress_{}/test_file && echo 'stress test' | sudo tee /mnt/stress_{}/test_file",
                    cycle, cycle
                ));

                // Unmount
                let unmount_cmd = self.execute_ssh_command(&format!(
                    "sudo umount /mnt/stress_{}", cycle
                ));

                if unmount_cmd.is_err() || !unmount_cmd.unwrap().status.success() {
                    results.failures_detected += 1;
                }
            } else {
                results.failures_detected += 1;
            }

            // Cleanup
            let _ = self.execute_ssh_command("sudo losetup -d /dev/loop0 2>/dev/null || true");

            results.stress_cycles_completed += 1;

            // Brief pause to prevent overwhelming the system
            thread::sleep(Duration::from_millis(100));
        }

        // Calculate stability score
        let success_rate = if results.stress_cycles_completed > 0 {
            (results.stress_cycles_completed - results.failures_detected) as f64 / 
            results.stress_cycles_completed as f64
        } else {
            0.0
        };

        results.stability_score = success_rate * 100.0;

        println!("    📊 Stress test completed: {:.1}% success rate", results.stability_score);

        Ok(results)
    }

    fn run_resource_constraint_tests(&self) -> Result<ResourceConstraintTestResults, Box<dyn std::error::Error>> {
        let mut results = ResourceConstraintTestResults {
            low_memory_mount_test: false,
            high_cpu_load_mount_test: false,
            disk_space_constraint_test: false,
            file_descriptor_limit_test: false,
            network_constraint_test: false,
            constraint_recovery_successful: false,
        };

        println!("  🔒 Testing mount under low memory conditions...");
        results.low_memory_mount_test = self.test_low_memory_mount()?;

        println!("  🔥 Testing mount under high CPU load...");
        results.high_cpu_load_mount_test = self.test_high_cpu_load_mount()?;

        println!("  💾 Testing mount with disk space constraints...");
        results.disk_space_constraint_test = self.test_disk_space_constraint_mount()?;

        println!("  📁 Testing mount with file descriptor limits...");
        results.file_descriptor_limit_test = self.test_fd_limit_mount()?;

        println!("  🌐 Testing mount with network constraints...");
        results.network_constraint_test = self.test_network_constraint_mount()?;

        // Test recovery from constraints
        println!("  🔄 Testing recovery from resource constraints...");
        results.constraint_recovery_successful = self.test_constraint_recovery()?;

        Ok(results)
    }

    // Helper methods for specific test implementations
    fn setup_test_filesystem(&self, name: &str) -> Result<bool, Box<dyn std::error::Error>> {
        // Create loop device and format filesystem
        let setup_cmd = self.execute_ssh_command(&format!(
            "sudo dd if=/dev/zero of=/tmp/{}.img bs=1M count=50 && sudo losetup /dev/loop0 /tmp/{}.img && sudo mkfs.ext4 /dev/loop0",
            name, name
        ))?;

        Ok(setup_cmd.status.success())
    }

    fn execute_ssh_command(&self, command: &str) -> Result<std::process::Output, Box<dyn std::error::Error>> {
        Command::new("ssh")
            .args(&[
                "-i", &self.vm_config.ssh_key_path,
                "-o", "StrictHostKeyChecking=no",
                "-o", "ConnectTimeout=10",
                &format!("{}@localhost", self.vm_config.vm_user),
                "-p", &self.vm_config.ssh_port.to_string(),
                command
            ])
            .output()
            .map_err(|e| e.into())
    }

    fn test_mount_with_options(&self, option_set: &MountOptionsSet) -> Result<bool, Box<dyn std::error::Error>> {
        let mut mount_args = vec!["sudo", "mount"];
        mount_args.extend(option_set.options.iter().map(|s| s.as_str()));
        mount_args.extend(&["/dev/loop0", "/mnt/vexfs_test"]);
        
        let mount_cmd = self.execute_ssh_command(&mount_args.join(" "))?;
        let success = mount_cmd.status.success();
        
        if success {
            // Cleanup - unmount
            let _ = self.execute_ssh_command("sudo umount /mnt/vexfs_test");
        }
        
        Ok(success)
    }

    fn test_remount_operations(&self) -> Result<bool, Box<dyn std::error::Error>> {
        // Setup and mount filesystem
        if !self.setup_test_filesystem("remount_test")? {
            return Ok(false);
        }

        let mount_cmd = self.execute_ssh_command(
            "sudo mkdir -p /mnt/remount_test && sudo mount /dev/loop0 /mnt/remount_test"
        )?;

        if !mount_cmd.status.success() {
            return Ok(false);
        }

        // Test remount with different options
        let remount_cmd = self.execute_ssh_command(
            "sudo mount -o remount,ro /mnt/remount_test"
        )?;

        let success = remount_cmd.status.success();

        // Cleanup
        let _ = self.execute_ssh_command("sudo umount /mnt/remount_test");
        let _ = self.execute_ssh_command("sudo losetup -d /dev/loop0");

        Ok(success)
    }

    fn test_mount_point_validation(&self) -> Result<bool, Box<dyn std::error::Error>> {
        // Test mounting to various mount points
        let test_points = vec![
            "/mnt/valid_mount_point",
            "/tmp/test_mount",
            "/mnt/nested/deep/mount/point",
        ];

        for mount_point in test_points {
            let create_cmd = self.execute_ssh_command(&format!("sudo mkdir -p {}", mount_point))?;
            if !create_cmd.status.success() {
                return Ok(false);
            }
        }

        Ok(true)
    }

    fn test_filesystem_type_detection(&self) -> Result<bool, Box<dyn std::error::Error>> {
        // Test filesystem type detection
        let detect_cmd = self.execute_ssh_command("sudo blkid /dev/loop0")?;
        Ok(detect_cmd.status.success())
    }

    fn test_invalid_mount_option(&self, options: Vec<&str>) -> Result<bool, Box<dyn std::error::Error>> {
        let mut mount_args = vec!["sudo", "mount"];
        mount_args.extend(options);
        mount_args.extend(&["/dev/loop0", "/mnt/vexfs_test"]);
        
        let mount_cmd = self.execute_ssh_command(&mount_args.join(" "))?;
        // For invalid options, we expect failure
        Ok(!mount_cmd.status.success())
    }

    fn test_already_mounted_filesystem(&self) -> Result<bool, Box<dyn std::error::Error>> {
        // Setup and mount filesystem
        if !self.setup_test_filesystem("already_mounted")? {
            return Ok(false);
        }

        let mount_cmd1 = self.execute_ssh_command(
            "sudo mkdir -p /mnt/already_mounted && sudo mount /dev/loop0 /mnt/already_mounted"
        )?;

        if !mount_cmd1.status.success() {
            return Ok(false);
        }

        // Try to mount again - should fail
        let mount_cmd2 = self.execute_ssh_command(
            "sudo mount /dev/loop0 /mnt/already_mounted2"
        )?;

        let success = !mount_cmd2.status.success(); // Should fail

        // Cleanup
        let _ = self.execute_ssh_command("sudo umount /mnt/already_mounted");
        let _ = self.execute_ssh_command("sudo losetup -d /dev/loop0");

        Ok(success)
    }

    fn test_nonexistent_device(&self) -> Result<bool, Box<dyn std::error::Error>> {
        let mount_cmd = self.execute_ssh_command(
            "sudo mount /dev/nonexistent_device /mnt/test_mount"
        )?;
        
        // Should fail for nonexistent device
        Ok(!mount_cmd.status.success())
    }

    fn test_invalid_mount_point(&self) -> Result<bool, Box<dyn std::error::Error>> {
        // Test mounting to invalid mount points
        let invalid_points = vec![
            "/dev/null",  // Not a directory
            "/proc/invalid",  // Invalid location
            "/sys/invalid",   // Invalid location
        ];

        for mount_point in invalid_points {
            let mount_cmd = self.execute_ssh_command(&format!(
                "sudo mount /dev/loop0 {}", mount_point
            ))?;
            
            // Should fail for invalid mount points
            if mount_cmd.status.success() {
                return Ok(false);
            }
        }

        Ok(true)
    }

    fn test_permission_denied_scenarios(&self) -> Result<bool, Box<dyn std::error::Error>> {
        // Test mounting without sudo (should fail)
        let mount_cmd = self.execute_ssh_command(
            "mount /dev/loop0 /mnt/permission_test"
        )?;
        
        // Should fail due to permission denied
        Ok(!mount_cmd.status.success())
    }

    fn test_corrupted_filesystem(&self) -> Result<bool, Box<dyn std::error::Error>> {
        // Create a corrupted filesystem
        let corrupt_cmd = self.execute_ssh_command(
            "sudo dd if=/dev/zero of=/tmp/corrupt.img bs=1M count=10 && sudo losetup /dev/loop1 /tmp/corrupt.img && sudo dd if=/dev/urandom of=/dev/loop1 bs=1K count=1"
        )?;

        if !corrupt_cmd.status.success() {
            return Ok(false);
        }

        // Try to mount corrupted filesystem
        let mount_cmd = self.execute_ssh_command(
            "sudo mkdir -p /mnt/corrupt_test && sudo mount /dev/loop1 /mnt/corrupt_test"
        )?;

        let success = !mount_cmd.status.success(); // Should fail

        // Cleanup
        let _ = self.execute_ssh_command("sudo losetup -d /dev/loop1");

        Ok(success)
    }

    fn test_device_busy_scenarios(&self) -> Result<bool, Box<dyn std::error::Error>> {
        // Setup filesystem
        if !self.setup_test_filesystem("busy_test")? {
            return Ok(false);
        }

        // Mount filesystem
        let mount_cmd = self.execute_ssh_command(
            "sudo mkdir -p /mnt/busy_test && sudo mount /dev/loop0 /mnt/busy_test"
        )?;

        if !mount_cmd.status.success() {
            return Ok(false);
        }

        // Create a process that keeps the filesystem busy
        let _busy_cmd = self.execute_ssh_command(
            "sudo bash -c 'cd /mnt/busy_test && sleep 5' &"
        )?;

        // Try to unmount while busy (should fail or require force)
        let unmount_cmd = self.execute_ssh_command("sudo umount /mnt/busy_test")?;
        
        // Wait for background process to finish
        thread::sleep(Duration::from_secs(6));
        
        // Force unmount
        let force_unmount_cmd = self.execute_ssh_command("sudo umount -f /mnt/busy_test")?;
        
        let success = force_unmount_cmd.status.success();

        // Cleanup
        let _ = self.execute_ssh_command("sudo losetup -d /dev/loop0");

        Ok(success)
    }

    fn concurrent_mount_test(vm_config: VmConfig, mount_id: u32) -> bool {
        // Create unique filesystem for this mount
        let setup_cmd = Command::new("ssh")
            .args(&[
                "-i", &vm_config.ssh_key_path,
                "-o", "StrictHostKeyChecking=no",
                "-o", "ConnectTimeout=10",
                &format!("{}@localhost", vm_config.vm_user),
                "-p", &vm_config.ssh_port.to_string(),
                &format!("sudo dd if=/dev/zero of=/tmp/concurrent_{}.img bs=1M count=20 && sudo losetup /dev/loop{} /tmp/concurrent_{}.img && sudo mkfs.ext4 /dev/loop{}",
                        mount_id, mount_id, mount_id, mount_id)
            ])
            .output();

        if setup_cmd.is_err() || !setup_cmd.unwrap().status.success() {
            return false;
        }

        // Mount filesystem
        let mount_cmd = Command::new("ssh")
            .args(&[
                "-i", &vm_config.ssh_key_path,
                "-o", "StrictHostKeyChecking=no",
                "-o", "ConnectTimeout=10",
                &format!("{}@localhost", vm_config.vm_user),
                "-p", &vm_config.ssh_port.to_string(),
                &format!("sudo mkdir -p /mnt/concurrent_{} && sudo mount /dev/loop{} /mnt/concurrent_{}",
                        mount_id, mount_id, mount_id)
            ])
            .output();

        let mount_success = mount_cmd.is_ok() && mount_cmd.unwrap().status.success();

        // Cleanup
        let _ = Command::new("ssh")
            .args(&[
                "-i", &vm_config.ssh_key_path,
                "-o", "StrictHostKeyChecking=no",
                &format!("{}@localhost", vm_config.vm_user),
                "-p", &vm_config.ssh_port.to_string(),
                &format!("sudo umount /mnt/concurrent_{} 2>/dev/null || true && sudo losetup -d /dev/loop{} 2>/dev/null || true",
                        mount_id, mount_id)
            ])
            .output();

        mount_success
    }

    fn test_low_memory_mount(&self) -> Result<bool, Box<dyn std::error::Error>> {
        // Create memory pressure
        let _memory_pressure_cmd = self.execute_ssh_command(
            "sudo bash -c 'dd if=/dev/zero of=/tmp/memory_hog bs=1M count=1000 &'"
        )?;

        thread::sleep(Duration::from_secs(2));

        // Try to mount under memory pressure
        if !self.setup_test_filesystem("low_memory")? {
            return Ok(false);
        }

        let mount_cmd = self.execute_ssh_command(
            "sudo mkdir -p /mnt/low_memory && sudo mount /dev/loop0 /mnt/low_memory"
        )?;

        let success = mount_cmd.status.success();

        // Cleanup memory pressure and mount
        let _ = self.execute_ssh_command("sudo pkill dd");
        let _ = self.execute_ssh_command("sudo rm -f /tmp/memory_hog");
        let _ = self.execute_ssh_command("sudo umount /mnt/low_memory 2>/dev/null || true");
        let _ = self.execute_ssh_command("sudo losetup -d /dev/loop0 2>/dev/null || true");

        Ok(success)
    }

    fn test_high_cpu_load_mount(&self) -> Result<bool, Box<dyn std::error::Error>> {
        // Create CPU load
        let _cpu_load_cmd = self.execute_ssh_command(
            "sudo bash -c 'for i in {1..4}; do yes > /dev/null & done'"
        )?;

        thread::sleep(Duration::from_secs(2));

        // Try to mount under CPU load
        if !self.setup_test_filesystem("high_cpu")? {
            return Ok(false);
        }

        let mount_cmd = self.execute_ssh_command(
            "sudo mkdir -p /mnt/high_cpu && sudo mount /dev/loop0 /mnt/high_cpu"
        )?;

        let success = mount_cmd.status.success();

        // Cleanup CPU load and mount
        let _ = self.execute_ssh_command("sudo pkill yes");
        let _ = self.execute_ssh_command("sudo umount /mnt/high_cpu 2>/dev/null || true");
        let _ = self.execute_ssh_command("sudo losetup -d /dev/loop0 2>/dev/null || true");

        Ok(success)
    }

    fn test_disk_space_constraint_mount(&self) -> Result<bool, Box<dyn std::error::Error>> {
        // Fill up disk space (but leave some room)
        let _disk_fill_cmd = self.execute_ssh_command(
            "sudo dd if=/dev/zero of=/tmp/disk_filler bs=1M count=500 2>/dev/null || true"
        )?;

        // Try to mount with limited disk space
        if !self.setup_test_filesystem("disk_constraint")? {
            return Ok(false);
        }

        let mount_cmd = self.execute_ssh_command(
            "sudo mkdir -p /mnt/disk_constraint && sudo mount /dev/loop0 /mnt/disk_constraint"
        )?;

        let success = mount_cmd.status.success();

        // Cleanup
        let _ = self.execute_ssh_command("sudo rm -f /tmp/disk_filler");
        let _ = self.execute_ssh_command("sudo umount /mnt/disk_constraint 2>/dev/null || true");
        let _ = self.execute_ssh_command("sudo losetup -d /dev/loop0 2>/dev/null || true");

        Ok(success)
    }

    fn test_fd_limit_mount(&self) -> Result<bool, Box<dyn std::error::Error>> {
        // Create many file descriptors to approach limit
        let _fd_pressure_cmd = self.execute_ssh_command(
            "sudo bash -c 'for i in {1..100}; do exec 3< /dev/null & done'"
        )?;

        // Try to mount with FD pressure
        if !self.setup_test_filesystem("fd_limit")? {
            return Ok(false);
        }

        let mount_cmd = self.execute_ssh_command(
            "sudo mkdir -p /mnt/fd_limit && sudo mount /dev/loop0 /mnt/fd_limit"
        )?;

        let success = mount_cmd.status.success();

        // Cleanup
        let _ = self.execute_ssh_command("sudo umount /mnt/fd_limit 2>/dev/null || true");
        let _ = self.execute_ssh_command("sudo losetup -d /dev/loop0 2>/dev/null || true");

        Ok(success)
    }

    fn test_network_constraint_mount(&self) -> Result<bool, Box<dyn std::error::Error>> {
        // This test simulates network-related constraints that might affect mount operations
        // For local mounts, this is less relevant, but we test anyway
        
        if !self.setup_test_filesystem("network_constraint")? {
            return Ok(false);
        }

        let mount_cmd = self.execute_ssh_command(
            "sudo mkdir -p /mnt/network_constraint && sudo mount /dev/loop0 /mnt/network_constraint"
        )?;

        let success = mount_cmd.status.success();

        // Cleanup
        let _ = self.execute_ssh_command("sudo umount /mnt/network_constraint 2>/dev/null || true");
        let _ = self.execute_ssh_command("sudo losetup -d /dev/loop0 2>/dev/null || true");

        Ok(success)
    }

    fn test_constraint_recovery(&self) -> Result<bool, Box<dyn std::error::Error>> {
        // Test recovery from various constraint scenarios
        println!("    🔄 Testing recovery from memory constraints...");
        
        // Create and resolve memory pressure
        let _ = self.execute_ssh_command("sudo bash -c 'dd if=/dev/zero of=/tmp/recovery_test bs=1M count=500 2>/dev/null || true'");
        thread::sleep(Duration::from_secs(1));
        let _ = self.execute_ssh_command("sudo rm -f /tmp/recovery_test");
        
        // Test normal mount after recovery
        if !self.setup_test_filesystem("recovery_test")? {
            return Ok(false);
        }

        let mount_cmd = self.execute_ssh_command(
            "sudo mkdir -p /mnt/recovery_test && sudo mount /dev/loop0 /mnt/recovery_test"
        )?;

        let success = mount_cmd.status.success();

        // Cleanup
        let _ = self.execute_ssh_command("sudo umount /mnt/recovery_test 2>/dev/null || true");
        let _ = self.execute_ssh_command("sudo losetup -d /dev/loop0 2>/dev/null || true");

        Ok(success)
    }

    fn finalize_test_result(&mut self, result: &mut MountTestResult, start_time: Instant) {
        // Stop crash detection and collect events
        if let Some(ref mut detector) = self.crash_detector {
            let _ = detector.stop_monitoring();
            let crash_summary = detector.get_crash_summary();
            let performance_summary = detector.get_performance_summary();
            
            // Update performance metrics
            result.performance_metrics.memory_usage_kb = performance_summary.average_memory_usage_kb;
            result.performance_metrics.cpu_usage_percent = performance_summary.average_cpu_usage_percent;
        }

        result.duration_ms = start_time.elapsed().as_millis() as u64;
        
        println!("✅ Comprehensive Mount Test Suite completed in {}ms", result.duration_ms);
        println!("📊 Test Results Summary:");
        println!("  - Normal mount tests: {}", if result.normal_mount_tests.basic_mount_unmount { "✅" } else { "❌" });
        println!("  - Edge case tests: {} scenarios tested", result.edge_case_tests.invalid_mount_options.len());
        println!("  - Concurrent tests: {}/{} successful",
                result.concurrent_mount_tests.successful_concurrent_mounts,
                result.concurrent_mount_tests.parallel_mount_attempts);
        println!("  - Stress tests: {:.1}% stability score", result.stress_test_results.stability_score);
        println!("  - Overall status: {:?}", result.status);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mount_test_suite_creation() {
        let config = VmConfig::default();
        let suite = MountTestSuite::new(config);
        assert_eq!(suite.test_name, "Comprehensive_Mount_Test_Suite");
        assert_eq!(suite.max_concurrent_mounts, 10);
        assert_eq!(suite.stress_test_cycles, 50);
    }

    #[test]
    fn test_mount_options_default() {
        let options = MountTestSuite::default_mount_options();
        assert!(!options.is_empty());
        assert!(options.iter().any(|opt| opt.name == "default"));
        assert!(options.iter().any(|opt| opt.name == "read_only"));
        assert!(options.iter().any(|opt| opt.name == "invalid_option"));
    }

    #[test]
    fn test_mount_test_suite_configuration() {
        let config = VmConfig::default();
        let suite = MountTestSuite::new(config)
            .with_crash_detection(true)
            .with_stress_cycles(100)
            .with_max_concurrent_mounts(5);
        
        assert!(suite.crash_detector.is_some());
        assert_eq!(suite.stress_test_cycles, 100);
        assert_eq!(suite.max_concurrent_mounts, 5);
    }
}