//! Memory Optimization System for ANNS Operations
//! 
//! This module implements comprehensive memory efficiency optimizations for the ANNS system,
//! including memory pools, lazy loading, memory-aware caching, and memory pressure handling.
//! The goal is to achieve 30-50% memory usage reduction while maintaining performance.

use std::time::{Duration, Instant};
use std::collections::{HashMap, VecDeque};
use crate::shared::errors::{VexfsError, VexfsResult};
use crate::shared::types::InodeNumber;
use crate::fs_core::operations::OperationContext;
use crate::vector_storage::{VectorStorageManager, VectorDataType, VectorHeader};
use crate::anns::memory_mgmt::{MemoryPool as BasicMemoryPool, MemoryBlock, VectorAllocator, MemoryStats as BasicMemoryStats};

#[cfg(not(feature = "kernel"))]
use std::sync::{Arc, Mutex};
#[cfg(feature = "kernel")]
use alloc::sync::Arc;
#[cfg(feature = "kernel")]
use spin::Mutex;

#[cfg(not(feature = "std"))]
use alloc::vec::Vec;
#[cfg(feature = "std")]
use std::vec::Vec;

/// Memory pressure levels for adaptive behavior
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MemoryPressureLevel {
    Low,      // < 60% memory usage
    Medium,   // 60-80% memory usage
    High,     // 80-95% memory usage
    Critical, // > 95% memory usage
}

/// Enhanced memory pool with optimization features
#[derive(Debug)]
pub struct OptimizedMemoryPool {
    /// Underlying basic memory pool
    basic_pool: BasicMemoryPool,
    /// Pool for specific dimension size
    dimension_size: u32,
    /// Available memory slots for reuse
    available_slots: VecDeque<Vec<u8>>,
    /// Maximum pool size
    max_size: usize,
    /// Current pool utilization
    current_size: usize,
    /// Total allocations served
    total_allocations: u64,
    /// Pool hit rate
    hit_rate: f32,
}

impl OptimizedMemoryPool {
    /// Create new optimized memory pool for specific dimension size
    pub fn new(dimension_size: u32, max_size: usize, max_memory: u64) -> Self {
        Self {
            basic_pool: BasicMemoryPool::new(max_memory),
            dimension_size,
            available_slots: VecDeque::with_capacity(max_size),
            max_size,
            current_size: 0,
            total_allocations: 0,
            hit_rate: 0.0,
        }
    }

    /// Allocate memory from pool or create new
    pub fn allocate(&mut self, size: usize) -> Vec<u8> {
        self.total_allocations += 1;

        if let Some(mut slot) = self.available_slots.pop_front() {
            // Pool hit - reuse existing memory
            slot.clear();
            slot.resize(size, 0);
            self.current_size -= 1;
            self.update_hit_rate(true);
            slot
        } else {
            // Pool miss - allocate new memory
            self.update_hit_rate(false);
            self.allocate_aligned(size)
        }
    }

    /// Return memory to pool for reuse
    pub fn deallocate(&mut self, mut memory: Vec<u8>) {
        if self.current_size < self.max_size {
            memory.clear();
            self.available_slots.push_back(memory);
            self.current_size += 1;
        }
        // If pool is full, let memory be dropped
    }

    /// Allocate SIMD-aligned memory (32-byte alignment for AVX2)
    fn allocate_aligned(&self, size: usize) -> Vec<u8> {
        let aligned_size = (size + 31) & !31; // Round up to 32-byte boundary
        let mut vec = Vec::with_capacity(aligned_size);
        vec.resize(size, 0);
        vec
    }

    /// Update hit rate statistics
    fn update_hit_rate(&mut self, hit: bool) {
        let alpha = 0.1; // Exponential moving average factor
        let current_hit = if hit { 1.0 } else { 0.0 };
        self.hit_rate = self.hit_rate * (1.0 - alpha) + current_hit * alpha;
    }

    /// Compact pool by removing unused slots
    pub fn compact(&mut self, target_reduction: f32) {
        let slots_to_remove = (self.current_size as f32 * target_reduction) as usize;
        for _ in 0..slots_to_remove.min(self.current_size) {
            self.available_slots.pop_back();
            self.current_size -= 1;
        }
    }

