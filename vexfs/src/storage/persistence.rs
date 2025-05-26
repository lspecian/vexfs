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

//! Persistence and Serialization
//!
//! This module provides on-disk serialization and data persistence capabilities
//! for VexFS structures, ensuring data integrity and efficient storage.

use crate::shared::errors::{VexfsError, VexfsResult};
use crate::ondisk::{S_IFMT, S_IFDIR, S_IFREG, S_IFLNK};
use crate::shared::types::*;
use crate::shared::constants::*;
use crate::shared::utils::*;
use core::mem;
use core::slice;

/// Trait for on-disk serialization of VexFS structures
pub trait OnDiskSerializable {
    /// Serialize structure to byte array
    fn to_bytes(&self) -> &[u8];
    
    /// Deserialize structure from byte array
    fn from_bytes(data: &[u8]) -> VexfsResult<Self> where Self: Sized;
    
    /// Get the size of the serialized structure
    fn serialized_size() -> usize where Self: Sized;
    
    /// Validate the structure integrity
    fn validate(&self) -> VexfsResult<()>;
    
    /// Update structure checksum if applicable
    fn update_checksum(&mut self);
}

/// On-disk inode structure (128 bytes)
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct DiskInode {
    /// File mode and type
    pub i_mode: u16,
    /// Owner UID
    pub i_uid: u16,
    /// Size in bytes (lower 32 bits)
    pub i_size_lo: u32,
    /// Access time
    pub i_atime: u32,
    /// Creation time
    pub i_ctime: u32,
    /// Modification time
    pub i_mtime: u32,
    /// Deletion time
    pub i_dtime: u32,
    /// Group ID
    pub i_gid: u16,
    /// Hard link count
    pub i_links_count: u16,
    /// Number of 512-byte sectors used
    pub i_blocks: u32,
    /// File flags
    pub i_flags: u32,
    /// OS specific field 1
    pub l_i_version: u32,
    /// Direct block pointers
    pub i_block: [u32; 15],
    /// File version (for NFS)
    pub i_generation: u32,
    /// Extended attributes block
    pub i_file_acl: u32,
    /// Size in bytes (upper 32 bits) or directory ACL
    pub i_size_high: u32,
    /// Fragment address (obsolete)
    pub i_faddr: u32,
    /// Fragment number (obsolete)
    pub l_i_frag: u8,
    /// Fragment size (obsolete)
    pub l_i_fsize: u8,
    /// Padding
    pub i_pad1: u16,
    /// High 16-bits of UID
    pub l_i_uid_high: u16,
    /// High 16-bits of GID
    pub l_i_gid_high: u16,
    /// Reserved
    pub l_i_reserved2: u32,
    /// Extra inode size
    pub i_extra_isize: u16,
    /// Padding to 128 bytes
    pub i_pad: [u8; 2],
}

impl OnDiskSerializable for DiskInode {
    fn to_bytes(&self) -> &[u8] {
        unsafe {
            slice::from_raw_parts(
                self as *const Self as *const u8,
                mem::size_of::<Self>()
            )
        }
    }

    fn from_bytes(data: &[u8]) -> VexfsResult<Self> {
        if data.len() < mem::size_of::<Self>() {
            return Err(VexfsError::InvalidData("insufficient data for DiskInode".to_string()));
        }

        let inode = unsafe {
            *(data.as_ptr() as *const Self)
        };

        inode.validate()?;
        Ok(inode)
    }

    fn serialized_size() -> usize {
        mem::size_of::<Self>()
    }

    fn validate(&self) -> VexfsResult<()> {
        // Validate file mode
        if self.i_mode & S_IFMT == 0 {
            return Err(VexfsError::InvalidData("invalid file mode".to_string()));
        }

        // Validate link count
        if self.i_links_count == 0 && self.i_dtime == 0 {
            return Err(VexfsError::InvalidData("invalid link count".to_string()));
        }

        Ok(())
    }

    fn update_checksum(&mut self) {
        // DiskInode doesn't have embedded checksum
        // Checksum is handled at block level
    }
}

