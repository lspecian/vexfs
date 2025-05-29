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

//! Vector-specific Copy-on-Write Integration for VexFS
//!
//! This module provides specialized CoW functionality for vector embeddings,
//! ensuring atomic updates and efficient snapshot capabilities for vector data.

use crate::shared::errors::{VexfsError, VexfsResult};
use crate::shared::types::*;
use crate::shared::constants::*;
use crate::fs_core::cow::{CowManager, CowMapping, CowExtent, CowBlockRef};
use crate::fs_core::snapshot::{SnapshotManager, SnapshotId, DeltaEntry, DeltaChangeType};
use crate::storage::StorageManager;

#[cfg(not(feature = "kernel"))]
use std::sync::{Arc, RwLock, Mutex};
#[cfg(not(feature = "kernel"))]
use std::collections::{HashMap, BTreeMap};

#[cfg(feature = "kernel")]
use alloc::sync::Arc;
#[cfg(feature = "kernel")]
use alloc::collections::BTreeMap;

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

/// Vector-specific CoW manager for handling vector embedding updates
pub struct VectorCowManager {
    /// Base CoW manager
    cow_manager: Arc<CowManager>,
    
    /// Vector-specific mappings
    #[cfg(not(feature = "kernel"))]
    vector_mappings: RwLock<HashMap<InodeNumber, VectorCowMapping>>,
    #[cfg(feature = "kernel")]
    vector_mappings: crate::shared::types::VexfsRwLock<BTreeMap<InodeNumber, VectorCowMapping>>,
    
    /// Storage manager reference
    storage: Arc<StorageManager>,
    
    /// Vector CoW statistics
    #[cfg(not(feature = "kernel"))]
    stats: Mutex<VectorCowStats>,
    #[cfg(feature = "kernel")]
    stats: crate::shared::types::VexfsMutex<VectorCowStats>,
}

impl VectorCowManager {
    /// Create a new vector CoW manager
    pub fn new(cow_manager: Arc<CowManager>, storage: Arc<StorageManager>) -> Self {
        Self {
            cow_manager,
            #[cfg(not(feature = "kernel"))]
            vector_mappings: RwLock::new(HashMap::new()),
            #[cfg(feature = "kernel")]
            vector_mappings: crate::shared::types::VexfsRwLock::new(BTreeMap::new()),
            storage,
            #[cfg(not(feature = "kernel"))]
            stats: Mutex::new(VectorCowStats::default()),
            #[cfg(feature = "kernel")]
            stats: crate::shared::types::VexfsMutex::new(VectorCowStats::default()),
        }
    }

    /// Perform atomic vector update with CoW semantics
    pub fn cow_update_vector(
        &self,
        inode: InodeNumber,
        vector_index: usize,
        new_vector: &Vector,
    ) -> VexfsResult<()> {
        // Get or create vector mapping
        let vector_mapping = self.get_vector_mapping(inode)?;
        
        #[cfg(not(feature = "kernel"))]
        let mut mapping = vector_mapping.write().map_err(|_| VexfsError::LockConflict("failed to acquire vector mapping lock".to_string()))?;
        #[cfg(feature = "kernel")]
        let mut mapping = vector_mapping.write();
        
        // Calculate vector data layout
        let vector_size = new_vector.dimensions as usize * core::mem::size_of::<VectorComponent>();
        let vectors_per_block = VEXFS_DEFAULT_BLOCK_SIZE / vector_size;
        let block_index = vector_index / vectors_per_block;
        let block_offset = (vector_index % vectors_per_block) * vector_size;
        
        // Perform CoW on the block containing this vector
        let logical_block = mapping.vector_start_block + block_index as u64;
        let physical_block = self.cow_manager.cow_write(inode, logical_block, &[])?;
        
        // Read current block data
        let mut block_data = self.storage.read_block(physical_block)?;
        
        // Update vector data in the block
        let vector_bytes = vector_to_bytes(new_vector)?;
        if block_offset + vector_bytes.len() <= block_data.len() {
            block_data[block_offset..block_offset + vector_bytes.len()].copy_from_slice(&vector_bytes);
        } else {
            return Err(VexfsError::InvalidArgument("vector data exceeds block boundary".to_string()));
        }
        
        // Write updated block back
        self.storage.write_block(physical_block, block_data)?;
        
        // Update vector mapping metadata
        mapping.last_updated_vector = vector_index;
        mapping.update_count += 1;
        mapping.last_update_time = 0; // TODO: Get current timestamp
        
        // Update statistics
        #[cfg(not(feature = "kernel"))]
        {
            let mut stats = self.stats.lock().map_err(|_| VexfsError::LockConflict("failed to acquire stats lock".to_string()))?;
            stats.vector_updates += 1;
            stats.total_vectors_modified += 1;
        }
        #[cfg(feature = "kernel")]
        {
            let mut stats = self.stats.lock();
            stats.vector_updates += 1;
            stats.total_vectors_modified += 1;
        }
        
        Ok(())
    }

