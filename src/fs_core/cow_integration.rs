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

//! CoW and Snapshot Integration for VexFS
//!
//! This module provides integration between Copy-on-Write functionality,
//! snapshot management, and the core filesystem operations.

use crate::shared::errors::{VexfsError, VexfsResult};
use crate::shared::types::*;
use crate::shared::constants::*;
use crate::fs_core::cow::{CowManager, CowStats};
use crate::fs_core::snapshot::{SnapshotManager, SnapshotId, SnapshotStats};
use crate::fs_core::operations::OperationContext;
use crate::fs_core::inode::{Inode, InodeManager};
use crate::storage::{StorageManager, TransactionManager};

#[cfg(not(feature = "kernel"))]
use std::sync::{Arc, RwLock, Mutex};
#[cfg(not(feature = "kernel"))]
use std::collections::HashMap;

#[cfg(feature = "kernel")]
use alloc::sync::Arc;
#[cfg(feature = "kernel")]
use alloc::collections::BTreeMap;

#[cfg(not(feature = "std"))]
use alloc::{vec::Vec, string::String};
#[cfg(feature = "std")]
use std::{vec::Vec, string::String};

/// CoW-enabled filesystem operations
pub struct CowFilesystemOperations {
    /// CoW manager for handling copy-on-write operations
    cow_manager: Arc<CowManager>,
    
    /// Snapshot manager for handling snapshots
    snapshot_manager: Arc<SnapshotManager>,
    
    /// Storage manager reference
    storage: Arc<StorageManager>,
    
    /// Transaction manager for atomic operations
    transaction_manager: Arc<TransactionManager>,
    
    /// Configuration for CoW operations
    config: CowConfig,
}

impl CowFilesystemOperations {
    /// Create a new CoW filesystem operations manager
    pub fn new(
        storage: Arc<StorageManager>,
        transaction_manager: Arc<TransactionManager>,
        config: CowConfig,
    ) -> VexfsResult<Self> {
        let cow_manager = Arc::new(CowManager::new(storage.clone()));
        let snapshot_manager = Arc::new(SnapshotManager::new(
            cow_manager.clone(),
            storage.clone(),
        ));

        Ok(Self {
            cow_manager,
            snapshot_manager,
            storage,
            transaction_manager,
            config,
        })
    }

    /// Write data to a file with CoW semantics
    pub fn cow_write_file(
        &self,
        inode: InodeNumber,
        offset: u64,
        data: &[u8],
        context: &mut OperationContext,
    ) -> VexfsResult<usize> {
        // Calculate logical block offset
        let block_size = VEXFS_DEFAULT_BLOCK_SIZE as u64;
        let logical_block = offset / block_size;
        
        // Perform CoW write operation
        let physical_block = self.cow_manager.cow_write(inode, logical_block, data)?;
        
        // Update inode metadata
        let inode_arc = context.inode_manager.get_inode(inode)?;
        let mut inode_data = (*inode_arc).clone();
        
        // Update size if necessary
        let new_size = offset + data.len() as u64;
        if new_size > inode_data.size {
            inode_data.size = new_size;
        }
        
        // Update timestamps
        inode_data.touch_mtime();
        inode_data.touch_ctime();
        
        // Save updated inode
        context.inode_manager.put_inode(Arc::new(inode_data))?;
        
        Ok(data.len())
    }

    /// Read data from a file with CoW awareness
    pub fn cow_read_file(
        &self,
        inode: InodeNumber,
        offset: u64,
        size: usize,
        _context: &mut OperationContext,
    ) -> VexfsResult<Vec<u8>> {
        let block_size = VEXFS_DEFAULT_BLOCK_SIZE as u64;
        let logical_block = offset / block_size;
        let block_offset = offset % block_size;
        
        // Read from CoW-aware storage
        let block_data = self.cow_manager.cow_read(inode, logical_block)?;
        
        // Extract the requested portion
        let start = block_offset as usize;
        let end = (start + size).min(block_data.len());
        
        if start >= block_data.len() {
            return Ok(Vec::new());
        }
        
        Ok(block_data[start..end].to_vec())
    }