impl DiskInode {
    /// Create new inode with default values
    pub fn new(mode: u16, uid: u16, gid: u16) -> Self {
        Self {
            i_mode: mode,
            i_uid: uid,
            i_size_lo: 0,
            i_atime: 0,
            i_ctime: 0,
            i_mtime: 0,
            i_dtime: 0,
            i_gid: gid,
            i_links_count: 1,
            i_blocks: 0,
            i_flags: 0,
            l_i_version: 0,
            i_block: [0; 15],
            i_generation: 0,
            i_file_acl: 0,
            i_size_high: 0,
            i_faddr: 0,
            l_i_frag: 0,
            l_i_fsize: 0,
            i_pad1: 0,
            l_i_uid_high: 0,
            l_i_gid_high: 0,
            l_i_reserved2: 0,
            i_extra_isize: 0,
            i_pad: [0; 2],
        }
    }

    /// Get full file size (combining high and low parts)
    pub fn get_size(&self) -> u64 {
        ((self.i_size_high as u64) << 32) | (self.i_size_lo as u64)
    }

    /// Set full file size (splitting into high and low parts)
    pub fn set_size(&mut self, size: u64) {
        self.i_size_lo = size as u32;
        self.i_size_high = (size >> 32) as u32;
    }

    /// Check if inode is a directory
    pub fn is_dir(&self) -> bool {
        (self.i_mode & S_IFMT) == S_IFDIR
    }

    /// Check if inode is a regular file
    pub fn is_file(&self) -> bool {
        (self.i_mode & S_IFMT) == S_IFREG
    }

    /// Check if inode is a symbolic link
    pub fn is_symlink(&self) -> bool {
        (self.i_mode & S_IFMT) == S_IFLNK
    }
}

/// Directory entry structure
#[derive(Debug, Clone)]
#[repr(C, packed)]
pub struct DiskDirEntry {
    /// Inode number
    pub inode: u32,
    /// Record length
    pub rec_len: u16,
    /// Name length
    pub name_len: u8,
    /// File type
    pub file_type: u8,
    // Name follows here (variable length)
}

impl DiskDirEntry {
    /// Create new directory entry
    pub fn new(inode: InodeNumber, name: &str, file_type: u8) -> VexfsResult<Vec<u8>> {
        if name.len() > VEXFS_MAX_FILENAME_LEN {
            return Err(VexfsError::InvalidArgument("filename too long".to_string()));
        }

        let name_bytes = name.as_bytes();
        let total_len = mem::size_of::<Self>() + name_bytes.len();
        let aligned_len = align_up(total_len, 4); // 4-byte alignment

        let mut data = vec![0u8; aligned_len];
        
        let entry = Self {
            inode,
            rec_len: aligned_len as u16,
            name_len: name_bytes.len() as u8,
            file_type,
        };

        // Copy header
        let header_bytes = unsafe {
            slice::from_raw_parts(
                &entry as *const Self as *const u8,
                mem::size_of::<Self>()
            )
        };
        data[0..mem::size_of::<Self>()].copy_from_slice(header_bytes);

        // Copy name
        data[mem::size_of::<Self>()..mem::size_of::<Self>() + name_bytes.len()]
            .copy_from_slice(name_bytes);

        Ok(data)
    }

    /// Get entry name from raw data
    pub fn get_name(data: &[u8]) -> VexfsResult<&str> {
        if data.len() < mem::size_of::<Self>() {
            return Err(VexfsError::InvalidData("insufficient data for directory entry".to_string()));
        }

        let entry = unsafe {
            &*(data.as_ptr() as *const Self)
        };

        let name_start = mem::size_of::<Self>();
        let name_end = name_start + entry.name_len as usize;

        if data.len() < name_end {
            return Err(VexfsError::InvalidData("directory entry name truncated".to_string()));
        }

        let name_bytes = &data[name_start..name_end];
        core::str::from_utf8(name_bytes)
            .map_err(|_| VexfsError::InvalidData("invalid UTF-8 in filename".to_string()))
    }

