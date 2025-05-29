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

//! Copy-on-Write (CoW) Implementation for VexFS
//!
//! This module provides atomic Copy-on-Write functionality for both regular file data
//! and vector embeddings, enabling efficient snapshots and versioning capabilities.

use crate::shared::errors::{VexfsError, VexfsResult};
use crate::shared::types::*;
use crate::shared::constants::*;
use crate::storage::{StorageManager, TransactionManager};
use crate::fs_core::inode::{Inode, InodeManager};

#[cfg(not(feature = "kernel"))]
use std::sync::{Arc, RwLock, Mutex};
#[cfg(not(feature = "kernel"))]
use std::collections::{HashMap, BTreeMap};

#[cfg(feature = "kernel")]
use alloc::sync::Arc;
#[cfg(feature = "kernel")]
use alloc::collections::{BTreeMap};

#[cfg(not(feature = "std"))]
use alloc::{vec::Vec, string::String, boxed::Box};
#[cfg(feature = "std")]
#[cfg(feature = "kernel")]
use alloc::{vec::Vec, string::String, boxed::Box};
#[cfg(not(feature = "kernel"))]
use std::{vec::Vec, string::String, boxed::Box};

#[cfg(feature = "kernel")]
use alloc::string::ToString;
#[cfg(not(feature = "kernel"))]
use std::string::ToString;

/// Copy-on-Write block reference
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CowBlockRef {
    /// Original block number
    pub original_block: BlockNumber,
    /// Current block number (may be different after CoW)
    pub current_block: BlockNumber,
    /// Reference count for this block
    pub ref_count: u32,
    /// Generation number for versioning
    pub generation: u32,
    /// Flags for block state
    pub flags: CowBlockFlags,
}

impl CowBlockRef {
    /// Create a new CoW block reference
    pub fn new(block_number: BlockNumber) -> Self {
        Self {
            original_block: block_number,
            current_block: block_number,
            ref_count: 1,
            generation: 0,
            flags: CowBlockFlags::ORIGINAL,
        }
    }

    /// Check if this block has been copied
    pub fn is_copied(&self) -> bool {
        self.original_block != self.current_block
    }

    /// Check if this block is shared
    pub fn is_shared(&self) -> bool {
        self.ref_count > 1
    }

    /// Check if this block needs CoW on write
    pub fn needs_cow(&self) -> bool {
        self.is_shared() || self.flags.contains(CowBlockFlags::SNAPSHOT)
    }
}

/// CoW block flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CowBlockFlags(pub u32);

impl CowBlockFlags {
    pub const ORIGINAL: Self = Self(0x01);
    pub const COPIED: Self = Self(0x02);
    pub const SHARED: Self = Self(0x04);
    pub const SNAPSHOT: Self = Self(0x08);
    pub const DIRTY: Self = Self(0x10);
    pub const COMPRESSED: Self = Self(0x20);

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

/// CoW extent representing a contiguous range of blocks
#[derive(Debug, Clone)]
pub struct CowExtent {
    /// Starting logical block offset
    pub logical_start: u64,
    /// Number of blocks in this extent
    pub block_count: u32,
    /// CoW block references
    pub blocks: Vec<CowBlockRef>,
    /// Extent flags
    pub flags: CowExtentFlags,
    /// Creation timestamp
    pub created_at: Timestamp,
    /// Last modification timestamp
    pub modified_at: Timestamp,
}

impl CowExtent {
    /// Create a new CoW extent
    pub fn new(logical_start: u64, physical_blocks: Vec<BlockNumber>) -> Self {
        let blocks: Vec<CowBlockRef> = physical_blocks.into_iter()
            .map(|block_num| CowBlockRef::new(block_num))
            .collect();
        
        let now = 0; // TODO: Get current timestamp
        
        Self {
            logical_start,
            block_count: blocks.len() as u32,
            blocks,
            flags: CowExtentFlags::ACTIVE,
            created_at: now,
            modified_at: now,
        }
    }

