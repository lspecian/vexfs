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

//! Snapshot Management for VexFS
//!
//! This module provides comprehensive snapshot functionality including creation,
//! traversal, access, and garbage collection of filesystem snapshots.

use crate::shared::errors::{VexfsError, VexfsResult};
use crate::shared::types::*;
use crate::shared::constants::*;
use crate::fs_core::cow::{CowManager, CowMapping};
use crate::fs_core::inode::{Inode, InodeManager};
use crate::storage::StorageManager;

#[cfg(not(feature = "kernel"))]
use std::sync::{Arc, RwLock, Mutex};
#[cfg(not(feature = "kernel"))]
use std::collections::{HashMap, BTreeMap, BTreeSet};

#[cfg(feature = "kernel")]
use alloc::sync::Arc;
#[cfg(feature = "kernel")]
use alloc::collections::{BTreeMap, BTreeSet};

#[cfg(not(feature = "std"))]
use alloc::{vec::Vec, string::String, boxed::Box};
#[cfg(feature = "std")]
use std::{vec::Vec, string::String, boxed::Box};

/// Snapshot identifier
pub type SnapshotId = u64;

/// Snapshot metadata
#[derive(Debug, Clone)]
pub struct SnapshotMetadata {
    /// Unique snapshot identifier
    pub id: SnapshotId,
    /// Human-readable name
    pub name: String,
    /// Description of the snapshot
    pub description: String,
    /// Creation timestamp
    pub created_at: Timestamp,
    /// Root inode of the snapshot
    pub root_inode: InodeNumber,
    /// Parent snapshot (if any)
    pub parent_snapshot: Option<SnapshotId>,
    /// Child snapshots
    pub child_snapshots: Vec<SnapshotId>,
    /// Snapshot flags
    pub flags: SnapshotFlags,
    /// Size of snapshot data
    pub data_size: u64,
    /// Number of inodes in snapshot
    pub inode_count: u64,
    /// Reference count
    pub ref_count: u32,
}

impl SnapshotMetadata {
    /// Create new snapshot metadata
    pub fn new(id: SnapshotId, name: String, root_inode: InodeNumber) -> Self {
        Self {
            id,
            name,
            description: String::new(),
            created_at: 0, // TODO: Get current timestamp
            root_inode,
            parent_snapshot: None,
            child_snapshots: Vec::new(),
            flags: SnapshotFlags::ACTIVE,
            data_size: 0,
            inode_count: 0,
            ref_count: 1,
        }
    }

    /// Check if this snapshot is read-only
    pub fn is_readonly(&self) -> bool {
        self.flags.contains(SnapshotFlags::READONLY)
    }

    /// Check if this snapshot is marked for deletion
    pub fn is_marked_for_deletion(&self) -> bool {
        self.flags.contains(SnapshotFlags::MARKED_FOR_DELETION)
    }

    /// Add a child snapshot
    pub fn add_child(&mut self, child_id: SnapshotId) {
        if !self.child_snapshots.contains(&child_id) {
            self.child_snapshots.push(child_id);
        }
    }

    /// Remove a child snapshot
    pub fn remove_child(&mut self, child_id: SnapshotId) {
        self.child_snapshots.retain(|&id| id != child_id);
    }
}

/// Snapshot flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SnapshotFlags(pub u32);

impl SnapshotFlags {
    pub const ACTIVE: Self = Self(0x01);
    pub const READONLY: Self = Self(0x02);
    pub const COMPRESSED: Self = Self(0x04);
    pub const ENCRYPTED: Self = Self(0x08);
    pub const MARKED_FOR_DELETION: Self = Self(0x10);
    pub const INCREMENTAL: Self = Self(0x20);
    pub const VECTOR_ENABLED: Self = Self(0x40);

    pub fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    pub fn insert(&mut self, other: Self) {
        self.0 |= other.0;
    }

    pub fn remove(&mut self, other: Self) {
        self.0 &= !other.0;
    }
}

