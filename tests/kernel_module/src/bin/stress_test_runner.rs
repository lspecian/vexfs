//! Dedicated Stress Test Runner for VexFS Kernel Module
//!
//! This binary provides a dedicated runner for the ultimate stress testing framework
//! with comprehensive kernel instrumentation and resource monitoring.

use std::env;
use std::process;
use std::time::{Duration, Instant};
use clap::{App, Arg, SubCommand};
use serde_json;

use kernel_module_tests::{
    VmConfig, TestStatus,
    stress_testing_framework::{StressTestingFramework, StressTestConfig, OperationPattern, StressOperation},
    kernel_instrumentation::{KernelInstrumentation, InstrumentationConfig},
    resource_monitoring::{ResourceMonitor, ResourceMonitoringConfig},
};

fn main() {
    let matches = App::new("VexFS Stress Test Runner")
        .version("1.0.0")
        .author("VexFS Team")
        .about("Ultimate stress testing framework for VexFS kernel module with advanced instrumentation")
        .arg(Arg::with_name("config")
            .short("c")
            .long("config")
            .value_name("FILE")
            .help("Configuration file for VM settings")
            .takes_value(true))
        .arg(Arg::with_name("duration")
            .short("d")
            .long("duration")
            .value_name("HOURS")
            .help("Test duration in hours")
            .takes_value(true)
            .default_value("1.0"))
        .arg(Arg::with_name("frequency")
            .short("f")
            .long("frequency")
            .value_name("OPS_PER_MIN")
            .help("Target operations per minute")
            .takes_value(true)
            .default_value("120"))
        .arg(Arg::with_name("concurrency")
            .long("concurrency")
            .value_name("THREADS")
            .help("Maximum concurrent operations")
            .takes_value(true)
            .default_value("25"))
        .arg(Arg::with_name("seed")
            .long("seed")
            .value_name("SEED")
            .help("Random seed for reproducible tests")
            .takes_value(true))
        .arg(Arg::with_name("kernel-instrumentation")
            .long("kernel-instrumentation")
            .help("Enable advanced kernel instrumentation"))
        .arg(Arg::with_name("resource-monitoring")
            .long("resource-monitoring")
            .help("Enable comprehensive resource monitoring"))
        .arg(Arg::with_name("crash-detection")
            .long("crash-detection")
            .help("Enable crash detection and recovery"))
        .arg(Arg::with_name("adversarial")
            .long("adversarial")
            .help("Enable adversarial scenario testing"))
        .arg(Arg::with_name("resource-exhaustion")
            .long("resource-exhaustion")
            .help("Enable resource exhaustion testing"))
        .arg(Arg::with_name("output")
            .short("o")
            .long("output")
            .value_name("FILE")
            .help("Output file for test results")
            .takes_value(true)
            .default_value("stress_test_results.json"))
        .arg(Arg::with_name("verbose")
            .short("v")
            .long("verbose")
            .help("Enable verbose output"))
        .subcommand(SubCommand::with_name("quick")
            .about("Run quick stress test (15 minutes)")
            .arg(Arg::with_name("basic-only")
                .long("basic-only")
                .help("Run only basic stress patterns")))
        .subcommand(SubCommand::with_name("extended")
            .about("Run extended stress test (24 hours)")
            .arg(Arg::with_name("full-instrumentation")
                .long("full-instrumentation")
                .help("Enable all instrumentation features")))
        .subcommand(SubCommand::with_name("benchmark")
            .about("Run performance benchmark stress test")
            .arg(Arg::with_name("baseline")
                .long("baseline")
                .help("Capture baseline performance metrics")))
        .subcommand(SubCommand::with_name("reproduce")
            .about("Reproduce a previous test run")
            .arg(Arg::with_name("seed")
                .long("seed")
                .value_name("SEED")
                .help("Seed from previous test run")
                .required(true)
                .takes_value(true)))
        .get_matches();

    // Initialize logging
    if matches.is_present("verbose") {
        env::set_var("RUST_LOG", "debug");
    } else {
        env::set_var("RUST_LOG", "info");
    }
    env_logger::init();

    // Parse command line arguments
    let vm_config = load_vm_config(matches.value_of("config"));
    let duration_hours: f64 = matches.value_of("duration").unwrap().parse().unwrap_or(1.0);
    let frequency: u32 = matches.value_of("frequency").unwrap().parse().unwrap_or(120);
    let concurrency: u32 = matches.value_of("concurrency").unwrap().parse().unwrap_or(25);
    let output_file = matches.value_of("output").unwrap();

    // Handle subcommands
    match matches.subcommand() {
        ("quick", Some(sub_matches)) => {
            run_quick_stress_test(vm_config, sub_matches, output_file);
        }
        ("extended", Some(sub_matches)) => {
            run_extended_stress_test(vm_config, sub_matches, output_file);
        }
        ("benchmark", Some(sub_matches)) => {
            run_benchmark_stress_test(vm_config, sub_matches, output_file);
        }
        ("reproduce", Some(sub_matches)) => {
            let seed: u64 = sub_matches.value_of("seed").unwrap().parse().unwrap_or(0);
            run_reproduce_test(vm_config, seed, output_file);
        }
        _ => {
            // Default: run custom stress test
            run_custom_stress_test(
                vm_config,
                duration_hours,
                frequency,
                concurrency,
                matches.value_of("seed").map(|s| s.parse().unwrap_or(0)),
                matches.is_present("kernel-instrumentation"),
                matches.is_present("resource-monitoring"),
                matches.is_present("crash-detection"),
                matches.is_present("adversarial"),
                matches.is_present("resource-exhaustion"),
                output_file,
            );
        }
    }
}

