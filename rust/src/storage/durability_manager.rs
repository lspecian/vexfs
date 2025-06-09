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

//! Durability Manager
//!
//! This module ensures ACID durability guarantees by managing proper
//! synchronization of data and metadata to persistent storage through
//! coordinated fsync/msync operations and write barriers.

extern crate alloc;
use alloc::{vec::Vec, collections::BTreeMap, string::String, format, sync::Arc};
use core::{sync::atomic::{AtomicU64, AtomicU32, AtomicBool, Ordering}, mem};

use crate::shared::{
    errors::{VexfsError, VexfsResult},
    types::*,
    constants::*,
    utils::*,
};

/// Durability policies for different data types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DurabilityPolicy {
    /// No explicit durability guarantees (fastest)
    None,
    /// Metadata only durability
    MetadataOnly,
    /// Data and metadata durability
    DataAndMetadata,
    /// Strict durability with barriers
    Strict,
    /// Configurable durability based on transaction flags
    Configurable,
}

/// Sync operation types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncOperation {
    /// File data sync (fsync)
    FileSync,
    /// Memory mapped region sync (msync)
    MemorySync,
    /// Directory sync (fsync on directory)
    DirectorySync,
    /// Journal sync
    JournalSync,
    /// Metadata sync
    MetadataSync,
    /// Full filesystem sync
    FilesystemSync,
}

/// Sync request for batching operations
#[derive(Debug, Clone)]
pub struct SyncRequest {
    /// Type of sync operation
    pub operation: SyncOperation,
    /// Target identifier (inode, block, etc.)
    pub target: u64,
    /// Priority level
    pub priority: SyncPriority,
    /// Transaction ID that requested this sync
    pub transaction_id: TransactionId,
    /// Request timestamp
    pub timestamp: u64,
    /// Completion callback identifier
    pub callback_id: Option<u64>,
}

/// Sync priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SyncPriority {
    /// Low priority (background sync)
    Low = 1,
    /// Normal priority (regular transactions)
    Normal = 2,
    /// High priority (critical operations)
    High = 3,
    /// Critical priority (system integrity)
    Critical = 4,
}

impl SyncRequest {
    pub fn new(
        operation: SyncOperation,
        target: u64,
        priority: SyncPriority,
        transaction_id: TransactionId,
    ) -> Self {
        Self {
            operation,
            target,
            priority,
            transaction_id,
            timestamp: get_current_timestamp(),
            callback_id: None,
        }
    }

    pub fn with_callback(mut self, callback_id: u64) -> Self {
        self.callback_id = Some(callback_id);
        self
    }
}

/// Write barrier types for ordering guarantees
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteBarrier {
    /// No barrier
    None,
    /// Flush write cache
    FlushCache,
    /// Full memory barrier
    FullBarrier,
    /// Ordered writes
    OrderedWrites,
}

/// Durability checkpoint information
#[derive(Debug, Clone)]
pub struct DurabilityCheckpoint {
    /// Checkpoint ID
    pub checkpoint_id: u64,
    /// Timestamp when checkpoint was created
    pub timestamp: u64,
    /// Transaction ID up to which durability is guaranteed
    pub last_durable_transaction: TransactionId,
    /// Journal position at checkpoint
    pub journal_position: u64,
    /// Metadata version at checkpoint
    pub metadata_version: u64,
    /// Checkpoint completion status
    pub completed: bool,
}

impl DurabilityCheckpoint {
    pub fn new(
        checkpoint_id: u64,
        last_durable_transaction: TransactionId,
        journal_position: u64,
        metadata_version: u64,
    ) -> Self {
        Self {
            checkpoint_id,
            timestamp: get_current_timestamp(),
            last_durable_transaction,
            journal_position,
            metadata_version,
            completed: false,
        }
    }

    pub fn mark_completed(&mut self) {
        self.completed = true;
    }
}

/// Durability statistics
#[derive(Debug, Clone, Copy, Default)]
pub struct DurabilityStats {
    /// Total sync operations performed
    pub total_sync_ops: u64,
    /// File sync operations
    pub file_sync_ops: u64,
    /// Memory sync operations
    pub memory_sync_ops: u64,
    /// Directory sync operations
    pub directory_sync_ops: u64,
    /// Journal sync operations
    pub journal_sync_ops: u64,
    /// Metadata sync operations
    pub metadata_sync_ops: u64,
    /// Filesystem sync operations
    pub filesystem_sync_ops: u64,
    /// Average sync latency (microseconds)
    pub avg_sync_latency_us: u64,
    /// Total sync time (microseconds)
    pub total_sync_time_us: u64,
    /// Sync failures
    pub sync_failures: u64,
    /// Checkpoints created
    pub checkpoints_created: u64,
    /// Checkpoints completed
    pub checkpoints_completed: u64,
    /// Write barriers issued
    pub write_barriers_issued: u64,
    /// Batched sync operations
    pub batched_sync_ops: u64,
}

