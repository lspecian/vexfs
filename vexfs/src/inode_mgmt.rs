//! Inode Management System for VexFS
//! 
//! This module implements the core inode data structure and management functions
//! including allocation, deallocation, reading, and writing of inodes.

#![no_std]

use crate::ondisk::*;

/// In-memory inode structure with additional metadata
#[derive(Debug, Clone)]
pub struct VexfsInodeInfo {
    /// On-disk inode data
    pub disk_inode: VexfsInode,
    
    /// Inode number
    pub ino: u64,
    
    /// Dirty flag - needs to be written to disk
    pub dirty: bool,
    
    /// Reference count for memory management
    pub ref_count: u32,
    
    /// Block allocation cache for performance
    pub cached_blocks: [u64; VEXFS_N_DIRECT],
    
    /// Indirect block cache
    pub indirect_block: Option<u64>,
    
    /// Double indirect block cache
    pub double_indirect_block: Option<u64>,
    
    /// Triple indirect block cache
    pub triple_indirect_block: Option<u64>,
}

/// Inode allocation and management operations
pub struct VexfsInodeManager {
    /// Superblock reference for metadata
    pub superblock: VexfsSuperblock,
    
    /// Cached inode table location
    pub inode_table_start: u64,
    
    /// Cached group descriptor information
    pub group_desc: VexfsGroupDesc,
}

/// Inode allocation state
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InodeAllocState {
    Free,
    Allocated,
    Reserved,
}

/// Error types for inode operations
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InodeError {
    NotFound,
    NoSpace,
    InvalidInode,
    IoError,
    PermissionDenied,
    AlreadyExists,
}

impl VexfsInodeInfo {
    /// Create a new in-memory inode structure
    pub fn new(ino: u64, mode: u16, uid: u16, gid: u16) -> Self {
        Self {
            disk_inode: VexfsInode::new(mode, uid, gid),
            ino,
            dirty: true,
            ref_count: 1,
            cached_blocks: [0; VEXFS_N_DIRECT],
            indirect_block: None,
            double_indirect_block: None,
            triple_indirect_block: None,
        }
    }
    
    /// Create from existing on-disk inode
    pub fn from_disk(ino: u64, disk_inode: VexfsInode) -> Self {
        let mut cached_blocks = [0; VEXFS_N_DIRECT];
        
        // Cache direct block pointers
        for i in 0..VEXFS_N_DIRECT {
            cached_blocks[i] = disk_inode.i_block[i] as u64;
        }
        
        Self {
            disk_inode,
            ino,
            dirty: false,
            ref_count: 1,
            cached_blocks,
            indirect_block: if disk_inode.i_block[12] != 0 { 
                Some(disk_inode.i_block[12] as u64) 
            } else { 
                None 
            },
            double_indirect_block: if disk_inode.i_block[13] != 0 { 
                Some(disk_inode.i_block[13] as u64) 
            } else { 
                None 
            },
            triple_indirect_block: if disk_inode.i_block[14] != 0 { 
                Some(disk_inode.i_block[14] as u64) 
            } else { 
                None 
            },
        }
    }
    
    /// Get file size
    pub fn size(&self) -> u64 {
        self.disk_inode.get_size()
    }
    
    /// Set file size and mark dirty
    pub fn set_size(&mut self, size: u64) {
        self.disk_inode.set_size(size);
        self.dirty = true;
    }
    
    /// Check if inode is a directory
    pub fn is_dir(&self) -> bool {
        self.disk_inode.is_dir()
    }
    
    /// Check if inode is a regular file
    pub fn is_file(&self) -> bool {
        self.disk_inode.is_file()
    }
    
    /// Get block number for a given file block offset
    pub fn get_block(&self, block_offset: u64) -> Option<u64> {
        if block_offset < VEXFS_N_DIRECT as u64 {
            // Direct blocks
            let block_num = self.cached_blocks[block_offset as usize];
            if block_num != 0 {
                Some(block_num)
            } else {
                None
            }
        } else {
            // For now, only handle direct blocks
            // Indirect blocks will be implemented in the file operations
            None
        }
    }
    
