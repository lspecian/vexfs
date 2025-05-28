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

//! Space Allocation Module
//!
//! This module implements block allocation, deallocation, and free space management
//! algorithms for VexFS, including bitmap-based allocation and extent-based tracking.

use crate::shared::errors::{VexfsError, VexfsResult};
use crate::shared::types::*;
use crate::shared::constants::*;
use crate::shared::utils::*;

/// Block allocation result
#[derive(Debug, Clone, Copy)]
pub struct AllocationResult {
    /// Starting block number
    pub start_block: BlockNumber,
    /// Number of blocks allocated
    pub block_count: u32,
    /// Allocation flags
    pub flags: u32,
}

impl AllocationResult {
    /// Create new allocation result
    pub fn new(start_block: BlockNumber, block_count: u32) -> Self {
        Self {
            start_block,
            block_count,
            flags: 0,
        }
    }

    /// Get end block (exclusive)
    pub fn end_block(&self) -> BlockNumber {
        self.start_block + self.block_count as u64
    }

    /// Check if this allocation contains the given block
    pub fn contains_block(&self, block: BlockNumber) -> bool {
        block >= self.start_block && block < self.end_block()
    }
}

/// Free space information
#[derive(Debug, Clone, Copy)]
pub struct FreeSpaceInfo {
    /// Total blocks in filesystem
    pub total_blocks: u64,
    /// Free blocks available
    pub free_blocks: u64,
    /// Reserved blocks (for root)
    pub reserved_blocks: u64,
    /// Largest contiguous free extent
    pub largest_free_extent: u32,
    /// Number of free extents
    pub free_extents: u32,
    /// Fragmentation percentage (0-100)
    pub fragmentation: u8,
}

/// Block group descriptor for group-based allocation
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct BlockGroup {
    /// Block bitmap location
    pub block_bitmap: BlockNumber,
    /// Inode bitmap location
    pub inode_bitmap: BlockNumber,
    /// Inode table location
    pub inode_table: BlockNumber,
    /// Free blocks count
    pub free_blocks_count: u32,
    /// Free inodes count
    pub free_inodes_count: u32,
    /// Used directories count
    pub used_dirs_count: u32,
    /// Flags for this block group
    pub flags: u16,
    /// Padding
    pub reserved: [u16; 7],
    /// Checksum
    pub checksum: u32,
}

impl BlockGroup {
    /// Create new block group
    pub fn new() -> Self {
        Self {
            block_bitmap: 0,
            inode_bitmap: 0,
            inode_table: 0,
            free_blocks_count: 0,
            free_inodes_count: 0,
            used_dirs_count: 0,
            flags: 0,
            reserved: [0; 7],
            checksum: 0,
        }
    }

    /// Initialize block group with layout information
    pub fn initialize(&mut self, group_idx: u32, layout: &crate::storage::layout::VexfsLayout) -> VexfsResult<()> {
        let group_start = group_idx as u64 * layout.blocks_per_group as u64;
        
        self.block_bitmap = group_start + 1; // After superblock
        self.inode_bitmap = self.block_bitmap + 1;
        self.inode_table = self.inode_bitmap + 1;
        self.free_blocks_count = layout.blocks_per_group;
        self.free_inodes_count = layout.inodes_per_group;
        self.used_dirs_count = 0;
        self.flags = 0;
        
        self.update_checksum();
        Ok(())
    }

    /// Update checksum
    pub fn update_checksum(&mut self) {
        let data = unsafe {
            core::slice::from_raw_parts(
                self as *const Self as *const u8,
                core::mem::size_of::<Self>() - core::mem::size_of::<u32>() // Exclude checksum field
            )
        };
        self.checksum = crc32(data);
    }

    /// Verify checksum
    pub fn verify_checksum(&self) -> bool {
        let data = unsafe {
            core::slice::from_raw_parts(
                self as *const Self as *const u8,
                core::mem::size_of::<Self>() - core::mem::size_of::<u32>()
            )
        };
        verify_checksum(data, self.checksum)
    }
}

/// Allocation strategy enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AllocationStrategy {
    /// First fit allocation
    FirstFit,
    /// Best fit allocation
    BestFit,
    /// Buddy allocation
    Buddy,
    /// Extent-based allocation
    Extent,
}

/// Allocation policy for different scenarios
#[derive(Debug, Clone, Copy)]
pub struct AllocationPolicy {
    /// Strategy to use
    pub strategy: AllocationStrategy,
    /// Prefer contiguous allocations
    pub prefer_contiguous: bool,
    /// Maximum fragmentation tolerance (0-100)
    pub max_fragmentation: u8,
    /// Minimum extent size for extent-based allocation
    pub min_extent_size: u32,
}

