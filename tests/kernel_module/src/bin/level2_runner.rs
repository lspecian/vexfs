//! Level 2 Test Runner - VM-Isolated Mount Operations
//! 
//! This runner executes Level 2 tests which perform VM-isolated mount operations
//! testing. These tests require QEMU/KVM and perform actual kernel module loading
//! and filesystem mounting in an isolated virtual machine environment.

use std::env;
use std::process;
use vexfs_kernel_tests::level2_vm_mount_operations::{Level2TestRunner, VmConfig, TestStatus};

fn main() {
    // Parse command line arguments
    let args: Vec<String> = env::args().collect();
    
    let mut vm_memory_mb = 512;
    let mut vm_timeout_seconds = 300;
    let mut test_disk_size_mb = 100;
    let mut verbose = false;
    let mut i = 1;
    
    while i < args.len() {
        match args[i].as_str() {
            "--vm-memory" => {
                if i + 1 < args.len() {
                    vm_memory_mb = args[i + 1].parse().unwrap_or(512);
                    i += 2;
                } else {
                    eprintln!("Error: --vm-memory requires a value");
                    process::exit(1);
                }
            }
            "--vm-timeout" => {
                if i + 1 < args.len() {
                    vm_timeout_seconds = args[i + 1].parse().unwrap_or(300);
                    i += 2;
                } else {
                    eprintln!("Error: --vm-timeout requires a value");
                    process::exit(1);
                }
            }
            "--test-disk-size" => {
                if i + 1 < args.len() {
                    test_disk_size_mb = args[i + 1].parse().unwrap_or(100);
                    i += 2;
                } else {
                    eprintln!("Error: --test-disk-size requires a value");
                    process::exit(1);
                }
            }
            "--verbose" => {
                verbose = true;
                i += 1;
            }
            "--help" | "-h" => {
                print_help();
                process::exit(0);
            }
            _ => {
                eprintln!("Error: Unknown argument '{}'", args[i]);
                print_help();
                process::exit(1);
            }
        }
    }
    
    println!("VexFS Level 2 Test Runner - VM-Isolated Mount Operations");
    println!("=========================================================");
    println!();
    
    if verbose {
        println!("Configuration:");
        println!("  VM Memory: {} MB", vm_memory_mb);
        println!("  VM Timeout: {} seconds", vm_timeout_seconds);
        println!("  Test Disk Size: {} MB", test_disk_size_mb);
        println!("  Verbose: {}", verbose);
        println!();
    }
    
    // Create VM configuration
    let vm_config = VmConfig {
        vm_image_path: "/tmp/vexfs_test_vm.img".to_string(),
        vm_memory_mb,
        vm_cpus: 2,
        ssh_port: 2222,
        ssh_key_path: "tests/vm_keys/vexfs_test_key".to_string(),
        vm_user: "vexfs".to_string(),
    };
    
    // Create and run Level 2 tests
    let runner = Level2TestRunner::new(vm_config);
    
    match runner.run_level2_tests() {
        Ok(result) => {
            println!("Level 2 Test Results:");
            println!("====================");
            
            match result.status {
                TestStatus::Success => {
                    println!("✅ Level 2 Tests: PASSED");
                    println!("🎉 All Level 2 tests passed!");
                    
                    if verbose {
                        println!("Test Details:");
                        println!("  Duration: {:.2}s", result.duration_ms as f64 / 1000.0);
                        println!("  VM Setup: {}", if result.vm_setup.vm_started { "✅" } else { "❌" });
                        println!("  Module Loading: {}", if result.module_loading.module_loaded { "✅" } else { "❌" });
                        println!("  Mount Operations: {}", if result.mount_operations.mount_successful { "✅" } else { "❌" });
                        println!("  Basic Operations: {}", if result.basic_operations.file_creation { "✅" } else { "❌" });
                    }
                    
                    process::exit(0);
                }
                _ => {
                    println!("❌ Level 2 Tests: FAILED");
                    if let Some(ref error) = result.error_details {
                        println!("   Error: {}", error);
                    }
                    
                    if verbose {
                        println!("Failure Details:");
                        println!("  VM Setup: {}", if result.vm_setup.vm_started { "✅" } else { "❌" });
                        println!("  SSH Access: {}", if result.vm_setup.ssh_accessible { "✅" } else { "❌" });
                        println!("  Module Compiled: {}", if result.module_loading.module_compiled { "✅" } else { "❌" });
                        println!("  Module Loaded: {}", if result.module_loading.module_loaded { "✅" } else { "❌" });
                        println!("  Mount Success: {}", if result.mount_operations.mount_successful { "✅" } else { "❌" });
                    }
                    
                    println!("💥 Level 2 tests failed!");
                    process::exit(1);
                }
            }
        }
        Err(e) => {
            eprintln!("❌ Level 2 test runner failed: {}", e);
            process::exit(1);
        }
    }
}

fn print_help() {
    println!("VexFS Level 2 Test Runner - VM-Isolated Mount Operations");
    println!();
    println!("USAGE:");
    println!("    level2_runner [OPTIONS]");
    println!();
    println!("OPTIONS:");
    println!("    --vm-memory <MB>        Set VM memory in MB (default: 512)");
    println!("    --vm-timeout <seconds>  Set VM operation timeout (default: 300)");
    println!("    --test-disk-size <MB>   Set test disk size in MB (default: 100)");
    println!("    --keep-vm               Don't cleanup VM after tests");
    println!("    --verbose               Enable verbose output");
    println!("    --help, -h              Show this help message");
    println!();
    println!("DESCRIPTION:");
    println!("    Level 2 tests perform VM-isolated mount operations testing.");
    println!("    These tests require QEMU/KVM and perform actual kernel module");
    println!("    loading and filesystem mounting in an isolated virtual machine.");
    println!();
    println!("REQUIREMENTS:");
    println!("    - QEMU/KVM installed and accessible");
    println!("    - Sufficient disk space for VM images");
    println!("    - Root privileges may be required for some operations");
    println!();
    println!("EXAMPLES:");
    println!("    level2_runner");
    println!("    level2_runner --vm-memory 1024 --verbose");
    println!("    level2_runner --keep-vm --test-disk-size 200");
}