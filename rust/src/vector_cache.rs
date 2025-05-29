//! Vector Caching System for VexFS
//!
//! This module implements comprehensive caching mechanisms for vector embeddings
//! and ANNS index structures to improve query performance and optimize memory usage.
//! It provides kernel memory caches, cache eviction policies, prefetching strategies,
//! and cache coherence mechanisms.

use crate::shared::errors::{VexfsError, VexfsResult};
use crate::shared::types::{InodeNumber, BlockNumber, VectorId, Size, Timestamp};
use crate::shared::constants::*;
use crate::fs_core::operations::OperationContext;
use crate::vector_storage::{VectorHeader, VectorDataType, CompressionType};
use crate::storage::cache::{CacheStats, CacheState};

#[cfg(not(feature = "kernel"))]
use std::sync::{Arc, RwLock, Mutex};
#[cfg(feature = "kernel")]
use alloc::sync::Arc;

#[cfg(not(feature = "std"))]
use alloc::{vec::Vec, collections::BTreeMap, boxed::Box};
#[cfg(feature = "std")]
use std::{vec::Vec, collections::BTreeMap, boxed::Box};

use core::mem;

/// Simple index segment placeholder for caching
#[derive(Debug, Clone)]
pub struct IndexSegment {
    pub id: u64,
    pub data: Vec<u8>,
    pub level: u8,
    pub connections: u32,
}

impl IndexSegment {
    pub fn new(id: u64, data: Vec<u8>) -> Self {
        Self {
            id,
            data,
            level: 0,
            connections: 0,
        }
    }

    pub fn memory_size(&self) -> usize {
        mem::size_of::<Self>() + self.data.len()
    }
}

/// Vector cache entry containing cached vector data and metadata
#[derive(Debug, Clone)]
pub struct VectorCacheEntry {
    /// Vector ID
    pub vector_id: VectorId,
    /// Associated file inode
    pub inode: InodeNumber,
    /// Vector header information
    pub header: VectorHeader,
    /// Cached vector data (decompressed)
    pub data: Vec<u8>,
    /// Cache state
    pub state: CacheState,
    /// Access count for LFU eviction
    pub access_count: u64,
    /// Last access timestamp
    pub last_access: Timestamp,
    /// Entry size in bytes
    pub size: usize,
    /// Reference count for pinning
    pub ref_count: u32,
    /// Prefetch priority (higher = more important)
    pub prefetch_priority: u8,
    /// Compression ratio achieved
    pub compression_ratio: f32,
}

impl VectorCacheEntry {
    /// Create new vector cache entry
    pub fn new(vector_id: VectorId, inode: InodeNumber, header: VectorHeader, data: Vec<u8>) -> Self {
        let size = mem::size_of::<Self>() + data.len();
        let compression_ratio = if header.compressed_size > 0 {
            header.original_size as f32 / header.compressed_size as f32
        } else {
            1.0
        };

        Self {
            vector_id,
            inode,
            header,
            data,
            state: CacheState::Clean,
            access_count: 1,
            last_access: crate::shared::utils::current_time(),
            size,
            ref_count: 0,
            prefetch_priority: 0,
            compression_ratio,
        }
    }

    /// Mark entry as accessed
    pub fn mark_accessed(&mut self) {
        self.access_count += 1;
        self.last_access = crate::shared::utils::current_time();
    }

    /// Check if entry can be evicted
    pub fn can_evict(&self) -> bool {
        self.ref_count == 0 && self.state != CacheState::Locked
    }

    /// Get entry age in seconds
    pub fn get_age(&self) -> u64 {
        crate::shared::utils::current_time().saturating_sub(self.last_access)
    }

    /// Calculate access frequency (accesses per second)
    pub fn access_frequency(&self) -> f64 {
        let age = self.get_age().max(1);
        self.access_count as f64 / age as f64
    }

    /// Calculate cache value score for eviction decisions
    pub fn cache_value_score(&self) -> f64 {
        let frequency = self.access_frequency();
        let size_penalty = 1.0 / (self.size as f64).sqrt();
        let compression_bonus = self.compression_ratio as f64;
        let priority_bonus = (self.prefetch_priority as f64) / 255.0;
        
        frequency * size_penalty * compression_bonus * (1.0 + priority_bonus)
    }
}

