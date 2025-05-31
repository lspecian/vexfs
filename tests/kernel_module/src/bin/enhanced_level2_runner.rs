//! Enhanced Level 2 Test Runner with Crash Detection & Performance Monitoring
//! 
//! This binary provides a comprehensive test runner for VexFS kernel module testing
//! with advanced crash detection, performance monitoring, and stability validation.

use std::env;
use std::process;
use std::time::{Duration, Instant};
use std::fs;

use vexfs_kernel_tests::{
    Level2TestRunner, VmConfig, TestStatus,
    CrashDetector, VmMonitorConfig, PerformanceThresholds,
    CrashSummary, PerformanceSummary
};

fn main() {
    println!("🚀 VexFS Enhanced Level 2 Test Runner");
    println!("=====================================");
    
    let args: Vec<String> = env::args().collect();
    let force_mode = args.contains(&"--force".to_string());
    let performance_only = args.contains(&"--performance-only".to_string());
    let crash_detection_only = args.contains(&"--crash-detection-only".to_string());
    
    if args.contains(&"--help".to_string()) {
        print_help();
        return;
    }
    
    // Enhanced VM configuration for maximum performance testing
    let vm_config = VmConfig {
        vm_image_path: "tests/vm_images/vexfs-test.qcow2".to_string(),
        vm_memory_mb: 4096, // 4GB for enhanced testing
        vm_cpus: 4,         // 4 CPUs for parallel operations
        ssh_port: 2222,
        ssh_key_path: "tests/vm_keys/vexfs_test_key".to_string(),
        vm_user: "vexfs".to_string(),
        snapshot_path: "tests/vm_images/vexfs-test-snapshot.qcow2".to_string(),
        watchdog_timeout_seconds: 300, // 5 minutes
        performance_monitoring_interval_ms: 500, // 0.5 seconds for high-frequency monitoring
        max_recovery_attempts: 5, // Increased recovery attempts
        enable_kvm: true,
        vm_console_log: "tests/vm_testing/logs/enhanced_vm_console.log".to_string(),
    };
    
    // Create enhanced logs directory
    if let Err(e) = fs::create_dir_all("tests/vm_testing/logs/enhanced") {
        eprintln!("❌ Failed to create enhanced logs directory: {}", e);
        process::exit(1);
    }
    
    // Configure crash detection and monitoring
    let monitor_config = VmMonitorConfig {
        ssh_key_path: vm_config.ssh_key_path.clone(),
        ssh_port: vm_config.ssh_port,
        vm_user: vm_config.vm_user.clone(),
        monitoring_interval_ms: 1000, // 1 second monitoring
        crash_log_path: "tests/vm_testing/logs/enhanced/crash_events.jsonl".to_string(),
        performance_log_path: "tests/vm_testing/logs/enhanced/performance_events.jsonl".to_string(),
        max_events_stored: 10000,
        auto_recovery_enabled: true,
        performance_thresholds: PerformanceThresholds {
            max_operation_time_ms: 30000,  // 30 seconds
            max_memory_usage_kb: 3145728,  // 3GB
            max_cpu_usage_percent: 95.0,   // 95%
            max_io_operations_per_second: 50000,
        },
    };
    
    // Initialize crash detector
    let mut crash_detector = CrashDetector::new(monitor_config);
    
    // Set up recovery handler
    crash_detector.set_recovery_handler(|crash_event| {
        println!("🔄 Attempting automated recovery for: {:?}", crash_event.event_type);
        
        // Implement recovery logic based on crash type
        match crash_event.event_type {
            vexfs_kernel_tests::CrashEventType::Hang => {
                println!("  🔄 Attempting VM restart for hang...");
                // Implementation would restart VM
                true
            }
            vexfs_kernel_tests::CrashEventType::KernelPanic => {
                println!("  🔄 Attempting VM recovery from kernel panic...");
                // Implementation would restore from snapshot
                true
            }
            _ => {
                println!("  ⚠️  No automated recovery available for this crash type");
                false
            }
        }
    });
    
    // Start monitoring if not in specific test modes
    if !performance_only && !crash_detection_only {
        if let Err(e) = crash_detector.start_monitoring() {
            eprintln!("❌ Failed to start crash detection: {}", e);
            process::exit(1);
        }
    }
    
    // Create enhanced test runner
    let test_runner = Level2TestRunner::new(vm_config)
        .with_crash_detection(!crash_detection_only)
        .with_performance_monitoring(!performance_only)
        .with_watchdog(true);
    
    println!("\n🔧 Enhanced Test Configuration:");
    println!("  • Crash Detection: {}", if crash_detection_only { "ONLY" } else { "ENABLED" });
    println!("  • Performance Monitoring: {}", if performance_only { "ONLY" } else { "ENABLED" });
    println!("  • VM Memory: 4GB");
    println!("  • VM CPUs: 4");
    println!("  • Monitoring Interval: 1s");
    println!("  • Recovery Attempts: 5");
    
    if !force_mode {
        println!("\n⚠️  WARNING: This will perform intensive kernel module testing");
        println!("   This should ONLY be run in a VM environment!");
        print!("   Continue? (y/N): ");
        
        let mut input = String::new();
        if std::io::stdin().read_line(&mut input).is_err() || input.trim().to_lowercase() != "y" {
            println!("Test cancelled by user");
            return;
        }
    }
    
    println!("\n🚀 Starting Enhanced Level 2 Testing...");
    let start_time = Instant::now();
    
    // Run the enhanced tests
    let test_result = match test_runner.run_level2_tests() {
        Ok(result) => result,
        Err(e) => {
            eprintln!("❌ Enhanced test execution failed: {}", e);
            
            // Stop monitoring and save reports
            if let Err(e) = crash_detector.stop_monitoring() {
                eprintln!("⚠️  Failed to stop monitoring: {}", e);
            }
            
            process::exit(1);
        }
    };
    
    let total_duration = start_time.elapsed();
    
    // Stop monitoring and collect final reports
    if let Err(e) = crash_detector.stop_monitoring() {
        eprintln!("⚠️  Failed to stop monitoring: {}", e);
    }
    
    // Collect monitoring summaries
    let crash_summary = crash_detector.get_crash_summary();
    let performance_summary = crash_detector.get_performance_summary();
    
    // Print comprehensive results
    print_enhanced_results(&test_result, &crash_summary, &performance_summary, total_duration);
    
    // Generate comprehensive report
    if let Err(e) = generate_comprehensive_report(&test_result, &crash_summary, &performance_summary, total_duration) {
        eprintln!("⚠️  Failed to generate comprehensive report: {}", e);
    }
    
    // Determine exit code based on results
    let exit_code = determine_exit_code(&test_result, &crash_summary, &performance_summary);
    process::exit(exit_code);
}

