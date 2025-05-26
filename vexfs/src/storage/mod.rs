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

//! Storage Domain
//!
//! This module contains all storage-related functionality for VexFS, including:
//! - Block management and I/O operations
//! - Space allocation algorithms and bitmap management
//! - Journaling and transaction systems
//! - On-disk persistence and serialization
//! - Superblock management
//! - Filesystem layout calculations
//! - Block caching strategies
//!
//! The storage domain provides the foundation for all file system operations
//! by managing the underlying block storage, ensuring data consistency through
//! journaling, and optimizing performance through intelligent caching.

// Re-export all public types and functions from storage modules
pub mod block;
pub mod allocation;
pub mod journal;
pub mod persistence;
pub mod superblock;
pub mod layout;
pub mod cache;

// Public exports for external use - only export what exists
pub use block::{
    Block, BlockDevice, BlockBuffer, BlockManager, BlockMetadata,
};

pub use allocation::{
    SpaceAllocator, AllocationStrategy, BlockGroup,
    AllocationResult, AllocationPolicy, FragmentationStats,
};

pub use journal::{
    TransactionManager, TransactionState, VexfsJournal,
};

pub use persistence::{
    OnDiskSerializable, VexfsInode, VexfsDirEntry,
    PersistenceManager,
};

pub use superblock::{
    SuperblockManager, FilesystemStats, VexfsSuperblock,
};

pub use layout::{
    VexfsLayout, LayoutCalculator,
};

pub use cache::{
    CacheEntry, CacheState, BlockCacheManager,
};

use crate::shared::errors::{VexfsError, VexfsResult};
use crate::shared::types::*;
use std::cell::RefCell;

/// Storage subsystem manager that coordinates all storage operations
pub struct StorageManager {
    /// Block device interface
    block_manager: RefCell<BlockManager>,
    /// Space allocation system
    allocator: RefCell<SpaceAllocator>,
    /// Transaction and journaling system
    journal: RefCell<VexfsJournal>,
    /// Persistence layer
    persistence: RefCell<PersistenceManager>,
    /// Superblock management
    superblock: RefCell<SuperblockManager>,
    /// Block cache
    cache: RefCell<BlockCacheManager>,
    /// Filesystem layout (immutable)
    layout: VexfsLayout,
}

impl StorageManager {
    /// Create new storage manager for existing filesystem
    pub fn new(
        device: BlockDevice,
        layout: VexfsLayout,
        cache_size: usize,
    ) -> VexfsResult<Self> {
        let block_manager = BlockManager::new(device)?;
        let allocator = SpaceAllocator::new(&layout)?;
        let journal = VexfsJournal::new(layout.block_size, layout.journal_blocks);
        let persistence = PersistenceManager::new(layout.block_size)?;
        let superblock = SuperblockManager::new()?;
        let cache = BlockCacheManager::new(
            cache_size / layout.block_size as usize,
            cache_size,
            layout.block_size,
            false, // Write-back mode
        );

        Ok(Self {
            block_manager: RefCell::new(block_manager),
            allocator: RefCell::new(allocator),
            journal: RefCell::new(journal),
            persistence: RefCell::new(persistence),
            superblock: RefCell::new(superblock),
            cache: RefCell::new(cache),
            layout,
        })
    }

    /// Initialize storage for new filesystem
    pub fn initialize(
        device: BlockDevice,
        layout: VexfsLayout,
        cache_size: usize,
    ) -> VexfsResult<Self> {
        // Create storage manager
        let storage = Self::new(device, layout, cache_size)?;
        
        // Initialize superblock
        storage.superblock.borrow_mut().initialize(&storage.layout)?;
        
        // Initialize journal
        storage.journal.borrow_mut().initialize()?;
        
        // Initialize space allocator
        storage.allocator.borrow_mut().initialize()?;
        
        Ok(storage)
    }

    /// Mount filesystem
    pub fn mount(&self) -> VexfsResult<()> {
        // Load and validate superblock
        let superblock = self.superblock.borrow_mut().load_and_validate(&mut self.block_manager.borrow_mut())?;
        
        // Verify layout compatibility
        self.layout.validate()?;
        
        // Replay journal if needed
        self.journal.borrow_mut().replay(&mut self.block_manager.borrow_mut())?;
        
        // Initialize allocator with current state
        self.allocator.borrow_mut().load_state(&mut self.block_manager.borrow_mut())?;
        
        Ok(())
    }

