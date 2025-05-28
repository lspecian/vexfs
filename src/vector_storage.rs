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
use crate::security::{SecurityManager, SecurityContext, VectorOperation};

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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
    /// Security manager for encryption and access control
    security_manager: Option<SecurityManager>,
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
            security_manager: None,
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

    /// Create a new vector storage manager with security
    pub fn new_with_security(
        storage_manager: Arc<StorageManager>,
        security_manager: SecurityManager,
        block_size: u32,
        total_blocks: u64
    ) -> Self {
        Self {
            storage_manager,
            security_manager: Some(security_manager),
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

    /// Set security manager
    pub fn set_security_manager(&mut self, security_manager: SecurityManager) {
        self.security_manager = Some(security_manager);
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

        // Apply compression to vector data
        let compressed_data = VectorCompression::compress(data, compression, data_type)?;
        
        // Calculate space needed
        let header_size = VectorHeader::SIZE;
        let total_size = header_size + compressed_data.len();
        let aligned_size = (total_size + VECTOR_ALIGNMENT - 1) & !(VECTOR_ALIGNMENT - 1);

        // Allocate space using storage manager
        let blocks_needed = (aligned_size as u32 + self.block_size - 1) / self.block_size;
        let allocated_blocks = self.storage_manager.allocate_blocks(blocks_needed, None)?;
        
        if allocated_blocks.is_empty() {
            return Err(VexfsError::OutOfSpace);
        }

        let start_block = allocated_blocks[0];

        // Create vector header with compression info
        let header = VectorHeader {
            magic: VectorHeader::MAGIC,
            version: self.format_version,
            vector_id,
            file_inode,
            data_type,
            compression,
            dimensions,
            original_size: data.len() as u32,
            compressed_size: compressed_data.len() as u32,
            created_timestamp: 0, // TODO: get current time from context
            modified_timestamp: 0,
            checksum: self.calculate_checksum(&compressed_data),
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

        let compressed_data = data[vector_data_start..vector_data_end].to_vec();

        // Verify integrity of compressed data
        if !self.verify_vector_integrity(&header, &compressed_data) {
            return Err(VexfsError::VectorError(crate::shared::errors::VectorErrorKind::DeserializationError));
        }

        // Decompress vector data
        let vector_data = VectorCompression::decompress(
            &compressed_data,
            header.compression,
            header.original_size,
            header.data_type,
        )?;

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

    /// Select optimal compression strategy based on vector characteristics
    pub fn select_compression_strategy(
        &self,
        data: &[u8],
        data_type: VectorDataType,
        dimensions: u32,
    ) -> CompressionType {
        VectorCompressionStrategy::select_optimal(data, data_type, dimensions)
    }

    /// Store vector with automatic compression strategy selection
    pub fn store_vector_auto_compress(
        &mut self,
        context: &mut OperationContext,
        data: &[u8],
        file_inode: InodeNumber,
        data_type: VectorDataType,
        dimensions: u32,
    ) -> VexfsResult<u64> {
        let compression = self.select_compression_strategy(data, data_type, dimensions);
        self.store_vector(context, data, file_inode, data_type, dimensions, compression)
    }

    /// Benchmark compression effectiveness for different strategies
    pub fn benchmark_compression(
        &self,
        data: &[u8],
        data_type: VectorDataType,
    ) -> VexfsResult<CompressionBenchmark> {
        VectorCompressionStrategy::benchmark_all(data, data_type)
    }
}

/// Compression strategy selector with optimization integration
pub struct VectorCompressionStrategy;

impl VectorCompressionStrategy {
    /// Select optimal compression based on vector characteristics
    pub fn select_optimal(
        data: &[u8],
        data_type: VectorDataType,
        dimensions: u32,
    ) -> CompressionType {
        // Analyze vector characteristics
        let sparsity = Self::calculate_sparsity(data, data_type);
        let entropy = Self::calculate_entropy(data);
        let dimension_factor = Self::get_dimension_factor(dimensions);
        
        // Decision tree for compression selection
        match data_type {
            VectorDataType::Float32 => {
                if sparsity > 0.8 {
                    // High sparsity - use sparse encoding
                    CompressionType::SparseEncoding
                } else if dimensions >= 512 {
                    // High-dimensional - use product quantization
                    CompressionType::ProductQuantization
                } else if entropy < 0.5 {
                    // Low entropy - use 4-bit quantization
                    CompressionType::Quantization4Bit
                } else {
                    // Balanced - use 8-bit quantization
                    CompressionType::Quantization8Bit
                }
            }
            VectorDataType::Float16 => {
                if sparsity > 0.7 {
                    CompressionType::SparseEncoding
                } else {
                    CompressionType::Quantization8Bit
                }
            }
            VectorDataType::Int8 | VectorDataType::Int16 => {
                if sparsity > 0.6 {
                    CompressionType::SparseEncoding
                } else {
                    CompressionType::Quantization8Bit
                }
            }
            VectorDataType::Binary => {
                // Binary vectors are already compact
                CompressionType::SparseEncoding
            }
        }
    }

    /// Calculate sparsity (fraction of near-zero elements)
    fn calculate_sparsity(data: &[u8], data_type: VectorDataType) -> f32 {
        match data_type {
            VectorDataType::Float32 => {
                if let Ok(floats) = VectorCompression::bytes_to_f32_slice(data) {
                    let threshold = 1e-6f32;
                    let zero_count = floats.iter().filter(|&&x| x.abs() < threshold).count();
                    zero_count as f32 / floats.len() as f32
                } else {
                    0.0
                }
            }
            _ => {
                // For other types, count actual zeros
                let zero_count = data.iter().filter(|&&x| x == 0).count();
                zero_count as f32 / data.len() as f32
            }
        }
    }

    /// Calculate entropy (measure of randomness)
    fn calculate_entropy(data: &[u8]) -> f32 {
        let mut counts = [0u32; 256];
        for &byte in data {
            counts[byte as usize] += 1;
        }
        
        let len = data.len() as f32;
        let mut entropy = 0.0f32;
        
        for &count in &counts {
            if count > 0 {
                let p = count as f32 / len;
                entropy -= p * p.log2();
            }
        }
        
        entropy / 8.0 // Normalize to 0-1 range
    }

    /// Get dimension factor for compression selection
    fn get_dimension_factor(dimensions: u32) -> f32 {
        match dimensions {
            0..=128 => 0.25,
            129..=256 => 0.5,
            257..=512 => 0.75,
            _ => 1.0,
        }
    }

    /// Benchmark all compression strategies
    pub fn benchmark_all(
        data: &[u8],
        data_type: VectorDataType,
    ) -> VexfsResult<CompressionBenchmark> {
        let original_size = data.len();
        let mut results = Vec::new();

        let strategies = [
            CompressionType::None,
            CompressionType::Quantization4Bit,
            CompressionType::Quantization8Bit,
            CompressionType::ProductQuantization,
            CompressionType::SparseEncoding,
        ];

        for &strategy in &strategies {
            let start = std::time::Instant::now();
            
            match VectorCompression::compress(data, strategy, data_type) {
                Ok(compressed) => {
                    let compress_time = start.elapsed();
                    
                    // Test decompression
                    let decompress_start = std::time::Instant::now();
                    match VectorCompression::decompress(
                        &compressed,
                        strategy,
                        original_size as u32,
                        data_type,
                    ) {
                        Ok(_decompressed) => {
                            let decompress_time = decompress_start.elapsed();
                            
                            results.push(CompressionResult {
                                strategy,
                                original_size,
                                compressed_size: compressed.len(),
                                compression_ratio: original_size as f32 / compressed.len() as f32,
                                compress_time,
                                decompress_time,
                                success: true,
                            });
                        }
                        Err(_) => {
                            results.push(CompressionResult {
                                strategy,
                                original_size,
                                compressed_size: compressed.len(),
                                compression_ratio: 1.0,
                                compress_time,
                                decompress_time: std::time::Duration::ZERO,
                                success: false,
                            });
                        }
                    }
                }
                Err(_) => {
                    results.push(CompressionResult {
                        strategy,
                        original_size,
                        compressed_size: original_size,
                        compression_ratio: 1.0,
                        compress_time: start.elapsed(),
                        decompress_time: std::time::Duration::ZERO,
                        success: false,
                    });
                }
            }
        }

        Ok(CompressionBenchmark {
            data_type,
            original_size,
            results,
        })
    }
}

/// Compression benchmark result
#[derive(Debug, Clone)]
pub struct CompressionBenchmark {
    pub data_type: VectorDataType,
    pub original_size: usize,
    pub results: Vec<CompressionResult>,
}

/// Individual compression strategy result
#[derive(Debug, Clone)]
pub struct CompressionResult {
    pub strategy: CompressionType,
    pub original_size: usize,
    pub compressed_size: usize,
    pub compression_ratio: f32,
    pub compress_time: std::time::Duration,
    pub decompress_time: std::time::Duration,
    pub success: bool,
}

impl CompressionBenchmark {
    /// Get the best compression strategy by ratio
    pub fn best_by_ratio(&self) -> Option<&CompressionResult> {
        self.results
            .iter()
            .filter(|r| r.success)
            .max_by(|a, b| a.compression_ratio.partial_cmp(&b.compression_ratio).unwrap())
    }

    /// Get the fastest compression strategy
    pub fn fastest_compression(&self) -> Option<&CompressionResult> {
        self.results
            .iter()
            .filter(|r| r.success)
            .min_by_key(|r| r.compress_time)
    }

    /// Get the fastest decompression strategy
    pub fn fastest_decompression(&self) -> Option<&CompressionResult> {
        self.results
            .iter()
            .filter(|r| r.success)
            .min_by_key(|r| r.decompress_time)
    }

    /// Get balanced strategy (good ratio + reasonable speed)
    pub fn balanced_strategy(&self) -> Option<&CompressionResult> {
        self.results
            .iter()
            .filter(|r| r.success && r.compression_ratio > 1.1)
            .min_by(|a, b| {
                let score_a = a.compress_time.as_nanos() as f32 / a.compression_ratio;
                let score_b = b.compress_time.as_nanos() as f32 / b.compression_ratio;
                score_a.partial_cmp(&score_b).unwrap()
            })
    }
}

/// Vector compression utilities with advanced algorithms
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
            CompressionType::Quantization4Bit => {
                Self::quantize_4bit(data, data_type)
            }
            CompressionType::Quantization8Bit => {
                Self::quantize_8bit(data, data_type)
            }
            CompressionType::ProductQuantization => {
                Self::product_quantize(data, data_type)
            }
            CompressionType::SparseEncoding => {
                Self::sparse_encode(data, data_type)
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
            CompressionType::Quantization4Bit => {
                Self::dequantize_4bit(data, original_size, data_type)
            }
            CompressionType::Quantization8Bit => {
                Self::dequantize_8bit(data, original_size, data_type)
            }
            CompressionType::ProductQuantization => {
                Self::product_dequantize(data, original_size, data_type)
            }
            CompressionType::SparseEncoding => {
                Self::sparse_decode(data, original_size, data_type)
            }
        }
    }

    /// 4-bit quantization for maximum compression
    fn quantize_4bit(data: &[u8], data_type: VectorDataType) -> VexfsResult<Vec<u8>> {
        match data_type {
            VectorDataType::Float32 => {
                let floats = Self::bytes_to_f32_slice(data)?;
                let mut compressed = Vec::new();
                
                // Find min/max for quantization range
                let min_val = floats.iter().fold(f32::INFINITY, |a, &b| a.min(b));
                let max_val = floats.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
                let range = max_val - min_val;
                
                // Store quantization parameters (8 bytes)
                compressed.extend_from_slice(&min_val.to_le_bytes());
                compressed.extend_from_slice(&range.to_le_bytes());
                
                // Quantize to 4-bit values (0-15)
                let mut packed_data = Vec::new();
                for chunk in floats.chunks(2) {
                    let val1 = if chunk.len() > 0 {
                        ((chunk[0] - min_val) / range * 15.0).round().clamp(0.0, 15.0) as u8
                    } else { 0 };
                    let val2 = if chunk.len() > 1 {
                        ((chunk[1] - min_val) / range * 15.0).round().clamp(0.0, 15.0) as u8
                    } else { 0 };
                    
                    // Pack two 4-bit values into one byte
                    packed_data.push((val1 << 4) | val2);
                }
                
                compressed.extend_from_slice(&packed_data);
                Ok(compressed)
            }
            _ => {
                // For other types, fall back to 8-bit quantization
                Self::quantize_8bit(data, data_type)
            }
        }
    }

    /// 8-bit quantization for balanced compression/quality
    fn quantize_8bit(data: &[u8], data_type: VectorDataType) -> VexfsResult<Vec<u8>> {
        match data_type {
            VectorDataType::Float32 => {
                let floats = Self::bytes_to_f32_slice(data)?;
                let mut compressed = Vec::new();
                
                // Find min/max for quantization range
                let min_val = floats.iter().fold(f32::INFINITY, |a, &b| a.min(b));
                let max_val = floats.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
                let range = max_val - min_val;
                
                // Store quantization parameters (8 bytes)
                compressed.extend_from_slice(&min_val.to_le_bytes());
                compressed.extend_from_slice(&range.to_le_bytes());
                
                // Quantize to 8-bit values (0-255)
                for &val in floats {
                    let quantized = ((val - min_val) / range * 255.0).round().clamp(0.0, 255.0) as u8;
                    compressed.push(quantized);
                }
                
                Ok(compressed)
            }
            VectorDataType::Float16 => {
                // Already 16-bit, quantize to 8-bit
                let mut compressed = Vec::new();
                for chunk in data.chunks(2) {
                    if chunk.len() == 2 {
                        // Simple downsampling for demonstration
                        compressed.push(chunk[1]); // Take high byte
                    }
                }
                Ok(compressed)
            }
            _ => {
                // For integer types, apply delta encoding
                Self::delta_encode(data)
            }
        }
    }

    /// Product quantization for high-dimensional vectors
    fn product_quantize(data: &[u8], data_type: VectorDataType) -> VexfsResult<Vec<u8>> {
        match data_type {
            VectorDataType::Float32 => {
                let floats = Self::bytes_to_f32_slice(data)?;
                let dimensions = floats.len();
                
                // Use 8 subspaces for product quantization
                let subspace_size = (dimensions + 7) / 8; // Round up
                let mut compressed = Vec::new();
                
                // Store metadata
                compressed.extend_from_slice(&(dimensions as u32).to_le_bytes());
                compressed.extend_from_slice(&(subspace_size as u32).to_le_bytes());
                
                // For each subspace, find centroids and quantize
                for subspace in 0..8 {
                    let start_idx = subspace * subspace_size;
                    let end_idx = (start_idx + subspace_size).min(dimensions);
                    
                    if start_idx >= dimensions {
                        break;
                    }
                    
                    let subvector = &floats[start_idx..end_idx];
                    
                    // Simple quantization for each subspace (in practice would use k-means)
                    let min_val = subvector.iter().fold(f32::INFINITY, |a, &b| a.min(b));
                    let max_val = subvector.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
                    let range = max_val - min_val;
                    
                    // Store subspace parameters
                    compressed.extend_from_slice(&min_val.to_le_bytes());
                    compressed.extend_from_slice(&range.to_le_bytes());
                    
                    // Quantize subspace to 8-bit
                    for &val in subvector {
                        let quantized = if range > 0.0 {
                            ((val - min_val) / range * 255.0).round().clamp(0.0, 255.0) as u8
                        } else {
                            0
                        };
                        compressed.push(quantized);
                    }
                }
                
                Ok(compressed)
            }
            _ => {
                // Fall back to 8-bit quantization for other types
                Self::quantize_8bit(data, data_type)
            }
        }
    }

    /// Sparse encoding for vectors with many zeros
    fn sparse_encode(data: &[u8], data_type: VectorDataType) -> VexfsResult<Vec<u8>> {
        match data_type {
            VectorDataType::Float32 => {
                let floats = Self::bytes_to_f32_slice(data)?;
                let mut compressed = Vec::new();
                
                // Store original dimension count
                compressed.extend_from_slice(&(floats.len() as u32).to_le_bytes());
                
                // Find non-zero elements (with small threshold)
                let threshold = 1e-6f32;
                let mut non_zero_count = 0u32;
                let mut indices = Vec::new();
                let mut values = Vec::new();
                
                for (i, &val) in floats.iter().enumerate() {
                    if val.abs() > threshold {
                        indices.push(i as u32);
                        values.push(val);
                        non_zero_count += 1;
                    }
                }
                
                // Store non-zero count
                compressed.extend_from_slice(&non_zero_count.to_le_bytes());
                
                // Store indices (4 bytes each)
                for &idx in &indices {
                    compressed.extend_from_slice(&idx.to_le_bytes());
                }
                
                // Store values (4 bytes each)
                for &val in &values {
                    compressed.extend_from_slice(&val.to_le_bytes());
                }
                
                Ok(compressed)
            }
            _ => {
                // For other types, use run-length encoding
                Self::run_length_encode(data)
            }
        }
    }

    /// Delta encoding for correlated data
    fn delta_encode(data: &[u8]) -> VexfsResult<Vec<u8>> {
        if data.is_empty() {
            return Ok(Vec::new());
        }
        
        let mut compressed = Vec::new();
        compressed.push(data[0]); // Store first value as-is
        
        for i in 1..data.len() {
            let delta = data[i].wrapping_sub(data[i-1]);
            compressed.push(delta);
        }
        
        Ok(compressed)
    }

    /// Run-length encoding for repetitive data
    fn run_length_encode(data: &[u8]) -> VexfsResult<Vec<u8>> {
        if data.is_empty() {
            return Ok(Vec::new());
        }
        
        let mut compressed = Vec::new();
        let mut current_byte = data[0];
        let mut count = 1u8;
        
        for &byte in &data[1..] {
            if byte == current_byte && count < 255 {
                count += 1;
            } else {
                compressed.push(count);
                compressed.push(current_byte);
                current_byte = byte;
                count = 1;
            }
        }
        
        // Add final run
        compressed.push(count);
        compressed.push(current_byte);
        
        Ok(compressed)
    }

    /// Dequantize 4-bit data
    fn dequantize_4bit(data: &[u8], original_size: u32, data_type: VectorDataType) -> VexfsResult<Vec<u8>> {
        match data_type {
            VectorDataType::Float32 => {
                if data.len() < 8 {
                    return Err(VexfsError::VectorError(crate::shared::errors::VectorErrorKind::DeserializationError));
                }
                
                // Read quantization parameters
                let min_val = f32::from_le_bytes([data[0], data[1], data[2], data[3]]);
                let range = f32::from_le_bytes([data[4], data[5], data[6], data[7]]);
                
                let packed_data = &data[8..];
                let mut floats = Vec::new();
                
                // Unpack 4-bit values
                for &packed_byte in packed_data {
                    let val1 = (packed_byte >> 4) & 0x0F;
                    let val2 = packed_byte & 0x0F;
                    
                    let float1 = min_val + (val1 as f32 / 15.0) * range;
                    let float2 = min_val + (val2 as f32 / 15.0) * range;
                    
                    floats.push(float1);
                    floats.push(float2);
                }
                
                // Truncate to original size
                let expected_floats = original_size as usize / 4;
                floats.truncate(expected_floats);
                
                Ok(Self::f32_slice_to_bytes(&floats))
            }
            _ => {
                Self::dequantize_8bit(data, original_size, data_type)
            }
        }
    }

    /// Dequantize 8-bit data
    fn dequantize_8bit(data: &[u8], original_size: u32, data_type: VectorDataType) -> VexfsResult<Vec<u8>> {
        match data_type {
            VectorDataType::Float32 => {
                if data.len() < 8 {
                    return Err(VexfsError::VectorError(crate::shared::errors::VectorErrorKind::DeserializationError));
                }
                
                // Read quantization parameters
                let min_val = f32::from_le_bytes([data[0], data[1], data[2], data[3]]);
                let range = f32::from_le_bytes([data[4], data[5], data[6], data[7]]);
                
                let quantized_data = &data[8..];
                let mut floats = Vec::new();
                
                for &quantized in quantized_data {
                    let float_val = min_val + (quantized as f32 / 255.0) * range;
                    floats.push(float_val);
                }
                
                Ok(Self::f32_slice_to_bytes(&floats))
            }
            VectorDataType::Float16 => {
                // Reconstruct 16-bit values (simple upsampling)
                let mut result = Vec::new();
                for &byte in data {
                    result.push(0); // Low byte
                    result.push(byte); // High byte
                }
                Ok(result)
            }
            _ => {
                Self::delta_decode(data)
            }
        }
    }

    /// Product dequantization
    fn product_dequantize(data: &[u8], original_size: u32, data_type: VectorDataType) -> VexfsResult<Vec<u8>> {
        match data_type {
            VectorDataType::Float32 => {
                if data.len() < 8 {
                    return Err(VexfsError::VectorError(crate::shared::errors::VectorErrorKind::DeserializationError));
                }
                
                // Read metadata
                let dimensions = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as usize;
                let subspace_size = u32::from_le_bytes([data[4], data[5], data[6], data[7]]) as usize;
                
                let mut floats = vec![0.0f32; dimensions];
                let mut offset = 8;
                
                // Reconstruct each subspace
                for subspace in 0..8 {
                    let start_idx = subspace * subspace_size;
                    let end_idx = (start_idx + subspace_size).min(dimensions);
                    
                    if start_idx >= dimensions || offset + 8 > data.len() {
                        break;
                    }
                    
                    // Read subspace parameters
                    let min_val = f32::from_le_bytes([data[offset], data[offset+1], data[offset+2], data[offset+3]]);
                    let range = f32::from_le_bytes([data[offset+4], data[offset+5], data[offset+6], data[offset+7]]);
                    offset += 8;
                    
                    // Dequantize subspace
                    for i in start_idx..end_idx {
                        if offset >= data.len() {
                            break;
                        }
                        let quantized = data[offset];
                        floats[i] = min_val + (quantized as f32 / 255.0) * range;
                        offset += 1;
                    }
                }
                
                Ok(Self::f32_slice_to_bytes(&floats))
            }
            _ => {
                Self::dequantize_8bit(data, original_size, data_type)
            }
        }
    }

    /// Sparse decoding
    fn sparse_decode(data: &[u8], original_size: u32, data_type: VectorDataType) -> VexfsResult<Vec<u8>> {
        match data_type {
            VectorDataType::Float32 => {
                if data.len() < 8 {
                    return Err(VexfsError::VectorError(crate::shared::errors::VectorErrorKind::DeserializationError));
                }
                
                // Read metadata
                let dimensions = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as usize;
                let non_zero_count = u32::from_le_bytes([data[4], data[5], data[6], data[7]]) as usize;
                
                let mut floats = vec![0.0f32; dimensions];
                let mut offset = 8;
                
                // Read indices
                let mut indices = Vec::new();
                for _ in 0..non_zero_count {
                    if offset + 4 > data.len() {
                        return Err(VexfsError::VectorError(crate::shared::errors::VectorErrorKind::DeserializationError));
                    }
                    let idx = u32::from_le_bytes([data[offset], data[offset+1], data[offset+2], data[offset+3]]) as usize;
                    indices.push(idx);
                    offset += 4;
                }
                
                // Read values
                for (i, &idx) in indices.iter().enumerate() {
                    if offset + 4 > data.len() || idx >= dimensions {
                        return Err(VexfsError::VectorError(crate::shared::errors::VectorErrorKind::DeserializationError));
                    }
                    let val = f32::from_le_bytes([data[offset], data[offset+1], data[offset+2], data[offset+3]]);
                    floats[idx] = val;
                    offset += 4;
                }
                
                Ok(Self::f32_slice_to_bytes(&floats))
            }
            _ => {
                Self::run_length_decode(data)
            }
        }
    }

    /// Delta decoding
    fn delta_decode(data: &[u8]) -> VexfsResult<Vec<u8>> {
        if data.is_empty() {
            return Ok(Vec::new());
        }
        
        let mut decoded = Vec::new();
        decoded.push(data[0]); // First value as-is
        
        for i in 1..data.len() {
            let prev = decoded[i-1];
            let delta = data[i];
            decoded.push(prev.wrapping_add(delta));
        }
        
        Ok(decoded)
    }

    /// Run-length decoding
    fn run_length_decode(data: &[u8]) -> VexfsResult<Vec<u8>> {
        let mut decoded = Vec::new();
        
        for chunk in data.chunks(2) {
            if chunk.len() == 2 {
                let count = chunk[0];
                let value = chunk[1];
                for _ in 0..count {
                    decoded.push(value);
                }
            }
        }
        
        Ok(decoded)
    }

    /// Helper: Convert bytes to f32 slice
    pub fn bytes_to_f32_slice(data: &[u8]) -> VexfsResult<&[f32]> {
        if data.len() % 4 != 0 {
            return Err(VexfsError::VectorError(crate::shared::errors::VectorErrorKind::DeserializationError));
        }
        
        // Safe conversion from bytes to f32 slice
        let float_slice = unsafe {
            core::slice::from_raw_parts(
                data.as_ptr() as *const f32,
                data.len() / 4
            )
        };
        
        Ok(float_slice)
    }

    /// Helper: Convert f32 slice to bytes
    fn f32_slice_to_bytes(floats: &[f32]) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(floats.len() * 4);
        for &f in floats {
            bytes.extend_from_slice(&f.to_le_bytes());
        }
        bytes
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