    /// Set block number for a given file block offset
    pub fn set_block(&mut self, block_offset: u64, block_num: u64) -> Result<(), InodeError> {
        if block_offset < VEXFS_N_DIRECT as u64 {
            // Direct blocks
            self.cached_blocks[block_offset as usize] = block_num;
            self.disk_inode.i_block[block_offset as usize] = block_num as u32;
            self.dirty = true;
            Ok(())
        } else {
            // For now, only handle direct blocks
            Err(InodeError::NoSpace)
        }
    }
    
    /// Increment reference count
    pub fn get(&mut self) -> u32 {
        self.ref_count += 1;
        self.ref_count
    }
    
    /// Decrement reference count
    pub fn put(&mut self) -> u32 {
        if self.ref_count > 0 {
            self.ref_count -= 1;
        }
        self.ref_count
    }
    
    /// Mark inode as dirty
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }
    
    /// Update access time
    pub fn touch_atime(&mut self, time: u32) {
        self.disk_inode.i_atime = time;
        self.dirty = true;
    }
    
    /// Update modification time
    pub fn touch_mtime(&mut self, time: u32) {
        self.disk_inode.i_mtime = time;
        self.dirty = true;
    }
    
    /// Update change time
    pub fn touch_ctime(&mut self, time: u32) {
        self.disk_inode.i_ctime = time;
        self.dirty = true;
    }
    
    /// Calculate number of blocks used by file
    pub fn calculate_blocks(&self) -> u32 {
        let size = self.size();
        let block_size = VEXFS_DEFAULT_BLOCK_SIZE as u64;
        ((size + block_size - 1) / block_size) as u32
    }
}

impl VexfsInodeManager {
    /// Create a new inode manager
    pub fn new(superblock: VexfsSuperblock, group_desc: VexfsGroupDesc) -> Self {
        let layout = VexfsLayout::calculate(&superblock);
        
        Self {
            superblock,
            inode_table_start: group_desc.bg_inode_table,
            group_desc,
        }
    }
    
    /// Calculate the block and offset for a given inode number
    pub fn inode_location(&self, ino: u64) -> (u64, u32) {
        // Inode numbers start at 1, array indices start at 0
        let inode_index = ino - 1;
        
        let inodes_per_block = self.superblock.s_block_size / self.superblock.s_inode_size as u32;
        let block_offset = inode_index / inodes_per_block as u64;
        let byte_offset = (inode_index % inodes_per_block as u64) * self.superblock.s_inode_size as u64;
        
        (self.inode_table_start + block_offset, byte_offset as u32)
    }
    
    /// Allocate a new inode number
    pub fn allocate_inode(&mut self) -> Result<u64, InodeError> {
        // This is a simplified allocation - in a real implementation,
        // we would scan the inode bitmap to find a free inode
        
        if self.superblock.s_free_inodes_count == 0 {
            return Err(InodeError::NoSpace);
        }
        
        // For now, use a simple counter approach
        // In a real implementation, we'd scan the bitmap
        let new_ino = (self.superblock.s_inodes_count - self.superblock.s_free_inodes_count + 1) as u64;
        
        if new_ino > self.superblock.s_inodes_count as u64 {
            return Err(InodeError::NoSpace);
        }
        
        Ok(new_ino)
    }
    
    /// Mark an inode as allocated in the bitmap
    pub fn mark_inode_used(&mut self, ino: u64) -> Result<(), InodeError> {
        if ino == 0 || ino > self.superblock.s_inodes_count as u64 {
            return Err(InodeError::InvalidInode);
        }
        
        // Update superblock counters
        if self.superblock.s_free_inodes_count > 0 {
            self.superblock.s_free_inodes_count -= 1;
        }
        
        // Update group descriptor
        if self.group_desc.bg_free_inodes_count > 0 {
            self.group_desc.bg_free_inodes_count -= 1;
        }
        
        Ok(())
    }
    
