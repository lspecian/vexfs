//! Dedicated Mount-Level Test Runner for VexFS Kernel Module
//!
//! This binary provides a dedicated runner for comprehensive mount-level testing
//! with crash recovery capabilities. It integrates the mount test suite and
//! recovery manager to provide maximum kernel performance validation.

use std::env;
use std::process;
use std::time::{Duration, Instant};
use std::fs;
use serde_json;

use vexfs_kernel_tests::{
    VmConfig, TestStatus,
    mount_test_suite::{MountTestSuite, MountTestResult},
    mount_recovery::{MountRecoveryManager, RecoveryConfig},
    Level2TestRunner,
};

#[derive(Debug)]
struct MountLevelTestConfig {
    pub vm_config: VmConfig,
    pub recovery_config: RecoveryConfig,
    pub test_cycles: u32,
    pub max_concurrent_mounts: u32,
    pub stress_test_cycles: u32,
    pub enable_crash_detection: bool,
    pub enable_recovery: bool,
    pub output_dir: String,
    pub verbose: bool,
}

impl Default for MountLevelTestConfig {
    fn default() -> Self {
        Self {
            vm_config: VmConfig::default(),
            recovery_config: RecoveryConfig::default(),
            test_cycles: 1,
            max_concurrent_mounts: 5,
            stress_test_cycles: 25,
            enable_crash_detection: true,
            enable_recovery: true,
            output_dir: "tests/vm_testing/results".to_string(),
            verbose: false,
        }
    }
}