    /// Parse directory entry from raw data
    pub fn from_bytes(data: &[u8]) -> VexfsResult<Self> {
        if data.len() < mem::size_of::<Self>() {
            return Err(VexfsError::InvalidData("insufficient data for directory entry".to_string()));
        }

        let entry = unsafe {
            *(data.as_ptr() as *const Self)
        };

        // Validate entry
        if entry.rec_len < mem::size_of::<Self>() as u16 {
            return Err(VexfsError::InvalidData("invalid directory entry record length".to_string()));
        }

        if entry.name_len as usize + mem::size_of::<Self>() > entry.rec_len as usize {
            return Err(VexfsError::InvalidData("directory entry name length exceeds record".to_string()));
        }

        Ok(entry)
    }

    /// Get the total size of this entry including name
    pub fn total_size(&self) -> usize {
        self.rec_len as usize
    }
}

/// Block group descriptor
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct DiskGroupDesc {
    /// Block bitmap block
    pub bg_block_bitmap: u64,
    /// Inode bitmap block
    pub bg_inode_bitmap: u64,
    /// Inode table start block
    pub bg_inode_table: u64,
    /// Number of free blocks in group
    pub bg_free_blocks_count: u16,
    /// Number of free inodes in group
    pub bg_free_inodes_count: u16,
    /// Number of directories in group
    pub bg_used_dirs_count: u16,
    /// Padding
    pub bg_pad: u16,
    /// Reserved
    pub bg_reserved: [u32; 3],
}

impl OnDiskSerializable for DiskGroupDesc {
    fn to_bytes(&self) -> &[u8] {
        unsafe {
            slice::from_raw_parts(
                self as *const Self as *const u8,
                mem::size_of::<Self>()
            )
        }
    }

    fn from_bytes(data: &[u8]) -> VexfsResult<Self> {
        if data.len() < mem::size_of::<Self>() {
            return Err(VexfsError::InvalidData("insufficient data for DiskGroupDesc".to_string()));
        }

        let desc = unsafe {
            *(data.as_ptr() as *const Self)
        };

        desc.validate()?;
        Ok(desc)
    }

    fn serialized_size() -> usize {
        mem::size_of::<Self>()
    }

    fn validate(&self) -> VexfsResult<()> {
        // Validate block numbers are non-zero
        if self.bg_block_bitmap == 0 || self.bg_inode_bitmap == 0 || self.bg_inode_table == 0 {
            return Err(VexfsError::InvalidData("invalid block group descriptor".to_string()));
        }

        Ok(())
    }

    fn update_checksum(&mut self) {
        // DiskGroupDesc doesn't have embedded checksum
    }
}

impl DiskGroupDesc {
    /// Create new block group descriptor
    pub fn new(
        block_bitmap: BlockNumber,
        inode_bitmap: BlockNumber,
        inode_table: BlockNumber,
        free_blocks: u16,
        free_inodes: u16,
    ) -> Self {
        Self {
            bg_block_bitmap: block_bitmap,
            bg_inode_bitmap: inode_bitmap,
            bg_inode_table: inode_table,
            bg_free_blocks_count: free_blocks,
            bg_free_inodes_count: free_inodes,
            bg_used_dirs_count: 0,
            bg_pad: 0,
            bg_reserved: [0; 3],
        }
    }

    /// Update free block count
    pub fn update_free_blocks(&mut self, delta: i32) {
        if delta < 0 {
            self.bg_free_blocks_count = self.bg_free_blocks_count.saturating_sub((-delta) as u16);
        } else {
            self.bg_free_blocks_count = self.bg_free_blocks_count.saturating_add(delta as u16);
        }
    }

    /// Update free inode count
    pub fn update_free_inodes(&mut self, delta: i32) {
        if delta < 0 {
            self.bg_free_inodes_count = self.bg_free_inodes_count.saturating_sub((-delta) as u16);
        } else {
            self.bg_free_inodes_count = self.bg_free_inodes_count.saturating_add(delta as u16);
        }
    }

    /// Update directory count
    pub fn update_dir_count(&mut self, delta: i32) {
        if delta < 0 {
            self.bg_used_dirs_count = self.bg_used_dirs_count.saturating_sub((-delta) as u16);
        } else {
            self.bg_used_dirs_count = self.bg_used_dirs_count.saturating_add(delta as u16);
        }
    }
}

