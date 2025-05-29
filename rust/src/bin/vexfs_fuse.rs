use clap::{Arg, Command};
use fuse::mount;
use std::path::Path;

use vexfs::fuse_impl::VexFSFuse;

fn main() {
    let matches = Command::new("vexfs-fuse")
        .version("1.0.0")
        .about("VexFS FUSE filesystem for development and testing")
        .arg(
            Arg::new("mountpoint")
                .help("Directory to mount VexFS")
                .required(true)
                .index(1),
        )
        .arg(
            Arg::new("foreground")
                .short('f')
                .long("foreground")
                .help("Run in foreground")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("debug")
                .short('d')
                .long("debug")
                .help("Enable debug output")
                .action(clap::ArgAction::SetTrue),
        )
        .get_matches();

    let mountpoint = matches.get_one::<String>("mountpoint").unwrap();
    let foreground = matches.get_flag("foreground");
    let debug = matches.get_flag("debug");

    // Validate mountpoint
    if !Path::new(mountpoint).exists() {
        eprintln!("Error: Mount point '{}' does not exist", mountpoint);
        std::process::exit(1);
    }

    if !Path::new(mountpoint).is_dir() {
        eprintln!("Error: Mount point '{}' is not a directory", mountpoint);
        std::process::exit(1);
    }

    // Create VexFS FUSE filesystem
    let filesystem = match VexFSFuse::new() {
        Ok(fs) => fs,
        Err(e) => {
            eprintln!("Error creating VexFS: {}", e);
            std::process::exit(1);
        }
    };

    println!("🚀 Starting VexFS FUSE filesystem");
    println!("📁 Mount point: {}", mountpoint);
    println!("🔧 Debug mode: {}", if debug { "enabled" } else { "disabled" });
    println!("🖥️  Foreground: {}", if foreground { "yes" } else { "no" });
    println!("");
    println!("💡 Usage examples:");
    println!("   echo '0.1,0.2,0.3' > {}/query.vec", mountpoint);
    println!("   echo 'Hello world' > {}/document.txt", mountpoint);
    println!("   ls -la {}/", mountpoint);
    println!("");
    println!("🛑 To unmount: fusermount -u {}", mountpoint);
    println!("");

    // Set up mount options
    let mut options = vec![
        "-o", "rw",
        "-o", "fsname=vexfs",
        "-o", "subtype=vexfs",
        "-o", "allow_other",
    ];

    if foreground {
        options.push("-f");
    }

    if debug {
        options.push("-d");
    }

    // Mount the filesystem
    match mount(filesystem, mountpoint, &options) {
        Ok(()) => {
            println!("✅ VexFS mounted successfully at {}", mountpoint);
        }
        Err(e) => {
            eprintln!("❌ Failed to mount VexFS: {}", e);
            eprintln!("");
            eprintln!("💡 Troubleshooting:");
            eprintln!("   - Make sure FUSE is installed: sudo apt-get install fuse");
            eprintln!("   - Check if you have permission to mount: groups | grep fuse");
            eprintln!("   - Try running with sudo if needed");
            eprintln!("   - Make sure the mount point is empty");
            std::process::exit(1);
        }
    }
}