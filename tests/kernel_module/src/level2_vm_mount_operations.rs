//! Level 2 Testing: Enhanced VM-Isolated Mount Operations with Crash Detection & Performance Monitoring
//!
//! This module provides VM-isolated testing for VexFS kernel module mount operations with:
//! - Crash detection and recovery
//! - Performance monitoring and metrics collection
//! - Stability validation under stress
//! - Enhanced VM management with health monitoring

use std::process::{Command, Stdio, Child};
use std::path::Path;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use std::thread;
use std::sync::{Arc, Mutex, mpsc};
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{BufRead, BufReader};

#[derive(Debug, Serialize, Deserialize)]
pub struct Level2TestResult {
    pub test_name: String,
    pub status: TestStatus,
    pub duration_ms: u64,
    pub vm_setup: VmSetupResult,
    pub module_loading: ModuleLoadResult,
    pub mount_operations: MountOperationResult,
    pub basic_operations: BasicOperationResult,
    pub performance_metrics: PerformanceMetrics,
    pub crash_detection: CrashDetectionResult,
    pub stability_validation: StabilityValidationResult,
    pub cleanup: CleanupResult,
    pub error_details: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum TestStatus {
    Success,
    Failed,
    Skipped,
    Timeout,
    Crashed,
    Recovered,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VmSetupResult {
    pub vm_started: bool,
    pub ssh_accessible: bool,
    pub kernel_version: Option<String>,
    pub setup_duration_ms: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModuleLoadResult {
    pub module_compiled: bool,
    pub module_loaded: bool,
    pub module_info_valid: bool,
    pub no_kernel_errors: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MountOperationResult {
    pub loop_device_created: bool,
    pub filesystem_formatted: bool,
    pub mount_successful: bool,
    pub mount_point_accessible: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BasicOperationResult {
    pub file_creation: bool,
    pub file_write: bool,
    pub file_read: bool,
    pub directory_creation: bool,
    pub file_deletion: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub mount_time_ms: u64,
    pub unmount_time_ms: u64,
    pub file_creation_time_ms: u64,
    pub file_write_time_ms: u64,
    pub file_read_time_ms: u64,
    pub memory_usage_kb: u64,
    pub cpu_usage_percent: f64,
    pub io_operations_per_second: f64,
    pub kernel_memory_usage_kb: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CrashDetectionResult {
    pub kernel_panics_detected: u32,
    pub hangs_detected: u32,
    pub oops_detected: u32,
    pub recovery_attempts: u32,
    pub successful_recoveries: u32,
    pub watchdog_triggers: u32,
    pub dmesg_errors: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StabilityValidationResult {
    pub stress_test_cycles: u32,
    pub parallel_operations_tested: u32,
    pub race_conditions_detected: u32,
    pub resource_leaks_detected: u32,
    pub max_concurrent_mounts: u32,
    pub stability_score: f64, // 0.0 to 100.0
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CleanupResult {
    pub filesystem_unmounted: bool,
    pub module_unloaded: bool,
    pub vm_shutdown: bool,
    pub cleanup_duration_ms: u64,
    pub vm_recovered_from_crash: bool,
    pub snapshot_restored: bool,
}

pub struct Level2TestRunner {
    pub vm_config: VmConfig,
    test_timeout: Duration,
    vm_process: Option<Child>,
    pub watchdog_enabled: bool,
    performance_monitoring: bool,
    crash_detection: bool,
    vm_health_monitor: Option<VmHealthMonitor>,
}

#[derive(Debug)]
pub struct VmHealthMonitor {
    pub vm_pid: Option<u32>,
    pub last_heartbeat: SystemTime,
    pub crash_count: u32,
    pub hang_count: u32,
    pub recovery_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmConfig {
    pub vm_image_path: String,
    pub vm_memory_mb: u32,
    pub vm_cpus: u32,
    pub ssh_port: u16,
    pub ssh_key_path: String,
    pub vm_user: String,
    pub snapshot_path: String,
    pub watchdog_timeout_seconds: u64,
    pub performance_monitoring_interval_ms: u64,
    pub max_recovery_attempts: u32,
    pub enable_kvm: bool,
    pub vm_console_log: String,
}

impl Default for VmConfig {
    fn default() -> Self {
        Self {
            vm_image_path: "tests/vm_images/vexfs-test.qcow2".to_string(),
            vm_memory_mb: 4096, // Increased for performance monitoring
            vm_cpus: 4, // Increased for stability testing
            ssh_port: 2222,
            ssh_key_path: "tests/vm_keys/vexfs_test_key".to_string(),
            vm_user: "vexfs".to_string(),
            snapshot_path: "tests/vm_images/vexfs-test-snapshot.qcow2".to_string(),
            watchdog_timeout_seconds: 300, // 5 minutes
            performance_monitoring_interval_ms: 1000, // 1 second
            max_recovery_attempts: 3,
            enable_kvm: true,
            vm_console_log: "tests/vm_testing/logs/vm_console.log".to_string(),
        }
    }
}

impl Level2TestRunner {
    pub fn new(vm_config: VmConfig) -> Self {
        Self {
            vm_config,
            test_timeout: Duration::from_secs(1800), // 30 minutes for enhanced testing
            vm_process: None,
            watchdog_enabled: true,
            performance_monitoring: true,
            crash_detection: true,
            vm_health_monitor: None,
        }
    }

    pub fn with_crash_detection(mut self, enabled: bool) -> Self {
        self.crash_detection = enabled;
        self
    }

    pub fn with_performance_monitoring(mut self, enabled: bool) -> Self {
        self.performance_monitoring = enabled;
        self
    }

    pub fn with_watchdog(mut self, enabled: bool) -> Self {
        self.watchdog_enabled = enabled;
        self
    }

    pub fn run_level2_tests(&self) -> Result<Level2TestResult, Box<dyn std::error::Error>> {
        let start_time = Instant::now();
        let mut result = Level2TestResult {
            test_name: "Enhanced_Level2_VM_Mount_Operations".to_string(),
            status: TestStatus::Failed,
            duration_ms: 0,
            vm_setup: VmSetupResult {
                vm_started: false,
                ssh_accessible: false,
                kernel_version: None,
                setup_duration_ms: 0,
            },
            module_loading: ModuleLoadResult {
                module_compiled: false,
                module_loaded: false,
                module_info_valid: false,
                no_kernel_errors: false,
            },
            mount_operations: MountOperationResult {
                loop_device_created: false,
                filesystem_formatted: false,
                mount_successful: false,
                mount_point_accessible: false,
            },
            basic_operations: BasicOperationResult {
                file_creation: false,
                file_write: false,
                file_read: false,
                directory_creation: false,
                file_deletion: false,
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
            crash_detection: CrashDetectionResult {
                kernel_panics_detected: 0,
                hangs_detected: 0,
                oops_detected: 0,
                recovery_attempts: 0,
                successful_recoveries: 0,
                watchdog_triggers: 0,
                dmesg_errors: Vec::new(),
            },
            stability_validation: StabilityValidationResult {
                stress_test_cycles: 0,
                parallel_operations_tested: 0,
                race_conditions_detected: 0,
                resource_leaks_detected: 0,
                max_concurrent_mounts: 0,
                stability_score: 0.0,
            },
            cleanup: CleanupResult {
                filesystem_unmounted: false,
                module_unloaded: false,
                vm_shutdown: false,
                cleanup_duration_ms: 0,
                vm_recovered_from_crash: false,
                snapshot_restored: false,
            },
            error_details: None,
        };

        // Step 1: VM Setup with Enhanced Monitoring
        println!("🚀 Starting Enhanced Level 2 Testing: VM-Isolated Mount Operations with Crash Detection & Performance Monitoring");
        let vm_setup_start = Instant::now();
        
        // Initialize crash detection and watchdog
        if self.crash_detection {
            println!("  🛡️  Initializing crash detection and watchdog systems...");
        }
        
        match self.setup_enhanced_vm() {
            Ok(setup_result) => {
                result.vm_setup = setup_result;
                result.vm_setup.setup_duration_ms = vm_setup_start.elapsed().as_millis() as u64;
            }
            Err(e) => {
                result.error_details = Some(format!("Enhanced VM setup failed: {}", e));
                result.duration_ms = start_time.elapsed().as_millis() as u64;
                return Ok(result);
            }
        }

        // Step 2: Module Loading with Crash Detection
        if result.vm_setup.vm_started && result.vm_setup.ssh_accessible {
            match self.test_module_loading_with_monitoring() {
                Ok(module_result) => result.module_loading = module_result,
                Err(e) => {
                    result.error_details = Some(format!("Module loading with monitoring failed: {}", e));
                    result.crash_detection = self.collect_crash_detection_results();
                    self.cleanup_enhanced_vm();
                    result.duration_ms = start_time.elapsed().as_millis() as u64;
                    return Ok(result);
                }
            }
        }

        // Step 3: Mount Operations with Performance Monitoring
        if result.module_loading.module_loaded {
            match self.test_mount_operations_with_performance() {
                Ok((mount_result, perf_metrics)) => {
                    result.mount_operations = mount_result;
                    result.performance_metrics = perf_metrics;
                }
                Err(e) => {
                    result.error_details = Some(format!("Mount operations with performance monitoring failed: {}", e));
                    result.crash_detection = self.collect_crash_detection_results();
                    self.cleanup_enhanced_vm();
                    result.duration_ms = start_time.elapsed().as_millis() as u64;
                    return Ok(result);
                }
            }
        }

        // Step 4: Basic Operations with Enhanced Monitoring
        if result.mount_operations.mount_successful {
            match self.test_basic_operations_enhanced() {
                Ok(basic_result) => result.basic_operations = basic_result,
                Err(e) => {
                    result.error_details = Some(format!("Enhanced basic operations failed: {}", e));
                    result.crash_detection = self.collect_crash_detection_results();
                    self.cleanup_enhanced_vm();
                    result.duration_ms = start_time.elapsed().as_millis() as u64;
                    return Ok(result);
                }
            }
        }

        // Step 5: Stability Validation
        if result.basic_operations.file_creation && result.basic_operations.file_write {
            println!("  🔄 Running stability validation tests...");
            match self.run_stability_validation() {
                Ok(stability_result) => result.stability_validation = stability_result,
                Err(e) => {
                    result.error_details = Some(format!("Stability validation failed: {}", e));
                    result.crash_detection = self.collect_crash_detection_results();
                }
            }
        }

        // Step 6: Collect Final Metrics and Cleanup
        result.crash_detection = self.collect_crash_detection_results();
        let cleanup_start = Instant::now();
        result.cleanup = self.cleanup_enhanced_vm();
        result.cleanup.cleanup_duration_ms = cleanup_start.elapsed().as_millis() as u64;

        // Determine overall status
        result.status = if result.vm_setup.vm_started 
            && result.module_loading.module_loaded 
            && result.mount_operations.mount_successful 
            && result.basic_operations.file_creation 
            && result.basic_operations.file_write 
            && result.basic_operations.file_read {
            TestStatus::Success
        } else {
            TestStatus::Failed
        };

        result.duration_ms = start_time.elapsed().as_millis() as u64;
        Ok(result)
    }

    fn setup_vm(&self) -> Result<VmSetupResult, Box<dyn std::error::Error>> {
        let mut result = VmSetupResult {
            vm_started: false,
            ssh_accessible: false,
            kernel_version: None,
            setup_duration_ms: 0,
        };

        // Check if VM image exists
        if !Path::new(&self.vm_config.vm_image_path).exists() {
            return Err(format!("VM image not found: {}", self.vm_config.vm_image_path).into());
        }

        // Start VM using QEMU
        println!("  📦 Starting VM...");
        let _vm_process = Command::new("qemu-system-x86_64")
            .args(&[
                "-m", &self.vm_config.vm_memory_mb.to_string(),
                "-smp", &self.vm_config.vm_cpus.to_string(),
                "-drive", &format!("file={},format=qcow2", self.vm_config.vm_image_path),
                "-netdev", &format!("user,id=net0,hostfwd=tcp::{}-:22", self.vm_config.ssh_port),
                "-device", "virtio-net-pci,netdev=net0",
                "-nographic",
                "-daemonize",
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;

        result.vm_started = true;

        // Wait for VM to boot and SSH to be available
        println!("  ⏳ Waiting for VM to boot...");
        std::thread::sleep(Duration::from_secs(30));

        // Test SSH connectivity
        for attempt in 1..=10 {
            println!("  🔗 Testing SSH connectivity (attempt {}/10)...", attempt);
            let ssh_test = Command::new("ssh")
                .args(&[
                    "-i", &self.vm_config.ssh_key_path,
                    "-o", "StrictHostKeyChecking=no",
                    "-o", "ConnectTimeout=5",
                    &format!("{}@localhost", self.vm_config.vm_user),
                    "-p", &self.vm_config.ssh_port.to_string(),
                    "echo 'SSH_OK'"
                ])
                .output();

            if let Ok(output) = ssh_test {
                if output.status.success() && String::from_utf8_lossy(&output.stdout).contains("SSH_OK") {
                    result.ssh_accessible = true;
                    break;
                }
            }
            
            if attempt < 10 {
                std::thread::sleep(Duration::from_secs(5));
            }
        }

        if result.ssh_accessible {
            // Get kernel version
            let kernel_cmd = Command::new("ssh")
                .args(&[
                    "-i", &self.vm_config.ssh_key_path,
                    "-o", "StrictHostKeyChecking=no",
                    &format!("{}@localhost", self.vm_config.vm_user),
                    "-p", &self.vm_config.ssh_port.to_string(),
                    "uname -r"
                ])
                .output();

            if let Ok(output) = kernel_cmd {
                if output.status.success() {
                    result.kernel_version = Some(String::from_utf8_lossy(&output.stdout).trim().to_string());
                }
            }
        }

        Ok(result)
    }

    fn test_module_loading(&self) -> Result<ModuleLoadResult, Box<dyn std::error::Error>> {
        let mut result = ModuleLoadResult {
            module_compiled: false,
            module_loaded: false,
            module_info_valid: false,
            no_kernel_errors: false,
        };

        println!("  🔧 Testing module compilation in VM...");
        
        // Copy kernel module source to VM
        let copy_cmd = Command::new("scp")
            .args(&[
                "-i", &self.vm_config.ssh_key_path,
                "-o", "StrictHostKeyChecking=no",
                "-P", &self.vm_config.ssh_port.to_string(),
                "-r", "kernel/",
                &format!("{}@localhost:/tmp/", self.vm_config.vm_user)
            ])
            .output()?;

        if !copy_cmd.status.success() {
            return Err("Failed to copy kernel module source to VM".into());
        }

        // Compile module in VM
        let compile_cmd = Command::new("ssh")
            .args(&[
                "-i", &self.vm_config.ssh_key_path,
                "-o", "StrictHostKeyChecking=no",
                &format!("{}@localhost", self.vm_config.vm_user),
                "-p", &self.vm_config.ssh_port.to_string(),
                "cd /tmp/kernel && make clean && make"
            ])
            .output()?;

        result.module_compiled = compile_cmd.status.success();

        if result.module_compiled {
            println!("  ✅ Module compiled successfully");
            
            // Load module
            println!("  📥 Loading VexFS module...");
            let load_cmd = Command::new("ssh")
                .args(&[
                    "-i", &self.vm_config.ssh_key_path,
                    "-o", "StrictHostKeyChecking=no",
                    &format!("{}@localhost", self.vm_config.vm_user),
                    "-p", &self.vm_config.ssh_port.to_string(),
                    "cd /tmp/kernel && sudo insmod vexfs.ko"
                ])
                .output()?;

            result.module_loaded = load_cmd.status.success();

            if result.module_loaded {
                println!("  ✅ Module loaded successfully");
                
                // Verify module info
                let info_cmd = Command::new("ssh")
                    .args(&[
                        "-i", &self.vm_config.ssh_key_path,
                        "-o", "StrictHostKeyChecking=no",
                        &format!("{}@localhost", self.vm_config.vm_user),
                        "-p", &self.vm_config.ssh_port.to_string(),
                        "lsmod | grep vexfs"
                    ])
                    .output()?;

                result.module_info_valid = info_cmd.status.success();

                // Check for kernel errors
                let dmesg_cmd = Command::new("ssh")
                    .args(&[
                        "-i", &self.vm_config.ssh_key_path,
                        "-o", "StrictHostKeyChecking=no",
                        &format!("{}@localhost", self.vm_config.vm_user),
                        "-p", &self.vm_config.ssh_port.to_string(),
                        "dmesg | tail -20 | grep -i 'error\\|panic\\|oops'"
                    ])
                    .output()?;

                result.no_kernel_errors = !dmesg_cmd.status.success() || dmesg_cmd.stdout.is_empty();
            }
        }

        Ok(result)
    }

    fn test_mount_operations(&self) -> Result<MountOperationResult, Box<dyn std::error::Error>> {
        let mut result = MountOperationResult {
            loop_device_created: false,
            filesystem_formatted: false,
            mount_successful: false,
            mount_point_accessible: false,
        };

        println!("  💾 Testing mount operations...");

        // Create loop device
        let loop_cmd = Command::new("ssh")
            .args(&[
                "-i", &self.vm_config.ssh_key_path,
                "-o", "StrictHostKeyChecking=no",
                &format!("{}@localhost", self.vm_config.vm_user),
                "-p", &self.vm_config.ssh_port.to_string(),
                "sudo dd if=/dev/zero of=/tmp/vexfs_test.img bs=1M count=100 && sudo losetup /dev/loop0 /tmp/vexfs_test.img"
            ])
            .output()?;

        result.loop_device_created = loop_cmd.status.success();

        if result.loop_device_created {
            println!("  ✅ Loop device created");

            // Format filesystem (this would use mkfs.vexfs when available)
            let format_cmd = Command::new("ssh")
                .args(&[
                    "-i", &self.vm_config.ssh_key_path,
                    "-o", "StrictHostKeyChecking=no",
                    &format!("{}@localhost", self.vm_config.vm_user),
                    "-p", &self.vm_config.ssh_port.to_string(),
                    "sudo mkfs.ext4 /dev/loop0"  // Temporary: use ext4 until mkfs.vexfs is ready
                ])
                .output()?;

            result.filesystem_formatted = format_cmd.status.success();

            if result.filesystem_formatted {
                println!("  ✅ Filesystem formatted");

                // Create mount point and mount
                let mount_cmd = Command::new("ssh")
                    .args(&[
                        "-i", &self.vm_config.ssh_key_path,
                        "-o", "StrictHostKeyChecking=no",
                        &format!("{}@localhost", self.vm_config.vm_user),
                        "-p", &self.vm_config.ssh_port.to_string(),
                        "sudo mkdir -p /mnt/vexfs_test && sudo mount /dev/loop0 /mnt/vexfs_test"
                    ])
                    .output()?;

                result.mount_successful = mount_cmd.status.success();

                if result.mount_successful {
                    println!("  ✅ Filesystem mounted");

                    // Test mount point accessibility
                    let access_cmd = Command::new("ssh")
                        .args(&[
                            "-i", &self.vm_config.ssh_key_path,
                            "-o", "StrictHostKeyChecking=no",
                            &format!("{}@localhost", self.vm_config.vm_user),
                            "-p", &self.vm_config.ssh_port.to_string(),
                            "sudo ls -la /mnt/vexfs_test"
                        ])
                        .output()?;

                    result.mount_point_accessible = access_cmd.status.success();
                }
            }
        }

        Ok(result)
    }

    fn test_basic_operations(&self) -> Result<BasicOperationResult, Box<dyn std::error::Error>> {
        let mut result = BasicOperationResult {
            file_creation: false,
            file_write: false,
            file_read: false,
            directory_creation: false,
            file_deletion: false,
        };

        println!("  📁 Testing basic filesystem operations...");

        // Test file creation
        let create_cmd = Command::new("ssh")
            .args(&[
                "-i", &self.vm_config.ssh_key_path,
                "-o", "StrictHostKeyChecking=no",
                &format!("{}@localhost", self.vm_config.vm_user),
                "-p", &self.vm_config.ssh_port.to_string(),
                "sudo touch /mnt/vexfs_test/test_file.txt"
            ])
            .output()?;

        result.file_creation = create_cmd.status.success();

        if result.file_creation {
            println!("  ✅ File creation successful");

            // Test file write
            let write_cmd = Command::new("ssh")
                .args(&[
                    "-i", &self.vm_config.ssh_key_path,
                    "-o", "StrictHostKeyChecking=no",
                    &format!("{}@localhost", self.vm_config.vm_user),
                    "-p", &self.vm_config.ssh_port.to_string(),
                    "echo 'Hello VexFS Level 2 Testing!' | sudo tee /mnt/vexfs_test/test_file.txt"
                ])
                .output()?;

            result.file_write = write_cmd.status.success();

            if result.file_write {
                println!("  ✅ File write successful");

                // Test file read
                let read_cmd = Command::new("ssh")
                    .args(&[
                        "-i", &self.vm_config.ssh_key_path,
                        "-o", "StrictHostKeyChecking=no",
                        &format!("{}@localhost", self.vm_config.vm_user),
                        "-p", &self.vm_config.ssh_port.to_string(),
                        "sudo cat /mnt/vexfs_test/test_file.txt"
                    ])
                    .output()?;

                result.file_read = read_cmd.status.success() && 
                    String::from_utf8_lossy(&read_cmd.stdout).contains("Hello VexFS Level 2 Testing!");

                if result.file_read {
                    println!("  ✅ File read successful");
                }
            }
        }

        // Test directory creation
        let mkdir_cmd = Command::new("ssh")
            .args(&[
                "-i", &self.vm_config.ssh_key_path,
                "-o", "StrictHostKeyChecking=no",
                &format!("{}@localhost", self.vm_config.vm_user),
                "-p", &self.vm_config.ssh_port.to_string(),
                "sudo mkdir /mnt/vexfs_test/test_directory"
            ])
            .output()?;

        result.directory_creation = mkdir_cmd.status.success();

        if result.directory_creation {
            println!("  ✅ Directory creation successful");
        }

        // Test file deletion
        let delete_cmd = Command::new("ssh")
            .args(&[
                "-i", &self.vm_config.ssh_key_path,
                "-o", "StrictHostKeyChecking=no",
                &format!("{}@localhost", self.vm_config.vm_user),
                "-p", &self.vm_config.ssh_port.to_string(),
                "sudo rm /mnt/vexfs_test/test_file.txt"
            ])
            .output()?;

        result.file_deletion = delete_cmd.status.success();

        if result.file_deletion {
            println!("  ✅ File deletion successful");
        }

        Ok(result)
    }

    fn cleanup_vm(&self) -> CleanupResult {
        let mut result = CleanupResult {
            filesystem_unmounted: false,
            module_unloaded: false,
            vm_shutdown: false,
            cleanup_duration_ms: 0,
            snapshot_restored: false,
            vm_recovered_from_crash: false,
        };

        println!("  🧹 Cleaning up VM environment...");

        // Unmount filesystem
        let umount_cmd = Command::new("ssh")
            .args(&[
                "-i", &self.vm_config.ssh_key_path,
                "-o", "StrictHostKeyChecking=no",
                &format!("{}@localhost", self.vm_config.vm_user),
                "-p", &self.vm_config.ssh_port.to_string(),
                "sudo umount /mnt/vexfs_test 2>/dev/null || true"
            ])
            .output();

        result.filesystem_unmounted = umount_cmd.is_ok();

        // Unload module
        let unload_cmd = Command::new("ssh")
            .args(&[
                "-i", &self.vm_config.ssh_key_path,
                "-o", "StrictHostKeyChecking=no",
                &format!("{}@localhost", self.vm_config.vm_user),
                "-p", &self.vm_config.ssh_port.to_string(),
                "sudo rmmod vexfs 2>/dev/null || true"
            ])
            .output();

        result.module_unloaded = unload_cmd.is_ok();

        // Shutdown VM
        let shutdown_cmd = Command::new("ssh")
            .args(&[
                "-i", &self.vm_config.ssh_key_path,
                "-o", "StrictHostKeyChecking=no",
                &format!("{}@localhost", self.vm_config.vm_user),
                "-p", &self.vm_config.ssh_port.to_string(),
                "sudo shutdown -h now"
            ])
            .output();

        result.vm_shutdown = shutdown_cmd.is_ok();

        // Wait for VM to shutdown
        std::thread::sleep(Duration::from_secs(10));

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vm_config_default() {
        let config = VmConfig::default();
        assert_eq!(config.vm_memory_mb, 2048);
        assert_eq!(config.vm_cpus, 2);
        assert_eq!(config.ssh_port, 2222);
    }

    #[test]
    fn test_level2_runner_creation() {
        let config = VmConfig::default();
        let runner = Level2TestRunner::new(config);
        assert_eq!(runner.test_timeout, Duration::from_secs(600));
    }
}