/// Delta entry representing changes between snapshots
#[derive(Debug, Clone)]
pub struct DeltaEntry {
    /// Inode number
    pub inode: InodeNumber,
    /// Type of change
    pub change_type: DeltaChangeType,
    /// Old data (for modifications and deletions)
    pub old_data: Option<Vec<u8>>,
    /// New data (for modifications and creations)
    pub new_data: Option<Vec<u8>>,
    /// Offset within file (for partial changes)
    pub offset: u64,
    /// Size of change
    pub size: u64,
    /// Checksum of data
    pub checksum: u32,
}

impl DeltaEntry {
    /// Create a new delta entry
    pub fn new(inode: InodeNumber, change_type: DeltaChangeType) -> Self {
        Self {
            inode,
            change_type,
            old_data: None,
            new_data: None,
            offset: 0,
            size: 0,
            checksum: 0,
        }
    }

    /// Calculate the space overhead of this delta entry
    pub fn space_overhead(&self) -> u64 {
        let mut overhead = core::mem::size_of::<DeltaEntry>() as u64;
        
        if let Some(ref old_data) = self.old_data {
            overhead += old_data.len() as u64;
        }
        
        if let Some(ref new_data) = self.new_data {
            overhead += new_data.len() as u64;
        }
        
        overhead
    }
}

/// Types of changes in delta entries
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeltaChangeType {
    /// File or directory created
    Created,
    /// File or directory modified
    Modified,
    /// File or directory deleted
    Deleted,
    /// Metadata changed
    MetadataChanged,
    /// Vector data modified
    VectorModified,
}

/// Delta storage for efficient incremental snapshots
#[derive(Debug)]
pub struct DeltaStorage {
    /// Base snapshot ID
    pub base_snapshot: SnapshotId,
    /// Target snapshot ID
    pub target_snapshot: SnapshotId,
    /// Delta entries
    pub entries: Vec<DeltaEntry>,
    /// Total size of delta data
    pub total_size: u64,
    /// Compression ratio achieved
    pub compression_ratio: f32,
    /// Creation timestamp
    pub created_at: Timestamp,
}

impl DeltaStorage {
    /// Create new delta storage
    pub fn new(base_snapshot: SnapshotId, target_snapshot: SnapshotId) -> Self {
        Self {
            base_snapshot,
            target_snapshot,
            entries: Vec::new(),
            total_size: 0,
            compression_ratio: 1.0,
            created_at: 0, // TODO: Get current timestamp
        }
    }

    /// Add a delta entry
    pub fn add_entry(&mut self, entry: DeltaEntry) {
        self.total_size += entry.space_overhead();
        self.entries.push(entry);
    }

    /// Calculate space efficiency
    pub fn space_efficiency(&self) -> f32 {
        if self.total_size == 0 {
            return 100.0;
        }
        
        let uncompressed_size = self.entries.iter()
            .map(|entry| entry.size)
            .sum::<u64>();
        
        if uncompressed_size == 0 {
            return 100.0;
        }
        
        (1.0 - (self.total_size as f32 / uncompressed_size as f32)) * 100.0
    }
}

/// Snapshot statistics
#[derive(Debug, Clone, Copy, Default)]
pub struct SnapshotStats {
    /// Total number of snapshots created
    pub total_snapshots: u64,
    /// Number of active snapshots
    pub active_snapshots: u64,
    /// Total space used by snapshots
    pub total_space_used: u64,
    /// Space saved through deduplication
    pub space_saved: u64,
    /// Number of garbage collection runs
    pub gc_runs: u64,
    /// Total space freed by GC
    pub space_freed: u64,
    /// Average snapshot creation time (nanoseconds)
    pub avg_creation_time_ns: u64,
    /// Average delta size
    pub avg_delta_size: u64,
}

impl SnapshotStats {
    /// Calculate space efficiency
    pub fn space_efficiency(&self) -> f64 {
        if self.total_space_used == 0 {
            return 100.0;
        }
        
        (self.space_saved as f64 / self.total_space_used as f64) * 100.0
    }

    /// Calculate compression ratio
    pub fn compression_ratio(&self) -> f64 {
        if self.total_space_used == 0 {
            return 1.0;
        }
        
        let uncompressed_size = self.total_space_used + self.space_saved;
        uncompressed_size as f64 / self.total_space_used as f64
    }
}

