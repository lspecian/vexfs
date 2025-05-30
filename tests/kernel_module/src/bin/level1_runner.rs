//! VexFS Level 1 Kernel Module Test Runner Binary
//! 
//! This binary provides a command-line interface to the Level 1 basic validation
//! tests implemented in level1_basic_validation.rs. It serves as the bridge
//! between the main test orchestration script and the comprehensive Rust test
//! implementation.

use std::env;
use std::path::PathBuf;
use std::process;

// Import the Level 1 test implementation from the library
use vexfs_kernel_tests::{Level1TestRunner, Level1TestConfig, BuildVariant};

fn main() {
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 2 {
        print_usage(&args[0]);
        process::exit(1);
    }
    
    let mut config = Level1TestConfig::default();
    let mut i = 1;
    
    // Parse command line arguments
    while i < args.len() {
        match args[i].as_str() {
            "--build-variant" => {
                if i + 1 >= args.len() {
                    eprintln!("Error: --build-variant requires a value");
                    process::exit(1);
                }
                config.build_variant = parse_build_variant(&args[i + 1]);
                i += 2;
            },
            "--kernel-dir" => {
                if i + 1 >= args.len() {
                    eprintln!("Error: --kernel-dir requires a value");
                    process::exit(1);
                }
                config.vexfs_kernel_dir = PathBuf::from(&args[i + 1]);
                i += 2;
            },
            "--kernel-build-dir" => {
                if i + 1 >= args.len() {
                    eprintln!("Error: --kernel-build-dir requires a value");
                    process::exit(1);
                }
                config.kernel_build_dir = PathBuf::from(&args[i + 1]);
                i += 2;
            },
            "--enable-sudo" => {
                config.enable_sudo_tests = true;
                i += 1;
            },
            "--disable-sudo" => {
                config.enable_sudo_tests = false;
                i += 1;
            },
            "--help" | "-h" => {
                print_usage(&args[0]);
                process::exit(0);
            },
            _ => {
                eprintln!("Error: Unknown argument: {}", args[i]);
                print_usage(&args[0]);
                process::exit(1);
            }
        }
    }
    
    // Validate configuration
    if !config.vexfs_kernel_dir.exists() {
        eprintln!("Error: VexFS kernel directory does not exist: {}", 
                 config.vexfs_kernel_dir.display());
        process::exit(1);
    }
    
    if !config.kernel_build_dir.exists() {
        eprintln!("Error: Kernel build directory does not exist: {}", 
                 config.kernel_build_dir.display());
        process::exit(1);
    }
    
    // Print configuration
    println!("🔧 VexFS Level 1 Test Configuration:");
    println!("   Build variant: {:?}", config.build_variant);
    println!("   VexFS kernel dir: {}", config.vexfs_kernel_dir.display());
    println!("   Kernel build dir: {}", config.kernel_build_dir.display());
    println!("   Sudo tests enabled: {}", config.enable_sudo_tests);
    println!();
    
    // Create and run test runner
    let mut runner = match Level1TestRunner::new(config) {
        Ok(runner) => runner,
        Err(e) => {
            eprintln!("Error: Failed to create test runner: {}", e);
            process::exit(1);
        }
    };
    
    // Run the complete test suite
    match runner.run_complete_test_suite() {
        Ok(()) => {
            println!("\n🎉 Level 1 test suite completed successfully!");
            process::exit(0);
        },
        Err(e) => {
            eprintln!("\n💥 Level 1 test suite failed: {}", e);
            process::exit(1);
        }
    }
}

fn parse_build_variant(variant: &str) -> BuildVariant {
    match variant.to_lowercase().as_str() {
        "standard" => BuildVariant::Standard,
        _ => {
            eprintln!("Error: Invalid build variant: {}. Valid options: standard", variant);
            process::exit(1);
        }
    }
}

fn print_usage(program_name: &str) {
    println!("VexFS Level 1 Kernel Module Test Runner");
    println!();
    println!("USAGE:");
    println!("    {} [OPTIONS]", program_name);
    println!();
    println!("OPTIONS:");
    println!("    --build-variant <VARIANT>    Build variant to test [standard]");
    println!("                                 Default: standard");
    println!("    --kernel-dir <PATH>          Path to VexFS kernel module source directory");
    println!("                                 Default: current directory");
    println!("    --kernel-build-dir <PATH>    Path to kernel build directory");
    println!("                                 Default: /lib/modules/$(uname -r)/build");
    println!("    --enable-sudo                Enable tests requiring sudo privileges");
    println!("    --disable-sudo               Disable tests requiring sudo privileges (default)");
    println!("    --help, -h                   Show this help message");
    println!();
    println!("DESCRIPTION:");
    println!("    This tool runs Level 1 basic validation tests for the VexFS kernel module.");
    println!("    Level 1 tests are HOST-SAFE and focus on module lifecycle operations:");
    println!();
    println!("    Test Cases:");
    println!("      TC1.1 - Module compilation verification");
    println!("      TC1.2 - Module information validation (modinfo)");
    println!("      TC1.3 - Module loading (requires --enable-sudo)");
    println!("      TC1.4 - Module listing verification (lsmod)");
    println!("      TC1.5 - Module unloading (requires --enable-sudo)");
    println!("      TC1.6 - Resource leak detection");
    println!("      TC1.7 - Kernel health check (panic/oops detection)");
    println!();
    println!("    Build Variants:");
    println!("      standard - Standard build with full FFI (production kernel module)");
    println!();
    println!("EXAMPLES:");
    println!("    # Run tests without sudo");
    println!("    {} --build-variant standard", program_name);
    println!();
    println!("    # Run complete test suite with sudo");
    println!("    {} --build-variant standard --enable-sudo", program_name);
    println!();
    println!("    # Test with custom paths");
    println!("    {} --build-variant standard \\", program_name);
    println!("                     --kernel-dir /path/to/vexfs \\");
    println!("                     --enable-sudo");
    println!();
    println!("SAFETY:");
    println!("    Level 1 tests are designed to be HOST-SAFE. They do not perform");
    println!("    dangerous mount operations that could crash the system. However,");
    println!("    tests requiring sudo (TC1.3-TC1.5) do load/unload kernel modules,");
    println!("    which carries some risk.");
    println!();
    println!("    For maximum safety, run without --enable-sudo to test only");
    println!("    compilation and static analysis (TC1.1-TC1.2, TC1.6-TC1.7).");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_build_variant() {
        assert_eq!(parse_build_variant("standard"), BuildVariant::Standard);
        assert_eq!(parse_build_variant("STANDARD"), BuildVariant::Standard);
    }
}