    /// Get physical block number for logical offset
    pub fn get_physical_block(&self, logical_offset: u64) -> Option<BlockNumber> {
        if logical_offset < self.logical_start || 
           logical_offset >= self.logical_start + self.block_count as u64 {
            return None;
        }

        let index = (logical_offset - self.logical_start) as usize;
        self.blocks.get(index).map(|block_ref| block_ref.current_block)
    }

    /// Check if any blocks in this extent need CoW
    pub fn needs_cow(&self) -> bool {
        self.blocks.iter().any(|block_ref| block_ref.needs_cow())
    }

    /// Perform CoW on a specific block within this extent
    pub fn cow_block(&mut self, logical_offset: u64, new_block: BlockNumber) -> VexfsResult<()> {
        if logical_offset < self.logical_start || 
           logical_offset >= self.logical_start + self.block_count as u64 {
            return Err(VexfsError::InvalidArgument("offset out of range".to_string()));
        }

        let index = (logical_offset - self.logical_start) as usize;
        if let Some(block_ref) = self.blocks.get_mut(index) {
            // Decrease reference count on original block
            if block_ref.ref_count > 0 {
                block_ref.ref_count -= 1;
            }

            // Update to new block
            block_ref.current_block = new_block;
            block_ref.generation += 1;
            block_ref.flags.remove(CowBlockFlags::ORIGINAL);
            block_ref.flags.insert(CowBlockFlags::COPIED);
            block_ref.ref_count = 1;

            self.modified_at = 0; // TODO: Get current timestamp
            Ok(())
        } else {
            Err(VexfsError::InvalidArgument("invalid block index".to_string()))
        }
    }
}

/// CoW extent flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CowExtentFlags(pub u32);

impl CowExtentFlags {
    pub const ACTIVE: Self = Self(0x01);
    pub const SNAPSHOT: Self = Self(0x02);
    pub const COMPRESSED: Self = Self(0x04);
    pub const ENCRYPTED: Self = Self(0x08);
    pub const VECTOR_DATA: Self = Self(0x10);

    pub fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    pub fn insert(&mut self, other: Self) {
        self.0 |= other.0;
    }
}

/// CoW mapping for an inode
#[derive(Debug)]
pub struct CowMapping {
    /// Inode number this mapping belongs to
    pub inode: InodeNumber,
    /// Ordered extents by logical offset
    pub extents: BTreeMap<u64, CowExtent>,
    /// Total logical size
    pub logical_size: u64,
    /// CoW generation counter
    pub generation: u32,
    /// Mapping flags
    pub flags: CowMappingFlags,
    /// Reference count for this mapping
    pub ref_count: u32,
    /// Parent mapping (for snapshots)
    pub parent_mapping: Option<Box<CowMapping>>,
}

impl CowMapping {
    /// Create a new CoW mapping
    pub fn new(inode: InodeNumber) -> Self {
        Self {
            inode,
            extents: BTreeMap::new(),
            logical_size: 0,
            generation: 0,
            flags: CowMappingFlags::ACTIVE,
            ref_count: 1,
            parent_mapping: None,
        }
    }

    /// Add an extent to the mapping
    pub fn add_extent(&mut self, extent: CowExtent) -> VexfsResult<()> {
        let logical_start = extent.logical_start;
        
        // Check for overlaps
        if self.has_overlap(logical_start, extent.block_count as u64) {
            return Err(VexfsError::InvalidArgument("extent overlaps existing extent".to_string()));
        }

        // Update logical size if needed
        let extent_end = logical_start + extent.block_count as u64;
        if extent_end > self.logical_size {
            self.logical_size = extent_end;
        }

        self.extents.insert(logical_start, extent);
        self.generation += 1;
        Ok(())
    }

    /// Get physical block for logical offset
    pub fn get_physical_block(&self, logical_offset: u64) -> Option<BlockNumber> {
        // Find the extent containing this offset
        for (start_offset, extent) in self.extents.iter() {
            if logical_offset >= *start_offset && 
               logical_offset < *start_offset + extent.block_count as u64 {
                return extent.get_physical_block(logical_offset);
            }
        }

        // Check parent mapping if this is a snapshot
        if let Some(ref parent) = self.parent_mapping {
            return parent.get_physical_block(logical_offset);
        }

        None
    }