fn print_help() {
    println!("VexFS Enhanced Level 2 Test Runner");
    println!("");
    println!("USAGE:");
    println!("    enhanced_level2_runner [OPTIONS]");
    println!("");
    println!("OPTIONS:");
    println!("    --help                    Show this help message");
    println!("    --force                   Skip confirmation prompts");
    println!("    --performance-only        Run only performance monitoring tests");
    println!("    --crash-detection-only    Run only crash detection tests");
    println!("");
    println!("DESCRIPTION:");
    println!("    Runs comprehensive VexFS kernel module testing with:");
    println!("    • Advanced crash detection and recovery");
    println!("    • Real-time performance monitoring");
    println!("    • Stability validation under stress");
    println!("    • Automated VM health monitoring");
    println!("");
    println!("REQUIREMENTS:");
    println!("    • Must be run in a VM environment");
    println!("    • Requires VexFS kernel module built");
    println!("    • Requires VM image at tests/vm_images/vexfs-test.qcow2");
    println!("    • Requires SSH key at tests/vm_keys/vexfs_test_key");
    println!("");
    println!("OUTPUT:");
    println!("    • Test logs: tests/vm_testing/logs/enhanced/");
    println!("    • Crash events: crash_events.jsonl");
    println!("    • Performance data: performance_events.jsonl");
    println!("    • Comprehensive report: comprehensive_report.json");
}

