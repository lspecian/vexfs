//! VexFS Level 1 Basic Kernel Module Validation Tests
//!
//! This module implements testing of the REAL production VexFS kernel module
//! lifecycle operations as specified in Task 32.1. These tests focus on
//! basic module loading, unloading, and listing operations without
//! dangerous mount operations that could crash the host system.
//!
//! Safety Level: PRODUCTION MODULE TESTING
//! - Tests REAL production kernel module compilation and basic lifecycle
//! - Detects kernel panics, oopses, and resource leaks
//! - No mount operations (those require VM testing)
//! - Integrates with kselftest framework
//! - Uses vm-build target for full FFI-enabled production module

use std::process::Command;
use std::path::PathBuf;
use std::fs;
use std::time::{Duration, Instant};
use std::thread;

/// Test configuration for Level 1 validation
#[derive(Debug, Clone)]
pub struct Level1TestConfig {
    /// Path to kernel build directory
    pub kernel_build_dir: PathBuf,
    /// Path to VexFS kernel module source
    pub vexfs_kernel_dir: PathBuf,
    /// Whether to run tests requiring sudo
    pub enable_sudo_tests: bool,
    /// Build variant to test
    pub build_variant: BuildVariant,
}

/// VexFS kernel module build variants
#[derive(Debug, Clone, PartialEq)]
pub enum BuildVariant {
    /// Standard build with full FFI (production kernel module)
    Standard,
}

impl BuildVariant {
    fn make_target(&self) -> &'static str {
        match self {
            BuildVariant::Standard => "all",
        }
    }

    fn module_name(&self) -> &'static str {
        match self {
            BuildVariant::Standard => "vexfs.ko",
        }
    }
}

/// Kernel module lifecycle test results
#[derive(Debug, Clone, serde::Serialize)]
pub struct ModuleTestResult {
    pub operation: String,
    pub success: bool,
    pub duration: Duration,
    pub output: String,
    pub error_output: String,
    pub exit_code: Option<i32>,
    pub kernel_messages: Vec<String>,
    pub resource_usage: ResourceUsage,
}

/// System resource usage tracking
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct ResourceUsage {
    pub memory_kb: u64,
    pub file_descriptors: u32,
    pub kernel_threads: u32,
}

/// Kernel panic/oops detection
#[derive(Debug, Clone)]
pub struct KernelHealthCheck {
    pub has_panic: bool,
    pub has_oops: bool,
    pub has_warning: bool,
    pub suspicious_messages: Vec<String>,
}

/// Main Level 1 test runner
pub struct Level1TestRunner {
    config: Level1TestConfig,
    results: Vec<ModuleTestResult>,
    initial_resources: ResourceUsage,
}

impl Level1TestRunner {
    /// Create new test runner with configuration
    pub fn new(config: Level1TestConfig) -> Result<Self, Box<dyn std::error::Error>> {
        let initial_resources = Self::measure_resources()?;
        
        Ok(Self {
            config,
            results: Vec::new(),
            initial_resources,
        })
    }

    /// Run complete Level 1 validation test suite
    pub fn run_complete_test_suite(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!("🧪 Starting VexFS Level 1 Basic Kernel Module Validation");
        println!("Build variant: {:?}", self.config.build_variant);
        println!("Safety level: HOST-SAFE (no mount operations)");
        
        // Test 1: Module compilation
        self.test_module_compilation()?;
        
        // Test 2: Module information validation
        self.test_module_info_validation()?;
        
        if self.config.enable_sudo_tests {
            // Test 3: Module loading (requires sudo)
            self.test_module_loading()?;
            
            // Test 4: Module listing verification
            self.test_module_listing()?;
            
            // Test 5: Module unloading
            self.test_module_unloading()?;
            
            // Test 6: Resource leak detection
            self.test_resource_leak_detection()?;
        } else {
            println!("⚠️  Skipping sudo-required tests (enable_sudo_tests=false)");
        }
        
        // Test 7: Kernel health check
        self.test_kernel_health_check()?;
        
        // Generate test report
        self.generate_test_report()?;
        
        Ok(())
    }

