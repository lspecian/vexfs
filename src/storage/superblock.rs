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

//! Superblock Management
//!
//! This module handles VexFS superblock operations including mount/unmount,
//! metadata management, and filesystem state tracking.

use crate::shared::errors::{VexfsError, VexfsResult};
use crate::shared::types::*;
use crate::shared::constants::*;
use crate::shared::utils::*;
use crate::storage::persistence::{OnDiskSerializable, PersistenceManager};
use core::mem;
use core::slice;

/// VexFS on-disk superblock structure
/// Located at the beginning of the filesystem (block 0)
/// Size: 1024 bytes (fits in quarter of 4KB block with room for expansion)
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct VexfsSuperblock {
    /// Magic number (VEXFS_MAGIC)
    pub s_magic: u64,
    
    /// Total number of blocks in filesystem
    pub s_blocks_count: u64,
    
    /// Number of free blocks
    pub s_free_blocks_count: u64,
    
    /// Total number of inodes
    pub s_inodes_count: u32,
    
    /// Number of free inodes
    pub s_free_inodes_count: u32,
    
    /// Block size in bytes
    pub s_block_size: u32,
    
    /// Inode size in bytes
    pub s_inode_size: u16,
    
    /// Major version
    pub s_version_major: u16,
    
    /// Minor version
    pub s_version_minor: u16,
    
    /// Creation timestamp
    pub s_mkfs_time: u64,
    
    /// Last mount timestamp
    pub s_mount_time: u64,
    
    /// Last write timestamp
    pub s_wtime: u64,
    
    /// Mount count since last fsck
    pub s_mount_count: u16,
    
    /// Maximum mount count before fsck
    pub s_max_mount_count: u16,
    
    /// Filesystem state (clean/error)
    pub s_state: u16,
    
    /// Error behavior
    pub s_errors: u16,
    
    /// Compatible feature flags
    pub s_feature_compat: u32,
    
    /// Incompatible feature flags
    pub s_feature_incompat: u32,
    
    /// Read-only compatible feature flags
    pub s_feature_ro_compat: u32,
    
    /// Filesystem UUID
    pub s_uuid: [u8; 16],
    
    /// Volume name
    pub s_volume_name: [u8; 64],
    
    /// First data block
    pub s_first_data_block: u64,
    
    /// Blocks per group
    pub s_blocks_per_group: u32,
    
    /// Inodes per group
    pub s_inodes_per_group: u32,
    
    /// Number of block groups
    pub s_group_count: u32,
    
    /// Journal inode number
    pub s_journal_inum: u32,
    
    /// Journal block count
    pub s_journal_blocks: u32,
    
    /// First journal block
    pub s_journal_first_block: u64,
    
    // Vector Storage Metadata Extensions
    /// Vector index magic number for validation
    pub s_vector_magic: u32,
    
    /// Vector storage format version
    pub s_vector_version: u16,
    
    /// Vector dimension size (0 = no vectors)
    pub s_vector_dimensions: u16,
    
    /// Vector index algorithm (0=none, 1=HNSW, 2=IVF, etc.)
    pub s_vector_algorithm: u8,
    
    /// Vector distance metric (0=L2, 1=cosine, 2=dot, etc.)
    pub s_vector_metric: u8,
    
    /// Vector index parameters (algorithm-specific)
    pub s_vector_params: [u16; 4],
    
    /// Block containing vector index metadata
    pub s_vector_index_block: u64,
    
    /// Number of blocks used for vector indices
    pub s_vector_index_blocks: u32,
    
    /// Total number of vectors stored
    pub s_vector_count: u64,
    
    /// Vector storage feature flags
    pub s_vector_features: u32,
    
    /// Checksum for superblock integrity
    pub s_checksum: u32,
    
    /// Reserved padding for future extensions
    pub s_reserved: [u32; 126],
}

impl OnDiskSerializable for VexfsSuperblock {
    fn to_bytes(&self) -> &[u8] {
        unsafe {
            slice::from_raw_parts(
                self as *const Self as *const u8,
                mem::size_of::<Self>()
            )
        }
    }

    fn from_bytes(data: &[u8]) -> VexfsResult<Self> {
        if data.len() < mem::size_of::<Self>() {
            return Err(VexfsError::InvalidData("insufficient data for VexfsSuperblock".to_string()));
        }

        let sb = unsafe {
            *(data.as_ptr() as *const Self)
        };

        sb.validate()?;
        Ok(sb)
    }

    fn serialized_size() -> usize {
        mem::size_of::<Self>()
    }

