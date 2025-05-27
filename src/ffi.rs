/*
 * VexFS - Vector Extended File System
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
 *
 * Note: Kernel module components are licensed under GPL v2.
 * See LICENSE.kernel for kernel-specific licensing terms.
 */

//! FFI (Foreign Function Interface) module for C integration
//!
//! This module provides C-compatible functions that can be called from the
//! C kernel module. All functions use the C calling convention and handle
//! error conditions appropriately for kernel space.

use core::ffi::{c_int, c_void, c_char};
use crate::shared::errors::{VexfsError, VexfsResult};
use crate::fs_core::operations::FilesystemOperations;
use crate::storage::StorageManager;

/// Error codes for C FFI
pub const VEXFS_SUCCESS: c_int = 0;
pub const VEXFS_ERROR_GENERIC: c_int = -1;
pub const VEXFS_ERROR_NOMEM: c_int = -12;  // -ENOMEM
pub const VEXFS_ERROR_INVAL: c_int = -22;  // -EINVAL
pub const VEXFS_ERROR_NOSPC: c_int = -28;  // -ENOSPC
pub const VEXFS_ERROR_PERMISSION: c_int = -1;  // -EPERM
pub const VEXFS_ERROR_NOENT: c_int = -2;   // -ENOENT
pub const VEXFS_ERROR_IO: c_int = -5;      // -EIO
pub const VEXFS_ERROR_EXIST: c_int = -17;  // -EEXIST
pub const VEXFS_ERROR_NOTDIR: c_int = -20; // -ENOTDIR
pub const VEXFS_ERROR_ISDIR: c_int = -21;  // -EISDIR

/// VexFS filesystem constants
pub const VEXFS_NAME_LEN: usize = 255;     // Maximum filename length
pub const VEXFS_MAX_FILE_SIZE: u64 = 1_099_511_627_776; // 1TB max file size
pub const VEXFS_BLOCK_SIZE: u32 = 4096;    // Standard 4KB block size

/// File mode constants (matching Unix/Linux)
pub const VEXFS_S_IFREG: u16 = 0o100000;  // Regular file
pub const VEXFS_S_IFDIR: u16 = 0o040000;  // Directory
pub const VEXFS_S_IFLNK: u16 = 0o120000;  // Symbolic link

/// Helper function to convert VexFS errors to FFI error codes
pub fn to_ffi_result<T>(result: VexfsResult<T>) -> c_int {
    match result {
        Ok(_) => VEXFS_SUCCESS,
        Err(err) => match err {
            VexfsError::InvalidArgument(_) => VEXFS_ERROR_INVAL,
            VexfsError::OutOfMemory => VEXFS_ERROR_NOMEM,
            VexfsError::NoSpaceLeft => VEXFS_ERROR_NOSPC,
            VexfsError::PermissionDenied => VEXFS_ERROR_PERMISSION,
            VexfsError::FileExists => VEXFS_ERROR_EXIST,
            VexfsError::FileNotFound => VEXFS_ERROR_NOENT,
            VexfsError::NotDirectory => VEXFS_ERROR_NOTDIR,
            VexfsError::IsDirectory => VEXFS_ERROR_ISDIR,
            VexfsError::IoError(_) => VEXFS_ERROR_IO,
            _ => VEXFS_ERROR_GENERIC,
        }
    }
}

/// Helper function for legacy string-based error results
pub fn to_ffi_result_legacy<T>(result: Result<T, &'static str>) -> c_int {
    match result {
        Ok(_) => VEXFS_SUCCESS,
        Err(msg) => match msg {
            "Invalid arguments" => VEXFS_ERROR_INVAL,
            "Out of memory" => VEXFS_ERROR_NOMEM,
            "No space left" => VEXFS_ERROR_NOSPC,
            "Permission denied" => -1, // EPERM
            "File exists" => -17, // EEXIST
            "No such file or directory" => -2, // ENOENT
            "Not a directory" => -20, // ENOTDIR
            "Is a directory" => -21, // EISDIR
            "Directory not empty" => -39, // ENOTEMPTY
            "Name too long" => -36, // ENAMETOOLONG
            "Cross-device link" => -18, // EXDEV
            "Too many links" => -31, // EMLINK
            _ => VEXFS_ERROR_GENERIC,
        }
    }
}