    /// Create a snapshot of the filesystem
    pub fn create_snapshot(
        &self,
        name: String,
        root_inode: InodeNumber,
        parent_snapshot: Option<SnapshotId>,
    ) -> VexfsResult<SnapshotId> {
        self.snapshot_manager.create_snapshot(name, root_inode, parent_snapshot)
    }

    /// Delete a snapshot
    pub fn delete_snapshot(&self, snapshot_id: SnapshotId, force: bool) -> VexfsResult<()> {
        self.snapshot_manager.delete_snapshot(snapshot_id, force)
    }

    /// List all snapshots
    pub fn list_snapshots(&self) -> VexfsResult<Vec<crate::fs_core::snapshot::SnapshotMetadata>> {
        self.snapshot_manager.list_snapshots()
    }

    /// Access a file from a specific snapshot
    pub fn read_from_snapshot(
        &self,
        snapshot_id: SnapshotId,
        inode: InodeNumber,
        offset: u64,
        size: usize,
    ) -> VexfsResult<Vec<u8>> {
        // Get snapshot metadata
        let snapshot = self.snapshot_manager.get_snapshot(snapshot_id)?;
        
        // For now, use the snapshot's root inode mapping
        // In a full implementation, this would traverse the snapshot's inode tree
        let logical_block = offset / VEXFS_DEFAULT_BLOCK_SIZE as u64;
        
        // Read from the snapshot's CoW mapping
        self.cow_manager.cow_read(snapshot.root_inode, logical_block)
    }

    /// Perform garbage collection on obsolete snapshots and CoW data
    pub fn garbage_collect(&self) -> VexfsResult<GarbageCollectionSummary> {
        let snapshot_gc_result = self.snapshot_manager.garbage_collect()?;
        
        // TODO: Implement CoW-specific garbage collection
        // This would involve:
        // 1. Finding unreferenced CoW blocks
        // 2. Consolidating shared blocks
        // 3. Cleaning up orphaned mappings
        
        Ok(GarbageCollectionSummary {
            snapshots_deleted: snapshot_gc_result.snapshots_deleted,
            space_freed: snapshot_gc_result.space_freed,
            cow_blocks_freed: 0, // TODO: Implement
            errors: snapshot_gc_result.errors,
            duration_ms: snapshot_gc_result.duration_ms,
        })
    }

    /// Get comprehensive statistics
    pub fn get_stats(&self) -> VexfsResult<CowFilesystemStats> {
        let cow_stats = self.cow_manager.get_stats()?;
        let snapshot_stats = self.snapshot_manager.get_stats()?;
        
        Ok(CowFilesystemStats {
            cow_stats,
            snapshot_stats,
            total_space_used: cow_stats.blocks_copied as u64 * VEXFS_DEFAULT_BLOCK_SIZE as u64 + snapshot_stats.total_space_used,
            space_efficiency: calculate_space_efficiency(&cow_stats, &snapshot_stats),
        })
    }

    /// Optimize CoW mappings and snapshots
    pub fn optimize(&self) -> VexfsResult<OptimizationResult> {
        let mut result = OptimizationResult::default();
        
        // TODO: Implement optimization strategies:
        // 1. Consolidate fragmented CoW extents
        // 2. Merge similar snapshots
        // 3. Compress old snapshot data
        // 4. Defragment CoW mappings
        
        result.operations_performed = 0;
        result.space_saved = 0;
        result.performance_improvement = 0.0;
        
        Ok(result)
    }

    /// Create an incremental snapshot based on changes since the last snapshot
    pub fn create_incremental_snapshot(
        &self,
        name: String,
        base_snapshot: SnapshotId,
        root_inode: InodeNumber,
    ) -> VexfsResult<SnapshotId> {
        // Create the new snapshot
        let snapshot_id = self.snapshot_manager.create_snapshot(
            name,
            root_inode,
            Some(base_snapshot),
        )?;
        
        // TODO: Create delta between base and new snapshot
        // This would involve comparing the two snapshots and storing only differences
        
        Ok(snapshot_id)
    }