fn main() {
    println!("🚀 VexFS Mount-Level Test Runner with Crash Recovery");
    println!("====================================================");

    let config = parse_command_line_args();
    
    if config.verbose {
        println!("📋 Test Configuration:");
        println!("  - VM Memory: {}MB", config.vm_config.vm_memory_mb);
        println!("  - VM CPUs: {}", config.vm_config.vm_cpus);
        println!("  - SSH Port: {}", config.vm_config.ssh_port);
        println!("  - Test Cycles: {}", config.test_cycles);
        println!("  - Max Concurrent Mounts: {}", config.max_concurrent_mounts);
        println!("  - Stress Test Cycles: {}", config.stress_test_cycles);
        println!("  - Crash Detection: {}", config.enable_crash_detection);
        println!("  - Auto Recovery: {}", config.enable_recovery);
        println!("  - Output Directory: {}", config.output_dir);
        println!();
    }

    // Create output directory
    if let Err(e) = fs::create_dir_all(&config.output_dir) {
        eprintln!("❌ Failed to create output directory: {}", e);
        process::exit(1);
    }

    // Initialize VM and test infrastructure
    let vm_runner = Level2TestRunner::new(config.vm_config.clone())
        .with_crash_detection(config.enable_crash_detection)
        .with_performance_monitoring(true)
        .with_watchdog(true);

    println!("🔧 Setting up VM infrastructure...");
    match vm_runner.setup_enhanced_vm() {
        Ok(vm_setup) => {
            if !vm_setup.vm_started || !vm_setup.ssh_accessible {
                eprintln!("❌ VM setup failed - cannot proceed with mount tests");
                process::exit(1);
            }
            println!("✅ VM infrastructure ready (kernel: {})", 
                    vm_setup.kernel_version.unwrap_or_else(|| "unknown".to_string()));
        }
        Err(e) => {
            eprintln!("❌ VM setup failed: {}", e);
            process::exit(1);
        }
    }

    // Load and verify kernel module
    println!("📦 Loading VexFS kernel module...");
    match vm_runner.test_module_loading_with_monitoring() {
        Ok(module_result) => {
            if !module_result.module_loaded {
                eprintln!("❌ Kernel module loading failed - cannot proceed");
                process::exit(1);
            }
            println!("✅ VexFS kernel module loaded successfully");
        }
        Err(e) => {
            eprintln!("❌ Module loading failed: {}", e);
            process::exit(1);
        }
    }

    // Run comprehensive mount tests
    let mut overall_success = true;
    let mut test_results = Vec::new();

    for cycle in 1..=config.test_cycles {
        println!("\n🔄 Running Mount Test Cycle {}/{}", cycle, config.test_cycles);
        println!("{}", "=".repeat(50));

        let cycle_start = Instant::now();
        
        // Initialize mount test suite
        let mut mount_suite = MountTestSuite::new(config.vm_config.clone())
            .with_crash_detection(config.enable_crash_detection)
            .with_stress_cycles(config.stress_test_cycles)
            .with_max_concurrent_mounts(config.max_concurrent_mounts);

        // Initialize recovery manager
        let mut recovery_manager = MountRecoveryManager::new(config.vm_config.clone())
            .with_recovery_config(config.recovery_config.clone())
            .with_crash_detection(config.enable_recovery);

        // Start monitoring
        if let Err(e) = recovery_manager.start_monitoring() {
            eprintln!("⚠️  Warning: Failed to start recovery monitoring: {}", e);
        }

        // Run comprehensive mount tests
        match mount_suite.run_comprehensive_mount_tests() {
            Ok(mut result) => {
                let cycle_duration = cycle_start.elapsed();
                result.duration_ms = cycle_duration.as_millis() as u64;
                
                println!("\n📊 Cycle {} Results:", cycle);
                print_test_results(&result, config.verbose);
                
                if !matches!(result.status, TestStatus::Success) {
                    overall_success = false;
                    println!("❌ Cycle {} failed", cycle);
                } else {
                    println!("✅ Cycle {} completed successfully", cycle);
                }
                
                test_results.push(result);
            }
            Err(e) => {
                eprintln!("❌ Cycle {} failed with error: {}", cycle, e);
                overall_success = false;
            }
        }

        // Stop monitoring
        if let Err(e) = recovery_manager.stop_monitoring() {
            eprintln!("⚠️  Warning: Failed to stop recovery monitoring: {}", e);
        }

        // Brief pause between cycles
        if cycle < config.test_cycles {
            println!("⏳ Pausing before next cycle...");
            std::thread::sleep(Duration::from_secs(5));
        }
    }

    // Generate comprehensive report
    println!("\n📋 Generating Comprehensive Test Report...");
    match generate_comprehensive_report(&test_results, &config) {
        Ok(report_path) => {
            println!("✅ Comprehensive report saved to: {}", report_path);
        }
        Err(e) => {
            eprintln!("⚠️  Warning: Failed to generate report: {}", e);
        }
    }

    // Cleanup VM
    println!("\n🧹 Cleaning up VM environment...");
    let cleanup_result = vm_runner.cleanup_enhanced_vm();
    if cleanup_result.vm_shutdown {
        println!("✅ VM cleanup completed");
    } else {
        println!("⚠️  VM cleanup had issues");
    }

    // Final summary
    println!("\n🏁 Mount-Level Testing Complete");
    println!("{}", "=".repeat(40));
    
    if overall_success {
        println!("✅ ALL MOUNT TESTS PASSED");
        println!("🎯 VexFS kernel module demonstrates maximum mount performance");
        process::exit(0);
    } else {
        println!("❌ SOME MOUNT TESTS FAILED");
        println!("🔍 Review test results for detailed analysis");
        process::exit(1);
    }
}