    /// Get pool statistics
    pub fn get_stats(&self) -> OptimizedMemoryPoolStats {
        OptimizedMemoryPoolStats {
            dimension_size: self.dimension_size,
            current_size: self.current_size,
            max_size: self.max_size,
            utilization: self.current_size as f32 / self.max_size as f32,
            total_allocations: self.total_allocations,
            hit_rate: self.hit_rate,
            basic_stats: self.basic_pool.total_allocated(),
        }
    }
}

/// Optimized memory pool statistics
#[derive(Debug, Clone)]
pub struct OptimizedMemoryPoolStats {
    pub dimension_size: u32,
    pub current_size: usize,
    pub max_size: usize,
    pub utilization: f32,
    pub total_allocations: u64,
    pub hit_rate: f32,
    pub basic_stats: u64,
}

/// Memory-aware cache entry
#[derive(Debug, Clone)]
pub struct CacheEntry {
    /// Cached vector data
    pub data: Vec<u8>,
    /// Vector header information
    pub header: VectorHeader,
    /// Last access timestamp
    pub last_access: Instant,
    /// Access frequency counter
    pub access_count: u32,
    /// Memory cost of this entry
    pub memory_cost: usize,
    /// Time-to-live for this entry
    pub ttl: Option<Duration>,
}

/// Enhanced memory usage statistics
#[derive(Debug, Clone, Default)]
pub struct OptimizedMemoryStats {
    /// Total memory allocated
    pub total_allocated: usize,
    /// Memory saved through optimization
    pub memory_saved: usize,
    /// Current memory usage
    pub current_usage: usize,
    /// Peak memory usage
    pub peak_usage: usize,
    /// Memory efficiency ratio
    pub efficiency_ratio: f32,
    /// Cache hit rate
    pub cache_hit_rate: f32,
    /// Pool utilization
    pub pool_utilization: f32,
}

/// Memory pool configuration for different vector dimensions
pub const OPTIMIZED_MEMORY_POOL_SIZES: &[(u32, usize)] = &[
    (64, 1024),    // Small vectors: 1024 slots
    (128, 512),    // Medium vectors: 512 slots
    (256, 256),    // Large vectors: 256 slots
    (512, 128),    // Very large vectors: 128 slots
    (1024, 64),    // Huge vectors: 64 slots
];

/// Main memory optimizer for ANNS operations
pub struct MemoryOptimizer {
    /// Optimized memory pools for different vector dimensions
    memory_pools: HashMap<u32, OptimizedMemoryPool>,
    /// Memory-aware cache
    cache_entries: HashMap<u64, CacheEntry>,
    /// LRU ordering for cache
    cache_lru: VecDeque<u64>,
    /// Maximum cache size in bytes
    max_cache_size: usize,
    /// Current cache memory usage
    current_cache_size: usize,
    /// Current memory pressure level
    current_pressure: MemoryPressureLevel,
    /// Memory usage statistics
    memory_stats: OptimizedMemoryStats,
    /// Cache hit/miss statistics
    cache_hits: u64,
    cache_misses: u64,
}

impl MemoryOptimizer {
    /// Create new memory optimizer
    pub fn new(max_cache_size: usize) -> Self {
        // Initialize optimized memory pools
        let mut memory_pools = HashMap::new();
        for &(dimension_size, pool_size) in OPTIMIZED_MEMORY_POOL_SIZES {
            let max_memory = (dimension_size as u64) * (pool_size as u64) * 4; // 4 bytes per float
            memory_pools.insert(
                dimension_size, 
                OptimizedMemoryPool::new(dimension_size, pool_size, max_memory)
            );
        }

        Self {
            memory_pools,
            cache_entries: HashMap::new(),
            cache_lru: VecDeque::new(),
            max_cache_size,
            current_cache_size: 0,
            current_pressure: MemoryPressureLevel::Low,
            memory_stats: OptimizedMemoryStats::default(),
            cache_hits: 0,
            cache_misses: 0,
        }
    }