    fn validate(&self) -> VexfsResult<()> {
        // Check magic number
        if self.s_magic != VEXFS_MAGIC as u64 {
            return Err(VexfsError::InvalidMagic);
        }

        // Check version compatibility
        if self.s_version_major > VEXFS_VERSION_MAJOR {
            return Err(VexfsError::UnsupportedVersion);
        }

        // Check block size is power of 2 and within bounds
        if !self.s_block_size.is_power_of_two() || 
           self.s_block_size < VEXFS_MIN_BLOCK_SIZE as u32 ||
           self.s_block_size > VEXFS_MAX_BLOCK_SIZE as u32 {
            return Err(VexfsError::InvalidData("invalid block size".to_string()));
        }

        // Check inode size
        if self.s_inode_size < VEXFS_MIN_INODE_SIZE as u16 ||
           self.s_inode_size > VEXFS_MAX_INODE_SIZE as u16 {
            return Err(VexfsError::InvalidData("invalid inode size".to_string()));
        }

        // Check filesystem consistency
        if self.s_free_blocks_count > self.s_blocks_count {
            return Err(VexfsError::InvalidData("free blocks count exceeds total".to_string()));
        }

        if self.s_free_inodes_count > self.s_inodes_count {
            return Err(VexfsError::InvalidData("free inodes count exceeds total".to_string()));
        }

        // Check block group parameters
        if self.s_blocks_per_group == 0 || self.s_inodes_per_group == 0 {
            return Err(VexfsError::InvalidData("invalid block group parameters".to_string()));
        }

        // Verify checksum if present
        if self.s_checksum != 0 {
            let calculated = self.calculate_checksum();
            if calculated != self.s_checksum {
                return Err(VexfsError::ChecksumMismatch);
            }
        }

        Ok(())
    }

    fn update_checksum(&mut self) {
        self.s_checksum = 0; // Clear for calculation
        self.s_checksum = self.calculate_checksum();
    }
}

impl VexfsSuperblock {
    /// Create new superblock with default values
    pub fn new(
        blocks_count: u64,
        inodes_count: u32,
        block_size: u32,
        blocks_per_group: u32,
        inodes_per_group: u32,
    ) -> VexfsResult<Self> {
        // Validate parameters
        if !block_size.is_power_of_two() || 
           block_size < VEXFS_MIN_BLOCK_SIZE as u32 ||
           block_size > VEXFS_MAX_BLOCK_SIZE as u32 {
            return Err(VexfsError::InvalidArgument("invalid block size".to_string()));
        }

        if blocks_per_group == 0 || inodes_per_group == 0 {
            return Err(VexfsError::InvalidArgument("invalid block group parameters".to_string()));
        }

        let group_count = (blocks_count + blocks_per_group as u64 - 1) / blocks_per_group as u64;

        let mut sb = Self {
            s_magic: VEXFS_MAGIC as u64,
            s_blocks_count: blocks_count,
            s_free_blocks_count: blocks_count,
            s_inodes_count: inodes_count,
            s_free_inodes_count: inodes_count,
            s_block_size: block_size,
            s_inode_size: VEXFS_DEFAULT_INODE_SIZE as u16,
            s_version_major: VEXFS_VERSION_MAJOR,
            s_version_minor: VEXFS_VERSION_MINOR,
            s_mkfs_time: current_time(),
            s_mount_time: 0,
            s_wtime: current_time(),
            s_mount_count: 0,
            s_max_mount_count: VEXFS_DEFAULT_MAX_MOUNT_COUNT as u16,
            s_state: VEXFS_VALID_FS,
            s_errors: VEXFS_ERRORS_CONTINUE,
            s_feature_compat: VEXFS_FEATURE_COMPAT_JOURNAL,
            s_feature_incompat: VEXFS_FEATURE_INCOMPAT_FILETYPE | VEXFS_FEATURE_INCOMPAT_EXTENTS,
            s_feature_ro_compat: 0,
            s_uuid: generate_uuid(),
            s_volume_name: [0; 64],
            s_first_data_block: 1,
            s_blocks_per_group: blocks_per_group,
            s_inodes_per_group: inodes_per_group,
            s_group_count: group_count as u32,
            s_journal_inum: 0,
            s_journal_blocks: VEXFS_DEFAULT_JOURNAL_BLOCKS,
            s_journal_first_block: 0,
            s_vector_magic: 0,
            s_vector_version: 0,
            s_vector_dimensions: 0,
            s_vector_algorithm: 0,
            s_vector_metric: 0,
            s_vector_params: [0; 4],
            s_vector_index_block: 0,
            s_vector_index_blocks: 0,
            s_vector_count: 0,
            s_vector_features: 0,
            s_checksum: 0,
            s_reserved: [0; 126],
        };

        sb.update_checksum();
        Ok(sb)
    }

