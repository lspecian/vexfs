//! Space Allocation System for VexFS
//! 
//! This module implements block allocation, deallocation, and free space management
//! algorithms for VexFS, including bitmap-based allocation and extent-based tracking.



use crate::ondisk::*;
use core::mem;

/// Space allocation error types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SpaceAllocError {
    NoSpace,
    InvalidBlock,
    AlreadyAllocated,
    NotAllocated,
    CorruptedBitmap,
    IoError,
    InvalidSize,
    FragmentationLimit,
}

/// Block allocation result
#[derive(Debug, Clone, Copy)]
pub struct AllocResult {
    /// Starting block number
    pub start_block: u64,
    
    /// Number of blocks allocated
    pub block_count: u32,
    
    /// Allocation flags
    pub flags: u32,
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

/// Handle for managing allocated blocks
#[derive(Debug, Clone, Copy)]
pub struct BlockHandle {
    /// Starting block number
    pub start_block: u64,
    
    /// Number of blocks
    pub block_count: u32,
    
    /// Block group this allocation belongs to
    pub block_group: u32,
    
    /// Allocation flags
    pub flags: u32,
}

impl BlockHandle {
    /// Create a new block handle
    pub fn new(start_block: u64, block_count: u32, block_group: u32) -> Self {
        Self {
            start_block,
            block_count,
            block_group,
            flags: 0,
        }
    }
    
    /// Get the end block (exclusive)
    pub fn end_block(&self) -> u64 {
        self.start_block + self.block_count as u64
    }
    