/// Persistence manager for on-disk operations
pub struct PersistenceManager {
    /// Block size for alignment calculations
    block_size: u32,
    /// Checksum validation enabled
    checksum_enabled: bool,
}

impl PersistenceManager {
    /// Create new persistence manager
    pub fn new(block_size: u32, checksum_enabled: bool) -> Self {
        Self {
            block_size,
            checksum_enabled,
        }
    }

    /// Serialize structure to block-aligned buffer
    pub fn serialize_to_block<T: OnDiskSerializable>(&self, data: &T) -> VexfsResult<Vec<u8>> {
        let serialized = data.to_bytes();
        let aligned_size = align_up(serialized.len(), self.block_size as usize);
        
        let mut buffer = vec![0u8; aligned_size];
        buffer[0..serialized.len()].copy_from_slice(serialized);
        
        if self.checksum_enabled {
            self.add_block_checksum(&mut buffer)?;
        }
        
        Ok(buffer)
    }

    /// Deserialize structure from block buffer
    pub fn deserialize_from_block<T: OnDiskSerializable>(&self, buffer: &[u8]) -> VexfsResult<T> {
        if self.checksum_enabled {
            self.verify_block_checksum(buffer)?;
        }
        
        T::from_bytes(buffer)
    }

    /// Write multiple directory entries to block
    pub fn serialize_dir_entries(&self, entries: &[Vec<u8>]) -> VexfsResult<Vec<u8>> {
        let mut buffer = Vec::new();
        
        for entry_data in entries {
            buffer.extend_from_slice(entry_data);
        }
        
        // Pad to block boundary
        let aligned_size = align_up(buffer.len(), self.block_size as usize);
        buffer.resize(aligned_size, 0);
        
        if self.checksum_enabled {
            self.add_block_checksum(&mut buffer)?;
        }
        
        Ok(buffer)
    }

    /// Read directory entries from block
    pub fn deserialize_dir_entries(&self, buffer: &[u8]) -> VexfsResult<Vec<DiskDirEntry>> {
        if self.checksum_enabled {
            self.verify_block_checksum(buffer)?;
        }
        
        let mut entries = Vec::new();
        let mut offset = 0;
        
        while offset < buffer.len() {
            if offset + mem::size_of::<DiskDirEntry>() > buffer.len() {
                break;
            }
            
            let entry = DiskDirEntry::from_bytes(&buffer[offset..])?;
            
            if entry.rec_len == 0 {
                break; // End of entries
            }
            
            let rec_len = entry.rec_len;
            entries.push(entry);
            offset += rec_len as usize;
        }
        
        Ok(entries)
    }

    /// Calculate checksum for data
    pub fn calculate_checksum(&self, data: &[u8]) -> u32 {
        crc32(data)
    }

    /// Verify data integrity
    pub fn verify_integrity<T: OnDiskSerializable>(&self, data: &T) -> VexfsResult<()> {
        data.validate()
    }

    // Private helper methods
    fn add_block_checksum(&self, buffer: &mut [u8]) -> VexfsResult<()> {
        if buffer.len() < 4 {
            return Err(VexfsError::InvalidData("buffer too small for checksum".to_string()));
        }
        
        // Calculate checksum of all data except last 4 bytes
        let data_len = buffer.len() - 4;
        let checksum = self.calculate_checksum(&buffer[0..data_len]);
        
        // Store checksum at end of buffer
        buffer[data_len..].copy_from_slice(&checksum.to_le_bytes());
        
        Ok(())
    }

    fn verify_block_checksum(&self, buffer: &[u8]) -> VexfsResult<()> {
        if buffer.len() < 4 {
            return Err(VexfsError::InvalidData("buffer too small for checksum verification".to_string()));
        }
        
        let data_len = buffer.len() - 4;
        let expected_checksum = u32::from_le_bytes([
            buffer[data_len],
            buffer[data_len + 1],
            buffer[data_len + 2],
            buffer[data_len + 3],
        ]);
        
        let actual_checksum = self.calculate_checksum(&buffer[0..data_len]);
        
        if actual_checksum != expected_checksum {
            return Err(VexfsError::ChecksumMismatch);
        }
        
        Ok(())
    }
}