/// ANNS index cache entry for caching index segments
#[derive(Debug, Clone)]
pub struct IndexCacheEntry {
    /// Index segment identifier
    pub segment_id: u64,
    /// Associated inode
    pub inode: InodeNumber,
    /// Cached index segment
    pub segment: IndexSegment,
    /// Cache state
    pub state: CacheState,
    /// Access count
    pub access_count: u64,
    /// Last access timestamp
    pub last_access: Timestamp,
    /// Entry size in bytes
    pub size: usize,
    /// Reference count
    pub ref_count: u32,
    /// Level in HNSW hierarchy (0 = base layer)
    pub level: u8,
    /// Number of connections in this segment
    pub connection_count: u32,
}

impl IndexCacheEntry {
    /// Create new index cache entry
    pub fn new(segment_id: u64, inode: InodeNumber, segment: IndexSegment) -> Self {
        let size = mem::size_of::<Self>() + segment.memory_size();
        
        Self {
            segment_id,
            inode,
            segment,
            state: CacheState::Clean,
            access_count: 1,
            last_access: crate::shared::utils::current_time(),
            size,
            ref_count: 0,
            level: 0,
            connection_count: 0,
        }
    }

    /// Mark entry as accessed
    pub fn mark_accessed(&mut self) {
        self.access_count += 1;
        self.last_access = crate::shared::utils::current_time();
    }

    /// Check if entry can be evicted
    pub fn can_evict(&self) -> bool {
        self.ref_count == 0 && self.state != CacheState::Locked
    }

    /// Calculate cache value score
    pub fn cache_value_score(&self) -> f64 {
        let age = crate::shared::utils::current_time().saturating_sub(self.last_access).max(1);
        let frequency = self.access_count as f64 / age as f64;
        let size_penalty = 1.0 / (self.size as f64).sqrt();
        let level_bonus = if self.level == 0 { 2.0 } else { 1.0 }; // Base layer is more valuable
        
        frequency * size_penalty * level_bonus
    }
}

/// Cache eviction policy
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EvictionPolicy {
    /// Least Recently Used
    LRU,
    /// Least Frequently Used
    LFU,
    /// Adaptive Replacement Cache (combines LRU and LFU)
    ARC,
    /// Custom value-based eviction
    ValueBased,
}

/// Prefetch strategy
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PrefetchStrategy {
    /// No prefetching
    None,
    /// Sequential prefetching
    Sequential,
    /// Spatial locality prefetching
    Spatial,
    /// Predictive prefetching based on access patterns
    Predictive,
    /// Hybrid strategy combining multiple approaches
    Hybrid,
}

/// Cache coherence mode
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CoherenceMode {
    /// No coherence (cache-only)
    None,
    /// Write-through coherence
    WriteThrough,
    /// Write-back coherence
    WriteBack,
    /// Invalidation-based coherence
    Invalidation,
}

/// Vector cache configuration
#[derive(Debug, Clone)]
pub struct VectorCacheConfig {
    /// Maximum cache size in bytes
    pub max_size: usize,
    /// Maximum number of entries
    pub max_entries: usize,
    /// Eviction policy
    pub eviction_policy: EvictionPolicy,
    /// Prefetch strategy
    pub prefetch_strategy: PrefetchStrategy,
    /// Coherence mode
    pub coherence_mode: CoherenceMode,
    /// Enable compression in cache
    pub enable_compression: bool,
    /// Memory pressure threshold (0.0-1.0)
    pub memory_pressure_threshold: f32,
    /// Prefetch batch size
    pub prefetch_batch_size: usize,
    /// Enable cache warming
    pub enable_cache_warming: bool,
}

impl Default for VectorCacheConfig {
    fn default() -> Self {
        Self {
            max_size: VEXFS_DEFAULT_VECTOR_CACHE_SIZE * 1024, // Convert to bytes
            max_entries: VEXFS_DEFAULT_VECTOR_CACHE_SIZE,
            eviction_policy: EvictionPolicy::ARC,
            prefetch_strategy: PrefetchStrategy::Hybrid,
            coherence_mode: CoherenceMode::WriteBack,
            enable_compression: true,
            memory_pressure_threshold: VEXFS_DEFAULT_MEMORY_PRESSURE_THRESHOLD,
            prefetch_batch_size: 8,
            enable_cache_warming: true,
        }
    }
}

/// Vector cache statistics
#[derive(Debug, Clone)]
pub struct VectorCacheStats {
    /// Vector-specific hit count
    pub hits: u64,
    /// Vector-specific miss count
    pub misses: u64,
    /// Index hit count
    pub index_hits: u64,
    /// Index miss count
    pub index_misses: u64,
    /// Prefetch hits
    pub prefetch_hits: u64,
    /// Prefetch misses
    pub prefetch_misses: u64,
    /// Compression savings in bytes
    pub compression_savings: u64,
    /// Average access latency in microseconds
    pub avg_access_latency_us: u64,
    /// Memory pressure level (0.0-1.0)
    pub memory_pressure: f32,
    /// Cache warming progress (0.0-1.0)
    pub warming_progress: f32,
    /// Total entries
    pub total_entries: usize,
    /// Eviction count
    pub eviction_count: u64,
    /// Memory usage
    pub memory_usage: u64,
}

