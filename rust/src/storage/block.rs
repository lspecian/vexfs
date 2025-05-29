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

//! Block Management Module
//!
//! This module provides core block operations, device management, and block I/O functionality.
//! It serves as the foundation for all storage operations in VexFS.

use crate::shared::errors::{VexfsError, VexfsResult};
use crate::shared::types::*;
use crate::shared::constants::*;
use crate::shared::utils::*;

#[cfg(not(feature = "kernel"))]
use std::{vec::Vec, vec, format, string::String};
#[cfg(feature = "kernel")]
use alloc::{vec::Vec, vec, format, string::String};

/// Block type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockType {
    /// Superblock
    Superblock,
    /// Inode table block
    InodeTable,
    /// Data block
    Data,
    /// Directory block
    Directory,
    /// Indirect block
    Indirect,
    /// Journal block
    Journal,
    /// Bitmap block
    Bitmap,
    /// Vector index block
    VectorIndex,
    /// Vector data block
    VectorData,
}

/// Block state enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockState {
    /// Block is free and available for allocation
    Free,
    /// Block is allocated and in use
    Allocated,
    /// Block is marked for deletion but not yet freed
    PendingDelete,
    /// Block is corrupted and should not be used
    Corrupted,
    /// Block is reserved for system use
    Reserved,
}

/// Block metadata structure
#[derive(Debug, Clone, Copy)]
pub struct BlockMetadata {
    /// Block number
    pub block_num: BlockNumber,
    /// Block type
    pub block_type: BlockType,
    /// Block state
    pub state: BlockState,
    /// Reference count
    pub ref_count: u32,
    /// Last modification timestamp
    pub mtime: Timestamp,
    /// Checksum of block data
    pub checksum: u32,
    /// Size of valid data in block
    pub data_size: u32,
}

impl BlockMetadata {
    /// Create new block metadata
    pub fn new(block_num: BlockNumber, block_type: BlockType) -> Self {
        Self {
            block_num,
            block_type,
            state: BlockState::Free,
            ref_count: 0,
            mtime: current_timestamp(),
            checksum: 0,
            data_size: 0,
        }
    }

    /// Mark block as allocated
    pub fn allocate(&mut self) -> VexfsResult<()> {
        if self.state != BlockState::Free {
            return Err(VexfsError::InvalidArgument(
                "cannot allocate non-free block".to_string()
            ));
        }
        self.state = BlockState::Allocated;
        self.ref_count = 1;
        self.mtime = current_timestamp();
        Ok(())
    }

    /// Add reference to block
    pub fn add_ref(&mut self) -> VexfsResult<()> {
        if self.state != BlockState::Allocated {
            return Err(VexfsError::InvalidArgument(
                "cannot reference non-allocated block".to_string()
            ));
        }
        self.ref_count = self.ref_count.saturating_add(1);
        Ok(())
    }

    /// Remove reference from block
    pub fn remove_ref(&mut self) -> VexfsResult<bool> {
        if self.ref_count == 0 {
            return Err(VexfsError::InvalidArgument(
                "cannot remove reference from unreferenced block".to_string()
            ));
        }
        self.ref_count -= 1;
        if self.ref_count == 0 {
            self.state = BlockState::Free;
            Ok(true) // Block is now free
        } else {
            Ok(false) // Block still has references
        }
    }

    /// Update block checksum
    pub fn update_checksum(&mut self, data: &[u8]) {
        self.checksum = crc32(data);
        self.mtime = current_timestamp();
    }

    /// Verify block checksum
    pub fn verify_checksum(&self, data: &[u8]) -> bool {
        verify_checksum(data, self.checksum)
    }
}

/// Block buffer for I/O operations
pub struct BlockBuffer {
    /// Block data
    data: Vec<u8>,
    /// Block size
    block_size: u32,
    /// Dirty flag
    dirty: bool,
    /// Metadata
    metadata: BlockMetadata,
}

impl BlockBuffer {
    /// Create new block buffer
    pub fn new(block_num: BlockNumber, block_size: u32, block_type: BlockType) -> Self {
        Self {
            data: vec![0u8; block_size as usize],
            block_size,
            dirty: false,
            metadata: BlockMetadata::new(block_num, block_type),
        }
    }