    /// Test 1: Module compilation for all build variants
    fn test_module_compilation(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!("\n📦 Test 1: Module Compilation");
        
        let start_time = Instant::now();
        
        // Clean previous builds
        let clean_result = self.run_make_command("clean")?;
        if !clean_result.success {
            return Err(format!("Failed to clean build: {}", clean_result.error_output).into());
        }
        
        // Build the module
        let build_result = self.run_make_command(self.config.build_variant.make_target())?;
        
        let duration = start_time.elapsed();
        
        if build_result.success {
            // Verify module file exists
            let module_path = self.config.vexfs_kernel_dir
                .join(self.config.build_variant.module_name());
                
            if module_path.exists() {
                println!("✅ Module compilation successful: {}", module_path.display());
                
                // Check module size (should be reasonable)
                let metadata = fs::metadata(&module_path)?;
                let size_kb = metadata.len() / 1024;
                println!("   Module size: {} KB", size_kb);
                
                if size_kb > 10240 { // > 10MB seems excessive
                    println!("⚠️  Warning: Module size is unusually large");
                }
            } else {
                return Err(format!("Module file not found: {}", module_path.display()).into());
            }
        }
        
        self.results.push(ModuleTestResult {
            operation: "module_compilation".to_string(),
            success: build_result.success,
            duration,
            output: build_result.output,
            error_output: build_result.error_output,
            exit_code: build_result.exit_code,
            kernel_messages: Vec::new(),
            resource_usage: Self::measure_resources()?,
        });
        
        Ok(())
    }

    /// Test 2: Module information validation using modinfo
    fn test_module_info_validation(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!("\n📋 Test 2: Module Information Validation");
        
        let module_path = self.config.vexfs_kernel_dir
            .join(self.config.build_variant.module_name());
            
        if !module_path.exists() {
            return Err("Module file not found for info validation".into());
        }
        
        let start_time = Instant::now();
        
        let output = Command::new("modinfo")
            .arg(&module_path)
            .output()?;
            
        let duration = start_time.elapsed();
        let success = output.status.success();
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        
        if success {
            println!("✅ Module info validation successful");
            
            // Validate expected module metadata
            let required_fields = ["license", "description", "author", "version"];
            for field in &required_fields {
                if stdout.to_lowercase().contains(field) {
                    println!("   ✓ Found {}", field);
                } else {
                    println!("   ⚠️  Missing {}", field);
                }
            }
            
            // Check for VexFS-specific information
            if stdout.contains("vexfs") || stdout.contains("VexFS") {
                println!("   ✓ VexFS identifier found");
            } else {
                println!("   ⚠️  VexFS identifier not found");
            }
        } else {
            println!("❌ Module info validation failed");
        }
        
        self.results.push(ModuleTestResult {
            operation: "module_info_validation".to_string(),
            success,
            duration,
            output: stdout.to_string(),
            error_output: stderr.to_string(),
            exit_code: output.status.code(),
            kernel_messages: Vec::new(),
            resource_usage: Self::measure_resources()?,
        });
        
        Ok(())
    }