/// FFI Result type for convenience
pub type FFIResult<T> = Result<T, &'static str>;

/// Initialize the VexFS Rust components
/// Called during module_init from C kernel module
#[no_mangle]
pub extern "C" fn vexfs_rust_init() -> c_int {
    // TODO: Initialize global state, allocators, etc.
    // For now, just return success
    VEXFS_SUCCESS
}

/// Cleanup the VexFS Rust components
/// Called during module_exit from C kernel module
#[no_mangle]
pub extern "C" fn vexfs_rust_exit() {
    // TODO: Cleanup global state, free resources, etc.
    // For now, this is a no-op
}

/// Initialize the VexFS superblock structure
/// Called during filesystem mount from C kernel module
///
/// # Arguments
/// * `sb_ptr` - Pointer to the Linux superblock structure
///
/// # Returns
/// * `VEXFS_SUCCESS` on success
/// * Error code on failure
#[no_mangle]
pub extern "C" fn vexfs_rust_fill_super(sb_ptr: *mut c_void) -> c_int {
    if sb_ptr.is_null() {
        return VEXFS_ERROR_INVAL;
    }

    // TODO: Initialize VexFS-specific superblock data
    // This will include vector index metadata, journal setup, etc.
    
    VEXFS_SUCCESS
}

/// Test function to verify FFI is working
/// This is a simple test function that can be called from C
#[no_mangle]
pub extern "C" fn vexfs_rust_test_basic() -> c_int {
    // Perform a basic test operation
    let test_value = 42;
    if test_value == 42 {
        VEXFS_SUCCESS
    } else {
        VEXFS_ERROR_GENERIC
    }
}

/// Test function for vector operations
/// This tests that vector-related code can be called via FFI
#[no_mangle]
pub extern "C" fn vexfs_rust_test_vector_ops() -> c_int {
    // TODO: Test basic vector operations
    // For now, just return success
    VEXFS_SUCCESS
}

/// Get version information
/// Returns a packed version number (major << 16 | minor << 8 | patch)
#[no_mangle]
pub extern "C" fn vexfs_rust_get_version() -> c_int {
    // Version 0.1.0 = 0 << 16 | 1 << 8 | 0 = 256
    (0 << 16) | (1 << 8) | 0
}

/// Filesystem statistics FFI function
/// Called to fill filesystem statistics from Rust implementation
///
/// # Arguments
/// * `blocks` - Pointer to store total blocks
/// * `free_blocks` - Pointer to store free blocks
/// * `files` - Pointer to store total files
/// * `free_files` - Pointer to store free files
///
/// # Returns
/// * `VEXFS_SUCCESS` on success
/// * Error code on failure
#[no_mangle]
pub extern "C" fn vexfs_rust_get_statfs(
    blocks: *mut u64,
    free_blocks: *mut u64,
    files: *mut u64,
    free_files: *mut u64,
) -> c_int {
    if blocks.is_null() || free_blocks.is_null() || files.is_null() || free_files.is_null() {
        return VEXFS_ERROR_INVAL;
    }

    unsafe {
        // TODO: Get actual filesystem statistics
        // For now, return placeholder values
        *blocks = 1000000;      // 1M blocks
        *free_blocks = 900000;  // 900K free
        *files = 10000;         // 10K files
        *free_files = 9000;     // 9K free
    }

    VEXFS_SUCCESS
}

/// Create and initialize a new inode
/// Called from C kernel module when creating inodes
///
/// # Arguments
/// * `sb_ptr` - Pointer to the Linux superblock structure
/// * `ino` - Inode number to assign
/// * `mode` - File mode (permissions and type)
///
/// # Returns
/// * Pointer to allocated inode on success
/// * NULL on failure
#[no_mangle]
pub extern "C" fn vexfs_rust_new_inode(
    sb_ptr: *mut c_void,
    ino: u64,
    mode: u32,
) -> *mut c_void {
    if sb_ptr.is_null() {
        return core::ptr::null_mut();
    }

    // TODO: Implement actual inode creation
    // For now, return a placeholder that indicates success to C layer
    // The C layer will handle the actual Linux inode allocation
    
    // Return non-null to indicate success - C layer handles actual inode
    1 as *mut c_void
}

