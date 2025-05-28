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

//! FS Core Domain
//!
//! This module provides the core filesystem functionality including file and directory
//! operations, inode management, path resolution, permissions, and locking mechanisms.
//! It represents the heart of the VexFS filesystem implementation using Domain-Driven
//! Design principles.

pub mod file;
pub mod directory;
pub mod inode;
pub mod path;
pub mod permissions;
pub mod operations;
pub mod locking;
pub mod enhanced_operation_context;
pub mod cow;
pub mod snapshot;
pub mod cow_integration;
pub mod cow_vector_integration;
pub mod cow_garbage_collection;

// Re-export commonly used types and functions
pub use file::{File, FileOperations};
pub use directory::{Directory, DirectoryOperations, DirectoryEntry};
pub use inode::{Inode, InodeManager};
pub use path::{Path, PathComponent, PathResolver, PathValidator};
pub use permissions::{
    AccessMode, UserContext, AccessCheck, PermissionChecker, SecurityPolicy
};
pub use operations::{FilesystemOperations, OperationContext};
pub use enhanced_operation_context::{
    EnhancedOperationContext, OperationMetadata, OperationType, CancellationToken,
    CancellationReason, TimeoutConfig, TimeoutAction, TelemetryCollector,
    TelemetryEvent, TelemetryEventType, TelemetrySeverity, ProgressReporter,
    ResourceTracker, LifecycleHooks, OperationPriority, ResourceLimits,
    ResourceUsageSummary
};
pub use locking::{
    LockType, LockScope, LockManager, FileLock, DirectoryLock, LockGuard
};
pub use cow::{
    CowManager, CowMapping, CowExtent, CowBlockRef, CowStats
};
pub use snapshot::{
    SnapshotManager, SnapshotMetadata, SnapshotId, SnapshotStats,
    DeltaStorage, DeltaEntry, GarbageCollectionResult
};
pub use cow_integration::{
    CowFilesystemOperations, CowSnapshotIntegration, CowConfig,
    CowFilesystemStats, GarbageCollectionSummary, OptimizationResult
};
pub use cow_vector_integration::{
    VectorCowManager, VectorCowMapping, VectorCowStats, VectorOptimizationResult
};

pub use cow_garbage_collection::{
    CowGarbageCollector, GcConfig, GcStats, CompactionResult
};

use crate::shared::{
    errors::VexfsError,
    types::*,
};
use crate::shared::constants::{
    VEXFS_BLOCK_SIZE, VEXFS_VERSION_MAJOR, VEXFS_VERSION_MINOR, VEXFS_VERSION_PATCH,
    VEXFS_MAGIC, VEXFS_ROOT_INO
};

// Create a combined version constant
pub const VEXFS_VERSION: u32 = ((VEXFS_VERSION_MAJOR as u32) << 16) |
                               ((VEXFS_VERSION_MINOR as u32) << 8) |
                               (VEXFS_VERSION_PATCH as u32);

// Use the shared Result type directly
use crate::shared::types::Result as FsResult;

/// File system statistics
#[derive(Debug, Clone)]
pub struct FsStats {
    pub total_inodes: u64,
    pub free_inodes: u64,
    pub total_blocks: u64,
    pub free_blocks: u64,
    pub files: u64,
    pub directories: u64,
    pub symlinks: u64,
    pub block_size: u32,
}

impl FsStats {
    pub fn new() -> Self {
        Self {
            total_inodes: 0,
            free_inodes: 0,
            total_blocks: 0,
            free_blocks: 0,
            files: 0,
            directories: 0,
            symlinks: 0,
            block_size: VEXFS_BLOCK_SIZE as u32,
        }
    }

    pub fn used_inodes(&self) -> u64 {
        self.total_inodes.saturating_sub(self.free_inodes)
    }

    pub fn used_blocks(&self) -> u64 {
        self.total_blocks.saturating_sub(self.free_blocks)
    }

    pub fn inode_utilization_percent(&self) -> f64 {
        if self.total_inodes == 0 {
            0.0
        } else {
            (self.used_inodes() as f64 / self.total_inodes as f64) * 100.0
        }
    }

    pub fn block_utilization_percent(&self) -> f64 {
        if self.total_blocks == 0 {
            0.0
        } else {
            (self.used_blocks() as f64 / self.total_blocks as f64) * 100.0
        }
    }
}