    /// Check if this handle contains the given block
    pub fn contains_block(&self, block: u64) -> bool {
        block >= self.start_block && block < self.end_block()
    }
}

/// Block group descriptor for group-based allocation
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct VexfsBlockGroup {
    /// Block bitmap location
    pub block_bitmap: u64,
    
    /// Inode bitmap location
    pub inode_bitmap: u64,
    
    /// Inode table location
    pub inode_table: u64,
    
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

/// Space allocator for VexFS
pub struct VexfsSpaceAllocator {
    /// Superblock reference
    pub superblock: VexfsSuperblock,
    
    /// Block groups
    pub block_groups: [VexfsBlockGroup; VEXFS_MAX_BLOCK_GROUPS],
    
    /// Number of block groups
    pub block_groups_count: u32,
    
    /// Blocks per group
    pub blocks_per_group: u32,
    
    /// Block size
    pub block_size: u32,
    
    /// Allocation policy
    pub alloc_policy: AllocPolicy,
    
    /// Free space cache
    pub free_space_cache: FreeSpaceCache,
}

/// Allocation policy
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AllocPolicy {
    /// First fit allocation
    FirstFit,
    
    /// Best fit allocation
    BestFit,
    
    /// Buddy allocation
    Buddy,
    
    /// Extent-based allocation
    Extent,
}

/// Free space cache for fast allocation
#[derive(Debug)]
pub struct FreeSpaceCache {
    /// Cached free block count per group
    pub free_blocks_per_group: [u32; VEXFS_MAX_BLOCK_GROUPS],
    
    /// Last allocation group
    pub last_alloc_group: u32,
    
    /// Dirty flags for groups
    pub dirty_groups: u64, // Bitmap for dirty groups
    
    /// Cache validity
    pub valid: bool,
}

/// Allocation hint for better placement
#[derive(Debug, Clone, Copy)]
pub struct AllocHint {
    /// Preferred starting block
    pub preferred_start: u64,
    
    /// Goal block (for locality)
    pub goal_block: u64,
    
    /// Allocation flags
    pub flags: u32,
    
    /// Minimum contiguous blocks needed
    pub min_contiguous: u32,
    
    /// Maximum search distance
    pub max_search_distance: u32,
}

/// Extent for free space tracking
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FreeExtent {
    /// Starting block
    pub start: u64,
    
    /// Length in blocks
    pub length: u32,
    
    /// Next extent in list
    pub next: Option<u64>,
}

impl VexfsSpaceAllocator {
    /// Create a new space allocator
    pub fn new(superblock: VexfsSuperblock, block_size: u32) -> Self {
        let blocks_per_group = if superblock.s_blocks_per_group > 0 {
            superblock.s_blocks_per_group
        } else {
            VEXFS_DEFAULT_BLOCKS_PER_GROUP
        };
        
        let block_groups_count = ((superblock.s_blocks_count + blocks_per_group as u64 - 1) / blocks_per_group as u64) as u32;
        
        Self {
            superblock,
            block_groups: [VexfsBlockGroup::new(); VEXFS_MAX_BLOCK_GROUPS],
            block_groups_count,
            blocks_per_group,
            block_size,
            alloc_policy: AllocPolicy::FirstFit,
            free_space_cache: FreeSpaceCache::new(),
        }
    }
    
    /// Initialize the allocator with block group information
    pub fn init(&mut self) -> Result<(), SpaceAllocError> {
        // Initialize block groups
        for i in 0..self.block_groups_count {
            self.init_block_group(i)?;
        }
        
        // Initialize free space cache
        self.update_free_space_cache()?;
        
        Ok(())
    }
    
    /// Allocate blocks
    pub fn allocate_blocks(&mut self, count: u32, hint: Option<AllocHint>) -> Result<AllocResult, SpaceAllocError> {
        if count == 0 {
            return Err(SpaceAllocError::InvalidSize);
        }
        
        // Check if we have enough free blocks
        if self.superblock.s_free_blocks_count < count as u64 {
            return Err(SpaceAllocError::NoSpace);
        }
        
        let result = match self.alloc_policy {
            AllocPolicy::FirstFit => self.allocate_first_fit(count, hint)?,
            AllocPolicy::BestFit => self.allocate_best_fit(count, hint)?,
            AllocPolicy::Buddy => self.allocate_buddy(count, hint)?,
            AllocPolicy::Extent => self.allocate_extent(count, hint)?,
        };
        
        // Update free block count
        self.superblock.s_free_blocks_count -= result.block_count as u64;
        
        // Update free space cache
        self.invalidate_cache_for_range(result.start_block, result.block_count);
        
        Ok(result)
    }
    
    /// Free blocks
    pub fn free_blocks(&mut self, start_block: u64, count: u32) -> Result<(), SpaceAllocError> {
        if count == 0 {
            return Ok(());
        }
        
        // Validate block range
        if start_block + count as u64 > self.superblock.s_blocks_count {
            return Err(SpaceAllocError::InvalidBlock);
        }
        
        // Free blocks in the bitmap
        self.free_blocks_in_bitmap(start_block, count)?;
        
        // Update free block count
        self.superblock.s_free_blocks_count += count as u64;
        
        // Update free space cache
        self.invalidate_cache_for_range(start_block, count);
        
        // Try to coalesce with adjacent free blocks
        self.coalesce_free_blocks(start_block, count)?;
        
        Ok(())
    }
    
    /// Check if a block is allocated
    pub fn is_block_allocated(&self, block_num: u64) -> Result<bool, SpaceAllocError> {
        if block_num >= self.superblock.s_blocks_count {
            return Err(SpaceAllocError::InvalidBlock);
        }
        
        let group_num = self.block_to_group(block_num);
        let block_in_group = self.block_in_group(block_num);
        
        // In a real implementation, we would read the bitmap from disk
        // For now, assume all blocks are free
        Ok(false)
    }
    
    /// Get free space information
    pub fn get_free_space_info(&self) -> Result<FreeSpaceInfo, SpaceAllocError> {
        let mut largest_free_extent = 0u32;
        let mut free_extents = 0u32;
        let mut total_free_blocks = 0u64;
        
        // Calculate statistics from all block groups
        for i in 0..self.block_groups_count {
            total_free_blocks += self.block_groups[i as usize].free_blocks_count as u64;
            
            // In a real implementation, we would scan the bitmap to find extents
            // For now, use approximate values
            free_extents += 1;
            largest_free_extent = core::cmp::max(
                largest_free_extent,
                self.block_groups[i as usize].free_blocks_count
            );
        }
        
        // Calculate fragmentation (simplified)
        let total_blocks = self.superblock.s_blocks_count;
        let fragmentation = if total_blocks > 0 {
            let ideal_extents = if total_free_blocks > 0 { 1 } else { 0 };
            let extra_extents = if free_extents > ideal_extents {
                free_extents - ideal_extents
            } else {
                0
            };
            core::cmp::min(100, (extra_extents * 100 / core::cmp::max(1, free_extents)) as u8)
        } else {
            0
        };
        
        Ok(FreeSpaceInfo {
            total_blocks,
            free_blocks: total_free_blocks,
            reserved_blocks: self.superblock.s_r_blocks_count,
            largest_free_extent,
            free_extents,
            fragmentation,
        })
    }
    
    /// Reserve blocks for critical operations
    pub fn reserve_blocks(&mut self, count: u32) -> Result<(), SpaceAllocError> {
        if self.superblock.s_free_blocks_count < count as u64 {
            return Err(SpaceAllocError::NoSpace);
        }
        
        self.superblock.s_r_blocks_count += count as u64;
        Ok(())
    }
    
    /// Release reserved blocks
    pub fn release_reserved_blocks(&mut self, count: u32) -> Result<(), SpaceAllocError> {
        let to_release = core::cmp::min(count as u64, self.superblock.s_r_blocks_count);
        self.superblock.s_r_blocks_count -= to_release;
        Ok(())
    }
    
    /// Set allocation policy
    pub fn set_alloc_policy(&mut self, policy: AllocPolicy) {
        self.alloc_policy = policy;
    }
    
    /// First fit allocation algorithm
    fn allocate_first_fit(&mut self, count: u32, hint: Option<AllocHint>) -> Result<AllocResult, SpaceAllocError> {
        let start_group = if let Some(h) = hint {
            self.block_to_group(h.preferred_start)
        } else {
            self.free_space_cache.last_alloc_group
        };
        
        // Search starting from the hint group
        for i in 0..self.block_groups_count {
            let group_idx = (start_group + i) % self.block_groups_count;
            
            if self.block_groups[group_idx as usize].free_blocks_count >= count {
                if let Ok(result) = self.allocate_in_group(group_idx, count, hint) {
                    self.free_space_cache.last_alloc_group = group_idx;
                    return Ok(result);
                }
            }
        }
        
        Err(SpaceAllocError::NoSpace)
    }
    
    /// Best fit allocation algorithm
    fn allocate_best_fit(&mut self, count: u32, hint: Option<AllocHint>) -> Result<AllocResult, SpaceAllocError> {
        let mut best_group = None;
        let mut best_fit_size = u32::MAX;
        
        // Find the group with the smallest suitable free space
        for i in 0..self.block_groups_count {
            let free_blocks = self.block_groups[i as usize].free_blocks_count;
            
            if free_blocks >= count && free_blocks < best_fit_size {
                best_fit_size = free_blocks;
                best_group = Some(i);
            }
        }
        
        if let Some(group_idx) = best_group {
            self.allocate_in_group(group_idx, count, hint)
        } else {
            Err(SpaceAllocError::NoSpace)
        }
    }
    
    /// Buddy allocation algorithm
    fn allocate_buddy(&mut self, count: u32, hint: Option<AllocHint>) -> Result<AllocResult, SpaceAllocError> {
        // Find the smallest power of 2 >= count
        let buddy_size = count.next_power_of_two();
        
        // Try to allocate using buddy system
        self.allocate_first_fit(buddy_size, hint)
    }
    
    /// Extent-based allocation algorithm
    fn allocate_extent(&mut self, count: u32, hint: Option<AllocHint>) -> Result<AllocResult, SpaceAllocError> {
        // For extent allocation, try to find a contiguous block
        // that exactly matches the request or is slightly larger
        let max_waste = count / 8; // Allow up to 12.5% waste
        
        for i in 0..self.block_groups_count {
            if let Ok(extent) = self.find_free_extent_in_group(i, count, count + max_waste) {
                return self.allocate_extent_in_group(i, extent.start, count);
            }
        }
        
        // Fallback to first fit if no suitable extent found
        self.allocate_first_fit(count, hint)
    }
    
    /// Allocate blocks within a specific group
    fn allocate_in_group(&mut self, group_idx: u32, count: u32, _hint: Option<AllocHint>) -> Result<AllocResult, SpaceAllocError> {
        let group = &mut self.block_groups[group_idx as usize];
        
        if group.free_blocks_count < count {
            return Err(SpaceAllocError::NoSpace);
        }
        
        // Calculate starting block for this group
        let group_start_block = group_idx as u64 * self.blocks_per_group as u64;
        
        // In a real implementation, we would:
        // 1. Read the block bitmap for this group
        // 2. Find free blocks using bitmap operations
        // 3. Mark the blocks as allocated
        // 4. Write the bitmap back to disk
        
        // For now, simulate allocation
        let allocated_start = group_start_block + 10; // Dummy offset
        
        // Update group statistics
        group.free_blocks_count -= count;
        
        Ok(AllocResult {
            start_block: allocated_start,
            block_count: count,
            flags: 0,
        })
    }
    
    /// Free blocks in bitmap
    fn free_blocks_in_bitmap(&mut self, start_block: u64, count: u32) -> Result<(), SpaceAllocError> {
        let mut current_block = start_block;
        let mut remaining = count;
        
        while remaining > 0 {
            let group_num = self.block_to_group(current_block);
            let block_in_group = self.block_in_group(current_block);
            let blocks_in_this_group = core::cmp::min(
                remaining,
                self.blocks_per_group - block_in_group as u32
            );
            
            // In a real implementation, we would:
            // 1. Read the block bitmap for this group
            // 2. Clear the bits for the blocks being freed
            // 3. Write the bitmap back to disk
            
            // Update group statistics
            self.block_groups[group_num as usize].free_blocks_count += blocks_in_this_group;
            
            current_block += blocks_in_this_group as u64;
            remaining -= blocks_in_this_group;
        }
        
        Ok(())
    }
    
    /// Coalesce adjacent free blocks
    fn coalesce_free_blocks(&mut self, start_block: u64, count: u32) -> Result<(), SpaceAllocError> {
        // In a real implementation, this would:
        // 1. Check if blocks before start_block are free
        // 2. Check if blocks after start_block + count are free
        // 3. Merge adjacent free extents in extent trees/lists
        // 4. Update free space tracking structures
        
        // For now, this is a no-op
        Ok(())
    }
    
    /// Find a free extent in a group
    fn find_free_extent_in_group(&self, group_idx: u32, min_size: u32, max_size: u32) -> Result<FreeExtent, SpaceAllocError> {
        // In a real implementation, this would scan the bitmap
        // For now, return a dummy extent
        let group_start_block = group_idx as u64 * self.blocks_per_group as u64;
        
        Ok(FreeExtent {
            start: group_start_block + 20, // Dummy offset
            length: min_size,
            next: None,
        })
    }
    
    /// Allocate a specific extent in a group
    fn allocate_extent_in_group(&mut self, group_idx: u32, start_block: u64, count: u32) -> Result<AllocResult, SpaceAllocError> {
        let group = &mut self.block_groups[group_idx as usize];
        
        if group.free_blocks_count < count {
            return Err(SpaceAllocError::NoSpace);
        }
        
        // In a real implementation, we would mark the specific blocks as allocated
        // For now, just update counters
        group.free_blocks_count -= count;
        
        Ok(AllocResult {
            start_block,
            block_count: count,
            flags: 0,
        })
    }
    
    /// Initialize a block group
    fn init_block_group(&mut self, group_idx: u32) -> Result<(), SpaceAllocError> {
        let group = &mut self.block_groups[group_idx as usize];
        
        // Calculate block group layout
        let group_start_block = group_idx as u64 * self.blocks_per_group as u64;
        let blocks_in_group = core::cmp::min(
            self.blocks_per_group as u64,
            self.superblock.s_blocks_count - group_start_block
        );
        
        // Set up block group descriptor
        group.block_bitmap = group_start_block + 1; // Bitmap after group descriptor
        group.inode_bitmap = group.block_bitmap + 1;
        group.inode_table = group.inode_bitmap + 1;
        group.free_blocks_count = blocks_in_group as u32 - 10; // Reserve some blocks
        group.free_inodes_count = self.superblock.s_inodes_per_group;
        group.used_dirs_count = 0;
        group.flags = 0;
        let checksum = self.calculate_group_checksum(group);
        group.checksum = checksum;
        
        Ok(())
    }
    
    /// Update free space cache
    fn update_free_space_cache(&mut self) -> Result<(), SpaceAllocError> {
        for i in 0..self.block_groups_count {
            self.free_space_cache.free_blocks_per_group[i as usize] = 
                self.block_groups[i as usize].free_blocks_count;
        }
        
        self.free_space_cache.valid = true;
        self.free_space_cache.dirty_groups = 0;
        Ok(())
    }
    
    /// Invalidate cache for a range of blocks
    fn invalidate_cache_for_range(&mut self, start_block: u64, count: u32) {
        let start_group = self.block_to_group(start_block);
        let end_group = self.block_to_group(start_block + count as u64 - 1);
        
        for group in start_group..=end_group {
            if group < VEXFS_MAX_BLOCK_GROUPS as u32 {
                self.free_space_cache.dirty_groups |= 1u64 << group;
            }
        }
    }
    
    /// Convert block number to group number
    fn block_to_group(&self, block_num: u64) -> u32 {
        (block_num / self.blocks_per_group as u64) as u32
    }
    
    /// Get block offset within group
    fn block_in_group(&self, block_num: u64) -> u32 {
        (block_num % self.blocks_per_group as u64) as u32
    }
    
    /// Calculate checksum for a block group
    fn calculate_group_checksum(&self, group: &VexfsBlockGroup) -> u32 {
        // Simple XOR checksum for now
        let mut checksum = 0u32;
        
        checksum ^= group.block_bitmap as u32;
        checksum ^= group.inode_bitmap as u32;
        checksum ^= group.inode_table as u32;
        checksum ^= group.free_blocks_count;
        checksum ^= group.free_inodes_count;
        checksum ^= group.used_dirs_count;
        checksum ^= group.flags as u32;
        
        checksum
    }
    
    /// Allocate blocks specifically for file data
    pub fn allocate_data_blocks(&mut self, count: u32, inode_hint: Option<u64>) -> Result<BlockHandle, SpaceAllocError> {
        let hint = if let Some(ino) = inode_hint {
            // Create allocation hint based on inode number for locality
            Some(AllocHint::new((ino % 1000) * self.blocks_per_group as u64))
        } else {
            None
        };
        
        let result = self.allocate_blocks(count, hint)?;
        let block_group = self.block_to_group(result.start_block);
        
        Ok(BlockHandle::new(result.start_block, result.block_count, block_group))
    }
    
    /// Allocate blocks for a specific file
    pub fn allocate_for_file(&mut self, count: u32, ino: u64) -> Result<BlockHandle, SpaceAllocError> {
        self.allocate_data_blocks(count, Some(ino))
    }
}

impl VexfsBlockGroup {
    /// Create a new block group
    pub const fn new() -> Self {
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
    
    /// Check if block group is valid
    pub fn is_valid(&self, allocator: &VexfsSpaceAllocator) -> bool {
        let expected_checksum = allocator.calculate_group_checksum(self);
        self.checksum == expected_checksum
    }
}

impl FreeSpaceCache {
    /// Create a new free space cache
    pub fn new() -> Self {
        Self {
            free_blocks_per_group: [0; VEXFS_MAX_BLOCK_GROUPS],
            last_alloc_group: 0,
            dirty_groups: 0,
            valid: false,
        }
    }
    
    /// Invalidate the entire cache
    pub fn invalidate(&mut self) {
        self.valid = false;
        self.dirty_groups = 0;
    }
    
    /// Check if a group is dirty
    pub fn is_group_dirty(&self, group_idx: u32) -> bool {
        if group_idx < 64 {
            (self.dirty_groups & (1u64 << group_idx)) != 0
        } else {
            false
        }
    }
    
    /// Mark a group as clean
    pub fn mark_group_clean(&mut self, group_idx: u32) {
        if group_idx < 64 {
            self.dirty_groups &= !(1u64 << group_idx);
        }
    }
}

impl AllocHint {
    /// Create a new allocation hint
    pub fn new(preferred_start: u64) -> Self {
        Self {
            preferred_start,
            goal_block: preferred_start,
            flags: 0,
            min_contiguous: 1,
            max_search_distance: 1000,
        }
    }
    
    /// Create hint for contiguous allocation
    pub fn contiguous(start: u64, min_blocks: u32) -> Self {
        Self {
            preferred_start: start,
            goal_block: start,
            flags: ALLOC_HINT_CONTIGUOUS,
            min_contiguous: min_blocks,
            max_search_distance: 100,
        }
    }
    
    /// Create hint for metadata allocation
    pub fn metadata(start: u64) -> Self {
        Self {
            preferred_start: start,
            goal_block: start,
            flags: ALLOC_HINT_METADATA,
            min_contiguous: 1,
            max_search_distance: 50,
        }
    }
}

// Allocation hint flags
pub const ALLOC_HINT_CONTIGUOUS: u32 = 0x01;
pub const ALLOC_HINT_METADATA: u32 = 0x02;
pub const ALLOC_HINT_DATA: u32 = 0x04;
pub const ALLOC_HINT_LOCALITY: u32 = 0x08;

// Constants
pub const VEXFS_MAX_BLOCK_GROUPS: usize = 256;
pub const VEXFS_DEFAULT_BLOCKS_PER_GROUP: u32 = 32768;

impl FreeExtent {
    /// Create a new free extent
    pub fn new(start: u64, length: u32) -> Self {
        Self {
            start,
            length,
            next: None,
        }
    }
    
    /// Check if this extent can satisfy an allocation request
    pub fn can_satisfy(&self, size: u32) -> bool {
        self.length >= size
    }
    
    /// Split this extent for allocation
    pub fn split(&self, alloc_size: u32) -> Option<FreeExtent> {
        if self.length > alloc_size {
            Some(FreeExtent::new(
                self.start + alloc_size as u64,
                self.length - alloc_size
            ))
        } else {
            None
        }
    }
    
    /// Check if this extent is adjacent to another
    pub fn is_adjacent(&self, other: &FreeExtent) -> bool {
        self.start + self.length as u64 == other.start ||
        other.start + other.length as u64 == self.start
    }
    
    /// Merge with another adjacent extent
    pub fn merge(&self, other: &FreeExtent) -> Option<FreeExtent> {
        if self.is_adjacent(other) {
            let new_start = core::cmp::min(self.start, other.start);
            let new_length = self.length + other.length;
            
            Some(FreeExtent::new(new_start, new_length))
        } else {
            None
        }
    }
}

/// Bitmap operations for block allocation
pub struct BitmapOps;

impl BitmapOps {
    /// Set a bit in bitmap
    pub fn set_bit(bitmap: &mut [u8], bit_num: u32) {
        let byte_idx = (bit_num / 8) as usize;
        let bit_idx = bit_num % 8;
        
        if byte_idx < bitmap.len() {
            bitmap[byte_idx] |= 1u8 << bit_idx;
        }
    }
    
    /// Clear a bit in bitmap
    pub fn clear_bit(bitmap: &mut [u8], bit_num: u32) {
        let byte_idx = (bit_num / 8) as usize;
        let bit_idx = bit_num % 8;
        
        if byte_idx < bitmap.len() {
            bitmap[byte_idx] &= !(1u8 << bit_idx);
        }
    }
    
    /// Test a bit in bitmap
    pub fn test_bit(bitmap: &[u8], bit_num: u32) -> bool {
        let byte_idx = (bit_num / 8) as usize;
        let bit_idx = bit_num % 8;
        
        if byte_idx < bitmap.len() {
            (bitmap[byte_idx] & (1u8 << bit_idx)) != 0
        } else {
            false
        }
    }
    
    /// Find first zero bit in bitmap
    pub fn find_first_zero_bit(bitmap: &[u8], start_bit: u32, max_bits: u32) -> Option<u32> {
        for bit in start_bit..core::cmp::min(start_bit + max_bits, bitmap.len() as u32 * 8) {
            if !Self::test_bit(bitmap, bit) {
                return Some(bit);
            }
        }
        None
    }
    
    /// Find first contiguous zero bits
    pub fn find_contiguous_zero_bits(bitmap: &[u8], count: u32, start_bit: u32, max_bits: u32) -> Option<u32> {
        let mut consecutive = 0;
        let mut start_pos = None;
        
        for bit in start_bit..core::cmp::min(start_bit + max_bits, bitmap.len() as u32 * 8) {
            if !Self::test_bit(bitmap, bit) {
                if start_pos.is_none() {
                    start_pos = Some(bit);
                }
                consecutive += 1;
                
                if consecutive >= count {
                    return start_pos;
                }
            } else {
                consecutive = 0;
                start_pos = None;
            }
        }
        
        None
    }
    
    /// Count zero bits in bitmap
    pub fn count_zero_bits(bitmap: &[u8]) -> u32 {
        let mut count = 0;
        
        for &byte in bitmap {
            count += byte.count_zeros();
        }
        
        count
    }
}