impl Default for VectorCacheStats {
    fn default() -> Self {
        Self {
            hits: 0,
            misses: 0,
            index_hits: 0,
            index_misses: 0,
            prefetch_hits: 0,
            prefetch_misses: 0,
            compression_savings: 0,
            avg_access_latency_us: 0,
            memory_pressure: 0.0,
            warming_progress: 0.0,
            total_entries: 0,
            eviction_count: 0,
            memory_usage: 0,
        }
    }
}

impl VectorCacheStats {
    /// Calculate vector hit rate
    pub fn vector_hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }

    /// Calculate index hit rate
    pub fn index_hit_rate(&self) -> f64 {
        let total = self.index_hits + self.index_misses;
        if total == 0 {
            0.0
        } else {
            self.index_hits as f64 / total as f64
        }
    }

    /// Calculate prefetch effectiveness
    pub fn prefetch_effectiveness(&self) -> f64 {
        let total = self.prefetch_hits + self.prefetch_misses;
        if total == 0 {
            0.0
        } else {
            self.prefetch_hits as f64 / total as f64
        }
    }

    /// Calculate overall cache efficiency
    pub fn overall_efficiency(&self) -> f64 {
        let vector_rate = self.vector_hit_rate();
        let index_rate = self.index_hit_rate();
        let prefetch_rate = self.prefetch_effectiveness();
        
        // Weighted average with vector cache being most important
        (vector_rate * 0.6) + (index_rate * 0.3) + (prefetch_rate * 0.1)
    }
}

/// Access pattern tracker for predictive prefetching
#[derive(Debug)]
struct AccessPattern {
    /// Recent access sequence
    recent_accesses: Vec<VectorId>,
    /// Access frequency map
    frequency_map: BTreeMap<VectorId, u64>,
    /// Spatial locality map (vector_id -> nearby vectors)
    spatial_map: BTreeMap<VectorId, Vec<VectorId>>,
    /// Temporal patterns
    temporal_patterns: Vec<Vec<VectorId>>,
}

impl AccessPattern {
    fn new() -> Self {
        Self {
            recent_accesses: Vec::with_capacity(1000),
            frequency_map: BTreeMap::new(),
            spatial_map: BTreeMap::new(),
            temporal_patterns: Vec::new(),
        }
    }

    /// Record a vector access
    fn record_access(&mut self, vector_id: VectorId) {
        // Update recent accesses
        self.recent_accesses.push(vector_id);
        if self.recent_accesses.len() > 1000 {
            self.recent_accesses.remove(0);
        }

        // Update frequency
        *self.frequency_map.entry(vector_id).or_insert(0) += 1;

        // Analyze patterns periodically
        if self.recent_accesses.len() % 100 == 0 {
            self.analyze_patterns();
        }
    }

    /// Predict next likely accesses
    fn predict_next_accesses(&self, current_vector: VectorId, count: usize) -> Vec<VectorId> {
        let mut predictions = Vec::new();

        // Add spatially related vectors
        if let Some(spatial_neighbors) = self.spatial_map.get(&current_vector) {
            predictions.extend(spatial_neighbors.iter().take(count / 2));
        }

        // Add frequently accessed vectors
        let mut frequent: Vec<_> = self.frequency_map.iter().collect();
        frequent.sort_by(|a, b| b.1.cmp(a.1));
        predictions.extend(frequent.iter().take(count / 2).map(|(id, _)| **id));

        predictions.truncate(count);
        predictions
    }

    /// Analyze access patterns to build prediction models
    fn analyze_patterns(&mut self) {
        // Analyze spatial locality
        self.analyze_spatial_locality();
        
        // Analyze temporal patterns
        self.analyze_temporal_patterns();
    }

    fn analyze_spatial_locality(&mut self) {
        // Build spatial relationships based on co-occurrence in access patterns
        for window in self.recent_accesses.windows(5) {
            for (i, &vector_id) in window.iter().enumerate() {
                let neighbors = self.spatial_map.entry(vector_id).or_insert_with(Vec::new);
                for (j, &neighbor_id) in window.iter().enumerate() {
                    if i != j && !neighbors.contains(&neighbor_id) {
                        neighbors.push(neighbor_id);
                    }
                }
                // Keep only top 10 spatial neighbors
                neighbors.sort_by_key(|&id| self.frequency_map.get(&id).unwrap_or(&0));
                neighbors.truncate(10);
            }
        }
    }

