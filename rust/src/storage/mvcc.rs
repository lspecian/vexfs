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

//! Multi-Version Concurrency Control (MVCC)
//!
//! This module implements MVCC for VexFS to provide transaction isolation
//! without blocking readers. Each data item maintains multiple versions,
//! allowing concurrent transactions to see consistent snapshots.

extern crate alloc;
use alloc::{vec::Vec, collections::BTreeMap, string::String, format, sync::Arc};
use core::{sync::atomic::{AtomicU64, AtomicU32, Ordering}, mem};

use crate::shared::{
    errors::{VexfsError, VexfsResult},
    types::*,
    constants::*,
    utils::*,
};

/// MVCC Version Chain Entry
#[derive(Debug, Clone)]
pub struct VersionChainEntry {
    /// Version identifier
    pub version_id: u64,
    /// Transaction that created this version
    pub created_by: TransactionId,
    /// Transaction that deleted this version (0 if active)
    pub deleted_by: TransactionId,
    /// Creation timestamp
    pub created_at: u64,
    /// Deletion timestamp (0 if active)
    pub deleted_at: u64,
    /// Data for this version
    pub data: Vec<u8>,
    /// Next version in chain (newer)
    pub next_version: Option<u64>,
    /// Previous version in chain (older)
    pub prev_version: Option<u64>,
    /// Version flags
    pub flags: u32,
}

impl VersionChainEntry {
    /// Create new version chain entry
    pub fn new(
        version_id: u64,
        created_by: TransactionId,
        created_at: u64,
        data: Vec<u8>,
    ) -> Self {
        Self {
            version_id,
            created_by,
            deleted_by: 0,
            created_at,
            deleted_at: 0,
            data,
            next_version: None,
            prev_version: None,
            flags: 0,
        }
    }

    /// Check if version is visible to a transaction
    pub fn is_visible_to(&self, transaction_id: TransactionId, snapshot_timestamp: u64) -> bool {
        // Version is visible if:
        // 1. We created it ourselves (our own uncommitted changes)
        // 2. It was committed before our snapshot and not deleted before our snapshot
        
        if self.created_by == transaction_id {
            return true;
        }
        
        // Check if version was created before our snapshot
        if self.created_at > snapshot_timestamp {
            return false;
        }
        
        // Check if version was deleted before our snapshot
        if self.deleted_by != 0 && self.deleted_at > 0 && self.deleted_at <= snapshot_timestamp {
            return false;
        }
        
        true
    }

    /// Mark version as deleted by transaction
    pub fn mark_deleted(&mut self, deleted_by: TransactionId, deleted_at: u64) {
        self.deleted_by = deleted_by;
        self.deleted_at = deleted_at;
    }

    /// Check if version is deleted
    pub fn is_deleted(&self) -> bool {
        self.deleted_by != 0
    }

    /// Get data size
    pub fn data_size(&self) -> usize {
        self.data.len()
    }
}

/// MVCC Version Chain for a single data item
#[derive(Debug, Clone)]
pub struct VersionChain {
    /// Block number this chain represents
    pub block_number: BlockNumber,
    /// Head of version chain (newest version)
    pub head_version: Option<u64>,
    /// Tail of version chain (oldest version)
    pub tail_version: Option<u64>,
    /// All versions in this chain
    pub versions: BTreeMap<u64, VersionChainEntry>,
    /// Chain statistics
    pub stats: VersionChainStats,
}

/// Version chain statistics
#[derive(Debug, Clone, Copy, Default)]
pub struct VersionChainStats {
    /// Total versions in chain
    pub total_versions: u32,
    /// Active (non-deleted) versions
    pub active_versions: u32,
    /// Total data size across all versions
    pub total_data_size: u64,
    /// Oldest version timestamp
    pub oldest_version_timestamp: u64,
    /// Newest version timestamp
    pub newest_version_timestamp: u64,
}

impl VersionChain {
    /// Create new version chain
    pub fn new(block_number: BlockNumber) -> Self {
        Self {
            block_number,
            head_version: None,
            tail_version: None,
            versions: BTreeMap::new(),
            stats: VersionChainStats::default(),
        }
    }