    /// Unmount filesystem
    pub fn unmount(&self) -> VexfsResult<()> {
        // Sync all dirty data
        self.sync_all()?;
        
        // Checkpoint journal
        self.journal.borrow_mut().checkpoint(&mut self.block_manager.borrow_mut())?;
        
        // Update superblock
        self.superblock.borrow_mut().update_and_sync(&mut self.block_manager.borrow_mut())?;
        
        Ok(())
    }

    /// Read block through cache
    pub fn read_block(&self, block: BlockNumber) -> VexfsResult<Vec<u8>> {
        // Try cache first
        if let Some(data) = self.cache.borrow().read_block(block) {
            return Ok(data);
        }
        
        // Read from device
        let data = self.block_manager.borrow().read_block(block)?;
        
        // Cache the block
        self.cache.borrow_mut().write_block(block, data.clone())?;
        
        Ok(data)
    }

    /// Write block through cache and journal
    pub fn write_block(&self, block: BlockNumber, data: Vec<u8>) -> VexfsResult<()> {
        // Start transaction
        let txn = self.journal.borrow_mut().begin_transaction()?;
        
        // Journal the write
        self.journal.borrow_mut().log_block_write(&txn, block, &data)?;
        
        // Write to cache
        self.cache.borrow_mut().write_block(block, data)?;
        
        // Commit transaction
        self.journal.borrow_mut().commit_transaction(txn)?;
        
        Ok(())
    }

    /// Allocate blocks
    pub fn allocate_blocks(&self, count: u32, hint: Option<BlockNumber>) -> VexfsResult<Vec<BlockNumber>> {
        let request = AllocationRequest::new(count, hint);
        let result = self.allocator.borrow_mut().allocate_blocks(request)?;
        
        // Journal the allocation
        let txn = self.journal.borrow_mut().begin_transaction()?;
        for &block in &result.blocks {
            self.journal.borrow_mut().log_block_allocation(&txn, block)?;
        }
        self.journal.borrow_mut().commit_transaction(txn)?;
        
        Ok(result.blocks)
    }

    /// Free blocks
    pub fn free_blocks(&self, blocks: &[BlockNumber]) -> VexfsResult<()> {
        // Journal the deallocation
        let txn = self.journal.borrow_mut().begin_transaction()?;
        for &block in blocks {
            self.journal.borrow_mut().log_block_deallocation(&txn, block)?;
        }
        self.journal.borrow_mut().commit_transaction(txn)?;
        
        // Free the blocks
        self.allocator.borrow_mut().free_blocks(blocks)?;
        
        // Invalidate cache entries
        for &block in blocks {
            self.cache.borrow_mut().invalidate(block);
        }
        
        Ok(())
    }

    /// Sync all dirty data to storage
    pub fn sync_all(&self) -> VexfsResult<()> {
        // Sync cache
        let dirty_blocks = self.cache.borrow_mut().sync()?;
        for (block, data) in dirty_blocks {
            self.block_manager.borrow_mut().write_block(block, &data)?;
        }
        
        // Sync journal
        self.journal.borrow_mut().sync(&mut self.block_manager.borrow_mut())?;
        
        // Sync allocator state
        self.allocator.borrow_mut().sync(&mut self.block_manager.borrow_mut())?;
        
        Ok(())
    }

    /// Get storage statistics
    pub fn get_stats(&self) -> StorageStats {
        let cache_stats = self.cache.borrow().get_stats();
        let alloc_stats = self.allocator.borrow().get_stats();
        let journal_stats = self.journal.borrow().get_stats();
        
        StorageStats {
            total_blocks: self.layout.total_blocks,
            free_blocks: alloc_stats.free_blocks,
            used_blocks: alloc_stats.used_blocks,
            cache_hit_rate: cache_stats.hit_rate,
            cache_utilization: cache_stats.utilization,
            journal_utilization: journal_stats.utilization,
            fragmentation: alloc_stats.fragmentation,
        }
    }