    fn analyze_temporal_patterns(&mut self) {
        // Extract common access sequences
        let mut patterns = Vec::new();
        for window in self.recent_accesses.windows(3) {
            patterns.push(window.to_vec());
        }
        
        // Keep unique patterns
        patterns.sort();
        patterns.dedup();
        
        self.temporal_patterns = patterns;
        self.temporal_patterns.truncate(100); // Keep top 100 patterns
    }
}

/// Main vector cache manager
pub struct VectorCacheManager {
    /// Configuration
    config: VectorCacheConfig,
    /// Vector cache entries
    vector_cache: BTreeMap<VectorId, VectorCacheEntry>,
    /// Index cache entries
    index_cache: BTreeMap<u64, IndexCacheEntry>,
    /// Current cache size in bytes
    current_size: usize,
    /// Cache statistics
    stats: VectorCacheStats,
    /// Access pattern tracker
    access_patterns: AccessPattern,
    /// Prefetch queue
    prefetch_queue: Vec<VectorId>,
    /// Cache warming queue
    warming_queue: Vec<VectorId>,
    /// Memory pressure monitor
    memory_pressure: f32,
}

impl VectorCacheManager {
    /// Create new vector cache manager
    pub fn new(config: VectorCacheConfig) -> Self {
        Self {
            config,
            vector_cache: BTreeMap::new(),
            index_cache: BTreeMap::new(),
            current_size: 0,
            stats: VectorCacheStats::default(),
            access_patterns: AccessPattern::new(),
            prefetch_queue: Vec::new(),
            warming_queue: Vec::new(),
            memory_pressure: 0.0,
        }
    }

    /// Create with default configuration
    pub fn with_defaults() -> Self {
        Self::new(VectorCacheConfig::default())
    }

    /// Get vector from cache
    pub fn get_vector(&mut self, vector_id: VectorId) -> Option<VectorCacheEntry> {
        let start_time = crate::shared::utils::current_time();
        
        if let Some(entry) = self.vector_cache.get_mut(&vector_id) {
            entry.mark_accessed();
            self.stats.hits += 1;
            let result = entry.clone();
            
            // Record access pattern after releasing the borrow
            drop(entry);
            self.access_patterns.record_access(vector_id);
            
            // Update latency stats
            let latency = crate::shared::utils::current_time() - start_time;
            self.update_latency_stats(latency);
            
            // Trigger prefetching if enabled
            if self.config.prefetch_strategy != PrefetchStrategy::None {
                self.trigger_prefetch(vector_id);
            }
            
            Some(result)
        } else {
            self.stats.misses += 1;
            None
        }
    }

    /// Insert vector into cache
    pub fn insert_vector(
        &mut self,
        vector_id: VectorId,
        inode: InodeNumber,
        header: VectorHeader,
        data: Vec<u8>,
    ) -> VexfsResult<()> {
        // Check if we need to evict entries
        let entry_size = mem::size_of::<VectorCacheEntry>() + data.len();
        while self.should_evict(entry_size) {
            self.evict_vector_entry()?;
        }

        // Create and insert entry
        let entry = VectorCacheEntry::new(vector_id, inode, header, data);
        self.current_size += entry.size;
        self.vector_cache.insert(vector_id, entry);

        // Update statistics
        self.stats.total_entries += 1;
        self.update_memory_pressure();

        Ok(())
    }

    /// Get index segment from cache
    pub fn get_index_segment(&mut self, segment_id: u64) -> Option<IndexCacheEntry> {
        let start_time = crate::shared::utils::current_time();
        
        if let Some(entry) = self.index_cache.get_mut(&segment_id) {
            entry.mark_accessed();
            self.stats.index_hits += 1;
            let result = entry.clone();
            
            // Release the borrow before calling update_latency_stats
            drop(entry);
            let latency = crate::shared::utils::current_time() - start_time;
            self.update_latency_stats(latency);
            
            Some(result)
        } else {
            self.stats.index_misses += 1;
            None
        }
    }

    /// Insert index segment into cache
    pub fn insert_index_segment(
        &mut self,
        segment_id: u64,
        inode: InodeNumber,
        segment: IndexSegment,
    ) -> VexfsResult<()> {
        let entry_size = mem::size_of::<IndexCacheEntry>() + segment.memory_size();
        
        // Check if we need to evict entries
        while self.should_evict(entry_size) {
            self.evict_index_entry()?;
        }

        // Create and insert entry
        let entry = IndexCacheEntry::new(segment_id, inode, segment);
        self.current_size += entry.size;
        self.index_cache.insert(segment_id, entry);

        self.stats.total_entries += 1;
        self.update_memory_pressure();

        Ok(())
    }