/// Initialize VFS-specific inode data
/// Called from C kernel module after Linux inode allocation
///
/// # Arguments
/// * `inode_ptr` - Pointer to the Linux inode structure
/// * `ino` - Inode number
/// * `mode` - File mode
///
/// # Returns
/// * `VEXFS_SUCCESS` on success
/// * Error code on failure
#[no_mangle]
pub extern "C" fn vexfs_rust_init_inode(
    inode_ptr: *mut c_void,
    ino: u64,
    mode: u32,
) -> c_int {
    if inode_ptr.is_null() {
        return VEXFS_ERROR_INVAL;
    }

    // TODO: Initialize VexFS-specific inode data
    // This includes vector metadata, extended attributes, etc.
    
    VEXFS_SUCCESS
}

/// Cleanup VFS-specific inode data
/// Called from C kernel module before inode deallocation
///
/// # Arguments
/// * `inode_ptr` - Pointer to the Linux inode structure
#[no_mangle]
pub extern "C" fn vexfs_rust_destroy_inode(inode_ptr: *mut c_void) {
    if inode_ptr.is_null() {
        return;
    }

    // TODO: Cleanup VexFS-specific inode data
    // Free vector data, indices, etc.
}

/// Write inode to storage
/// Called from C kernel module when inode needs to be persisted
///
/// # Arguments
/// * `inode_ptr` - Pointer to the Linux inode structure
///
/// # Returns
/// * `VEXFS_SUCCESS` on success
/// * Error code on failure
#[no_mangle]
pub extern "C" fn vexfs_rust_write_inode(inode_ptr: *mut c_void) -> c_int {
    if inode_ptr.is_null() {
        return VEXFS_ERROR_INVAL;
    }

    // TODO: Implement inode persistence
    // Write inode data, vector metadata, etc. to storage
    
    VEXFS_SUCCESS
}

/// Synchronize filesystem data
/// Called from C kernel module during sync operations
///
/// # Arguments
/// * `sb_ptr` - Pointer to the Linux superblock structure
/// * `wait` - Whether to wait for completion
///
/// # Returns
/// * `VEXFS_SUCCESS` on success
/// * Error code on failure
#[no_mangle]
pub extern "C" fn vexfs_rust_sync_fs(sb_ptr: *mut c_void, wait: c_int) -> c_int {
    if sb_ptr.is_null() {
        return VEXFS_ERROR_INVAL;
    }

    // TODO: Implement filesystem synchronization
    // Flush journals, vector indices, metadata, etc.
    
    VEXFS_SUCCESS
}

/// Put (cleanup) superblock
/// Called from C kernel module during unmount
///
/// # Arguments
/// * `sb_ptr` - Pointer to the Linux superblock structure
#[no_mangle]
pub extern "C" fn vexfs_rust_put_super(sb_ptr: *mut c_void) {
    if sb_ptr.is_null() {
        return;
    }

    // TODO: Cleanup superblock-specific data
    // Close journals, cleanup vector indices, etc.
}

// ============================================================================
// File Operations FFI Functions
// ============================================================================

/// Helper to get filesystem operations instance
/// TODO: Replace with proper global state management
fn get_fs_ops() -> VexfsResult<FilesystemOperations> {
    // For now, create a new instance each time
    // In production, this should be a global singleton
    let storage_manager = StorageManager::new()?;
    FilesystemOperations::new(storage_manager)
}

/// Create a new file
#[no_mangle]
pub extern "C" fn vexfs_create_file(
    path: *const c_char,
    mode: u32,
) -> c_int {
    if path.is_null() {
        return VEXFS_ERROR_INVAL;
    }

    // Convert C string to Rust string
    let path_str = match unsafe { core::ffi::CStr::from_ptr(path) }.to_str() {
        Ok(s) => s,
        Err(_) => return VEXFS_ERROR_INVAL,
    };

    // Get filesystem operations and create file
    match get_fs_ops() {
        Ok(fs_ops) => match fs_ops.create_file(path_str, mode) {
            Ok(_) => VEXFS_SUCCESS,
            Err(err) => to_ffi_result(Err(err)),
        },
        Err(err) => to_ffi_result(Err(err)),
    }
}