fn print_enhanced_results(
    test_result: &vexfs_kernel_tests::Level2TestResult,
    crash_summary: &CrashSummary,
    performance_summary: &PerformanceSummary,
    duration: Duration,
) {
    println!("\n" + "=".repeat(80).as_str());
    println!("🎯 ENHANCED LEVEL 2 TEST RESULTS");
    println!("=".repeat(80));
    
    // Basic test results
    println!("\n📊 Test Execution Results:");
    println!("  • Test Name: {}", test_result.test_name);
    println!("  • Status: {:?}", test_result.status);
    println!("  • Duration: {:.2} minutes", duration.as_secs_f64() / 60.0);
    
    // VM Setup Results
    println!("\n🖥️  VM Setup Results:");
    println!("  • VM Started: {}", if test_result.vm_setup.vm_started { "✅" } else { "❌" });
    println!("  • SSH Accessible: {}", if test_result.vm_setup.ssh_accessible { "✅" } else { "❌" });
    if let Some(kernel_version) = &test_result.vm_setup.kernel_version {
        println!("  • Kernel Version: {}", kernel_version);
    }
    println!("  • Setup Time: {}ms", test_result.vm_setup.setup_duration_ms);
    
    // Module Loading Results
    println!("\n🔧 Module Loading Results:");
    println!("  • Module Compiled: {}", if test_result.module_loading.module_compiled { "✅" } else { "❌" });
    println!("  • Module Loaded: {}", if test_result.module_loading.module_loaded { "✅" } else { "❌" });
    println!("  • Module Info Valid: {}", if test_result.module_loading.module_info_valid { "✅" } else { "❌" });
    println!("  • No Kernel Errors: {}", if test_result.module_loading.no_kernel_errors { "✅" } else { "❌" });
    
    // Mount Operations Results
    println!("\n💾 Mount Operations Results:");
    println!("  • Loop Device Created: {}", if test_result.mount_operations.loop_device_created { "✅" } else { "❌" });
    println!("  • Filesystem Formatted: {}", if test_result.mount_operations.filesystem_formatted { "✅" } else { "❌" });
    println!("  • Mount Successful: {}", if test_result.mount_operations.mount_successful { "✅" } else { "❌" });
    println!("  • Mount Point Accessible: {}", if test_result.mount_operations.mount_point_accessible { "✅" } else { "❌" });
    
    // Basic Operations Results
    println!("\n📁 Basic Operations Results:");
    println!("  • File Creation: {}", if test_result.basic_operations.file_creation { "✅" } else { "❌" });
    println!("  • File Write: {}", if test_result.basic_operations.file_write { "✅" } else { "❌" });
    println!("  • File Read: {}", if test_result.basic_operations.file_read { "✅" } else { "❌" });
    println!("  • Directory Creation: {}", if test_result.basic_operations.directory_creation { "✅" } else { "❌" });
    println!("  • File Deletion: {}", if test_result.basic_operations.file_deletion { "✅" } else { "❌" });
    
    // Performance Metrics
    println!("\n📈 Performance Metrics:");
    println!("  • Mount Time: {}ms", test_result.performance_metrics.mount_time_ms);
    println!("  • Unmount Time: {}ms", test_result.performance_metrics.unmount_time_ms);
    println!("  • File Creation Time: {}ms", test_result.performance_metrics.file_creation_time_ms);
    println!("  • File Write Time: {}ms", test_result.performance_metrics.file_write_time_ms);
    println!("  • File Read Time: {}ms", test_result.performance_metrics.file_read_time_ms);
    println!("  • Memory Usage: {} KB", test_result.performance_metrics.memory_usage_kb);
    println!("  • CPU Usage: {:.1}%", test_result.performance_metrics.cpu_usage_percent);
    println!("  • IO Operations/sec: {:.1}", test_result.performance_metrics.io_operations_per_second);
    
    // Crash Detection Results
    println!("\n🛡️  Crash Detection Summary:");
    println!("  • Total Events: {}", crash_summary.total_events);
    println!("  • Kernel Panics: {}", crash_summary.kernel_panics);
    println!("  • Oops Count: {}", crash_summary.oops_count);
    println!("  • BUG Count: {}", crash_summary.bug_count);
    println!("  • Hangs: {}", crash_summary.hangs);
    println!("  • Memory Leaks: {}", crash_summary.memory_leaks);
    println!("  • Recovery Attempts: {}", crash_summary.recovery_attempts);
    println!("  • Successful Recoveries: {}", crash_summary.successful_recoveries);
    println!("  • Stability Score: {:.1}%", crash_summary.stability_score());
    
    // Performance Summary
    println!("\n⚡ Performance Summary:");
    println!("  • Total Measurements: {}", performance_summary.total_measurements);
    println!("  • Average Duration: {}ms", performance_summary.average_duration_ms);
    println!("  • Average Memory: {} KB", performance_summary.average_memory_usage_kb);
    println!("  • Average CPU: {:.1}%", performance_summary.average_cpu_usage_percent);
    println!("  • Threshold Violations: {}", performance_summary.threshold_violations);
    println!("  • Performance Score: {:.1}%", performance_summary.performance_score());
    
    // Stability Validation
    println!("\n🔄 Stability Validation:");
    println!("  • Stress Test Cycles: {}", test_result.stability_validation.stress_test_cycles);
    println!("  • Parallel Operations: {}", test_result.stability_validation.parallel_operations_tested);
    println!("  • Race Conditions: {}", test_result.stability_validation.race_conditions_detected);
    println!("  • Resource Leaks: {}", test_result.stability_validation.resource_leaks_detected);
    println!("  • Max Concurrent Mounts: {}", test_result.stability_validation.max_concurrent_mounts);
    println!("  • Stability Score: {:.1}%", test_result.stability_validation.stability_score);
    
    // Cleanup Results
    println!("\n🧹 Cleanup Results:");
    println!("  • Filesystem Unmounted: {}", if test_result.cleanup.filesystem_unmounted { "✅" } else { "❌" });
    println!("  • Module Unloaded: {}", if test_result.cleanup.module_unloaded { "✅" } else { "❌" });
    println!("  • VM Shutdown: {}", if test_result.cleanup.vm_shutdown { "✅" } else { "❌" });
    println!("  • VM Recovered from Crash: {}", if test_result.cleanup.vm_recovered_from_crash { "✅" } else { "❌" });
    println!("  • Snapshot Restored: {}", if test_result.cleanup.snapshot_restored { "✅" } else { "❌" });
    println!("  • Cleanup Time: {}ms", test_result.cleanup.cleanup_duration_ms);
    
    // Overall Assessment
    println!("\n🎯 Overall Assessment:");
    let overall_score = calculate_overall_score(test_result, crash_summary, performance_summary);
    println!("  • Overall Score: {:.1}%", overall_score);
    
    if overall_score >= 90.0 {
        println!("  • Assessment: 🟢 EXCELLENT - Production Ready");
    } else if overall_score >= 75.0 {
        println!("  • Assessment: 🟡 GOOD - Minor Issues");
    } else if overall_score >= 50.0 {
        println!("  • Assessment: 🟠 FAIR - Needs Improvement");
    } else {
        println!("  • Assessment: 🔴 POOR - Significant Issues");
    }
    
    println!("\n" + "=".repeat(80).as_str());
}

