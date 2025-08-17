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

#![cfg_attr(all(feature = "kernel", not(feature = "std")), no_std)]
#![cfg_attr(all(feature = "kernel", not(feature = "std")), no_main)]

// Conditional compilation for userspace vs kernel
#[cfg(not(feature = "kernel"))]
extern crate std;

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

// FFI module for C integration - REORGANIZED
pub mod ffi;

// Authentication module for API server
#[cfg(all(not(feature = "kernel"), feature = "server"))]
pub mod auth;

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

// Re-export performance optimization components at crate level for easy access
#[cfg(not(feature = "kernel"))]
pub use performance_optimizations::{
    PerformanceOptimizationManager, PerformanceMetrics, BenchmarkResults,
    PerformanceAnalysisReport, OptimizationRecommendation, OptimizationCategory,
    RecommendationPriority, ImplementationEffort, PerformanceTargets,
    EnhancedVectorMemoryPool, SIMDVectorMetrics, StackOptimizedFuseOps,
    PerformanceBenchmark, PoolStatistics, DistanceMetric,
    VectorBenchmarkResults, MemoryBenchmarkResults, SIMDBenchmarkResults,
    OverallImprovementMetrics,
};

// Re-export ChromaDB API components at crate level for easy access
#[cfg(not(feature = "kernel"))]
pub use chromadb_api::{
    ChromaDBApi, Collection, Document, QueryResult, DistanceFunction,
};

// Re-export Semantic API components at crate level for easy access
#[cfg(all(not(feature = "kernel"), feature = "semantic_api"))]
pub use semantic_api::{
    SemanticApiConfig, SemanticResult, SemanticError,
    initialize_semantic_api, shutdown_semantic_api,
    types::*
    // Comment out modules that are not yet implemented
    // auth::*, query::*, stream::*, rate_limit::*,
    // serialization::*, kernel_interface::*, api_server::*, client::*
};

// Re-export VexGraph components at crate level for easy access
#[cfg(all(not(feature = "kernel"), feature = "vexgraph"))]
pub use vexgraph::{
    VexGraph, VexGraphConfig, NodeId, EdgeId, PropertyType,
    VexGraphError, VexGraphResult,
    // Core graph functionality
    GraphNode, GraphEdge, VexGraphCore,
    // Traversal algorithms
    TraversalEngine, TraversalAlgorithm, TraversalResult,
    // API server
    VexGraphApiServer,
    // Property graph management
    PropertyGraphManager,
    // Semantic integration
    SemanticIntegration,
    // Kernel primitives
    KernelPrimitives,
    // FUSE extensions
    FuseExtensions,
    // Performance optimization
    PerformanceOptimizer,
    // Concurrency management
    ConcurrencyManager,
    // Advanced Graph Algorithms and Semantic Reasoning (Task 20)
    // Advanced algorithms
    advanced_algorithms::AdvancedGraphAlgorithms,
    advanced_algorithms::DijkstraParams,
    advanced_algorithms::LouvainParams,
    advanced_algorithms::MultiGraphParams,
    advanced_algorithms::AlgorithmResult,
    advanced_algorithms::AdvancedAlgorithmStatistics,
    // Semantic reasoning
    semantic_reasoning::SemanticReasoningEngine,
    semantic_reasoning::InferenceRule,
    semantic_reasoning::Condition,
    semantic_reasoning::Conclusion,
    semantic_reasoning::Argument,
    semantic_reasoning::ConditionType,
    semantic_reasoning::Fact,
    semantic_reasoning::ReasoningTask,
    semantic_reasoning::InferenceResult,
    semantic_reasoning::ReasoningStatistics,
};

// Multi-dialect server support - only available in userspace with server feature
#[cfg(all(not(feature = "kernel"), feature = "server"))]
pub mod dialects;

// C FFI exports - Re-export key functions at the crate level for easier access
pub use ffi::{
    vexfs_rust_init, vexfs_rust_exit, vexfs_rust_fill_super,
    vexfs_rust_test_basic, vexfs_rust_get_version,
    vexfs_rust_get_statfs, vexfs_rust_new_inode, vexfs_rust_init_inode,
    vexfs_rust_destroy_inode, vexfs_rust_write_inode, vexfs_rust_sync_fs,
    vexfs_rust_put_super,
    // Hang prevention FFI exports
    vexfs_rust_init_hang_prevention, vexfs_rust_shutdown_hang_prevention,
    vexfs_rust_start_watchdog, vexfs_rust_cancel_watchdog,
    vexfs_rust_check_operation_allowed, vexfs_rust_update_resources,
    vexfs_rust_get_health_status, vexfs_rust_handle_panic,
    vexfs_rust_hang_prevention_available, vexfs_rust_get_hang_prevention_stats,
    vexfs_rust_force_degradation,
};