/// Open a file
#[no_mangle]
pub extern "C" fn vexfs_open_file(
    path: *const c_char,
    flags: u32,
) -> c_int {
    if path.is_null() {
        return VEXFS_ERROR_INVAL;
    }

    let path_str = match unsafe { core::ffi::CStr::from_ptr(path) }.to_str() {
        Ok(s) => s,
        Err(_) => return VEXFS_ERROR_INVAL,
    };

    match get_fs_ops() {
        Ok(fs_ops) => match fs_ops.open_file(path_str, flags) {
            Ok(_) => VEXFS_SUCCESS,
            Err(err) => to_ffi_result(Err(err)),
        },
        Err(err) => to_ffi_result(Err(err)),
    }
}

/// Read from a file
#[no_mangle]
pub extern "C" fn vexfs_read_file(
    path: *const c_char,
    buffer: *mut c_void,
    size: usize,
    offset: u64,
) -> c_int {
    if path.is_null() || buffer.is_null() {
        return VEXFS_ERROR_INVAL;
    }

    let path_str = match unsafe { core::ffi::CStr::from_ptr(path) }.to_str() {
        Ok(s) => s,
        Err(_) => return VEXFS_ERROR_INVAL,
    };

    let buffer_slice = unsafe { core::slice::from_raw_parts_mut(buffer as *mut u8, size) };

    match get_fs_ops() {
        Ok(fs_ops) => match fs_ops.read_file(path_str, buffer_slice, offset) {
            Ok(bytes_read) => bytes_read as c_int,
            Err(err) => to_ffi_result(Err(err)),
        },
        Err(err) => to_ffi_result(Err(err)),
    }
}

/// Write to a file
#[no_mangle]
pub extern "C" fn vexfs_write_file(
    path: *const c_char,
    buffer: *const c_void,
    size: usize,
    offset: u64,
) -> c_int {
    if path.is_null() || buffer.is_null() {
        return VEXFS_ERROR_INVAL;
    }

    let path_str = match unsafe { core::ffi::CStr::from_ptr(path) }.to_str() {
        Ok(s) => s,
        Err(_) => return VEXFS_ERROR_INVAL,
    };

    let buffer_slice = unsafe { core::slice::from_raw_parts(buffer as *const u8, size) };

    match get_fs_ops() {
        Ok(fs_ops) => match fs_ops.write_file(path_str, buffer_slice, offset) {
            Ok(bytes_written) => bytes_written as c_int,
            Err(err) => to_ffi_result(Err(err)),
        },
        Err(err) => to_ffi_result(Err(err)),
    }
}

/// Close a file
#[no_mangle]
pub extern "C" fn vexfs_close_file(path: *const c_char) -> c_int {
    if path.is_null() {
        return VEXFS_ERROR_INVAL;
    }

    let path_str = match unsafe { core::ffi::CStr::from_ptr(path) }.to_str() {
        Ok(s) => s,
        Err(_) => return VEXFS_ERROR_INVAL,
    };

    match get_fs_ops() {
        Ok(fs_ops) => match fs_ops.close_file(path_str) {
            Ok(_) => VEXFS_SUCCESS,
            Err(err) => to_ffi_result(Err(err)),
        },
        Err(err) => to_ffi_result(Err(err)),
    }
}

/// Delete a file
#[no_mangle]
pub extern "C" fn vexfs_unlink_file(path: *const c_char) -> c_int {
    if path.is_null() {
        return VEXFS_ERROR_INVAL;
    }

    let path_str = match unsafe { core::ffi::CStr::from_ptr(path) }.to_str() {
        Ok(s) => s,
        Err(_) => return VEXFS_ERROR_INVAL,
    };

    match get_fs_ops() {
        Ok(fs_ops) => match fs_ops.unlink_file(path_str) {
            Ok(_) => VEXFS_SUCCESS,
            Err(err) => to_ffi_result(Err(err)),
        },
        Err(err) => to_ffi_result(Err(err)),
    }
}

