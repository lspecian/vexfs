//! Memory management for ANNS operations
//! 
//! This module provides functionality for managing memory allocation
//! and deallocation for ANNS indices and operations.

use std::vec::Vec;
use core::alloc::{Layout, GlobalAlloc};
use crate::anns::AnnsError;

/// Memory pool for managing vector data allocations
#[derive(Debug)]
pub struct MemoryPool {
    pools: Vec<Pool>,
    total_allocated: u64,
    max_allocation: u64,
    current_usage: u64,
}

impl MemoryPool {
    /// Create a new memory pool
    pub fn new(max_allocation: u64) -> Self {
        Self {
            pools: Vec::new(),
            total_allocated: 0,
            max_allocation,
            current_usage: 0,
        }
    }

    /// Allocate memory from the pool
    pub fn allocate(&mut self, size: u64) -> Result<MemoryBlock, AnnsError> {
        if self.current_usage + size > self.max_allocation {
            return Err(AnnsError::OutOfMemory);
        }

        // Find a suitable pool or create a new one
        let pool_index = self.find_or_create_pool(size)?;
        let block = self.pools[pool_index].allocate(size)?;
        
        self.current_usage += size;
        self.total_allocated += size;
        
        Ok(block)
    }

    /// Deallocate memory back to the pool
    pub fn deallocate(&mut self, block: MemoryBlock) -> Result<(), AnnsError> {
        // Find the pool this block belongs to
        for pool in &mut self.pools {
            if pool.owns_block(&block) {
                let size = block.size();
                pool.deallocate(block)?;
                self.current_usage -= size;
                return Ok(());
            }
        }
        
        Err(AnnsError::InvalidMemoryBlock)
    }

    /// Find or create a suitable pool for the given size
    fn find_or_create_pool(&mut self, size: u64) -> Result<usize, AnnsError> {
        // Try to find an existing pool with enough space
        for (index, pool) in self.pools.iter().enumerate() {
            if pool.can_allocate(size) {
                return Ok(index);
            }
        }

        // Create a new pool
        let pool_size = (size * 2).max(4096); // At least 4KB pools
        let pool = Pool::new(pool_size)?;
        self.pools.push(pool);
        
        Ok(self.pools.len() - 1)
    }

    /// Get current memory usage
    pub fn current_usage(&self) -> u64 {
        self.current_usage
    }

    /// Get total allocated memory
    pub fn total_allocated(&self) -> u64 {
        self.total_allocated
    }

    /// Get maximum allocation limit
    pub fn max_allocation(&self) -> u64 {
        self.max_allocation
    }

    /// Get number of pools
    pub fn pool_count(&self) -> usize {
        self.pools.len()
    }

    /// Reset the memory pool (deallocate all memory)
    pub fn reset(&mut self) {
        self.pools.clear();
        self.current_usage = 0;
        // Note: total_allocated is cumulative and not reset
    }

    /// Check if the pool can allocate the requested size
    pub fn can_allocate(&self, size: u64) -> bool {
        self.current_usage + size <= self.max_allocation
    }
}

/// Individual memory pool
#[derive(Debug)]
struct Pool {
    buffer: Vec<u8>,
    allocated_blocks: Vec<BlockInfo>,
    free_space: u64,
    total_size: u64,
}

impl Pool {
    /// Create a new pool with the specified size
    fn new(size: u64) -> Result<Self, AnnsError> {
        let mut buffer = Vec::new();
        buffer.try_reserve(size as usize).map_err(|_| AnnsError::OutOfMemory)?;
        buffer.resize(size as usize, 0);

        Ok(Self {
            buffer,
            allocated_blocks: Vec::new(),
            free_space: size,
            total_size: size,
        })
    }

    /// Allocate a block from this pool
    fn allocate(&mut self, size: u64) -> Result<MemoryBlock, AnnsError> {
        if size > self.free_space {
            return Err(AnnsError::OutOfMemory);
        }

        // Find a suitable offset (simple linear allocation for now)
        let offset = self.find_free_offset(size)?;
        
        let block_info = BlockInfo {
            offset,
            size,
            is_allocated: true,
        };

        let block = MemoryBlock {
            ptr: unsafe { self.buffer.as_mut_ptr().add(offset as usize) },
            size,
            pool_id: self as *const Pool as u64,
        };

        self.allocated_blocks.push(block_info);
        self.free_space -= size;

        Ok(block)
    }

    /// Deallocate a block
    fn deallocate(&mut self, block: MemoryBlock) -> Result<(), AnnsError> {
        // Find and remove the block
        let block_index = self.allocated_blocks.iter()
            .position(|info| info.offset == self.ptr_to_offset(block.ptr))
            .ok_or(AnnsError::InvalidMemoryBlock)?;

        let block_info = self.allocated_blocks.remove(block_index);
        self.free_space += block_info.size;

        Ok(())
    }

    /// Check if this pool can allocate the requested size
    fn can_allocate(&self, size: u64) -> bool {
        size <= self.free_space
    }

    /// Check if this pool owns the given block
    fn owns_block(&self, block: &MemoryBlock) -> bool {
        let buffer_start = self.buffer.as_ptr() as u64;
        let buffer_end = buffer_start + self.total_size;
        let block_ptr = block.ptr as u64;
        
        block_ptr >= buffer_start && block_ptr < buffer_end
    }