impl Default for AllocationPolicy {
    fn default() -> Self {
        Self {
            strategy: AllocationStrategy::FirstFit,
            prefer_contiguous: true,
            max_fragmentation: 20,
            min_extent_size: 8,
        }
    }
}

/// Allocation hint for better placement
#[derive(Debug, Clone, Copy)]
pub struct AllocationHint {
    /// Preferred starting block
    pub preferred_start: BlockNumber,
    /// Goal block (for locality)
    pub goal_block: BlockNumber,
    /// Allocation flags
    pub flags: u32,
    /// Minimum contiguous blocks needed
    pub min_contiguous: u32,
    /// Maximum search distance
    pub max_search_distance: u32,
}

impl Default for AllocationHint {
    fn default() -> Self {
        Self {
            preferred_start: 0,
            goal_block: 0,
            flags: 0,
            min_contiguous: 1,
            max_search_distance: 1000,
        }
    }
}

/// Free space bitmap for tracking allocated blocks
pub struct FreeSpaceBitmap {
    /// Bitmap data
    bitmap: Vec<u64>,
    /// Number of bits (blocks) represented
    total_bits: u64,
    /// Block size each bit represents
    block_size: u32,
    /// Cached free block count
    free_count: u64,
    /// Dirty flag
    dirty: bool,
}

impl FreeSpaceBitmap {
    /// Create new bitmap
    pub fn new(total_blocks: u64, block_size: u32) -> VexfsResult<Self> {
        let bitmap_size = (total_blocks + 63) / 64; // Round up to u64 boundaries
        let bitmap = vec![0u64; bitmap_size as usize];
        
        Ok(Self {
            bitmap,
            total_bits: total_blocks,
            block_size,
            free_count: total_blocks,
            dirty: false,
        })
    }

    /// Check if block is allocated
    pub fn is_allocated(&self, block: BlockNumber) -> VexfsResult<bool> {
        if block >= self.total_bits {
            return Err(VexfsError::InvalidArgument("block number out of range".to_string()));
        }
        
        let word_idx = (block / 64) as usize;
        let bit_idx = block % 64;
        
        Ok((self.bitmap[word_idx] & (1u64 << bit_idx)) != 0)
    }

    /// Set block as allocated
    pub fn set_allocated(&mut self, block: BlockNumber) -> VexfsResult<()> {
        if block >= self.total_bits {
            return Err(VexfsError::InvalidArgument("block number out of range".to_string()));
        }
        
        let word_idx = (block / 64) as usize;
        let bit_idx = block % 64;
        let mask = 1u64 << bit_idx;
        
        if (self.bitmap[word_idx] & mask) == 0 {
            self.bitmap[word_idx] |= mask;
            self.free_count -= 1;
            self.dirty = true;
        }
        
        Ok(())
    }

    /// Set block as free
    pub fn set_free(&mut self, block: BlockNumber) -> VexfsResult<()> {
        if block >= self.total_bits {
            return Err(VexfsError::InvalidArgument("block number out of range".to_string()));
        }
        
        let word_idx = (block / 64) as usize;
        let bit_idx = block % 64;
        let mask = 1u64 << bit_idx;
        
        if (self.bitmap[word_idx] & mask) != 0 {
            self.bitmap[word_idx] &= !mask;
            self.free_count += 1;
            self.dirty = true;
        }
        
        Ok(())
    }

    /// Find first free block
    pub fn find_first_free(&self) -> Option<BlockNumber> {
        for (word_idx, &word) in self.bitmap.iter().enumerate() {
            if word != u64::MAX {
                let bit_idx = word.trailing_ones();
                if bit_idx < 64 {
                    let block = word_idx as u64 * 64 + bit_idx as u64;
                    if block < self.total_bits {
                        return Some(block);
                    }
                }
            }
        }
        None
    }