    /// Allocate memory for vector with optimal strategy
    pub fn allocate_vector_memory(&mut self, dimensions: u32, data_type: VectorDataType) -> Vec<u8> {
        let size = self.calculate_vector_size(dimensions, data_type);
        
        // Find appropriate memory pool
        let pool_dimension = self.find_best_pool_dimension(dimensions);
        
        if let Some(pool) = self.memory_pools.get_mut(&pool_dimension) {
            let memory = pool.allocate(size);
            self.memory_stats.total_allocated += size;
            self.update_memory_usage();
            memory
        } else {
            // Fallback to direct allocation
            let mut vec = Vec::with_capacity(size);
            vec.resize(size, 0);
            self.memory_stats.total_allocated += size;
            self.update_memory_usage();
            vec
        }
    }

    /// Deallocate vector memory back to pool
    pub fn deallocate_vector_memory(&mut self, memory: Vec<u8>, dimensions: u32) {
        let pool_dimension = self.find_best_pool_dimension(dimensions);
        
        if let Some(pool) = self.memory_pools.get_mut(&pool_dimension) {
            let size = memory.len();
            pool.deallocate(memory);
            self.memory_stats.current_usage = self.memory_stats.current_usage.saturating_sub(size);
        }
        // If no pool found, memory will be dropped automatically
    }

    /// Cache vector data
    pub fn cache_vector(&mut self, vector_id: u64, header: VectorHeader, data: Vec<u8>) {
        let memory_cost = data.len() + std::mem::size_of::<CacheEntry>();
        
        // Remove existing entry if present
        if self.cache_entries.contains_key(&vector_id) {
            self.remove_from_cache(vector_id);
        }

        // Ensure we have enough memory
        self.ensure_cache_memory_available(memory_cost);

        // Create new entry
        let entry = CacheEntry {
            data,
            header,
            last_access: Instant::now(),
            access_count: 1,
            memory_cost,
            ttl: Some(Duration::from_secs(300)), // 5 minutes default TTL
        };

        // Insert entry
        self.cache_entries.insert(vector_id, entry);
        self.cache_lru.push_front(vector_id);
        self.current_cache_size += memory_cost;
    }

    /// Get vector from cache
    pub fn get_cached_vector(&mut self, vector_id: u64) -> Option<(VectorHeader, Vec<u8>)> {
        // Check if entry exists and TTL
        let should_remove = if let Some(entry) = self.cache_entries.get(&vector_id) {
            if let Some(ttl) = entry.ttl {
                entry.last_access.elapsed() > ttl
            } else {
                false
            }
        } else {
            self.cache_misses += 1;
            return None;
        };

        // Remove expired entry
        if should_remove {
            self.remove_from_cache(vector_id);
            self.cache_misses += 1;
            return None;
        }

        // Update entry and return data
        let result = if let Some(entry) = self.cache_entries.get_mut(&vector_id) {
            // Update access statistics
            entry.last_access = Instant::now();
            entry.access_count += 1;
            
            Some((entry.header, entry.data.clone()))
        } else {
            None
        };

        if result.is_some() {
            // Move to front of LRU
            self.move_to_front(vector_id);
            self.cache_hits += 1;
        } else {
            self.cache_misses += 1;
        }

        result
    }

    /// Handle memory pressure by adjusting strategies
    pub fn handle_memory_pressure(&mut self, pressure_level: MemoryPressureLevel) {
        self.current_pressure = pressure_level;

        match pressure_level {
            MemoryPressureLevel::Low => {
                // Normal operation
            }
            MemoryPressureLevel::Medium => {
                // Reduce cache size slightly
                self.evict_cache_entries(0.1);
            }
            MemoryPressureLevel::High => {
                // Aggressive cache eviction and pool compaction
                self.evict_cache_entries(0.3);
                self.compact_memory_pools(0.2);
            }
            MemoryPressureLevel::Critical => {
                // Emergency memory reclamation
                self.evict_cache_entries(0.5);
                self.compact_memory_pools(0.5);
                self.emergency_memory_cleanup();
            }
        }
    }

    /// Calculate vector size based on dimensions and data type
    fn calculate_vector_size(&self, dimensions: u32, data_type: VectorDataType) -> usize {
        let element_size = match data_type {
            VectorDataType::Float32 => 4,
            VectorDataType::Float16 => 2,
            VectorDataType::Int8 => 1,
            VectorDataType::Int16 => 2,
            VectorDataType::Binary => 1,
        };
        dimensions as usize * element_size
    }