/// Durability Manager
pub struct DurabilityManager {
    /// Current durability policy
    policy: DurabilityPolicy,
    /// Pending sync requests
    pending_syncs: Vec<SyncRequest>,
    /// Completed checkpoints
    checkpoints: BTreeMap<u64, DurabilityCheckpoint>,
    /// Next checkpoint ID
    next_checkpoint_id: AtomicU64,
    /// Durability statistics
    stats: DurabilityStats,
    /// Sync batching enabled
    batching_enabled: AtomicBool,
    /// Batch timeout (milliseconds)
    batch_timeout_ms: u64,
    /// Maximum batch size
    max_batch_size: usize,
    /// Last batch flush time
    last_batch_flush: AtomicU64,
    /// Sync thread enabled
    sync_thread_enabled: AtomicBool,
    /// Force sync flag
    force_sync_flag: AtomicBool,
}

impl DurabilityManager {
    /// Create new durability manager
    pub fn new(policy: DurabilityPolicy) -> Self {
        Self {
            policy,
            pending_syncs: Vec::new(),
            checkpoints: BTreeMap::new(),
            next_checkpoint_id: AtomicU64::new(1),
            stats: DurabilityStats::default(),
            batching_enabled: AtomicBool::new(true),
            batch_timeout_ms: 100, // 100ms batch timeout
            max_batch_size: 64,
            last_batch_flush: AtomicU64::new(0),
            sync_thread_enabled: AtomicBool::new(false),
            force_sync_flag: AtomicBool::new(false),
        }
    }

    /// Ensure durability for a transaction
    pub fn ensure_durability(
        &mut self,
        transaction_id: TransactionId,
        data_blocks: &[BlockNumber],
        metadata_blocks: &[BlockNumber],
    ) -> VexfsResult<()> {
        match self.policy {
            DurabilityPolicy::None => {
                // No durability guarantees needed
                Ok(())
            }
            DurabilityPolicy::MetadataOnly => {
                self.sync_metadata_blocks(transaction_id, metadata_blocks)
            }
            DurabilityPolicy::DataAndMetadata => {
                self.sync_data_and_metadata(transaction_id, data_blocks, metadata_blocks)
            }
            DurabilityPolicy::Strict => {
                self.sync_strict(transaction_id, data_blocks, metadata_blocks)
            }
            DurabilityPolicy::Configurable => {
                // Use transaction flags to determine policy
                self.sync_configurable(transaction_id, data_blocks, metadata_blocks)
            }
        }
    }

    /// Sync metadata blocks only
    fn sync_metadata_blocks(
        &mut self,
        transaction_id: TransactionId,
        metadata_blocks: &[BlockNumber],
    ) -> VexfsResult<()> {
        for &block in metadata_blocks {
            let request = SyncRequest::new(
                SyncOperation::MetadataSync,
                block,
                SyncPriority::Normal,
                transaction_id,
            );
            self.add_sync_request(request)?;
        }
        
        self.flush_pending_syncs()?;
        Ok(())
    }

    /// Sync both data and metadata blocks
    fn sync_data_and_metadata(
        &mut self,
        transaction_id: TransactionId,
        data_blocks: &[BlockNumber],
        metadata_blocks: &[BlockNumber],
    ) -> VexfsResult<()> {
        // First sync data blocks
        for &block in data_blocks {
            let request = SyncRequest::new(
                SyncOperation::FileSync,
                block,
                SyncPriority::Normal,
                transaction_id,
            );
            self.add_sync_request(request)?;
        }
        
        // Then sync metadata blocks
        for &block in metadata_blocks {
            let request = SyncRequest::new(
                SyncOperation::MetadataSync,
                block,
                SyncPriority::Normal,
                transaction_id,
            );
            self.add_sync_request(request)?;
        }
        
        self.flush_pending_syncs()?;
        Ok(())
    }