    /// Invalidate vector cache entry
    pub fn invalidate_vector(&mut self, vector_id: VectorId) -> VexfsResult<()> {
        if let Some(entry) = self.vector_cache.remove(&vector_id) {
            self.current_size = self.current_size.saturating_sub(entry.size);
            self.stats.total_entries = self.stats.total_entries.saturating_sub(1);
            self.update_memory_pressure();
        }
        Ok(())
    }

    /// Invalidate index cache entry
    pub fn invalidate_index_segment(&mut self, segment_id: u64) -> VexfsResult<()> {
        if let Some(entry) = self.index_cache.remove(&segment_id) {
            self.current_size = self.current_size.saturating_sub(entry.size);
            self.stats.total_entries = self.stats.total_entries.saturating_sub(1);
            self.update_memory_pressure();
        }
        Ok(())
    }

    /// Flush all dirty entries
    pub fn flush_dirty(&mut self) -> VexfsResult<Vec<(VectorId, Vec<u8>)>> {
        let mut dirty_vectors = Vec::new();
        
        for (vector_id, entry) in self.vector_cache.iter_mut() {
            if entry.state == CacheState::Dirty {
                dirty_vectors.push((*vector_id, entry.data.clone()));
                entry.state = CacheState::Clean;
            }
        }
        
        Ok(dirty_vectors)
    }

    /// Get cache statistics
    pub fn get_stats(&self) -> VectorCacheStats {
        let mut stats = self.stats.clone();
        stats.memory_usage = self.current_size as u64;
        stats.memory_pressure = self.memory_pressure;
        stats.total_entries = self.vector_cache.len() + self.index_cache.len();
        stats
    }

    /// Perform cache maintenance
    pub fn maintenance(&mut self) -> VexfsResult<()> {
        // Update memory pressure
        self.update_memory_pressure();
        
        // Process prefetch queue
        self.process_prefetch_queue()?;
        
        // Process cache warming
        if self.config.enable_cache_warming {
            self.process_cache_warming()?;
        }
        
        // Evict entries if under memory pressure
        if self.memory_pressure > self.config.memory_pressure_threshold {
            self.aggressive_eviction()?;
        }
        
        Ok(())
    }

    /// Warm cache with frequently accessed vectors
    pub fn warm_cache(&mut self, context: &mut OperationContext) -> VexfsResult<()> {
        if !self.config.enable_cache_warming {
            return Ok(());
        }

        // Identify frequently accessed vectors from access patterns
        let mut frequent_vectors: Vec<_> = self.access_patterns.frequency_map.iter().collect();
        frequent_vectors.sort_by(|a, b| b.1.cmp(a.1));
        
        // Add to warming queue
        for (vector_id, _) in frequent_vectors.iter().take(100) {
            if !self.vector_cache.contains_key(vector_id) {
                self.warming_queue.push(**vector_id);
            }
        }
        
        Ok(())
    }

    /// Set cache configuration
    pub fn set_config(&mut self, config: VectorCacheConfig) {
        self.config = config;
        self.update_memory_pressure();
    }

    /// Get current cache utilization
    pub fn get_utilization(&self) -> f64 {
        if self.config.max_size == 0 {
            0.0
        } else {
            self.current_size as f64 / self.config.max_size as f64
        }
    }

    // Private helper methods

    fn should_evict(&self, new_entry_size: usize) -> bool {
        (self.current_size + new_entry_size) > self.config.max_size ||
        self.vector_cache.len() + self.index_cache.len() >= self.config.max_entries
    }

    fn evict_vector_entry(&mut self) -> VexfsResult<()> {
        let victim_id = match self.config.eviction_policy {
            EvictionPolicy::LRU => self.find_lru_vector_victim(),
            EvictionPolicy::LFU => self.find_lfu_vector_victim(),
            EvictionPolicy::ARC => self.find_arc_vector_victim(),
            EvictionPolicy::ValueBased => self.find_value_based_vector_victim(),
        };

        if let Some(vector_id) = victim_id {
            if let Some(entry) = self.vector_cache.remove(&vector_id) {
                self.current_size = self.current_size.saturating_sub(entry.size);
                self.stats.eviction_count += 1;
            }
        }

        Ok(())
    }