    /// Create a snapshot of vector data with delta compression
    pub fn create_vector_snapshot(
        &self,
        inode: InodeNumber,
        snapshot_manager: &SnapshotManager,
        snapshot_name: String,
    ) -> VexfsResult<SnapshotId> {
        // Get vector mapping
        let vector_mapping = self.get_vector_mapping(inode)?;
        
        #[cfg(not(feature = "kernel"))]
        let mapping = vector_mapping.read().map_err(|_| VexfsError::LockConflict("failed to acquire vector mapping lock".to_string()))?;
        #[cfg(feature = "kernel")]
        let mapping = vector_mapping.read();
        
        // Create base snapshot
        let snapshot_id = snapshot_manager.create_snapshot(
            snapshot_name,
            inode,
            mapping.parent_snapshot,
        )?;
        
        // Create delta entries for modified vectors
        let mut delta_entries = Vec::new();
        for &modified_vector in &mapping.modified_vectors {
            let delta_entry = DeltaEntry {
                inode,
                change_type: DeltaChangeType::VectorModified,
                old_data: None, // TODO: Store old vector data for rollback
                new_data: None, // TODO: Store new vector data
                offset: modified_vector as u64,
                size: mapping.vector_size as u64,
                checksum: 0, // TODO: Calculate checksum
            };
            delta_entries.push(delta_entry);
        }
        
        // Store delta entries (simplified - in practice would use DeltaStorage)
        // TODO: Implement proper delta storage integration
        
        Ok(snapshot_id)
    }

    /// Restore vector data from a snapshot
    pub fn restore_vector_from_snapshot(
        &self,
        inode: InodeNumber,
        snapshot_id: SnapshotId,
        vector_index: Option<usize>,
    ) -> VexfsResult<()> {
        // TODO: Implement vector restoration from snapshot
        // This would involve:
        // 1. Reading delta entries for the snapshot
        // 2. Applying reverse deltas to restore previous state
        // 3. Updating vector mappings
        
        Ok(())
    }