    /// Get block data as slice
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Get mutable block data
    pub fn data_mut(&mut self) -> &mut [u8] {
        self.dirty = true;
        &mut self.data
    }

    /// Get block metadata
    pub fn metadata(&self) -> &BlockMetadata {
        &self.metadata
    }

    /// Get mutable block metadata
    pub fn metadata_mut(&mut self) -> &mut BlockMetadata {
        &mut self.metadata
    }

    /// Check if block is dirty
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Mark block as clean
    pub fn mark_clean(&mut self) {
        self.dirty = false;
    }

    /// Mark block as dirty
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// Validate block size
    pub fn validate_size(&self) -> VexfsResult<()> {
        validate_block_size(self.block_size)
    }

    /// Calculate and update checksum
    pub fn update_checksum(&mut self) {
        self.metadata.update_checksum(&self.data);
    }

    /// Verify block integrity
    pub fn verify_integrity(&self) -> VexfsResult<()> {
        if !self.metadata.verify_checksum(&self.data) {
            return Err(VexfsError::ChecksumError);
        }
        Ok(())
    }

    /// Zero block data
    pub fn zero(&mut self) -> VexfsResult<()> {
        safe_zero_memory(self.data.as_mut_ptr(), self.data.len())?;
        self.dirty = true;
        self.metadata.data_size = 0;
        Ok(())
    }

    /// Copy data into block
    pub fn copy_from(&mut self, offset: u32, data: &[u8]) -> VexfsResult<()> {
        let start = offset as usize;
        let end = start + data.len();
        
        if end > self.data.len() {
            return Err(VexfsError::InvalidArgument(
                "data exceeds block size".to_string()
            ));
        }
        
        self.data[start..end].copy_from_slice(data);
        self.dirty = true;
        self.metadata.data_size = max(self.metadata.data_size, end as u32);
        Ok(())
    }

    /// Copy data from block
    pub fn copy_to(&self, offset: u32, data: &mut [u8]) -> VexfsResult<()> {
        let start = offset as usize;
        let end = start + data.len();
        
        if end > self.data.len() {
            return Err(VexfsError::InvalidArgument(
                "read exceeds block size".to_string()
            ));
        }
        
        data.copy_from_slice(&self.data[start..end]);
        Ok(())
    }
}

/// Block handle for managing block lifetimes
pub struct BlockHandle {
    /// Block number
    block_num: BlockNumber,
    /// Reference to block manager
    // We'll use a marker here since we can't have self-referential structs easily
    _phantom: core::marker::PhantomData<()>,
}

impl BlockHandle {
    /// Create new block handle
    pub fn new(block_num: BlockNumber) -> Self {
        Self {
            block_num,
            _phantom: core::marker::PhantomData,
        }
    }

    /// Get block number
    pub fn block_num(&self) -> BlockNumber {
        self.block_num
    }
}

/// Block device operations trait
pub trait BlockDeviceOps {
    /// Read blocks from device
    fn read_blocks(&self, start_block: BlockNumber, blocks: &mut [BlockBuffer]) -> VexfsResult<()>;
    
    /// Write blocks to device
    fn write_blocks(&self, start_block: BlockNumber, blocks: &[BlockBuffer]) -> VexfsResult<()>;
    
    /// Flush pending writes
    fn flush(&self) -> VexfsResult<()>;
    
    /// Get device size in blocks
    fn size_in_blocks(&self) -> u64;
    
    /// Get block size
    fn block_size(&self) -> u32;
    
    /// Check if device is read-only
    fn is_read_only(&self) -> bool;
}

/// Block device abstraction
pub struct BlockDevice {
    /// Device size in bytes
    size: u64,
    /// Block size
    block_size: u32,
    /// Read-only flag
    read_only: bool,
    /// Device name/identifier
    name: String,
}

impl BlockDevice {
    /// Create new block device
    pub fn new(size: u64, block_size: u32, read_only: bool, name: String) -> VexfsResult<Self> {
        validate_block_size(block_size)?;
        
        if size == 0 {
            return Err(VexfsError::InvalidArgument("device size cannot be zero".to_string()));
        }
        
        if size % block_size as u64 != 0 {
            return Err(VexfsError::InvalidArgument(
                "device size must be aligned to block size".to_string()
            ));
        }
        
        Ok(Self {
            size,
            block_size,
            read_only,
            name,
        })
    }