    /// Test 3: Module loading (requires sudo)
    fn test_module_loading(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!("\n🔄 Test 3: Module Loading (requires sudo)");
        
        let module_path = self.config.vexfs_kernel_dir
            .join(self.config.build_variant.module_name());
            
        let start_time = Instant::now();
        
        // Capture kernel messages before loading
        let pre_load_dmesg = self.capture_recent_kernel_messages()?;
        
        let output = Command::new("sudo")
            .arg("insmod")
            .arg(&module_path)
            .output()?;
            
        let duration = start_time.elapsed();
        let success = output.status.success();
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        
        // Capture kernel messages after loading
        thread::sleep(Duration::from_millis(100)); // Allow kernel messages to appear
        let post_load_dmesg = self.capture_recent_kernel_messages()?;
        let new_messages = self.diff_kernel_messages(&pre_load_dmesg, &post_load_dmesg);
        
        if success {
            println!("✅ Module loading successful");
            
            // Check for expected VexFS initialization messages
            let vexfs_messages: Vec<_> = new_messages.iter()
                .filter(|msg| msg.to_lowercase().contains("vexfs"))
                .collect();
                
            if !vexfs_messages.is_empty() {
                println!("   ✓ VexFS initialization messages found:");
                for msg in &vexfs_messages {
                    println!("     {}", msg);
                }
            } else {
                println!("   ⚠️  No VexFS initialization messages found");
            }
            
            // Check for error messages
            let error_messages: Vec<_> = new_messages.iter()
                .filter(|msg| msg.to_lowercase().contains("error") || 
                             msg.to_lowercase().contains("failed") ||
                             msg.to_lowercase().contains("panic"))
                .collect();
                
            if !error_messages.is_empty() {
                println!("   ⚠️  Error messages detected:");
                for msg in &error_messages {
                    println!("     {}", msg);
                }
            }
        } else {
            println!("❌ Module loading failed");
        }
        
        self.results.push(ModuleTestResult {
            operation: "module_loading".to_string(),
            success,
            duration,
            output: stdout.to_string(),
            error_output: stderr.to_string(),
            exit_code: output.status.code(),
            kernel_messages: new_messages,
            resource_usage: Self::measure_resources()?,
        });
        
        Ok(())
    }

    /// Test 4: Module listing verification using lsmod
    fn test_module_listing(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!("\n📝 Test 4: Module Listing Verification");
        
        let start_time = Instant::now();
        
        let output = Command::new("lsmod")
            .output()?;
            
        let duration = start_time.elapsed();
        let success = output.status.success();
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        
        if success {
            // Check if VexFS module is listed
            let vexfs_listed = stdout.lines()
                .any(|line| line.starts_with("vexfs"));
                
            if vexfs_listed {
                println!("✅ VexFS module found in lsmod output");
                
                // Extract module information
                if let Some(vexfs_line) = stdout.lines().find(|line| line.starts_with("vexfs")) {
                    println!("   Module info: {}", vexfs_line);
                    
                    // Parse module size and usage count
                    let parts: Vec<&str> = vexfs_line.split_whitespace().collect();
                    if parts.len() >= 3 {
                        println!("   Size: {} bytes", parts[1]);
                        println!("   Usage count: {}", parts[2]);
                    }
                }
            } else {
                println!("❌ VexFS module not found in lsmod output");
            }
        } else {
            println!("❌ lsmod command failed");
        }
        
        self.results.push(ModuleTestResult {
            operation: "module_listing".to_string(),
            success: success && stdout.contains("vexfs"),
            duration,
            output: stdout.to_string(),
            error_output: stderr.to_string(),
            exit_code: output.status.code(),
            kernel_messages: Vec::new(),
            resource_usage: Self::measure_resources()?,
        });
        
        Ok(())
    }