    fn evict_index_entry(&mut self) -> VexfsResult<()> {
        let victim_id = match self.config.eviction_policy {
            EvictionPolicy::LRU => self.find_lru_index_victim(),
            EvictionPolicy::LFU => self.find_lfu_index_victim(),
            EvictionPolicy::ARC => self.find_arc_index_victim(),
            EvictionPolicy::ValueBased => self.find_value_based_index_victim(),
        };

        if let Some(segment_id) = victim_id {
            if let Some(entry) = self.index_cache.remove(&segment_id) {
                self.current_size = self.current_size.saturating_sub(entry.size);
                self.stats.eviction_count += 1;
            }
        }

        Ok(())
    }

    fn find_lru_vector_victim(&self) -> Option<VectorId> {
        self.vector_cache
            .iter()
            .filter(|(_, entry)| entry.can_evict())
            .min_by_key(|(_, entry)| entry.last_access)
            .map(|(id, _)| *id)
    }

    fn find_lfu_vector_victim(&self) -> Option<VectorId> {
        self.vector_cache
            .iter()
            .filter(|(_, entry)| entry.can_evict())
            .min_by_key(|(_, entry)| entry.access_count)
            .map(|(id, _)| *id)
    }

    fn find_arc_vector_victim(&self) -> Option<VectorId> {
        // Simplified ARC: combine recency and frequency
        self.vector_cache
            .iter()
            .filter(|(_, entry)| entry.can_evict())
            .min_by(|(_, a), (_, b)| {
                let score_a = a.access_frequency() * (1.0 / a.get_age() as f64);
                let score_b = b.access_frequency() * (1.0 / b.get_age() as f64);
                score_a.partial_cmp(&score_b).unwrap_or(core::cmp::Ordering::Equal)
            })
            .map(|(id, _)| *id)
    }

    fn find_value_based_vector_victim(&self) -> Option<VectorId> {
        self.vector_cache
            .iter()
            .filter(|(_, entry)| entry.can_evict())
            .min_by(|(_, a), (_, b)| {
                a.cache_value_score().partial_cmp(&b.cache_value_score()).unwrap_or(core::cmp::Ordering::Equal)
            })
            .map(|(id, _)| *id)
    }

    fn find_lru_index_victim(&self) -> Option<u64> {
        self.index_cache
            .iter()
            .filter(|(_, entry)| entry.can_evict())
            .min_by_key(|(_, entry)| entry.last_access)
            .map(|(id, _)| *id)
    }

    fn find_lfu_index_victim(&self) -> Option<u64> {
        self.index_cache
            .iter()
            .filter(|(_, entry)| entry.can_evict())
            .min_by_key(|(_, entry)| entry.access_count)
            .map(|(id, _)| *id)
    }

    fn find_arc_index_victim(&self) -> Option<u64> {
        self.index_cache
            .iter()
            .filter(|(_, entry)| entry.can_evict())
            .min_by(|(_, a), (_, b)| {
                let age_a = a.last_access;
                let age_b = b.last_access;
                let freq_a = a.access_count;
                let freq_b = b.access_count;
                
                // Combine recency and frequency
                let score_a = (freq_a as f64) / (age_a as f64 + 1.0);
                let score_b = (freq_b as f64) / (age_b as f64 + 1.0);
                
                score_a.partial_cmp(&score_b).unwrap_or(core::cmp::Ordering::Equal)
            })
            .map(|(id, _)| *id)
    }

    fn find_value_based_index_victim(&self) -> Option<u64> {
        self.index_cache
            .iter()
            .filter(|(_, entry)| entry.can_evict())
            .min_by(|(_, a), (_, b)| {
                a.cache_value_score().partial_cmp(&b.cache_value_score()).unwrap_or(core::cmp::Ordering::Equal)
            })
            .map(|(id, _)| *id)
    }

    fn trigger_prefetch(&mut self, vector_id: VectorId) {
        if self.prefetch_queue.len() >= self.config.prefetch_batch_size * 2 {
            return; // Queue is full
        }

        let predictions = self.access_patterns.predict_next_accesses(
            vector_id,
            self.config.prefetch_batch_size,
        );

        for predicted_id in predictions {
            if !self.vector_cache.contains_key(&predicted_id) &&
               !self.prefetch_queue.contains(&predicted_id) {
                self.prefetch_queue.push(predicted_id);
            }
        }
    }

    fn process_prefetch_queue(&mut self) -> VexfsResult<()> {
        // Process a limited number of prefetch requests per maintenance cycle
        let batch_size = self.config.prefetch_batch_size.min(self.prefetch_queue.len());
        
        for _ in 0..batch_size {
            if let Some(vector_id) = self.prefetch_queue.pop() {
                // In a real implementation, this would trigger async loading
                // For now, we just track the prefetch attempt
                if self.vector_cache.contains_key(&vector_id) {
                    self.stats.prefetch_hits += 1;
                } else {
                    self.stats.prefetch_misses += 1;
                }
            }
        }
        
        Ok(())
    }