    /// Find next free block after given block
    pub fn find_next_free(&self, after: BlockNumber) -> Option<BlockNumber> {
        let start_word = ((after + 1) / 64) as usize;
        let start_bit = (after + 1) % 64;
        
        // Check remaining bits in the starting word
        if start_word < self.bitmap.len() {
            let word = self.bitmap[start_word];
            let mask = !((1u64 << start_bit) - 1); // Mask out bits before start_bit
            let masked_word = word | !mask;
            
            if masked_word != u64::MAX {
                let bit_idx = masked_word.trailing_ones();
                if bit_idx < 64 {
                    let block = start_word as u64 * 64 + bit_idx as u64;
                    if block < self.total_bits {
                        return Some(block);
                    }
                }
            }
        }
        
        // Check subsequent words
        for word_idx in (start_word + 1)..self.bitmap.len() {
            let word = self.bitmap[word_idx];
            if word != u64::MAX {
                let bit_idx = word.trailing_ones();
                if bit_idx < 64 {
                    let block = word_idx as u64 * 64 + bit_idx as u64;
                    if block < self.total_bits {
                        return Some(block);
                    }
                }
            }
        }
        
        None
    }

    /// Get free block count
    pub fn free_count(&self) -> u64 {
        self.free_count
    }

    /// Check if bitmap is dirty
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Mark bitmap as clean
    pub fn mark_clean(&mut self) {
        self.dirty = false;
    }

    /// Recalculate free count (for verification)
    pub fn recalculate_free_count(&mut self) {
        let mut count = 0;
        for &word in &self.bitmap {
            count += word.count_zeros() as u64;
        }
        
        // Adjust for any padding bits beyond total_bits
        let _total_words = self.bitmap.len() as u64;
        let used_bits_in_last_word = self.total_bits % 64;
        if used_bits_in_last_word > 0 {
            let padding_bits = 64 - used_bits_in_last_word;
            count -= padding_bits;
        }
        
        self.free_count = count;
    }
}

/// Fragmentation tracker for analyzing allocation patterns
pub struct FragmentationTracker {
    /// Total fragments
    fragment_count: u32,
    /// Average fragment size
    avg_fragment_size: f32,
    /// Largest fragment size
    largest_fragment: u32,
    /// Total free space
    total_free_space: u64,
}

impl FragmentationTracker {
    /// Create new fragmentation tracker
    pub fn new() -> Self {
        Self {
            fragment_count: 0,
            avg_fragment_size: 0.0,
            largest_fragment: 0,
            total_free_space: 0,
        }
    }

    /// Update fragmentation statistics
    pub fn update(&mut self, bitmap: &FreeSpaceBitmap) {
        let mut fragments = Vec::new();
        let mut current_fragment_start = None;
        let mut current_fragment_size = 0u32;
        
        // Scan bitmap to find free fragments
        for block in 0..bitmap.total_bits {
            match bitmap.is_allocated(block) {
                Ok(false) => {
                    // Free block
                    if current_fragment_start.is_none() {
                        current_fragment_start = Some(block);
                        current_fragment_size = 1;
                    } else {
                        current_fragment_size += 1;
                    }
                }
                Ok(true) => {
                    // Allocated block - end current fragment if any
                    if let Some(_start) = current_fragment_start {
                        fragments.push(current_fragment_size);
                        current_fragment_start = None;
                        current_fragment_size = 0;
                    }
                }
                Err(_) => break, // Error accessing bitmap
            }
        }
        
        // Handle final fragment
        if current_fragment_start.is_some() && current_fragment_size > 0 {
            fragments.push(current_fragment_size);
        }
        
        // Update statistics
        self.fragment_count = fragments.len() as u32;
        self.largest_fragment = fragments.iter().max().copied().unwrap_or(0);
        self.total_free_space = bitmap.free_count();
        
        if !fragments.is_empty() {
            let total_fragment_size: u32 = fragments.iter().sum();
            self.avg_fragment_size = total_fragment_size as f32 / fragments.len() as f32;
        } else {
            self.avg_fragment_size = 0.0;
        }
    }

    /// Get fragmentation percentage (0-100)
    pub fn fragmentation_percentage(&self) -> u8 {
        if self.total_free_space == 0 || self.fragment_count <= 1 {
            return 0;
        }
        
        // Calculate fragmentation as deviation from ideal (single fragment)
        let ideal_fragments = 1;
        let excess_fragments = self.fragment_count.saturating_sub(ideal_fragments);
        let fragmentation = (excess_fragments * 100) / max(1, self.fragment_count);
        
        min(100, fragmentation) as u8
    }

    /// Get statistics
    pub fn stats(&self) -> FragmentationStats {
        FragmentationStats {
            fragment_count: self.fragment_count,
            avg_fragment_size: self.avg_fragment_size,
            largest_fragment: self.largest_fragment,
            total_free_space: self.total_free_space,
            fragmentation_percentage: self.fragmentation_percentage(),
        }
    }
}

