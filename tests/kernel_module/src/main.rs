//! VexFS Kernel Module Test Runner
//! 
//! This binary runs the Level 1 Basic Kernel Module Validation tests
//! against the REAL production VexFS kernel module.

use std::path::PathBuf;

mod level1_basic_validation;

use level1_basic_validation::{Level1TestRunner, Level1TestConfig, BuildVariant};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 VexFS Kernel Module Test Runner");
    println!("===================================");
    
    // Configure test to use REAL production kernel module
    let config = Level1TestConfig {
        kernel_build_dir: PathBuf::from("/lib/modules")
            .join(get_kernel_version()?)
            .join("build"),
        vexfs_kernel_dir: PathBuf::from("../../"), // Relative to tests/kernel_module/
        enable_sudo_tests: false, // Start with compilation tests only
        build_variant: BuildVariant::Standard, // REAL production module
    };
    
    println!("Configuration:");
    println!("  Kernel build dir: {}", config.kernel_build_dir.display());
    println!("  VexFS kernel dir: {}", config.vexfs_kernel_dir.display());
    println!("  Build variant: {:?} (REAL production module)", config.build_variant);
    println!("  Sudo tests: {}", config.enable_sudo_tests);
    println!();
    
    // Create and run test runner
    let mut runner = Level1TestRunner::new(config)?;
    
    // Run the complete test suite
    runner.run_complete_test_suite()?;
    
    println!("\n🎉 Test run completed!");
    
    Ok(())
}

fn get_kernel_version() -> Result<String, Box<dyn std::error::Error>> {
    let output = std::process::Command::new("uname")
        .arg("-r")
        .output()?;
        
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err("Failed to get kernel version".into())
    }
}