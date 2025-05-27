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

//! Inode Management Module
//!
//! This module provides inode allocation, caching, and persistence services.
//! It integrates with the storage domain for block-level operations and maintains
//! inode metadata consistency.

use crate::shared::{
    errors::VexfsError,
    types::*,
    constants::*,
};
use crate::storage::StorageManager;
use super::FsResult;

extern crate alloc;
use alloc::sync::Arc; // Add Arc import
use alloc::collections::BTreeMap;
use core::mem;
use derivative::Derivative; // Import Derivative

/// In-memory inode representation
#[derive(Debug, Clone)] // Keep standard Debug for Inode for now
pub struct Inode {
    pub ino: InodeNumber,
    pub file_type: FileType,
    pub mode: FileMode,
    pub uid: UserId,
    pub gid: GroupId,
    pub size: FileSize,
    pub atime: Timestamp,
    pub mtime: Timestamp,
    pub ctime: Timestamp,
    pub nlink: LinkCount,
    pub blocks: u64,
    pub block_addrs: [BlockNumber; VEXFS_DIRECT_BLOCKS],
    pub indirect_block: BlockNumber,
    pub double_indirect: BlockNumber,
    pub triple_indirect: BlockNumber,
    pub dirty: bool,
    pub ref_count: u32,
}

impl Inode {
    /// Create a new inode with default values
    pub fn new(ino: InodeNumber, file_type: FileType, mode: FileMode, uid: UserId, gid: GroupId) -> Self {
        let now = 0; // TODO: Get current timestamp
        Self {
            ino,
            file_type,
            mode,
            uid,
            gid,
            size: 0,
            atime: now,
            mtime: now,
            ctime: now,
            nlink: 1,
            blocks: 0,
            block_addrs: [0; VEXFS_DIRECT_BLOCKS],
            indirect_block: 0,
            double_indirect: 0,
            triple_indirect: 0,
            dirty: true,
            ref_count: 1,
        }
    }

    /// Check if inode is a directory
    pub fn is_dir(&self) -> bool {
        self.file_type == FileType::Directory
    }

    /// Check if inode is a regular file
    pub fn is_file(&self) -> bool {
        self.file_type == FileType::Regular
    }

    /// Check if inode is a symbolic link
    pub fn is_symlink(&self) -> bool {
        self.file_type == FileType::Symlink
    }

    /// Check if inode is a directory (alias for is_dir)
    pub fn is_directory(&self) -> bool {
        self.is_dir()
    }

    /// Check if inode is a regular file (alias for is_file)
    pub fn is_regular_file(&self) -> bool {
        self.is_file()
    }

    /// Check if inode is a vector file
    pub fn is_vector_file(&self) -> bool {
        self.file_type == FileType::VectorFile
    }

    /// Mark inode as dirty (needs to be written to disk)
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// Update modification time
    pub fn touch_mtime(&mut self) {
        self.mtime = 0; // TODO: Get current timestamp
        self.mark_dirty();
    }

    /// Update access time
    pub fn touch_atime(&mut self) {
        self.atime = 0; // TODO: Get current timestamp
        self.mark_dirty();
    }

    /// Update change time
    pub fn touch_ctime(&mut self) {
        self.ctime = 0; // TODO: Get current timestamp
        self.mark_dirty();
    }

    /// Get inode statistics
    pub fn to_stat(&self) -> InodeStat {
        InodeStat {
            ino: self.ino,
            size: self.size,
            blocks: self.blocks,
            atime: self.atime,
            mtime: self.mtime,
            ctime: self.ctime,
            mode: self.mode,
            nlink: self.nlink,
            uid: self.uid,
            gid: self.gid,
            rdev: 0,
            blksize: VEXFS_DEFAULT_BLOCK_SIZE as u32,
        }
    }

    /// Increment reference count
    pub fn inc_ref(&mut self) {
        self.ref_count += 1;
    }

    /// Decrement reference count
    pub fn dec_ref(&mut self) -> u32 {
        if self.ref_count > 0 {
            self.ref_count -= 1;
        }
        self.ref_count
    }
}

// Public helper functions that were being imported
pub fn get_inode(manager: &mut InodeManager, inode_num: InodeNumber) -> FsResult<Arc<Inode>> {
    manager.get_inode(inode_num)
}

pub fn put_inode(manager: &mut InodeManager, inode: Arc<Inode>) -> FsResult<()> {
    manager.put_inode(inode)
}

pub fn create_inode(manager: &mut InodeManager, file_type: FileType, mode: FileMode, uid: UserId, gid: GroupId) -> FsResult<Arc<Inode>> {
    manager.create_inode(file_type, mode, uid, gid)
}