/// Fragmentation statistics
#[derive(Debug, Clone)]
pub struct FragmentationStats {
    pub fragment_count: u32,
    pub avg_fragment_size: f32,
    pub largest_fragment: u32,
    pub total_free_space: u64,
    pub fragmentation_percentage: u8,
}

/// Space allocator for managing block allocation
pub struct SpaceAllocator {
    /// Free space bitmap
    bitmap: FreeSpaceBitmap,
    /// Block groups
    block_groups: Vec<BlockGroup>,
    /// Allocation policy
    policy: AllocationPolicy,
    /// Fragmentation tracker
    fragmentation: FragmentationTracker,
    /// Total blocks
    total_blocks: u64,
    /// Blocks per group
    blocks_per_group: u32,
}

impl SpaceAllocator {
    /// Create new space allocator
    pub fn new(total_blocks: u64, block_size: u32, blocks_per_group: u32) -> VexfsResult<Self> {
        let bitmap = FreeSpaceBitmap::new(total_blocks, block_size)?;
        let group_count = (total_blocks + blocks_per_group as u64 - 1) / blocks_per_group as u64;
        let block_groups = vec![BlockGroup::new(); group_count as usize];
        
        Ok(Self {
            bitmap,
            block_groups,
            policy: AllocationPolicy::default(),
            fragmentation: FragmentationTracker::new(),
            total_blocks,
            blocks_per_group,
        })
    }

    /// Initialize allocator with layout information
    pub fn initialize(&mut self) -> VexfsResult<()> {
        // Initialize block groups with layout information
        let group_count = self.block_groups.len() as u32;
        for (group_idx, group) in self.block_groups.iter_mut().enumerate() {
            let layout = crate::storage::layout::VexfsLayout {
                block_size: self.bitmap.block_size,
                total_blocks: self.total_blocks,
                blocks_per_group: self.blocks_per_group,
                group_count,
                inodes_per_group: self.blocks_per_group, // Default assumption
                inode_size: 256, // Default inode size
                journal_blocks: 0, // No journal for allocator init
                vector_blocks: 0, // No vector blocks for allocator init
            };
            group.initialize(group_idx as u32, &layout)?;
        }
        Ok(())
    }

    /// Load allocation state from storage
    pub fn load_state(&mut self) -> VexfsResult<()> {
        // TODO: Load bitmap and block group state from disk
        // For now, assume clean state
        self.bitmap.mark_clean();
        Ok(())
    }

    /// Allocate blocks
    pub fn allocate_blocks(&mut self, count: u32, hint: Option<AllocationHint>) -> VexfsResult<AllocationResult> {
        self.allocate(count, hint)
    }

    /// Sync allocation state to storage
    pub fn sync(&mut self) -> VexfsResult<()> {
        // TODO: Write bitmap and block group state to disk
        if self.bitmap.is_dirty() {
            self.bitmap.mark_clean();
        }
        Ok(())
    }

    /// Allocate blocks
    pub fn allocate(&mut self, count: u32, hint: Option<AllocationHint>) -> VexfsResult<AllocationResult> {
        if count == 0 {
            return Err(VexfsError::InvalidArgument("cannot allocate zero blocks".to_string()));
        }
        
        if self.bitmap.free_count() < count as u64 {
            return Err(VexfsError::NoSpace);
        }
        
        let result = match self.policy.strategy {
            AllocationStrategy::FirstFit => self.allocate_first_fit(count, hint),
            AllocationStrategy::BestFit => self.allocate_best_fit(count, hint),
            AllocationStrategy::Buddy => self.allocate_buddy(count, hint),
            AllocationStrategy::Extent => self.allocate_extent(count, hint),
        }?;
        
        // Mark blocks as allocated in bitmap
        for block in result.start_block..result.end_block() {
            self.bitmap.set_allocated(block)?;
        }
        
        // Update fragmentation statistics
        self.fragmentation.update(&self.bitmap);
        
        Ok(result)
    }

    /// Free blocks
    pub fn free(&mut self, start_block: BlockNumber, count: u32) -> VexfsResult<()> {
        if count == 0 {
            return Ok(());
        }
        
        if start_block + count as u64 > self.total_blocks {
            return Err(VexfsError::InvalidArgument("block range exceeds filesystem size".to_string()));
        }
        
        // Mark blocks as free in bitmap
        for block in start_block..(start_block + count as u64) {
            self.bitmap.set_free(block)?;
        }
        
        // Update fragmentation statistics
        self.fragmentation.update(&self.bitmap);
        
        Ok(())
    }

    /// Check if block is allocated
    pub fn is_allocated(&self, block: BlockNumber) -> VexfsResult<bool> {
        self.bitmap.is_allocated(block)
    }