    /// Sync with strict durability guarantees
    fn sync_strict(
        &mut self,
        transaction_id: TransactionId,
        data_blocks: &[BlockNumber],
        metadata_blocks: &[BlockNumber],
    ) -> VexfsResult<()> {
        // Issue write barrier before syncing
        self.issue_write_barrier(WriteBarrier::FullBarrier)?;
        
        // Sync data blocks with high priority
        for &block in data_blocks {
            let request = SyncRequest::new(
                SyncOperation::FileSync,
                block,
                SyncPriority::High,
                transaction_id,
            );
            self.add_sync_request(request)?;
        }
        
        // Sync metadata blocks with high priority
        for &block in metadata_blocks {
            let request = SyncRequest::new(
                SyncOperation::MetadataSync,
                block,
                SyncPriority::High,
                transaction_id,
            );
            self.add_sync_request(request)?;
        }
        
        // Force immediate flush
        self.force_sync_flag.store(true, Ordering::Relaxed);
        self.flush_pending_syncs()?;
        
        // Issue another barrier after syncing
        self.issue_write_barrier(WriteBarrier::FullBarrier)?;
        
        Ok(())
    }

    /// Sync with configurable policy based on transaction flags
    fn sync_configurable(
        &mut self,
        transaction_id: TransactionId,
        data_blocks: &[BlockNumber],
        metadata_blocks: &[BlockNumber],
    ) -> VexfsResult<()> {
        // In a real implementation, would check transaction flags
        // For now, default to data and metadata sync
        self.sync_data_and_metadata(transaction_id, data_blocks, metadata_blocks)
    }

    /// Add sync request to pending queue
    pub fn add_sync_request(&mut self, request: SyncRequest) -> VexfsResult<()> {
        if self.batching_enabled.load(Ordering::Relaxed) {
            self.pending_syncs.push(request);
            
            // Check if we should flush based on batch size or timeout
            if self.pending_syncs.len() >= self.max_batch_size {
                self.flush_pending_syncs()?;
            } else {
                self.check_batch_timeout()?;
            }
        } else {
            // Execute immediately
            self.execute_sync_request(&request)?;
        }
        
        Ok(())
    }

    /// Check if batch timeout has been reached
    fn check_batch_timeout(&mut self) -> VexfsResult<()> {
        let current_time = get_current_timestamp();
        let last_flush = self.last_batch_flush.load(Ordering::Relaxed);
        
        if current_time - last_flush >= self.batch_timeout_ms {
            self.flush_pending_syncs()?;
        }
        
        Ok(())
    }

    /// Flush all pending sync requests
    pub fn flush_pending_syncs(&mut self) -> VexfsResult<()> {
        if self.pending_syncs.is_empty() {
            return Ok(());
        }
        
        // Sort by priority (highest first)
        self.pending_syncs.sort_by(|a, b| b.priority.cmp(&a.priority));
        
        // Execute all pending syncs
        // Clone the pending syncs to avoid borrowing conflicts
        let pending_syncs = self.pending_syncs.clone();
        for request in &pending_syncs {
            self.execute_sync_request(request)?;
        }
        
        // Update statistics
        self.stats.batched_sync_ops += self.pending_syncs.len() as u64;
        
        // Clear pending syncs
        self.pending_syncs.clear();
        self.last_batch_flush.store(get_current_timestamp(), Ordering::Relaxed);
        self.force_sync_flag.store(false, Ordering::Relaxed);
        
        Ok(())
    }

    /// Execute a single sync request
    fn execute_sync_request(&mut self, request: &SyncRequest) -> VexfsResult<()> {
        let start_time = get_current_timestamp();
        
        let result = match request.operation {
            SyncOperation::FileSync => self.execute_file_sync(request.target),
            SyncOperation::MemorySync => self.execute_memory_sync(request.target),
            SyncOperation::DirectorySync => self.execute_directory_sync(request.target),
            SyncOperation::JournalSync => self.execute_journal_sync(request.target),
            SyncOperation::MetadataSync => self.execute_metadata_sync(request.target),
            SyncOperation::FilesystemSync => self.execute_filesystem_sync(),
        };
        
        let sync_time = get_current_timestamp() - start_time;
        self.update_sync_stats(request.operation, sync_time, result.is_ok());
        
        result
    }

    /// Execute file sync operation
    fn execute_file_sync(&mut self, _target: u64) -> VexfsResult<()> {
        // In kernel context, would call appropriate sync functions
        // For now, simulate the operation
        self.stats.file_sync_ops += 1;
        Ok(())
    }