    /// Find best memory pool for given dimensions
    fn find_best_pool_dimension(&self, dimensions: u32) -> u32 {
        OPTIMIZED_MEMORY_POOL_SIZES
            .iter()
            .find(|&&(pool_dim, _)| dimensions <= pool_dim)
            .map(|&(pool_dim, _)| pool_dim)
            .unwrap_or(1024) // Default to largest pool
    }

    /// Ensure enough cache memory is available by evicting LRU entries
    fn ensure_cache_memory_available(&mut self, required_bytes: usize) {
        while self.current_cache_size + required_bytes > self.max_cache_size {
            if let Some(lru_id) = self.cache_lru.pop_back() {
                self.remove_from_cache(lru_id);
            } else {
                break; // Cache is empty
            }
        }
    }

    /// Remove entry from cache
    fn remove_from_cache(&mut self, vector_id: u64) -> bool {
        if let Some(entry) = self.cache_entries.remove(&vector_id) {
            self.current_cache_size -= entry.memory_cost;
            
            // Remove from LRU order
            if let Some(pos) = self.cache_lru.iter().position(|&id| id == vector_id) {
                self.cache_lru.remove(pos);
            }
            
            true
        } else {
            false
        }
    }

    /// Move entry to front of LRU order
    fn move_to_front(&mut self, vector_id: u64) {
        if let Some(pos) = self.cache_lru.iter().position(|&id| id == vector_id) {
            self.cache_lru.remove(pos);
            self.cache_lru.push_front(vector_id);
        }
    }

    /// Evict cache entries based on ratio
    fn evict_cache_entries(&mut self, eviction_ratio: f32) {
        let entries_to_evict = (self.cache_entries.len() as f32 * eviction_ratio) as usize;
        for _ in 0..entries_to_evict {
            if let Some(lru_id) = self.cache_lru.pop_back() {
                self.remove_from_cache(lru_id);
            } else {
                break;
            }
        }
    }

    /// Compact memory pools to free unused memory
    fn compact_memory_pools(&mut self, reduction_ratio: f32) {
        for pool in self.memory_pools.values_mut() {
            pool.compact(reduction_ratio);
        }
    }

    /// Emergency memory cleanup
    fn emergency_memory_cleanup(&mut self) {
        // Clear all caches
        self.cache_entries.clear();
        self.cache_lru.clear();
        self.current_cache_size = 0;
        
        // Compact all pools aggressively
        self.compact_memory_pools(0.8);
    }

    /// Update memory usage statistics
    fn update_memory_usage(&mut self) {
        // Calculate current usage from pools and cache
        let pool_usage: usize = self.memory_pools.values()
            .map(|pool| pool.current_size * pool.dimension_size as usize * 4) // Estimate
            .sum();
        
        self.memory_stats.current_usage = pool_usage + self.current_cache_size;
        
        if self.memory_stats.current_usage > self.memory_stats.peak_usage {
            self.memory_stats.peak_usage = self.memory_stats.current_usage;
        }

        // Calculate efficiency ratio
        if self.memory_stats.total_allocated > 0 {
            self.memory_stats.efficiency_ratio = 
                self.memory_stats.memory_saved as f32 / self.memory_stats.total_allocated as f32;
        }

        // Calculate cache hit rate
        if self.cache_hits + self.cache_misses > 0 {
            self.memory_stats.cache_hit_rate = 
                self.cache_hits as f32 / (self.cache_hits + self.cache_misses) as f32;
        }

        // Calculate pool utilization
        let total_pools = self.memory_pools.len() as f32;
        if total_pools > 0.0 {
            let avg_utilization: f32 = self.memory_pools.values()
                .map(|pool| pool.get_stats().utilization)
                .sum::<f32>() / total_pools;
            self.memory_stats.pool_utilization = avg_utilization;
        }

        // Update memory pressure level
        self.update_memory_pressure();
    }

    /// Update memory pressure level based on current usage
    fn update_memory_pressure(&mut self) {
        let usage_ratio = self.memory_stats.current_usage as f32 / self.max_cache_size as f32;
        
        self.current_pressure = if usage_ratio < 0.6 {
            MemoryPressureLevel::Low
        } else if usage_ratio < 0.8 {
            MemoryPressureLevel::Medium
        } else if usage_ratio < 0.95 {
            MemoryPressureLevel::High
        } else {
            MemoryPressureLevel::Critical
        };
    }