/// Garbage collection result
#[derive(Debug, Default)]
pub struct GarbageCollectionResult {
    /// Number of snapshots deleted
    pub snapshots_deleted: u64,
    /// Space freed in bytes
    pub space_freed: u64,
    /// Number of errors encountered
    pub errors: u64,
    /// Time taken for GC
    pub duration_ms: u64,
}

/// Snapshot manager for comprehensive snapshot operations
pub struct SnapshotManager {
    /// Active snapshots by ID
    #[cfg(not(feature = "kernel"))]
    snapshots: RwLock<HashMap<SnapshotId, Arc<RwLock<SnapshotMetadata>>>>,
    #[cfg(feature = "kernel")]
    snapshots: crate::shared::types::VexfsRwLock<BTreeMap<SnapshotId, Arc<crate::shared::types::VexfsRwLock<SnapshotMetadata>>>>,
    
    /// Delta storage by snapshot pair
    #[cfg(not(feature = "kernel"))]
    deltas: RwLock<HashMap<(SnapshotId, SnapshotId), DeltaStorage>>,
    #[cfg(feature = "kernel")]
    deltas: crate::shared::types::VexfsRwLock<BTreeMap<(SnapshotId, SnapshotId), DeltaStorage>>,
    
    /// Next snapshot ID
    #[cfg(not(feature = "kernel"))]
    next_snapshot_id: Mutex<SnapshotId>,
    #[cfg(feature = "kernel")]
    next_snapshot_id: crate::shared::types::VexfsMutex<SnapshotId>,
    
    /// Reference to CoW manager
    cow_manager: Arc<CowManager>,
    
    /// Reference to storage manager
    storage: Arc<StorageManager>,
    
    /// Snapshot statistics
    #[cfg(not(feature = "kernel"))]
    stats: Mutex<SnapshotStats>,
    #[cfg(feature = "kernel")]
    stats: crate::shared::types::VexfsMutex<SnapshotStats>,
}

impl SnapshotManager {
    /// Create a new snapshot manager
    pub fn new(cow_manager: Arc<CowManager>, storage: Arc<StorageManager>) -> Self {
        Self {
            #[cfg(not(feature = "kernel"))]
            snapshots: RwLock::new(HashMap::new()),
            #[cfg(feature = "kernel")]
            snapshots: crate::shared::types::VexfsRwLock::new(BTreeMap::new()),
            
            #[cfg(not(feature = "kernel"))]
            deltas: RwLock::new(HashMap::new()),
            #[cfg(feature = "kernel")]
            deltas: crate::shared::types::VexfsRwLock::new(BTreeMap::new()),
            
            #[cfg(not(feature = "kernel"))]
            next_snapshot_id: Mutex::new(1),
            #[cfg(feature = "kernel")]
            next_snapshot_id: crate::shared::types::VexfsMutex::new(1),
            
            cow_manager,
            storage,
            
            #[cfg(not(feature = "kernel"))]
            stats: Mutex::new(SnapshotStats::default()),
            #[cfg(feature = "kernel")]
            stats: crate::shared::types::VexfsMutex::new(SnapshotStats::default()),
        }
    }