    fn process_cache_warming(&mut self) -> VexfsResult<()> {
        // Process warming queue
        let batch_size = 4.min(self.warming_queue.len());
        
        for _ in 0..batch_size {
            if let Some(_vector_id) = self.warming_queue.pop() {
                // In a real implementation, this would trigger async loading
                // Update warming progress
                let total_warming = self.access_patterns.frequency_map.len().min(100);
                let remaining = self.warming_queue.len();
                self.stats.warming_progress = if total_warming > 0 {
                    1.0 - (remaining as f32 / total_warming as f32)
                } else {
                    1.0
                };
            }
        }
        
        Ok(())
    }

    fn aggressive_eviction(&mut self) -> VexfsResult<()> {
        // Evict multiple entries when under memory pressure
        let target_evictions = (self.vector_cache.len() + self.index_cache.len()) / 10; // Evict 10%
        
        for _ in 0..target_evictions {
            if self.vector_cache.len() > self.index_cache.len() {
                self.evict_vector_entry()?;
            } else {
                self.evict_index_entry()?;
            }
            
            // Stop if memory pressure is relieved
            if self.memory_pressure <= self.config.memory_pressure_threshold {
                break;
            }
        }
        
        Ok(())
    }

    fn update_memory_pressure(&mut self) {
        self.memory_pressure = if self.config.max_size > 0 {
            self.current_size as f32 / self.config.max_size as f32
        } else {
            0.0
        };
    }

    fn update_latency_stats(&mut self, latency: u64) {
        // Simple moving average for latency
        if self.stats.avg_access_latency_us == 0 {
            self.stats.avg_access_latency_us = latency;
        } else {
            self.stats.avg_access_latency_us = 
                (self.stats.avg_access_latency_us * 9 + latency) / 10;
        }
    }
}

/// Vector cache integration with VectorStorageManager
pub struct VectorCacheIntegration {
    /// Cache manager
    cache_manager: VectorCacheManager,
    /// Enable cache coherence
    coherence_enabled: bool,
}

impl VectorCacheIntegration {
    /// Create new cache integration
    pub fn new(config: VectorCacheConfig) -> Self {
        let coherence_enabled = config.coherence_mode != CoherenceMode::None;
        Self {
            cache_manager: VectorCacheManager::new(config),
            coherence_enabled,
        }
    }

    /// Get vector with caching
    pub fn get_vector_cached(
        &mut self,
        vector_id: VectorId,
        context: &mut OperationContext,
        storage_manager: &mut crate::vector_storage::VectorStorageManager,
    ) -> VexfsResult<(VectorHeader, Vec<u8>)> {
        // Try cache first
        if let Some(entry) = self.cache_manager.get_vector(vector_id) {
            return Ok((entry.header, entry.data.clone()));
        }

        // Cache miss - load from storage
        let (header, data) = storage_manager.get_vector(context, vector_id)?;
        
        // Insert into cache
        let inode = header.file_inode;
        self.cache_manager.insert_vector(vector_id, inode, header, data.clone())?;
        
        Ok((header, data))
    }

    /// Store vector with caching
    pub fn store_vector_cached(
        &mut self,
        context: &mut OperationContext,
        storage_manager: &mut crate::vector_storage::VectorStorageManager,
        data: &[u8],
        file_inode: InodeNumber,
        data_type: VectorDataType,
        dimensions: u32,
        compression: CompressionType,
    ) -> VexfsResult<u64> {
        // Store to persistent storage
        let vector_id = storage_manager.store_vector(
            context, data, file_inode, data_type, dimensions, compression
        )?;

        // Handle cache coherence
        match self.cache_manager.config.coherence_mode {
            CoherenceMode::WriteThrough => {
                // Data is already written to storage, cache the result
                let header = VectorHeader {
                    magic: crate::vector_storage::VectorHeader::MAGIC,
                    version: 1,
                    vector_id,
                    file_inode,
                    data_type,
                    compression,
                    dimensions,
                    original_size: data.len() as u32,
                    compressed_size: data.len() as u32, // Simplified
                    created_timestamp: crate::shared::utils::current_time(),
                    modified_timestamp: crate::shared::utils::current_time(),
                    checksum: 0, // Simplified
                    flags: 0,
                    reserved: [],
                };
                
                self.cache_manager.insert_vector(vector_id, file_inode, header, data.to_vec())?;
            }
            CoherenceMode::WriteBack => {
                // Cache the data, mark as dirty for later writeback
                // This is handled by the storage manager in this case
            }
            CoherenceMode::Invalidation => {
                // Invalidate any existing cache entries for this vector
                self.cache_manager.invalidate_vector(vector_id)?;
            }
            CoherenceMode::None => {
                // No cache coherence
            }
        }

        Ok(vector_id)
    }