    /// Get free space information
    pub fn free_space_info(&self) -> FreeSpaceInfo {
        let stats = self.fragmentation.stats();
        
        FreeSpaceInfo {
            total_blocks: self.total_blocks,
            free_blocks: self.bitmap.free_count(),
            reserved_blocks: 0, // TODO: Track reserved blocks
            largest_free_extent: stats.largest_fragment,
            free_extents: stats.fragment_count,
            fragmentation: stats.fragmentation_percentage,
        }
    }

    /// Set allocation policy
    pub fn set_policy(&mut self, policy: AllocationPolicy) {
        self.policy = policy;
    }

    /// Get current policy
    pub fn policy(&self) -> AllocationPolicy {
        self.policy
    }

    // Private allocation algorithm implementations
    fn allocate_first_fit(&self, count: u32, hint: Option<AllocationHint>) -> VexfsResult<AllocationResult> {
        let start = hint.map(|h| h.preferred_start).unwrap_or(0);
        
        if let Some(block) = self.find_contiguous_blocks(start, count as u64) {
            Ok(AllocationResult::new(block, count))
        } else {
            Err(VexfsError::NoSpace)
        }
    }

    fn allocate_best_fit(&self, count: u32, _hint: Option<AllocationHint>) -> VexfsResult<AllocationResult> {
        // For now, fallback to first fit
        // A proper implementation would find the smallest suitable free extent
        self.allocate_first_fit(count, None)
    }

    fn allocate_buddy(&self, count: u32, hint: Option<AllocationHint>) -> VexfsResult<AllocationResult> {
        let buddy_size = count.next_power_of_two();
        self.allocate_first_fit(buddy_size, hint)
    }

    fn allocate_extent(&self, count: u32, hint: Option<AllocationHint>) -> VexfsResult<AllocationResult> {
        // Try to find an extent that closely matches the request
        self.allocate_first_fit(count, hint)
    }

    fn find_contiguous_blocks(&self, start: BlockNumber, count: u64) -> Option<BlockNumber> {
        let mut current = start;
        
        while current < self.total_blocks {
            if let Some(free_block) = if current == 0 {
                self.bitmap.find_first_free()
            } else {
                self.bitmap.find_next_free(current - 1)
            } {
                if self.check_contiguous_free(free_block, count) {
                    return Some(free_block);
                }
                current = free_block + 1;
            } else {
                break;
            }
        }
        
        None
    }

    fn check_contiguous_free(&self, start: BlockNumber, count: u64) -> bool {
        for offset in 0..count {
            if let Ok(allocated) = self.bitmap.is_allocated(start + offset) {
                if allocated {
                    return false;
                }
            } else {
                return false;
            }
        }
        true
    }
}

/// Block allocator that combines space allocation with block management
pub struct BlockAllocator {
    /// Space allocator
    space_allocator: SpaceAllocator,
    /// Layout information
    layout: crate::storage::layout::VexfsLayout,
}

impl BlockAllocator {
    /// Create new block allocator
    pub fn new(layout: &crate::storage::layout::VexfsLayout) -> VexfsResult<Self> {
        let space_allocator = SpaceAllocator::new(
            layout.total_blocks,
            layout.block_size,
            layout.blocks_per_group,
        )?;
        
        Ok(Self {
            space_allocator,
            layout: layout.clone(),
        })
    }

    /// Initialize allocator with superblock
    pub fn initialize(&mut self, _superblock: &crate::storage::superblock::SuperblockManager) -> VexfsResult<()> {
        // Initialize block groups and load existing allocation state
        // This would typically read allocation bitmaps from disk
        Ok(())
    }

    /// Allocate blocks
    pub fn allocate_blocks(&mut self, count: u32, hint: Option<AllocationHint>) -> VexfsResult<AllocationResult> {
        self.space_allocator.allocate(count, hint)
    }

    /// Free blocks
    pub fn free_blocks(&mut self, start_block: BlockNumber, count: u32) -> VexfsResult<()> {
        self.space_allocator.free(start_block, count)
    }

    /// Get free blocks count
    pub fn get_free_blocks_count(&self) -> u64 {
        self.space_allocator.free_space_info().free_blocks
    }

    /// Get used blocks count
    pub fn used_blocks(&self) -> u64 {
        let info = self.space_allocator.free_space_info();
        info.total_blocks - info.free_blocks
    }

    /// Get free space information
    pub fn free_space_info(&self) -> FreeSpaceInfo {
        self.space_allocator.free_space_info()
    }
}