    /// Perform CoW on a logical block
    pub fn cow_block(&mut self, logical_offset: u64, storage: &StorageManager) -> VexfsResult<BlockNumber> {
        // Find the extent containing this offset
        for (start_offset, extent) in self.extents.iter_mut() {
            if logical_offset >= *start_offset && 
               logical_offset < *start_offset + extent.block_count as u64 {
                
                // Check if CoW is needed
                let block_index = (logical_offset - start_offset) as usize;
                if let Some(block_ref) = extent.blocks.get(block_index) {
                    if !block_ref.needs_cow() {
                        return Ok(block_ref.current_block);
                    }

                    // Allocate new block
                    let new_blocks = storage.allocate_blocks(1, Some(block_ref.current_block))?;
                    let new_block = new_blocks[0];

                    // Copy data from old block to new block
                    let old_data = storage.read_block(block_ref.current_block)?;
                    storage.write_block(new_block, old_data)?;

                    // Update extent
                    extent.cow_block(logical_offset, new_block)?;
                    self.generation += 1;

                    return Ok(new_block);
                }
            }
        }

        Err(VexfsError::InvalidArgument("logical offset not found".to_string()))
    }

    /// Create a snapshot of this mapping
    pub fn create_snapshot(&self) -> CowMapping {
        let mut snapshot = CowMapping::new(self.inode);
        snapshot.parent_mapping = Some(Box::new(self.clone()));
        snapshot.logical_size = self.logical_size;
        snapshot.flags = CowMappingFlags::SNAPSHOT;
        
        // Mark all blocks in parent as shared
        // Note: In a real implementation, we'd need to update reference counts
        
        snapshot
    }

    /// Check if a range overlaps with existing extents
    fn has_overlap(&self, start: u64, length: u64) -> bool {
        let end = start + length;
        
        for (extent_start, extent) in self.extents.iter() {
            let extent_end = extent_start + extent.block_count as u64;
            if start < extent_end && end > *extent_start {
                return true;
            }
        }
        
        false
    }
}

impl Clone for CowMapping {
    fn clone(&self) -> Self {
        Self {
            inode: self.inode,
            extents: self.extents.clone(),
            logical_size: self.logical_size,
            generation: self.generation,
            flags: self.flags,
            ref_count: 1, // New clone gets its own reference
            parent_mapping: self.parent_mapping.clone(),
        }
    }
}

/// CoW mapping flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CowMappingFlags(pub u32);

impl CowMappingFlags {
    pub const ACTIVE: Self = Self(0x01);
    pub const SNAPSHOT: Self = Self(0x02);
    pub const READONLY: Self = Self(0x04);
    pub const COMPRESSED: Self = Self(0x08);
    pub const VECTOR_ENABLED: Self = Self(0x10);

    pub fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    pub fn insert(&mut self, other: Self) {
        self.0 |= other.0;
    }
}

/// CoW manager for handling Copy-on-Write operations
pub struct CowManager {
    /// Active CoW mappings by inode
    #[cfg(not(feature = "kernel"))]
    mappings: RwLock<HashMap<InodeNumber, Arc<RwLock<CowMapping>>>>,
    #[cfg(feature = "kernel")]
    mappings: crate::shared::types::VexfsRwLock<BTreeMap<InodeNumber, Arc<crate::shared::types::VexfsRwLock<CowMapping>>>>,
    
    /// Reference to storage manager
    storage: Arc<StorageManager>,
    
    /// CoW statistics
    #[cfg(not(feature = "kernel"))]
    stats: Mutex<CowStats>,
    #[cfg(feature = "kernel")]
    stats: crate::shared::types::VexfsMutex<CowStats>,
}

impl CowManager {
    /// Create a new CoW manager
    pub fn new(storage: Arc<StorageManager>) -> Self {
        Self {
            #[cfg(not(feature = "kernel"))]
            mappings: RwLock::new(HashMap::new()),
            #[cfg(feature = "kernel")]
            mappings: crate::shared::types::VexfsRwLock::new(BTreeMap::new()),
            
            storage,
            
            #[cfg(not(feature = "kernel"))]
            stats: Mutex::new(CowStats::default()),
            #[cfg(feature = "kernel")]
            stats: crate::shared::types::VexfsMutex::new(CowStats::default()),
        }
    }