    /// Get vector-specific statistics
    pub fn get_vector_stats(&self) -> VexfsResult<VectorCowStats> {
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

    /// Optimize vector CoW mappings
    pub fn optimize_vector_mappings(&self) -> VexfsResult<VectorOptimizationResult> {
        let mut result = VectorOptimizationResult::default();
        
        #[cfg(not(feature = "kernel"))]
        let mappings = self.vector_mappings.read().map_err(|_| VexfsError::LockConflict("failed to acquire mappings lock".to_string()))?;
        #[cfg(feature = "kernel")]
        let mappings = self.vector_mappings.read();
        
        for (inode, mapping) in mappings.iter() {
            // Consolidate fragmented vector blocks
            if mapping.fragmentation_ratio > 0.3 {
                // TODO: Implement defragmentation
                result.mappings_optimized += 1;
                result.fragmentation_reduced += mapping.fragmentation_ratio;
            }
            
            // Compress rarely accessed vectors
            if mapping.access_frequency < 0.1 {
                // TODO: Implement compression
                result.vectors_compressed += mapping.vector_count as u64;
            }
        }
        
        Ok(result)
    }

    // Private helper methods
    
    fn get_vector_mapping(&self, inode: InodeNumber) -> VexfsResult<Arc<crate::shared::types::VexfsRwLock<VectorCowMapping>>> {
        #[cfg(not(feature = "kernel"))]
        {
            let mappings = self.vector_mappings.read().map_err(|_| VexfsError::LockConflict("failed to acquire mappings lock".to_string()))?;
            
            if let Some(mapping) = mappings.get(&inode) {
                Ok(Arc::new(RwLock::new(mapping.clone())))
            } else {
                drop(mappings);
                
                // Create new mapping
                let new_mapping = VectorCowMapping::new(inode);
                let mut mappings = self.vector_mappings.write().map_err(|_| VexfsError::LockConflict("failed to acquire mappings write lock".to_string()))?;
                mappings.insert(inode, new_mapping.clone());
                Ok(Arc::new(RwLock::new(new_mapping)))
            }
        }
        #[cfg(feature = "kernel")]
        {
            let mappings = self.vector_mappings.read();
            
            if let Some(mapping) = mappings.get(&inode) {
                Ok(Arc::new(crate::shared::types::VexfsRwLock::new(mapping.clone())))
            } else {
                drop(mappings);
                
                // Create new mapping
                let new_mapping = VectorCowMapping::new(inode);
                let mut mappings = self.vector_mappings.write();
                mappings.insert(inode, new_mapping.clone());
                Ok(Arc::new(crate::shared::types::VexfsRwLock::new(new_mapping)))
            }
        }
    }
}

/// Vector-specific CoW mapping
#[derive(Debug, Clone)]
pub struct VectorCowMapping {
    /// Inode number
    pub inode: InodeNumber,
    /// Starting block for vector data
    pub vector_start_block: BlockNumber,
    /// Number of vectors in this mapping
    pub vector_count: usize,
    /// Size of each vector in bytes
    pub vector_size: usize,
    /// Vector dimensions
    pub dimensions: VectorDimension,
    /// Last updated vector index
    pub last_updated_vector: usize,
    /// Update count
    pub update_count: u64,
    /// Last update timestamp
    pub last_update_time: Timestamp,
    /// Modified vector indices
    pub modified_vectors: Vec<usize>,
    /// Parent snapshot ID
    pub parent_snapshot: Option<SnapshotId>,
    /// Fragmentation ratio (0.0 = no fragmentation, 1.0 = fully fragmented)
    pub fragmentation_ratio: f32,
    /// Access frequency (0.0 = never accessed, 1.0 = frequently accessed)
    pub access_frequency: f32,
}

impl VectorCowMapping {
    /// Create a new vector CoW mapping
    pub fn new(inode: InodeNumber) -> Self {
        Self {
            inode,
            vector_start_block: 0,
            vector_count: 0,
            vector_size: 0,
            dimensions: 0,
            last_updated_vector: 0,
            update_count: 0,
            last_update_time: 0,
            modified_vectors: Vec::new(),
            parent_snapshot: None,
            fragmentation_ratio: 0.0,
            access_frequency: 1.0,
        }
    }

    /// Mark a vector as modified
    pub fn mark_vector_modified(&mut self, vector_index: usize) {
        if !self.modified_vectors.contains(&vector_index) {
            self.modified_vectors.push(vector_index);
        }
        self.last_updated_vector = vector_index;
        self.update_count += 1;
    }