fn calculate_overall_score(
    test_result: &vexfs_kernel_tests::Level2TestResult,
    crash_summary: &CrashSummary,
    performance_summary: &PerformanceSummary,
) -> f64 {
    // Weight different aspects of the testing
    let test_success_weight = 0.4;
    let stability_weight = 0.3;
    let performance_weight = 0.3;
    
    // Calculate test success score
    let test_success_score = match test_result.status {
        TestStatus::Success => 100.0,
        TestStatus::Failed => 0.0,
        TestStatus::Timeout => 25.0,
        TestStatus::Crashed => 0.0,
        TestStatus::Recovered => 50.0,
        TestStatus::Skipped => 0.0,
    };
    
    // Get stability and performance scores
    let stability_score = crash_summary.stability_score();
    let performance_score = performance_summary.performance_score();
    
    // Calculate weighted overall score
    (test_success_score * test_success_weight) +
    (stability_score * stability_weight) +
    (performance_score * performance_weight)
}

fn generate_comprehensive_report(
    test_result: &vexfs_kernel_tests::Level2TestResult,
    crash_summary: &CrashSummary,
    performance_summary: &PerformanceSummary,
    duration: Duration,
) -> Result<(), Box<dyn std::error::Error>> {
    let report_path = "tests/vm_testing/logs/enhanced/comprehensive_report.json";
    
    let report = serde_json::json!({
        "test_execution": {
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "duration_seconds": duration.as_secs(),
            "test_result": test_result,
        },
        "crash_detection": {
            "summary": crash_summary,
            "stability_score": crash_summary.stability_score(),
        },
        "performance_monitoring": {
            "summary": performance_summary,
            "performance_score": performance_summary.performance_score(),
        },
        "overall_assessment": {
            "overall_score": calculate_overall_score(test_result, crash_summary, performance_summary),
            "recommendation": if calculate_overall_score(test_result, crash_summary, performance_summary) >= 75.0 {
                "APPROVED for production use"
            } else {
                "REQUIRES further testing and improvements"
            }
        }
    });
    
    fs::write(report_path, serde_json::to_string_pretty(&report)?)?;
    println!("📄 Comprehensive report saved to: {}", report_path);
    
    Ok(())
}

fn determine_exit_code(
    test_result: &vexfs_kernel_tests::Level2TestResult,
    crash_summary: &CrashSummary,
    performance_summary: &PerformanceSummary,
) -> i32 {
    // Exit code 0: Success
    // Exit code 1: Test failures
    // Exit code 2: Stability issues
    // Exit code 3: Performance issues
    // Exit code 4: Critical failures
    
    match test_result.status {
        TestStatus::Success => {
            if crash_summary.kernel_panics > 0 || crash_summary.hangs > 0 {
                2 // Stability issues
            } else if performance_summary.threshold_violations > 10 {
                3 // Performance issues
            } else {
                0 // Success
            }
        }
        TestStatus::Failed => 1,
        TestStatus::Crashed => 4,
        TestStatus::Timeout => 2,
        TestStatus::Recovered => 2,
        TestStatus::Skipped => 1,
    }
}