    /// Test 5: Module unloading (requires sudo)
    fn test_module_unloading(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!("\n🔄 Test 5: Module Unloading");
        
        let start_time = Instant::now();
        
        // Capture kernel messages before unloading
        let pre_unload_dmesg = self.capture_recent_kernel_messages()?;
        
        let output = Command::new("sudo")
            .arg("rmmod")
            .arg("vexfs")
            .output()?;
            
        let duration = start_time.elapsed();
        let success = output.status.success();
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        
        // Capture kernel messages after unloading
        thread::sleep(Duration::from_millis(100));
        let post_unload_dmesg = self.capture_recent_kernel_messages()?;
        let new_messages = self.diff_kernel_messages(&pre_unload_dmesg, &post_unload_dmesg);
        
        if success {
            println!("✅ Module unloading successful");
            
            // Verify module is no longer listed
            let lsmod_output = Command::new("lsmod").output()?;
            let lsmod_stdout = String::from_utf8_lossy(&lsmod_output.stdout);
            
            if !lsmod_stdout.contains("vexfs") {
                println!("   ✓ Module no longer listed in lsmod");
            } else {
                println!("   ⚠️  Module still listed in lsmod");
            }
            
            // Check for cleanup messages
            let cleanup_messages: Vec<_> = new_messages.iter()
                .filter(|msg| msg.to_lowercase().contains("vexfs") && 
                             (msg.to_lowercase().contains("unload") ||
                              msg.to_lowercase().contains("cleanup") ||
                              msg.to_lowercase().contains("exit")))
                .collect();
                
            if !cleanup_messages.is_empty() {
                println!("   ✓ Cleanup messages found:");
                for msg in &cleanup_messages {
                    println!("     {}", msg);
                }
            }
        } else {
            println!("❌ Module unloading failed");
        }
        
        self.results.push(ModuleTestResult {
            operation: "module_unloading".to_string(),
            success,
            duration,
            output: stdout.to_string(),
            error_output: stderr.to_string(),
            exit_code: output.status.code(),
            kernel_messages: new_messages,
            resource_usage: Self::measure_resources()?,
        });
        
        Ok(())
    }

    /// Test 6: Resource leak detection
    fn test_resource_leak_detection(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!("\n🔍 Test 6: Resource Leak Detection");
        
        let current_resources = Self::measure_resources()?;
        
        // Compare with initial resources
        let memory_diff = current_resources.memory_kb as i64 - self.initial_resources.memory_kb as i64;
        let fd_diff = current_resources.file_descriptors as i32 - self.initial_resources.file_descriptors as i32;
        let thread_diff = current_resources.kernel_threads as i32 - self.initial_resources.kernel_threads as i32;
        
        println!("   Memory change: {} KB", memory_diff);
        println!("   File descriptor change: {}", fd_diff);
        println!("   Kernel thread change: {}", thread_diff);
        
        // Define acceptable thresholds
        let memory_threshold = 1024; // 1MB
        let fd_threshold = 10;
        let thread_threshold = 5;
        
        let mut leak_detected = false;
        
        if memory_diff.abs() > memory_threshold {
            println!("   ⚠️  Significant memory change detected");
            leak_detected = true;
        }
        
        if fd_diff.abs() > fd_threshold {
            println!("   ⚠️  Significant file descriptor change detected");
            leak_detected = true;
        }
        
        if thread_diff.abs() > thread_threshold {
            println!("   ⚠️  Significant kernel thread change detected");
            leak_detected = true;
        }
        
        if !leak_detected {
            println!("✅ No significant resource leaks detected");
        }
        
        self.results.push(ModuleTestResult {
            operation: "resource_leak_detection".to_string(),
            success: !leak_detected,
            duration: Duration::from_millis(0),
            output: format!("Memory: {}KB, FD: {}, Threads: {}", memory_diff, fd_diff, thread_diff),
            error_output: String::new(),
            exit_code: Some(0),
            kernel_messages: Vec::new(),
            resource_usage: current_resources,
        });
        
        Ok(())
    }

    /// Test 7: Kernel health check for panics/oopses
    fn test_kernel_health_check(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!("\n🏥 Test 7: Kernel Health Check");
        
        let health_check = self.perform_kernel_health_check()?;
        
        if health_check.has_panic {
            println!("❌ Kernel panic detected!");
        } else {
            println!("✅ No kernel panics detected");
        }
        
        if health_check.has_oops {
            println!("❌ Kernel oops detected!");
        } else {
            println!("✅ No kernel oopses detected");
        }
        
        if health_check.has_warning {
            println!("⚠️  Kernel warnings detected");
        } else {
            println!("✅ No kernel warnings detected");
        }
        
        if !health_check.suspicious_messages.is_empty() {
            println!("   Suspicious messages:");
            for msg in &health_check.suspicious_messages {
                println!("     {}", msg);
            }
        }
        
        let success = !health_check.has_panic && !health_check.has_oops;
        
        self.results.push(ModuleTestResult {
            operation: "kernel_health_check".to_string(),
            success,
            duration: Duration::from_millis(0),
            output: format!("Panic: {}, Oops: {}, Warnings: {}", 
                          health_check.has_panic, health_check.has_oops, health_check.has_warning),
            error_output: String::new(),
            exit_code: Some(0),
            kernel_messages: health_check.suspicious_messages,
            resource_usage: Self::measure_resources()?,
        });
        
        Ok(())
    }