    /// Create a new snapshot
    pub fn create_snapshot(&self, name: String, root_inode: InodeNumber, parent_snapshot: Option<SnapshotId>) -> VexfsResult<SnapshotId> {
        // Generate new snapshot ID
        let snapshot_id = {
            #[cfg(not(feature = "kernel"))]
            {
                let mut next_id = self.next_snapshot_id.lock().map_err(|_| VexfsError::LockConflict("failed to acquire ID lock".to_string()))?;
                let id = *next_id;
                *next_id += 1;
                id
            }
            #[cfg(feature = "kernel")]
            {
                let mut next_id = self.next_snapshot_id.lock();
                let id = *next_id;
                *next_id += 1;
                id
            }
        };

        // Create snapshot metadata
        let mut metadata = SnapshotMetadata::new(snapshot_id, name, root_inode);
        metadata.parent_snapshot = parent_snapshot;

        // Create CoW snapshot for the root inode
        let snapshot_inode = self.cow_manager.create_snapshot(root_inode)?;
        metadata.root_inode = snapshot_inode;

        // Store snapshot metadata
        #[cfg(not(feature = "kernel"))]
        {
            let mut snapshots = self.snapshots.write().map_err(|_| VexfsError::LockConflict("failed to acquire snapshots lock".to_string()))?;
            snapshots.insert(snapshot_id, Arc::new(RwLock::new(metadata.clone())));
        }
        #[cfg(feature = "kernel")]
        {
            let mut snapshots = self.snapshots.write();
            snapshots.insert(snapshot_id, Arc::new(crate::shared::types::VexfsRwLock::new(metadata.clone())));
        }

        // Update statistics
        #[cfg(not(feature = "kernel"))]
        {
            let mut stats = self.stats.lock().map_err(|_| VexfsError::LockConflict("failed to acquire stats lock".to_string()))?;
            stats.total_snapshots += 1;
            stats.active_snapshots += 1;
        }
        #[cfg(feature = "kernel")]
        {
            let mut stats = self.stats.lock();
            stats.total_snapshots += 1;
            stats.active_snapshots += 1;
        }

        Ok(snapshot_id)
    }

    /// Delete a snapshot
    pub fn delete_snapshot(&self, snapshot_id: SnapshotId, force: bool) -> VexfsResult<()> {
        // Get snapshot metadata
        let metadata_arc = {
            #[cfg(not(feature = "kernel"))]
            {
                let snapshots = self.snapshots.read().map_err(|_| VexfsError::LockConflict("failed to acquire snapshots lock".to_string()))?;
                snapshots.get(&snapshot_id)
                    .ok_or_else(|| VexfsError::InvalidArgument("snapshot not found".to_string()))?
                    .clone()
            }
            #[cfg(feature = "kernel")]
            {
                let snapshots = self.snapshots.read();
                snapshots.get(&snapshot_id)
                    .ok_or_else(|| VexfsError::InvalidArgument("snapshot not found".to_string()))?
                    .clone()
            }
        };

        // Check if snapshot can be deleted
        {
            #[cfg(not(feature = "kernel"))]
            let metadata = metadata_arc.read().map_err(|_| VexfsError::LockConflict("failed to acquire metadata lock".to_string()))?;
            #[cfg(feature = "kernel")]
            let metadata = metadata_arc.read();

            if !force && !metadata.child_snapshots.is_empty() {
                return Err(VexfsError::InvalidOperation("cannot delete snapshot with children".to_string()));
            }

            if metadata.ref_count > 1 && !force {
                return Err(VexfsError::InvalidOperation("snapshot is still referenced".to_string()));
            }
        }

        // Mark for deletion or delete immediately
        if force {
            self.immediate_delete(snapshot_id)?;
        } else {
            self.mark_for_deletion(snapshot_id)?;
        }

        Ok(())
    }

    /// List all snapshots
    pub fn list_snapshots(&self) -> VexfsResult<Vec<SnapshotMetadata>> {
        #[cfg(not(feature = "kernel"))]
        {
            let snapshots = self.snapshots.read().map_err(|_| VexfsError::LockConflict("failed to acquire snapshots lock".to_string()))?;
            let mut result = Vec::new();
            
            for snapshot_arc in snapshots.values() {
                let metadata = snapshot_arc.read().map_err(|_| VexfsError::LockConflict("failed to acquire metadata lock".to_string()))?;
                result.push(metadata.clone());
            }
            
            Ok(result)
        }
        #[cfg(feature = "kernel")]
        {
            let snapshots = self.snapshots.read();
            let mut result = Vec::new();
            
            for snapshot_arc in snapshots.values() {
                let metadata = snapshot_arc.read();
                result.push(metadata.clone());
            }
            
            Ok(result)
        }
    }