    /// Free an inode and mark it as available
    pub fn free_inode(&mut self, ino: u64) -> Result<(), InodeError> {
        if ino == 0 || ino > self.superblock.s_inodes_count as u64 {
            return Err(InodeError::InvalidInode);
        }
        
        if ino == VEXFS_ROOT_INO {
            return Err(InodeError::PermissionDenied);
        }
        
        // Update superblock counters
        self.superblock.s_free_inodes_count += 1;
        
        // Update group descriptor
        self.group_desc.bg_free_inodes_count += 1;
        
        Ok(())
    }
    
    /// Check if an inode number is valid
    pub fn is_valid_ino(&self, ino: u64) -> bool {
        ino > 0 && ino <= self.superblock.s_inodes_count as u64
    }
    
    /// Get the state of an inode (allocated/free)
    pub fn get_inode_state(&self, ino: u64) -> Result<InodeAllocState, InodeError> {
        if !self.is_valid_ino(ino) {
            return Err(InodeError::InvalidInode);
        }
        
        // For now, assume all inodes below the free count are allocated
        // In a real implementation, we'd check the bitmap
        if ino <= (self.superblock.s_inodes_count - self.superblock.s_free_inodes_count) as u64 {
            Ok(InodeAllocState::Allocated)
        } else if ino < VEXFS_FIRST_USER_INO {
            Ok(InodeAllocState::Reserved)
        } else {
            Ok(InodeAllocState::Free)
        }
    }
    
    /// Create a new inode with specified attributes
    pub fn create_inode(&mut self, mode: u16, uid: u16, gid: u16) -> Result<VexfsInodeInfo, InodeError> {
        let ino = self.allocate_inode()?;
        self.mark_inode_used(ino)?;
        
        let mut inode_info = VexfsInodeInfo::new(ino, mode, uid, gid);
        
        // Set creation time (in a real kernel, we'd get current time)
        let current_time = 0; // Placeholder - would be current timestamp
        inode_info.disk_inode.i_ctime = current_time;
        inode_info.disk_inode.i_mtime = current_time;
        inode_info.disk_inode.i_atime = current_time;
        
        Ok(inode_info)
    }
    
    /// Read an inode from disk (stub - would do actual I/O)
    pub fn read_inode(&self, ino: u64) -> Result<VexfsInodeInfo, InodeError> {
        if !self.is_valid_ino(ino) {
            return Err(InodeError::InvalidInode);
        }
        
        // In a real implementation, this would:
        // 1. Calculate inode location on disk
        // 2. Read the inode block
        // 3. Extract the specific inode
        // 4. Create VexfsInodeInfo from disk data
        
        let (block_num, offset) = self.inode_location(ino);
        
        // For now, create a dummy inode
        // In real implementation: read from block_num at offset
        let disk_inode = if ino == VEXFS_ROOT_INO {
            VexfsInode::new(S_IFDIR | 0o755, 0, 0)
        } else {
            return Err(InodeError::NotFound);
        };
        
        Ok(VexfsInodeInfo::from_disk(ino, disk_inode))
    }
    
    /// Write an inode to disk (stub - would do actual I/O)
    pub fn write_inode(&self, inode_info: &mut VexfsInodeInfo) -> Result<(), InodeError> {
        if !inode_info.dirty {
            return Ok(()); // Nothing to write
        }
        
        if !self.is_valid_ino(inode_info.ino) {
            return Err(InodeError::InvalidInode);
        }
        
        // In a real implementation, this would:
        // 1. Calculate inode location on disk
        // 2. Read the inode block
        // 3. Update the specific inode
        // 4. Write the block back to disk
        
        let (block_num, offset) = self.inode_location(inode_info.ino);
        
        // Update block count
        inode_info.disk_inode.i_blocks = inode_info.calculate_blocks();
        
        // Mark as clean
        inode_info.dirty = false;
        
        // In real implementation: write to block_num at offset
        
        Ok(())
    }
    