    /// Generate comprehensive test report
    fn generate_test_report(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("\n📊 Level 1 Test Report");
        println!("======================");
        
        let total_tests = self.results.len();
        let passed_tests = self.results.iter().filter(|r| r.success).count();
        let failed_tests = total_tests - passed_tests;
        
        println!("Total tests: {}", total_tests);
        println!("Passed: {} ✅", passed_tests);
        println!("Failed: {} ❌", failed_tests);
        println!("Success rate: {:.1}%", (passed_tests as f64 / total_tests as f64) * 100.0);
        
        println!("\nDetailed Results:");
        for result in &self.results {
            let status = if result.success { "✅ PASS" } else { "❌ FAIL" };
            println!("  {} {} ({:.2}s)", status, result.operation, result.duration.as_secs_f64());
            
            if !result.success && !result.error_output.is_empty() {
                println!("    Error: {}", result.error_output.lines().next().unwrap_or(""));
            }
        }
        
        // Save detailed report to file
        let report_path = self.config.vexfs_kernel_dir.join("test_results/level1_report.json");
        if let Some(parent) = report_path.parent() {
            fs::create_dir_all(parent)?;
        }
        
        let report_json = serde_json::to_string_pretty(&self.results)?;
        fs::write(&report_path, report_json)?;
        
        println!("\nDetailed report saved to: {}", report_path.display());
        
        Ok(())
    }

    /// Helper: Run make command in VexFS kernel directory
    fn run_make_command(&self, target: &str) -> Result<ModuleTestResult, Box<dyn std::error::Error>> {
        let start_time = Instant::now();
        
        // Run make from within the VexFS kernel directory (not using -C)
        // This ensures $(PWD) in the Makefile resolves correctly
        let output = Command::new("make")
            .current_dir(&self.config.vexfs_kernel_dir)
            .arg(target)
            .output()?;
            
        let duration = start_time.elapsed();
        
        Ok(ModuleTestResult {
            operation: format!("make_{}", target),
            success: output.status.success(),
            duration,
            output: String::from_utf8_lossy(&output.stdout).to_string(),
            error_output: String::from_utf8_lossy(&output.stderr).to_string(),
            exit_code: output.status.code(),
            kernel_messages: Vec::new(),
            resource_usage: Self::measure_resources()?,
        })
    }

    /// Helper: Measure current system resource usage
    fn measure_resources() -> Result<ResourceUsage, Box<dyn std::error::Error>> {
        // Read memory info
        let meminfo = fs::read_to_string("/proc/meminfo")?;
        let memory_kb = meminfo.lines()
            .find(|line| line.starts_with("MemAvailable:"))
            .and_then(|line| line.split_whitespace().nth(1))
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(0);
        
        // Count file descriptors
        let fd_count = fs::read_dir("/proc/self/fd")?.count() as u32;
        
        // Count kernel threads (approximate)
        let stat = fs::read_to_string("/proc/stat")?;
        let processes = stat.lines()
            .find(|line| line.starts_with("processes"))
            .and_then(|line| line.split_whitespace().nth(1))
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(0);
        
        Ok(ResourceUsage {
            memory_kb,
            file_descriptors: fd_count,
            kernel_threads: processes,
        })
    }