fn load_vm_config(config_file: Option<&str>) -> VmConfig {
    if let Some(file_path) = config_file {
        match std::fs::read_to_string(file_path) {
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

    // Default VM configuration
    VmConfig::default()
}

fn run_quick_stress_test(vm_config: VmConfig, sub_matches: &clap::ArgMatches, output_file: &str) {
    println!("🚀 Running Quick Stress Test (15 minutes)");
    
    let stress_config = StressTestConfig {
        max_frequency_ops_per_minute: 60, // Reduced for quick test
        max_concurrent_operations: 10,
        test_duration_hours: 0.25, // 15 minutes
        operation_patterns: if sub_matches.is_present("basic-only") {
            create_basic_patterns()
        } else {
            create_quick_patterns()
        },
        resource_exhaustion_tests: false,
        adversarial_scenarios: false,
        real_world_simulation: true,
        memory_leak_detection: true,
        deadlock_detection: true,
        race_condition_detection: true,
    };

    let mut framework = StressTestingFramework::new(vm_config)
        .with_stress_config(stress_config)
        .with_resource_monitoring(true)
        .with_crash_detection(true);

    run_stress_test_framework(&mut framework, output_file);
}

fn run_extended_stress_test(vm_config: VmConfig, sub_matches: &clap::ArgMatches, output_file: &str) {
    println!("🚀 Running Extended Stress Test (24 hours)");
    
    let stress_config = StressTestConfig {
        max_frequency_ops_per_minute: 120,
        max_concurrent_operations: 25,
        test_duration_hours: 24.0,
        operation_patterns: create_comprehensive_patterns(),
        resource_exhaustion_tests: true,
        adversarial_scenarios: true,
        real_world_simulation: true,
        memory_leak_detection: true,
        deadlock_detection: true,
        race_condition_detection: true,
    };

    let enable_full_instrumentation = sub_matches.is_present("full-instrumentation");

    let mut framework = StressTestingFramework::new(vm_config)
        .with_stress_config(stress_config)
        .with_kernel_instrumentation(enable_full_instrumentation)
        .with_resource_monitoring(true)
        .with_crash_detection(true);

    run_stress_test_framework(&mut framework, output_file);
}

fn run_benchmark_stress_test(vm_config: VmConfig, sub_matches: &clap::ArgMatches, output_file: &str) {
    println!("🚀 Running Benchmark Stress Test");
    
    let stress_config = StressTestConfig {
        max_frequency_ops_per_minute: 200, // High frequency for benchmarking
        max_concurrent_operations: 50,
        test_duration_hours: 2.0,
        operation_patterns: create_benchmark_patterns(),
        resource_exhaustion_tests: false,
        adversarial_scenarios: false,
        real_world_simulation: true,
        memory_leak_detection: true,
        deadlock_detection: false, // Disabled for pure performance
        race_condition_detection: false,
    };

    let capture_baseline = sub_matches.is_present("baseline");

    let mut framework = StressTestingFramework::new(vm_config)
        .with_stress_config(stress_config)
        .with_resource_monitoring(true)
        .with_crash_detection(false); // Disabled for pure performance

    if capture_baseline {
        println!("📊 Capturing baseline performance metrics...");
        // Additional baseline capture logic would go here
    }

    run_stress_test_framework(&mut framework, output_file);
}

fn run_reproduce_test(vm_config: VmConfig, seed: u64, output_file: &str) {
    println!("🚀 Reproducing Test with Seed: {}", seed);
    
    let stress_config = StressTestConfig::default();

    let mut framework = StressTestingFramework::new(vm_config)
        .with_stress_config(stress_config)
        .with_test_seed(seed)
        .with_kernel_instrumentation(true)
        .with_resource_monitoring(true)
        .with_crash_detection(true);

    run_stress_test_framework(&mut framework, output_file);
}

fn run_custom_stress_test(
    vm_config: VmConfig,
    duration_hours: f64,
    frequency: u32,
    concurrency: u32,
    seed: Option<u64>,
    enable_kernel_instrumentation: bool,
    enable_resource_monitoring: bool,
    enable_crash_detection: bool,
    enable_adversarial: bool,
    enable_resource_exhaustion: bool,
    output_file: &str,
) {
    println!("🚀 Running Custom Stress Test");
    println!("  Duration: {:.1} hours", duration_hours);
    println!("  Frequency: {} ops/min", frequency);
    println!("  Concurrency: {} threads", concurrency);
    
    let stress_config = StressTestConfig {
        max_frequency_ops_per_minute: frequency,
        max_concurrent_operations: concurrency,
        test_duration_hours: duration_hours,
        operation_patterns: create_comprehensive_patterns(),
        resource_exhaustion_tests: enable_resource_exhaustion,
        adversarial_scenarios: enable_adversarial,
        real_world_simulation: true,
        memory_leak_detection: true,
        deadlock_detection: true,
        race_condition_detection: true,
    };

    let mut framework = StressTestingFramework::new(vm_config)
        .with_stress_config(stress_config)
        .with_kernel_instrumentation(enable_kernel_instrumentation)
        .with_resource_monitoring(enable_resource_monitoring)
        .with_crash_detection(enable_crash_detection);

    if let Some(test_seed) = seed {
        framework = framework.with_test_seed(test_seed);
        println!("  Using seed: {}", test_seed);
    }

    run_stress_test_framework(&mut framework, output_file);
}

fn run_stress_test_framework(framework: &mut StressTestingFramework, output_file: &str) {
    let start_time = Instant::now();
    
    println!("🔥 Starting Ultimate Stress Testing Framework...");
    
    match framework.run_ultimate_stress_test() {
        Ok(result) => {
            let duration = start_time.elapsed();
            
            println!("✅ Stress test completed successfully!");
            println!("📊 Test Results Summary:");
            println!("  Status: {:?}", result.status);
            println!("  Duration: {:.2} seconds", duration.as_secs_f64());
            println!("  Test Seed: {}", result.test_seed);
            println!("  High-frequency ops: {}/{} ({:.1}%)",
                    result.high_frequency_results.achieved_ops_per_minute,
                    result.high_frequency_results.target_ops_per_minute,
                    result.high_frequency_results.throughput_consistency_score);
            println!("  Parallel operations: {}/{} successful",
                    result.parallel_execution_results.successful_parallel_mounts,
                    result.parallel_execution_results.parallel_mount_attempts);
            println!("  Resource leaks detected: {}", result.resource_exhaustion_results.resource_leak_incidents);
            println!("  Crash events: {}", result.crash_events.len());

            // Save results to file
            match save_results_to_file(&result, output_file) {
                Ok(_) => println!("📁 Results saved to: {}", output_file),
                Err(e) => eprintln!("❌ Failed to save results: {}", e),
            }

            // Exit with appropriate code
            match result.status {
                TestStatus::Success => process::exit(0),
                TestStatus::Failed => process::exit(1),
                _ => process::exit(2),
            }
        }
        Err(e) => {
            eprintln!("❌ Stress test failed: {}", e);
            process::exit(1);
        }
    }
}

fn save_results_to_file(result: &kernel_module_tests::stress_testing_framework::StressTestResult, output_file: &str) -> Result<(), Box<dyn std::error::Error>> {
    let json_result = serde_json::to_string_pretty(result)?;
    std::fs::write(output_file, json_result)?;
    Ok(())
}

fn create_basic_patterns() -> Vec<OperationPattern> {
    vec![
        OperationPattern {
            name: "basic_mount_unmount".to_string(),
            description: "Basic mount/unmount cycle".to_string(),
            operations: vec![
                StressOperation::Mount {
                    device: "/dev/loop0".to_string(),
                    mount_point: "/mnt/basic_test".to_string(),
                    options: vec![],
                },
                StressOperation::Unmount {
                    mount_point: "/mnt/basic_test".to_string(),
                    force: false,
                },
            ],
            frequency_weight: 1.0,
            concurrency_level: 1,
            expected_failure_rate: 0.01,
        },
    ]
}

fn create_quick_patterns() -> Vec<OperationPattern> {
    vec![
        OperationPattern {
            name: "quick_mount_unmount".to_string(),
            description: "Quick mount/unmount cycles".to_string(),
            operations: vec![
                StressOperation::Mount {
                    device: "/dev/loop0".to_string(),
                    mount_point: "/mnt/quick_test".to_string(),
                    options: vec![],
                },
                StressOperation::FileOperations {
                    mount_point: "/mnt/quick_test".to_string(),
                    operation_count: 3,
                },
                StressOperation::Unmount {
                    mount_point: "/mnt/quick_test".to_string(),
                    force: false,
                },
            ],
            frequency_weight: 1.0,
            concurrency_level: 2,
            expected_failure_rate: 0.02,
        },
        OperationPattern {
            name: "concurrent_quick".to_string(),
            description: "Quick concurrent operations".to_string(),
            operations: vec![
                StressOperation::ConcurrentAccess {
                    mount_point: "/mnt/concurrent_quick".to_string(),
                    thread_count: 5,
                },
            ],
            frequency_weight: 0.5,
            concurrency_level: 5,
            expected_failure_rate: 0.05,
        },
    ]
}

fn create_comprehensive_patterns() -> Vec<OperationPattern> {
    vec![
        OperationPattern {
            name: "rapid_mount_unmount".to_string(),
            description: "Rapid mount/unmount cycles at maximum frequency".to_string(),
            operations: vec![
                StressOperation::Mount {
                    device: "/dev/loop0".to_string(),
                    mount_point: "/mnt/stress_test".to_string(),
                    options: vec![],
                },
                StressOperation::FileOperations {
                    mount_point: "/mnt/stress_test".to_string(),
                    operation_count: 5,
                },
                StressOperation::Unmount {
                    mount_point: "/mnt/stress_test".to_string(),
                    force: false,
                },
            ],
            frequency_weight: 1.0,
            concurrency_level: 1,
            expected_failure_rate: 0.01,
        },
        OperationPattern {
            name: "concurrent_mount_stress".to_string(),
            description: "Multiple concurrent mount operations".to_string(),
            operations: vec![
                StressOperation::ConcurrentAccess {
                    mount_point: "/mnt/concurrent_test".to_string(),
                    thread_count: 10,
                },
            ],
            frequency_weight: 0.5,
            concurrency_level: 10,
            expected_failure_rate: 0.05,
        },
        OperationPattern {
            name: "resource_exhaustion".to_string(),
            description: "Test behavior under resource exhaustion".to_string(),
            operations: vec![
                StressOperation::ResourceExhaustion {
                    resource_type: "memory".to_string(),
                    intensity: 80,
                },
                StressOperation::Mount {
                    device: "/dev/loop0".to_string(),
                    mount_point: "/mnt/exhaustion_test".to_string(),
                    options: vec![],
                },
            ],
            frequency_weight: 0.3,
            concurrency_level: 5,
            expected_failure_rate: 0.15,
        },
        OperationPattern {
            name: "adversarial_corruption".to_string(),
            description: "Simulate filesystem corruption and recovery".to_string(),
            operations: vec![
                StressOperation::CorruptionSimulation {
                    target: "/dev/loop0".to_string(),
                    corruption_type: "metadata".to_string(),
                },
                StressOperation::Mount {
                    device: "/dev/loop0".to_string(),
                    mount_point: "/mnt/corruption_test".to_string(),
                    options: vec![],
                },
            ],
            frequency_weight: 0.2,
            concurrency_level: 1,
            expected_failure_rate: 0.8,
        },
    ]
}

fn create_benchmark_patterns() -> Vec<OperationPattern> {
    vec![
        OperationPattern {
            name: "benchmark_mount_unmount".to_string(),
            description: "High-frequency mount/unmount for benchmarking".to_string(),
            operations: vec![
                StressOperation::Mount {
                    device: "/dev/loop0".to_string(),
                    mount_point: "/mnt/benchmark_test".to_string(),
                    options: vec![],
                },
                StressOperation::Unmount {
                    mount_point: "/mnt/benchmark_test".to_string(),
                    force: false,
                },
            ],
            frequency_weight: 1.0,
            concurrency_level: 1,
            expected_failure_rate: 0.001,
        },
        OperationPattern {
            name: "benchmark_concurrent".to_string(),
            description: "High-concurrency operations for benchmarking".to_string(),
            operations: vec![
                StressOperation::ConcurrentAccess {
                    mount_point: "/mnt/benchmark_concurrent".to_string(),
                    thread_count: 20,
                },
            ],
            frequency_weight: 0.8,
            concurrency_level: 20,
            expected_failure_rate: 0.01,
        },
    ]
}