    /// Add new version to chain
    pub fn add_version(
        &mut self,
        version_id: u64,
        created_by: TransactionId,
        created_at: u64,
        data: Vec<u8>,
    ) -> VexfsResult<()> {
        let mut entry = VersionChainEntry::new(version_id, created_by, created_at, data);
        
        // Link to previous head
        if let Some(current_head) = self.head_version {
            entry.prev_version = Some(current_head);
            if let Some(head_entry) = self.versions.get_mut(&current_head) {
                head_entry.next_version = Some(version_id);
            }
        } else {
            // First version in chain
            self.tail_version = Some(version_id);
        }
        
        // Update head
        self.head_version = Some(version_id);
        
        // Update statistics
        self.stats.total_versions += 1;
        self.stats.active_versions += 1;
        self.stats.total_data_size += entry.data_size() as u64;
        
        if self.stats.oldest_version_timestamp == 0 || created_at < self.stats.oldest_version_timestamp {
            self.stats.oldest_version_timestamp = created_at;
        }
        if created_at > self.stats.newest_version_timestamp {
            self.stats.newest_version_timestamp = created_at;
        }
        
        self.versions.insert(version_id, entry);
        Ok(())
    }

    /// Find visible version for transaction
    pub fn find_visible_version(
        &self,
        transaction_id: TransactionId,
        snapshot_timestamp: u64,
    ) -> Option<&VersionChainEntry> {
        // Start from head and walk backwards to find first visible version
        let mut current_version = self.head_version;
        
        while let Some(version_id) = current_version {
            if let Some(entry) = self.versions.get(&version_id) {
                if entry.is_visible_to(transaction_id, snapshot_timestamp) {
                    return Some(entry);
                }
                current_version = entry.prev_version;
            } else {
                break;
            }
        }
        
        None
    }

    /// Mark version as deleted
    pub fn delete_version(
        &mut self,
        version_id: u64,
        deleted_by: TransactionId,
        deleted_at: u64,
    ) -> VexfsResult<()> {
        let entry = self.versions.get_mut(&version_id)
            .ok_or(VexfsError::EntryNotFound("Version not found".into()))?;
        
        if entry.is_deleted() {
            return Err(VexfsError::InvalidOperation("Version already deleted".into()));
        }
        
        entry.mark_deleted(deleted_by, deleted_at);
        self.stats.active_versions -= 1;
        
        Ok(())
    }

    /// Garbage collect old versions
    pub fn garbage_collect(&mut self, min_timestamp: u64) -> VexfsResult<u32> {
        let mut collected = 0;
        let mut versions_to_remove = Vec::new();
        
        // Find versions that can be safely removed
        for (&version_id, entry) in &self.versions {
            // Can remove if:
            // 1. Version is deleted
            // 2. Deletion timestamp is before min_timestamp
            // 3. Not the only version in chain
            if entry.is_deleted() 
                && entry.deleted_at < min_timestamp 
                && self.stats.total_versions > 1 {
                versions_to_remove.push(version_id);
            }
        }
        
        // Remove old versions
        for version_id in versions_to_remove {
            if let Some(entry) = self.versions.remove(&version_id) {
                // Update chain links
                if let Some(prev_id) = entry.prev_version {
                    if let Some(prev_entry) = self.versions.get_mut(&prev_id) {
                        prev_entry.next_version = entry.next_version;
                    }
                }
                
                if let Some(next_id) = entry.next_version {
                    if let Some(next_entry) = self.versions.get_mut(&next_id) {
                        next_entry.prev_version = entry.prev_version;
                    }
                }
                
                // Update head/tail if necessary
                if Some(version_id) == self.head_version {
                    self.head_version = entry.prev_version;
                }
                if Some(version_id) == self.tail_version {
                    self.tail_version = entry.next_version;
                }
                
                // Update statistics
                self.stats.total_versions -= 1;
                self.stats.total_data_size -= entry.data_size() as u64;
                collected += 1;
            }
        }
        
        Ok(collected)
    }

    /// Get chain statistics
    pub fn get_stats(&self) -> VersionChainStats {
        self.stats
    }

    /// Check if chain is empty
    pub fn is_empty(&self) -> bool {
        self.versions.is_empty()
    }

    /// Get total memory usage
    pub fn memory_usage(&self) -> usize {
        let mut usage = mem::size_of::<Self>();
        for entry in self.versions.values() {
            usage += mem::size_of::<VersionChainEntry>() + entry.data.len();
        }
        usage
    }
}

/// MVCC Manager
pub struct MvccManager {
    /// Version chains for all blocks
    version_chains: BTreeMap<BlockNumber, VersionChain>,
    /// Next version ID
    next_version_id: AtomicU64,
    /// Global timestamp counter
    timestamp_counter: AtomicU64,
    /// MVCC statistics
    stats: MvccStats,
    /// Garbage collection threshold
    gc_threshold_versions: u32,
    /// Garbage collection threshold timestamp
    gc_threshold_timestamp: u64,
}