    /// Set volume name
    pub fn set_volume_name(&mut self, name: &str) -> VexfsResult<()> {
        if name.len() > 63 {
            return Err(VexfsError::InvalidArgument("volume name too long".to_string()));
        }

        self.s_volume_name.fill(0);
        let name_bytes = name.as_bytes();
        self.s_volume_name[0..name_bytes.len()].copy_from_slice(name_bytes);
        self.update_checksum();

        Ok(())
    }

    /// Get volume name as string
    pub fn get_volume_name(&self) -> VexfsResult<&str> {
        let null_pos = self.s_volume_name.iter().position(|&b| b == 0).unwrap_or(64);
        core::str::from_utf8(&self.s_volume_name[0..null_pos])
            .map_err(|_| VexfsError::InvalidData("invalid UTF-8 in volume name".to_string()))
    }

    /// Check if filesystem has journal
    pub fn has_journal(&self) -> bool {
        (self.s_feature_compat & VEXFS_FEATURE_COMPAT_JOURNAL) != 0
    }

    /// Check if filesystem supports vectors
    pub fn supports_vectors(&self) -> bool {
        self.s_vector_magic == VEXFS_VECTOR_MAGIC && self.s_vector_dimensions > 0
    }

    /// Enable vector storage
    pub fn enable_vectors(&mut self, dimensions: u16, algorithm: u8, metric: u8) -> VexfsResult<()> {
        if dimensions == 0 {
            return Err(VexfsError::InvalidArgument("vector dimensions must be > 0".to_string()));
        }

        self.s_vector_magic = VEXFS_VECTOR_MAGIC;
        self.s_vector_version = VEXFS_VECTOR_VERSION as u16;
        self.s_vector_dimensions = dimensions;
        self.s_vector_algorithm = algorithm;
        self.s_vector_metric = metric;
        self.s_vector_count = 0;
        self.s_vector_features = VEXFS_VECTOR_FEATURE_HNSW;
        self.update_checksum();

        Ok(())
    }

    /// Disable vector storage
    pub fn disable_vectors(&mut self) {
        self.s_vector_magic = 0;
        self.s_vector_version = 0;
        self.s_vector_dimensions = 0;
        self.s_vector_algorithm = 0;
        self.s_vector_metric = 0;
        self.s_vector_params = [0; 4];
        self.s_vector_index_block = 0;
        self.s_vector_index_blocks = 0;
        self.s_vector_count = 0;
        self.s_vector_features = 0;
        self.update_checksum();
    }

    /// Update free block count
    pub fn update_free_blocks(&mut self, delta: i64) {
        if delta < 0 {
            self.s_free_blocks_count = self.s_free_blocks_count.saturating_sub((-delta) as u64);
        } else {
            self.s_free_blocks_count = self.s_free_blocks_count.saturating_add(delta as u64);
        }
        self.s_wtime = current_time();
        self.update_checksum();
    }

    /// Update free inode count
    pub fn update_free_inodes(&mut self, delta: i32) {
        if delta < 0 {
            self.s_free_inodes_count = self.s_free_inodes_count.saturating_sub((-delta) as u32);
        } else {
            self.s_free_inodes_count = self.s_free_inodes_count.saturating_add(delta as u32);
        }
        self.s_wtime = current_time();
        self.update_checksum();
    }

    /// Update vector count
    pub fn update_vector_count(&mut self, delta: i64) {
        if delta < 0 {
            self.s_vector_count = self.s_vector_count.saturating_sub((-delta) as u64);
        } else {
            self.s_vector_count = self.s_vector_count.saturating_add(delta as u64);
        }
        self.s_wtime = current_time();
        self.update_checksum();
    }

    /// Mark filesystem as mounted
    pub fn mark_mounted(&mut self) {
        self.s_mount_time = current_time();
        self.s_mount_count = self.s_mount_count.saturating_add(1);
        self.s_state = VEXFS_VALID_FS;
        self.update_checksum();
    }

    /// Mark filesystem as unmounted cleanly
    pub fn mark_unmounted(&mut self) {
        self.s_state = VEXFS_VALID_FS;
        self.s_wtime = current_time();
        self.update_checksum();
    }

    /// Mark filesystem as having errors
    pub fn mark_error(&mut self) {
        self.s_state = VEXFS_ERROR_FS;
        self.s_wtime = current_time();
        self.update_checksum();
    }

