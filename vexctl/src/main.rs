/*
 * VexFS Control Tool (vexctl)
 * Copyright 2025 VexFS Contributors
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

use clap::{Parser, Subcommand};
use std::fs::File;
use std::os::unix::io::AsRawFd;
use nix::ioctl_none;

// Define the IOCTL command number components, matching the kernel module's ioctl.rs
const VEXFS_IOCTL_MAGIC: u8 = b'V'; // Uppercase V to match kernel module
const VEXFS_IOCTL_CMD_GET_STATUS: u8 = 0x10; // Match kernel module definition

// Use nix's ioctl macro to properly define the ioctl command
ioctl_none!(vexfs_get_status, VEXFS_IOCTL_MAGIC, VEXFS_IOCTL_CMD_GET_STATUS);


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
            
            // Use the properly defined ioctl function from nix
            match unsafe { vexfs_get_status(fd) } {
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