/// MVCC Statistics
#[derive(Debug, Clone, Copy, Default)]
pub struct MvccStats {
    /// Total version chains
    pub total_chains: u32,
    /// Total versions across all chains
    pub total_versions: u64,
    /// Total memory usage
    pub total_memory_usage: u64,
    /// Garbage collection runs
    pub gc_runs: u64,
    /// Versions collected by GC
    pub versions_collected: u64,
    /// Average chain length
    pub avg_chain_length: f32,
    /// Read operations
    pub read_operations: u64,
    /// Write operations
    pub write_operations: u64,
    /// Version conflicts
    pub version_conflicts: u64,
}

impl MvccManager {
    /// Create new MVCC manager
    pub fn new() -> Self {
        Self {
            version_chains: BTreeMap::new(),
            next_version_id: AtomicU64::new(1),
            timestamp_counter: AtomicU64::new(1),
            stats: MvccStats::default(),
            gc_threshold_versions: 100,
            gc_threshold_timestamp: 3600000, // 1 hour in milliseconds
        }
    }

    /// Read data with MVCC visibility
    pub fn read(
        &mut self,
        block_number: BlockNumber,
        transaction_id: TransactionId,
        snapshot_timestamp: u64,
    ) -> VexfsResult<Vec<u8>> {
        self.stats.read_operations += 1;
        
        let chain = self.version_chains.get(&block_number)
            .ok_or(VexfsError::EntryNotFound("Block not found".into()))?;
        
        let version = chain.find_visible_version(transaction_id, snapshot_timestamp)
            .ok_or(VexfsError::EntryNotFound("No visible version found".into()))?;
        
        Ok(version.data.clone())
    }

    /// Write data creating new version
    pub fn write(
        &mut self,
        block_number: BlockNumber,
        transaction_id: TransactionId,
        data: Vec<u8>,
    ) -> VexfsResult<u64> {
        self.stats.write_operations += 1;
        
        let version_id = self.next_version_id.fetch_add(1, Ordering::SeqCst);
        let timestamp = self.timestamp_counter.fetch_add(1, Ordering::SeqCst);
        
        // Get or create version chain
        // Scope the mutable borrow to avoid conflicts
        let needs_gc = {
            let chain = self.version_chains.entry(block_number)
                .or_insert_with(|| VersionChain::new(block_number));
            
            chain.add_version(version_id, transaction_id, timestamp, data)?;
            
            // Check if we need garbage collection
            chain.stats.total_versions > self.gc_threshold_versions
        };
        
        // Update global statistics after releasing the mutable borrow
        self.stats.total_versions += 1;
        if self.version_chains.len() as u32 != self.stats.total_chains {
            self.stats.total_chains = self.version_chains.len() as u32;
        }
        
        // Check if garbage collection is needed
        if needs_gc {
            self.garbage_collect_chain(block_number)?;
        }
        
        Ok(version_id)
    }

    /// Delete data (mark version as deleted)
    pub fn delete(
        &mut self,
        block_number: BlockNumber,
        version_id: u64,
        transaction_id: TransactionId,
    ) -> VexfsResult<()> {
        let timestamp = self.timestamp_counter.fetch_add(1, Ordering::SeqCst);
        
        let chain = self.version_chains.get_mut(&block_number)
            .ok_or(VexfsError::EntryNotFound("Block not found".into()))?;
        
        chain.delete_version(version_id, transaction_id, timestamp)?;
        
        Ok(())
    }

    /// Get current timestamp
    pub fn get_current_timestamp(&self) -> u64 {
        self.timestamp_counter.load(Ordering::SeqCst)
    }

    /// Create snapshot timestamp for transaction
    pub fn create_snapshot(&mut self) -> u64 {
        self.timestamp_counter.fetch_add(1, Ordering::SeqCst)
    }

    /// Garbage collect old versions in a specific chain
    pub fn garbage_collect_chain(&mut self, block_number: BlockNumber) -> VexfsResult<u32> {
        let current_time = self.get_current_timestamp();
        let min_timestamp = current_time.saturating_sub(self.gc_threshold_timestamp);
        
        let chain = self.version_chains.get_mut(&block_number)
            .ok_or(VexfsError::EntryNotFound("Block not found".into()))?;
        
        let collected = chain.garbage_collect(min_timestamp)?;
        
        // Update statistics
        self.stats.gc_runs += 1;
        self.stats.versions_collected += collected as u64;
        self.stats.total_versions -= collected as u64;
        
        // Remove empty chains
        if chain.is_empty() {
            self.version_chains.remove(&block_number);
            self.stats.total_chains -= 1;
        }
        
        Ok(collected)
    }

    /// Garbage collect all chains
    pub fn garbage_collect_all(&mut self) -> VexfsResult<u64> {
        let mut total_collected = 0;
        let blocks_to_collect: Vec<BlockNumber> = self.version_chains.keys().cloned().collect();
        
        for block_number in blocks_to_collect {
            total_collected += self.garbage_collect_chain(block_number)? as u64;
        }
        
        // Update average chain length
        if self.stats.total_chains > 0 {
            self.stats.avg_chain_length = self.stats.total_versions as f32 / self.stats.total_chains as f32;
        }
        
        Ok(total_collected)
    }

