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
//! This library provides userspace functionality for VexFS.
//! Kernel functionality is implemented via C FFI with the vexfs_module_entry.c file.

// Standard library for userspace operations
extern crate std;
use std::prelude::*;

// C FFI exports for integration with C kernel module
#[no_mangle]
pub extern "C" fn vexfs_rust_vector_search() -> std::os::raw::c_int {
    // C FFI function for vector search operations
    0
}

#[no_mangle]
pub extern "C" fn vexfs_rust_vector_storage() -> std::os::raw::c_int {
    // C FFI function for vector storage operations
    0
}

// Core userspace modules for VexFS
pub mod ondisk;
pub mod vector_storage;
pub mod vector_metrics;
pub mod knn_search;
pub mod result_scoring;
pub mod vector_search;
pub mod anns;

// Userspace vector handlers (stub implementation for testing)
#[path = "vector_handlers_stub.rs"]
pub mod vector_handlers;

// Userspace testing modules
pub mod vector_test;

// Userspace API for testing and development
pub fn init_vexfs_userspace() -> Result<(), String> {
    println!("VexFS: Initializing in userspace mode");
    Ok(())
}

pub fn test_vector_operations() -> Result<(), String> {
    println!("VexFS: Running vector operation tests");
    // Basic vector operations test
    Ok(())
}