    /// Get device size in bytes
    pub fn size(&self) -> u64 {
        self.size
    }

    /// Get device size in blocks
    pub fn size_in_blocks(&self) -> u64 {
        self.size / self.block_size as u64
    }

    /// Get block size
    pub fn block_size(&self) -> u32 {
        self.block_size
    }

    /// Check if device is read-only
    pub fn is_read_only(&self) -> bool {
        self.read_only
    }

    /// Get device name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Validate block number
    pub fn validate_block(&self, block_num: BlockNumber) -> VexfsResult<()> {
        if block_num >= self.size_in_blocks() {
            return Err(VexfsError::InvalidArgument(format!(
                "block {} exceeds device size in blocks {}",
                block_num,
                self.size_in_blocks()
            )));
        }
        Ok(())
    }
}

/// Block manager for coordinating block operations
pub struct BlockManager {
    /// Block device
    device: BlockDevice,
    /// Block metadata cache
    metadata_cache: Vec<Option<BlockMetadata>>,
}

impl BlockManager {
    /// Create new block manager
    pub fn new(device: BlockDevice) -> VexfsResult<Self> {
        let cache_size = min(device.size_in_blocks(), VEXFS_MAX_CACHED_BLOCKS as u64);
        let metadata_cache = vec![None; cache_size as usize];
        
        Ok(Self {
            device,
            metadata_cache,
        })
    }

    /// Get block device
    pub fn device(&self) -> &BlockDevice {
        &self.device
    }

    /// Allocate a new block buffer
    pub fn allocate_buffer(&self, block_num: BlockNumber, block_type: BlockType) -> VexfsResult<BlockBuffer> {
        self.device.validate_block(block_num)?;
        Ok(BlockBuffer::new(block_num, self.device.block_size(), block_type))
    }

    /// Get block metadata
    pub fn get_metadata(&self, block_num: BlockNumber) -> VexfsResult<Option<&BlockMetadata>> {
        self.device.validate_block(block_num)?;
        
        if (block_num as usize) < self.metadata_cache.len() {
            Ok(self.metadata_cache[block_num as usize].as_ref())
        } else {
            Ok(None)
        }
    }

    /// Update block metadata
    pub fn update_metadata(&mut self, metadata: BlockMetadata) -> VexfsResult<()> {
        self.device.validate_block(metadata.block_num)?;
        
        if (metadata.block_num as usize) < self.metadata_cache.len() {
            self.metadata_cache[metadata.block_num as usize] = Some(metadata);
        }
        
        Ok(())
    }

    /// Validate block range
    pub fn validate_range(&self, start_block: BlockNumber, count: u64) -> VexfsResult<()> {
        if start_block + count > self.device.size_in_blocks() {
            return Err(VexfsError::InvalidArgument(
                "block range exceeds device size".to_string()
            ));
        }
        Ok(())
    }

    /// Read block data from device
    pub fn read_block(&self, block_num: BlockNumber) -> VexfsResult<Vec<u8>> {
        self.device.validate_block(block_num)?;
        
        // TODO: Implement actual device I/O
        // For now, return a zero-filled block
        Ok(vec![0u8; self.device.block_size() as usize])
    }

    /// Write block data to device
    pub fn write_block(&mut self, block_num: BlockNumber, data: &[u8]) -> VexfsResult<()> {
        self.device.validate_block(block_num)?;
        
        if data.len() != self.device.block_size() as usize {
            return Err(VexfsError::InvalidArgument(
                "data size doesn't match block size".to_string()
            ));
        }
        
        // TODO: Implement actual device I/O
        // For now, just validate and return success
        Ok(())
    }

    /// Get device statistics
    pub fn get_stats(&self) -> BlockManagerStats {
        BlockManagerStats {
            device_size: self.device.size(),
            block_size: self.device.block_size(),
            total_blocks: self.device.size_in_blocks(),
            cached_metadata_entries: self.metadata_cache.iter().filter(|m| m.is_some()).count() as u64,
        }
    }
}

/// Block manager statistics
#[derive(Debug, Clone)]
pub struct BlockManagerStats {
    pub device_size: u64,
    pub block_size: u32,
    pub total_blocks: u64,
    pub cached_metadata_entries: u64,
}

/// Convenience type alias for a block
pub type Block = BlockBuffer;