/// File system configuration
#[derive(Debug, Clone)]
pub struct FsConfig {
    pub case_sensitive: bool,
    pub max_filename_length: usize,
    pub max_path_depth: usize,
    pub enable_quotas: bool,
    pub enable_extended_attributes: bool,
    pub default_file_mode: u16,
    pub default_dir_mode: u16,
}

impl Default for FsConfig {
    fn default() -> Self {
        Self {
            case_sensitive: true,
            max_filename_length: 255,
            max_path_depth: 32,
            enable_quotas: false,
            enable_extended_attributes: true,
            default_file_mode: 0o644,
            default_dir_mode: 0o755,
        }
    }
}

/// Main filesystem context that coordinates all operations
pub struct FileSystem {
    pub inode_manager: InodeManager,
    pub lock_manager: LockManager,
    pub config: FsConfig,
    pub stats: FsStats,
}

impl FileSystem {
    /// Create a new filesystem instance
    pub fn new(storage_manager: crate::storage::StorageManager, config: FsConfig) -> FsResult<Self> {
        let inode_manager = InodeManager::new(storage_manager)?;
        let lock_manager = LockManager::new();
        let stats = FsStats::new();

        Ok(Self {
            inode_manager,
            lock_manager,
            config,
            stats,
        })
    }

    /// Mount the filesystem
    pub fn mount(&mut self) -> FsResult<()> {
        // Initialize filesystem components
        // TODO: Add proper initialization when InodeManager supports it
        
        // Update statistics
        self.update_stats()?;
        
        Ok(())
    }

    /// Unmount the filesystem
    pub fn unmount(&mut self) -> FsResult<()> {
        // Flush all pending operations
        self.inode_manager.sync()?;
        
        // Release all locks
        // Note: In a real implementation, we'd need to track lock owners
        // and release locks for the unmounting process
        
        Ok(())
    }

    /// Update filesystem statistics
    pub fn update_stats(&mut self) -> FsResult<()> {
        // Get statistics from inode manager
        let (_cached_inodes, _dirty_inodes) = self.inode_manager.cache_stats();
        
        // TODO: Get actual storage statistics when available
        self.stats.total_blocks = 1000000; // Placeholder
        self.stats.free_blocks = 800000;   // Placeholder
        self.stats.total_inodes = 100000;  // Placeholder
        self.stats.free_inodes = 90000;    // Placeholder
        
        // Count file types (this would be expensive for large filesystems)
        // In practice, these might be cached or updated incrementally
        
        Ok(())
    }

    /// Get filesystem statistics
    pub fn get_stats(&self) -> &FsStats {
        &self.stats
    }

    /// Get filesystem configuration
    pub fn get_config(&self) -> &FsConfig {
        &self.config
    }

    /// Update filesystem configuration
    pub fn update_config(&mut self, new_config: FsConfig) -> FsResult<()> {
        // Validate configuration changes
        if new_config.max_filename_length > 4096 {
            return Err(VexfsError::InvalidArgument("Filename length too large".into()));
        }
        
        if new_config.max_path_depth > 256 {
            return Err(VexfsError::InvalidArgument("Path depth too large".into()));
        }
        
        self.config = new_config;
        Ok(())
    }

    /// Create a new operation context
    pub fn create_operation_context(&mut self, user: UserContext) -> OperationContext {
        OperationContext::new(
            user,
            VEXFS_ROOT_INO, // Use root as default cwd
            &mut self.inode_manager,
            &mut self.lock_manager,
        )
    }

    /// Perform filesystem check and repair
    pub fn fsck(&mut self, repair: bool) -> FsResult<Vec<String>> {
        let mut issues = Vec::new();
        
        // TODO: Implement filesystem check when InodeManager supports verification
        // For now, just return basic cache statistics
        let (_cached_inodes, dirty_inodes) = self.inode_manager.cache_stats();
        
        if dirty_inodes > 0 {
            issues.push(format!("Found {} dirty inodes in cache", dirty_inodes));
            if repair {
                // Sync dirty inodes
                if let Err(e) = self.inode_manager.sync() {
                    issues.push(format!("Failed to sync dirty inodes: {}", e));
                } else {
                    issues.push("Successfully synced dirty inodes".to_string());
                }
            }
        }
        
        // Check directory structure integrity
        // This would involve walking the directory tree and validating entries
        
        // Check block allocation consistency
        // This would verify that allocated blocks match the allocation bitmap
        
        Ok(issues)
    }

