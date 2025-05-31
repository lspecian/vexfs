use std::process::Command;
use std::path::PathBuf;
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Debug: Make Command Execution Analysis");
    
    // Show current working directory
    let current_dir = env::current_dir()?;
    println!("Current working directory: {}", current_dir.display());
    
    // Show environment variables that might affect make
    println!("\n📋 Environment Variables:");
    for (key, value) in env::vars() {
        if key.contains("PATH") || key.contains("MAKE") || key.contains("CC") || key.contains("KERNEL") {
            println!("  {}: {}", key, value);
        }
    }
    
    // Test the exact path resolution
    let vexfs_kernel_dir = PathBuf::from("/home/luis/Development/oss/vexfs");
    let kernel_dir = vexfs_kernel_dir.join("kernel");
    println!("\n📁 Path Resolution:");
    println!("  VexFS kernel dir: {}", vexfs_kernel_dir.display());
    println!("  Kernel dir: {}", kernel_dir.display());
    println!("  Kernel dir exists: {}", kernel_dir.exists());
    println!("  Makefile exists: {}", kernel_dir.join("Makefile").exists());
    
    // Test which make command is being used
    println!("\n🔧 Make Command Location:");
    let which_make = Command::new("which")
        .arg("make")
        .output()?;
    
    if which_make.status.success() {
        let make_path = String::from_utf8_lossy(&which_make.stdout);
        println!("  Make command found at: {}", make_path.trim());
    } else {
        println!("  ❌ Make command not found in PATH");
    }
    
    // Test the exact command that's failing
    println!("\n🧪 Testing Make Command Execution:");
    println!("  Command: make -C {} clean", kernel_dir.display());
    
    let output = Command::new("make")
        .arg("-C")
        .arg(&kernel_dir)
        .arg("clean")
        .output()?;
    
    println!("  Exit status: {}", output.status);
    println!("  Success: {}", output.status.success());
    
    if !output.stdout.is_empty() {
        println!("  Stdout:");
        for line in String::from_utf8_lossy(&output.stdout).lines() {
            println!("    {}", line);
        }
    }
    
    if !output.stderr.is_empty() {
        println!("  Stderr:");
        for line in String::from_utf8_lossy(&output.stderr).lines() {
            println!("    {}", line);
        }
    }
    
    // Test with explicit environment
    println!("\n🔄 Testing with Explicit Environment:");
    let mut cmd = Command::new("make");
    cmd.arg("-C")
       .arg(&kernel_dir)
       .arg("clean");
    
    // Ensure PATH is set
    if let Ok(path) = env::var("PATH") {
        cmd.env("PATH", path);
    }
    
    let output2 = cmd.output()?;
    println!("  Exit status: {}", output2.status);
    println!("  Success: {}", output2.status.success());
    
    if !output2.stderr.is_empty() {
        println!("  Stderr:");
        for line in String::from_utf8_lossy(&output2.stderr).lines() {
            println!("    {}", line);
        }
    }
    
    Ok(())
}