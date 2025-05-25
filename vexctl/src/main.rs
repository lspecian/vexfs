use clap::{Parser, Subcommand};
use std::fs::File;
use std::os::unix::io::AsRawFd;
// We will use raw ioctl call for simplicity with a predefined command number.
// use nix::ioctl_read_bad; 

// Define the IOCTL command number components, matching the kernel module's ioctl.rs
const VEXFS_IOCTL_MAGIC: u8 = b'v';
const VEXFS_IOCTL_CMD_GET_STATUS: u8 = 0x01;

// Pre-calculate the full IOCTL command number based on common Linux _IO macro behavior.
// _IO(type, nr) -> ( (IOC_NONE << DIRSHIFT) | (type << TYPESHIFT) | (nr << NRSHIFT) | (0 << SIZESHIFT) )
// For VEXFS_IOCTL_MAGIC = 'v' (0x76), VEXFS_IOCTL_CMD_GET_STATUS = 1:
// Assuming IOC_NONE = 0, no size.
// Typical bit shifts (can vary by arch, but often consistent for _IO):
// TYPE is 8 bits, NR is 8 bits.
// Command = (TYPE << 8) | NR
// So, (0x76 << 8) | 0x01 = 0x7600 | 0x01 = 0x7601
// If direction and size are involved at higher bits, they are 0 for _IO.
// So, the number is often directly 0x00007601 (assuming 32-bit command).
// This matches the placeholder `VEXFS_GET_STATUS_CMD_FULL` in the kernel module.
const VEXFS_IOCTL_GET_STATUS_FULL_CMD: u32 = 0x00007601;


#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Get status of a mounted VexFS filesystem
    Status {
        /// Path to the mounted VexFS filesystem (e.g., /mnt/vexfs)
        #[clap(value_parser)]
        path: String,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Status { path } => {
            println!("Attempting to get status for VexFS mounted at: {}", path);

            let file = match File::open(path) {
                Ok(f) => f,
                Err(e) => {
                    eprintln!("Error opening '{}': {}", path, e);
                    return Err(e.into());
                }
            };

            let fd = file.as_raw_fd();
            
            // Directly use the libc::ioctl call via nix::sys::ioctl::ioctl
            // The third argument to ioctl can be an int or a pointer. For _IO commands, it's often ignored (or 0).
            // The return value of the ioctl syscall itself is what we're interested in.
            match unsafe { nix::sys::ioctl::ioctl(fd, VEXFS_IOCTL_GET_STATUS_FULL_CMD as u64, 0 as *mut _) } {
                Ok(status_code) => {
                    // status_code is the direct integer return from the ioctl syscall.
                    println!("VexFS status for '{}': {}", path, status_code);
                    if status_code == 12345 {
                        println!("Status interpretation: OK (Magic number 12345 received from kernel)");
                    } else if status_code < 0 {
                        // This should ideally not happen if the kernel module returns a positive status.
                        // Negative values from ioctl syscall usually mean an error in the syscall itself.
                        eprintln!("Received negative status code {}, which typically indicates an ioctl syscall error.", status_code);
                    } else {
                        println!("Received status code: {}. Expected 12345 for OK.", status_code);
                    }
                }
                Err(e) => {
                    // This Err(e) is from the ioctl syscall failing (e.g., fd not open, device not supporting the ioctl, etc.)
                    eprintln!("Error calling VEXFS_IOCTL_GET_STATUS on '{}': {}", path, e);
                    // e is nix::Error, which can be converted to std::io::Error
                    let std_io_error: std::io::Error = e.into();
                    return Err(std_io_error.into());
                }
            }
        }
    }

    Ok(())
}