pub fn delete_inode(manager: &mut InodeManager, inode_num: InodeNumber) -> FsResult<()> {
    manager.delete_inode(inode_num)
}

pub fn deallocate_inode_blocks(inode_num: InodeNumber) -> FsResult<()> {
    // TODO: Implement block deallocation
    // This should integrate with the storage layer to free allocated blocks
    Ok(())
}

/// Inode manager for allocation, caching, and persistence
#[derive(Derivative)]
#[derivative(Debug)] // Apply Derivative Debug
#[derivative(PartialEq)] // Apply Derivative PartialEq
pub struct InodeManager {
    #[derivative(Debug="ignore")] // Ignore for Debug
    #[derivative(PartialEq="ignore")] // Ignore for PartialEq
    storage: StorageManager,
    inode_cache: BTreeMap<InodeNumber, Inode>,
    next_inode: InodeNumber,
    free_inodes: Vec<InodeNumber>,
    dirty_inodes: BTreeMap<InodeNumber, bool>,
}

impl InodeManager {
    /// Create a new inode manager
    pub fn new(storage: &StorageManager) -> FsResult<Self> {
        Ok(Self {
            storage: storage.clone(), // Assuming StorageManager is Clone or this needs adjustment
            inode_cache: BTreeMap::new(),
            next_inode: VEXFS_ROOT_INO + 1,
            free_inodes: Vec::new(),
            dirty_inodes: BTreeMap::new(),
        })
    }

    /// Internal helper to create and cache an inode.
    /// This is called by `allocate_inode`.
    fn create_and_cache_inode(&mut self, ino: InodeNumber, file_type: FileType, mode: FileMode, uid: UserId, gid: GroupId) {
        let inode = Inode::new(ino, file_type, mode, uid, gid);
        self.inode_cache.insert(ino, inode); // Store the Inode directly
        self.dirty_inodes.insert(ino, true);
    }
    
    /// Allocate a new inode number and create the inode structure.
    /// The inode is cached and marked dirty.
    pub fn allocate_inode(&mut self, file_type: FileType, mode: FileMode, uid: UserId, gid: GroupId) -> FsResult<InodeNumber> {
        let ino = if let Some(free_ino) = self.free_inodes.pop() {
            free_ino
        } else {
            let ino = self.next_inode;
            self.next_inode += 1;
            // TODO: Check for inode number overflow and superblock update
            ino
        };
        self.create_and_cache_inode(ino, file_type, mode, uid, gid);
        Ok(ino)
    }

    /// Create an inode and return an Arc to it.
    /// This is a higher-level function that uses allocate_inode and get_inode.
    pub fn create_inode(&mut self, file_type: FileType, mode: FileMode, uid: UserId, gid: GroupId) -> FsResult<Arc<Inode>> {
        let ino = self.allocate_inode(file_type, mode, uid, gid)?;
        self.get_inode(ino) // get_inode will now return Arc<Inode>
    }

    /// Deallocate an inode
    pub fn deallocate_inode(&mut self, ino: InodeNumber) -> FsResult<()> {
        if ino == VEXFS_ROOT_INO {
            return Err(VexfsError::InvalidOperation("Cannot deallocate root inode".into()));
        }

        // Remove from cache
        self.inode_cache.remove(&ino);
        self.dirty_inodes.remove(&ino);

        // Add to free list
        self.free_inodes.push(ino);
        // TODO: Persist free_inodes list or update bitmap on disk

        Ok(())
    }

    /// Get an Arc<Inode> from cache or load from storage
    pub fn get_inode(&mut self, ino: InodeNumber) -> FsResult<Arc<Inode>> {
        if !self.inode_cache.contains_key(&ino) {
            self.load_inode(ino)?; // load_inode will now insert Inode into cache
        }
        
        // Clone the Inode from cache and wrap in Arc
        self.inode_cache.get(&ino)
            .map(|inode_ref| Arc::new(inode_ref.clone())) // Clone Inode and wrap in Arc
            .ok_or_else(|| VexfsError::InodeNotFound(ino))
    }

    /// Get an Arc<Inode> for modification.
    /// The caller can use Arc::make_mut or clone the inner Inode.
    /// Marks the inode as dirty.
    pub fn get_inode_mut(&mut self, ino: InodeNumber) -> FsResult<Arc<Inode>> {
        if !self.inode_cache.contains_key(&ino) {
            self.load_inode(ino)?;
        }
        
        self.dirty_inodes.insert(ino, true);
        // Clone the Inode from cache and wrap in Arc
        self.inode_cache.get(&ino)
            .map(|inode_ref| Arc::new(inode_ref.clone())) // Clone Inode and wrap in Arc
            .ok_or_else(|| VexfsError::InodeNotFound(ino))
    }