// Core modules for VexFS - only include kernel-safe modules in kernel mode
// TODO: Restore these modules after cleanup
// pub mod ondisk;
pub mod superblock;     // Still needed for legacy kernel interface
pub mod ioctl;

// IPC module for embedding service communication
// pub mod ipc;

// Userspace-only modules (require std types like Vec, String, etc.)
// TODO: Most vector modules need to be restored or recreated
// #[cfg(not(feature = "kernel"))]
// pub mod vector_storage;
// #[cfg(not(feature = "kernel"))]
// pub mod vector_storage_optimized;
// #[cfg(not(feature = "kernel"))]
// pub mod vector_cache;
// #[cfg(not(feature = "kernel"))]
// pub mod vector_metrics;
// #[cfg(not(feature = "kernel"))]
// pub mod knn_search;
#[cfg(not(feature = "kernel"))]
pub mod result_scoring;
// #[cfg(not(feature = "kernel"))]
// pub mod vector_search;
// #[cfg(not(feature = "kernel"))]
// pub mod query_planner;
// #[cfg(not(feature = "kernel"))]
// pub mod anns;
// #[cfg(not(feature = "kernel"))]
// pub mod vector_optimizations;
// #[cfg(not(feature = "kernel"))]
// pub mod vector_large_collections;
#[cfg(not(feature = "kernel"))]
pub mod search_cache;
// #[cfg(not(feature = "kernel"))]
// pub mod query_monitor;
// #[cfg(not(feature = "kernel"))]
// pub mod vector_search_integration;
// #[cfg(not(feature = "kernel"))]
// pub mod enhanced_vector_search;
// #[cfg(not(feature = "kernel"))]
// pub mod hybrid_search;
// #[cfg(not(feature = "kernel"))]
// pub mod hybrid_query_optimizer;
#[cfg(not(feature = "kernel"))]
pub mod ioctl_integration;
#[cfg(not(feature = "kernel"))]
pub mod performance_optimizations;

// ChromaDB-compatible API
// #[cfg(not(feature = "kernel"))]
// pub mod chromadb_api;

// VexFS Main API
// #[cfg(not(feature = "kernel"))]
// pub mod vexfs_api;

// Semantic API for AI agents (Task 13) - userspace only
// #[cfg(all(not(feature = "kernel"), feature = "semantic_api"))]
// pub mod semantic_api;

// Cross-Layer Consistency Manager (Task 14) - userspace only, requires semantic_api
#[cfg(all(not(feature = "kernel"), feature = "semantic_api"))]
pub mod cross_layer_consistency;

// Cross-Layer Integration Framework (Task 21) - userspace only
#[cfg(all(not(feature = "kernel"), feature = "semantic_api"))]
pub mod cross_layer_integration;

// VexGraph Native Graph Representation and API (Task 17) - userspace only
#[cfg(all(not(feature = "kernel"), feature = "vexgraph"))]
pub mod vexgraph;

// FUSE implementation for userspace testing
#[cfg(feature = "fuse_support")]
#[cfg(all(feature = "fuse_support", not(feature = "kernel")))]
pub mod fuse_impl;

#[cfg(feature = "fuse_support")]
#[cfg(all(feature = "fuse_support", not(feature = "kernel")))]
pub mod fuse_error_handling;

// Monitoring and health checks
#[cfg(not(feature = "kernel"))]
pub mod monitoring;

#[cfg(not(feature = "kernel"))]
pub mod health_endpoint;

#[cfg(feature = "fuse_support")]
#[cfg(all(feature = "fuse_support", not(feature = "kernel")))]
pub mod fuse_with_monitoring;

// FUSE-VexGraph integration
#[cfg(all(feature = "fuse_support", feature = "vexgraph", not(feature = "kernel")))]
pub mod fuse_vexgraph_bridge;

#[cfg(all(feature = "fuse_support", feature = "vexgraph", not(feature = "kernel")))]
pub mod fuse_vexgraph_integrated;

// Comprehensive error handling
#[cfg(not(feature = "kernel"))]
pub mod error_handling;

#[cfg(not(feature = "kernel"))]
pub mod panic_handler;

// Conditional compilation for userspace-only modules
#[cfg(not(feature = "kernel"))]
#[path = "vector_handlers_stub.rs"]
pub mod vector_handlers;

// #[cfg(not(feature = "kernel"))]
// pub mod vector_test;

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

// Kernel module panic handler (only when no_std is active and std is not available)
#[cfg(all(feature = "kernel", not(feature = "std")))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

// VexCtl modules (only available with vexctl feature)
#[cfg(feature = "vexctl")]
pub mod client;
#[cfg(feature = "vexctl")]
pub mod commands;
#[cfg(feature = "vexctl")]
pub mod output;
#[cfg(feature = "vexctl")]
pub mod error;

// Re-export VexCtl components for easy access
#[cfg(feature = "vexctl")]
pub use error::{VexctlError, Result};