    /// Delete vector with cache invalidation
    pub fn delete_vector_cached(
        &mut self,
        context: &mut OperationContext,
        storage_manager: &mut crate::vector_storage::VectorStorageManager,
        vector_id: VectorId,
    ) -> VexfsResult<()> {
        // Delete from storage
        storage_manager.delete_vector(context, vector_id)?;
        
        // Invalidate cache entry
        self.cache_manager.invalidate_vector(vector_id)?;
        
        Ok(())
    }

    /// Get cache statistics
    pub fn get_cache_stats(&self) -> VectorCacheStats {
        self.cache_manager.get_stats()
    }

    /// Perform cache maintenance
    pub fn maintenance(&mut self) -> VexfsResult<()> {
        self.cache_manager.maintenance()
    }

    /// Warm cache
    pub fn warm_cache(&mut self, context: &mut OperationContext) -> VexfsResult<()> {
        self.cache_manager.warm_cache(context)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vector_cache_entry_creation() {
        let header = VectorHeader {
            magic: 0x56455856,
            version: 1,
            vector_id: 123,
            file_inode: 456,
            data_type: VectorDataType::Float32,
            compression: CompressionType::None,
            dimensions: 128,
            original_size: 512,
            compressed_size: 512,
            created_timestamp: 0,
            modified_timestamp: 0,
            checksum: 0,
            flags: 0,
            reserved: [],
        };
        
        let data = vec![0u8; 512];
        let entry = VectorCacheEntry::new(123, 456, header, data);
        
        assert_eq!(entry.vector_id, 123);
        assert_eq!(entry.inode, 456);
        assert_eq!(entry.access_count, 1);
        assert!(entry.can_evict());
    }

    #[test]
    fn test_cache_manager_basic_operations() {
        let mut cache = VectorCacheManager::with_defaults();
        
        // Test cache miss
        assert!(cache.get_vector(123).is_none());
        assert_eq!(cache.stats.misses, 1);
        
        // Insert vector
        let header = VectorHeader {
            magic: 0x56455856,
            version: 1,
            vector_id: 123,
            file_inode: 456,
            data_type: VectorDataType::Float32,
            compression: CompressionType::None,
            dimensions: 128,
            original_size: 512,
            compressed_size: 512,
            created_timestamp: 0,
            modified_timestamp: 0,
            checksum: 0,
            flags: 0,
            reserved: [],
        };
        
        let data = vec![0u8; 512];
        cache.insert_vector(123, 456, header, data).unwrap();
        
        // Test cache hit
        assert!(cache.get_vector(123).is_some());
        assert_eq!(cache.stats.hits, 1);
    }

    #[test]
    fn test_eviction_policies() {
        let config = VectorCacheConfig {
            max_size: 1024, // Small cache for testing
            max_entries: 2,
            eviction_policy: EvictionPolicy::LRU,
            ..Default::default()
        };
        
        let mut cache = VectorCacheManager::new(config);
        
        // Fill cache to capacity
        for i in 0..3 {
            let header = VectorHeader {
                magic: 0x56455856,
                version: 1,
                vector_id: i,
                file_inode: i + 100,
                data_type: VectorDataType::Float32,
                compression: CompressionType::None,
                dimensions: 32,
                original_size: 128,
                compressed_size: 128,
                created_timestamp: 0,
                modified_timestamp: 0,
                checksum: 0,
                flags: 0,
                reserved: [],
            };
            
            let data = vec![0u8; 128];
            cache.insert_vector(i, i + 100, header, data).unwrap();
        }
        
        // Should have evicted the first entry
        assert!(cache.get_vector(0).is_none());
        assert!(cache.get_vector(1).is_some());
        assert!(cache.get_vector(2).is_some());
    }

    #[test]
    fn test_access_pattern_tracking() {
        let mut patterns = AccessPattern::new();
        
        // Record some accesses
        for i in 0..10 {
            patterns.record_access(i % 3); // Access vectors 0, 1, 2 repeatedly
        }
        
        // Should have frequency data
        assert!(patterns.frequency_map.contains_key(&0));
        assert!(patterns.frequency_map.contains_key(&1));
        assert!(patterns.frequency_map.contains_key(&2));
        
        // Test predictions
        let predictions = patterns.predict_next_accesses(0, 2);
        assert!(!predictions.is_empty());
    }
}