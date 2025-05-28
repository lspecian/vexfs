//! Vector Embedding Storage System for VexFS
//! 
//! This module implements the core vector storage capabilities that make VexFS vector-native,
//! including on-disk format, allocation mechanisms, file-to-embedding linking, metadata
//! management, and serialization/compression.
//!
//! This module has been integrated with the fs_core architecture to use OperationContext
//! patterns and work seamlessly with the established VexFS components.

use core::mem;
use crate::shared::errors::{VexfsError, VexfsResult};
use crate::shared::types::{InodeNumber, BlockNumber, Result};
use crate::fs_core::operations::OperationContext;
use crate::storage::StorageManager;

#[cfg(not(feature = "kernel"))]
use std::sync::Arc;
#[cfg(feature = "kernel")]
use alloc::sync::Arc;

#[cfg(not(feature = "std"))]
use alloc::{vec::Vec, string::String, collections::BTreeMap};
#[cfg(feature = "std")]
use std::{vec::Vec, string::String, collections::BTreeMap};

// Re-export VectorStorageError for backward compatibility
pub use crate::shared::errors::{VectorErrorKind as VectorStorageError};

// Forward declarations for types that will be defined in other modules
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SpaceAllocError {
    NoSpace,
    InvalidBlock,
    AlreadyAllocated,
    NotAllocated,
}

#[derive(Debug, Clone, Copy)]
pub struct AllocResult {
    pub start_block: u64,
    pub block_count: u32,
    pub fragmentation_score: u8,
}

#[derive(Debug, Clone, Copy)]
pub enum AllocHint {
    Sequential,
    Random,
    Clustered,
    VectorAligned,
}

/// Vector data types supported
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum VectorDataType {
    Float32 = 0,
    Float16 = 1,
    Int8 = 2,
    Int16 = 3,
    Binary = 4,
}

/// Compression algorithms supported
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum CompressionType {
    None = 0,
    Quantization4Bit = 1,
    Quantization8Bit = 2,
    ProductQuantization = 3,
    SparseEncoding = 4,
}

/// Vector storage format version
pub const VECTOR_FORMAT_VERSION: u32 = 1;

/// Maximum vector dimensions supported
pub const MAX_VECTOR_DIMENSIONS: u32 = 4096;

/// Vector block size alignment (64 bytes for cache efficiency)
pub const VECTOR_ALIGNMENT: usize = 64;

/// Vector header stored on disk
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct VectorHeader {
    /// Magic number for validation
    pub magic: u32,
    /// Format version
    pub version: u32,
    /// Vector ID
    pub vector_id: u64,
    /// Associated file inode
    pub file_inode: u64,
    /// Data type of vector elements
    pub data_type: VectorDataType,
    /// Compression algorithm used
    pub compression: CompressionType,
    /// Number of dimensions
    pub dimensions: u32,
    /// Original size in bytes before compression
    pub original_size: u32,
    /// Compressed size in bytes
    pub compressed_size: u32,
    /// Timestamp when vector was created
    pub created_timestamp: u64,
    /// Timestamp when vector was last modified
    pub modified_timestamp: u64,
    /// CRC32 checksum for integrity
    pub checksum: u32,
    /// Flags for future use
    pub flags: u32,
    /// Reserved bytes for future expansion (reduced to fit in 64 bytes)
    pub reserved: [u8; 0],
}

impl VectorHeader {
    pub const MAGIC: u32 = 0x56454358; // "VECX"
    pub const SIZE: usize = mem::size_of::<VectorHeader>();
}

/// File-to-vector mapping entry
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct FileVectorMapping {
    /// File inode number
    pub file_inode: u64,
    /// Vector ID
    pub vector_id: u64,
    /// Relationship type (e.g., content=0, summary=1, etc.)
    pub relationship_type: u8,
    /// Confidence score (0-255)
    pub confidence: u8,
    /// Flags
    pub flags: u16,
    /// Timestamp when mapping was created
    pub timestamp: u64,
}

/// Vector metadata entry for indexing and search
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct VectorMetadata {
    /// Vector ID
    pub vector_id: u64,
    /// File inode
    pub file_inode: u64,
    /// Model version that generated this vector
    pub model_version: u32,
    /// Confidence score
    pub confidence: f32,
    /// Access count
    pub access_count: u32,
    /// Last access timestamp
    pub last_access: u64,
    /// Creation timestamp
    pub created: u64,
    /// Flags for metadata
    pub flags: u32,
}

