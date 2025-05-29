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

//! Filesystem Layout Management
//!
//! This module handles VexFS filesystem layout calculations and organization,
//! including block group layout and space allocation planning.

use crate::shared::errors::{VexfsError, VexfsResult};
use crate::shared::types::*;
use crate::shared::constants::*;
use crate::shared::utils::*;

#[cfg(feature = "kernel")]
use alloc::vec::Vec;

#[cfg(not(feature = "kernel"))]
use std::vec::Vec;

/// Filesystem layout calculator and manager
#[derive(Debug, Clone)]
pub struct VexfsLayout {
    /// Total filesystem size in blocks
    pub total_blocks: u64,
    /// Block size in bytes
    pub block_size: u32,
    /// Blocks per block group
    pub blocks_per_group: u32,
    /// Inodes per block group
    pub inodes_per_group: u32,
    /// Total number of block groups
    pub group_count: u32,
    /// Inode size in bytes
    pub inode_size: u16,
    /// Journal size in blocks
    pub journal_blocks: u32,
    /// Vector index blocks
    pub vector_blocks: u32,
}

impl VexfsLayout {
    /// Calculate optimal layout for given filesystem parameters
    pub fn calculate(
        device_size: u64,
        block_size: u32,
        inode_ratio: u32,
        journal_size: Option<u32>,
        vector_enabled: bool,
    ) -> VexfsResult<Self> {
        // Validate block size
        if !block_size.is_power_of_two() || 
           block_size < VEXFS_MIN_BLOCK_SIZE as u32 ||
           block_size > VEXFS_MAX_BLOCK_SIZE as u32 {
            return Err(VexfsError::InvalidArgument("invalid block size".to_string()));
        }

        let total_blocks = device_size / block_size as u64;
        if total_blocks < VEXFS_MIN_FILESYSTEM_BLOCKS {
            return Err(VexfsError::InvalidArgument("filesystem too small".to_string()));
        }

        // Calculate optimal blocks per group (typically 32K blocks = 128MB at 4KB blocks)
        let blocks_per_group = Self::calculate_blocks_per_group(block_size)?;
        
        // Calculate inodes per group based on inode ratio
        let inodes_per_group = Self::calculate_inodes_per_group(blocks_per_group, inode_ratio)?;
        
        // Calculate number of block groups
        let group_count = ((total_blocks + blocks_per_group as u64 - 1) / blocks_per_group as u64) as u32;
        
        // Calculate journal size
        let journal_blocks = journal_size.unwrap_or_else(|| {
            Self::calculate_default_journal_size(total_blocks)
        });
        
        // Calculate vector index space
        let vector_blocks = if vector_enabled {
            Self::calculate_vector_blocks(total_blocks)
        } else {
            0
        };

        Ok(Self {
            total_blocks,
            block_size,
            blocks_per_group,
            inodes_per_group,
            group_count,
            inode_size: VEXFS_DEFAULT_INODE_SIZE as u16,
            journal_blocks,
            vector_blocks,
        })
    }

    /// Get layout for specific block group
    pub fn get_group_layout(&self, group_id: u32) -> VexfsResult<BlockGroupLayout> {
        if group_id >= self.group_count {
            return Err(VexfsError::InvalidArgument("invalid group ID".to_string()));
        }

        let start_block = group_id as u64 * self.blocks_per_group as u64;
        
        // Last group might have fewer blocks
        let blocks_in_group = if group_id == self.group_count - 1 {
            let remaining = self.total_blocks - start_block;
            core::cmp::min(remaining, self.blocks_per_group as u64) as u32
        } else {
            self.blocks_per_group
        };

        // Block group descriptor block (group 0 has special layout)
        let group_desc_block = if group_id == 0 {
            2 // Block 0 = superblock, Block 1 = group descriptors
        } else {
            start_block + 1
        };

        // Block bitmap location
        let block_bitmap_block = start_block + if group_id == 0 { 3 } else { 2 };
        
        // Inode bitmap location
        let inode_bitmap_block = block_bitmap_block + 1;
        
        // Inode table location
        let inode_table_block = inode_bitmap_block + 1;
        let inode_table_blocks = Self::calculate_inode_table_blocks(
            self.inodes_per_group,
            self.inode_size,
            self.block_size,
        );

        // Data blocks start after inode table
        let data_start_block = inode_table_block + inode_table_blocks as u64;
        let data_blocks = (start_block + blocks_in_group as u64).saturating_sub(data_start_block);

        Ok(BlockGroupLayout {
            group_id,
            start_block,
            blocks_in_group,
            group_desc_block,
            block_bitmap_block,
            inode_bitmap_block,
            inode_table_block,
            inode_table_blocks,
            data_start_block,
            data_blocks: data_blocks as u32,
            free_blocks: data_blocks as u32, // Initially all data blocks are free
            free_inodes: self.inodes_per_group,
        })
    }

