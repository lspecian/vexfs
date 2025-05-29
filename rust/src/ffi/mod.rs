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

use core::ffi::c_int;
use crate::shared::errors::{VexfsError, VexfsResult};

pub mod kernel;
pub mod error_handling;
pub mod hang_prevention;

/// Error codes for C FFI - these must match the C header definitions
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
pub const VEXFS_ERROR_FS: c_int = 2;       // Filesystem-specific error

/// Helper function to convert VexFS errors to FFI error codes
pub fn to_ffi_result<T>(result: VexfsResult<T>) -> c_int {
    match result {
        Ok(_) => VEXFS_SUCCESS,
        Err(err) => match err {
            VexfsError::InvalidArgument(_) => VEXFS_ERROR_INVAL,
            VexfsError::OutOfMemory => VEXFS_ERROR_NOMEM,
            VexfsError::NoSpaceLeft => VEXFS_ERROR_NOSPC,
            VexfsError::PermissionDenied(_) => VEXFS_ERROR_PERMISSION,
            VexfsError::FileExists => VEXFS_ERROR_EXIST,
            VexfsError::FileNotFound => VEXFS_ERROR_NOENT,
            VexfsError::NotADirectory(_) => VEXFS_ERROR_NOTDIR,
            VexfsError::IsDirectory => VEXFS_ERROR_ISDIR,
            VexfsError::IoError(_) => VEXFS_ERROR_IO,
            _ => VEXFS_ERROR_GENERIC,
        }
    }
}

// Re-export kernel FFI functions for easier access
pub use kernel::{
    vexfs_rust_init,
    vexfs_rust_exit,
    vexfs_rust_fill_super,
    vexfs_rust_get_statfs,
    vexfs_rust_test_basic,
    vexfs_rust_test_vector_ops,
    vexfs_rust_get_version,
    vexfs_rust_new_inode,
    vexfs_rust_init_inode,
    vexfs_rust_destroy_inode,
    vexfs_rust_write_inode,
    vexfs_rust_sync_fs,
    vexfs_rust_put_super,
    vexfs_rust_cleanup_superblock,
};

// Re-export hang prevention FFI functions
pub use hang_prevention::{
    vexfs_rust_init_hang_prevention,
    vexfs_rust_shutdown_hang_prevention,
    vexfs_rust_start_watchdog,
    vexfs_rust_cancel_watchdog,
    vexfs_rust_check_operation_allowed,
    vexfs_rust_update_resources,
    vexfs_rust_get_health_status,
    vexfs_rust_handle_panic,
    vexfs_rust_hang_prevention_available,
    vexfs_rust_get_hang_prevention_stats,
    vexfs_rust_force_degradation,
};

// Re-export userspace FFI functions for easier access
#[cfg(not(feature = "kernel"))]
pub use kernel::{
    vexfs_rust_vector_search,
    vexfs_rust_vector_storage,
    vexfs_rust_userspace_init,
};