/// Truncate a file
#[no_mangle]
pub extern "C" fn vexfs_truncate_file(path: *const c_char, size: u64) -> c_int {
    if path.is_null() {
        return VEXFS_ERROR_INVAL;
    }

    let path_str = match unsafe { core::ffi::CStr::from_ptr(path) }.to_str() {
        Ok(s) => s,
        Err(_) => return VEXFS_ERROR_INVAL,
    };

    match get_fs_ops() {
        Ok(fs_ops) => match fs_ops.truncate_file(path_str, size) {
            Ok(_) => VEXFS_SUCCESS,
            Err(err) => to_ffi_result(Err(err)),
        },
        Err(err) => to_ffi_result(Err(err)),
    }
}

// ============================================================================
// Directory Operations FFI Functions
// ============================================================================

/// Create a directory
#[no_mangle]
pub extern "C" fn vexfs_create_dir(path: *const c_char, mode: u32) -> c_int {
    if path.is_null() {
        return VEXFS_ERROR_INVAL;
    }

    let path_str = match unsafe { core::ffi::CStr::from_ptr(path) }.to_str() {
        Ok(s) => s,
        Err(_) => return VEXFS_ERROR_INVAL,
    };

    match get_fs_ops() {
        Ok(fs_ops) => match fs_ops.create_directory(path_str, mode) {
            Ok(_) => VEXFS_SUCCESS,
            Err(err) => to_ffi_result(Err(err)),
        },
        Err(err) => to_ffi_result(Err(err)),
    }
}

/// Remove a directory
#[no_mangle]
pub extern "C" fn vexfs_remove_dir(path: *const c_char) -> c_int {
    if path.is_null() {
        return VEXFS_ERROR_INVAL;
    }

    let path_str = match unsafe { core::ffi::CStr::from_ptr(path) }.to_str() {
        Ok(s) => s,
        Err(_) => return VEXFS_ERROR_INVAL,
    };

    match get_fs_ops() {
        Ok(fs_ops) => match fs_ops.remove_directory(path_str) {
            Ok(_) => VEXFS_SUCCESS,
            Err(err) => to_ffi_result(Err(err)),
        },
        Err(err) => to_ffi_result(Err(err)),
    }
}

/// List directory contents
#[no_mangle]
pub extern "C" fn vexfs_list_dir(
    path: *const c_char,
    buffer: *mut c_void,
    buffer_size: usize,
) -> c_int {
    if path.is_null() || buffer.is_null() {
        return VEXFS_ERROR_INVAL;
    }

    let path_str = match unsafe { core::ffi::CStr::from_ptr(path) }.to_str() {
        Ok(s) => s,
        Err(_) => return VEXFS_ERROR_INVAL,
    };

    let buffer_slice = unsafe { core::slice::from_raw_parts_mut(buffer as *mut u8, buffer_size) };

    match get_fs_ops() {
        Ok(fs_ops) => match fs_ops.list_directory(path_str, buffer_slice) {
            Ok(entries_count) => entries_count as c_int,
            Err(err) => to_ffi_result(Err(err)),
        },
        Err(err) => to_ffi_result(Err(err)),
    }
}

/// Rename a file or directory
#[no_mangle]
pub extern "C" fn vexfs_rename(
    old_path: *const c_char,
    new_path: *const c_char,
) -> c_int {
    if old_path.is_null() || new_path.is_null() {
        return VEXFS_ERROR_INVAL;
    }

    let old_path_str = match unsafe { core::ffi::CStr::from_ptr(old_path) }.to_str() {
        Ok(s) => s,
        Err(_) => return VEXFS_ERROR_INVAL,
    };

    let new_path_str = match unsafe { core::ffi::CStr::from_ptr(new_path) }.to_str() {
        Ok(s) => s,
        Err(_) => return VEXFS_ERROR_INVAL,
    };

    match get_fs_ops() {
        Ok(fs_ops) => match fs_ops.rename_entry(old_path_str, new_path_str) {
            Ok(_) => VEXFS_SUCCESS,
            Err(err) => to_ffi_result(Err(err)),
        },
        Err(err) => to_ffi_result(Err(err)),
    }
}