    /// Put an updated inode (wrapped in Arc) into the cache and mark as dirty.
    pub fn put_inode(&mut self, inode_arc: Arc<Inode>) -> FsResult<()> {
        let ino = inode_arc.ino;
        // To store the Inode, not Arc<Inode>, we dereference and clone.
        // Or, change inode_cache to store Arc<Inode>.
        // For now, let's assume inode_cache stores Inode.
        let mut inode_to_store = (*inode_arc).clone();
        inode_to_store.mark_dirty(); // Ensure it's marked dirty
        self.inode_cache.insert(ino, inode_to_store);
        self.dirty_inodes.insert(ino, true);
        Ok(())
    }

    /// Get inode statistics
    pub fn get_stat(&mut self, ino: InodeNumber) -> FsResult<InodeStat> {
        let inode = self.get_inode(ino)?;
        Ok(inode.to_stat())
    }

    /// Load an inode from storage
    fn load_inode(&mut self, ino: InodeNumber) -> FsResult<()> {
        // Calculate inode location on disk
        let inode_size = mem::size_of::<Inode>() as u64;
        let inodes_per_block = VEXFS_DEFAULT_BLOCK_SIZE / inode_size;
        let block_num = (ino - 1) / inodes_per_block;
        let offset = ((ino - 1) % inodes_per_block) * inode_size;

        // Read inode from storage
        let mut buffer = vec![0u8; inode_size as usize];
        self.storage.read_block(block_num, offset, &mut buffer)?;

        // TODO: Deserialize inode from buffer
        // For now, create a dummy inode
        let inode = Inode::new(ino, FileType::Regular, FileMode::new(VEXFS_DEFAULT_FILE_MODE), 0, 0);
        self.inode_cache.insert(ino, inode);

        Ok(())
    }

    /// Write a dirty inode to storage
    fn write_inode(&mut self, ino: InodeNumber) -> FsResult<()> {
        let inode = self.inode_cache.get(&ino).ok_or_else(|| {
            VexfsError::InodeNotFound(ino)
        })?;

        // Calculate inode location on disk
        let inode_size = mem::size_of::<Inode>() as u64;
        let inodes_per_block = VEXFS_DEFAULT_BLOCK_SIZE / inode_size;
        let block_num = (ino - 1) / inodes_per_block;
        let offset = ((ino - 1) % inodes_per_block) * inode_size;

        // TODO: Serialize inode to buffer
        let buffer = vec![0u8; inode_size as usize];
        
        // Write inode to storage
        self.storage.write_block(block_num, offset, &buffer)?;

        Ok(())
    }

    /// Synchronize all dirty inodes to storage
    pub fn sync(&mut self) -> FsResult<()> {
        let dirty_inodes: Vec<InodeNumber> = self.dirty_inodes.keys().copied().collect();
        
        for ino in dirty_inodes {
            self.write_inode(ino)?;
            self.dirty_inodes.remove(&ino);
            
            // Mark inode as clean
            if let Some(inode) = self.inode_cache.get_mut(&ino) {
                inode.dirty = false;
            }
        }

        Ok(())
    }

    /// Evict unused inodes from cache
    pub fn evict_unused(&mut self) {
        let mut to_evict = Vec::new();
        
        for (ino, inode) in &self.inode_cache {
            if inode.ref_count == 0 && !inode.dirty {
                to_evict.push(*ino);
            }
        }
        
        for ino in to_evict {
            self.inode_cache.remove(&ino);
        }
    }

    /// Get cache statistics
    pub fn cache_stats(&self) -> (usize, usize) {
        (self.inode_cache.len(), self.dirty_inodes.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inode_creation() {
        let inode = Inode::new(1, FileType::Regular, FileMode::new(0o644), 1000, 1000);
        assert_eq!(inode.ino, 1);
        assert_eq!(inode.file_type, FileType::Regular);
        assert_eq!(inode.uid, 1000);
        assert_eq!(inode.gid, 1000);
        assert!(inode.dirty);
    }

    #[test]
    fn test_inode_stat_conversion() {
        let inode = Inode::new(42, FileType::Directory, FileMode::new(0o755), 0, 0);
        let stat = inode.to_stat();
        assert_eq!(stat.ino, 42);
        assert_eq!(stat.mode.permissions(), 0o755);
    }

    #[test]
    fn test_inode_reference_counting() {
        let mut inode = Inode::new(1, FileType::Regular, FileMode::new(0o644), 0, 0);
        assert_eq!(inode.ref_count, 1);
        
        inode.inc_ref();
        assert_eq!(inode.ref_count, 2);
        
        let count = inode.dec_ref();
        assert_eq!(count, 1);
        assert_eq!(inode.ref_count, 1);
    }
}