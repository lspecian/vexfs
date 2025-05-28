/*
 * VexFS Connection Management
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

//! VexFS connection management

use crate::{Result, VexctlError};
use std::fs::File;
use std::os::unix::io::{AsRawFd, RawFd};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Configuration for VexFS connections
#[derive(Debug, Clone)]
pub struct ConnectionConfig {
    /// Timeout for operations
    pub timeout: Duration,
    /// Enable verbose logging
    pub verbose: bool,
    /// Buffer size for operations
    pub buffer_size: usize,
    /// Maximum retry attempts
    pub max_retries: u32,
    /// Retry delay
    pub retry_delay: Duration,
}

impl Default for ConnectionConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(30),
            verbose: false,
            buffer_size: 64 * 1024, // 64KB
            max_retries: 3,
            retry_delay: Duration::from_millis(100),
        }
    }
}

/// VexFS filesystem connection
pub struct VexfsConnection {
    /// Path to the mounted filesystem
    mount_path: PathBuf,
    /// File handle to the filesystem
    file: File,
    /// Raw file descriptor
    fd: RawFd,
    /// Connection configuration
    config: ConnectionConfig,
}

impl VexfsConnection {
    /// Connect to a VexFS filesystem at the given path
    pub fn connect<P: AsRef<Path>>(path: P) -> Result<Self> {
        Self::connect_with_config(path, ConnectionConfig::default())
    }

    /// Connect to a VexFS filesystem with custom configuration
    pub fn connect_with_config<P: AsRef<Path>>(
        path: P,
        config: ConnectionConfig,
    ) -> Result<Self> {
        let mount_path = path.as_ref().to_path_buf();
        
        // Verify the path exists
        if !mount_path.exists() {
            return Err(VexctlError::FilesystemNotFound {
                path: mount_path.display().to_string(),
            });
        }

        // Open the filesystem root directory
        let file = File::open(&mount_path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::PermissionDenied {
                VexctlError::PermissionDenied {
                    operation: format!("open filesystem at {}", mount_path.display()),
                }
            } else {
                VexctlError::Io(e)
            }
        })?;

        let fd = file.as_raw_fd();

        // Verify this is actually a VexFS filesystem by attempting a basic status call
        let connection = Self {
            mount_path,
            file,
            fd,
            config,
        };

        // Test the connection
        connection.verify_filesystem()?;

        Ok(connection)
    }

    /// Get the mount path
    pub fn mount_path(&self) -> &Path {
        &self.mount_path
    }

    /// Get the raw file descriptor
    pub fn fd(&self) -> RawFd {
        self.fd
    }

    /// Get the connection configuration
    pub fn config(&self) -> &ConnectionConfig {
        &self.config
    }

    /// Check if the filesystem is accessible
    pub fn is_accessible(&self) -> bool {
        self.mount_path.exists() && self.mount_path.is_dir()
    }

    /// Get filesystem information
    pub fn filesystem_info(&self) -> Result<FilesystemInfo> {
        use std::fs;

        let metadata = fs::metadata(&self.mount_path)?;
        
        // Try to read some basic filesystem statistics
        let statvfs = self.get_statvfs()?;

        Ok(FilesystemInfo {
            mount_path: self.mount_path.clone(),
            total_space: statvfs.total_space,
            free_space: statvfs.free_space,
            used_space: statvfs.used_space,
            total_inodes: statvfs.total_inodes,
            free_inodes: statvfs.free_inodes,
            block_size: statvfs.block_size,
            is_read_only: statvfs.is_read_only,
            filesystem_type: "vexfs".to_string(),
        })
    }

    /// Verify this is a VexFS filesystem
    fn verify_filesystem(&self) -> Result<()> {
        // Try to perform a basic IOCTL operation to verify VexFS
        use crate::client::ioctl::IoctlClient;
        
        let ioctl_client = IoctlClient::new(self);
        match ioctl_client.get_status() {
            Ok(_) => Ok(()),
            Err(VexctlError::Ioctl(nix::Error::ENOTTY)) => {
                Err(VexctlError::InvalidFilesystem {
                    path: self.mount_path.display().to_string(),
                })
            }
            Err(e) => Err(e),
        }
    }

    /// Get filesystem statistics using statvfs
    fn get_statvfs(&self) -> Result<StatVfs> {
        use nix::sys::statvfs::statvfs;

        let statvfs = statvfs(&self.mount_path)?;
        
        let block_size = statvfs.block_size() as u64;
        let total_blocks = statvfs.blocks();
        let free_blocks = statvfs.blocks_free();
        let available_blocks = statvfs.blocks_available();

        Ok(StatVfs {
            total_space: total_blocks * block_size,
            free_space: free_blocks * block_size,
            used_space: (total_blocks - free_blocks) * block_size,
            available_space: available_blocks * block_size,
            total_inodes: statvfs.files(),
            free_inodes: statvfs.files_free(),
            block_size: block_size as u32,
            is_read_only: statvfs.flags().contains(nix::sys::statvfs::FsFlags::ST_RDONLY),
        })
    }

    /// Execute an operation with retry logic
    pub fn with_retry<F, T>(&self, operation: F) -> Result<T>
    where
        F: Fn() -> Result<T>,
    {
        let mut last_error = None;
        
        for attempt in 0..=self.config.max_retries {
            match operation() {
                Ok(result) => return Ok(result),
                Err(e) => {
                    if !e.is_recoverable() || attempt == self.config.max_retries {
                        return Err(e);
                    }
                    
                    if self.config.verbose {
                        eprintln!("Attempt {} failed: {}, retrying...", attempt + 1, e);
                    }
                    
                    last_error = Some(e);
                    std::thread::sleep(self.config.retry_delay);
                }
            }
        }
        
        Err(last_error.unwrap_or_else(|| VexctlError::internal("Retry loop failed")))
    }
}

/// Filesystem information
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FilesystemInfo {
    pub mount_path: PathBuf,
    pub total_space: u64,
    pub free_space: u64,
    pub used_space: u64,
    pub total_inodes: u64,
    pub free_inodes: u64,
    pub block_size: u32,
    pub is_read_only: bool,
    pub filesystem_type: String,
}

impl FilesystemInfo {
    /// Get space utilization percentage
    pub fn space_utilization(&self) -> f64 {
        if self.total_space == 0 {
            0.0
        } else {
            (self.used_space as f64 / self.total_space as f64) * 100.0
        }
    }

    /// Get inode utilization percentage
    pub fn inode_utilization(&self) -> f64 {
        if self.total_inodes == 0 {
            0.0
        } else {
            let used_inodes = self.total_inodes - self.free_inodes;
            (used_inodes as f64 / self.total_inodes as f64) * 100.0
        }
    }
}

/// Internal statvfs wrapper
struct StatVfs {
    pub total_space: u64,
    pub free_space: u64,
    pub used_space: u64,
    pub available_space: u64,
    pub total_inodes: u64,
    pub free_inodes: u64,
    pub block_size: u32,
    pub is_read_only: bool,
}

/// Helper function to find VexFS mount points
pub fn find_vexfs_mounts() -> Result<Vec<PathBuf>> {
    use std::fs;
    use std::io::{BufRead, BufReader};

    let mut mounts = Vec::new();
    
    // Read /proc/mounts to find VexFS filesystems
    let mounts_file = fs::File::open("/proc/mounts")?;
    let reader = BufReader::new(mounts_file);
    
    for line in reader.lines() {
        let line = line?;
        let parts: Vec<&str> = line.split_whitespace().collect();
        
        if parts.len() >= 3 && parts[2] == "vexfs" {
            mounts.push(PathBuf::from(parts[1]));
        }
    }
    
    Ok(mounts)
}

/// Helper function to check if a path is a VexFS mount
pub fn is_vexfs_mount<P: AsRef<Path>>(path: P) -> bool {
    VexfsConnection::connect(path).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_connection_config_default() {
        let config = ConnectionConfig::default();
        assert_eq!(config.timeout, Duration::from_secs(30));
        assert!(!config.verbose);
        assert_eq!(config.buffer_size, 64 * 1024);
        assert_eq!(config.max_retries, 3);
    }

    #[test]
    fn test_filesystem_info_utilization() {
        let info = FilesystemInfo {
            mount_path: PathBuf::from("/test"),
            total_space: 1000,
            free_space: 300,
            used_space: 700,
            total_inodes: 100,
            free_inodes: 20,
            block_size: 4096,
            is_read_only: false,
            filesystem_type: "vexfs".to_string(),
        };

        assert_eq!(info.space_utilization(), 70.0);
        assert_eq!(info.inode_utilization(), 80.0);
    }

    #[test]
    fn test_find_vexfs_mounts() {
        // This test will only work on systems with /proc/mounts
        if Path::new("/proc/mounts").exists() {
            let result = find_vexfs_mounts();
            assert!(result.is_ok());
        }
    }
}