    /// Sync all dirty inodes to disk
    pub fn sync_inodes(&self) -> Result<(), InodeError> {
        // In a real implementation, this would iterate through
        // all cached inodes and write dirty ones to disk
        Ok(())
    }
    
    /// Get filesystem statistics
    pub fn get_stats(&self) -> (u32, u32, u32) {
        (
            self.superblock.s_inodes_count,
            self.superblock.s_free_inodes_count,
            self.superblock.s_inodes_count - self.superblock.s_free_inodes_count,
        )
    }
}

/// Inode cache for performance
pub struct VexfsInodeCache {
    /// Maximum number of cached inodes
    pub max_size: usize,
    
    /// Currently cached inodes (in a real implementation, this would be a HashMap)
    pub cached_count: usize,
}

impl VexfsInodeCache {
    /// Create a new inode cache
    pub fn new(max_size: usize) -> Self {
        Self {
            max_size,
            cached_count: 0,
        }
    }
    
    /// Look up an inode in the cache
    pub fn lookup(&self, ino: u64) -> Option<VexfsInodeInfo> {
        // In a real implementation, this would look up in a hash table
        None
    }
    
    /// Insert an inode into the cache
    pub fn insert(&mut self, inode_info: VexfsInodeInfo) -> Result<(), InodeError> {
        if self.cached_count >= self.max_size {
            // In a real implementation, we'd evict an LRU inode
            return Err(InodeError::NoSpace);
        }
        
        self.cached_count += 1;
        Ok(())
    }
    
    /// Remove an inode from the cache
    pub fn remove(&mut self, ino: u64) -> Option<VexfsInodeInfo> {
        // In a real implementation, this would remove from hash table
        if self.cached_count > 0 {
            self.cached_count -= 1;
        }
        None
    }
    
    /// Flush all dirty inodes in cache
    pub fn flush(&self, manager: &VexfsInodeManager) -> Result<(), InodeError> {
        // In a real implementation, iterate through cache and write dirty inodes
        Ok(())
    }
}

/// Utility functions for inode operations
impl VexfsInodeInfo {
    /// Convert to directory entry file type
    pub fn to_file_type(&self) -> u8 {
        VexfsDirEntry::mode_to_file_type(self.disk_inode.i_mode)
    }
    
    /// Check if inode has extended attributes
    pub fn has_xattr(&self) -> bool {
        self.disk_inode.i_file_acl != 0
    }
    
    /// Get effective UID (combining high and low parts)
    pub fn get_uid(&self) -> u32 {
        (self.disk_inode.l_i_uid_high as u32) << 16 | self.disk_inode.i_uid as u32
    }
    
    /// Get effective GID (combining high and low parts)
    pub fn get_gid(&self) -> u32 {
        (self.disk_inode.l_i_gid_high as u32) << 16 | self.disk_inode.i_gid as u32
    }
    
    /// Set effective UID (splitting into high and low parts)
    pub fn set_uid(&mut self, uid: u32) {
        self.disk_inode.i_uid = uid as u16;
        self.disk_inode.l_i_uid_high = (uid >> 16) as u16;
        self.dirty = true;
    }
    
    /// Set effective GID (splitting into high and low parts)
    pub fn set_gid(&mut self, gid: u32) {
        self.disk_inode.i_gid = gid as u16;
        self.disk_inode.l_i_gid_high = (gid >> 16) as u16;
        self.dirty = true;
    }
    
    /// Check permissions for a given operation
    pub fn check_permission(&self, uid: u32, gid: u32, mode: u16) -> bool {
        // Simplified permission check
        if uid == 0 {
            return true; // Root can do anything
        }
        
        if uid == self.get_uid() {
            return (self.disk_inode.i_mode & (mode << 6)) != 0; // Owner permissions
        }
        
        if gid == self.get_gid() {
            return (self.disk_inode.i_mode & (mode << 3)) != 0; // Group permissions
        }
        
        (self.disk_inode.i_mode & mode) != 0 // Other permissions
    }
}