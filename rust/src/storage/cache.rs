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

//! Block Caching System
//!
//! This module provides block caching strategies including LRU and LFU algorithms
//! for improving VexFS performance through intelligent block buffering.

use crate::shared::errors::{VexfsError, VexfsResult};
use crate::shared::types::*;
use crate::shared::constants::*;
use crate::shared::utils::*;
use crate::shared::types::{BlockNumber, InodeNumber};
use core::mem;

#[cfg(not(feature = "kernel"))]
use std::collections::BTreeMap;
#[cfg(feature = "kernel")]
use alloc::collections::BTreeMap;
#[cfg(feature = "kernel")]
use alloc::vec::Vec;

/// Cache entry state
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CacheState {
    /// Clean - matches disk
    Clean,
    /// Dirty - needs writeback
    Dirty,
    /// Locked - being accessed
    Locked,
    /// Invalid - needs reload
    Invalid,
}

/// Block cache entry
#[derive(Debug, Clone)]
pub struct CacheEntry {
    /// Block number
    pub block: BlockNumber,
    /// Block data
    pub data: Vec<u8>,
    /// Cache state
    pub state: CacheState,
    /// Reference count
    pub ref_count: u32,
    /// Access count for LFU
    pub access_count: u64,
    /// Last access time for LRU
    pub last_access: u64,
    /// Dirty since timestamp
    pub dirty_time: u64,
}

impl CacheEntry {
    /// Create new cache entry
    pub fn new(block: BlockNumber, data: Vec<u8>) -> Self {
        Self {
            block,
            data,
            state: CacheState::Clean,
            ref_count: 0,
            access_count: 1,
            last_access: current_time(),
            dirty_time: 0,
        }
    }

    /// Mark entry as accessed
    pub fn mark_accessed(&mut self) {
        self.access_count += 1;
        self.last_access = current_time();
    }

    /// Mark entry as dirty
    pub fn mark_dirty(&mut self) {
        if self.state != CacheState::Dirty {
            self.state = CacheState::Dirty;
            self.dirty_time = current_time();
        }
    }

    /// Mark entry as clean
    pub fn mark_clean(&mut self) {
        self.state = CacheState::Clean;
        self.dirty_time = 0;
    }

    /// Check if entry can be evicted
    pub fn can_evict(&self) -> bool {
        self.ref_count == 0 && self.state != CacheState::Locked
    }

    /// Get entry age in seconds
    pub fn get_age(&self) -> u64 {
        current_time().saturating_sub(self.last_access)
    }

    /// Check if entry needs writeback
    pub fn needs_writeback(&self) -> bool {
        self.state == CacheState::Dirty
    }
}

/// LRU (Least Recently Used) cache implementation
pub struct LruCache {
    /// Cache entries indexed by block number
    entries: BTreeMap<BlockNumber, CacheEntry>,
    /// Maximum cache size in entries
    max_entries: usize,
    /// Current cache size in bytes
    current_size: usize,
    /// Maximum cache size in bytes
    max_size: usize,
    /// Block size for size calculations
    block_size: u32,
    /// Cache hit count
    hit_count: u64,
    /// Cache miss count
    miss_count: u64,
    /// Next eviction candidate
    eviction_cursor: Option<BlockNumber>,
}

impl LruCache {
    /// Create new LRU cache
    pub fn new(max_entries: usize, max_size: usize, block_size: u32) -> Self {
        Self {
            entries: BTreeMap::new(),
            max_entries,
            current_size: 0,
            max_size,
            block_size,
            hit_count: 0,
            miss_count: 0,
            eviction_cursor: None,
        }
    }

    /// Get block from cache
    pub fn get(&mut self, block: BlockNumber) -> Option<&mut CacheEntry> {
        if let Some(entry) = self.entries.get_mut(&block) {
            entry.mark_accessed();
            self.hit_count += 1;
            Some(entry)
        } else {
            self.miss_count += 1;
            None
        }
    }