    /// Check if filesystem needs fsck
    pub fn needs_fsck(&self) -> bool {
        self.s_state == VEXFS_ERROR_FS || 
        (self.s_max_mount_count > 0 && self.s_mount_count >= self.s_max_mount_count)
    }

    /// Get filesystem utilization percentage
    pub fn get_utilization(&self) -> u32 {
        if self.s_blocks_count == 0 {
            return 0;
        }
        let used = self.s_blocks_count - self.s_free_blocks_count;
        ((used * 100) / self.s_blocks_count) as u32
    }

    /// Get inode utilization percentage
    pub fn get_inode_utilization(&self) -> u32 {
        if self.s_inodes_count == 0 {
            return 0;
        }
        let used = self.s_inodes_count - self.s_free_inodes_count;
        (used * 100) / self.s_inodes_count
    }

    /// Calculate superblock checksum
    fn calculate_checksum(&self) -> u32 {
        let mut data = *self;
        data.s_checksum = 0; // Exclude checksum field
        
        let bytes = unsafe {
            slice::from_raw_parts(
                &data as *const Self as *const u8,
                mem::size_of::<Self>()
            )
        };
        
        crc32(bytes)
    }
}

/// Superblock manager for mount/unmount operations
pub struct SuperblockManager {
    /// Persistence manager for I/O
    persistence: PersistenceManager,
    /// Current superblock
    superblock: Option<VexfsSuperblock>,
    /// Backup superblocks enabled
    backup_enabled: bool,
}

impl SuperblockManager {
    /// Create new superblock manager
    pub fn new() -> VexfsResult<Self> {
        Ok(Self {
            persistence: PersistenceManager::new(4096, true), // Default block size, checksum enabled
            superblock: None,
            backup_enabled: true,
        })
    }

    /// Create new superblock manager with specific parameters
    pub fn new_with_params(block_size: u32, backup_enabled: bool) -> VexfsResult<Self> {
        Ok(Self {
            persistence: PersistenceManager::new(block_size, true), // Checksum enabled
            superblock: None,
            backup_enabled,
        })
    }

    /// Initialize superblock for new filesystem
    pub fn initialize(&mut self, layout: &crate::storage::layout::VexfsLayout) -> VexfsResult<()> {
        // Calculate total inodes from layout
        let total_inodes = layout.group_count * layout.inodes_per_group;
        
        let sb = VexfsSuperblock::new(
            layout.total_blocks,
            total_inodes,
            layout.block_size,
            layout.blocks_per_group,
            layout.inodes_per_group,
        )?;
        
        self.superblock = Some(sb);
        Ok(())
    }

    /// Load and validate superblock from storage
    pub fn load_and_validate(&mut self, block_manager: &mut crate::storage::block::BlockManager) -> VexfsResult<VexfsSuperblock> {
        // Read superblock from block 0
        let data = block_manager.read_block(0)?;
        let sb = self.load_superblock(&data)?;
        Ok(*sb)
    }

    /// Update and sync superblock to storage
    pub fn update_and_sync(&mut self, block_manager: &mut crate::storage::block::BlockManager) -> VexfsResult<()> {
        let data = self.sync()?;
        block_manager.write_block(0, &data)?;
        Ok(())
    }

    /// Load superblock from storage
    pub fn load_superblock(&mut self, data: &[u8]) -> VexfsResult<&VexfsSuperblock> {
        let sb = self.persistence.deserialize_from_block::<VexfsSuperblock>(data)?;
        
        // Additional validation for mount
        if sb.needs_fsck() {
            return Err(VexfsError::NeedsFsck);
        }

        self.superblock = Some(sb);
        Ok(self.superblock.as_ref().unwrap())
    }

    /// Get current superblock
    pub fn get_superblock(&self) -> VexfsResult<&VexfsSuperblock> {
        self.superblock.as_ref().ok_or(VexfsError::NotMounted)
    }

    /// Get mutable superblock reference
    pub fn get_superblock_mut(&mut self) -> VexfsResult<&mut VexfsSuperblock> {
        self.superblock.as_mut().ok_or(VexfsError::NotMounted)
    }

    /// Serialize superblock for writing
    pub fn serialize_superblock(&self) -> VexfsResult<Vec<u8>> {
        let sb = self.get_superblock()?;
        self.persistence.serialize_to_block(sb)
    }

