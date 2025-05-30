//! VexFS Kernel Module Testing Library
//!
//! This library provides testing infrastructure for VexFS kernel module
//! validation across multiple levels of testing complexity.

pub mod level1_basic_validation;
pub mod level2_vm_mount_operations;

pub use level1_basic_validation::*;
pub use level2_vm_mount_operations::*;