fn parse_command_line_args() -> MountLevelTestConfig {
    let args: Vec<String> = env::args().collect();
    let mut config = MountLevelTestConfig::default();

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--vm-memory" => {
                if i + 1 < args.len() {
                    config.vm_config.vm_memory_mb = args[i + 1].parse().unwrap_or(4096);
                    i += 1;
                }
            }
            "--vm-cpus" => {
                if i + 1 < args.len() {
                    config.vm_config.vm_cpus = args[i + 1].parse().unwrap_or(4);
                    i += 1;
                }
            }
            "--ssh-port" => {
                if i + 1 < args.len() {
                    config.vm_config.ssh_port = args[i + 1].parse().unwrap_or(2222);
                    i += 1;
                }
            }
            "--test-cycles" => {
                if i + 1 < args.len() {
                    config.test_cycles = args[i + 1].parse().unwrap_or(1);
                    i += 1;
                }
            }
            "--max-concurrent" => {
                if i + 1 < args.len() {
                    config.max_concurrent_mounts = args[i + 1].parse().unwrap_or(5);
                    i += 1;
                }
            }
            "--stress-cycles" => {
                if i + 1 < args.len() {
                    config.stress_test_cycles = args[i + 1].parse().unwrap_or(25);
                    i += 1;
                }
            }
            "--output-dir" => {
                if i + 1 < args.len() {
                    config.output_dir = args[i + 1].clone();
                    i += 1;
                }
            }
            "--no-crash-detection" => {
                config.enable_crash_detection = false;
            }
            "--no-recovery" => {
                config.enable_recovery = false;
            }
            "--verbose" | "-v" => {
                config.verbose = true;
            }
            "--help" | "-h" => {
                print_help();
                process::exit(0);
            }
            _ => {
                eprintln!("Unknown argument: {}", args[i]);
                print_help();
                process::exit(1);
            }
        }
        i += 1;
    }

    config
}

fn print_help() {
    println!("VexFS Mount-Level Test Runner");
    println!();
    println!("USAGE:");
    println!("    mount_level_runner [OPTIONS]");
    println!();
    println!("OPTIONS:");
    println!("    --vm-memory <MB>        VM memory in MB (default: 4096)");
    println!("    --vm-cpus <COUNT>       Number of VM CPUs (default: 4)");
    println!("    --ssh-port <PORT>       SSH port for VM access (default: 2222)");
    println!("    --test-cycles <COUNT>   Number of test cycles (default: 1)");
    println!("    --max-concurrent <COUNT> Max concurrent mounts (default: 5)");
    println!("    --stress-cycles <COUNT> Stress test cycles (default: 25)");
    println!("    --output-dir <DIR>      Output directory (default: tests/vm_testing/results)");
    println!("    --no-crash-detection    Disable crash detection");
    println!("    --no-recovery          Disable auto recovery");
    println!("    --verbose, -v          Verbose output");
    println!("    --help, -h             Show this help message");
    println!();
    println!("EXAMPLES:");
    println!("    # Basic mount testing");
    println!("    mount_level_runner");
    println!();
    println!("    # Intensive testing with multiple cycles");
    println!("    mount_level_runner --test-cycles 5 --stress-cycles 100 --verbose");
    println!();
    println!("    # High-concurrency testing");
    println!("    mount_level_runner --max-concurrent 10 --vm-memory 8192 --vm-cpus 8");
}