    /// Insert block into cache
    pub fn insert(&mut self, block: BlockNumber, data: Vec<u8>) -> VexfsResult<()> {
        // Check if we need to evict entries
        while self.should_evict() {
            self.evict_lru()?;
        }

        let entry = CacheEntry::new(block, data);
        let entry_size = entry.data.len();

        if let Some(old_entry) = self.entries.insert(block, entry) {
            // Replace existing entry
            self.current_size = self.current_size.saturating_sub(old_entry.data.len());
        }

        self.current_size += entry_size;
        Ok(())
    }

    /// Remove block from cache
    pub fn remove(&mut self, block: BlockNumber) -> Option<CacheEntry> {
        if let Some(entry) = self.entries.remove(&block) {
            self.current_size = self.current_size.saturating_sub(entry.data.len());
            
            // Update eviction cursor if needed
            if self.eviction_cursor == Some(block) {
                self.eviction_cursor = self.entries.keys().next().copied();
            }
            
            Some(entry)
        } else {
            None
        }
    }

    /// Flush dirty entries
    pub fn flush_dirty(&mut self) -> VexfsResult<Vec<(BlockNumber, Vec<u8>)>> {
        let mut dirty_blocks = Vec::new();
        
        for (block, entry) in self.entries.iter_mut() {
            if entry.needs_writeback() {
                dirty_blocks.push((*block, entry.data.clone()));
                entry.mark_clean();
            }
        }
        
        Ok(dirty_blocks)
    }

    /// Get cache statistics
    pub fn get_stats(&self) -> CacheStats {
        let total_requests = self.hit_count + self.miss_count;
        let hit_rate = if total_requests > 0 {
            (self.hit_count as f32 / total_requests as f32) * 100.0
        } else {
            0.0
        };

        let dirty_count = self.entries.values()
            .filter(|e| e.needs_writeback())
            .count();

        CacheStats {
            hit_count: self.hit_count,
            miss_count: self.miss_count,
            hit_rate,
            entry_count: self.entries.len(),
            max_entries: self.max_entries,
            current_size: self.current_size,
            max_size: self.max_size,
            dirty_count,
            utilization: (self.current_size as f32 / self.max_size as f32) * 100.0,
        }
    }

    /// Clear all cache entries
    pub fn clear(&mut self) {
        self.entries.clear();
        self.current_size = 0;
        self.eviction_cursor = None;
    }

    /// Shrink cache to target size
    pub fn shrink_to(&mut self, target_entries: usize) -> VexfsResult<()> {
        while self.entries.len() > target_entries {
            self.evict_lru()?;
        }
        Ok(())
    }

    // Private methods

    fn should_evict(&self) -> bool {
        self.entries.len() >= self.max_entries || self.current_size >= self.max_size
    }

    fn evict_lru(&mut self) -> VexfsResult<()> {
        let evict_block = self.find_lru_candidate()?;
        
        let entry = self.entries.get(&evict_block)
            .ok_or(VexfsError::Internal("LRU candidate not found".to_string()))?;

        if entry.needs_writeback() {
            return Err(VexfsError::CacheDirty);
        }

        if !entry.can_evict() {
            return Err(VexfsError::CacheLocked);
        }

        self.remove(evict_block);
        Ok(())
    }

    fn find_lru_candidate(&self) -> VexfsResult<BlockNumber> {
        let mut oldest_time = u64::MAX;
        let mut candidate = None;

        for (block, entry) in &self.entries {
            if entry.can_evict() && entry.last_access < oldest_time {
                oldest_time = entry.last_access;
                candidate = Some(*block);
            }
        }

        candidate.ok_or(VexfsError::NoSpace)
    }
}

/// LFU (Least Frequently Used) cache implementation
pub struct LfuCache {
    /// Cache entries indexed by block number
    entries: BTreeMap<BlockNumber, CacheEntry>,
    /// Maximum cache size in entries
    max_entries: usize,
    /// Current cache size in bytes
    current_size: usize,
    /// Maximum cache size in bytes
    max_size: usize,
    /// Block size for size calculations
    block_size: u32,
    /// Cache hit count
    hit_count: u64,
    /// Cache miss count
    miss_count: u64,
}

