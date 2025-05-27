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

//! VexFS Shared Domain
//!
//! This module contains shared components that are used across all other domains in VexFS.
//! It provides the foundational types, errors, constants, utilities, configuration, and macros
//! that enable consistent behavior throughout the filesystem implementation.
//!
//! The shared domain is designed to work in both kernel and userspace environments,
//! ensuring compatibility across different deployment scenarios.

// =============================================================================
// Module Declarations
// =============================================================================

pub mod constants;
pub mod errors;
pub mod types;
pub mod utils;
pub mod config;

#[macro_use]
pub mod macros;

// =============================================================================
// Public Re-exports
// =============================================================================

// Core error types and result handling
pub use errors::{
    VexfsError, VexfsResult, IoErrorKind, VectorErrorKind, IndexErrorKind,
    JournalErrorKind,
};

// Essential type definitions
pub use types::{
    // Primitive types
    BlockNumber, InodeNumber, FileOffset, FileSize, Timestamp,
    
    // Filesystem types
    FileType, FileMode, FilePermissions, DirectoryEntry, InodeFlags,
    
    // Vector types
    VectorId, VectorDimensions, SimilarityMetric, VectorData, VectorMetadata,
    SearchResult,
    
    // Configuration types
    NodeId, TransactionId, SequenceNumber,
    
    // Result type
    Result,
};

// Fundamental constants
pub use constants::{
    // Magic numbers and identifiers
    VEXFS_MAGIC, VEXFS_VERSION_MAJOR, VEXFS_VERSION_MINOR, VEXFS_VERSION_PATCH,
    
    // Size and limit constants
    VEXFS_DEFAULT_BLOCK_SIZE, VEXFS_MAX_FILE_SIZE, VEXFS_MAX_NAME_LEN,
    VEXFS_MAX_NAME_LENGTH, VEXFS_MAX_PATH_LEN, VEXFS_MAX_VECTOR_DIMS,
    
    // Alignment constants
    VEXFS_DEFAULT_ALIGNMENT, VEXFS_CACHE_LINE_SIZE,
    
    // Default values
    VEXFS_DEFAULT_FILE_MODE, VEXFS_DEFAULT_DIR_MODE,
};

// Utility functions
pub use utils::{
    // Path utilities
    normalize_path, parent_path, filename, join_paths, validate_filename,
    
    // Alignment utilities  
    align_up, align_down, is_aligned, next_power_of_2, is_power_of_2,
    
    // Block utilities
    offset_to_block, offset_in_block, blocks_for_size, block_to_offset,
    
    // Checksum utilities
    crc32, verify_checksum, checksum_struct,
    
    // Time utilities
    current_timestamp, timestamp_to_secs, timestamp_to_nsecs, should_update_atime,
    
    // Math utilities
    min, max, clamp, gcd, lcm,
    
    // Vector utilities
    euclidean_distance, cosine_similarity, dot_product, manhattan_distance, normalize_vector,
    
    // Memory utilities
    safe_copy_memory, safe_zero_memory, safe_compare_memory,
};

// Configuration system
pub use config::{
    VexfsConfig, VexfsConfigBuilder,
    FilesystemConfig, VectorConfig, CacheConfig, IoConfig,
    JournalConfig, MemoryConfig, DebugConfig, HnswConfig,
    CacheReplacementPolicy, CacheWritePolicy, JournalFlushPolicy, LogLevel,
};

// =============================================================================
// Shared Domain API
// =============================================================================

/// Initialize the shared domain with the given configuration
pub fn init_shared_domain(config: VexfsConfig) -> VexfsResult<()> {
    vexfs_info!("Initializing VexFS Shared Domain v{}.{}.{}", 
               VEXFS_VERSION_MAJOR, VEXFS_VERSION_MINOR, VEXFS_VERSION_PATCH);
    
    // Validate configuration
    config.filesystem.validate()?;
    config.vector.hnsw.validate()?;
    
    // Initialize global configuration (userspace only)
    #[cfg(not(feature = "kernel"))]
    {
        crate::shared::config::init_config(config)?;
    }
    
    vexfs_info!("Shared domain initialized successfully");
    Ok(())
}

/// Get the shared domain version
pub fn get_version() -> (u32, u32, u32) {
    (VEXFS_VERSION_MAJOR, VEXFS_VERSION_MINOR, VEXFS_VERSION_PATCH)
}