    /// Helper: Capture recent kernel messages
    fn capture_recent_kernel_messages(&self) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let output = Command::new("dmesg")
            .arg("--time-format=iso")
            .arg("--level=0,1,2,3,4") // Emergency, Alert, Critical, Error, Warning
            .output()?;
            
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            Ok(stdout.lines().map(|s| s.to_string()).collect())
        } else {
            Ok(Vec::new())
        }
    }

    /// Helper: Find new messages by comparing before/after
    fn diff_kernel_messages(&self, before: &[String], after: &[String]) -> Vec<String> {
        after.iter()
            .filter(|msg| !before.contains(msg))
            .cloned()
            .collect()
    }

    /// Helper: Perform comprehensive kernel health check
    fn perform_kernel_health_check(&self) -> Result<KernelHealthCheck, Box<dyn std::error::Error>> {
        let messages = self.capture_recent_kernel_messages()?;
        
        let mut health_check = KernelHealthCheck {
            has_panic: false,
            has_oops: false,
            has_warning: false,
            suspicious_messages: Vec::new(),
        };
        
        for message in &messages {
            let lower_msg = message.to_lowercase();
            
            if lower_msg.contains("panic") {
                health_check.has_panic = true;
                health_check.suspicious_messages.push(message.clone());
            }
            
            if lower_msg.contains("oops") {
                health_check.has_oops = true;
                health_check.suspicious_messages.push(message.clone());
            }
            
            if lower_msg.contains("warning") || lower_msg.contains("warn") {
                health_check.has_warning = true;
                health_check.suspicious_messages.push(message.clone());
            }
            
            // Check for VexFS-related issues
            if lower_msg.contains("vexfs") && 
               (lower_msg.contains("error") || lower_msg.contains("failed") || 
                lower_msg.contains("timeout") || lower_msg.contains("hang")) {
                health_check.suspicious_messages.push(message.clone());
            }
        }
        
        Ok(health_check)
    }
}

impl Default for Level1TestConfig {
    fn default() -> Self {
        Self {
            kernel_build_dir: PathBuf::from("/lib/modules").join(
                std::process::Command::new("uname")
                    .arg("-r")
                    .output()
                    .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
                    .unwrap_or_else(|_| "unknown".to_string())
            ).join("build"),
            vexfs_kernel_dir: PathBuf::from("."),
            enable_sudo_tests: false, // Default to safe mode
            build_variant: BuildVariant::Standard, // Test the REAL production kernel module
        }
    }
}

// Serde support for JSON reporting
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct SerializableModuleTestResult {
    operation: String,
    success: bool,
    duration_ms: u64,
    output: String,
    error_output: String,
    exit_code: Option<i32>,
    kernel_messages: Vec<String>,
    memory_kb: u64,
    file_descriptors: u32,
    kernel_threads: u32,
}

impl From<&ModuleTestResult> for SerializableModuleTestResult {
    fn from(result: &ModuleTestResult) -> Self {
        Self {
            operation: result.operation.clone(),
            success: result.success,
            duration_ms: result.duration.as_millis() as u64,
            output: result.output.clone(),
            error_output: result.error_output.clone(),
            exit_code: result.exit_code,
            kernel_messages: result.kernel_messages.clone(),
            memory_kb: result.resource_usage.memory_kb,
            file_descriptors: result.resource_usage.file_descriptors,
            kernel_threads: result.resource_usage.kernel_threads,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_variant_properties() {
        assert_eq!(BuildVariant::Standard.make_target(), "all");
        assert_eq!(BuildVariant::Standard.module_name(), "vexfs.ko");
    }

    #[test]
    fn test_config_default() {
        let config = Level1TestConfig::default();
        assert_eq!(config.build_variant, BuildVariant::Standard);
        assert!(!config.enable_sudo_tests);
    }

    #[test]
    fn test_resource_measurement() {
        let resources = Level1TestRunner::measure_resources();
        assert!(resources.is_ok());
        
        let resources = resources.unwrap();
        assert!(resources.memory_kb > 0);
    }
}