    /// Get or create CoW mapping for an inode
    pub fn get_mapping(&self, inode: InodeNumber) -> VexfsResult<Arc<crate::shared::types::VexfsRwLock<CowMapping>>> {
        #[cfg(not(feature = "kernel"))]
        {
            let mappings = self.mappings.read().map_err(|_| VexfsError::LockConflict("failed to acquire read lock".to_string()))?;
            
            if let Some(mapping) = mappings.get(&inode) {
                Ok(mapping.clone())
            } else {
                drop(mappings);
                
                // Create new mapping
                let new_mapping = Arc::new(RwLock::new(CowMapping::new(inode)));
                let mut mappings = self.mappings.write().map_err(|_| VexfsError::LockConflict("failed to acquire write lock".to_string()))?;
                mappings.insert(inode, new_mapping.clone());
                Ok(new_mapping)
            }
        }
        
        #[cfg(feature = "kernel")]
        {
            let mappings = self.mappings.read();
            
            if let Some(mapping) = mappings.get(&inode) {
                Ok(mapping.clone())
            } else {
                drop(mappings);
                
                // Create new mapping
                let new_mapping = Arc::new(crate::shared::types::VexfsRwLock::new(CowMapping::new(inode)));
                let mut mappings = self.mappings.write();
                mappings.insert(inode, new_mapping.clone());
                Ok(new_mapping)
            }
        }
    }

    /// Perform CoW operation on a block
    pub fn cow_write(&self, inode: InodeNumber, logical_offset: u64, data: &[u8]) -> VexfsResult<BlockNumber> {
        let mapping_arc = self.get_mapping(inode)?;
        
        #[cfg(not(feature = "kernel"))]
        let mut mapping = mapping_arc.write().map_err(|_| VexfsError::LockConflict("failed to acquire write lock".to_string()))?;
        #[cfg(feature = "kernel")]
        let mut mapping = mapping_arc.write();
        
        // Perform CoW if needed
        let physical_block = mapping.cow_block(logical_offset, &self.storage)?;
        
        // Write data to the (possibly new) block
        self.storage.write_block(physical_block, data.to_vec())?;
        
        // Update statistics
        #[cfg(not(feature = "kernel"))]
        {
            let mut stats = self.stats.lock().map_err(|_| VexfsError::LockConflict("failed to acquire stats lock".to_string()))?;
            stats.cow_operations += 1;
            stats.blocks_copied += 1;
        }
        #[cfg(feature = "kernel")]
        {
            let mut stats = self.stats.lock();
            stats.cow_operations += 1;
            stats.blocks_copied += 1;
        }
        
        Ok(physical_block)
    }

    /// Read data with CoW awareness
    pub fn cow_read(&self, inode: InodeNumber, logical_offset: u64) -> VexfsResult<Vec<u8>> {
        let mapping_arc = self.get_mapping(inode)?;
        
        #[cfg(not(feature = "kernel"))]
        let mapping = mapping_arc.read().map_err(|_| VexfsError::LockConflict("failed to acquire read lock".to_string()))?;
        #[cfg(feature = "kernel")]
        let mapping = mapping_arc.read();
        
        if let Some(physical_block) = mapping.get_physical_block(logical_offset) {
            self.storage.read_block(physical_block)
        } else {
            Err(VexfsError::InvalidArgument("logical offset not mapped".to_string()))
        }
    }