/// Create a hard link
#[no_mangle]
pub extern "C" fn vexfs_link(
    target: *const c_char,
    link_path: *const c_char,
) -> c_int {
    if target.is_null() || link_path.is_null() {
        return VEXFS_ERROR_INVAL;
    }

    let target_str = match unsafe { core::ffi::CStr::from_ptr(target) }.to_str() {
        Ok(s) => s,
        Err(_) => return VEXFS_ERROR_INVAL,
    };

    let link_path_str = match unsafe { core::ffi::CStr::from_ptr(link_path) }.to_str() {
        Ok(s) => s,
        Err(_) => return VEXFS_ERROR_INVAL,
    };

    match get_fs_ops() {
        Ok(fs_ops) => match fs_ops.create_hard_link(target_str, link_path_str) {
            Ok(_) => VEXFS_SUCCESS,
            Err(err) => to_ffi_result(Err(err)),
        },
        Err(err) => to_ffi_result(Err(err)),
    }
}

/// Create a symbolic link
#[no_mangle]
pub extern "C" fn vexfs_symlink(
    target: *const c_char,
    link_path: *const c_char,
) -> c_int {
    if target.is_null() || link_path.is_null() {
        return VEXFS_ERROR_INVAL;
    }

    let target_str = match unsafe { core::ffi::CStr::from_ptr(target) }.to_str() {
        Ok(s) => s,
        Err(_) => return VEXFS_ERROR_INVAL,
    };

    let link_path_str = match unsafe { core::ffi::CStr::from_ptr(link_path) }.to_str() {
        Ok(s) => s,
        Err(_) => return VEXFS_ERROR_INVAL,
    };

    match get_fs_ops() {
        Ok(fs_ops) => match fs_ops.create_symbolic_link(target_str, link_path_str) {
            Ok(_) => VEXFS_SUCCESS,
            Err(err) => to_ffi_result(Err(err)),
        },
        Err(err) => to_ffi_result(Err(err)),
    }
}

/// Cleanup superblock during unmount
/// Called from vexfs_kill_sb in C kernel module
///
/// # Arguments
/// * `sb_ptr` - Pointer to the Linux superblock structure
#[no_mangle]
pub extern "C" fn vexfs_rust_cleanup_superblock(sb_ptr: *mut c_void) {
    if sb_ptr.is_null() {
        return;
    }

    // TODO: Implement superblock cleanup
    // This would typically involve:
    // - Flushing any cached data
    // - Cleaning up allocated resources
    // - Syncing journal if present
}

#[cfg(not(feature = "kernel"))]
pub mod userspace_ffi {
    //! Userspace FFI functions for testing and development
    
    use std::os::raw::c_int;

    /// Userspace test function for vector search operations
    #[no_mangle]
    pub extern "C" fn vexfs_rust_vector_search() -> c_int {
        // TODO: Implement actual vector search
        0
    }

    /// Userspace test function for vector storage operations
    #[no_mangle]
    pub extern "C" fn vexfs_rust_vector_storage() -> c_int {
        // TODO: Implement actual vector storage
        0
    }

    /// Userspace initialization for testing
    #[no_mangle]
    pub extern "C" fn vexfs_rust_userspace_init() -> c_int {
        println!("VexFS: Initializing Rust components in userspace");
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ffi_basic() {
        assert_eq!(vexfs_rust_test_basic(), VEXFS_SUCCESS);
    }

    #[test]
    fn test_version() {
        let version = vexfs_rust_get_version();
        assert_eq!(version, 256); // 0.1.0
    }

    #[test]
    fn test_statfs() {
        let mut blocks = 0u64;
        let mut free_blocks = 0u64;
        let mut files = 0u64;
        let mut free_files = 0u64;

        let result = vexfs_rust_get_statfs(
            &mut blocks,
            &mut free_blocks,
            &mut files,
            &mut free_files,
        );

        assert_eq!(result, VEXFS_SUCCESS);
        assert!(blocks > 0);
        assert!(free_blocks > 0);
        assert!(files > 0);
        assert!(free_files > 0);
    }

    #[test]
    fn test_null_pointers() {
        let result = vexfs_rust_fill_super(core::ptr::null_mut());
        assert_eq!(result, VEXFS_ERROR_INVAL);

        let result = vexfs_rust_get_statfs(
            core::ptr::null_mut(),
            core::ptr::null_mut(),
            core::ptr::null_mut(),
            core::ptr::null_mut(),
        );
        assert_eq!(result, VEXFS_ERROR_INVAL);
    }
}