impl LfuCache {
    /// Create new LFU cache
    pub fn new(max_entries: usize, max_size: usize, block_size: u32) -> Self {
        Self {
            entries: BTreeMap::new(),
            max_entries,
            current_size: 0,
            max_size,
            block_size,
            hit_count: 0,
            miss_count: 0,
        }
    }

    /// Get block from cache
    pub fn get(&mut self, block: BlockNumber) -> Option<&mut CacheEntry> {
        if let Some(entry) = self.entries.get_mut(&block) {
            entry.mark_accessed();
            self.hit_count += 1;
            Some(entry)
        } else {
            self.miss_count += 1;
            None
        }
    }

    /// Insert block into cache
    pub fn insert(&mut self, block: BlockNumber, data: Vec<u8>) -> VexfsResult<()> {
        // Check if we need to evict entries
        while self.should_evict() {
            self.evict_lfu()?;
        }

        let entry = CacheEntry::new(block, data);
        let entry_size = entry.data.len();

        if let Some(old_entry) = self.entries.insert(block, entry) {
            // Replace existing entry
            self.current_size = self.current_size.saturating_sub(old_entry.data.len());
        }

        self.current_size += entry_size;
        Ok(())
    }

    /// Remove block from cache
    pub fn remove(&mut self, block: BlockNumber) -> Option<CacheEntry> {
        if let Some(entry) = self.entries.remove(&block) {
            self.current_size = self.current_size.saturating_sub(entry.data.len());
            Some(entry)
        } else {
            None
        }
    }

    /// Flush dirty entries
    pub fn flush_dirty(&mut self) -> VexfsResult<Vec<(BlockNumber, Vec<u8>)>> {
        let mut dirty_blocks = Vec::new();
        
        for (block, entry) in self.entries.iter_mut() {
            if entry.needs_writeback() {
                dirty_blocks.push((*block, entry.data.clone()));
                entry.mark_clean();
            }
        }
        
        Ok(dirty_blocks)
    }

    /// Get cache statistics
    pub fn get_stats(&self) -> CacheStats {
        let total_requests = self.hit_count + self.miss_count;
        let hit_rate = if total_requests > 0 {
            (self.hit_count as f32 / total_requests as f32) * 100.0
        } else {
            0.0
        };

        let dirty_count = self.entries.values()
            .filter(|e| e.needs_writeback())
            .count();

        CacheStats {
            hit_count: self.hit_count,
            miss_count: self.miss_count,
            hit_rate,
            entry_count: self.entries.len(),
            max_entries: self.max_entries,
            current_size: self.current_size,
            max_size: self.max_size,
            dirty_count,
            utilization: (self.current_size as f32 / self.max_size as f32) * 100.0,
        }
    }

    /// Clear all cache entries
    pub fn clear(&mut self) {
        self.entries.clear();
        self.current_size = 0;
    }

    // Private methods

    fn should_evict(&self) -> bool {
        self.entries.len() >= self.max_entries || self.current_size >= self.max_size
    }

    fn evict_lfu(&mut self) -> VexfsResult<()> {
        let evict_block = self.find_lfu_candidate()?;
        
        let entry = self.entries.get(&evict_block)
            .ok_or(VexfsError::Internal("LFU candidate not found".to_string()))?;

        if entry.needs_writeback() {
            return Err(VexfsError::CacheDirty);
        }

        if !entry.can_evict() {
            return Err(VexfsError::CacheLocked);
        }

        self.remove(evict_block);
        Ok(())
    }

    fn find_lfu_candidate(&self) -> VexfsResult<BlockNumber> {
        let mut lowest_count = u64::MAX;
        let mut candidate = None;

        for (block, entry) in &self.entries {
            if entry.can_evict() && entry.access_count < lowest_count {
                lowest_count = entry.access_count;
                candidate = Some(*block);
            }
        }

        candidate.ok_or(VexfsError::NoSpace)
    }
}