    /// Create new filesystem superblock
    pub fn create_filesystem(
        &mut self,
        blocks_count: u64,
        inodes_count: u32,
        block_size: u32,
        blocks_per_group: u32,
        inodes_per_group: u32,
        volume_name: Option<&str>,
    ) -> VexfsResult<&VexfsSuperblock> {
        let mut sb = VexfsSuperblock::new(
            blocks_count,
            inodes_count,
            block_size,
            blocks_per_group,
            inodes_per_group,
        )?;

        if let Some(name) = volume_name {
            sb.set_volume_name(name)?;
        }

        self.superblock = Some(sb);
        Ok(self.superblock.as_ref().unwrap())
    }

    /// Mount filesystem
    pub fn mount(&mut self) -> VexfsResult<()> {
        let sb = self.get_superblock_mut()?;
        sb.mark_mounted();
        Ok(())
    }

    /// Unmount filesystem
    pub fn unmount(&mut self) -> VexfsResult<()> {
        let sb = self.get_superblock_mut()?;
        sb.mark_unmounted();
        Ok(())
    }

    /// Check if mounted
    pub fn is_mounted(&self) -> bool {
        self.superblock.is_some()
    }

    /// Sync superblock changes
    pub fn sync(&mut self) -> VexfsResult<Vec<u8>> {
        let sb = self.get_superblock_mut()?;
        sb.s_wtime = current_time();
        sb.update_checksum();
        self.serialize_superblock()
    }

    /// Get filesystem statistics
    pub fn get_stats(&self) -> VexfsResult<FilesystemStats> {
        let sb = self.get_superblock()?;
        
        Ok(FilesystemStats {
            total_blocks: sb.s_blocks_count,
            free_blocks: sb.s_free_blocks_count,
            total_inodes: sb.s_inodes_count,
            free_inodes: sb.s_free_inodes_count,
            block_size: sb.s_block_size,
            utilization: sb.get_utilization(),
            inode_utilization: sb.get_inode_utilization(),
            mount_count: sb.s_mount_count,
            vector_count: sb.s_vector_count,
            supports_vectors: sb.supports_vectors(),
        })
    }

    /// Validate filesystem health
    pub fn validate_health(&self) -> VexfsResult<HealthStatus> {
        let sb = self.get_superblock()?;
        
        let mut status = HealthStatus {
            is_clean: sb.s_state == VEXFS_VALID_FS,
            needs_fsck: sb.needs_fsck(),
            utilization_warning: sb.get_utilization() > 90,
            inode_warning: sb.get_inode_utilization() > 90,
            mount_count_warning: sb.s_max_mount_count > 0 && 
                                 sb.s_mount_count >= (sb.s_max_mount_count * 90 / 100),
            errors: Vec::new(),
        };

        if sb.s_state == VEXFS_ERROR_FS {
            status.errors.push("Filesystem has errors".to_string());
        }

        if status.utilization_warning {
            status.errors.push("High disk utilization".to_string());
        }

        if status.inode_warning {
            status.errors.push("High inode utilization".to_string());
        }

        Ok(status)
    }
}

/// Filesystem statistics
#[derive(Debug, Clone)]
pub struct FilesystemStats {
    pub total_blocks: u64,
    pub free_blocks: u64,
    pub total_inodes: u32,
    pub free_inodes: u32,
    pub block_size: u32,
    pub utilization: u32,
    pub inode_utilization: u32,
    pub mount_count: u16,
    pub vector_count: u64,
    pub supports_vectors: bool,
}

/// Filesystem health status
#[derive(Debug, Clone)]
pub struct HealthStatus {
    pub is_clean: bool,
    pub needs_fsck: bool,
    pub utilization_warning: bool,
    pub inode_warning: bool,
    pub mount_count_warning: bool,
    pub errors: Vec<String>,
}

// Re-export filesystem state constants
pub use crate::shared::constants::{
    VEXFS_VALID_FS,
    VEXFS_ERROR_FS,
    VEXFS_ERRORS_CONTINUE,
    VEXFS_ERRORS_RO,
    VEXFS_ERRORS_PANIC,
};

// Re-export feature flag constants
pub use crate::shared::constants::{
    VEXFS_FEATURE_COMPAT_DIR_INDEX,
    VEXFS_FEATURE_COMPAT_RESIZE_INODE,
    VEXFS_FEATURE_COMPAT_JOURNAL,
    VEXFS_FEATURE_INCOMPAT_COMPRESSION,
    VEXFS_FEATURE_INCOMPAT_FILETYPE,
    VEXFS_FEATURE_INCOMPAT_64BIT,
    VEXFS_FEATURE_INCOMPAT_EXTENTS,
    VEXFS_FEATURE_RO_COMPAT_SPARSE_SUPER,
    VEXFS_FEATURE_RO_COMPAT_LARGE_FILE,
    VEXFS_FEATURE_RO_COMPAT_BTREE_DIR,
};