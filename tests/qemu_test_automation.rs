//! QEMU-based Automated Test Execution
//!
//! This module provides automated test execution in QEMU environments for comprehensive
//! kernel module testing and validation

use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use std::fs;
use std::path::Path;
use std::collections::HashMap;

/// QEMU test environment configuration
#[derive(Debug, Clone)]
pub struct QemuTestConfig {
    pub vm_memory: String,
    pub vm_cpus: u32,
    pub vm_disk_size: String,
    pub ssh_port: u16,
    pub ssh_key_path: String,
    pub vm_user: String,
    pub vm_host: String,
    pub test_timeout: Duration,
    pub boot_timeout: Duration,
    pub enable_kvm: bool,
    pub enable_graphics: bool,
    pub source_mount_path: String,
    pub vm_scripts_path: String,
}

impl Default for QemuTestConfig {
    fn default() -> Self {
        Self {
            vm_memory: "2G".to_string(),
            vm_cpus: 2,
            vm_disk_size: "20G".to_string(),
            ssh_port: 2222,
            ssh_key_path: "test_env/vm/keys/vexfs_vm_key".to_string(),
            vm_user: "vexfs".to_string(),
            vm_host: "localhost".to_string(),
            test_timeout: Duration::from_secs(300), // 5 minutes
            boot_timeout: Duration::from_secs(120), // 2 minutes
            enable_kvm: true,
            enable_graphics: false,
            source_mount_path: "/mnt/vexfs_source".to_string(),
            vm_scripts_path: "test_env".to_string(),
        }
    }
}

/// QEMU test result
#[derive(Debug, Clone)]
pub struct QemuTestResult {
    pub test_name: String,
    pub success: bool,
    pub execution_time: Duration,
    pub output: String,
    pub error_output: String,
    pub exit_code: Option<i32>,
}

impl QemuTestResult {
    pub fn new(test_name: &str) -> Self {
        Self {
            test_name: test_name.to_string(),
            success: false,
            execution_time: Duration::ZERO,
            output: String::new(),
            error_output: String::new(),
            exit_code: None,
        }
    }

    pub fn with_success(mut self, output: String, execution_time: Duration) -> Self {
        self.success = true;
        self.output = output;
        self.execution_time = execution_time;
        self.exit_code = Some(0);
        self
    }

    pub fn with_failure(mut self, error: String, execution_time: Duration, exit_code: Option<i32>) -> Self {
        self.success = false;
        self.error_output = error;
        self.execution_time = execution_time;
        self.exit_code = exit_code;
        self
    }
}

/// QEMU test automation system
pub struct QemuTestAutomation {
    config: QemuTestConfig,
    vm_running: bool,
    test_results: HashMap<String, QemuTestResult>,
}

impl QemuTestAutomation {
    pub fn new() -> Self {
        Self {
            config: QemuTestConfig::default(),
            vm_running: false,
            test_results: HashMap::new(),
        }
    }

    pub fn with_config(mut self, config: QemuTestConfig) -> Self {
        self.config = config;
        self
    }

    /// Check if QEMU test environment is available
    pub fn check_environment(&self) -> Result<(), String> {
        // Check if QEMU is installed
        if !self.check_qemu_available() {
            return Err("QEMU is not available".to_string());
        }

        // Check if test scripts exist
        if !Path::new(&format!("{}/run_qemu.sh", self.config.vm_scripts_path)).exists() {
            return Err("QEMU test scripts not found".to_string());
        }

        // Check if SSH key exists
        if !Path::new(&self.config.ssh_key_path).exists() {
            return Err("SSH key for VM access not found".to_string());
        }

        Ok(())
    }

    /// Start QEMU VM for testing
    pub fn start_vm(&mut self) -> Result<(), String> {
        if self.vm_running {
            return Ok(());
        }

        println!("🚀 Starting QEMU VM for testing...");

        let start_time = Instant::now();

        // Execute VM startup script
        let output = Command::new("bash")
            .arg(format!("{}/run_qemu.sh", self.config.vm_scripts_path))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to start VM: {}", e))?;

        // Wait for VM to boot
        self.wait_for_vm_boot()?;

        self.vm_running = true;
        println!("✅ VM started successfully in {:?}", start_time.elapsed());

        Ok(())
    }

    /// Stop QEMU VM
    pub fn stop_vm(&mut self) -> Result<(), String> {
        if !self.vm_running {
            return Ok(());
        }

        println!("🛑 Stopping QEMU VM...");

        // Execute VM shutdown script
        let output = Command::new("bash")
            .arg(format!("{}/vm_control.sh", self.config.vm_scripts_path))
            .arg("stop")
            .output()
            .map_err(|e| format!("Failed to stop VM: {}", e))?;

        if !output.status.success() {
            return Err(format!("VM shutdown failed: {}", String::from_utf8_lossy(&output.stderr)));
        }

        self.vm_running = false;
        println!("✅ VM stopped successfully");

        Ok(())
    }