    /// Sync filesystem to storage
    pub fn sync(&mut self) -> FsResult<()> {
        self.inode_manager.sync()?;
        Ok(())
    }

    /// Get detailed filesystem information
    pub fn get_fs_info(&self) -> FsInfo {
        FsInfo {
            version: VEXFS_VERSION,
            magic: VEXFS_MAGIC,
            block_size: VEXFS_BLOCK_SIZE as u32,
            inode_size: core::mem::size_of::<crate::ondisk::VexfsInode>() as u32,
            features: self.get_feature_flags(),
            stats: self.stats.clone(),
            config: self.config.clone(),
        }
    }

    /// Get feature flags
    fn get_feature_flags(&self) -> u64 {
        let mut flags = 0u64;
        
        if self.config.enable_extended_attributes {
            flags |= 0x1; // Extended attributes
        }
        
        if self.config.enable_quotas {
            flags |= 0x2; // Quotas
        }
        
        // Add other feature flags as needed
        flags |= 0x4; // Vector storage support
        flags |= 0x8; // Advanced indexing
        
        flags
    }
}

/// Comprehensive filesystem information
#[derive(Debug, Clone)]
pub struct FsInfo {
    pub version: u32,
    pub magic: u32,
    pub block_size: u32,
    pub inode_size: u32,
    pub features: u64,
    pub stats: FsStats,
    pub config: FsConfig,
}

/// Trait for filesystem initialization
pub trait FsInitializer {
    /// Initialize a new filesystem
    fn mkfs(&mut self, config: &FsConfig) -> FsResult<()>;
    
    /// Check if filesystem exists and is valid
    fn check_fs(&self) -> FsResult<bool>;
}

impl FsInitializer for FileSystem {
    fn mkfs(&mut self, config: &FsConfig) -> FsResult<()> {
        // TODO: Initialize storage layer when InodeManager supports it
        
        // Create root directory using the available create_inode method
        let root_inode_arc = self.inode_manager.create_inode(
            crate::shared::types::FileType::Directory,
            crate::shared::types::FileMode::new(0o755),
            0, // root uid
            0  // root gid
        )?;
        
        if root_inode_arc.ino != VEXFS_ROOT_INO {
            return Err(VexfsError::InvalidOperation("Root directory has wrong inode number".to_string()));
        }
        
        // Update configuration
        self.config = config.clone();
        
        // Initialize statistics
        self.update_stats()?;
        
        Ok(())
    }
    
    fn check_fs(&self) -> FsResult<bool> {
        // TODO: Check if superblock is valid when InodeManager supports verification
        // For now, just check if we can access the cache stats
        let (_cached, _dirty) = self.inode_manager.cache_stats();
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::StorageManager;

    #[test]
    fn test_fs_stats() {
        let mut stats = FsStats::new();
        stats.total_inodes = 1000;
        stats.free_inodes = 800;
        stats.total_blocks = 10000;
        stats.free_blocks = 8000;
        
        assert_eq!(stats.used_inodes(), 200);
        assert_eq!(stats.used_blocks(), 2000);
        assert_eq!(stats.inode_utilization_percent(), 20.0);
        assert_eq!(stats.block_utilization_percent(), 20.0);
    }

    #[test]
    fn test_fs_config() {
        let config = FsConfig::default();
        assert!(config.case_sensitive);
        assert_eq!(config.max_filename_length, 255);
        assert_eq!(config.default_file_mode, 0o644);
        assert_eq!(config.default_dir_mode, 0o755);
    }

    #[test]
    fn test_feature_flags() {
        let config = FsConfig {
            enable_extended_attributes: true,
            enable_quotas: true,
            ..Default::default()
        };
        
        // Create a mock storage manager for testing
        // In real code, this would be a proper StorageManager instance
        // let storage_manager = StorageManager::new_mock();
        // let fs = FileSystem::new(storage_manager, config).unwrap();
        // let flags = fs.get_feature_flags();
        // assert_eq!(flags & 0x3, 0x3); // Both features enabled
    }
}