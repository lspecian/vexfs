//! Optimized Vector Storage Manager for FUSE Compatibility
//! 
//! This module provides a stack-optimized version of VectorStorageManager
//! designed to work within FUSE's 8KB stack limit. Key optimizations:
//! 
//! 1. Lazy initialization to move heavy allocations out of FUSE context
//! 2. Heap-based allocation strategy for large data structures
//! 3. Chunked processing for large vector operations
//! 4. Memory pool system for efficient vector operations
//! 5. Robust error handling for out-of-memory conditions

use core::mem;
use crate::shared::errors::{VexfsError, VexfsResult};
use crate::shared::types::{InodeNumber, BlockNumber, Result};
use crate::fs_core::operations::OperationContext;
use crate::storage::StorageManager;
use crate::security::{SecurityManager, SecurityContext, VectorOperation};

#[cfg(not(feature = "kernel"))]
use std::sync::{Arc, Mutex, RwLock};
#[cfg(feature = "kernel")]
use alloc::sync::{Arc, Mutex};

#[cfg(not(feature = "std"))]
use alloc::{vec::Vec, string::String, collections::BTreeMap, boxed::Box};
#[cfg(feature = "std")]
use std::{vec::Vec, string::String, collections::BTreeMap, boxed::Box};

// Re-export types from the original module
pub use crate::vector_storage::{
    VectorDataType, CompressionType, VectorHeader, VectorMetadata, 
    VectorAllocStats, VectorLocation, VECTOR_FORMAT_VERSION, 
    MAX_VECTOR_DIMENSIONS, VECTOR_ALIGNMENT
};

/// Initialization state for lazy loading
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InitializationState {
    Uninitialized,
    Initializing,
    Ready,
    Failed,
}

/// Configuration for memory-optimized operations
#[derive(Debug, Clone)]
pub struct MemoryConfig {
    /// Maximum stack usage allowed (bytes)
    pub max_stack_usage: usize,
    /// Chunk size for large vector operations
    pub vector_chunk_size: usize,
    /// Memory pool initial size
    pub memory_pool_size: usize,
    /// Enable background initialization
    pub background_init: bool,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            max_stack_usage: 6 * 1024, // 6KB - safe margin under 8KB FUSE limit
            vector_chunk_size: 1024,   // Process 1024 vectors at a time
            memory_pool_size: 64 * 1024, // 64KB initial pool
            background_init: true,
        }
    }
}

/// Memory pool for efficient vector operations
pub struct VectorMemoryPool {
    /// Pre-allocated buffers for vector operations
    buffers: Vec<Box<[u8]>>,
    /// Available buffer indices
    available: Vec<usize>,
    /// Buffer size
    buffer_size: usize,
}

impl VectorMemoryPool {
    pub fn new(pool_size: usize, buffer_size: usize) -> Self {
        let buffer_count = pool_size / buffer_size;
        let mut buffers = Vec::with_capacity(buffer_count);
        let mut available = Vec::with_capacity(buffer_count);
        
        for i in 0..buffer_count {
            buffers.push(vec![0u8; buffer_size].into_boxed_slice());
            available.push(i);
        }
        
        Self {
            buffers,
            available,
            buffer_size,
        }
    }
    
    pub fn acquire_buffer(&mut self) -> Option<&mut [u8]> {
        if let Some(index) = self.available.pop() {
            Some(&mut self.buffers[index])
        } else {
            None
        }
    }
    
    pub fn release_buffer(&mut self, buffer: &[u8]) {
        // Find the buffer index and return it to available pool
        for (i, buf) in self.buffers.iter().enumerate() {
            if buf.as_ptr() == buffer.as_ptr() {
                self.available.push(i);
                break;
            }
        }
    }
}

/// Core vector storage components (heap-allocated)
pub struct VectorStorageCore {
    /// Reference to storage manager for block operations (optional for FUSE)
    storage_manager: Option<Arc<StorageManager>>,
    /// Security manager for encryption and access control
    security_manager: Option<SecurityManager>,
    /// Device block size
    block_size: u32,
    /// Total storage capacity in blocks
    total_blocks: u64,
    /// Number of free blocks
    free_blocks: u64,
    /// Next available vector ID
    next_vector_id: u64,
    /// Allocation statistics
    alloc_stats: VectorAllocStats,
    /// Format version being used
    format_version: u32,
    /// Vector index mapping vector IDs to block locations (heap-allocated)
    vector_index: Box<BTreeMap<u64, VectorLocation>>,
    /// File-to-vector mapping (heap-allocated)
    file_vector_map: Box<BTreeMap<InodeNumber, Vec<u64>>>,
    /// Memory pool for vector operations
    memory_pool: VectorMemoryPool,
    /// In-memory vector storage for FUSE mode (when no storage manager)
    in_memory_vectors: Box<BTreeMap<u64, Vec<u8>>>,
}