/// Vector allocation statistics
#[derive(Debug, Clone, Copy)]
pub struct VectorAllocStats {
    /// Total number of vectors stored
    pub total_vectors: u64,
    /// Total bytes allocated for vectors
    pub total_bytes: u64,
    /// Average vector size in bytes
    pub avg_vector_size: u32,
    /// Maximum vector size encountered
    pub max_vector_size: u32,
    /// Fragmentation score (0-100)
    pub fragmentation_score: u8,
    /// Number of free blocks
    pub free_blocks: u32,
    /// Largest free block size
    pub largest_free_block: u32,
}

/// Vector location information for indexing
#[derive(Debug, Clone, Copy)]
pub struct VectorLocation {
    /// Starting block number
    pub start_block: BlockNumber,
    /// Number of blocks used
    pub block_count: u32,
    /// Vector header information
    pub header: VectorHeader,
}

/// Main vector storage manager integrated with fs_core architecture
pub struct VectorStorageManager {
    /// Reference to storage manager for block operations
    storage_manager: Arc<StorageManager>,
    /// Device block size
    pub block_size: u32,
    /// Total storage capacity in blocks
    pub total_blocks: u64,
    /// Number of free blocks
    pub free_blocks: u64,
    /// Next available vector ID
    pub next_vector_id: u64,
    /// Allocation statistics
    pub alloc_stats: VectorAllocStats,
    /// Format version being used
    pub format_version: u32,
    /// Vector index mapping vector IDs to block locations
    vector_index: BTreeMap<u64, VectorLocation>,
    /// File-to-vector mapping
    file_vector_map: BTreeMap<InodeNumber, Vec<u64>>,
}

impl VectorStorageManager {
    /// Create a new vector storage manager integrated with fs_core
    pub fn new(storage_manager: Arc<StorageManager>, block_size: u32, total_blocks: u64) -> Self {
        Self {
            storage_manager,
            block_size,
            total_blocks,
            free_blocks: total_blocks,
            next_vector_id: 1,
            alloc_stats: VectorAllocStats {
                total_vectors: 0,
                total_bytes: 0,
                avg_vector_size: 0,
                max_vector_size: 0,
                fragmentation_score: 0,
                free_blocks: total_blocks as u32,
                largest_free_block: total_blocks as u32,
            },
            format_version: VECTOR_FORMAT_VERSION,
            vector_index: BTreeMap::new(),
            file_vector_map: BTreeMap::new(),
        }
    }

    /// Store a vector with metadata using OperationContext pattern
    pub fn store_vector(
        &mut self,
        context: &mut OperationContext,
        data: &[u8],
        file_inode: InodeNumber,
        data_type: VectorDataType,
        dimensions: u32,
        compression: CompressionType,
    ) -> VexfsResult<u64> {
        if dimensions > MAX_VECTOR_DIMENSIONS {
            return Err(VexfsError::VectorError(crate::shared::errors::VectorErrorKind::InvalidDimensions(dimensions as u16)));
        }

        let vector_id = self.next_vector_id;
        self.next_vector_id += 1;

        // Calculate space needed
        let header_size = VectorHeader::SIZE;
        let total_size = header_size + data.len();
        let aligned_size = (total_size + VECTOR_ALIGNMENT - 1) & !(VECTOR_ALIGNMENT - 1);

        // Allocate space using storage manager
        let blocks_needed = (aligned_size as u32 + self.block_size - 1) / self.block_size;
        let allocated_blocks = self.storage_manager.allocate_blocks(blocks_needed, None)?;
        
        if allocated_blocks.is_empty() {
            return Err(VexfsError::OutOfSpace);
        }

        let start_block = allocated_blocks[0];

        // Create vector header
        let header = VectorHeader {
            magic: VectorHeader::MAGIC,
            version: self.format_version,
            vector_id,
            file_inode,
            data_type,
            compression,
            dimensions,
            original_size: data.len() as u32,
            compressed_size: data.len() as u32, // TODO: implement compression
            created_timestamp: 0, // TODO: get current time from context
            modified_timestamp: 0,
            checksum: self.calculate_checksum(data),
            flags: 0,
            reserved: [],
        };

        // Serialize header and data
        let mut block_data = Vec::with_capacity(aligned_size);
        
        // Write header (unsafe conversion for now - in production would use proper serialization)
        let header_bytes = unsafe {
            core::slice::from_raw_parts(
                &header as *const VectorHeader as *const u8,
                header_size
            )
        };
        block_data.extend_from_slice(header_bytes);
        
        // Write vector data
        block_data.extend_from_slice(data);
        
        // Pad to alignment
        while block_data.len() < aligned_size {
            block_data.push(0);
        }

        // Write to storage using storage manager
        let mut offset = 0;
        for &block_num in &allocated_blocks {
            let chunk_size = core::cmp::min(self.block_size as usize, block_data.len() - offset);
            if chunk_size == 0 {
                break;
            }
            
            let chunk = block_data[offset..offset + chunk_size].to_vec();
            // Pad chunk to block size
            let mut padded_chunk = chunk;
            padded_chunk.resize(self.block_size as usize, 0);
            
            self.storage_manager.write_block(block_num, padded_chunk)?;
            offset += chunk_size;
        }

        // Update indices
        let location = VectorLocation {
            start_block,
            block_count: blocks_needed,
            header,
        };
        
        self.vector_index.insert(vector_id, location);
        
        // Update file-to-vector mapping
        self.file_vector_map
            .entry(file_inode)
            .or_insert_with(Vec::new)
            .push(vector_id);

        // Update statistics
        self.alloc_stats.total_vectors += 1;
        self.alloc_stats.total_bytes += aligned_size as u64;
        self.alloc_stats.avg_vector_size = (self.alloc_stats.total_bytes / self.alloc_stats.total_vectors) as u32;
        if aligned_size as u32 > self.alloc_stats.max_vector_size {
            self.alloc_stats.max_vector_size = aligned_size as u32;
        }

        Ok(vector_id)
    }