    /// Calculate space efficiency
    pub fn space_efficiency(&self) -> f32 {
        if self.vector_count == 0 {
            return 100.0;
        }
        
        let total_space = self.vector_count * self.vector_size;
        let modified_space = self.modified_vectors.len() * self.vector_size;
        
        if total_space == 0 {
            100.0
        } else {
            ((total_space - modified_space) as f32 / total_space as f32) * 100.0
        }
    }
}

/// Vector CoW statistics
#[derive(Debug, Clone, Copy, Default)]
pub struct VectorCowStats {
    /// Number of vector updates performed
    pub vector_updates: u64,
    /// Total number of vectors modified
    pub total_vectors_modified: u64,
    /// Number of vector snapshots created
    pub vector_snapshots_created: u64,
    /// Space saved through vector CoW
    pub vector_space_saved: u64,
    /// Average vector update time (nanoseconds)
    pub avg_vector_update_time_ns: u64,
    /// Vector fragmentation ratio
    pub vector_fragmentation: f32,
}

impl VectorCowStats {
    /// Calculate vector space efficiency
    pub fn vector_space_efficiency(&self) -> f64 {
        if self.total_vectors_modified == 0 {
            return 100.0;
        }
        
        (self.vector_space_saved as f64 / (self.total_vectors_modified * VEXFS_DEFAULT_VECTOR_DIMS as u64 * 4) as f64) * 100.0
    }
}

/// Vector optimization result
#[derive(Debug, Default)]
pub struct VectorOptimizationResult {
    /// Number of mappings optimized
    pub mappings_optimized: u64,
    /// Number of vectors compressed
    pub vectors_compressed: u64,
    /// Fragmentation reduced (total ratio)
    pub fragmentation_reduced: f32,
    /// Space saved through optimization
    pub space_saved: u64,
}

// Helper functions

/// Convert vector to byte representation
fn vector_to_bytes(vector: &Vector) -> VexfsResult<Vec<u8>> {
    let mut bytes = Vec::with_capacity(vector.data.len() * 4);
    
    for &component in &vector.data {
        bytes.extend_from_slice(&component.to_le_bytes());
    }
    
    Ok(bytes)
}

/// Convert bytes to vector representation
fn bytes_to_vector(bytes: &[u8], dimensions: VectorDimension) -> VexfsResult<Vector> {
    if bytes.len() != dimensions as usize * 4 {
        return Err(VexfsError::InvalidArgument("invalid vector byte length".to_string()));
    }
    
    let mut data = Vec::with_capacity(dimensions as usize);
    
    for chunk in bytes.chunks_exact(4) {
        let component = f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        data.push(component);
    }
    
    Vector::new(dimensions, data).map_err(|e| VexfsError::InvalidArgument(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vector_cow_mapping_creation() {
        let mapping = VectorCowMapping::new(1);
        assert_eq!(mapping.inode, 1);
        assert_eq!(mapping.vector_count, 0);
        assert_eq!(mapping.update_count, 0);
        assert!(mapping.modified_vectors.is_empty());
    }

    #[test]
    fn test_vector_modification_tracking() {
        let mut mapping = VectorCowMapping::new(1);
        mapping.mark_vector_modified(5);
        mapping.mark_vector_modified(10);
        mapping.mark_vector_modified(5); // Duplicate should not be added
        
        assert_eq!(mapping.modified_vectors.len(), 2);
        assert!(mapping.modified_vectors.contains(&5));
        assert!(mapping.modified_vectors.contains(&10));
        assert_eq!(mapping.update_count, 3);
        assert_eq!(mapping.last_updated_vector, 5);
    }

    #[test]
    fn test_vector_bytes_conversion() {
        let vector = Vector::new(3, vec![1.0, 2.0, 3.0]).unwrap();
        let bytes = vector_to_bytes(&vector).unwrap();
        let restored = bytes_to_vector(&bytes, 3).unwrap();
        
        assert_eq!(vector.dimensions, restored.dimensions);
        assert_eq!(vector.data, restored.data);
    }

    #[test]
    fn test_vector_cow_stats() {
        let mut stats = VectorCowStats::default();
        stats.total_vectors_modified = 100;
        stats.vector_space_saved = 50 * VEXFS_DEFAULT_VECTOR_DIMS as u64 * 4;
        
        let efficiency = stats.vector_space_efficiency();
        assert!(efficiency > 0.0);
        assert!(efficiency <= 100.0);
    }

    #[test]
    fn test_space_efficiency_calculation() {
        let mut mapping = VectorCowMapping::new(1);
        mapping.vector_count = 100;
        mapping.vector_size = 512;
        mapping.modified_vectors = vec![1, 2, 3, 4, 5]; // 5 out of 100 modified
        
        let efficiency = mapping.space_efficiency();
        assert_eq!(efficiency, 95.0); // 95% efficiency (5% modified)
    }
}