    /// Execute memory sync operation
    fn execute_memory_sync(&mut self, _target: u64) -> VexfsResult<()> {
        // In kernel context, would call msync or equivalent
        self.stats.memory_sync_ops += 1;
        Ok(())
    }

    /// Execute directory sync operation
    fn execute_directory_sync(&mut self, _target: u64) -> VexfsResult<()> {
        // In kernel context, would sync directory metadata
        self.stats.directory_sync_ops += 1;
        Ok(())
    }

    /// Execute journal sync operation
    fn execute_journal_sync(&mut self, _target: u64) -> VexfsResult<()> {
        // In kernel context, would sync journal to storage
        self.stats.journal_sync_ops += 1;
        Ok(())
    }

    /// Execute metadata sync operation
    fn execute_metadata_sync(&mut self, _target: u64) -> VexfsResult<()> {
        // In kernel context, would sync metadata blocks
        self.stats.metadata_sync_ops += 1;
        Ok(())
    }

    /// Execute filesystem sync operation
    fn execute_filesystem_sync(&mut self) -> VexfsResult<()> {
        // In kernel context, would sync entire filesystem
        self.stats.filesystem_sync_ops += 1;
        Ok(())
    }

    /// Issue write barrier
    pub fn issue_write_barrier(&mut self, barrier_type: WriteBarrier) -> VexfsResult<()> {
        match barrier_type {
            WriteBarrier::None => {
                // No barrier needed
            }
            WriteBarrier::FlushCache => {
                // In kernel context, would flush write cache
            }
            WriteBarrier::FullBarrier => {
                // In kernel context, would issue full memory barrier
            }
            WriteBarrier::OrderedWrites => {
                // In kernel context, would ensure write ordering
            }
        }
        
        self.stats.write_barriers_issued += 1;
        Ok(())
    }

    /// Create durability checkpoint
    pub fn create_checkpoint(
        &mut self,
        last_durable_transaction: TransactionId,
        journal_position: u64,
        metadata_version: u64,
    ) -> VexfsResult<u64> {
        let checkpoint_id = self.next_checkpoint_id.fetch_add(1, Ordering::SeqCst);
        
        let checkpoint = DurabilityCheckpoint::new(
            checkpoint_id,
            last_durable_transaction,
            journal_position,
            metadata_version,
        );
        
        self.checkpoints.insert(checkpoint_id, checkpoint);
        self.stats.checkpoints_created += 1;
        
        // Ensure all pending syncs are flushed for checkpoint
        self.flush_pending_syncs()?;
        
        Ok(checkpoint_id)
    }

    /// Complete durability checkpoint
    pub fn complete_checkpoint(&mut self, checkpoint_id: u64) -> VexfsResult<()> {
        let checkpoint = self.checkpoints.get_mut(&checkpoint_id)
            .ok_or(VexfsError::EntryNotFound("Checkpoint not found".into()))?;
        
        checkpoint.mark_completed();
        self.stats.checkpoints_completed += 1;
        
        Ok(())
    }

    /// Get latest completed checkpoint
    pub fn get_latest_checkpoint(&self) -> Option<&DurabilityCheckpoint> {
        self.checkpoints
            .values()
            .filter(|cp| cp.completed)
            .max_by_key(|cp| cp.checkpoint_id)
    }

    /// Cleanup old checkpoints
    pub fn cleanup_old_checkpoints(&mut self, keep_count: usize) -> VexfsResult<u32> {
        let mut completed_checkpoints: Vec<_> = self.checkpoints
            .iter()
            .filter(|(_, cp)| cp.completed)
            .map(|(&id, _)| id)
            .collect();
        
        completed_checkpoints.sort_unstable();
        
        let mut removed = 0;
        if completed_checkpoints.len() > keep_count {
            let to_remove = completed_checkpoints.len() - keep_count;
            for &checkpoint_id in &completed_checkpoints[..to_remove] {
                self.checkpoints.remove(&checkpoint_id);
                removed += 1;
            }
        }
        
        Ok(removed)
    }

    /// Update sync statistics
    fn update_sync_stats(&mut self, operation: SyncOperation, sync_time: u64, success: bool) {
        self.stats.total_sync_ops += 1;
        self.stats.total_sync_time_us += sync_time;
        
        // Update average latency
        self.stats.avg_sync_latency_us = 
            self.stats.total_sync_time_us / self.stats.total_sync_ops;
        
        if !success {
            self.stats.sync_failures += 1;
        }
        
        // Update operation-specific counters (already done in execute functions)
    }

