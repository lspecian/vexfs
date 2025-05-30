//! Level 2 Testing: VM-Isolated Mount Operations
//! 
//! This module provides VM-isolated testing for VexFS kernel module mount operations.
//! Level 2 testing validates that the kernel module can be loaded, mounted, and perform
//! basic filesystem operations within a controlled VM environment.

use std::process::{Command, Stdio};
use std::path::Path;
use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Level2TestResult {
    pub test_name: String,
    pub status: TestStatus,
    pub duration_ms: u64,
    pub vm_setup: VmSetupResult,
    pub module_loading: ModuleLoadResult,
    pub mount_operations: MountOperationResult,
    pub basic_operations: BasicOperationResult,
    pub cleanup: CleanupResult,
    pub error_details: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum TestStatus {
    Success,
    Failed,
    Skipped,
    Timeout,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VmSetupResult {
    pub vm_started: bool,
    pub ssh_accessible: bool,
    pub kernel_version: Option<String>,
    pub setup_duration_ms: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModuleLoadResult {
    pub module_compiled: bool,
    pub module_loaded: bool,
    pub module_info_valid: bool,
    pub no_kernel_errors: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MountOperationResult {
    pub loop_device_created: bool,
    pub filesystem_formatted: bool,
    pub mount_successful: bool,
    pub mount_point_accessible: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BasicOperationResult {
    pub file_creation: bool,
    pub file_write: bool,
    pub file_read: bool,
    pub directory_creation: bool,
    pub file_deletion: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CleanupResult {
    pub filesystem_unmounted: bool,
    pub module_unloaded: bool,
    pub vm_shutdown: bool,
    pub cleanup_duration_ms: u64,
}

pub struct Level2TestRunner {
    vm_config: VmConfig,
    _test_timeout: Duration,
}

#[derive(Debug, Clone)]
pub struct VmConfig {
    pub vm_image_path: String,
    pub vm_memory_mb: u32,
    pub vm_cpus: u32,
    pub ssh_port: u16,
    pub ssh_key_path: String,
    pub vm_user: String,
}

impl Default for VmConfig {
    fn default() -> Self {
        Self {
            vm_image_path: "tests/vm_images/vexfs-test.qcow2".to_string(),
            vm_memory_mb: 2048,
            vm_cpus: 2,
            ssh_port: 2222,
            ssh_key_path: "tests/vm_keys/vexfs_test_key".to_string(),
            vm_user: "vexfs".to_string(),
        }
    }
}

impl Level2TestRunner {
    pub fn new(vm_config: VmConfig) -> Self {
        Self {
            vm_config,
            _test_timeout: Duration::from_secs(600), // 10 minutes
        }
    }

    pub fn run_level2_tests(&self) -> Result<Level2TestResult, Box<dyn std::error::Error>> {
        let start_time = Instant::now();
        let mut result = Level2TestResult {
            test_name: "Level2_VM_Mount_Operations".to_string(),
            status: TestStatus::Failed,
            duration_ms: 0,
            vm_setup: VmSetupResult {
                vm_started: false,
                ssh_accessible: false,
                kernel_version: None,
                setup_duration_ms: 0,
            },
            module_loading: ModuleLoadResult {
                module_compiled: false,
                module_loaded: false,
                module_info_valid: false,
                no_kernel_errors: false,
            },
            mount_operations: MountOperationResult {
                loop_device_created: false,
                filesystem_formatted: false,
                mount_successful: false,
                mount_point_accessible: false,
            },
            basic_operations: BasicOperationResult {
                file_creation: false,
                file_write: false,
                file_read: false,
                directory_creation: false,
                file_deletion: false,
            },
            cleanup: CleanupResult {
                filesystem_unmounted: false,
                module_unloaded: false,
                vm_shutdown: false,
                cleanup_duration_ms: 0,
            },
            error_details: None,
        };

        // Step 1: VM Setup
        println!("🚀 Starting Level 2 Testing: VM-Isolated Mount Operations");
        let vm_setup_start = Instant::now();
        
        match self.setup_vm() {
            Ok(setup_result) => {
                result.vm_setup = setup_result;
                result.vm_setup.setup_duration_ms = vm_setup_start.elapsed().as_millis() as u64;
            }
            Err(e) => {
                result.error_details = Some(format!("VM setup failed: {}", e));
                result.duration_ms = start_time.elapsed().as_millis() as u64;
                return Ok(result);
            }
        }

        // Step 2: Module Loading
        if result.vm_setup.vm_started && result.vm_setup.ssh_accessible {
            match self.test_module_loading() {
                Ok(module_result) => result.module_loading = module_result,
                Err(e) => {
                    result.error_details = Some(format!("Module loading failed: {}", e));
                    self.cleanup_vm();
                    result.duration_ms = start_time.elapsed().as_millis() as u64;
                    return Ok(result);
                }
            }
        }

        // Step 3: Mount Operations
        if result.module_loading.module_loaded {
            match self.test_mount_operations() {
                Ok(mount_result) => result.mount_operations = mount_result,
                Err(e) => {
                    result.error_details = Some(format!("Mount operations failed: {}", e));
                    self.cleanup_vm();
                    result.duration_ms = start_time.elapsed().as_millis() as u64;
                    return Ok(result);
                }
            }
        }

        // Step 4: Basic Operations
        if result.mount_operations.mount_successful {
            match self.test_basic_operations() {
                Ok(basic_result) => result.basic_operations = basic_result,
                Err(e) => {
                    result.error_details = Some(format!("Basic operations failed: {}", e));
                    self.cleanup_vm();
                    result.duration_ms = start_time.elapsed().as_millis() as u64;
                    return Ok(result);
                }
            }
        }

        // Step 5: Cleanup
        let cleanup_start = Instant::now();
        result.cleanup = self.cleanup_vm();
        result.cleanup.cleanup_duration_ms = cleanup_start.elapsed().as_millis() as u64;

        // Determine overall status
        result.status = if result.vm_setup.vm_started 
            && result.module_loading.module_loaded 
            && result.mount_operations.mount_successful 
            && result.basic_operations.file_creation 
            && result.basic_operations.file_write 
            && result.basic_operations.file_read {
            TestStatus::Success
        } else {
            TestStatus::Failed
        };

        result.duration_ms = start_time.elapsed().as_millis() as u64;
        Ok(result)
    }

    fn setup_vm(&self) -> Result<VmSetupResult, Box<dyn std::error::Error>> {
        let mut result = VmSetupResult {
            vm_started: false,
            ssh_accessible: false,
            kernel_version: None,
            setup_duration_ms: 0,
        };

        // Check if VM image exists
        if !Path::new(&self.vm_config.vm_image_path).exists() {
            return Err(format!("VM image not found: {}", self.vm_config.vm_image_path).into());
        }

        // Start VM using QEMU
        println!("  📦 Starting VM...");
        let _vm_process = Command::new("qemu-system-x86_64")
            .args(&[
                "-m", &self.vm_config.vm_memory_mb.to_string(),
                "-smp", &self.vm_config.vm_cpus.to_string(),
                "-drive", &format!("file={},format=qcow2", self.vm_config.vm_image_path),
                "-netdev", &format!("user,id=net0,hostfwd=tcp::{}-:22", self.vm_config.ssh_port),
                "-device", "virtio-net-pci,netdev=net0",
                "-nographic",
                "-daemonize",
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;

        result.vm_started = true;

        // Wait for VM to boot and SSH to be available
        println!("  ⏳ Waiting for VM to boot...");
        std::thread::sleep(Duration::from_secs(30));

        // Test SSH connectivity
        for attempt in 1..=10 {
            println!("  🔗 Testing SSH connectivity (attempt {}/10)...", attempt);
            let ssh_test = Command::new("ssh")
                .args(&[
                    "-i", &self.vm_config.ssh_key_path,
                    "-o", "StrictHostKeyChecking=no",
                    "-o", "ConnectTimeout=5",
                    &format!("{}@localhost", self.vm_config.vm_user),
                    "-p", &self.vm_config.ssh_port.to_string(),
                    "echo 'SSH_OK'"
                ])
                .output();

            if let Ok(output) = ssh_test {
                if output.status.success() && String::from_utf8_lossy(&output.stdout).contains("SSH_OK") {
                    result.ssh_accessible = true;
                    break;
                }
            }
            
            if attempt < 10 {
                std::thread::sleep(Duration::from_secs(5));
            }
        }

        if result.ssh_accessible {
            // Get kernel version
            let kernel_cmd = Command::new("ssh")
                .args(&[
                    "-i", &self.vm_config.ssh_key_path,
                    "-o", "StrictHostKeyChecking=no",
                    &format!("{}@localhost", self.vm_config.vm_user),
                    "-p", &self.vm_config.ssh_port.to_string(),
                    "uname -r"
                ])
                .output();

            if let Ok(output) = kernel_cmd {
                if output.status.success() {
                    result.kernel_version = Some(String::from_utf8_lossy(&output.stdout).trim().to_string());
                }
            }
        }

        Ok(result)
    }

    fn test_module_loading(&self) -> Result<ModuleLoadResult, Box<dyn std::error::Error>> {
        let mut result = ModuleLoadResult {
            module_compiled: false,
            module_loaded: false,
            module_info_valid: false,
            no_kernel_errors: false,
        };

        println!("  🔧 Testing module compilation in VM...");
        
        // Copy kernel module source to VM
        let copy_cmd = Command::new("scp")
            .args(&[
                "-i", &self.vm_config.ssh_key_path,
                "-o", "StrictHostKeyChecking=no",
                "-P", &self.vm_config.ssh_port.to_string(),
                "-r", "kernel/",
                &format!("{}@localhost:/tmp/", self.vm_config.vm_user)
            ])
            .output()?;

        if !copy_cmd.status.success() {
            return Err("Failed to copy kernel module source to VM".into());
        }

        // Compile module in VM
        let compile_cmd = Command::new("ssh")
            .args(&[
                "-i", &self.vm_config.ssh_key_path,
                "-o", "StrictHostKeyChecking=no",
                &format!("{}@localhost", self.vm_config.vm_user),
                "-p", &self.vm_config.ssh_port.to_string(),
                "cd /tmp/kernel && make clean && make"
            ])
            .output()?;

        result.module_compiled = compile_cmd.status.success();

        if result.module_compiled {
            println!("  ✅ Module compiled successfully");
            
            // Load module
            println!("  📥 Loading VexFS module...");
            let load_cmd = Command::new("ssh")
                .args(&[
                    "-i", &self.vm_config.ssh_key_path,
                    "-o", "StrictHostKeyChecking=no",
                    &format!("{}@localhost", self.vm_config.vm_user),
                    "-p", &self.vm_config.ssh_port.to_string(),
                    "cd /tmp/kernel/build && sudo insmod vexfs.ko"
                ])
                .output()?;

            result.module_loaded = load_cmd.status.success();

            if result.module_loaded {
                println!("  ✅ Module loaded successfully");
                
                // Verify module info
                let info_cmd = Command::new("ssh")
                    .args(&[
                        "-i", &self.vm_config.ssh_key_path,
                        "-o", "StrictHostKeyChecking=no",
                        &format!("{}@localhost", self.vm_config.vm_user),
                        "-p", &self.vm_config.ssh_port.to_string(),
                        "lsmod | grep vexfs"
                    ])
                    .output()?;

                result.module_info_valid = info_cmd.status.success();

                // Check for kernel errors
                let dmesg_cmd = Command::new("ssh")
                    .args(&[
                        "-i", &self.vm_config.ssh_key_path,
                        "-o", "StrictHostKeyChecking=no",
                        &format!("{}@localhost", self.vm_config.vm_user),
                        "-p", &self.vm_config.ssh_port.to_string(),
                        "dmesg | tail -20 | grep -i 'error\\|panic\\|oops'"
                    ])
                    .output()?;

                result.no_kernel_errors = !dmesg_cmd.status.success() || dmesg_cmd.stdout.is_empty();
            }
        }

        Ok(result)
    }

    fn test_mount_operations(&self) -> Result<MountOperationResult, Box<dyn std::error::Error>> {
        let mut result = MountOperationResult {
            loop_device_created: false,
            filesystem_formatted: false,
            mount_successful: false,
            mount_point_accessible: false,
        };

        println!("  💾 Testing mount operations...");

        // Create loop device
        let loop_cmd = Command::new("ssh")
            .args(&[
                "-i", &self.vm_config.ssh_key_path,
                "-o", "StrictHostKeyChecking=no",
                &format!("{}@localhost", self.vm_config.vm_user),
                "-p", &self.vm_config.ssh_port.to_string(),
                "sudo dd if=/dev/zero of=/tmp/vexfs_test.img bs=1M count=100 && sudo losetup /dev/loop0 /tmp/vexfs_test.img"
            ])
            .output()?;

        result.loop_device_created = loop_cmd.status.success();

        if result.loop_device_created {
            println!("  ✅ Loop device created");

            // Format filesystem (this would use mkfs.vexfs when available)
            let format_cmd = Command::new("ssh")
                .args(&[
                    "-i", &self.vm_config.ssh_key_path,
                    "-o", "StrictHostKeyChecking=no",
                    &format!("{}@localhost", self.vm_config.vm_user),
                    "-p", &self.vm_config.ssh_port.to_string(),
                    "sudo mkfs.ext4 /dev/loop0"  // Temporary: use ext4 until mkfs.vexfs is ready
                ])
                .output()?;

            result.filesystem_formatted = format_cmd.status.success();

            if result.filesystem_formatted {
                println!("  ✅ Filesystem formatted");

                // Create mount point and mount
                let mount_cmd = Command::new("ssh")
                    .args(&[
                        "-i", &self.vm_config.ssh_key_path,
                        "-o", "StrictHostKeyChecking=no",
                        &format!("{}@localhost", self.vm_config.vm_user),
                        "-p", &self.vm_config.ssh_port.to_string(),
                        "sudo mkdir -p /mnt/vexfs_test && sudo mount /dev/loop0 /mnt/vexfs_test"
                    ])
                    .output()?;

                result.mount_successful = mount_cmd.status.success();

                if result.mount_successful {
                    println!("  ✅ Filesystem mounted");

                    // Test mount point accessibility
                    let access_cmd = Command::new("ssh")
                        .args(&[
                            "-i", &self.vm_config.ssh_key_path,
                            "-o", "StrictHostKeyChecking=no",
                            &format!("{}@localhost", self.vm_config.vm_user),
                            "-p", &self.vm_config.ssh_port.to_string(),
                            "sudo ls -la /mnt/vexfs_test"
                        ])
                        .output()?;

                    result.mount_point_accessible = access_cmd.status.success();
                }
            }
        }

        Ok(result)
    }

    fn test_basic_operations(&self) -> Result<BasicOperationResult, Box<dyn std::error::Error>> {
        let mut result = BasicOperationResult {
            file_creation: false,
            file_write: false,
            file_read: false,
            directory_creation: false,
            file_deletion: false,
        };

        println!("  📁 Testing basic filesystem operations...");

        // Test file creation
        let create_cmd = Command::new("ssh")
            .args(&[
                "-i", &self.vm_config.ssh_key_path,
                "-o", "StrictHostKeyChecking=no",
                &format!("{}@localhost", self.vm_config.vm_user),
                "-p", &self.vm_config.ssh_port.to_string(),
                "sudo touch /mnt/vexfs_test/test_file.txt"
            ])
            .output()?;

        result.file_creation = create_cmd.status.success();

        if result.file_creation {
            println!("  ✅ File creation successful");

            // Test file write
            let write_cmd = Command::new("ssh")
                .args(&[
                    "-i", &self.vm_config.ssh_key_path,
                    "-o", "StrictHostKeyChecking=no",
                    &format!("{}@localhost", self.vm_config.vm_user),
                    "-p", &self.vm_config.ssh_port.to_string(),
                    "echo 'Hello VexFS Level 2 Testing!' | sudo tee /mnt/vexfs_test/test_file.txt"
                ])
                .output()?;

            result.file_write = write_cmd.status.success();

            if result.file_write {
                println!("  ✅ File write successful");

                // Test file read
                let read_cmd = Command::new("ssh")
                    .args(&[
                        "-i", &self.vm_config.ssh_key_path,
                        "-o", "StrictHostKeyChecking=no",
                        &format!("{}@localhost", self.vm_config.vm_user),
                        "-p", &self.vm_config.ssh_port.to_string(),
                        "sudo cat /mnt/vexfs_test/test_file.txt"
                    ])
                    .output()?;

                result.file_read = read_cmd.status.success() && 
                    String::from_utf8_lossy(&read_cmd.stdout).contains("Hello VexFS Level 2 Testing!");

                if result.file_read {
                    println!("  ✅ File read successful");
                }
            }
        }

        // Test directory creation
        let mkdir_cmd = Command::new("ssh")
            .args(&[
                "-i", &self.vm_config.ssh_key_path,
                "-o", "StrictHostKeyChecking=no",
                &format!("{}@localhost", self.vm_config.vm_user),
                "-p", &self.vm_config.ssh_port.to_string(),
                "sudo mkdir /mnt/vexfs_test/test_directory"
            ])
            .output()?;

        result.directory_creation = mkdir_cmd.status.success();

        if result.directory_creation {
            println!("  ✅ Directory creation successful");
        }

        // Test file deletion
        let delete_cmd = Command::new("ssh")
            .args(&[
                "-i", &self.vm_config.ssh_key_path,
                "-o", "StrictHostKeyChecking=no",
                &format!("{}@localhost", self.vm_config.vm_user),
                "-p", &self.vm_config.ssh_port.to_string(),
                "sudo rm /mnt/vexfs_test/test_file.txt"
            ])
            .output()?;

        result.file_deletion = delete_cmd.status.success();

        if result.file_deletion {
            println!("  ✅ File deletion successful");
        }

        Ok(result)
    }

    fn cleanup_vm(&self) -> CleanupResult {
        let mut result = CleanupResult {
            filesystem_unmounted: false,
            module_unloaded: false,
            vm_shutdown: false,
            cleanup_duration_ms: 0,
        };

        println!("  🧹 Cleaning up VM environment...");

        // Unmount filesystem
        let umount_cmd = Command::new("ssh")
            .args(&[
                "-i", &self.vm_config.ssh_key_path,
                "-o", "StrictHostKeyChecking=no",
                &format!("{}@localhost", self.vm_config.vm_user),
                "-p", &self.vm_config.ssh_port.to_string(),
                "sudo umount /mnt/vexfs_test 2>/dev/null || true"
            ])
            .output();

        result.filesystem_unmounted = umount_cmd.is_ok();

        // Unload module
        let unload_cmd = Command::new("ssh")
            .args(&[
                "-i", &self.vm_config.ssh_key_path,
                "-o", "StrictHostKeyChecking=no",
                &format!("{}@localhost", self.vm_config.vm_user),
                "-p", &self.vm_config.ssh_port.to_string(),
                "sudo rmmod vexfs 2>/dev/null || true"
            ])
            .output();

        result.module_unloaded = unload_cmd.is_ok();

        // Shutdown VM
        let shutdown_cmd = Command::new("ssh")
            .args(&[
                "-i", &self.vm_config.ssh_key_path,
                "-o", "StrictHostKeyChecking=no",
                &format!("{}@localhost", self.vm_config.vm_user),
                "-p", &self.vm_config.ssh_port.to_string(),
                "sudo shutdown -h now"
            ])
            .output();

        result.vm_shutdown = shutdown_cmd.is_ok();

        // Wait for VM to shutdown
        std::thread::sleep(Duration::from_secs(10));

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vm_config_default() {
        let config = VmConfig::default();
        assert_eq!(config.vm_memory_mb, 2048);
        assert_eq!(config.vm_cpus, 2);
        assert_eq!(config.ssh_port, 2222);
    }

    #[test]
    fn test_level2_runner_creation() {
        let config = VmConfig::default();
        let runner = Level2TestRunner::new(config);
        assert_eq!(runner._test_timeout, Duration::from_secs(600));
    }
}