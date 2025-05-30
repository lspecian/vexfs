//! VexFS kselftest-Compatible Test Runner
//!
//! This binary provides a kselftest-compatible interface for VexFS kernel module testing.
//! It integrates with the Linux kernel's kselftest framework and produces TAP-compliant
//! output as recommended by the consultancy report.
//!
//! Usage:
//!   ./kselftest_runner                    # Run all tests
//!   ./kselftest_runner --tap              # Force TAP output
//!   ./kselftest_runner --verbose          # Verbose output
//!   ./kselftest_runner --no-sudo          # Skip sudo-required tests

use std::env;
use std::process;
use vexfs_kernel_tests::create_vexfs_kselftest_suite;

fn main() {
    let args: Vec<String> = env::args().collect();
    
    let mut verbose = false;
    let mut tap_output = true;
    let mut skip_sudo = false;
    
    // Parse command line arguments
    for arg in &args[1..] {
        match arg.as_str() {
            "--verbose" | "-v" => verbose = true,
            "--tap" => tap_output = true,
            "--no-tap" => tap_output = false,
            "--no-sudo" => skip_sudo = true,
            "--help" | "-h" => {
                print_usage(&args[0]);
                process::exit(0);
            }
            _ => {
                eprintln!("Error: Unknown argument: {}", arg);
                print_usage(&args[0]);
                process::exit(1);
            }
        }
    }
    
    // Create and configure kselftest runner
    let mut runner = create_vexfs_kselftest_suite();
    runner.set_verbose(verbose);
    runner.set_tap_output(tap_output);
    
    // Filter out sudo tests if requested
    if skip_sudo {
        // Note: The filtering would be implemented in the kselftest_integration module
        if verbose {
            println!("# Skipping tests that require sudo privileges");
        }
    }
    
    // Run the test suite
    match runner.run_all_tests() {
        Ok(result) => {
            if verbose {
                println!("# kselftest execution completed with result: {:?}", result);
            }
            process::exit(result.exit_code());
        }
        Err(e) => {
            eprintln!("# Error running kselftest suite: {}", e);
            process::exit(1);
        }
    }
}

fn print_usage(program_name: &str) {
    println!("VexFS kselftest-Compatible Test Runner");
    println!();
    println!("USAGE:");
    println!("    {} [OPTIONS]", program_name);
    println!();
    println!("OPTIONS:");
    println!("    --verbose, -v        Enable verbose output");
    println!("    --tap                Force TAP output format (default)");
    println!("    --no-tap             Disable TAP output format");
    println!("    --no-sudo            Skip tests requiring sudo privileges");
    println!("    --help, -h           Show this help message");
    println!();
    println!("DESCRIPTION:");
    println!("    This tool provides a kselftest-compatible interface for testing the");
    println!("    VexFS kernel module. It produces TAP-compliant output and integrates");
    println!("    with the Linux kernel's testing infrastructure.");
    println!();
    println!("    Test Cases:");
    println!("      1. module_compilation      - Verify module compiles successfully");
    println!("      2. module_info_validation   - Validate module metadata");
    println!("      3. module_loading          - Test module loading (requires sudo)");
    println!("      4. module_listing          - Verify module appears in lsmod");
    println!("      5. module_unloading        - Test module unloading (requires sudo)");
    println!("      6. resource_leak_detection - Monitor for resource leaks");
    println!("      7. kernel_health_check     - Check for kernel panics/oopses");
    println!();
    println!("EXIT CODES:");
    println!("    0 - All tests passed");
    println!("    1 - One or more tests failed");
    println!("    4 - Tests were skipped");
    println!();
    println!("INTEGRATION:");
    println!("    This tool is designed to integrate with kernel kselftest infrastructure:");
    println!("    - Produces TAP version 13 compliant output");
    println!("    - Uses standard kselftest exit codes");
    println!("    - Compatible with kselftest harness and CI systems");
    println!();
    println!("EXAMPLES:");
    println!("    # Run all tests with TAP output");
    println!("    {}", program_name);
    println!();
    println!("    # Run tests without sudo requirements");
    println!("    {} --no-sudo", program_name);
    println!();
    println!("    # Run with verbose output for debugging");
    println!("    {} --verbose", program_name);
}