    /// Get snapshot metadata
    pub fn get_snapshot(&self, snapshot_id: SnapshotId) -> VexfsResult<SnapshotMetadata> {
        #[cfg(not(feature = "kernel"))]
        {
            let snapshots = self.snapshots.read().map_err(|_| VexfsError::LockConflict("failed to acquire snapshots lock".to_string()))?;
            let snapshot_arc = snapshots.get(&snapshot_id)
                .ok_or_else(|| VexfsError::InvalidArgument("snapshot not found".to_string()))?;
            let metadata = snapshot_arc.read().map_err(|_| VexfsError::LockConflict("failed to acquire metadata lock".to_string()))?;
            Ok(metadata.clone())
        }
        #[cfg(feature = "kernel")]
        {
            let snapshots = self.snapshots.read();
            let snapshot_arc = snapshots.get(&snapshot_id)
                .ok_or_else(|| VexfsError::InvalidArgument("snapshot not found".to_string()))?;
            let metadata = snapshot_arc.read();
            Ok(metadata.clone())
        }
    }

    /// Garbage collect obsolete snapshots
    pub fn garbage_collect(&self) -> VexfsResult<GarbageCollectionResult> {
        let mut result = GarbageCollectionResult::default();

        // Find snapshots marked for deletion
        let snapshots_to_delete = {
            #[cfg(not(feature = "kernel"))]
            {
                let snapshots = self.snapshots.read().map_err(|_| VexfsError::LockConflict("failed to acquire snapshots lock".to_string()))?;
                let mut to_delete = Vec::new();
                
                for (id, snapshot_arc) in snapshots.iter() {
                    let metadata = snapshot_arc.read().map_err(|_| VexfsError::LockConflict("failed to acquire metadata lock".to_string()))?;
                    if metadata.is_marked_for_deletion() && metadata.ref_count == 0 {
                        to_delete.push(*id);
                    }
                }
                
                to_delete
            }
            #[cfg(feature = "kernel")]
            {
                let snapshots = self.snapshots.read();
                let mut to_delete = Vec::new();
                
                for (id, snapshot_arc) in snapshots.iter() {
                    let metadata = snapshot_arc.read();
                    if metadata.is_marked_for_deletion() && metadata.ref_count == 0 {
                        to_delete.push(*id);
                    }
                }
                
                to_delete
            }
        };

        // Delete obsolete snapshots
        for snapshot_id in snapshots_to_delete {
            match self.immediate_delete(snapshot_id) {
                Ok(()) => {
                    result.snapshots_deleted += 1;
                    result.space_freed += 0; // TODO: Calculate actual space freed
                }
                Err(_) => {
                    result.errors += 1;
                }
            }
        }

        // Update statistics
        #[cfg(not(feature = "kernel"))]
        {
            let mut stats = self.stats.lock().map_err(|_| VexfsError::LockConflict("failed to acquire stats lock".to_string()))?;
            stats.gc_runs += 1;
            stats.space_freed += result.space_freed;
        }
        #[cfg(feature = "kernel")]
        {
            let mut stats = self.stats.lock();
            stats.gc_runs += 1;
            stats.space_freed += result.space_freed;
        }

        Ok(result)
    }

    /// Get snapshot statistics
    pub fn get_stats(&self) -> VexfsResult<SnapshotStats> {
        #[cfg(not(feature = "kernel"))]
        {
            let stats = self.stats.lock().map_err(|_| VexfsError::LockConflict("failed to acquire stats lock".to_string()))?;
            Ok(*stats)
        }
        #[cfg(feature = "kernel")]
        {
            let stats = self.stats.lock();
            Ok(*stats)
        }
    }

    // Private helper methods

    fn mark_for_deletion(&self, snapshot_id: SnapshotId) -> VexfsResult<()> {
        #[cfg(not(feature = "kernel"))]
        {
            let snapshots = self.snapshots.read().map_err(|_| VexfsError::LockConflict("failed to acquire snapshots lock".to_string()))?;
            let snapshot_arc = snapshots.get(&snapshot_id)
                .ok_or_else(|| VexfsError::InvalidArgument("snapshot not found".to_string()))?;
            let mut metadata = snapshot_arc.write().map_err(|_| VexfsError::LockConflict("failed to acquire metadata lock".to_string()))?;
            metadata.flags.insert(SnapshotFlags::MARKED_FOR_DELETION);
        }
        #[cfg(feature = "kernel")]
        {
            let snapshots = self.snapshots.read();
            let snapshot_arc = snapshots.get(&snapshot_id)
                .ok_or_else(|| VexfsError::InvalidArgument("snapshot not found".to_string()))?;
            let mut metadata = snapshot_arc.write();
            metadata.flags.insert(SnapshotFlags::MARKED_FOR_DELETION);
        }
        
        Ok(())
    }