    /// Retrieve a vector by ID using OperationContext pattern
    pub fn get_vector(&mut self, context: &mut OperationContext, vector_id: u64) -> VexfsResult<(VectorHeader, Vec<u8>)> {
        // Look up vector location in index
        let location = self.vector_index.get(&vector_id)
            .ok_or_else(|| VexfsError::VectorError(crate::shared::errors::VectorErrorKind::VectorNotFound))?;

        // Read blocks from storage
        let mut data = Vec::new();
        for i in 0..location.block_count {
            let block_num = location.start_block + i as u64;
            let block_data = self.storage_manager.read_block(block_num)?;
            data.extend_from_slice(&block_data);
        }

        // Extract header
        if data.len() < VectorHeader::SIZE {
            return Err(VexfsError::VectorError(crate::shared::errors::VectorErrorKind::DeserializationError));
        }

        let header = location.header;
        
        // Extract vector data
        let vector_data_start = VectorHeader::SIZE;
        let vector_data_end = vector_data_start + header.compressed_size as usize;
        
        if data.len() < vector_data_end {
            return Err(VexfsError::VectorError(crate::shared::errors::VectorErrorKind::DeserializationError));
        }

        let vector_data = data[vector_data_start..vector_data_end].to_vec();

        // Verify integrity
        if !self.verify_vector_integrity(&header, &vector_data) {
            return Err(VexfsError::VectorError(crate::shared::errors::VectorErrorKind::DeserializationError));
        }

        Ok((header, vector_data))
    }

    /// Delete a vector using OperationContext pattern
    pub fn delete_vector(&mut self, context: &mut OperationContext, vector_id: u64) -> VexfsResult<()> {
        // Look up vector location
        let location = self.vector_index.remove(&vector_id)
            .ok_or_else(|| VexfsError::VectorError(crate::shared::errors::VectorErrorKind::VectorNotFound))?;

        // Free allocated blocks
        let blocks_to_free: Vec<BlockNumber> = (location.start_block..location.start_block + location.block_count as u64).collect();
        self.storage_manager.free_blocks(&blocks_to_free)?;

        // Remove from file-to-vector mapping
        let file_inode = location.header.file_inode;
        if let Some(vector_list) = self.file_vector_map.get_mut(&file_inode) {
            vector_list.retain(|&id| id != vector_id);
            if vector_list.is_empty() {
                self.file_vector_map.remove(&file_inode);
            }
        }

        // Update statistics
        self.alloc_stats.total_vectors = self.alloc_stats.total_vectors.saturating_sub(1);
        if self.alloc_stats.total_vectors > 0 {
            self.alloc_stats.avg_vector_size = (self.alloc_stats.total_bytes / self.alloc_stats.total_vectors) as u32;
        } else {
            self.alloc_stats.avg_vector_size = 0;
        }

        Ok(())
    }

    /// Get vectors associated with a file using OperationContext pattern
    pub fn get_file_vectors(&mut self, context: &mut OperationContext, file_inode: InodeNumber) -> VexfsResult<Vec<u64>> {
        Ok(self.file_vector_map.get(&file_inode).cloned().unwrap_or_default())
    }

    /// Get files associated with a vector using OperationContext pattern
    pub fn get_vector_files(&mut self, context: &mut OperationContext, vector_id: u64) -> VexfsResult<Vec<InodeNumber>> {
        let location = self.vector_index.get(&vector_id)
            .ok_or_else(|| VexfsError::VectorError(crate::shared::errors::VectorErrorKind::VectorNotFound))?;
        
        // Get file inode from header
        let file_inode = location.header.file_inode;
        Ok(vec![file_inode])
    }