/// Adaptive cache that switches between LRU and LFU based on workload
pub struct AdaptiveCache {
    /// LRU cache instance
    lru_cache: LruCache,
    /// LFU cache instance
    lfu_cache: LfuCache,
    /// Current active cache strategy
    active_strategy: CacheStrategy,
    /// Performance monitoring window
    monitor_window: u64,
    /// Last strategy evaluation time
    last_evaluation: u64,
    /// LRU performance in current window
    lru_performance: f32,
    /// LFU performance in current window
    lfu_performance: f32,
}

/// Cache strategy selection
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CacheStrategy {
    LRU,
    LFU,
}

impl AdaptiveCache {
    /// Create new adaptive cache
    pub fn new(max_entries: usize, max_size: usize, block_size: u32) -> Self {
        let half_entries = max_entries / 2;
        let half_size = max_size / 2;

        Self {
            lru_cache: LruCache::new(half_entries, half_size, block_size),
            lfu_cache: LfuCache::new(half_entries, half_size, block_size),
            active_strategy: CacheStrategy::LRU,
            monitor_window: 3600, // 1 hour monitoring window
            last_evaluation: current_time(),
            lru_performance: 0.0,
            lfu_performance: 0.0,
        }
    }

    /// Get block from cache
    pub fn get(&mut self, block: BlockNumber) -> Option<&mut CacheEntry> {
        self.evaluate_strategy();

        match self.active_strategy {
            CacheStrategy::LRU => self.lru_cache.get(block),
            CacheStrategy::LFU => self.lfu_cache.get(block),
        }
    }

    /// Insert block into cache
    pub fn insert(&mut self, block: BlockNumber, data: Vec<u8>) -> VexfsResult<()> {
        match self.active_strategy {
            CacheStrategy::LRU => self.lru_cache.insert(block, data),
            CacheStrategy::LFU => self.lfu_cache.insert(block, data),
        }
    }

    /// Remove block from cache
    pub fn remove(&mut self, block: BlockNumber) -> Option<CacheEntry> {
        // Try both caches
        if let Some(entry) = self.lru_cache.remove(block) {
            Some(entry)
        } else {
            self.lfu_cache.remove(block)
        }
    }

    /// Flush dirty entries from both caches
    pub fn flush_dirty(&mut self) -> VexfsResult<Vec<(BlockNumber, Vec<u8>)>> {
        let mut all_dirty = self.lru_cache.flush_dirty()?;
        let mut lfu_dirty = self.lfu_cache.flush_dirty()?;
        all_dirty.append(&mut lfu_dirty);
        Ok(all_dirty)
    }

    /// Get combined cache statistics
    pub fn get_stats(&self) -> CacheStats {
        let lru_stats = self.lru_cache.get_stats();
        let lfu_stats = self.lfu_cache.get_stats();

        let total_hits = lru_stats.hit_count + lfu_stats.hit_count;
        let total_misses = lru_stats.miss_count + lfu_stats.miss_count;
        let total_requests = total_hits + total_misses;
        
        let combined_hit_rate = if total_requests > 0 {
            (total_hits as f32 / total_requests as f32) * 100.0
        } else {
            0.0
        };

        CacheStats {
            hit_count: total_hits,
            miss_count: total_misses,
            hit_rate: combined_hit_rate,
            entry_count: lru_stats.entry_count + lfu_stats.entry_count,
            max_entries: lru_stats.max_entries + lfu_stats.max_entries,
            current_size: lru_stats.current_size + lfu_stats.current_size,
            max_size: lru_stats.max_size + lfu_stats.max_size,
            dirty_count: lru_stats.dirty_count + lfu_stats.dirty_count,
            utilization: ((lru_stats.current_size + lfu_stats.current_size) as f32 / 
                         (lru_stats.max_size + lfu_stats.max_size) as f32) * 100.0,
        }
    }

    /// Get current active strategy
    pub fn get_active_strategy(&self) -> CacheStrategy {
        self.active_strategy
    }

    /// Clear both caches
    pub fn clear(&mut self) {
        self.lru_cache.clear();
        self.lfu_cache.clear();
    }

    // Private methods