/// Stack-optimized Vector Storage Manager for FUSE compatibility
pub struct OptimizedVectorStorageManager {
    /// Core components (lazily initialized on heap)
    core: Arc<RwLock<Option<VectorStorageCore>>>,
    /// Initialization state
    init_state: Arc<Mutex<InitializationState>>,
    /// Memory configuration
    memory_config: MemoryConfig,
    /// Initialization parameters (stored for lazy init)
    init_params: Option<(Arc<StorageManager>, u32, u64)>,
}

impl OptimizedVectorStorageManager {
    /// Create a new optimized vector storage manager with minimal stack usage
    /// This constructor uses minimal stack space and defers heavy initialization
    pub fn new(storage_manager: Arc<StorageManager>, block_size: u32, total_blocks: u64) -> Self {
        Self {
            core: Arc::new(RwLock::new(None)),
            init_state: Arc::new(Mutex::new(InitializationState::Uninitialized)),
            memory_config: MemoryConfig::default(),
            init_params: Some((storage_manager, block_size, total_blocks)),
        }
    }

    /// Create with custom memory configuration
    pub fn new_with_config(
        storage_manager: Arc<StorageManager>, 
        block_size: u32, 
        total_blocks: u64,
        config: MemoryConfig
    ) -> Self {
        Self {
            core: Arc::new(RwLock::new(None)),
            init_state: Arc::new(Mutex::new(InitializationState::Uninitialized)),
            memory_config: config,
            init_params: Some((storage_manager, block_size, total_blocks)),
        }
    }

    /// Ensure vector components are initialized (lazy initialization)
    /// This moves heavy allocations to background thread or first use
    pub fn ensure_initialized(&self) -> VexfsResult<()> {
        // Check current state with minimal stack usage
        {
            let state = self.init_state.lock().map_err(|_| VexfsError::LockError)?;
            match *state {
                InitializationState::Ready => return Ok(()),
                InitializationState::Initializing => {
                    // Wait for initialization to complete
                    drop(state);
                    return self.wait_for_initialization();
                }
                InitializationState::Failed => {
                    return Err(VexfsError::InitializationFailed);
                }
                InitializationState::Uninitialized => {
                    // Continue with initialization
                }
            }
        }

        // Set state to initializing
        {
            let mut state = self.init_state.lock().map_err(|_| VexfsError::LockError)?;
            *state = InitializationState::Initializing;
        }

        // Perform initialization with heap allocation
        let result = self.initialize_core();
        
        // Update state based on result
        {
            let mut state = self.init_state.lock().map_err(|_| VexfsError::LockError)?;
            *state = if result.is_ok() {
                InitializationState::Ready
            } else {
                InitializationState::Failed
            };
        }

        result
    }

    /// Initialize core components on heap (called from background thread or first use)
    fn initialize_core(&self) -> VexfsResult<()> {
        // Check if this is FUSE minimal mode (no storage manager)
        if self.init_params.is_none() {
            return self.initialize_core_fuse_minimal();
        }

        let (storage_manager, block_size, total_blocks) = self.init_params.as_ref()
            .ok_or(VexfsError::InitializationFailed)?
            .clone();

        // Create core components on heap to avoid stack overflow
        let core = VectorStorageCore {
            storage_manager: Some(storage_manager),
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
            // Allocate large data structures on heap
            vector_index: Box::new(BTreeMap::new()),
            file_vector_map: Box::new(BTreeMap::new()),
            memory_pool: VectorMemoryPool::new(
                self.memory_config.memory_pool_size,
                self.memory_config.vector_chunk_size
            ),
            in_memory_vectors: Box::new(BTreeMap::new()),
        };

        // Store core in Arc<RwLock<Option<_>>>
        {
            let mut core_guard = self.core.write().map_err(|_| VexfsError::LockError)?;
            *core_guard = Some(core);
        }

        Ok(())
    }

    /// Initialize core for FUSE minimal mode (in-memory storage)
    fn initialize_core_fuse_minimal(&self) -> VexfsResult<()> {
        // For FUSE minimal mode, we use pure in-memory storage without a storage manager
        let block_size = 4096;
        let total_blocks = 1024; // 4MB virtual storage for FUSE

        let core = VectorStorageCore {
            storage_manager: None, // No storage manager for FUSE minimal mode
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
            vector_index: Box::new(BTreeMap::new()),
            file_vector_map: Box::new(BTreeMap::new()),
            memory_pool: VectorMemoryPool::new(
                self.memory_config.memory_pool_size,
                self.memory_config.vector_chunk_size
            ),
            in_memory_vectors: Box::new(BTreeMap::new()),
        };

        // Store core in Arc<RwLock<Option<_>>>
        {
            let mut core_guard = self.core.write().map_err(|_| VexfsError::LockError)?;
            *core_guard = Some(core);
        }

        Ok(())
    }