/// Check if the shared domain is compatible with a given version
pub fn is_version_compatible(major: u32, minor: u32, _patch: u32) -> bool {
    // Major version must match exactly
    if major != VEXFS_VERSION_MAJOR {
        return false;
    }
    
    // Minor version must be <= current (backward compatibility)
    if minor > VEXFS_VERSION_MINOR {
        return false;
    }
    
    // Patch version can be different (forward/backward compatible)
    true
}

/// Validate a magic number
pub fn validate_magic(magic: u64) -> VexfsResult<()> {
    if magic != VEXFS_MAGIC {
        return Err(VexfsError::InvalidMagic);
    }
    Ok(())
}

/// Create a default configuration suitable for the current environment
pub fn default_config_for_environment() -> VexfsConfig {
    let mut config = VexfsConfig::default();
    
    // Adjust configuration based on environment
    #[cfg(feature = "kernel")]
    {
        // Kernel environment: more conservative settings
        config.memory.pools_enabled = true;
        config.memory.debug_enabled = false;
        config.cache.page_cache_size = 32 * 1024 * 1024; // 32MB
        config.io.async_io_enabled = false; // Simpler I/O in kernel
        config.debug.logging_enabled = false;
    }
    
    #[cfg(not(feature = "kernel"))]
    {
        // Userspace environment: more aggressive settings
        config.memory.pools_enabled = false; // Use system allocator
        config.memory.debug_enabled = cfg!(debug_assertions);
        config.cache.page_cache_size = 128 * 1024 * 1024; // 128MB
        config.io.async_io_enabled = true;
        config.debug.logging_enabled = cfg!(debug_assertions);
    }
    
    config
}

/// Perform self-tests of the shared domain
#[cfg(test)]
pub fn self_test() -> VexfsResult<()> {
    vexfs_info!("Running shared domain self-tests");
    
    // Test basic functionality
    test_basic_types()?;
    test_error_handling()?;
    test_utility_functions()?;
    test_configuration()?;
    
    vexfs_info!("Shared domain self-tests passed");
    Ok(())
}

#[cfg(test)]
fn test_basic_types() -> VexfsResult<()> {
    // Test that basic types work as expected
    let block_num: BlockNumber = 42;
    let inode_num: InodeNumber = 123;
    let offset: FileOffset = 4096;
    
    assert!(block_num > 0);
    assert!(inode_num > 0);
    assert!(offset >= 0);
    
    Ok(())
}

#[cfg(test)]
fn test_error_handling() -> VexfsResult<()> {
    // Test error creation and conversion
    let error = VexfsError::NotFound;
    let result: VexfsResult<()> = Err(error);
    
    assert!(result.is_err());
    
    Ok(())
}

#[cfg(test)]
fn test_utility_functions() -> VexfsResult<()> {
    // Test path utilities
    let normalized = normalize_path("/a/b/../c");
    assert_eq!(normalized, "/a/c");
    
    // Test alignment utilities
    assert_eq!(align_up(10, 8), 16);
    assert!(is_aligned(16, 8));
    
    // Test block utilities
    assert_eq!(offset_to_block(4096, 4096), 1);
    
    // Test checksum
    let data = b"test data";
    let checksum = crc32(data);
    assert!(verify_checksum(data, checksum));
    
    Ok(())
}

#[cfg(test)]
fn test_configuration() -> VexfsResult<()> {
    // Test configuration creation and validation
    let config = VexfsConfigBuilder::new()
        .filesystem(FilesystemConfig::default())
        .build()?;
    
    assert_eq!(config.filesystem.block_size, VEXFS_DEFAULT_BLOCK_SIZE as u32);
    
    Ok(())
}

// =============================================================================
// Feature Gates and Conditional Compilation
// =============================================================================

/// Features available in the shared domain
pub struct SharedFeatures {
    pub vector_support: bool,
    pub encryption_support: bool,
    pub compression_support: bool,
    pub kernel_mode: bool,
    pub debug_features: bool,
}

impl SharedFeatures {
    /// Get the current feature set
    pub fn current() -> Self {
        Self {
            vector_support: true, // Always available
            encryption_support: cfg!(feature = "encryption"),
            compression_support: cfg!(feature = "compression"),
            kernel_mode: cfg!(feature = "kernel"),
            debug_features: cfg!(debug_assertions),
        }
    }
}

/// Get current feature configuration
pub fn get_features() -> SharedFeatures {
    SharedFeatures::current()
}

// =============================================================================
// Version and Build Information
// =============================================================================

