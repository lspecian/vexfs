use kernel::prelude::*;
use kernel::super_block::{SuperBlock, SuperOperations, Dentry}; // Added Dentry
use kernel::bindings; // For c_int, c_void, S_IFDIR, etc.
use kernel::sync::Arc; // Using Arc for s_fs_info if VexfsSuperblock needs to be shared
                       // and for Arc<Inode> from vexfs_get_inode

// Import inode functions
use crate::inode;

// Define a unique magic number for VexFS
pub const VEXFS_MAGIC: u32 = 0xDEADBEEF; // Example magic number

// VexFS-specific superblock information
#[repr(C)] // Ensure C layout compatibility if directly passed via s_fs_info raw pointer
pub struct VexfsSuperblock {
    pub magic: u32,
    // Add other VexFS-specific fields here later
    // For example: version, block_size, etc.
    pub custom_field: u64,
}

impl VexfsSuperblock {
    fn new() -> Self {
        VexfsSuperblock {
            magic: VEXFS_MAGIC,
            custom_field: 12345,
        }
    }
}

// Define the super_operations for VexFS.
// For now, most operations will be NULL or simple stubs.
// We'll need to define functions for these operations later.
// For example: alloc_inode, destroy_inode, write_inode, statfs, etc.
// C representation of super_operations.
// The `kernel` crate might provide a more Rusty way to define this,
// but often it involves defining a static struct similar to C.

// Example of how super_operations might be defined.
// The actual implementation details will depend on the `kernel` crate's API.
// For now, let's assume we'll define a static `SuperOperations` instance.
// We need to ensure the function pointers are compatible (e.g., extern "C" fn).

// Placeholder for vexfs_statfs if needed by super_block setup
#[no_mangle]
extern "C" fn vexfs_statfs(_dentry: *mut bindings::dentry, _kstatfs: *mut bindings::kstatfs) -> c_int {
    // In a real implementation, this would fill kstatfs with filesystem statistics.
    // For now, just return 0 (success) or an error code.
    // pr_info!("VexFS: vexfs_statfs called (dummy)\n");
    0
}


// TODO: Define more operations as needed.
// static VEXFS_SUPER_OPS: SuperOperations = SuperOperations {
// statfs: Some(vexfs_statfs),
// drop_inode: Some(generic_delete_inode), // Or a custom one
//     // ... other operations, many will be None initially
// };
// The `kernel` crate's `SuperOperations` struct needs to be populated.

// Define the SuperOperations for VexFS
// This needs to be a static instance.
// For now, most operations will be None or stubs.
pub static VEXFS_SUPER_OPS: SuperOperations = SuperOperations {
    // Initialize with default operations, or specific ones if available.
    alloc_inode: None, // To be implemented
    destroy_inode: None, // To be implemented
    write_inode: None, // To be implemented
    dirty_inode: None, // To be implemented
    drop_inode: Some(kernel::fs::generic_drop_inode), // Use a generic helper
    evict_inode: None, // To be implemented, often custom
    put_super: None, // kill_sb handles overall cleanup
    sync_fs: None, // To be implemented
    freeze_fs: None, // To be implemented
    unfreeze_fs: None, // To be implemented
    statfs: Some(vexfs_statfs), // Using the extern "C" function defined above
    remount_fs: None, // To be implemented
    show_options: None, // To be implemented
    // Add other operations as they are defined.
    // Ensure all fields required by the SuperOperations struct are listed.
};