    /// Get superblock layout information
    pub fn get_superblock_layout(&self) -> SuperblockLayout {
        SuperblockLayout {
            primary_block: 0,
            backup_blocks: self.get_backup_superblock_locations(),
            group_desc_block: 1,
            group_desc_blocks: self.calculate_group_desc_blocks(),
        }
    }

    /// Get journal layout information
    pub fn get_journal_layout(&self) -> VexfsResult<JournalLayout> {
        if self.journal_blocks == 0 {
            return Ok(JournalLayout {
                start_block: 0,
                block_count: 0,
                is_external: false,
            });
        }

        // Journal starts after superblock and group descriptors
        let journal_start = 1 + self.calculate_group_desc_blocks() as u64;
        
        Ok(JournalLayout {
            start_block: journal_start,
            block_count: self.journal_blocks,
            is_external: false,
        })
    }

    /// Get vector storage layout information
    pub fn get_vector_layout(&self) -> VectorLayout {
        if self.vector_blocks == 0 {
            return VectorLayout {
                start_block: 0,
                block_count: 0,
                metadata_blocks: 0,
                index_blocks: 0,
                data_blocks: 0,
            };
        }

        // Vector storage starts after journal
        let journal_layout = self.get_journal_layout().unwrap_or_else(|_| JournalLayout {
            start_block: 0,
            block_count: 0,
            is_external: false,
        });
        
        let vector_start = journal_layout.start_block + journal_layout.block_count as u64;
        
        // Allocate space: 10% metadata, 40% index, 50% data
        let metadata_blocks = core::cmp::max(1, self.vector_blocks / 10);
        let index_blocks = self.vector_blocks * 4 / 10;
        let data_blocks = self.vector_blocks - metadata_blocks - index_blocks;

        VectorLayout {
            start_block: vector_start,
            block_count: self.vector_blocks,
            metadata_blocks,
            index_blocks,
            data_blocks,
        }
    }

    /// Calculate total overhead blocks
    pub fn calculate_overhead(&self) -> u32 {
        let superblock_overhead = 1; // Primary superblock
        let group_desc_overhead = self.calculate_group_desc_blocks();
        let bitmap_overhead = self.group_count * 2; // Block and inode bitmaps per group
        let inode_table_overhead = self.group_count * Self::calculate_inode_table_blocks(
            self.inodes_per_group,
            self.inode_size,
            self.block_size,
        );
        
        superblock_overhead + group_desc_overhead + bitmap_overhead + inode_table_overhead + 
        self.journal_blocks + self.vector_blocks
    }

    /// Get available data blocks
    pub fn get_data_blocks(&self) -> u64 {
        self.total_blocks.saturating_sub(self.calculate_overhead() as u64)
    }

    /// Get filesystem utilization efficiency
    pub fn get_efficiency(&self) -> f32 {
        let data_blocks = self.get_data_blocks();
        (data_blocks as f32) / (self.total_blocks as f32) * 100.0
    }

    /// Validate layout consistency
    pub fn validate(&self) -> VexfsResult<()> {
        // Check total blocks
        if self.total_blocks < VEXFS_MIN_FILESYSTEM_BLOCKS {
            return Err(VexfsError::InvalidData("filesystem too small".to_string()));
        }

        // Check block group parameters
        if self.blocks_per_group == 0 || self.inodes_per_group == 0 {
            return Err(VexfsError::InvalidData("invalid block group parameters".to_string()));
        }

        // Check that overhead doesn't exceed total space
        let overhead = self.calculate_overhead();
        if overhead as u64 >= self.total_blocks {
            return Err(VexfsError::InvalidData("overhead exceeds total space".to_string()));
        }

        // Check efficiency is reasonable (at least 70%)
        if self.get_efficiency() < 70.0 {
            return Err(VexfsError::InvalidData("layout efficiency too low".to_string()));
        }

        Ok(())
    }