    /// Wait for VM to boot and become accessible
    fn wait_for_vm_boot(&self) -> Result<(), String> {
        println!("⏳ Waiting for VM to boot...");

        let start_time = Instant::now();
        let mut attempts = 0;
        const MAX_ATTEMPTS: u32 = 60; // 2 minutes with 2-second intervals

        while start_time.elapsed() < self.config.boot_timeout && attempts < MAX_ATTEMPTS {
            if self.test_vm_connectivity().is_ok() {
                println!("✅ VM is ready for testing");
                return Ok(());
            }

            std::thread::sleep(Duration::from_secs(2));
            attempts += 1;
            
            if attempts % 15 == 0 {
                println!("⏳ Still waiting for VM... ({} seconds)", start_time.elapsed().as_secs());
            }
        }

        Err("VM boot timeout - VM did not become accessible".to_string())
    }

    /// Test VM connectivity via SSH
    fn test_vm_connectivity(&self) -> Result<(), String> {
        let output = Command::new("ssh")
            .args(&[
                "-i", &self.config.ssh_key_path,
                "-o", "StrictHostKeyChecking=no",
                "-o", "ConnectTimeout=5",
                "-o", "BatchMode=yes",
                &format!("{}@{}", self.config.vm_user, self.config.vm_host),
                "-p", &self.config.ssh_port.to_string(),
                "echo 'VM_READY'"
            ])
            .output()
            .map_err(|e| format!("SSH connection failed: {}", e))?;

        if output.status.success() && String::from_utf8_lossy(&output.stdout).contains("VM_READY") {
            Ok(())
        } else {
            Err("VM not ready".to_string())
        }
    }

    /// Execute command in VM via SSH
    fn execute_in_vm(&self, command: &str) -> Result<QemuTestResult, String> {
        let start_time = Instant::now();

        let output = Command::new("ssh")
            .args(&[
                "-i", &self.config.ssh_key_path,
                "-o", "StrictHostKeyChecking=no",
                &format!("{}@{}", self.config.vm_user, self.config.vm_host),
                "-p", &self.config.ssh_port.to_string(),
                &format!("cd {} && {}", self.config.source_mount_path, command)
            ])
            .output()
            .map_err(|e| format!("Failed to execute command in VM: {}", e))?;

        let execution_time = start_time.elapsed();
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if output.status.success() {
            Ok(QemuTestResult::new("vm_command").with_success(stdout, execution_time))
        } else {
            Ok(QemuTestResult::new("vm_command").with_failure(
                stderr,
                execution_time,
                output.status.code()
            ))
        }
    }

    /// Run comprehensive test suite in VM
    pub fn run_comprehensive_tests(&mut self) -> Result<HashMap<String, QemuTestResult>, String> {
        if !self.vm_running {
            self.start_vm()?;
        }

        println!("🧪 Running comprehensive tests in QEMU VM");
        println!("==========================================");

        // Test 1: Verify VM environment
        self.run_test("vm_environment_check", "pwd && ls -la && uname -a")?;

        // Test 2: Rust compilation
        self.run_test("rust_compilation", "cargo check --quiet")?;

        // Test 3: Unit tests
        self.run_test("unit_tests", "cargo test --lib --quiet")?;

        // Test 4: Vector operations
        self.run_test("vector_operations", "timeout 30 cargo run --bin vector_test_runner --quiet")?;

        // Test 5: FFI integration
        self.run_test("ffi_integration", "./test_ffi")?;

        // Test 6: Kernel module build
        self.run_test("kernel_module_build", "make clean > /dev/null 2>&1 && make > /dev/null 2>&1 && ls -la vexfs.ko")?;

        // Test 7: Module loading
        self.run_test("module_loading", "sudo insmod vexfs.ko && lsmod | grep vexfs")?;

        // Test 8: Module functionality
        self.run_test("module_functionality", "sudo dmesg | tail -10")?;

        // Test 9: Module unloading
        self.run_test("module_unloading", "sudo rmmod vexfs")?;

        // Test 10: Performance benchmarks
        self.run_test("performance_benchmarks", "timeout 60 cargo run --bin vector_benchmark --quiet")?;

        // Test 11: Integration tests
        self.run_test("integration_tests", "cargo test --test '*' --quiet")?;

        // Test 12: Memory usage check
        self.run_test("memory_usage", "free -h && cat /proc/meminfo | grep -E '(MemTotal|MemFree|MemAvailable)'")?;

        // Test 13: System stability
        self.run_test("system_stability", "uptime && cat /proc/loadavg")?;

        println!();
        self.print_test_summary();

        Ok(self.test_results.clone())
    }