    /// Find a free offset for allocation
    fn find_free_offset(&self, size: u64) -> Result<u64, AnnsError> {
        let mut offset = 0u64;
        
        // Sort allocated blocks by offset
        let mut sorted_blocks = self.allocated_blocks.clone();
        sorted_blocks.sort_by_key(|block| block.offset);

        for block in &sorted_blocks {
            if offset + size <= block.offset {
                return Ok(offset);
            }
            offset = block.offset + block.size;
        }

        // Check if there's space at the end
        if offset + size <= self.total_size {
            Ok(offset)
        } else {
            Err(AnnsError::OutOfMemory)
        }
    }

    /// Convert pointer to offset within the pool
    fn ptr_to_offset(&self, ptr: *mut u8) -> u64 {
        let buffer_start = self.buffer.as_ptr() as u64;
        let ptr_addr = ptr as u64;
        ptr_addr - buffer_start
    }
}

/// Information about an allocated block
#[derive(Debug, Clone)]
struct BlockInfo {
    offset: u64,
    size: u64,
    is_allocated: bool,
}

/// A memory block allocated from a pool
#[derive(Debug)]
pub struct MemoryBlock {
    ptr: *mut u8,
    size: u64,
    pool_id: u64,
}

impl MemoryBlock {
    /// Get the pointer to the allocated memory
    pub fn as_ptr(&self) -> *mut u8 {
        self.ptr
    }

    /// Get the size of the allocated block
    pub fn size(&self) -> u64 {
        self.size
    }

    /// Get a slice to the allocated memory
    pub unsafe fn as_slice(&self) -> &[u8] {
        core::slice::from_raw_parts(self.ptr, self.size as usize)
    }

    /// Get a mutable slice to the allocated memory
    pub unsafe fn as_mut_slice(&mut self) -> &mut [u8] {
        core::slice::from_raw_parts_mut(self.ptr, self.size as usize)
    }

    /// Get the pool ID this block belongs to
    pub fn pool_id(&self) -> u64 {
        self.pool_id
    }
}

// MemoryBlock is not automatically Send/Sync due to raw pointer
unsafe impl Send for MemoryBlock {}
unsafe impl Sync for MemoryBlock {}

/// Memory allocator for vector operations
pub struct VectorAllocator {
    pool: MemoryPool,
    alignment: usize,
}

impl VectorAllocator {
    /// Create a new vector allocator
    pub fn new(max_memory: u64, alignment: usize) -> Self {
        Self {
            pool: MemoryPool::new(max_memory),
            alignment,
        }
    }

    /// Allocate memory for a vector
    pub fn allocate_vector(&mut self, dimension: u32, element_size: usize) -> Result<MemoryBlock, AnnsError> {
        let size = (dimension as u64) * (element_size as u64);
        let aligned_size = self.align_size(size);
        self.pool.allocate(aligned_size)
    }

    /// Deallocate vector memory
    pub fn deallocate_vector(&mut self, block: MemoryBlock) -> Result<(), AnnsError> {
        self.pool.deallocate(block)
    }

    /// Align size to the required alignment
    fn align_size(&self, size: u64) -> u64 {
        let alignment = self.alignment as u64;
        (size + alignment - 1) & !(alignment - 1)
    }

    /// Get memory statistics
    pub fn stats(&self) -> MemoryStats {
        MemoryStats {
            current_usage: self.pool.current_usage(),
            total_allocated: self.pool.total_allocated(),
            max_allocation: self.pool.max_allocation(),
            pool_count: self.pool.pool_count(),
            alignment: self.alignment,
        }
    }

    /// Reset the allocator
    pub fn reset(&mut self) {
        self.pool.reset();
    }
}

/// Memory usage statistics
#[derive(Debug, Clone)]
pub struct MemoryStats {
    pub current_usage: u64,
    pub total_allocated: u64,
    pub max_allocation: u64,
    pub pool_count: usize,
    pub alignment: usize,
}

impl MemoryStats {
    /// Get memory utilization as a percentage
    pub fn utilization_percent(&self) -> f32 {
        if self.max_allocation == 0 {
            0.0
        } else {
            (self.current_usage as f32 / self.max_allocation as f32) * 100.0
        }
    }

    /// Check if memory usage is critical (>90%)
    pub fn is_critical(&self) -> bool {
        self.utilization_percent() > 90.0
    }

    /// Check if memory usage is high (>75%)
    pub fn is_high(&self) -> bool {
        self.utilization_percent() > 75.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_pool_creation() {
        let pool = MemoryPool::new(1024);
        assert_eq!(pool.max_allocation(), 1024);
        assert_eq!(pool.current_usage(), 0);
        assert_eq!(pool.pool_count(), 0);
    }

    #[test]
    fn test_memory_allocation() {
        let mut pool = MemoryPool::new(1024);
        assert!(pool.can_allocate(512));
        
        let block = pool.allocate(512).unwrap();
        assert_eq!(block.size(), 512);
        assert_eq!(pool.current_usage(), 512);
    }

    #[test]
    fn test_vector_allocator() {
        let mut allocator = VectorAllocator::new(4096, 8);
        let block = allocator.allocate_vector(128, 4).unwrap();
        
        // Should allocate at least 128 * 4 = 512 bytes, aligned to 8
        assert!(block.size() >= 512);
        
        let stats = allocator.stats();
        assert!(stats.current_usage > 0);
    }

    #[test]
    fn test_memory_stats() {
        let stats = MemoryStats {
            current_usage: 750,
            total_allocated: 1000,
            max_allocation: 1000,
            pool_count: 2,
            alignment: 8,
        };
        
        assert_eq!(stats.utilization_percent(), 75.0);
        assert!(!stats.is_critical());
        assert!(stats.is_high());
    }
}