// This function will be called by the VFS to fill in the superblock.
// The `sb` parameter is a mutable reference to the kernel's `super_block`.
// `data` is mount-specific data (e.g., options).
// `silent` indicates whether to suppress error messages.
pub fn vexfs_fill_super(
    sb: &mut SuperBlock,
    _data: *mut core::ffi::c_void, // Mount options, unused for now
    _silent: c_int,
) -> Result<()> { // Returning Result for error handling
    pr_info!("VexFS: vexfs_fill_super called\n");

    // 1. Initialize VexfsSuperblock
    // We need to allocate VexfsSuperblock and store it in sb.s_fs_info.
    // The `kernel` crate might offer helpers for this, e.g., Box::into_raw.
    // s_fs_info is typically a *mut c_void.
    let vex_sb = Box::try_new(VexfsSuperblock::new())?;
    // The VexfsSuperblock instance needs to live as long as the superblock exists.
    // Box::into_raw converts the Box into a raw pointer, leaking its memory.
    // This is a common pattern for s_fs_info, which is then freed in kill_sb.
    sb.s_fs_info = Box::into_raw(vex_sb) as *mut core::ffi::c_void;

    // 2. Set up generic superblock fields
    sb.s_magic = VEXFS_MAGIC;

    // Set super_operations (s_op)
    // Assigning our static VEXFS_SUPER_OPS to the superblock's s_op field.
    // The `kernel` crate's `SuperBlock` struct has an `s_op` field which is
    // typically `*const struct super_operations` (from C bindings).
    // We take a reference to our static `VEXFS_SUPER_OPS`, cast it to a raw pointer
    // of the correct type (`*const bindings::super_operations`).
    sb.s_op = &VEXFS_SUPER_OPS as *const SuperOperations as *const bindings::super_operations;
    pr_info!("VexFS: sb.s_op set.\n");

    // 3. Get root inode and set sb.s_root
    pr_info!("VexFS: Attempting to get root inode...\n");
    let root_inode_arc = match inode::vexfs_get_inode(sb, bindings::S_IFDIR) {
        Ok(inode_arc) => inode_arc,
        Err(e) => {
            pr_err!("VexFS: Failed to get root inode: {:?}\n", e);
            // Clean up s_fs_info before returning
            unsafe {
                let _ = Box::from_raw(sb.s_fs_info as *mut VexfsSuperblock);
            }
            sb.s_fs_info = core::ptr::null_mut();
            return Err(e);
        }
    };

    pr_info!("VexFS: Root inode obtained successfully. Ino: {}\n", root_inode_arc.i_ino);

    // Create the root dentry
    // Dentry::d_make_root takes Arc<Inode>
    let root_dentry = match Dentry::d_make_root(root_inode_arc) {
        Some(dentry) => dentry,
        None => {
            pr_err!("VexFS: Failed to create root dentry (d_make_root returned None).\n");
            // Clean up s_fs_info. The Arc<Inode> for root_inode_arc will be dropped automatically.
            unsafe {
                let _ = Box::from_raw(sb.s_fs_info as *mut VexfsSuperblock);
            }
            sb.s_fs_info = core::ptr::null_mut();
            return Err(kernel::Error::ENOMEM); // ENOMEM is a common error for dentry creation failure
        }
    };
    
    sb.s_root = root_dentry;
    pr_info!("VexFS: sb.s_root set successfully.\n");

    pr_info!("VexFS: Superblock filled. Magic: {:#x}, Root Dentry: {:?}\n", sb.s_magic, sb.s_root);
    Ok(())
}

// Function to be called by kill_sb to clean up VexfsSuperblock
pub fn vexfs_kill_sb(sb: &mut SuperBlock) {
    pr_info!("VexFS: vexfs_kill_sb called\n");
    if !sb.s_fs_info.is_null() {
        unsafe {
            // Convert the raw pointer back to a Box to deallocate the memory.
            let _vex_sb_box = Box::from_raw(sb.s_fs_info as *mut VexfsSuperblock);
            // _vex_sb_box goes out of scope here and memory is freed.
            pr_info!("VexFS: VexfsSuperblock (s_fs_info) freed.\n");
        }
        sb.s_fs_info = core::ptr::null_mut();
    } else {
        pr_warn!("VexFS: vexfs_kill_sb: s_fs_info was null, nothing to free.\n");
    }
    // Additional cleanup for the superblock if needed.
}

// In a real scenario, s_op would point to a static struct like this:
// static VEXFS_S_OP: bindings::super_operations = bindings::super_operations {
//     statfs: Some(vexfs_statfs_c_wrapper), // wrapper for vexfs_statfs
//     // ... other function pointers, often extern "C"
//     // drop_inode: Some(generic_delete_inode_wrapper),
//     // etc.
// };
// The `kernel` crate should provide `SuperOperations` struct.
// We need to populate its fields.
// For now, we won't set sb.s_op in vexfs_fill_super until we have a proper static definition.
// This might cause issues if the kernel expects s_op to be set.
// Let's assume for now that fill_super's main job is s_fs_info and magic.
// The root dentry and s_op are critical and will be addressed.

// Placeholder for root inode function (to be implemented later)
// fn vexfs_get_root_inode(sb: &mut SuperBlock) -> Result<*mut bindings::inode> {
//    pr_info!("VexFS: vexfs_get_root_inode (dummy) called\n");
//    Err(kernel::Error::ENOENT) // Not found, or not implemented
// }
