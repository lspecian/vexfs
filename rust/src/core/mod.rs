//! Core Module
//!
//! This module contains core filesystem functionality for VexFS, including:
//! - On-disk format and layout
//! - Inode management
//! - Journal and transaction management
//! - File and directory operations

// Core modules
pub mod ondisk;

// Kernel-only modules (use kernel bindings)
#[cfg(feature = "kernel")]
pub mod inode;
#[cfg(feature = "kernel")]
pub mod journal;
#[cfg(feature = "kernel")]
pub mod file_ops;
#[cfg(feature = "kernel")]
pub mod dir_ops;

// Re-exports for convenience
pub use ondisk::{OnDiskSerialize, S_IFMT, S_IFREG, S_IFDIR, S_IFCHR, S_IFBLK, S_IFIFO, S_IFSOCK, S_IFLNK};
// Note: InodeOps and JournalOps traits not yet implemented in kernel mode
// #[cfg(feature = "kernel")]
// pub use inode::InodeOps;
// #[cfg(feature = "kernel")]
// pub use journal::JournalOps;