    // Private helper methods

    fn calculate_blocks_per_group(block_size: u32) -> VexfsResult<u32> {
        // Target group size: 128MB for good performance
        let target_group_size = 128 * 1024 * 1024;
        let blocks_per_group = target_group_size / block_size;
        
        // Ensure it's within reasonable bounds
        let min_blocks = 1024; // Minimum 1K blocks per group
        let max_blocks = 65536; // Maximum 64K blocks per group
        
        Ok(blocks_per_group.clamp(min_blocks, max_blocks))
    }

    fn calculate_inodes_per_group(blocks_per_group: u32, inode_ratio: u32) -> VexfsResult<u32> {
        // inode_ratio is bytes per inode (default 16384 = 16KB per inode)
        let block_size = 4096; // Assume 4KB blocks for calculation
        let bytes_per_group = blocks_per_group as u64 * block_size as u64;
        let inodes_per_group = (bytes_per_group / inode_ratio as u64) as u32;
        
        // Ensure reasonable bounds
        let min_inodes = 256;
        let max_inodes = 65536;
        
        Ok(inodes_per_group.clamp(min_inodes, max_inodes))
    }

    fn calculate_inode_table_blocks(inodes_per_group: u32, inode_size: u16, block_size: u32) -> u32 {
        let total_inode_bytes = inodes_per_group as u64 * inode_size as u64;
        ((total_inode_bytes + block_size as u64 - 1) / block_size as u64) as u32
    }

    fn calculate_group_desc_blocks(&self) -> u32 {
        let desc_size = 32; // Size of block group descriptor
        let descs_per_block = self.block_size / desc_size;
        (self.group_count + descs_per_block - 1) / descs_per_block
    }

    fn calculate_default_journal_size(total_blocks: u64) -> u32 {
        // Default journal size: 1% of filesystem, min 1MB, max 128MB
        let one_percent = total_blocks / 100;
        let min_blocks = (1024 * 1024) / 4096; // 1MB in 4KB blocks
        let max_blocks = (128 * 1024 * 1024) / 4096; // 128MB in 4KB blocks
        
        (one_percent as u32).clamp(min_blocks, max_blocks)
    }

    fn calculate_vector_blocks(total_blocks: u64) -> u32 {
        // Vector storage: 5% of filesystem for vector-enabled filesystems
        let five_percent = total_blocks / 20;
        let max_vector_blocks = (512 * 1024 * 1024) / 4096; // Max 512MB in 4KB blocks
        
        core::cmp::min(five_percent as u32, max_vector_blocks)
    }

    fn get_backup_superblock_locations(&self) -> Vec<u64> {
        let mut backups = Vec::new();
        
        // Sparse superblock: backup at groups 1, 3, 5, 7, 9, 25, 27, 49, 81, ...
        for &group in &[1, 3, 5, 7, 9, 25, 27, 49, 81] {
            if group < self.group_count {
                backups.push(group as u64 * self.blocks_per_group as u64);
            }
        }
        
        backups
    }
}

/// Block group layout information
#[derive(Debug, Clone)]
pub struct BlockGroupLayout {
    pub group_id: u32,
    pub start_block: u64,
    pub blocks_in_group: u32,
    pub group_desc_block: u64,
    pub block_bitmap_block: u64,
    pub inode_bitmap_block: u64,
    pub inode_table_block: u64,
    pub inode_table_blocks: u32,
    pub data_start_block: u64,
    pub data_blocks: u32,
    pub free_blocks: u32,
    pub free_inodes: u32,
}

impl BlockGroupLayout {
    /// Check if block is within this group's data area
    pub fn contains_data_block(&self, block: u64) -> bool {
        block >= self.data_start_block && 
        block < self.data_start_block + self.data_blocks as u64
    }

    /// Get relative block number within group
    pub fn get_relative_block(&self, block: u64) -> Option<u32> {
        if block >= self.start_block && block < self.start_block + self.blocks_in_group as u64 {
            Some((block - self.start_block) as u32)
        } else {
            None
        }
    }