/// Buffer manager for efficient I/O operations
pub struct BufferManager {
    /// Block size
    block_size: u32,
    /// Write buffer pool
    write_buffers: [Vec<u8>; VEXFS_MAX_WRITE_BUFFERS],
    /// Buffer allocation tracking
    buffer_allocated: [bool; VEXFS_MAX_WRITE_BUFFERS],
    /// Next buffer index
    next_buffer: usize,
}

impl BufferManager {
    /// Create new buffer manager
    pub fn new(block_size: u32) -> Self {
        Self {
            block_size,
            write_buffers: [const { Vec::new() }; VEXFS_MAX_WRITE_BUFFERS],
            buffer_allocated: [false; VEXFS_MAX_WRITE_BUFFERS],
            next_buffer: 0,
        }
    }

    /// Allocate a write buffer
    pub fn allocate_buffer(&mut self, size: usize) -> VexfsResult<usize> {
        let aligned_size = align_up(size, self.block_size as usize);
        
        // Find free buffer
        for i in 0..VEXFS_MAX_WRITE_BUFFERS {
            let idx = (self.next_buffer + i) % VEXFS_MAX_WRITE_BUFFERS;
            if !self.buffer_allocated[idx] {
                self.write_buffers[idx] = vec![0u8; aligned_size];
                self.buffer_allocated[idx] = true;
                self.next_buffer = (idx + 1) % VEXFS_MAX_WRITE_BUFFERS;
                return Ok(idx);
            }
        }
        
        Err(VexfsError::NoSpace)
    }

    /// Release a write buffer
    pub fn release_buffer(&mut self, buffer_id: usize) -> VexfsResult<()> {
        if buffer_id >= VEXFS_MAX_WRITE_BUFFERS {
            return Err(VexfsError::InvalidArgument("invalid buffer ID".to_string()));
        }
        
        if !self.buffer_allocated[buffer_id] {
            return Err(VexfsError::InvalidArgument("buffer not allocated".to_string()));
        }
        
        self.write_buffers[buffer_id].clear();
        self.buffer_allocated[buffer_id] = false;
        
        Ok(())
    }

    /// Get buffer reference
    pub fn get_buffer(&mut self, buffer_id: usize) -> VexfsResult<&mut Vec<u8>> {
        if buffer_id >= VEXFS_MAX_WRITE_BUFFERS {
            return Err(VexfsError::InvalidArgument("invalid buffer ID".to_string()));
        }
        
        if !self.buffer_allocated[buffer_id] {
            return Err(VexfsError::InvalidArgument("buffer not allocated".to_string()));
        }
        
        Ok(&mut self.write_buffers[buffer_id])
    }

    /// Check buffer allocation status
    pub fn is_allocated(&self, buffer_id: usize) -> bool {
        buffer_id < VEXFS_MAX_WRITE_BUFFERS && self.buffer_allocated[buffer_id]
    }

    /// Get buffer statistics
    pub fn get_stats(&self) -> (usize, usize) {
        let allocated = self.buffer_allocated.iter().filter(|&&x| x).count();
        (allocated, VEXFS_MAX_WRITE_BUFFERS)
    }
}

// Re-export directory entry file type constants
pub use crate::shared::constants::{
    VEXFS_FT_UNKNOWN,
    VEXFS_FT_REG_FILE,
    VEXFS_FT_DIR,
    VEXFS_FT_CHRDEV,
    VEXFS_FT_BLKDEV,
    VEXFS_FT_FIFO,
    VEXFS_FT_SOCK,
    VEXFS_FT_SYMLINK,
};

// Re-export for compatibility
pub type VexfsInode = DiskInode;
pub type VexfsDirEntry = DiskDirEntry;
pub type VexfsGroupDesc = DiskGroupDesc;
pub type OnDiskSerialize = OnDiskSerializable;