fn print_test_results(result: &MountTestResult, verbose: bool) {
    println!("  📊 Test Status: {:?}", result.status);
    println!("  ⏱️  Duration: {}ms", result.duration_ms);
    
    // Normal mount tests
    println!("  🔧 Normal Mount Tests:");
    println!("    - Basic mount/unmount: {}", 
            if result.normal_mount_tests.basic_mount_unmount { "✅" } else { "❌" });
    println!("    - Mount options tested: {}", result.normal_mount_tests.mount_with_options.len());
    println!("    - Remount operations: {}", 
            if result.normal_mount_tests.remount_tests { "✅" } else { "❌" });
    println!("    - Mount time: {}ms", result.normal_mount_tests.mount_time_ms);
    println!("    - Unmount time: {}ms", result.normal_mount_tests.unmount_time_ms);

    // Edge case tests
    println!("  ⚠️  Edge Case Tests:");
    println!("    - Invalid options tested: {}", result.edge_case_tests.invalid_mount_options.len());
    println!("    - Already mounted: {}", 
            if result.edge_case_tests.already_mounted_filesystem { "✅" } else { "❌" });
    println!("    - Nonexistent device: {}", 
            if result.edge_case_tests.nonexistent_device { "✅" } else { "❌" });
    println!("    - Corrupted filesystem: {}", 
            if result.edge_case_tests.corrupted_filesystem { "✅" } else { "❌" });

    // Concurrent tests
    println!("  🔄 Concurrent Tests:");
    println!("    - Parallel attempts: {}", result.concurrent_mount_tests.parallel_mount_attempts);
    println!("    - Successful mounts: {}", result.concurrent_mount_tests.successful_concurrent_mounts);
    println!("    - Max concurrent: {}", result.concurrent_mount_tests.max_concurrent_achieved);
    println!("    - Concurrent time: {}ms", result.concurrent_mount_tests.concurrent_mount_time_ms);

    // Stress tests
    println!("  💪 Stress Tests:");
    println!("    - Cycles completed: {}", result.stress_test_results.stress_cycles_completed);
    println!("    - Mount/unmount cycles: {}", result.stress_test_results.mount_unmount_cycles);
    println!("    - Failures detected: {}", result.stress_test_results.failures_detected);
    println!("    - Stability score: {:.1}%", result.stress_test_results.stability_score);

    // Resource constraint tests
    println!("  🔒 Resource Constraint Tests:");
    println!("    - Low memory: {}", 
            if result.resource_constraint_tests.low_memory_mount_test { "✅" } else { "❌" });
    println!("    - High CPU load: {}", 
            if result.resource_constraint_tests.high_cpu_load_mount_test { "✅" } else { "❌" });
    println!("    - Disk constraints: {}", 
            if result.resource_constraint_tests.disk_space_constraint_test { "✅" } else { "❌" });
    println!("    - Recovery successful: {}", 
            if result.resource_constraint_tests.constraint_recovery_successful { "✅" } else { "❌" });

    // Performance metrics
    if verbose {
        println!("  📈 Performance Metrics:");
        println!("    - Memory usage: {} KB", result.performance_metrics.memory_usage_kb);
        println!("    - CPU usage: {:.1}%", result.performance_metrics.cpu_usage_percent);
        println!("    - IO ops/sec: {:.1}", result.performance_metrics.io_operations_per_second);
        println!("    - Kernel memory: {} KB", result.performance_metrics.kernel_memory_usage_kb);
    }

    // Crash events
    if !result.crash_events.is_empty() {
        println!("  🚨 Crash Events: {} detected", result.crash_events.len());
        if verbose {
            for (i, event) in result.crash_events.iter().enumerate() {
                println!("    {}. {:?}: {}", i + 1, event.event_type, event.description);
            }
        }
    }

    // Error details
    if let Some(ref error) = result.error_details {
        println!("  ❌ Error Details: {}", error);
    }
}