    fn evaluate_strategy(&mut self) {
        let now = current_time();
        
        if now.saturating_sub(self.last_evaluation) >= self.monitor_window {
            let lru_stats = self.lru_cache.get_stats();
            let lfu_stats = self.lfu_cache.get_stats();
            
            self.lru_performance = lru_stats.hit_rate;
            self.lfu_performance = lfu_stats.hit_rate;
            
            // Switch to better performing strategy
            if self.lfu_performance > self.lru_performance + 5.0 {
                self.active_strategy = CacheStrategy::LFU;
            } else if self.lru_performance > self.lfu_performance + 5.0 {
                self.active_strategy = CacheStrategy::LRU;
            }
            
            self.last_evaluation = now;
        }
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub hit_count: u64,
    pub miss_count: u64,
    pub hit_rate: f32,
    pub entry_count: usize,
    pub max_entries: usize,
    pub current_size: usize,
    pub max_size: usize,
    pub dirty_count: usize,
    pub utilization: f32,
}

impl CacheStats {
    /// Get cache efficiency score (0-100)
    pub fn get_efficiency_score(&self) -> f32 {
        // Combine hit rate and utilization for overall efficiency
        (self.hit_rate * 0.7) + (self.utilization * 0.3)
    }

    /// Check if cache needs tuning
    pub fn needs_tuning(&self) -> bool {
        self.hit_rate < 80.0 || self.utilization > 95.0
    }
}

/// Block cache manager with configurable strategy
pub struct BlockCacheManager {
    /// Active cache implementation
    cache: AdaptiveCache,
    /// Block size
    block_size: u32,
    /// Write-through mode enabled
    write_through: bool,
    /// Sync interval for dirty blocks
    sync_interval: u64,
    /// Last sync time
    last_sync: u64,
}

impl BlockCacheManager {
    /// Create new block cache manager
    pub fn new(
        max_entries: usize,
        max_size: usize,
        block_size: u32,
        write_through: bool,
    ) -> Self {
        Self {
            cache: AdaptiveCache::new(max_entries, max_size, block_size),
            block_size,
            write_through,
            sync_interval: 30, // 30 seconds default
            last_sync: current_time(),
        }
    }

    /// Read block through cache
    pub fn read_block(&mut self, block: BlockNumber) -> Option<Vec<u8>> {
        if let Some(entry) = self.cache.get(block) {
            Some(entry.data.clone())
        } else {
            None
        }
    }

    /// Write block through cache
    pub fn write_block(&mut self, block: BlockNumber, data: Vec<u8>) -> VexfsResult<()> {
        if self.write_through {
            // In write-through mode, mark as clean since it's immediately written
            self.cache.insert(block, data)?;
            if let Some(entry) = self.cache.get(block) {
                entry.mark_clean();
            }
        } else {
            // In write-back mode, mark as dirty
            self.cache.insert(block, data)?;
            if let Some(entry) = self.cache.get(block) {
                entry.mark_dirty();
            }
        }
        Ok(())
    }

    /// Sync dirty blocks to storage
    pub fn sync(&mut self) -> VexfsResult<Vec<(BlockNumber, Vec<u8>)>> {
        let now = current_time();
        
        if now.saturating_sub(self.last_sync) >= self.sync_interval || self.write_through {
            let dirty_blocks = self.cache.flush_dirty()?;
            self.last_sync = now;
            Ok(dirty_blocks)
        } else {
            Ok(Vec::new())
        }
    }

    /// Invalidate block in cache
    pub fn invalidate(&mut self, block: BlockNumber) {
        self.cache.remove(block);
    }

    /// Get cache statistics
    pub fn get_stats(&self) -> CacheStats {
        self.cache.get_stats()
    }

    /// Set sync interval
    pub fn set_sync_interval(&mut self, interval: u64) {
        self.sync_interval = interval;
    }

    /// Check if cache needs maintenance
    pub fn needs_maintenance(&self) -> bool {
        let stats = self.get_stats();
        stats.needs_tuning() || stats.dirty_count > (stats.max_entries / 4)
    }
}