    /// Restore a file from a snapshot
    pub fn restore_from_snapshot(
        &self,
        snapshot_id: SnapshotId,
        source_inode: InodeNumber,
        target_inode: InodeNumber,
        context: &mut OperationContext,
    ) -> VexfsResult<()> {
        // Get snapshot metadata
        let _snapshot = self.snapshot_manager.get_snapshot(snapshot_id)?;
        
        // Read data from snapshot
        let data = self.read_from_snapshot(snapshot_id, source_inode, 0, usize::MAX)?;
        
        // Write data to target inode using CoW
        self.cow_write_file(target_inode, 0, &data, context)?;
        
        Ok(())
    }

    /// Check consistency of CoW mappings and snapshots
    pub fn check_consistency(&self) -> VexfsResult<ConsistencyReport> {
        let mut report = ConsistencyReport::default();
        
        // TODO: Implement consistency checks:
        // 1. Verify CoW block reference counts
        // 2. Check snapshot parent-child relationships
        // 3. Validate delta storage integrity
        // 4. Ensure no orphaned blocks
        
        report.cow_mappings_checked = 0;
        report.snapshots_checked = 0;
        report.errors_found = 0;
        report.warnings_found = 0;
        
        Ok(report)
    }
}

/// Configuration for CoW operations
#[derive(Debug, Clone)]
pub struct CowConfig {
    /// Enable automatic CoW for all writes
    pub auto_cow: bool,
    /// Maximum number of snapshots to keep
    pub max_snapshots: u32,
    /// Automatic garbage collection interval (seconds)
    pub gc_interval_seconds: u64,
    /// Enable compression for snapshot data
    pub compress_snapshots: bool,
    /// Enable incremental snapshots
    pub enable_incremental: bool,
    /// Maximum CoW extent size
    pub max_extent_size: u32,
}

impl Default for CowConfig {
    fn default() -> Self {
        Self {
            auto_cow: true,
            max_snapshots: 100,
            gc_interval_seconds: 3600, // 1 hour
            compress_snapshots: true,
            enable_incremental: true,
            max_extent_size: 1024, // 1024 blocks
        }
    }
}

/// Combined statistics for CoW and snapshot operations
#[derive(Debug, Clone)]
pub struct CowFilesystemStats {
    /// CoW-specific statistics
    pub cow_stats: CowStats,
    /// Snapshot-specific statistics
    pub snapshot_stats: SnapshotStats,
    /// Total space used by CoW and snapshots
    pub total_space_used: u64,
    /// Overall space efficiency percentage
    pub space_efficiency: f64,
}

/// Garbage collection summary
#[derive(Debug, Default)]
pub struct GarbageCollectionSummary {
    /// Number of snapshots deleted
    pub snapshots_deleted: u64,
    /// Space freed from snapshots
    pub space_freed: u64,
    /// Number of CoW blocks freed
    pub cow_blocks_freed: u64,
    /// Number of errors encountered
    pub errors: u64,
    /// Time taken for GC
    pub duration_ms: u64,
}

/// Optimization result
#[derive(Debug, Default)]
pub struct OptimizationResult {
    /// Number of optimization operations performed
    pub operations_performed: u64,
    /// Space saved through optimization
    pub space_saved: u64,
    /// Performance improvement percentage
    pub performance_improvement: f64,
}

/// Consistency check report
#[derive(Debug, Default)]
pub struct ConsistencyReport {
    /// Number of CoW mappings checked
    pub cow_mappings_checked: u64,
    /// Number of snapshots checked
    pub snapshots_checked: u64,
    /// Number of errors found
    pub errors_found: u64,
    /// Number of warnings found
    pub warnings_found: u64,
}

/// Calculate overall space efficiency
fn calculate_space_efficiency(cow_stats: &CowStats, snapshot_stats: &SnapshotStats) -> f64 {
    let total_space_used = (cow_stats.blocks_copied as u64) * (VEXFS_DEFAULT_BLOCK_SIZE as u64) + snapshot_stats.total_space_used;
    let total_space_saved = cow_stats.space_saved + snapshot_stats.space_saved;
    
    if total_space_used == 0 {
        return 100.0;
    }
    
    (total_space_saved as f64 / total_space_used as f64) * 100.0
}

/// High-level CoW and snapshot operations for filesystem integration
pub struct CowSnapshotIntegration {
    /// CoW filesystem operations
    cow_ops: Arc<CowFilesystemOperations>,
    
