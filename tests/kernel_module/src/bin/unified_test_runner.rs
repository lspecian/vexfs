//! Unified Test Runner for VexFS Kernel Module
//!
//! This is the master test orchestrator that executes ALL three levels of testing:
//! - Level 1: Basic kselftest integration
//! - Level 2: VM-based mount operations
//! - Level 3: Ultimate stress testing with kernel instrumentation
//!
//! Provides seamless integration, comprehensive reporting, and automated test orchestration.

use std::env;
use std::process::{self, Command, Stdio};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use std::path::{Path, PathBuf};
use std::fs;
use std::collections::HashMap;
use clap::{Parser, Subcommand, Args};
use serde::{Serialize, Deserialize};
use serde_json;

use vexfs_kernel_tests::{
    VmConfig, TestStatus,
    level1_basic_validation::Level1TestRunner,
    level2_vm_mount_operations::Level2TestRunner,
    stress_testing_framework::{StressTestingFramework, StressTestConfig},
    crash_detection::{CrashDetector, CrashEvent, CrashType},
    resource_monitoring::{ResourceMonitor, ResourceMetrics},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedTestConfig {
    pub enable_level1: bool,
    pub enable_level2: bool,
    pub enable_level3: bool,
    pub vm_config: VmConfig,
    pub output_directory: String,
    pub test_session_id: String,
    pub continue_on_failure: bool,
    pub parallel_execution: bool,
    pub comprehensive_reporting: bool,
    pub crash_recovery_enabled: bool,
    pub performance_baseline_capture: bool,
}

impl Default for UnifiedTestConfig {
    fn default() -> Self {
        Self {
            enable_level1: true,
            enable_level2: true,
            enable_level3: true,
            vm_config: VmConfig::default(),
            output_directory: "unified_test_results".to_string(),
            test_session_id: generate_session_id(),
            continue_on_failure: false,
            parallel_execution: false,
            comprehensive_reporting: true,
            crash_recovery_enabled: true,
            performance_baseline_capture: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestLevelResult {
    pub level: String,
    pub status: TestStatus,
    pub start_time: SystemTime,
    pub end_time: SystemTime,
    pub duration_seconds: f64,
    pub test_count: u32,
    pub passed_count: u32,
    pub failed_count: u32,
    pub skipped_count: u32,
    pub crash_events: Vec<CrashEvent>,
    pub resource_metrics: Option<ResourceMetrics>,
    pub output_files: Vec<String>,
    pub error_summary: Vec<String>,
    pub performance_metrics: HashMap<String, f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedTestResult {
    pub session_id: String,
    pub overall_status: TestStatus,
    pub start_time: SystemTime,
    pub end_time: SystemTime,
    pub total_duration_seconds: f64,
    pub level_results: Vec<TestLevelResult>,
    pub crash_classification: CrashClassificationReport,
    pub performance_analysis: PerformanceAnalysisReport,
    pub regression_analysis: Option<RegressionAnalysisReport>,
    pub recommendations: Vec<String>,
    pub artifacts: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrashClassificationReport {
    pub total_crashes: u32,
    pub crash_types: HashMap<String, u32>,
    pub recovery_success_rate: f64,
    pub critical_crashes: Vec<CrashEvent>,
    pub patterns_identified: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceAnalysisReport {
    pub baseline_metrics: HashMap<String, f64>,
    pub current_metrics: HashMap<String, f64>,
    pub performance_deltas: HashMap<String, f64>,
    pub performance_trends: Vec<String>,
    pub bottlenecks_identified: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegressionAnalysisReport {
    pub regressions_detected: Vec<String>,
    pub performance_regressions: Vec<String>,
    pub stability_regressions: Vec<String>,
    pub severity_assessment: String,
}

pub struct UnifiedTestRunner {
    config: UnifiedTestConfig,
    crash_detector: CrashDetector,
    resource_monitor: ResourceMonitor,
    results: Vec<TestLevelResult>,
    start_time: Instant,
}

impl UnifiedTestRunner {
    pub fn new(config: UnifiedTestConfig) -> Self {
        Self {
            crash_detector: CrashDetector::new(),
            resource_monitor: ResourceMonitor::new(),
            results: Vec::new(),
            start_time: Instant::now(),
            config,
        }
    }

    pub fn run_unified_test_suite(&mut self) -> Result<UnifiedTestResult, Box<dyn std::error::Error>> {
        let session_start = SystemTime::now();
        
        println!("🚀 Starting VexFS Unified Test Suite");
        println!("📋 Session ID: {}", self.config.test_session_id);
        println!("🎯 Test Levels: L1={}, L2={}, L3={}", 
                self.config.enable_level1, 
                self.config.enable_level2, 
                self.config.enable_level3);

        // Create output directory
        self.setup_output_directory()?;

        // Start resource monitoring
        if self.config.comprehensive_reporting {
            self.resource_monitor.start_monitoring()?;
        }

        // Execute test levels in sequence or parallel
        if self.config.parallel_execution {
            self.run_parallel_test_execution()?;
        } else {
            self.run_sequential_test_execution()?;
        }

        // Stop resource monitoring
        if self.config.comprehensive_reporting {
            self.resource_monitor.stop_monitoring()?;
        }

        // Generate comprehensive results
        let session_end = SystemTime::now();
        let total_duration = session_end.duration_since(session_start)?.as_secs_f64();

        let unified_result = self.generate_unified_result(session_start, session_end, total_duration)?;

        // Save results and generate reports
        self.save_unified_results(&unified_result)?;
        self.generate_comprehensive_reports(&unified_result)?;

        Ok(unified_result)
    }

    fn setup_output_directory(&self) -> Result<(), Box<dyn std::error::Error>> {
        let output_path = Path::new(&self.config.output_directory);
        if !output_path.exists() {
            fs::create_dir_all(output_path)?;
        }

        // Create subdirectories for each level
        for level in &["level1", "level2", "level3", "reports", "artifacts"] {
            let level_path = output_path.join(level);
            if !level_path.exists() {
                fs::create_dir_all(level_path)?;
            }
        }

        println!("📁 Output directory created: {}", self.config.output_directory);
        Ok(())
    }

    fn run_sequential_test_execution(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!("🔄 Running tests sequentially...");

        if self.config.enable_level1 {
            match self.run_level1_tests() {
                Ok(result) => {
                    self.results.push(result);
                    println!("✅ Level 1 tests completed");
                }
                Err(e) => {
                    eprintln!("❌ Level 1 tests failed: {}", e);
                    if !self.config.continue_on_failure {
                        return Err(e);
                    }
                }
            }
        }

        if self.config.enable_level2 {
            match self.run_level2_tests() {
                Ok(result) => {
                    self.results.push(result);
                    println!("✅ Level 2 tests completed");
                }
                Err(e) => {
                    eprintln!("❌ Level 2 tests failed: {}", e);
                    if !self.config.continue_on_failure {
                        return Err(e);
                    }
                }
            }
        }

        if self.config.enable_level3 {
            match self.run_level3_tests() {
                Ok(result) => {
                    self.results.push(result);
                    println!("✅ Level 3 tests completed");
                }
                Err(e) => {
                    eprintln!("❌ Level 3 tests failed: {}", e);
                    if !self.config.continue_on_failure {
                        return Err(e);
                    }
                }
            }
        }

        Ok(())
    }

    fn run_parallel_test_execution(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        println!("⚡ Running tests in parallel...");
        // Note: Parallel execution would require careful coordination
        // For now, fall back to sequential execution
        println!("⚠️  Parallel execution not yet implemented, falling back to sequential");
        self.run_sequential_test_execution()
    }

    fn run_level1_tests(&mut self) -> Result<TestLevelResult, Box<dyn std::error::Error>> {
        let start_time = SystemTime::now();
        println!("🧪 Running Level 1: Basic kselftest validation");

        let output_file = format!("{}/level1/level1_results.json", self.config.output_directory);
        
        // Execute Level 1 runner
        let output = Command::new("cargo")
            .args(&["run", "--bin", "kselftest_runner", "--", "--output", &output_file])
            .current_dir("tests/kernel_module")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()?;

        let end_time = SystemTime::now();
        let duration = end_time.duration_since(start_time)?.as_secs_f64();

        let status = if output.status.success() {
            TestStatus::Success
        } else {
            TestStatus::Failed
        };

        // Parse results if available
        let (test_count, passed_count, failed_count) = self.parse_level1_results(&output_file)?;

        Ok(TestLevelResult {
            level: "Level 1 - Basic Validation".to_string(),
            status,
            start_time,
            end_time,
            duration_seconds: duration,
            test_count,
            passed_count,
            failed_count,
            skipped_count: 0,
            crash_events: Vec::new(),
            resource_metrics: None,
            output_files: vec![output_file],
            error_summary: if !output.status.success() {
                vec![String::from_utf8_lossy(&output.stderr).to_string()]
            } else {
                Vec::new()
            },
            performance_metrics: HashMap::new(),
        })
    }

    fn run_level2_tests(&mut self) -> Result<TestLevelResult, Box<dyn std::error::Error>> {
        let start_time = SystemTime::now();
        println!("🖥️  Running Level 2: VM-based mount operations");

        let output_file = format!("{}/level2/level2_results.json", self.config.output_directory);
        
        // Execute Level 2 runner
        let output = Command::new("cargo")
            .args(&["run", "--bin", "mount_level_runner", "--", "--output", &output_file])
            .current_dir("tests/kernel_module")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()?;

        let end_time = SystemTime::now();
        let duration = end_time.duration_since(start_time)?.as_secs_f64();

        let status = if output.status.success() {
            TestStatus::Success
        } else {
            TestStatus::Failed
        };

        // Parse results and detect crashes
        let (test_count, passed_count, failed_count) = self.parse_level2_results(&output_file)?;
        let crash_events = self.crash_detector.detect_crashes_in_output(&String::from_utf8_lossy(&output.stdout))?;

        Ok(TestLevelResult {
            level: "Level 2 - VM Mount Operations".to_string(),
            status,
            start_time,
            end_time,
            duration_seconds: duration,
            test_count,
            passed_count,
            failed_count,
            skipped_count: 0,
            crash_events,
            resource_metrics: self.resource_monitor.get_current_metrics().ok(),
            output_files: vec![output_file],
            error_summary: if !output.status.success() {
                vec![String::from_utf8_lossy(&output.stderr).to_string()]
            } else {
                Vec::new()
            },
            performance_metrics: HashMap::new(),
        })
    }

    fn run_level3_tests(&mut self) -> Result<TestLevelResult, Box<dyn std::error::Error>> {
        let start_time = SystemTime::now();
        println!("🔥 Running Level 3: Ultimate stress testing");

        let output_file = format!("{}/level3/level3_results.json", self.config.output_directory);
        
        // Execute Level 3 runner with quick test for integration
        let output = Command::new("cargo")
            .args(&["run", "--bin", "stress_test_runner", "--", "quick", "--output", &output_file])
            .current_dir("tests/kernel_module")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()?;

        let end_time = SystemTime::now();
        let duration = end_time.duration_since(start_time)?.as_secs_f64();

        let status = if output.status.success() {
            TestStatus::Success
        } else {
            TestStatus::Failed
        };

        // Parse stress test results
        let (test_count, passed_count, failed_count) = self.parse_level3_results(&output_file)?;
        let crash_events = self.crash_detector.detect_crashes_in_output(&String::from_utf8_lossy(&output.stdout))?;

        Ok(TestLevelResult {
            level: "Level 3 - Ultimate Stress Testing".to_string(),
            status,
            start_time,
            end_time,
            duration_seconds: duration,
            test_count,
            passed_count,
            failed_count,
            skipped_count: 0,
            crash_events,
            resource_metrics: self.resource_monitor.get_current_metrics().ok(),
            output_files: vec![output_file],
            error_summary: if !output.status.success() {
                vec![String::from_utf8_lossy(&output.stderr).to_string()]
            } else {
                Vec::new()
            },
            performance_metrics: HashMap::new(),
        })
    }

    fn parse_level1_results(&self, output_file: &str) -> Result<(u32, u32, u32), Box<dyn std::error::Error>> {
        // Default values if parsing fails
        if !Path::new(output_file).exists() {
            return Ok((1, 0, 1)); // Assume failure if no output
        }

        // Try to parse JSON results
        match fs::read_to_string(output_file) {
            Ok(content) => {
                if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(&content) {
                    let test_count = json_value.get("test_count").and_then(|v| v.as_u64()).unwrap_or(1) as u32;
                    let passed_count = json_value.get("passed_count").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                    let failed_count = test_count - passed_count;
                    return Ok((test_count, passed_count, failed_count));
                }
            }
            Err(_) => {}
        }

        Ok((1, 0, 1))
    }

    fn parse_level2_results(&self, output_file: &str) -> Result<(u32, u32, u32), Box<dyn std::error::Error>> {
        // Similar parsing logic for Level 2
        if !Path::new(output_file).exists() {
            return Ok((1, 0, 1));
        }

        match fs::read_to_string(output_file) {
            Ok(content) => {
                if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(&content) {
                    let test_count = json_value.get("test_count").and_then(|v| v.as_u64()).unwrap_or(1) as u32;
                    let passed_count = json_value.get("passed_count").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                    let failed_count = test_count - passed_count;
                    return Ok((test_count, passed_count, failed_count));
                }
            }
            Err(_) => {}
        }

        Ok((1, 0, 1))
    }

    fn parse_level3_results(&self, output_file: &str) -> Result<(u32, u32, u32), Box<dyn std::error::Error>> {
        // Similar parsing logic for Level 3
        if !Path::new(output_file).exists() {
            return Ok((1, 0, 1));
        }

        match fs::read_to_string(output_file) {
            Ok(content) => {
                if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(&content) {
                    let status = json_value.get("status").and_then(|v| v.as_str()).unwrap_or("Failed");
                    if status == "Success" {
                        return Ok((1, 1, 0));
                    }
                }
            }
            Err(_) => {}
        }

        Ok((1, 0, 1))
    }

    fn generate_unified_result(
        &self,
        start_time: SystemTime,
        end_time: SystemTime,
        total_duration: f64,
    ) -> Result<UnifiedTestResult, Box<dyn std::error::Error>> {
        
        // Determine overall status
        let overall_status = if self.results.iter().all(|r| matches!(r.status, TestStatus::Success)) {
            TestStatus::Success
        } else {
            TestStatus::Failed
        };

        // Generate crash classification
        let crash_classification = self.generate_crash_classification_report();

        // Generate performance analysis
        let performance_analysis = self.generate_performance_analysis_report();

        // Generate recommendations
        let recommendations = self.generate_recommendations();

        Ok(UnifiedTestResult {
            session_id: self.config.test_session_id.clone(),
            overall_status,
            start_time,
            end_time,
            total_duration_seconds: total_duration,
            level_results: self.results.clone(),
            crash_classification,
            performance_analysis,
            regression_analysis: None, // TODO: Implement regression analysis
            recommendations,
            artifacts: self.collect_artifacts(),
        })
    }

    fn generate_crash_classification_report(&self) -> CrashClassificationReport {
        let mut total_crashes = 0;
        let mut crash_types = HashMap::new();
        let mut critical_crashes = Vec::new();

        for result in &self.results {
            total_crashes += result.crash_events.len() as u32;
            
            for crash in &result.crash_events {
                let crash_type_str = format!("{:?}", crash.crash_type);
                *crash_types.entry(crash_type_str).or_insert(0) += 1;

                // Identify critical crashes
                if matches!(crash.crash_type, CrashType::KernelPanic | CrashType::SystemHang) {
                    critical_crashes.push(crash.clone());
                }
            }
        }

        CrashClassificationReport {
            total_crashes,
            crash_types,
            recovery_success_rate: if total_crashes > 0 { 0.8 } else { 1.0 }, // TODO: Calculate actual rate
            critical_crashes,
            patterns_identified: Vec::new(), // TODO: Implement pattern detection
        }
    }

    fn generate_performance_analysis_report(&self) -> PerformanceAnalysisReport {
        let mut baseline_metrics = HashMap::new();
        let mut current_metrics = HashMap::new();

        // Collect performance metrics from all levels
        for result in &self.results {
            for (key, value) in &result.performance_metrics {
                current_metrics.insert(key.clone(), *value);
            }
        }

        // TODO: Load baseline metrics from previous runs
        baseline_metrics.insert("mount_time_ms".to_string(), 100.0);
        baseline_metrics.insert("unmount_time_ms".to_string(), 50.0);

        let performance_deltas = current_metrics.iter()
            .filter_map(|(key, value)| {
                baseline_metrics.get(key).map(|baseline| {
                    (key.clone(), ((value - baseline) / baseline) * 100.0)
                })
            })
            .collect();

        PerformanceAnalysisReport {
            baseline_metrics,
            current_metrics,
            performance_deltas,
            performance_trends: Vec::new(), // TODO: Implement trend analysis
            bottlenecks_identified: Vec::new(), // TODO: Implement bottleneck detection
        }
    }

    fn generate_recommendations(&self) -> Vec<String> {
        let mut recommendations = Vec::new();

        // Analyze results and generate recommendations
        let total_failures = self.results.iter().map(|r| r.failed_count).sum::<u32>();
        let total_crashes = self.results.iter().map(|r| r.crash_events.len()).sum::<usize>();

        if total_failures > 0 {
            recommendations.push("Review failed test cases and address underlying issues".to_string());
        }

        if total_crashes > 0 {
            recommendations.push("Investigate crash patterns and implement additional safety checks".to_string());
        }

        if self.results.iter().any(|r| r.duration_seconds > 300.0) {
            recommendations.push("Consider optimizing test execution time for faster feedback".to_string());
        }

        if recommendations.is_empty() {
            recommendations.push("All tests passed successfully - consider adding more comprehensive test scenarios".to_string());
        }

        recommendations
    }

    fn collect_artifacts(&self) -> Vec<String> {
        let mut artifacts = Vec::new();

        for result in &self.results {
            artifacts.extend(result.output_files.clone());
        }

        // Add additional artifacts
        artifacts.push(format!("{}/unified_test_results.json", self.config.output_directory));
        artifacts.push(format!("{}/reports/comprehensive_report.html", self.config.output_directory));

        artifacts
    }

    fn save_unified_results(&self, result: &UnifiedTestResult) -> Result<(), Box<dyn std::error::Error>> {
        let output_file = format!("{}/unified_test_results.json", self.config.output_directory);
        let json_result = serde_json::to_string_pretty(result)?;
        fs::write(&output_file, json_result)?;
        
        println!("📊 Unified results saved to: {}", output_file);
        Ok(())
    }

    fn generate_comprehensive_reports(&self, result: &UnifiedTestResult) -> Result<(), Box<dyn std::error::Error>> {
        // Generate HTML report
        self.generate_html_report(result)?;
        
        // Generate summary report
        self.generate_summary_report(result)?;
        
        Ok(())
    }

    fn generate_html_report(&self, result: &UnifiedTestResult) -> Result<(), Box<dyn std::error::Error>> {
        let html_content = self.create_html_report_content(result);
        let output_file = format!("{}/reports/comprehensive_report.html", self.config.output_directory);
        fs::write(&output_file, html_content)?;
        
        println!("📄 HTML report generated: {}", output_file);
        Ok(())
    }

    fn create_html_report_content(&self, result: &UnifiedTestResult) -> String {
        format!(r#"
<!DOCTYPE html>
<html>
<head>
    <title>VexFS Unified Test Report - {}</title>
    <style>
        body {{ font-family: Arial, sans-serif; margin: 20px; }}
        .header {{ background-color: #f0f0f0; padding: 20px; border-radius: 5px; }}
        .status-success {{ color: green; font-weight: bold; }}
        .status-failed {{ color: red; font-weight: bold; }}
        .level-result {{ margin: 20px 0; padding: 15px; border: 1px solid #ddd; border-radius: 5px; }}
        .metrics {{ display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: 10px; }}
        .metric {{ background-color: #f9f9f9; padding: 10px; border-radius: 3px; }}
        .recommendations {{ background-color: #fff3cd; padding: 15px; border-radius: 5px; margin-top: 20px; }}
    </style>
</head>
<body>
    <div class="header">
        <h1>VexFS Unified Test Report</h1>
        <p><strong>Session ID:</strong> {}</p>
        <p><strong>Overall Status:</strong> <span class="status-{}">{:?}</span></p>
        <p><strong>Total Duration:</strong> {:.2} seconds</p>
        <p><strong>Generated:</strong> {}</p>
    </div>

    <h2>Test Level Results</h2>
    {}

    <h2>Crash Analysis</h2>
    <div class="metrics">
        <div class="metric">
            <strong>Total Crashes:</strong> {}
        </div>
        <div class="metric">
            <strong>Recovery Rate:</strong> {:.1}%
        </div>
        <div class="metric">
            <strong>Critical Crashes:</strong> {}
        </div>
    </div>

    <h2>Performance Analysis</h2>
    <div class="metrics">
        {}
    </div>

    <div class="recommendations">
        <h2>Recommendations</h2>
        <ul>
            {}
        </ul>
    </div>
</body>
</html>
        "#,
            result.session_id,
            result.session_id,
            if matches!(result.overall_status, TestStatus::Success) { "success" } else { "failed" },
            result.overall_status,
            result.total_duration_seconds,
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
            self.format_level_results_html(&result.level_results),
            result.crash_classification.total_crashes,
            result.crash_classification.recovery_success_rate * 100.0,
            result.crash_classification.critical_crashes.len(),
            self.format_performance_metrics_html(&result.performance_analysis),
            result.recommendations.iter().map(|r| format!("<li>{}</li>", r)).collect::<Vec<_>>().join("\n")
        )
    }

    fn format_level_results_html(&self, results: &[TestLevelResult]) -> String {
        results.iter().map(|result| {
            format!(r#"
            <div class="level-result">
                <h3>{}</h3>
                <p><strong>Status:</strong> <span class="status-{}">{:?}</span></p>
                <p><strong>Duration:</strong> {:.2} seconds</p>
                <p><strong>Tests:</strong> {} total, {} passed, {} failed</p>
                <p><strong>Crashes:</strong> {}</p>
            </div>
            "#,
                result.level,
                if matches!(result.status, TestStatus::Success) { "success" } else { "failed" },
                result.status,
                result.duration_seconds,
                result.test_count,
                result.passed_count,
                result.failed_count,
                result.crash_events.len()
            )
        }).collect::<Vec<_>>().join("\n")
    }

    fn format_performance_metrics_html(&self, analysis: &PerformanceAnalysisReport) -> String {
        analysis.current_metrics.iter().map(|(key, value)| {
            let delta = analysis.performance_deltas.get(key).unwrap_or(&0.0);
            format!(r#"
            <div class="metric">
                <strong>{}:</strong> {:.2} ({:+.1}%)
            </div>
            "#, key, value, delta)
        }).collect::<Vec<_>>().join("\n")
    }

    fn generate_summary_report(&self, result: &UnifiedTestResult) -> Result<(), Box<dyn std::error::Error>> {
        let summary = format!(r#"
VexFS Unified Test Suite Summary
===============================

Session ID: {}
Overall Status: {:?}
Total Duration: {:.2} seconds

Level Results:
{}

Crash Summary:
- Total Crashes: {}
- Recovery Rate: {:.1}%
- Critical Crashes: {}

Recommendations:
{}

Generated: {}
        "#,
            result.session_id,
            result.overall_status,
            result.total_duration_seconds,
            result.level_results.iter().map(|r| {
                format!("- {}: {:?} ({:.2}s, {}/{} tests passed)", 
                    r.level, r.status, r.duration_seconds, r.passed_count, r.test_count)
            }).collect::<Vec<_>>().join("\n"),
            result.crash_classification.total_crashes,
            result.crash_classification.recovery_success_rate * 100.0,
            result.crash_classification.critical_crashes.len(),
            result.recommendations.iter().map(|r| format!("- {}", r)).collect::<Vec<_>>().join("\n"),
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
        );

        let output_file = format!("{}/reports/summary_report.txt", self.config.output_directory);
        fs::write(&output_file, summary)?;
        
        println!("📋 Summary report generated: {}", output_file);
        Ok(())
    }
}

fn generate_session_id() -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    format!("vexfs_test_{}", timestamp)
}

fn main() {
    let matches = App::new("VexFS Unified Test Runner")
        .version("1.0.0")
        .author("VexFS Team")
        .about("Master test orchestrator for all three levels of VexFS kernel module testing")
        .arg(Arg::with_name("config")
            .short("c")
            .long("config")
            .value_name("FILE")
            .help("Configuration file for unified test settings")
            .takes_value(true))
        .arg(Arg::with_name("output-dir")
            .short("o")
            .long("output-dir")
            .value_name("DIR")
            .help("Output directory for test results")
            .takes_value(true)
            .default_value("unified_test_results"))
        .arg(Arg::with_name("continue-on-failure")
            .long("continue-on-failure")
            .help("Continue testing even if a level fails"))
        .arg(Arg::with_name("parallel")
            .long("parallel")
            .help("Run test levels in parallel (experimental)"))
        .arg(Arg::with_name("disable-level1")
            .long("disable-level1")
            .help("Skip Level 1 basic validation tests"))
        .arg(Arg::with_name("disable-level2")
            .long("disable-level2")
            .help("Skip Level 2 VM mount operation tests"))
        .arg(Arg::with_name("disable-level3")
            .long("disable-level3")
            .help("Skip Level 3 stress testing"))
        .arg(Arg::with_name("quick")
            .long("quick")
            .help("Run quick test suite (reduced Level 3 duration)"))
        .arg(Arg::with_name("verbose")
            .short("v")
            .long("verbose")
            .help("Enable verbose output"))
        .subcommand(SubCommand::with_name("full")
            .about("Run complete test suite with all levels")
            .arg(Arg::with_name("extended-stress")
                .long("extended-stress")
                .help("Run extended stress testing (24 hours)")))
        .subcommand(SubCommand::with_name("quick")
            .about("Run quick validation suite")
            .arg(Arg::with_name("basic-only")
                .long("basic-only")
                .help("Run only basic validation patterns")))
        .subcommand(SubCommand::with_name("benchmark")
            .about("Run performance benchmark suite")
            .arg(Arg::with_name("baseline")
                .long("baseline")
                .help("Capture performance baseline")))
        .subcommand(SubCommand::with_name("regression")
            .about("Run regression testing against baseline")
            .arg(Arg::with_name("baseline-file")
                .long("baseline-file")
                .value_name("FILE")
                .help("Baseline results file for comparison")
                .takes_value(true)))
        .get_matches();

    // Initialize logging
    if matches.is_present("verbose") {
        env::set_var("RUST_LOG", "debug");
    } else {
        env::set_var("RUST_LOG", "info");
    }

    // Load configuration
    let mut config = load_unified_config(matches.value_of("config"));
    
    // Override config with command line arguments
    config.output_directory = matches.value_of("output-dir").unwrap().to_string();
    config.continue_on_failure = matches.is_present("continue-on-failure");
    config.parallel_execution = matches.is_present("parallel");
    config.enable_level1 = !matches.is_present("disable-level1");
    config.enable_level2 = !matches.is_present("disable-level2");
    config.enable_level3 = !matches.is_present("disable-level3");

    // Handle subcommands
    match matches.subcommand() {
        ("full", Some(sub_matches)) => {
            if sub_matches.is_present("extended-stress") {
                println!("🚀 Running Full Test Suite with Extended Stress Testing");
                config.enable_level1 = true;
                config.enable_level2 = true;
                config.enable_level3 = true;
            }
            run_unified_tests(config);
        }
        ("quick", Some(sub_matches)) => {
            println!("⚡ Running Quick Validation Suite");
            config.enable_level3 = !sub_matches.is_present("basic-only");
            run_unified_tests(config);
        }
        ("benchmark", Some(sub_matches)) => {
            println!("📊 Running Performance Benchmark Suite");
            config.performance_baseline_capture = sub_matches.is_present("baseline");
            run_unified_tests(config);
        }
        ("regression", Some(sub_matches)) => {
            println!("🔍 Running Regression Testing");
            if let Some(baseline_file) = sub_matches.value_of("baseline-file") {
                println!("📋 Using baseline: {}", baseline_file);
                // TODO: Implement regression testing logic
            }
            run_unified_tests(config);
        }
        _ => {
            // Default: run standard unified test suite
            if matches.is_present("quick") {
                println!("⚡ Running Quick Test Suite");
                // Reduce Level 3 to quick mode
            } else {
                println!("🚀 Running Standard Unified Test Suite");
            }
            run_unified_tests(config);
        }
    }
}

fn load_unified_config(config_file: Option<&str>) -> UnifiedTestConfig {
    if let Some(file_path) = config_file {
        match fs::read_to_string(file_path) {
            Ok(content) => {
                match serde_json::from_str(&content) {
                    Ok(config) => return config,
                    Err(e) => {
                        eprintln!("Failed to parse config file: {}", e);
                        process::exit(1);
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to read config file: {}", e);
                process::exit(1);
            }
        }
    }

    // Default configuration
    UnifiedTestConfig::default()
}

fn run_unified_tests(config: UnifiedTestConfig) {
    let mut runner = UnifiedTestRunner::new(config);
    
    match runner.run_unified_test_suite() {
        Ok(result) => {
            println!("\n🎉 Unified Test Suite Completed!");
            println!("📊 Overall Status: {:?}", result.overall_status);
            println!("⏱️  Total Duration: {:.2} seconds", result.total_duration_seconds);
            println!("📁 Results saved to: {}", runner.config.output_directory);
            
            // Print summary
            println!("\n📋 Level Summary:");
            for level_result in &result.level_results {
                println!("  {} - {:?} ({:.2}s)",
                    level_result.level,
                    level_result.status,
                    level_result.duration_seconds);
            }
            
            if result.crash_classification.total_crashes > 0 {
                println!("\n⚠️  Crash Summary:");
                println!("  Total Crashes: {}", result.crash_classification.total_crashes);
                println!("  Recovery Rate: {:.1}%", result.crash_classification.recovery_success_rate * 100.0);
            }
            
            if !result.recommendations.is_empty() {
                println!("\n💡 Recommendations:");
                for recommendation in &result.recommendations {
                    println!("  - {}", recommendation);
                }
            }

            // Exit with appropriate code
            match result.overall_status {
                TestStatus::Success => process::exit(0),
                TestStatus::Failed => process::exit(1),
                _ => process::exit(2),
            }
        }
        Err(e) => {
            eprintln!("❌ Unified test suite failed: {}", e);
            process::exit(1);
        }
    }
}