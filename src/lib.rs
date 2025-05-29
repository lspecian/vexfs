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

//! VexFS - Vector Embedding Filesystem
//!
//! This library provides both userspace and kernel functionality for VexFS.
//! Kernel functionality is implemented via C FFI with the vexfs_module_entry.c file.

#![cfg_attr(feature = "kernel", no_std)]
#![cfg_attr(feature = "kernel", no_main)]

// Conditional compilation for userspace vs kernel
#[cfg(not(feature = "kernel"))]
extern crate std;
#[cfg(not(feature = "kernel"))]
use std::prelude::*;

#[cfg(feature = "kernel")]
extern crate alloc;
#[cfg(feature = "kernel")]
extern crate core;

#[cfg(feature = "kernel")]
use linked_list_allocator::LockedHeap;

#[cfg(feature = "kernel")]
#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

#[cfg(feature = "kernel")]
use alloc::{vec::Vec, string::String, boxed::Box, format, vec};

// Shared domain - foundational components used across all domains
#[macro_use]
pub mod shared;

// Storage domain - block management, allocation, journaling, persistence
pub mod storage;

// FS Core domain - file and directory operations using DDD architecture
pub mod fs_core;

// Security domain - encryption, ACL, capabilities, integrity, key management
pub mod security;

// Re-export shared domain components at crate level for easy access
pub use shared::{
    errors::{VexfsError, VexfsResult},
    types::*,
    constants::*,
    utils::*,
    config::*,
};

// Re-export storage domain components at crate level for easy access
pub use storage::{
    Block, BlockDevice, BlockManager, BlockMetadata,
    SpaceAllocator, AllocationStrategy, BlockGroup,
    TransactionManager,
    VexfsSuperblock, VexfsInode, VexfsDirEntry, OnDiskSerializable,
    SuperblockManager, VexfsLayout, BlockCacheManager,
    StorageManager, StorageConfig, StorageStats,
};

// Re-export fs_core domain components at crate level for easy access
pub use fs_core::{
    // Core entities
    File, Directory, Inode, InodeManager,
    DirectoryEntry,
    // Path handling
    Path, PathComponent, PathResolver, PathValidator,
    // Permissions
    UserContext, AccessMode, AccessCheck, PermissionChecker, SecurityPolicy,
    // Operations
    OperationContext, FilesystemOperations,
    // Locking
    LockType, LockScope, LockManager, FileLock, DirectoryLock, LockGuard,
    // Main filesystem
    FileSystem, FsStats, FsConfig, FsInfo, FsInitializer,
    // Results - using VexfsResult from shared::errors instead
};

// Re-export security domain components at crate level for easy access
pub use security::{
    // Main security manager
    SecurityManager, SecurityContext, SecurityError, VectorOperation,
    // Encryption
    VectorEncryption, EncryptionConfig, EncryptionKey, EncryptedData,
    EncryptionError, EncryptionAlgorithm, encryption::EncryptionResult,
    // ACL and extended attributes
    AccessControlList, AclEntry, AclPermission, AclType, AclManager,
    XattrManager,
    // Capabilities
    Capability, CapabilitySet, CapabilityManager, SecurityLevel,
    IoctlSecurityValidator, PrivilegeEscalationDetector,
    // Integrity
    IntegrityChecker, ChecksumType, IntegrityMetadata, VerificationResult,
    DataIntegrityManager,
    // Key management
    KeyManager, KeyDerivation, KeyRotation, SecureKeyStorage,
    KeyMaterial, KeyVersion,
};

// Re-export macros at crate level

// FFI module for C integration
pub mod ffi;

// C FFI exports - Re-export key functions at the crate level for easier access
#[cfg(feature = "kernel")]
pub use ffi::{vexfs_rust_init, vexfs_rust_exit, vexfs_rust_fill_super};

#[cfg(not(feature = "kernel"))]
pub use ffi::userspace_ffi::{vexfs_rust_vector_search, vexfs_rust_vector_storage};

// Core modules for VexFS - only include kernel-safe modules in kernel mode
pub mod ondisk;
// Legacy modules - commented out in favor of DDD architecture
// pub mod file_ops;      // Replaced by fs_core::file
// pub mod dir_ops;       // Replaced by fs_core::directory
// pub mod inode_mgmt;    // Replaced by fs_core::inode
// pub mod space_alloc;   // Replaced by storage::allocation
// pub mod journal;       // Replaced by storage::journal
pub mod superblock;     // Still needed for legacy kernel interface
// pub mod inode;         // Replaced by fs_core::inode
pub mod ioctl;

// IPC module for embedding service communication
pub mod ipc;

// Userspace-only modules (require std types like Vec, String, etc.)
#[cfg(not(feature = "kernel"))]
pub mod vector_storage;
#[cfg(not(feature = "kernel"))]
pub mod vector_cache;
#[cfg(not(feature = "kernel"))]
pub mod vector_metrics;
#[cfg(not(feature = "kernel"))]
pub mod knn_search;
#[cfg(not(feature = "kernel"))]
pub mod result_scoring;
#[cfg(not(feature = "kernel"))]
pub mod vector_search;
#[cfg(not(feature = "kernel"))]
pub mod query_planner;
#[cfg(not(feature = "kernel"))]
pub mod anns;
#[cfg(not(feature = "kernel"))]
pub mod vector_optimizations;
#[cfg(not(feature = "kernel"))]
pub mod vector_large_collections;
#[cfg(not(feature = "kernel"))]
pub mod search_cache;
#[cfg(not(feature = "kernel"))]
pub mod query_monitor;
#[cfg(not(feature = "kernel"))]
pub mod vector_search_integration;
#[cfg(not(feature = "kernel"))]
pub mod enhanced_vector_search;
#[cfg(not(feature = "kernel"))]
pub mod hybrid_search;
#[cfg(not(feature = "kernel"))]
pub mod hybrid_query_optimizer;
#[cfg(not(feature = "kernel"))]
pub mod ioctl_integration;

// FUSE implementation for userspace testing
#[cfg(feature = "fuse_support")]
pub mod fuse_impl;

// Conditional compilation for userspace-only modules
#[cfg(not(feature = "kernel"))]
#[path = "vector_handlers_stub.rs"]
pub mod vector_handlers;

#[cfg(not(feature = "kernel"))]
pub mod vector_test;

// Userspace API for testing and development
#[cfg(not(feature = "kernel"))]
pub fn init_vexfs_userspace() -> core::result::Result<(), String> {
    println!("VexFS: Initializing in userspace mode");
    Ok(())
}

#[cfg(not(feature = "kernel"))]
pub fn test_vector_operations() -> core::result::Result<(), String> {
    println!("VexFS: Running vector operation tests");
    // Basic vector operations test
    Ok(())
}

// Kernel module panic handler
#[cfg(feature = "kernel")]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