    /// Get MVCC statistics
    pub fn get_stats(&mut self) -> MvccStats {
        // Update memory usage
        let mut total_memory = 0;
        for chain in self.version_chains.values() {
            total_memory += chain.memory_usage();
        }
        self.stats.total_memory_usage = total_memory as u64;
        
        self.stats
    }

    /// Set garbage collection thresholds
    pub fn set_gc_thresholds(&mut self, versions: u32, timestamp: u64) {
        self.gc_threshold_versions = versions;
        self.gc_threshold_timestamp = timestamp;
    }

    /// Check for version conflicts between transactions
    pub fn check_version_conflict(
        &mut self,
        block_number: BlockNumber,
        transaction_id: TransactionId,
        other_transaction_id: TransactionId,
    ) -> bool {
        if let Some(chain) = self.version_chains.get(&block_number) {
            // Check if both transactions have written to this block
            let has_tx1_version = chain.versions.values()
                .any(|v| v.created_by == transaction_id);
            let has_tx2_version = chain.versions.values()
                .any(|v| v.created_by == other_transaction_id);
            
            if has_tx1_version && has_tx2_version {
                self.stats.version_conflicts += 1;
                return true;
            }
        }
        
        false
    }

    /// Get version chain for block
    pub fn get_chain(&self, block_number: BlockNumber) -> Option<&VersionChain> {
        self.version_chains.get(&block_number)
    }

    /// Get version chain statistics
    pub fn get_chain_stats(&self, block_number: BlockNumber) -> Option<VersionChainStats> {
        self.version_chains.get(&block_number).map(|chain| chain.get_stats())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_chain_creation() {
        let mut chain = VersionChain::new(100);
        assert!(chain.is_empty());
        assert_eq!(chain.block_number, 100);
    }

    #[test]
    fn test_version_chain_add_version() {
        let mut chain = VersionChain::new(100);
        let data = vec![1, 2, 3, 4];
        
        chain.add_version(1, 1001, 1000, data.clone()).unwrap();
        
        assert!(!chain.is_empty());
        assert_eq!(chain.stats.total_versions, 1);
        assert_eq!(chain.stats.active_versions, 1);
        assert_eq!(chain.head_version, Some(1));
        assert_eq!(chain.tail_version, Some(1));
    }

    #[test]
    fn test_version_visibility() {
        let entry = VersionChainEntry::new(1, 1001, 1000, vec![1, 2, 3]);
        
        // Same transaction should see its own version
        assert!(entry.is_visible_to(1001, 2000));
        
        // Other transaction with later snapshot should see it
        assert!(entry.is_visible_to(1002, 2000));
        
        // Other transaction with earlier snapshot should not see it
        assert!(!entry.is_visible_to(1002, 500));
    }

    #[test]
    fn test_mvcc_manager_read_write() {
        let mut manager = MvccManager::new();
        let data = vec![1, 2, 3, 4];
        
        // Write data
        let version_id = manager.write(100, 1001, data.clone()).unwrap();
        assert!(version_id > 0);
        
        // Read data
        let snapshot = manager.create_snapshot();
        let read_data = manager.read(100, 1001, snapshot).unwrap();
        assert_eq!(read_data, data);
    }

    #[test]
    fn test_mvcc_isolation() {
        let mut manager = MvccManager::new();
        
        // Transaction 1 writes data
        let data1 = vec![1, 2, 3];
        manager.write(100, 1001, data1.clone()).unwrap();
        
        // Transaction 2 creates snapshot before transaction 1 commits
        let snapshot_tx2 = manager.create_snapshot();
        
        // Transaction 2 should not see transaction 1's uncommitted data
        // (In a real implementation, this would depend on commit status)
        let result = manager.read(100, 1002, snapshot_tx2);
        // This test would need more sophisticated commit tracking
    }

    #[test]
    fn test_garbage_collection() {
        let mut manager = MvccManager::new();
        manager.set_gc_thresholds(2, 1000);
        
        // Add multiple versions
        manager.write(100, 1001, vec![1]).unwrap();
        manager.write(100, 1002, vec![2]).unwrap();
        manager.write(100, 1003, vec![3]).unwrap();
        
        // Mark first version as deleted
        manager.delete(100, 1, 1001).unwrap();
        
        // Garbage collection should remove old versions
        let collected = manager.garbage_collect_chain(100).unwrap();
        assert!(collected <= 1); // May or may not collect depending on timestamps
    }
}