    /// Calculate CRC32 checksum for data integrity
    fn calculate_checksum(&self, data: &[u8]) -> u32 {
        // Simple checksum implementation (in practice would use proper CRC32)
        let mut checksum = 0u32;
        for &byte in data {
            checksum = checksum.wrapping_add(byte as u32);
        }
        checksum
    }

    /// Verify vector data integrity
    pub fn verify_vector_integrity(&self, header: &VectorHeader, data: &[u8]) -> bool {
        // Verify magic number
        if header.magic != VectorHeader::MAGIC {
            return false;
        }

        // Verify version
        if header.version != VECTOR_FORMAT_VERSION {
            return false;
        }

        // Verify checksum
        let calculated_checksum = self.calculate_checksum(data);
        if header.checksum != calculated_checksum {
            return false;
        }

        // Verify data size
        if data.len() != header.compressed_size as usize {
            return false;
        }

        true
    }

    /// Get storage statistics
    pub fn get_stats(&self) -> VectorAllocStats {
        self.alloc_stats
    }

    /// Compact storage to reduce fragmentation using OperationContext pattern
    pub fn compact_storage(&mut self, context: &mut OperationContext) -> VexfsResult<()> {
        // TODO: Implement storage compaction
        // This would involve:
        // 1. Identifying fragmented regions
        // 2. Moving vectors to consolidate free space
        // 3. Updating indices and mappings
        // 4. Using storage manager for block operations
        Ok(())
    }

    /// Sync vector storage to persistent storage
    pub fn sync(&mut self, context: &mut OperationContext) -> VexfsResult<()> {
        // Sync underlying storage manager
        self.storage_manager.sync_all()?;
        
        // TODO: Persist vector indices and mappings
        // This would involve writing the vector_index and file_vector_map
        // to dedicated metadata blocks
        
        Ok(())
    }

    /// Get storage manager reference for integration
    pub fn storage_manager(&self) -> &Arc<StorageManager> {
        &self.storage_manager
    }
}

/// Vector compression utilities
pub struct VectorCompression;

impl VectorCompression {
    /// Compress vector data using specified algorithm
    pub fn compress(
        data: &[u8],
        compression: CompressionType,
        data_type: VectorDataType,
    ) -> VexfsResult<Vec<u8>> {
        match compression {
            CompressionType::None => {
                Ok(data.to_vec())
            }
            _ => {
                // TODO: Implement compression algorithms
                Err(VexfsError::VectorError(crate::shared::errors::VectorErrorKind::SerializationError))
            }
        }
    }

    /// Decompress vector data
    pub fn decompress(
        data: &[u8],
        compression: CompressionType,
        original_size: u32,
        data_type: VectorDataType,
    ) -> VexfsResult<Vec<u8>> {
        match compression {
            CompressionType::None => {
                if data.len() != original_size as usize {
                    return Err(VexfsError::VectorError(crate::shared::errors::VectorErrorKind::DeserializationError));
                }
                Ok(data.to_vec())
            }
            _ => {
                // TODO: Implement decompression algorithms
                Err(VexfsError::VectorError(crate::shared::errors::VectorErrorKind::DeserializationError))
            }
        }
    }
}

/// Tests for vector storage functionality
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vector_header_size() {
        // Ensure header fits in a single cache line
        assert!(VectorHeader::SIZE <= 64);
    }

    #[test]
    fn test_vector_alignment() {
        assert_eq!(VECTOR_ALIGNMENT, 64);
        
        // Test alignment calculation
        let size = 100;
        let aligned = (size + VECTOR_ALIGNMENT - 1) & !(VECTOR_ALIGNMENT - 1);
        assert_eq!(aligned, 128); // Should align to next 64-byte boundary
    }

    #[test]
    fn test_vector_data_types() {
        assert_eq!(VectorDataType::Float32 as u8, 0);
        assert_eq!(VectorDataType::Float16 as u8, 1);
        assert_eq!(VectorDataType::Int8 as u8, 2);
        assert_eq!(VectorDataType::Int16 as u8, 3);
        assert_eq!(VectorDataType::Binary as u8, 4);
    }

    #[test]
    fn test_compression_types() {
        assert_eq!(CompressionType::None as u8, 0);
        assert_eq!(CompressionType::Quantization4Bit as u8, 1);
        assert_eq!(CompressionType::Quantization8Bit as u8, 2);
        assert_eq!(CompressionType::ProductQuantization as u8, 3);
        assert_eq!(CompressionType::SparseEncoding as u8, 4);
    }
}