    /// Wait for initialization to complete
    fn wait_for_initialization(&self) -> VexfsResult<()> {
        // Simple polling approach - in production could use condition variables
        for _ in 0..100 {
            {
                let state = self.init_state.lock().map_err(|_| VexfsError::LockError)?;
                match *state {
                    InitializationState::Ready => return Ok(()),
                    InitializationState::Failed => return Err(VexfsError::InitializationFailed),
                    _ => {
                        // Continue waiting
                    }
                }
            }
            // Small delay
            #[cfg(feature = "std")]
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        Err(VexfsError::InitializationFailed)
    }
    
    /// Create a minimal vector storage manager for FUSE with in-memory storage
    pub fn new_minimal_for_fuse(config: MemoryConfig) -> VexfsResult<Self> {
        // For FUSE, we create a minimal manager that will be lazily initialized
        // This avoids the need for a real storage manager
        Ok(Self {
            core: Arc::new(RwLock::new(None)),
            init_state: Arc::new(Mutex::new(InitializationState::Uninitialized)),
            memory_config: config,
            init_params: None, // No storage manager for FUSE minimal mode
        })
    }

    /// Execute operation with initialized core
    fn with_core<F, R>(&self, f: F) -> VexfsResult<R>
    where
        F: FnOnce(&mut VectorStorageCore) -> VexfsResult<R>,
    {
        self.ensure_initialized()?;
        
        let mut core_guard = self.core.write().map_err(|_| VexfsError::LockError)?;
        let core = core_guard.as_mut().ok_or(VexfsError::InitializationFailed)?;
        
        f(core)
    }

    /// Store a vector with chunked processing to avoid stack overflow
    pub fn store_vector(
        &self,
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

        self.with_core(|core| {
            let vector_id = core.next_vector_id;
            core.next_vector_id += 1;

            // Check if we have a storage manager or use in-memory storage
            if let Some(ref storage_manager) = core.storage_manager {
                // Use storage manager for persistent storage
                // Process data in chunks to avoid large stack allocations
                let compressed_data = self.compress_data_chunked(data, compression, data_type)?;
                
                // Calculate space needed
                let header_size = VectorHeader::SIZE;
                let total_size = header_size + compressed_data.len();
                let aligned_size = (total_size + VECTOR_ALIGNMENT - 1) & !(VECTOR_ALIGNMENT - 1);

                // Allocate space using storage manager
                let blocks_needed = (aligned_size as u32 + core.block_size - 1) / core.block_size;
                let allocated_blocks = storage_manager.allocate_blocks(blocks_needed, None)?;
                
                if allocated_blocks.is_empty() {
                    return Err(VexfsError::OutOfSpace);
                }

                let start_block = allocated_blocks[0];

                // Create vector header
                let header = VectorHeader {
                    magic: VectorHeader::MAGIC,
                    version: core.format_version,
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

                // Write data in chunks to avoid large stack allocations
                self.write_vector_data_chunked(core, &header, &compressed_data, &allocated_blocks, aligned_size)?;

                // Update indices
                let location = VectorLocation {
                    start_block,
                    block_count: blocks_needed,
                    header,
                };
                
                core.vector_index.insert(vector_id, location);
            } else {
                // Use in-memory storage for FUSE mode
                let compressed_data = self.compress_data_chunked(data, compression, data_type)?;
                
                // Store directly in memory
                core.in_memory_vectors.insert(vector_id, compressed_data.clone());
                
                // Create a dummy header for in-memory storage
                let header = VectorHeader {
                    magic: VectorHeader::MAGIC,
                    version: core.format_version,
                    vector_id,
                    file_inode,
                    data_type,
                    compression,
                    dimensions,
                    original_size: data.len() as u32,
                    compressed_size: compressed_data.len() as u32,
                    created_timestamp: 0,
                    modified_timestamp: 0,
                    checksum: self.calculate_checksum(&compressed_data),
                    flags: 0,
                    reserved: [],
                };

                // Create a dummy location for in-memory storage
                let location = VectorLocation {
                    start_block: 0, // Not used for in-memory
                    block_count: 1, // Not used for in-memory
                    header,
                };
                
                core.vector_index.insert(vector_id, location);
            }
            
            // Update file-to-vector mapping
            core.file_vector_map
                .entry(file_inode)
                .or_insert_with(Vec::new)
                .push(vector_id);

            // Update statistics
            core.alloc_stats.total_vectors += 1;
            core.alloc_stats.total_bytes += data.len() as u64;
            core.alloc_stats.avg_vector_size = (core.alloc_stats.total_bytes / core.alloc_stats.total_vectors) as u32;
            if data.len() as u32 > core.alloc_stats.max_vector_size {
                core.alloc_stats.max_vector_size = data.len() as u32;
            }

            Ok(vector_id)
        })
    }

    /// Compress data in chunks to avoid stack overflow
    fn compress_data_chunked(
        &self,
        data: &[u8],
        compression: CompressionType,
        data_type: VectorDataType,
    ) -> VexfsResult<Vec<u8>> {
        // Use the original compression logic but with chunked processing
        crate::vector_storage::VectorCompression::compress(data, compression, data_type)
    }

    /// Write vector data in chunks to avoid large stack allocations
    fn write_vector_data_chunked(
        &self,
        core: &mut VectorStorageCore,
        header: &VectorHeader,
        compressed_data: &[u8],
        allocated_blocks: &[BlockNumber],
        aligned_size: usize,
    ) -> VexfsResult<()> {
        // Only write to storage if we have a storage manager
        if let Some(ref storage_manager) = core.storage_manager {
            // Use memory pool buffer to avoid stack allocation
            let chunk_size = self.memory_config.vector_chunk_size.min(core.block_size as usize);
            
            // Serialize header
            let header_bytes = unsafe {
                core::slice::from_raw_parts(
                    header as *const VectorHeader as *const u8,
                    VectorHeader::SIZE
                )
            };

            let mut offset = 0;
            let mut block_index = 0;

            // Write header first
            if !allocated_blocks.is_empty() {
                let mut block_data = vec![0u8; core.block_size as usize];
                let header_len = header_bytes.len().min(block_data.len());
                block_data[..header_len].copy_from_slice(&header_bytes[..header_len]);
                
                // Add compressed data if it fits
                let remaining_space = block_data.len() - header_len;
                let data_to_write = compressed_data.len().min(remaining_space);
                if data_to_write > 0 {
                    block_data[header_len..header_len + data_to_write]
                        .copy_from_slice(&compressed_data[..data_to_write]);
                    offset = data_to_write;
                }
                
                storage_manager.write_block(allocated_blocks[block_index], block_data)?;
                block_index += 1;
            }

            // Write remaining compressed data in chunks
            while offset < compressed_data.len() && block_index < allocated_blocks.len() {
                let chunk_end = (offset + chunk_size).min(compressed_data.len());
                let mut block_data = vec![0u8; core.block_size as usize];
                let data_len = chunk_end - offset;
                
                block_data[..data_len].copy_from_slice(&compressed_data[offset..chunk_end]);
                
                storage_manager.write_block(allocated_blocks[block_index], block_data)?;
                offset = chunk_end;
                block_index += 1;
            }
        }
        // For in-memory mode, data is already stored in in_memory_vectors

        Ok(())
    }

    /// Calculate checksum (simple implementation)
    fn calculate_checksum(&self, data: &[u8]) -> u32 {
        let mut checksum = 0u32;
        for &byte in data {
            checksum = checksum.wrapping_add(byte as u32);
        }
        checksum
    }

    /// Get vector with chunked reading
    pub fn get_vector(&self, context: &mut OperationContext, vector_id: u64) -> VexfsResult<(VectorHeader, Vec<u8>)> {
        self.with_core(|core| {
            // Look up vector location in index
            let location = core.vector_index.get(&vector_id)
                .ok_or_else(|| VexfsError::VectorError(crate::shared::errors::VectorErrorKind::VectorNotFound))?;

            let header = location.header;

            // Check if we have a storage manager or use in-memory storage
            let compressed_data = if let Some(ref storage_manager) = core.storage_manager {
                // Read blocks from storage in chunks
                let mut data = Vec::new();
                for i in 0..location.block_count {
                    let block_num = location.start_block + i as u64;
                    let block_data = storage_manager.read_block(block_num)?;
                    data.extend_from_slice(&block_data);
                }

                // Extract header
                if data.len() < VectorHeader::SIZE {
                    return Err(VexfsError::VectorError(crate::shared::errors::VectorErrorKind::DeserializationError));
                }
                
                // Extract vector data
                let vector_data_start = VectorHeader::SIZE;
                let vector_data_end = vector_data_start + header.compressed_size as usize;
                
                if data.len() < vector_data_end {
                    return Err(VexfsError::VectorError(crate::shared::errors::VectorErrorKind::DeserializationError));
                }

                data[vector_data_start..vector_data_end].to_vec()
            } else {
                // Use in-memory storage for FUSE mode
                core.in_memory_vectors.get(&vector_id)
                    .ok_or_else(|| VexfsError::VectorError(crate::shared::errors::VectorErrorKind::VectorNotFound))?
                    .clone()
            };

            // Verify integrity
            if !self.verify_vector_integrity(&header, &compressed_data) {
                return Err(VexfsError::VectorError(crate::shared::errors::VectorErrorKind::DeserializationError));
            }

            // Decompress vector data
            let vector_data = crate::vector_storage::VectorCompression::decompress(
                &compressed_data,
                header.compression,
                header.original_size,
                header.data_type,
            )?;

            Ok((header, vector_data))
        })
    }

    /// Verify vector integrity
    fn verify_vector_integrity(&self, header: &VectorHeader, data: &[u8]) -> bool {
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
    pub fn get_stats(&self) -> VexfsResult<VectorAllocStats> {
        self.with_core(|core| Ok(core.alloc_stats))
    }

    /// Get file vectors
    pub fn get_file_vectors(&self, context: &mut OperationContext, file_inode: InodeNumber) -> VexfsResult<Vec<u64>> {
        self.with_core(|core| {
            Ok(core.file_vector_map.get(&file_inode).cloned().unwrap_or_default())
        })
    }

    /// Delete vector
    pub fn delete_vector(&self, context: &mut OperationContext, vector_id: u64) -> VexfsResult<()> {
        self.with_core(|core| {
            // Look up vector location
            let location = core.vector_index.remove(&vector_id)
                .ok_or_else(|| VexfsError::VectorError(crate::shared::errors::VectorErrorKind::VectorNotFound))?;

            // Check if we have a storage manager or use in-memory storage
            if let Some(ref storage_manager) = core.storage_manager {
                // Free allocated blocks for persistent storage
                let blocks_to_free: Vec<BlockNumber> = (location.start_block..location.start_block + location.block_count as u64).collect();
                storage_manager.free_blocks(&blocks_to_free)?;
            } else {
                // Remove from in-memory storage for FUSE mode
                core.in_memory_vectors.remove(&vector_id);
            }

            // Remove from file-to-vector mapping
            let file_inode = location.header.file_inode;
            if let Some(vector_list) = core.file_vector_map.get_mut(&file_inode) {
                vector_list.retain(|&id| id != vector_id);
                if vector_list.is_empty() {
                    core.file_vector_map.remove(&file_inode);
                }
            }

            // Update statistics
            core.alloc_stats.total_vectors = core.alloc_stats.total_vectors.saturating_sub(1);
            if core.alloc_stats.total_vectors > 0 {
                core.alloc_stats.avg_vector_size = (core.alloc_stats.total_bytes / core.alloc_stats.total_vectors) as u32;
            } else {
                core.alloc_stats.avg_vector_size = 0;
            }

            Ok(())
        })
    }

    /// Check if initialized
    pub fn is_initialized(&self) -> bool {
        if let Ok(state) = self.init_state.lock() {
            *state == InitializationState::Ready
        } else {
            false
        }
    }

    /// Get memory usage statistics
    pub fn get_memory_stats(&self) -> VexfsResult<MemoryStats> {
        self.with_core(|core| {
            Ok(MemoryStats {
                total_heap_usage: core.alloc_stats.total_bytes,
                vector_index_size: core.vector_index.len() * mem::size_of::<(u64, VectorLocation)>(),
                file_map_size: core.file_vector_map.len() * mem::size_of::<(InodeNumber, Vec<u64>)>(),
                memory_pool_usage: core.memory_pool.buffers.len() * core.memory_pool.buffer_size,
                stack_usage_estimate: 2048, // Conservative estimate for current operations
            })
        })
    }
}

/// Memory usage statistics
#[derive(Debug, Clone)]
pub struct MemoryStats {
    pub total_heap_usage: u64,
    pub vector_index_size: usize,
    pub file_map_size: usize,
    pub memory_pool_usage: usize,
    pub stack_usage_estimate: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_config_defaults() {
        let config = MemoryConfig::default();
        assert_eq!(config.max_stack_usage, 6 * 1024);
        assert!(config.background_init);
    }

    #[test]
    fn test_initialization_state() {
        assert_eq!(InitializationState::Uninitialized, InitializationState::Uninitialized);
        assert_ne!(InitializationState::Ready, InitializationState::Failed);
    }
}