    /// Run a single test in VM
    fn run_test(&mut self, test_name: &str, command: &str) -> Result<(), String> {
        print!("Running {}: ", test_name);

        let result = self.execute_in_vm(command)?;
        
        if result.success {
            println!("✅ PASSED ({:?})", result.execution_time);
        } else {
            println!("❌ FAILED: {}", result.error_output.lines().next().unwrap_or("Unknown error"));
        }

        self.test_results.insert(test_name.to_string(), result);
        Ok(())
    }

    /// Print test summary
    fn print_test_summary(&self) {
        let total_tests = self.test_results.len();
        let passed_tests = self.test_results.values().filter(|r| r.success).count();
        let failed_tests = total_tests - passed_tests;

        println!("📊 QEMU Test Summary");
        println!("===================");
        println!("Total Tests: {}", total_tests);
        println!("✅ Passed: {}", passed_tests);
        println!("❌ Failed: {}", failed_tests);
        println!("Success Rate: {:.1}%", (passed_tests as f64 / total_tests as f64) * 100.0);

        if failed_tests > 0 {
            println!();
            println!("❌ Failed Tests:");
            for (name, result) in &self.test_results {
                if !result.success {
                    println!("  • {}: {}", name, 
                        result.error_output.lines().next().unwrap_or("Unknown error"));
                }
            }
        }

        println!();
        println!("⏱️  Total Execution Time: {:?}", 
            self.test_results.values().map(|r| r.execution_time).sum::<Duration>());
    }

    /// Generate detailed test report
    pub fn generate_report(&self, output_path: &str) -> Result<(), String> {
        let mut report = String::new();
        
        report.push_str("# VexFS QEMU Test Report\n\n");
        report.push_str(&format!("Generated: {}\n\n", chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")));
        
        report.push_str("## Test Configuration\n\n");
        report.push_str(&format!("- VM Memory: {}\n", self.config.vm_memory));
        report.push_str(&format!("- VM CPUs: {}\n", self.config.vm_cpus));
        report.push_str(&format!("- SSH Port: {}\n", self.config.ssh_port));
        report.push_str(&format!("- Test Timeout: {:?}\n", self.config.test_timeout));
        report.push_str("\n");

        report.push_str("## Test Results\n\n");
        
        for (name, result) in &self.test_results {
            report.push_str(&format!("### {}\n\n", name));
            report.push_str(&format!("- Status: {}\n", if result.success { "✅ PASSED" } else { "❌ FAILED" }));
            report.push_str(&format!("- Execution Time: {:?}\n", result.execution_time));
            
            if result.success {
                report.push_str(&format!("- Output:\n```\n{}\n```\n\n", result.output));
            } else {
                report.push_str(&format!("- Error:\n```\n{}\n```\n\n", result.error_output));
            }
        }

        fs::write(output_path, report)
            .map_err(|e| format!("Failed to write report: {}", e))?;

        println!("📄 QEMU test report generated: {}", output_path);
        Ok(())
    }

    /// Check if QEMU is available
    fn check_qemu_available(&self) -> bool {
        Command::new("qemu-system-x86_64")
            .arg("--version")
            .output()
            .is_ok()
    }

    /// Get test results
    pub fn get_results(&self) -> &HashMap<String, QemuTestResult> {
        &self.test_results
    }

    /// Check if VM is running
    pub fn is_vm_running(&self) -> bool {
        self.vm_running
    }
}

impl Drop for QemuTestAutomation {
    fn drop(&mut self) {
        if self.vm_running {
            let _ = self.stop_vm();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_qemu_config_defaults() {
        let config = QemuTestConfig::default();
        assert_eq!(config.vm_memory, "2G");
        assert_eq!(config.vm_cpus, 2);
        assert_eq!(config.ssh_port, 2222);
        assert_eq!(config.vm_user, "vexfs");
    }

    #[test]
    fn test_qemu_test_result_creation() {
        let result = QemuTestResult::new("test_example");
        assert_eq!(result.test_name, "test_example");
        assert!(!result.success);
        assert_eq!(result.execution_time, Duration::ZERO);
    }

    #[test]
    fn test_qemu_test_automation_creation() {
        let automation = QemuTestAutomation::new();
        assert!(!automation.vm_running);
        assert_eq!(automation.test_results.len(), 0);
    }
}