    fn immediate_delete(&self, snapshot_id: SnapshotId) -> VexfsResult<()> {
        // Remove from snapshots map
        #[cfg(not(feature = "kernel"))]
        {
            let mut snapshots = self.snapshots.write().map_err(|_| VexfsError::LockConflict("failed to acquire snapshots lock".to_string()))?;
            snapshots.remove(&snapshot_id);
        }
        #[cfg(feature = "kernel")]
        {
            let mut snapshots = self.snapshots.write();
            snapshots.remove(&snapshot_id);
        }

        // Remove associated deltas
        #[cfg(not(feature = "kernel"))]
        {
            let mut deltas = self.deltas.write().map_err(|_| VexfsError::LockConflict("failed to acquire deltas lock".to_string()))?;
            deltas.retain(|(base, target), _| *base != snapshot_id && *target != snapshot_id);
        }
        #[cfg(feature = "kernel")]
        {
            let mut deltas = self.deltas.write();
            deltas.retain(|(base, target), _| *base != snapshot_id && *target != snapshot_id);
        }

        // Update statistics
        #[cfg(not(feature = "kernel"))]
        {
            let mut stats = self.stats.lock().map_err(|_| VexfsError::LockConflict("failed to acquire stats lock".to_string()))?;
            stats.active_snapshots = stats.active_snapshots.saturating_sub(1);
        }
        #[cfg(feature = "kernel")]
        {
            let mut stats = self.stats.lock();
            stats.active_snapshots = stats.active_snapshots.saturating_sub(1);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snapshot_metadata() {
        let mut metadata = SnapshotMetadata::new(1, "test_snapshot".to_string(), 100);
        assert_eq!(metadata.id, 1);
        assert_eq!(metadata.name, "test_snapshot");
        assert_eq!(metadata.root_inode, 100);
        assert!(!metadata.is_readonly());
        assert!(!metadata.is_marked_for_deletion());
        
        metadata.add_child(2);
        assert_eq!(metadata.child_snapshots.len(), 1);
        assert_eq!(metadata.child_snapshots[0], 2);
        
        metadata.remove_child(2);
        assert_eq!(metadata.child_snapshots.len(), 0);
    }

    #[test]
    fn test_snapshot_flags() {
        let mut flags = SnapshotFlags::ACTIVE;
        assert!(flags.contains(SnapshotFlags::ACTIVE));
        assert!(!flags.contains(SnapshotFlags::READONLY));
        
        flags.insert(SnapshotFlags::READONLY);
        assert!(flags.contains(SnapshotFlags::READONLY));
        
        flags.remove(SnapshotFlags::ACTIVE);
        assert!(!flags.contains(SnapshotFlags::ACTIVE));
        assert!(flags.contains(SnapshotFlags::READONLY));
    }

    #[test]
    fn test_delta_entry() {
        let mut entry = DeltaEntry::new(100, DeltaChangeType::Modified);
        assert_eq!(entry.inode, 100);
        assert_eq!(entry.change_type, DeltaChangeType::Modified);
        
        entry.new_data = Some(vec![1, 2, 3, 4]);
        entry.old_data = Some(vec![5, 6]);
        
        let overhead = entry.space_overhead();
        assert!(overhead > 0);
    }

    #[test]
    fn test_delta_storage() {
        let mut delta = DeltaStorage::new(1, 2);
        assert_eq!(delta.base_snapshot, 1);
        assert_eq!(delta.target_snapshot, 2);
        assert_eq!(delta.entries.len(), 0);
        
        let entry = DeltaEntry::new(100, DeltaChangeType::Created);
        delta.add_entry(entry);
        assert_eq!(delta.entries.len(), 1);
    }

    #[test]
    fn test_snapshot_stats() {
        let mut stats = SnapshotStats::default();
        stats.total_space_used = 1000;
        stats.space_saved = 200;
        
        assert_eq!(stats.space_efficiency(), 20.0);
        assert_eq!(stats.compression_ratio(), 1.2);
    }
}