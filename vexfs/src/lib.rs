use kernel::module;
use kernel::prelude::*;
use kernel::{KernelResult, ThisModule, Error};
// kernel::sync::Arc is not used with the current static registration approach.
// use kernel::sync::Arc; 
use kernel::super_block::{SuperBlock, FileSystemType, MountData, Dentry};
use kernel::bindings; // For MS_SILENT flag if used directly

// Import the superblock module where VEXFS_SUPER_OPS is now defined.
mod superblock;
mod inode; // Add inode module
pub mod ioctl; // Add ioctl module
pub mod ondisk; // Add on-disk format definitions
pub mod inode_mgmt; // Add inode management system
pub mod file_ops; // Add file operations
pub mod dir_ops; // Add directory operations
pub mod space_alloc; // Add space allocation system
pub mod journal; // Add journaling and transaction system
pub mod vector_storage; // Add vector embedding storage system
pub mod vector_metrics; // Add SIMD-optimized vector similarity metrics
pub mod knn_search; // Add k-NN search algorithm with metadata filtering
pub mod result_scoring; // Add result scoring, ranking and validation framework
pub mod vector_search; // Add main vector search API
pub mod vector_search_integration; // Add vector search integration layer
pub mod anns; // Add Approximate Nearest Neighbor Search system
pub mod vector_handlers; // Add core vector operation handlers

// The FileSystemType structure for VexFS
struct VexfsFsType;

// Implementation of the FileSystemType trait for VexfsFsType
impl kernel::super_block::FileSystemType for VexfsFsType {
    const NAME: &'static str = "vexfs"; // Filesystem name
    const REQUIRES_DEV: bool = true; // VexFS is a device-based filesystem

    fn mount(
        &'static self, // fs_type reference
        flags: u32,    // mount flags
        dev_name: &kernel::cstr::CStr,
        data: Option<&kernel::cstr::CStr>, // mount options
    ) -> Result<Dentry> {
        pr_info!("VexFS: VexfsFsType::mount called for device: {:?}, flags: {}\n", dev_name, flags);
        
        // Use SuperBlock::mount_bdev for block device filesystems.
        // It requires a fill_super callback.
        SuperBlock::mount_bdev(self, flags, dev_name, move |sb, mount_data| {
            // This closure is the `fill_super` callback.
            // `sb` is `&mut SuperBlock`, `mount_data` is `MountData`.
            pr_info!("VexFS: fill_super (closure via mount_bdev) called for dev: {:?}\n", dev_name);

            // Determine the silent flag for vexfs_fill_super.
            // MS_SILENT is defined in kernel::bindings.
            let silent_c_int = if (flags & bindings::MS_SILENT) != 0 { 1 } else { 0 };
            
            // Call the main fill_super logic from the superblock module.
            // `mount_data.data()` provides `*mut c_void`.
            match superblock::vexfs_fill_super(sb, mount_data.data(), silent_c_int) {
                Ok(()) => {
                    // vexfs_fill_super should have set sb.s_op.
                    // It also needs to set sb.s_root for a successful mount.
                    // If sb.s_root is not set by vexfs_fill_super (or a function it calls),
                    // mount_bdev will likely fail and return an error (e.g., -EINVAL or -ENOMEM).
                    // For this subtask, we are not yet creating the root inode in fill_super.
                    // This means mounting will likely fail in practice, but the registration
                    // and call to fill_super should work.
                    pr_info!("VexFS: fill_super closure completed successfully.\n");
                    Ok(())
                }
                Err(e) => {
                    pr_err!("VexFS: vexfs_fill_super (called from closure) failed: {:?}\n", e);
                    Err(e)
                }
            }
        })
        // mount_bdev returns Result<Dentry>
    }

    fn kill_sb(&'static self, sb: &mut SuperBlock) {
        pr_info!("VexFS: VexfsFsType::kill_sb called (from lib.rs)\n");
        // Call the cleanup logic from the superblock module.
        superblock::vexfs_kill_sb(sb);
    }
}

// Create a static instance of our FileSystemType implementation.
static VEXFS_FS_TYPE: VexfsFsType = VexfsFsType;

// Store the FileSystemType registration handle.
static mut FS_TYPE_REGISTRATION: Option<kernel::fs::RegisteredFileSystem> = None;

module! {
    type: VexFS,
    name: b"vexfs",
    author: b"AI Agent (for VexFS)",
    description: b"VDBHAX/VexFS: Vector-Native File System",
    license: b"GPL",
}

// Define the main module struct.
struct VexFS {} // Empty struct, registration handle stored in FS_TYPE_REGISTRATION.

// This is the entry point called by the `module!` macro.
impl kernel::Module for VexFS {
    fn init(_module: &'static ThisModule) -> KernelResult<Self> {
        pr_info!("VexFS: kernel::Module::init() called. Module is loading.\n");

        // Register the filesystem with the kernel.
        match kernel::fs::register_filesystem(&VEXFS_FS_TYPE) {
            Ok(registration) => {
                pr_info!("VexFS: Filesystem registered successfully with kernel.\n");
                // Store the registration handle to be used for unregistering.
                unsafe {
                    FS_TYPE_REGISTRATION = Some(registration);
                }
            }
            Err(e) => {
                pr_err!("VexFS: Failed to register filesystem with kernel: {:?}\n", e);
                return Err(e); // Propagate the error to fail module loading.
            }
        }
        Ok(VexFS {}) // Return an instance of our module struct.
    }

    fn exit(&mut self, _module: &'static ThisModule) {
        pr_info!("VexFS: kernel::Module::exit() called. Module is unloading.\n");

        // Unregister the filesystem.
        unsafe {
            // `take()` removes the value from Option, leaving None.
            if let Some(registration) = FS_TYPE_REGISTRATION.take() {
                match kernel::fs::unregister_filesystem(registration) {
                    Ok(()) => pr_info!("VexFS: Filesystem unregistered successfully from kernel.\n"),
                    Err(e) => pr_err!("VexFS: Failed to unregister filesystem: {:?}\n", e),
                }
            } else {
                // This case should ideally not happen if registration was successful.
                pr_warn!("VexFS: Filesystem registration handle not found, cannot unregister.\n");
            }
        }
    }
}

// This function is called by the C shim's module_init.
// Its role is to confirm that the Rust part of the module is ready.
// The actual initialization of the VexFS struct is handled by the `module!` macro
// and the `impl kernel::Module for VexFS` block above when the kernel loads the module.
#[no_mangle]
pub extern "C" fn vexfs_rust_init() -> core::ffi::c_int {
    // This message confirms the C shim successfully called into Rust.
    pr_info!("VexFS: vexfs_rust_init() (extern \"C\") called from C shim.\n");
    // We don't need to manually create VexFS or call VexFS::init here.
    // The `module!` macro ensures `VexFS::init` is called by the kernel module loader.
    // This function just needs to return success (0) to the C shim.
    // Any complex setup should be in `VexFS::init`.
    0 // Return 0 for success
}

// This function is called by the C shim's module_exit.
#[no_mangle]
pub extern "C" fn vexfs_rust_exit() {
    // This message confirms the C shim successfully called into Rust for exit.
    pr_info!("VexFS: vexfs_rust_exit() (extern \"C\") called from C shim.\n");
    // We don't need to manually destroy VexFS or call VexFS::exit here.
    // The `module!` macro ensures `VexFS::exit` is called by the kernel module unloader.
    // Any complex cleanup should be in `VexFS::exit`.
}