    /// Set durability policy
    pub fn set_policy(&mut self, policy: DurabilityPolicy) {
        self.policy = policy;
    }

    /// Get current durability policy
    pub fn get_policy(&self) -> DurabilityPolicy {
        self.policy
    }

    /// Enable/disable sync batching
    pub fn set_batching_enabled(&self, enabled: bool) {
        self.batching_enabled.store(enabled, Ordering::Relaxed);
    }

    /// Set batch timeout
    pub fn set_batch_timeout(&mut self, timeout_ms: u64) {
        self.batch_timeout_ms = timeout_ms;
    }

    /// Set maximum batch size
    pub fn set_max_batch_size(&mut self, size: usize) {
        self.max_batch_size = size;
    }

    /// Get durability statistics
    pub fn get_stats(&self) -> DurabilityStats {
        self.stats
    }

    /// Force immediate sync of all pending operations
    pub fn force_sync(&mut self) -> VexfsResult<()> {
        self.force_sync_flag.store(true, Ordering::Relaxed);
        self.flush_pending_syncs()
    }

    /// Check if durability is guaranteed up to a transaction
    pub fn is_durable(&self, transaction_id: TransactionId) -> bool {
        if let Some(checkpoint) = self.get_latest_checkpoint() {
            checkpoint.last_durable_transaction >= transaction_id
        } else {
            false
        }
    }
}

/// Helper function to get current timestamp
fn get_current_timestamp() -> u64 {
    // In kernel context, would use appropriate kernel time functions
    static TIMESTAMP_COUNTER: AtomicU64 = AtomicU64::new(1);
    TIMESTAMP_COUNTER.fetch_add(1, Ordering::SeqCst)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_durability_manager_creation() {
        let manager = DurabilityManager::new(DurabilityPolicy::DataAndMetadata);
        assert_eq!(manager.get_policy(), DurabilityPolicy::DataAndMetadata);
    }

    #[test]
    fn test_sync_request_creation() {
        let request = SyncRequest::new(
            SyncOperation::FileSync,
            100,
            SyncPriority::Normal,
            1001,
        );
        
        assert_eq!(request.operation, SyncOperation::FileSync);
        assert_eq!(request.target, 100);
        assert_eq!(request.priority, SyncPriority::Normal);
        assert_eq!(request.transaction_id, 1001);
    }

    #[test]
    fn test_checkpoint_creation() {
        let mut manager = DurabilityManager::new(DurabilityPolicy::Strict);
        
        let checkpoint_id = manager.create_checkpoint(1001, 500, 100).unwrap();
        assert!(checkpoint_id > 0);
        
        let checkpoint = manager.checkpoints.get(&checkpoint_id).unwrap();
        assert_eq!(checkpoint.last_durable_transaction, 1001);
        assert!(!checkpoint.completed);
        
        manager.complete_checkpoint(checkpoint_id).unwrap();
        let checkpoint = manager.checkpoints.get(&checkpoint_id).unwrap();
        assert!(checkpoint.completed);
    }

    #[test]
    fn test_sync_batching() {
        let mut manager = DurabilityManager::new(DurabilityPolicy::DataAndMetadata);
        manager.set_max_batch_size(3);
        
        // Add requests that should be batched
        let request1 = SyncRequest::new(SyncOperation::FileSync, 100, SyncPriority::Normal, 1001);
        let request2 = SyncRequest::new(SyncOperation::FileSync, 101, SyncPriority::Normal, 1002);
        
        manager.add_sync_request(request1).unwrap();
        manager.add_sync_request(request2).unwrap();
        
        assert_eq!(manager.pending_syncs.len(), 2);
        
        // Third request should trigger flush
        let request3 = SyncRequest::new(SyncOperation::FileSync, 102, SyncPriority::High, 1003);
        manager.add_sync_request(request3).unwrap();
        
        // Should have flushed due to batch size
        assert!(manager.pending_syncs.is_empty());
    }

    #[test]
    fn test_durability_policies() {
        let mut manager = DurabilityManager::new(DurabilityPolicy::MetadataOnly);
        
        let data_blocks = vec![100, 101, 102];
        let metadata_blocks = vec![200, 201];
        
        // Should succeed with metadata-only policy
        let result = manager.ensure_durability(1001, &data_blocks, &metadata_blocks);
        assert!(result.is_ok());
        
        // Change to strict policy
        manager.set_policy(DurabilityPolicy::Strict);
        let result = manager.ensure_durability(1002, &data_blocks, &metadata_blocks);
        assert!(result.is_ok());
    }
}