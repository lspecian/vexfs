//! Debug version of Level 1 test to understand the compilation failure

use std::process::Command;
use std::path::PathBuf;

fn main() {
    println!("🔍 Debug Level 1 Test - Investigating compilation failure");
    
    let vexfs_kernel_dir = PathBuf::from("/home/luis/Development/oss/vexfs");
    let build_dir = vexfs_kernel_dir.join("kernel/build");
    
    println!("📁 VexFS kernel dir: {}", vexfs_kernel_dir.display());
    println!("📁 Build dir: {}", build_dir.display());
    
    // Check if build directory exists
    if !build_dir.exists() {
        println!("❌ Build directory does not exist!");
        return;
    }
    
    println!("✅ Build directory exists");
    
    // Check if Makefile exists
    let makefile = build_dir.join("Makefile");
    if !makefile.exists() {
        println!("❌ Makefile does not exist!");
        return;
    }
    
    println!("✅ Makefile exists");
    
    // Test 1: Run make clean
    println!("\n🧹 Running make clean...");
    let clean_output = Command::new("make")
        .arg("-C")
        .arg(&build_dir)
        .arg("clean")
        .output();
        
    match clean_output {
        Ok(output) => {
            println!("Clean exit code: {:?}", output.status.code());
            println!("Clean stdout: {}", String::from_utf8_lossy(&output.stdout));
            println!("Clean stderr: {}", String::from_utf8_lossy(&output.stderr));
        }
        Err(e) => {
            println!("❌ Failed to run make clean: {}", e);
            return;
        }
    }
    
    // Test 2: Run make safe-build
    println!("\n🔨 Running make safe-build...");
    let build_output = Command::new("make")
        .arg("-C")
        .arg(&build_dir)
        .arg("safe-build")
        .output();
        
    match build_output {
        Ok(output) => {
            println!("Build exit code: {:?}", output.status.code());
            println!("Build success: {}", output.status.success());
            println!("Build stdout: {}", String::from_utf8_lossy(&output.stdout));
            println!("Build stderr: {}", String::from_utf8_lossy(&output.stderr));
            
            // Check if module file was created
            let module_path = build_dir.join("vexfs_safe.ko");
            if module_path.exists() {
                println!("✅ Module file created: {}", module_path.display());
                
                // Get file size
                if let Ok(metadata) = std::fs::metadata(&module_path) {
                    println!("   Module size: {} bytes", metadata.len());
                }
            } else {
                println!("❌ Module file NOT created: {}", module_path.display());
                
                // List all .ko files in build directory
                println!("📋 Listing all .ko files in build directory:");
                if let Ok(entries) = std::fs::read_dir(&build_dir) {
                    for entry in entries {
                        if let Ok(entry) = entry {
                            let path = entry.path();
                            if path.extension().and_then(|s| s.to_str()) == Some("ko") {
                                println!("   Found: {}", path.display());
                            }
                        }
                    }
                }
            }
        }
        Err(e) => {
            println!("❌ Failed to run make safe-build: {}", e);
        }
    }
}