    /// Create a snapshot of an inode's CoW mapping
    pub fn create_snapshot(&self, inode: InodeNumber) -> VexfsResult<InodeNumber> {
        let mapping_arc = self.get_mapping(inode)?;
        
        #[cfg(not(feature = "kernel"))]
        let mapping = mapping_arc.read().map_err(|_| VexfsError::LockConflict("failed to acquire read lock".to_string()))?;
        #[cfg(feature = "kernel")]
        let mapping = mapping_arc.read();
        
        let snapshot_mapping = mapping.create_snapshot();
        let snapshot_inode = snapshot_mapping.inode; // In practice, this would be a new inode number
        
        drop(mapping);
        
        // Store the snapshot mapping
        #[cfg(not(feature = "kernel"))]
        {
            let mut mappings = self.mappings.write().map_err(|_| VexfsError::LockConflict("failed to acquire write lock".to_string()))?;
            mappings.insert(snapshot_inode, Arc::new(RwLock::new(snapshot_mapping)));
        }
        #[cfg(feature = "kernel")]
        {
            let mut mappings = self.mappings.write();
            mappings.insert(snapshot_inode, Arc::new(crate::shared::types::VexfsRwLock::new(snapshot_mapping)));
        }
        
        // Update statistics
        #[cfg(not(feature = "kernel"))]
        {
            let mut stats = self.stats.lock().map_err(|_| VexfsError::LockConflict("failed to acquire stats lock".to_string()))?;
            stats.snapshots_created += 1;
        }
        #[cfg(feature = "kernel")]
        {
            let mut stats = self.stats.lock();
            stats.snapshots_created += 1;
        }
        
        Ok(snapshot_inode)
    }

    /// Get CoW statistics
    pub fn get_stats(&self) -> VexfsResult<CowStats> {
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

    /// Remove a CoW mapping (for cleanup)
    pub fn remove_mapping(&self, inode: InodeNumber) -> VexfsResult<()> {
        #[cfg(not(feature = "kernel"))]
        {
            let mut mappings = self.mappings.write().map_err(|_| VexfsError::LockConflict("failed to acquire write lock".to_string()))?;
            mappings.remove(&inode);
        }
        #[cfg(feature = "kernel")]
        {
            let mut mappings = self.mappings.write();
            mappings.remove(&inode);
        }
        
        Ok(())
    }
}

/// CoW statistics
#[derive(Debug, Clone, Copy, Default)]
pub struct CowStats {
    /// Number of CoW operations performed
    pub cow_operations: u64,
    /// Number of blocks copied
    pub blocks_copied: u64,
    /// Number of snapshots created
    pub snapshots_created: u64,
    /// Number of active mappings
    pub active_mappings: u64,
    /// Total space saved by sharing
    pub space_saved: u64,
    /// Average CoW overhead
    pub avg_cow_overhead_ns: u64,
}

impl CowStats {
    /// Calculate space efficiency
    pub fn space_efficiency(&self) -> f64 {
        if self.blocks_copied == 0 {
            return 100.0;
        }
        
        (self.space_saved as f64 / (self.blocks_copied as u64 * VEXFS_DEFAULT_BLOCK_SIZE as u64) as f64) * 100.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cow_block_ref() {
        let mut block_ref = CowBlockRef::new(100);
        assert_eq!(block_ref.original_block, 100);
        assert_eq!(block_ref.current_block, 100);
        assert!(!block_ref.is_copied());
        assert!(!block_ref.is_shared());
        
        block_ref.ref_count = 2;
        assert!(block_ref.is_shared());
        assert!(block_ref.needs_cow());
    }

    #[test]
    fn test_cow_extent() {
        let blocks = vec![100, 101, 102];
        let mut extent = CowExtent::new(0, blocks);
        
        assert_eq!(extent.logical_start, 0);
        assert_eq!(extent.block_count, 3);
        assert_eq!(extent.get_physical_block(1), Some(101));
        assert_eq!(extent.get_physical_block(5), None);
        
        // Test CoW operation
        extent.cow_block(1, 200).unwrap();
        assert_eq!(extent.get_physical_block(1), Some(200));
    }

    #[test]
    fn test_cow_mapping() {
        let mut mapping = CowMapping::new(1);
        
        let blocks = vec![100, 101, 102];
        let extent = CowExtent::new(0, blocks);
        mapping.add_extent(extent).unwrap();
        
        assert_eq!(mapping.get_physical_block(1), Some(101));
        assert_eq!(mapping.logical_size, 3);
        
        // Test snapshot creation
        let snapshot = mapping.create_snapshot();
        assert!(snapshot.flags.contains(CowMappingFlags::SNAPSHOT));
        assert!(snapshot.parent_mapping.is_some());
    }
}