/// Build information for the shared domain
pub struct BuildInfo {
    pub version: (u32, u32, u32),
    pub build_timestamp: &'static str,
    pub git_commit: &'static str,
    pub rust_version: &'static str,
    pub target: &'static str,
    pub features: SharedFeatures,
}

impl BuildInfo {
    /// Get build information
    pub fn get() -> Self {
        Self {
            version: get_version(),
            build_timestamp: "unknown",
            git_commit: "unknown",
            rust_version: "unknown",
            target: "unknown",
            features: SharedFeatures::current(),
        }
    }
}

/// Get build information
pub fn get_build_info() -> BuildInfo {
    BuildInfo::get()
}

// =============================================================================
// Shared Domain Statistics
// =============================================================================

/// Runtime statistics for the shared domain
#[derive(Debug, Default)]
pub struct SharedStats {
    pub allocations: u64,
    pub deallocations: u64,
    pub checksum_verifications: u64,
    pub checksum_failures: u64,
    pub path_normalizations: u64,
    pub config_updates: u64,
}

impl SharedStats {
    /// Reset all statistics
    pub fn reset(&mut self) {
        *self = Self::default();
    }
    
    /// Get memory usage statistics
    pub fn memory_usage(&self) -> i64 {
        self.allocations as i64 - self.deallocations as i64
    }
    
    /// Get checksum success rate
    pub fn checksum_success_rate(&self) -> f64 {
        if self.checksum_verifications == 0 {
            1.0
        } else {
            1.0 - (self.checksum_failures as f64 / self.checksum_verifications as f64)
        }
    }
}

// Global statistics instance (userspace only)
#[cfg(not(feature = "kernel"))]
static mut GLOBAL_STATS: SharedStats = SharedStats {
    allocations: 0,
    deallocations: 0,
    checksum_verifications: 0,
    checksum_failures: 0,
    path_normalizations: 0,
    config_updates: 0,
};

/// Get global statistics
#[cfg(not(feature = "kernel"))]
pub fn get_stats() -> &'static SharedStats {
    unsafe { &GLOBAL_STATS }
}

/// Update global statistics
#[cfg(not(feature = "kernel"))]
pub fn update_stats<F>(f: F) 
where 
    F: FnOnce(&mut SharedStats),
{
    unsafe {
        f(&mut GLOBAL_STATS);
    }
}

/// Reset global statistics
#[cfg(not(feature = "kernel"))]
pub fn reset_stats() {
    unsafe {
        GLOBAL_STATS.reset();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_compatibility() {
        // Same version should be compatible
        assert!(is_version_compatible(VEXFS_VERSION_MAJOR, VEXFS_VERSION_MINOR, VEXFS_VERSION_PATCH));
        
        // Different major version should not be compatible
        assert!(!is_version_compatible(VEXFS_VERSION_MAJOR + 1, VEXFS_VERSION_MINOR, VEXFS_VERSION_PATCH));
        
        // Older minor version should be compatible
        if VEXFS_VERSION_MINOR > 0 {
            assert!(is_version_compatible(VEXFS_VERSION_MAJOR, VEXFS_VERSION_MINOR - 1, VEXFS_VERSION_PATCH));
        }
        
        // Newer minor version should not be compatible
        assert!(!is_version_compatible(VEXFS_VERSION_MAJOR, VEXFS_VERSION_MINOR + 1, VEXFS_VERSION_PATCH));
    }

    #[test]
    fn test_magic_validation() {
        assert!(validate_magic(VEXFS_MAGIC).is_ok());
        assert!(validate_magic(VEXFS_MAGIC + 1).is_err());
    }

    #[test]
    fn test_default_config() {
        let config = default_config_for_environment();
        assert!(config.filesystem.validate().is_ok());
        assert!(config.vector.hnsw.validate().is_ok());
    }

    #[test]
    fn test_features() {
        let features = get_features();
        assert!(features.vector_support);
    }

    #[test]
    fn test_build_info() {
        let build_info = get_build_info();
        assert_eq!(build_info.version, get_version());
    }

    #[test]
    fn test_stats() {
        #[cfg(not(feature = "kernel"))]
        {
            reset_stats();
            let stats = get_stats();
            assert_eq!(stats.allocations, 0);
            
            update_stats(|s| s.allocations += 1);
            let stats = get_stats();
            assert_eq!(stats.allocations, 1);
        }
    }

    #[test]
    fn test_self_test() {
        assert!(self_test().is_ok());
    }
}