fn generate_comprehensive_report(results: &[MountTestResult], config: &MountLevelTestConfig) -> Result<String, Box<dyn std::error::Error>> {
    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
    let report_path = format!("{}/mount_test_report_{}.json", config.output_dir, timestamp);
    
    // Create comprehensive report structure
    let report = serde_json::json!({
        "test_summary": {
            "total_cycles": results.len(),
            "successful_cycles": results.iter().filter(|r| matches!(r.status, TestStatus::Success)).count(),
            "failed_cycles": results.iter().filter(|r| !matches!(r.status, TestStatus::Success)).count(),
            "total_duration_ms": results.iter().map(|r| r.duration_ms).sum::<u64>(),
            "average_duration_ms": if !results.is_empty() { 
                results.iter().map(|r| r.duration_ms).sum::<u64>() / results.len() as u64 
            } else { 0 }
        },
        "configuration": {
            "vm_memory_mb": config.vm_config.vm_memory_mb,
            "vm_cpus": config.vm_config.vm_cpus,
            "test_cycles": config.test_cycles,
            "max_concurrent_mounts": config.max_concurrent_mounts,
            "stress_test_cycles": config.stress_test_cycles,
            "crash_detection_enabled": config.enable_crash_detection,
            "recovery_enabled": config.enable_recovery
        },
        "aggregated_metrics": {
            "total_mount_operations": results.iter().map(|r| r.stress_test_results.mount_unmount_cycles).sum::<u32>(),
            "total_failures": results.iter().map(|r| r.stress_test_results.failures_detected).sum::<u32>(),
            "average_stability_score": if !results.is_empty() {
                results.iter().map(|r| r.stress_test_results.stability_score).sum::<f64>() / results.len() as f64
            } else { 0.0 },
            "total_crash_events": results.iter().map(|r| r.crash_events.len()).sum::<usize>(),
            "concurrent_mount_success_rate": if !results.is_empty() {
                let total_attempts: u32 = results.iter().map(|r| r.concurrent_mount_tests.parallel_mount_attempts).sum();
                let total_successful: u32 = results.iter().map(|r| r.concurrent_mount_tests.successful_concurrent_mounts).sum();
                if total_attempts > 0 { (total_successful as f64 / total_attempts as f64) * 100.0 } else { 0.0 }
            } else { 0.0 }
        },
        "detailed_results": results,
        "timestamp": timestamp.to_string(),
        "test_runner_version": "1.0.0"
    });

    // Write report to file
    let report_json = serde_json::to_string_pretty(&report)?;
    fs::write(&report_path, report_json)?;

    // Also create a summary text report
    let summary_path = format!("{}/mount_test_summary_{}.txt", config.output_dir, timestamp);
    let summary = format!(
        "VexFS Mount-Level Test Summary\n\
         ==============================\n\
         \n\
         Test Configuration:\n\
         - VM Memory: {}MB\n\
         - VM CPUs: {}\n\
         - Test Cycles: {}\n\
         - Max Concurrent Mounts: {}\n\
         - Stress Test Cycles: {}\n\
         \n\
         Results:\n\
         - Total Cycles: {}\n\
         - Successful Cycles: {}\n\
         - Failed Cycles: {}\n\
         - Success Rate: {:.1}%\n\
         - Average Duration: {}ms\n\
         - Total Mount Operations: {}\n\
         - Average Stability Score: {:.1}%\n\
         - Concurrent Mount Success Rate: {:.1}%\n\
         - Total Crash Events: {}\n\
         \n\
         Overall Assessment: {}\n",
        config.vm_config.vm_memory_mb,
        config.vm_config.vm_cpus,
        config.test_cycles,
        config.max_concurrent_mounts,
        config.stress_test_cycles,
        results.len(),
        results.iter().filter(|r| matches!(r.status, TestStatus::Success)).count(),
        results.iter().filter(|r| !matches!(r.status, TestStatus::Success)).count(),
        if !results.is_empty() {
            (results.iter().filter(|r| matches!(r.status, TestStatus::Success)).count() as f64 / results.len() as f64) * 100.0
        } else { 0.0 },
        if !results.is_empty() { 
            results.iter().map(|r| r.duration_ms).sum::<u64>() / results.len() as u64 
        } else { 0 },
        results.iter().map(|r| r.stress_test_results.mount_unmount_cycles).sum::<u32>(),
        if !results.is_empty() {
            results.iter().map(|r| r.stress_test_results.stability_score).sum::<f64>() / results.len() as f64
        } else { 0.0 },
        if !results.is_empty() {
            let total_attempts: u32 = results.iter().map(|r| r.concurrent_mount_tests.parallel_mount_attempts).sum();
            let total_successful: u32 = results.iter().map(|r| r.concurrent_mount_tests.successful_concurrent_mounts).sum();
            if total_attempts > 0 { (total_successful as f64 / total_attempts as f64) * 100.0 } else { 0.0 }
        } else { 0.0 },
        results.iter().map(|r| r.crash_events.len()).sum::<usize>(),
        if results.iter().all(|r| matches!(r.status, TestStatus::Success)) {
            "EXCELLENT - All mount tests passed with maximum performance"
        } else {
            "NEEDS ATTENTION - Some mount tests failed or encountered issues"
        }
    );

    fs::write(&summary_path, summary)?;

    Ok(report_path)
}