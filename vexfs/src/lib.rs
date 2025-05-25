//! VexFS - Vector Embedding Filesystem
//!
//! This library provides both kernel module functionality (when compiled with "kernel" feature)
//! and userspace testing functionality (default).

// Conditional no_std for kernel only
#![cfg_attr(feature = "kernel", no_std)]

// Enable alloc for kernel compilation
#[cfg(feature = "kernel")]
extern crate alloc;

// Use std for userspace testing (simpler for now)
#[cfg(not(feature = "kernel"))]
extern crate std;
#[cfg(not(feature = "kernel"))]
use std::prelude::*;

// Conditional compilation for kernel vs userspace
#[cfg(feature = "kernel")]
mod kernel_module {
    use kernel::module;
    use kernel::prelude::*;
    use kernel::{KernelResult, ThisModule, Error};
    use kernel::super_block::{SuperBlock, FileSystemType, MountData, Dentry};
    use kernel::bindings;

    // Import the superblock module where VEXFS_SUPER_OPS is now defined.
    use crate::superblock;

    // The FileSystemType structure for VexFS
    struct VexfsFsType;

    // Implementation of the FileSystemType trait for VexfsFsType
    impl kernel::super_block::FileSystemType for VexfsFsType {
        const NAME: &'static str = "vexfs";
        const REQUIRES_DEV: bool = true;

        fn mount(
            &'static self,
            flags: u32,
            dev_name: &kernel::cstr::CStr,
            data: Option<&kernel::cstr::CStr>,
        ) -> Result<Dentry> {
            pr_info!("VexFS: VexfsFsType::mount called for device: {:?}, flags: {}\n", dev_name, flags);
            
            SuperBlock::mount_bdev(self, flags, dev_name, move |sb, mount_data| {
                pr_info!("VexFS: fill_super (closure via mount_bdev) called for dev: {:?}\n", dev_name);

                let silent_c_int = if (flags & bindings::MS_SILENT) != 0 { 1 } else { 0 };
                
                match superblock::vexfs_fill_super(sb, mount_data.data(), silent_c_int) {
                    Ok(()) => {
                        pr_info!("VexFS: fill_super closure completed successfully.\n");
                        Ok(())
                    }
                    Err(e) => {
                        pr_err!("VexFS: vexfs_fill_super (called from closure) failed: {:?}\n", e);
                        Err(e)
                    }
                }
            })
        }

        fn kill_sb(&'static self, sb: &mut SuperBlock) {
            pr_info!("VexFS: VexfsFsType::kill_sb called (from lib.rs)\n");
            superblock::vexfs_kill_sb(sb);
        }
    }

    static VEXFS_FS_TYPE: VexfsFsType = VexfsFsType;
    static mut FS_TYPE_REGISTRATION: Option<kernel::fs::RegisteredFileSystem> = None;

    module! {
        type: VexFS,
        name: b"vexfs",
        author: b"AI Agent (for VexFS)",
        description: b"VDBHAX/VexFS: Vector-Native File System",
        license: b"GPL",
    }

    struct VexFS {}

    impl kernel::Module for VexFS {
        fn init(_module: &'static ThisModule) -> KernelResult<Self> {
            pr_info!("VexFS: kernel::Module::init() called. Module is loading.\n");

            match kernel::fs::register_filesystem(&VEXFS_FS_TYPE) {
                Ok(registration) => {
                    pr_info!("VexFS: Filesystem registered successfully with kernel.\n");
                    unsafe {
                        FS_TYPE_REGISTRATION = Some(registration);
                    }
                }
                Err(e) => {
                    pr_err!("VexFS: Failed to register filesystem with kernel: {:?}\n", e);
                    return Err(e);
                }
            }
            Ok(VexFS {})
        }

        fn exit(&mut self, _module: &'static ThisModule) {
            pr_info!("VexFS: kernel::Module::exit() called. Module is unloading.\n");

            unsafe {
                if let Some(registration) = FS_TYPE_REGISTRATION.take() {
                    match kernel::fs::unregister_filesystem(registration) {
                        Ok(()) => pr_info!("VexFS: Filesystem unregistered successfully from kernel.\n"),
                        Err(e) => pr_err!("VexFS: Failed to unregister filesystem: {:?}\n", e),
                    }
                } else {
                    pr_warn!("VexFS: Filesystem registration handle not found, cannot unregister.\n");
                }
            }
        }
    }

    #[no_mangle]
    pub extern "C" fn vexfs_rust_init() -> core::ffi::c_int {
        pr_info!("VexFS: vexfs_rust_init() (extern \"C\") called from C shim.\n");
        0
    }

    #[no_mangle]
    pub extern "C" fn vexfs_rust_exit() {
        pr_info!("VexFS: vexfs_rust_exit() (extern \"C\") called from C shim.\n");
    }
}

// Module declarations (only include modules that compile in userspace)
#[cfg(feature = "kernel")]
mod superblock;
#[cfg(feature = "kernel")]
mod inode;
#[cfg(feature = "kernel")]
pub mod ioctl;

// Core modules that should work in both kernel and userspace
pub mod ondisk;
pub mod vector_storage;
pub mod vector_metrics;
pub mod knn_search;
pub mod result_scoring;
pub mod vector_search;
pub mod anns;

// Conditional vector_handlers - real for kernel, stub for userspace
#[cfg(feature = "kernel")]
pub mod vector_handlers;
#[cfg(not(feature = "kernel"))]
#[path = "vector_handlers_stub.rs"]
pub mod vector_handlers;

// Modules that need conditional compilation
#[cfg(feature = "kernel")]
pub mod inode_mgmt;
#[cfg(feature = "kernel")]
pub mod file_ops;
#[cfg(feature = "kernel")]
pub mod dir_ops;
#[cfg(feature = "kernel")]
pub mod space_alloc;
#[cfg(feature = "kernel")]
pub mod journal;

// Userspace-only modules
#[cfg(not(feature = "kernel"))]
pub mod vector_test;

// Userspace API for testing
#[cfg(not(feature = "kernel"))]
pub fn init_vexfs_userspace() -> Result<(), String> {
    println!("VexFS: Initializing in userspace mode");
    Ok(())
}

#[cfg(not(feature = "kernel"))]
pub fn test_vector_operations() -> Result<(), String> {
    println!("VexFS: Running vector operation tests");
    // Basic vector operations test
    Ok(())
}