    /// Get data utilization percentage
    pub fn get_utilization(&self) -> u32 {
        if self.data_blocks == 0 {
            return 0;
        }
        let used = self.data_blocks - self.free_blocks;
        (used * 100) / self.data_blocks
    }

    /// Get inode utilization percentage
    pub fn get_inode_utilization(&self, inodes_per_group: u32) -> u32 {
        if inodes_per_group == 0 {
            return 0;
        }
        let used = inodes_per_group - self.free_inodes;
        (used * 100) / inodes_per_group
    }
}

/// Superblock layout information
#[derive(Debug, Clone)]
pub struct SuperblockLayout {
    pub primary_block: u64,
    pub backup_blocks: Vec<u64>,
    pub group_desc_block: u64,
    pub group_desc_blocks: u32,
}

/// Journal layout information
#[derive(Debug, Clone)]
pub struct JournalLayout {
    pub start_block: u64,
    pub block_count: u32,
    pub is_external: bool,
}

/// Vector storage layout information
#[derive(Debug, Clone)]
pub struct VectorLayout {
    pub start_block: u64,
    pub block_count: u32,
    pub metadata_blocks: u32,
    pub index_blocks: u32,
    pub data_blocks: u32,
}

impl VectorLayout {
    /// Get block range for metadata
    pub fn get_metadata_range(&self) -> (u64, u64) {
        (self.start_block, self.start_block + self.metadata_blocks as u64)
    }

    /// Get block range for index data
    pub fn get_index_range(&self) -> (u64, u64) {
        let start = self.start_block + self.metadata_blocks as u64;
        (start, start + self.index_blocks as u64)
    }

    /// Get block range for vector data
    pub fn get_data_range(&self) -> (u64, u64) {
        let start = self.start_block + self.metadata_blocks as u64 + self.index_blocks as u64;
        (start, start + self.data_blocks as u64)
    }
}

/// Layout calculator for mkfs operations
pub struct LayoutCalculator;

impl LayoutCalculator {
    /// Calculate layout for new filesystem
    pub fn calculate_mkfs_layout(
        device_size: u64,
        block_size: Option<u32>,
        inode_ratio: Option<u32>,
        journal_size: Option<u32>,
        enable_vectors: bool,
    ) -> VexfsResult<VexfsLayout> {
        let block_size = block_size.unwrap_or(VEXFS_DEFAULT_BLOCK_SIZE as u32);
        let inode_ratio = inode_ratio.unwrap_or(16384); // 16KB per inode default
        
        VexfsLayout::calculate(device_size, block_size, inode_ratio, journal_size, enable_vectors)
    }

    /// Estimate space requirements for given parameters
    pub fn estimate_requirements(
        file_count: u64,
        avg_file_size: u64,
        vector_count: Option<u64>,
        vector_dimensions: Option<u16>,
    ) -> SpaceEstimate {
        let data_size = file_count * avg_file_size;
        let inode_size = file_count * VEXFS_DEFAULT_INODE_SIZE as u64;
        
        let vector_size = if let (Some(count), Some(dims)) = (vector_count, vector_dimensions) {
            count * dims as u64 * 4 + count * 128 // 4 bytes per dim + index overhead
        } else {
            0
        };

        let metadata_overhead = (data_size + inode_size + vector_size) / 20; // ~5% overhead
        
        SpaceEstimate {
            total_data: data_size,
            inode_space: inode_size,
            vector_space: vector_size,
            metadata_overhead,
            recommended_size: data_size + inode_size + vector_size + metadata_overhead * 2,
        }
    }

    /// Validate layout against device constraints
    pub fn validate_device_layout(layout: &VexfsLayout, device_size: u64) -> VexfsResult<()> {
        if layout.total_blocks * layout.block_size as u64 > device_size {
            return Err(VexfsError::InvalidArgument("layout exceeds device size".to_string()));
        }

        layout.validate()
    }
}

/// Space usage estimate
#[derive(Debug, Clone)]
pub struct SpaceEstimate {
    pub total_data: u64,
    pub inode_space: u64,
    pub vector_space: u64,
    pub metadata_overhead: u64,
    pub recommended_size: u64,
}

impl SpaceEstimate {
    /// Get efficiency percentage
    pub fn get_efficiency(&self) -> f32 {
        if self.recommended_size == 0 {
            return 0.0;
        }
        (self.total_data as f32 / self.recommended_size as f32) * 100.0
    }
}