// Safe FUSE binary with comprehensive error handling
use std::env;
use std::path::Path;
use fuse;

#[path = "../fuse_impl_safe.rs"]
mod fuse_impl_safe;

#[path = "../fuse_error_handling.rs"]
mod fuse_error_handling;

fn main() {
    println!("VexFS Safe FUSE Implementation v0.0.4-alpha");
    println!("===========================================");
    
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <mountpoint> [options]", args[0]);
        eprintln!("\nOptions:");
        eprintln!("  -f    Run in foreground");
        eprintln!("  -d    Enable debug output");
        eprintln!("\nExample:");
        eprintln!("  {} /mnt/vexfs -f", args[0]);
        std::process::exit(1);
    }
    
    let mountpoint = &args[1];
    let path = Path::new(mountpoint);
    
    if !path.exists() {
        eprintln!("Error: Mount point {} does not exist", mountpoint);
        eprintln!("Please create it first: sudo mkdir -p {}", mountpoint);
        std::process::exit(1);
    }
    
    println!("Mounting VexFS at: {}", mountpoint);
    
    let vexfs = fuse_impl_safe::SafeVexFS::new();
    let mut options = vec!["-o", "fsname=vexfs", "-o", "auto_unmount"];
    
    // Add foreground option if specified
    if args.contains(&"-f".to_string()) {
        println!("Running in foreground mode...");
    } else {
        println!("Running in background mode...");
    }
    
    // Add debug option if specified
    if args.contains(&"-d".to_string()) {
        println!("Debug output enabled");
        options.push("-d");
    }
    
    // Mount the filesystem
    match fuse::mount(vexfs, path, &options) {
        Ok(_) => println!("VexFS unmounted successfully"),
        Err(e) => {
            eprintln!("Failed to mount VexFS: {}", e);
            eprintln!("\nTroubleshooting tips:");
            eprintln!("1. Make sure you have FUSE installed: sudo apt-get install fuse3");
            eprintln!("2. Check if another filesystem is already mounted there: mount | grep {}", mountpoint);
            eprintln!("3. Try unmounting first: sudo umount {}", mountpoint);
            eprintln!("4. Check permissions: you may need to run with sudo");
            std::process::exit(1);
        }
    }
}