    /// Background task configuration
    #[cfg(not(feature = "kernel"))]
    background_tasks: Mutex<BackgroundTaskConfig>,
    #[cfg(feature = "kernel")]
    background_tasks: crate::shared::types::VexfsMutex<BackgroundTaskConfig>,
}

impl CowSnapshotIntegration {
    /// Create new CoW and snapshot integration
    pub fn new(cow_ops: Arc<CowFilesystemOperations>) -> Self {
        Self {
            cow_ops,
            #[cfg(not(feature = "kernel"))]
            background_tasks: Mutex::new(BackgroundTaskConfig::default()),
            #[cfg(feature = "kernel")]
            background_tasks: crate::shared::types::VexfsMutex::new(BackgroundTaskConfig::default()),
        }
    }

    /// Initialize background tasks for automatic maintenance
    pub fn start_background_tasks(&self) -> VexfsResult<()> {
        // TODO: Implement background tasks:
        // 1. Periodic garbage collection
        // 2. Automatic snapshot creation
        // 3. CoW optimization
        // 4. Consistency checking
        
        #[cfg(not(feature = "kernel"))]
        {
            let mut tasks = self.background_tasks.lock().map_err(|_| VexfsError::LockConflict("failed to acquire background tasks lock".to_string()))?;
            tasks.gc_enabled = true;
            tasks.auto_snapshot_enabled = true;
        }
        #[cfg(feature = "kernel")]
        {
            let mut tasks = self.background_tasks.lock();
            tasks.gc_enabled = true;
            tasks.auto_snapshot_enabled = true;
        }
        
        Ok(())
    }

    /// Stop background tasks
    pub fn stop_background_tasks(&self) -> VexfsResult<()> {
        #[cfg(not(feature = "kernel"))]
        {
            let mut tasks = self.background_tasks.lock().map_err(|_| VexfsError::LockConflict("failed to acquire background tasks lock".to_string()))?;
            tasks.gc_enabled = false;
            tasks.auto_snapshot_enabled = false;
        }
        #[cfg(feature = "kernel")]
        {
            let mut tasks = self.background_tasks.lock();
            tasks.gc_enabled = false;
            tasks.auto_snapshot_enabled = false;
        }
        
        Ok(())
    }

    /// Get reference to CoW operations
    pub fn cow_ops(&self) -> &Arc<CowFilesystemOperations> {
        &self.cow_ops
    }
}

/// Background task configuration
#[derive(Debug, Default)]
struct BackgroundTaskConfig {
    /// Enable automatic garbage collection
    gc_enabled: bool,
    /// Enable automatic snapshot creation
    auto_snapshot_enabled: bool,
    /// GC interval in seconds
    gc_interval: u64,
    /// Auto-snapshot interval in seconds
    snapshot_interval: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cow_config() {
        let config = CowConfig::default();
        assert!(config.auto_cow);
        assert_eq!(config.max_snapshots, 100);
        assert!(config.compress_snapshots);
        assert!(config.enable_incremental);
    }

    #[test]
    fn test_space_efficiency_calculation() {
        let cow_stats = CowStats {
            blocks_copied: 100,
            space_saved: (50 as u64) * (VEXFS_DEFAULT_BLOCK_SIZE as u64),
            ..Default::default()
        };
        
        let snapshot_stats = SnapshotStats {
            total_space_used: 200u64 * VEXFS_DEFAULT_BLOCK_SIZE as u64,
            space_saved: 100u64 * VEXFS_DEFAULT_BLOCK_SIZE as u64,
            ..Default::default()
        };
        
        let efficiency = calculate_space_efficiency(&cow_stats, &snapshot_stats);
        assert!(efficiency > 0.0);
        assert!(efficiency <= 100.0);
    }

    #[test]
    fn test_garbage_collection_summary() {
        let mut summary = GarbageCollectionSummary::default();
        summary.snapshots_deleted = 5;
        summary.space_freed = 1024 * 1024;
        summary.cow_blocks_freed = 100;
        
        assert_eq!(summary.snapshots_deleted, 5);
        assert_eq!(summary.space_freed, 1024 * 1024);
        assert_eq!(summary.cow_blocks_freed, 100);
    }
}