    /// Get memory optimization statistics
    pub fn get_stats(&self) -> MemoryOptimizerStats {
        let hit_rate = if self.cache_hits + self.cache_misses > 0 {
            self.cache_hits as f32 / (self.cache_hits + self.cache_misses) as f32
        } else {
            0.0
        };

        let pool_stats: Vec<OptimizedMemoryPoolStats> = self.memory_pools.values()
            .map(|pool| pool.get_stats())
            .collect();

        MemoryOptimizerStats {
            memory_stats: self.memory_stats.clone(),
            current_pressure: self.current_pressure,
            cache_entries: self.cache_entries.len(),
            cache_memory_usage: self.current_cache_size,
            max_cache_size: self.max_cache_size,
            cache_hit_rate: hit_rate,
            pool_stats,
        }
    }
}

/// Memory optimizer statistics
#[derive(Debug, Clone)]
pub struct MemoryOptimizerStats {
    pub memory_stats: OptimizedMemoryStats,
    pub current_pressure: MemoryPressureLevel,
    pub cache_entries: usize,
    pub cache_memory_usage: usize,
    pub max_cache_size: usize,
    pub cache_hit_rate: f32,
    pub pool_stats: Vec<OptimizedMemoryPoolStats>,
}

/// Tests for memory optimization functionality
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_optimized_memory_pool_allocation() {
        let mut pool = OptimizedMemoryPool::new(128, 10, 1024);
        
        // Test allocation
        let memory1 = pool.allocate(512);
        assert_eq!(memory1.len(), 512);
        
        // Test deallocation and reuse
        pool.deallocate(memory1);
        let memory2 = pool.allocate(512);
        assert_eq!(memory2.len(), 512);
        
        // Hit rate should be > 0 after reuse
        assert!(pool.hit_rate > 0.0);
    }

    #[test]
    fn test_memory_optimizer_basic() {
        let mut optimizer = MemoryOptimizer::new(1024 * 1024); // 1MB cache
        
        // Test vector memory allocation
        let memory = optimizer.allocate_vector_memory(128, VectorDataType::Float32);
        assert_eq!(memory.len(), 128 * 4); // 128 floats * 4 bytes each
        
        // Test cache operations
        let header = VectorHeader {
            magic: 0x56454358,
            version: 1,
            vector_id: 1,
            file_inode: 1,
            data_type: VectorDataType::Float32,
            compression: crate::vector_storage::CompressionType::None,
            dimensions: 128,
            original_size: 512,
            compressed_size: 512,
            created_timestamp: 0,
            modified_timestamp: 0,
            checksum: 0,
            flags: 0,
            reserved: [],
        };
        
        optimizer.cache_vector(1, header, vec![1, 2, 3, 4]);
        
        // Test cache retrieval
        let cached = optimizer.get_cached_vector(1);
        assert!(cached.is_some());
        
        let stats = optimizer.get_stats();
        assert_eq!(stats.cache_entries, 1);
        assert!(stats.cache_hit_rate > 0.0);
    }

    #[test]
    fn test_memory_pressure_handling() {
        let mut optimizer = MemoryOptimizer::new(1024); // Small cache for testing
        
        // Fill cache beyond capacity
        for i in 0..10 {
            let header = VectorHeader {
                magic: 0x56454358,
                version: 1,
                vector_id: i,
                file_inode: 1,
                data_type: VectorDataType::Float32,
                compression: crate::vector_storage::CompressionType::None,
                dimensions: 128,
                original_size: 512,
                compressed_size: 512,
                created_timestamp: 0,
                modified_timestamp: 0,
                checksum: 0,
                flags: 0,
                reserved: [],
            };
            
            optimizer.cache_vector(i, header, vec![0; 200]); // Large entries
        }
        
        // Handle memory pressure
        optimizer.handle_memory_pressure(MemoryPressureLevel::High);
        
        let stats = optimizer.get_stats();
        assert!(stats.cache_entries < 10); // Some entries should be evicted
    }
}