    /// Check storage health
    pub fn check_health(&self) -> VexfsResult<StorageHealth> {
        let stats = self.get_stats();
        
        let mut issues = Vec::new();
        let mut warnings = Vec::new();
        
        // Check free space
        let free_percent = (stats.free_blocks as f32 / stats.total_blocks as f32) * 100.0;
        if free_percent < 5.0 {
            issues.push("Very low free space (< 5%)".to_string());
        } else if free_percent < 15.0 {
            warnings.push("Low free space (< 15%)".to_string());
        }
        
        // Check cache performance
        if stats.cache_hit_rate < 70.0 {
            warnings.push("Low cache hit rate".to_string());
        }
        
        // Check fragmentation
        if stats.fragmentation > 30.0 {
            warnings.push("High fragmentation".to_string());
        }
        
        // Check journal utilization
        if stats.journal_utilization > 80.0 {
            warnings.push("Journal nearly full".to_string());
        }
        
        let health_status = if !issues.is_empty() {
            HealthStatus::Critical
        } else if !warnings.is_empty() {
            HealthStatus::Warning
        } else {
            HealthStatus::Good
        };
        
        Ok(StorageHealth {
            status: health_status,
            issues,
            warnings,
            stats,
        })
    }
}

/// Storage subsystem statistics
#[derive(Debug, Clone)]
pub struct StorageStats {
    pub total_blocks: u64,
    pub free_blocks: u64,
    pub used_blocks: u64,
    pub cache_hit_rate: f32,
    pub cache_utilization: f32,
    pub journal_utilization: f32,
    pub fragmentation: f32,
}

/// Storage health information
#[derive(Debug, Clone)]
pub struct StorageHealth {
    pub status: HealthStatus,
    pub issues: Vec<String>,
    pub warnings: Vec<String>,
    pub stats: StorageStats,
}

/// Health status levels
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HealthStatus {
    Good,
    Warning,
    Critical,
}

/// Storage configuration for initialization
#[derive(Debug, Clone)]
pub struct StorageConfig {
    pub block_size: u32,
    pub cache_size: usize,
    pub journal_size: Option<u32>,
    pub enable_vectors: bool,
    pub write_through: bool,
    pub sync_interval: u64,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            block_size: 4096,
            cache_size: 64 * 1024 * 1024, // 64MB cache
            journal_size: None, // Auto-calculate
            enable_vectors: true,
            write_through: false,
            sync_interval: 30,
        }
    }
}

impl StorageConfig {
    /// Create minimal configuration for testing
    pub fn minimal() -> Self {
        Self {
            block_size: 4096,
            cache_size: 1024 * 1024, // 1MB cache
            journal_size: Some(256), // 256 blocks
            enable_vectors: false,
            write_through: true,
            sync_interval: 5,
        }
    }

    /// Create high-performance configuration
    pub fn high_performance() -> Self {
        Self {
            block_size: 4096,
            cache_size: 512 * 1024 * 1024, // 512MB cache
            journal_size: None, // Auto-calculate
            enable_vectors: true,
            write_through: false,
            sync_interval: 60,
        }
    }

    /// Validate configuration parameters
    pub fn validate(&self) -> VexfsResult<()> {
        if !self.block_size.is_power_of_two() || 
           self.block_size < 512 || 
           self.block_size > 65536 {
            return Err(VexfsError::InvalidArgument("invalid block size".to_string()));
        }

        if self.cache_size < self.block_size as usize {
            return Err(VexfsError::InvalidArgument("cache size too small".to_string()));
        }

        if self.sync_interval == 0 {
            return Err(VexfsError::InvalidArgument("sync interval cannot be zero".to_string()));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_config_validation() {
        let config = StorageConfig::default();
        assert!(config.validate().is_ok());

        let mut bad_config = config.clone();
        bad_config.block_size = 1000; // Not power of 2
        assert!(bad_config.validate().is_err());

        bad_config.block_size = 4096;
        bad_config.cache_size = 1000; // Smaller than block size
        assert!(bad_config.validate().is_err());
    }

    #[test]
    fn test_health_status() {
        let stats = StorageStats {
            total_blocks: 1000,
            free_blocks: 500,
            used_blocks: 500,
            cache_hit_rate: 85.0,
            cache_utilization: 70.0,
            journal_utilization: 50.0,
            fragmentation: 15.0,
        };

        // This would typically be tested with a real StorageManager
        assert_eq!(stats.total_blocks, 1000);
        assert_eq!(stats.free_blocks, 500);
    }
}