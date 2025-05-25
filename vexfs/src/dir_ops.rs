//! Directory Management System for VexFS
//! 
//! This module implements directory entry management and lookup operations
//! for VexFS directories, including creation, deletion, and traversal.

#![no_std]

use crate::ondisk::*;
use crate::inode_mgmt::*;

/// Directory operations error types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DirOpError {
    NotFound,
    AlreadyExists,
    NotDirectory,
    DirectoryNotEmpty,
    NameTooLong,
    InvalidName,
    NoSpace,
    IoError,
    PermissionDenied,
    CrossDevice,
    IsDirectory,
    TooManyLinks,
}

/// Directory handle for managing open directories
#[derive(Debug)]
pub struct VexfsDirHandle {
    /// Associated inode
    pub inode: VexfsInodeInfo,
    
    /// Current position in directory
    pub pos: u64,
    
    /// Open flags
    pub flags: u32,
    
    /// Reference count
    pub ref_count: u32,
    
    /// Cached directory entries
    pub cached_entries: [VexfsDirEntry; VEXFS_DIR_ENTRIES_PER_BLOCK],
    
    /// Number of cached entries
    pub cached_count: usize,
    
    /// Current block being read
    pub current_block: u64,
}

/// Directory operations manager
pub struct VexfsDirOps {
    /// Block size for directory operations
    pub block_size: u32,
    
    /// Maximum filename length
    pub max_name_len: u16,
    
    /// Inode manager reference
    pub inode_manager: VexfsInodeManager,
}

/// Directory entry iterator
pub struct DirEntryIterator {
    /// Directory handle
    pub dir_handle: VexfsDirHandle,
    
    /// Current position in directory
    pub pos: u64,
    
    /// End position
    pub end_pos: u64,
}

/// Directory lookup result
#[derive(Debug)]
pub struct LookupResult {
    /// Found inode number
    pub ino: u64,
    
    /// File type
    pub file_type: u8,
    
    /// Entry offset in directory
    pub offset: u32,
    
    /// Entry length
    pub rec_len: u16,
}

impl VexfsDirHandle {
    /// Create a new directory handle
    pub fn new(inode: VexfsInodeInfo, flags: u32) -> Result<Self, DirOpError> {
        if !inode.is_dir() {
            return Err(DirOpError::NotDirectory);
        }
        
        Ok(Self {
            inode,
            pos: 0,
            flags,
            ref_count: 1,
            cached_entries: [VexfsDirEntry::new(); VEXFS_DIR_ENTRIES_PER_BLOCK],
            cached_count: 0,
            current_block: 0,
        })
    }
    
    /// Get current directory position
    pub fn position(&self) -> u64 {
        self.pos
    }
    
    /// Seek to a specific position in directory
    pub fn seek(&mut self, pos: u64) -> Result<u64, DirOpError> {
        if pos > self.inode.size() {
            return Err(DirOpError::InvalidName);
        }
        
        self.pos = pos;
        
        // Clear cache if we're moving to a different block
        let new_block = pos / self.block_size() as u64;
        if new_block != self.current_block {
            self.invalidate_cache();
        }
        
        Ok(pos)
    }
    
    /// Get block size for this directory
    pub fn block_size(&self) -> u32 {
        VEXFS_DEFAULT_BLOCK_SIZE
    }
    
    /// Invalidate cached entries
    pub fn invalidate_cache(&mut self) {
        self.cached_count = 0;
        self.current_block = u64::MAX;
    }
    
    /// Increment reference count
    pub fn get(&mut self) -> u32 {
        self.ref_count += 1;
        self.ref_count
    }
    
    /// Decrement reference count
    pub fn put(&mut self) -> u32 {
        if self.ref_count > 0 {
            self.ref_count -= 1;
        }
        self.ref_count
    }
}

impl VexfsDirOps {
    /// Create a new directory operations manager
    pub fn new(block_size: u32, inode_manager: VexfsInodeManager) -> Self {
        Self {
            block_size,
            max_name_len: VEXFS_MAX_FILENAME_LEN,
            inode_manager,
        }
    }
    
    /// Open a directory and create a directory handle
    pub fn opendir(&self, inode: VexfsInodeInfo, flags: u32) -> Result<VexfsDirHandle, DirOpError> {
        // Check permissions
        let uid = 0; // In real implementation, get from current task
        let gid = 0; // In real implementation, get from current task
        
        if !inode.check_permission(uid, gid, 0o4) { // Read permission
            return Err(DirOpError::PermissionDenied);
        }
        
        VexfsDirHandle::new(inode, flags)
    }
    
    /// Create a new directory
    pub fn mkdir(&mut self, parent_inode: &mut VexfsInodeInfo, name: &str, mode: u16, uid: u32, gid: u32) -> Result<VexfsInodeInfo, DirOpError> {
        if name.len() > self.max_name_len as usize {
            return Err(DirOpError::NameTooLong);
        }
        
        if !self.is_valid_name(name) {
            return Err(DirOpError::InvalidName);
        }
        
        // Check if entry already exists
        if self.lookup_entry(parent_inode, name)?.is_some() {
            return Err(DirOpError::AlreadyExists);
        }
        
        // Check permissions in parent directory
        let current_uid = 0; // In real implementation, get from current task
        let current_gid = 0; // In real implementation, get from current task
        
        if !parent_inode.check_permission(current_uid, current_gid, 0o2) { // Write permission
            return Err(DirOpError::PermissionDenied);
        }
        
        // Create new inode for the directory
        let mut new_inode = self.inode_manager.create_inode(
            S_IFDIR | (mode & 0o777),
            uid as u16,
            gid as u16
        ).map_err(|_| DirOpError::NoSpace)?;
        
        // Initialize directory with "." and ".." entries
        self.init_directory(&mut new_inode, parent_inode.ino)?;
        
        // Add entry to parent directory
        self.add_entry(parent_inode, name, new_inode.ino, DT_DIR)?;
        
        // Update parent's link count and times
        parent_inode.disk_inode.i_links_count += 1;
        parent_inode.touch_mtime(0); // In real implementation, use current time
        parent_inode.touch_ctime(0);
        
        Ok(new_inode)
    }
    
    /// Remove a directory
    pub fn rmdir(&mut self, parent_inode: &mut VexfsInodeInfo, name: &str) -> Result<(), DirOpError> {
        if !self.is_valid_name(name) {
            return Err(DirOpError::InvalidName);
        }
        
        // Find the directory entry
        let lookup_result = self.lookup_entry(parent_inode, name)?
            .ok_or(DirOpError::NotFound)?;
        
        // Check permissions in parent directory
        let current_uid = 0; // In real implementation, get from current task
        let current_gid = 0; // In real implementation, get from current task
        
        if !parent_inode.check_permission(current_uid, current_gid, 0o2) { // Write permission
            return Err(DirOpError::PermissionDenied);
        }
        
        // Read the target inode
        let target_inode = self.inode_manager.read_inode(lookup_result.ino)
            .map_err(|_| DirOpError::NotFound)?;
        
        if !target_inode.is_dir() {
            return Err(DirOpError::NotDirectory);
        }
        
        // Check if directory is empty (only contains "." and "..")
        if !self.is_empty_directory(&target_inode)? {
            return Err(DirOpError::DirectoryNotEmpty);
        }
        
        // Remove entry from parent directory
        self.remove_entry(parent_inode, name)?;
        
        // Free the inode
        self.inode_manager.free_inode(lookup_result.ino)
            .map_err(|_| DirOpError::IoError)?;
        
        // Update parent's link count and times
        if parent_inode.disk_inode.i_links_count > 0 {
            parent_inode.disk_inode.i_links_count -= 1;
        }
        parent_inode.touch_mtime(0);
        parent_inode.touch_ctime(0);
        
        Ok(())
    }
    
    /// Create a regular file in a directory
    pub fn create_file(&mut self, parent_inode: &mut VexfsInodeInfo, name: &str, mode: u16, uid: u32, gid: u32) -> Result<VexfsInodeInfo, DirOpError> {
        if name.len() > self.max_name_len as usize {
            return Err(DirOpError::NameTooLong);
        }
        
        if !self.is_valid_name(name) {
            return Err(DirOpError::InvalidName);
        }
        
        // Check if entry already exists
        if self.lookup_entry(parent_inode, name)?.is_some() {
            return Err(DirOpError::AlreadyExists);
        }
        
        // Check permissions in parent directory
        let current_uid = 0; // In real implementation, get from current task
        let current_gid = 0; // In real implementation, get from current task
        
        if !parent_inode.check_permission(current_uid, current_gid, 0o2) { // Write permission
            return Err(DirOpError::PermissionDenied);
        }
        
        // Create new inode for the file
        let new_inode = self.inode_manager.create_inode(
            S_IFREG | (mode & 0o777),
            uid as u16,
            gid as u16
        ).map_err(|_| DirOpError::NoSpace)?;
        
        // Add entry to parent directory
        self.add_entry(parent_inode, name, new_inode.ino, DT_REG)?;
        
        // Update parent's times
        parent_inode.touch_mtime(0);
        parent_inode.touch_ctime(0);
        
        Ok(new_inode)
    }
    
    /// Remove a file from a directory
    pub fn unlink(&mut self, parent_inode: &mut VexfsInodeInfo, name: &str) -> Result<(), DirOpError> {
        if !self.is_valid_name(name) {
            return Err(DirOpError::InvalidName);
        }
        
        // Find the file entry
        let lookup_result = self.lookup_entry(parent_inode, name)?
            .ok_or(DirOpError::NotFound)?;
        
        // Check permissions in parent directory
        let current_uid = 0; // In real implementation, get from current task
        let current_gid = 0; // In real implementation, get from current task
        
        if !parent_inode.check_permission(current_uid, current_gid, 0o2) { // Write permission
            return Err(DirOpError::PermissionDenied);
        }
        
        // Read the target inode
        let mut target_inode = self.inode_manager.read_inode(lookup_result.ino)
            .map_err(|_| DirOpError::NotFound)?;
        
        if target_inode.is_dir() {
            return Err(DirOpError::IsDirectory);
        }
        
        // Remove entry from parent directory
        self.remove_entry(parent_inode, name)?;
        
        // Decrease link count
        if target_inode.disk_inode.i_links_count > 0 {
            target_inode.disk_inode.i_links_count -= 1;
        }
        
        // If no more links, free the inode
        if target_inode.disk_inode.i_links_count == 0 {
            self.inode_manager.free_inode(lookup_result.ino)
                .map_err(|_| DirOpError::IoError)?;
        }
        
        // Update parent's times
        parent_inode.touch_mtime(0);
        parent_inode.touch_ctime(0);
        
        Ok(())
    }
    
    /// Lookup an entry in a directory
    pub fn lookup_entry(&self, dir_inode: &VexfsInodeInfo, name: &str) -> Result<Option<LookupResult>, DirOpError> {
        if !dir_inode.is_dir() {
            return Err(DirOpError::NotDirectory);
        }
        
        if name.len() > self.max_name_len as usize {
            return Err(DirOpError::NameTooLong);
        }
        
        // Search through directory blocks
        let file_size = dir_inode.size();
        let mut offset = 0u64;
        
        while offset < file_size {
            let block_offset = offset / self.block_size as u64;
            
            // Get block for this offset
            if let Some(block_num) = dir_inode.get_block(block_offset) {
                // In a real implementation, we would read the block from disk
                // For now, simulate reading directory entries
                
                // Parse directory entries in this block
                let entries_in_block = self.parse_directory_block(block_num, offset % self.block_size as u64)?;
                
                for entry in entries_in_block.iter() {
                    if entry.name_len > 0 && entry.matches_name(name) {
                        return Ok(Some(LookupResult {
                            ino: entry.inode as u64,
                            file_type: entry.file_type,
                            offset: offset as u32,
                            rec_len: entry.rec_len,
                        }));
                    }
                    
                    offset += entry.rec_len as u64;
                    if offset >= file_size {
                        break;
                    }
                }
            } else {
                // Sparse directory block - skip
                offset += self.block_size as u64;
            }
        }
        
        Ok(None)
    }
    
    /// Add an entry to a directory
    pub fn add_entry(&mut self, dir_inode: &mut VexfsInodeInfo, name: &str, ino: u64, file_type: u8) -> Result<(), DirOpError> {
        if !dir_inode.is_dir() {
            return Err(DirOpError::NotDirectory);
        }
        
        if name.len() > self.max_name_len as usize {
            return Err(DirOpError::NameTooLong);
        }
        
        let entry_size = VexfsDirEntry::calc_size(name.len() as u8);
        
        // Find space for the new entry
        let insertion_point = self.find_insertion_point(dir_inode, entry_size)?;
        
        // Create the new directory entry
        let mut new_entry = VexfsDirEntry::new();
        new_entry.inode = ino as u32;
        new_entry.file_type = file_type;
        new_entry.name_len = name.len() as u8;
        new_entry.rec_len = entry_size;
        
        // In a real implementation, we would:
        // 1. Read the block containing the insertion point
        // 2. Insert the new entry
        // 3. Adjust the previous entry's rec_len
        // 4. Write the block back to disk
        
        // Update directory size if we added at the end
        let new_offset = insertion_point.offset + entry_size as u64;
        if new_offset > dir_inode.size() {
            dir_inode.set_size(new_offset);
        }
        
        // Update directory modification time
        dir_inode.touch_mtime(0);
        dir_inode.touch_ctime(0);
        
        Ok(())
    }
    
    /// Remove an entry from a directory
    pub fn remove_entry(&mut self, dir_inode: &mut VexfsInodeInfo, name: &str) -> Result<(), DirOpError> {
        let lookup_result = self.lookup_entry(dir_inode, name)?
            .ok_or(DirOpError::NotFound)?;
        
        // In a real implementation, we would:
        // 1. Read the block containing the entry
        // 2. Remove the entry by adjusting the previous entry's rec_len
        // 3. Write the block back to disk
        
        // Update directory modification time
        dir_inode.touch_mtime(0);
        dir_inode.touch_ctime(0);
        
        Ok(())
    }
    
    /// Read directory entries
    pub fn readdir(&self, dir_handle: &mut VexfsDirHandle, buffer: &mut [VexfsDirEntry], count: usize) -> Result<usize, DirOpError> {
        if !dir_handle.inode.is_dir() {
            return Err(DirOpError::NotDirectory);
        }
        
        let file_size = dir_handle.inode.size();
        if dir_handle.pos >= file_size {
            return Ok(0); // EOF
        }
        
        let mut entries_read = 0;
        let mut current_pos = dir_handle.pos;
        
        while entries_read < count && current_pos < file_size {
            let block_offset = current_pos / self.block_size as u64;
            
            // Get block for this offset
            if let Some(block_num) = dir_handle.inode.get_block(block_offset) {
                // Parse directory entries in this block
                let entries_in_block = self.parse_directory_block(block_num, current_pos % self.block_size as u64)?;
                
                for entry in entries_in_block.iter() {
                    if entries_read >= count {
                        break;
                    }
                    
                    if entry.name_len > 0 {
                        buffer[entries_read] = *entry;
                        entries_read += 1;
                    }
                    
                    current_pos += entry.rec_len as u64;
                    if current_pos >= file_size {
                        break;
                    }
                }
            } else {
                // Sparse directory block - skip
                current_pos += self.block_size as u64;
            }
        }
        
        dir_handle.pos = current_pos;
        Ok(entries_read)
    }
    
    /// Initialize a new directory with "." and ".." entries
    fn init_directory(&mut self, dir_inode: &mut VexfsInodeInfo, parent_ino: u64) -> Result<(), DirOpError> {
        // Create "." entry (self)
        self.add_entry(dir_inode, ".", dir_inode.ino, DT_DIR)?;
        
        // Create ".." entry (parent)
        self.add_entry(dir_inode, "..", parent_ino, DT_DIR)?;
        
        // Set initial link count (. and .. entries)
        dir_inode.disk_inode.i_links_count = 2;
        
        Ok(())
    }
    
    /// Check if a directory is empty (only contains "." and "..")
    fn is_empty_directory(&self, dir_inode: &VexfsInodeInfo) -> Result<bool, DirOpError> {
        let file_size = dir_inode.size();
        let mut offset = 0u64;
        let mut entry_count = 0;
        
        while offset < file_size {
            let block_offset = offset / self.block_size as u64;
            
            if let Some(block_num) = dir_inode.get_block(block_offset) {
                let entries_in_block = self.parse_directory_block(block_num, offset % self.block_size as u64)?;
                
                for entry in entries_in_block.iter() {
                    if entry.name_len > 0 {
                        entry_count += 1;
                        
                        // Skip "." and ".." entries
                        if entry_count > 2 {
                            return Ok(false); // Directory is not empty
                        }
                    }
                    
                    offset += entry.rec_len as u64;
                    if offset >= file_size {
                        break;
                    }
                }
            } else {
                offset += self.block_size as u64;
            }
        }
        
        Ok(entry_count <= 2)
    }
    
    /// Validate a filename
    fn is_valid_name(&self, name: &str) -> bool {
        if name.is_empty() || name == "." || name == ".." {
            return false;
        }
        
        // Check for invalid characters
        for ch in name.chars() {
            if ch == '/' || ch == '\0' {
                return false;
            }
        }
        
        true
    }
    
    /// Find insertion point for a new directory entry
    fn find_insertion_point(&self, dir_inode: &VexfsInodeInfo, entry_size: u16) -> Result<InsertionPoint, DirOpError> {
        let file_size = dir_inode.size();
        let mut offset = 0u64;
        
        // Search for free space in existing blocks
        while offset < file_size {
            let block_offset = offset / self.block_size as u64;
            
            if let Some(block_num) = dir_inode.get_block(block_offset) {
                // In a real implementation, we would check for free space in this block
                // For now, assume we need to append at the end
            }
            
            offset += self.block_size as u64;
        }
        
        // Append at the end of the directory
        Ok(InsertionPoint {
            block_num: (file_size / self.block_size as u64) + 1,
            offset: file_size,
            available_space: entry_size,
        })
    }
    
    /// Parse directory entries from a block (stub implementation)
    fn parse_directory_block(&self, block_num: u64, start_offset: u64) -> Result<[VexfsDirEntry; VEXFS_DIR_ENTRIES_PER_BLOCK], DirOpError> {
        // In a real implementation, this would:
        // 1. Read the block from disk
        // 2. Parse directory entries from the block data
        // 3. Return an array of parsed entries
        
        // For now, return empty entries
        Ok([VexfsDirEntry::new(); VEXFS_DIR_ENTRIES_PER_BLOCK])
    }
}

/// Insertion point for new directory entries
#[derive(Debug)]
struct InsertionPoint {
    pub block_num: u64,
    pub offset: u64,
    pub available_space: u16,
}

impl DirEntryIterator {
    /// Create a new directory entry iterator
    pub fn new(dir_handle: VexfsDirHandle) -> Self {
        let end_pos = dir_handle.inode.size();
        
        Self {
            dir_handle,
            pos: 0,
            end_pos,
        }
    }
    
    /// Get the next directory entry
    pub fn next(&mut self) -> Option<VexfsDirEntry> {
        if self.pos >= self.end_pos {
            return None;
        }
        
        // In a real implementation, this would read and parse the next entry
        // For now, return None to indicate end of iteration
        None
    }
    
    /// Reset iterator to beginning
    pub fn reset(&mut self) {
        self.pos = 0;
    }
    
    /// Seek to a specific position
    pub fn seek(&mut self, pos: u64) -> Result<(), DirOpError> {
        if pos > self.end_pos {
            return Err(DirOpError::InvalidName);
        }
        
        self.pos = pos;
        Ok(())
    }
}

impl VexfsDirEntry {
    /// Check if this entry matches a given name
    pub fn matches_name(&self, name: &str) -> bool {
        if self.name_len as usize != name.len() {
            return false;
        }
        
        // In a real implementation, we would compare the actual name data
        // For now, assume names don't match (since we don't have real data)
        false
    }
    
    /// Calculate the size needed for an entry with a given name length
    pub fn calc_size(name_len: u8) -> u16 {
        let base_size = core::mem::size_of::<VexfsDirEntry>() as u16;
        let name_size = name_len as u16;
        
        // Align to 4-byte boundary
        ((base_size + name_size + 3) / 4) * 4
    }
}

/// Directory statistics
#[derive(Debug, Clone, Copy)]
pub struct DirStats {
    pub total_entries: u32,
    pub total_size: u64,
    pub blocks_used: u32,
    pub free_space: u64,
}

impl VexfsDirOps {
    /// Get directory statistics
    pub fn get_dir_stats(&self, dir_inode: &VexfsInodeInfo) -> Result<DirStats, DirOpError> {
        if !dir_inode.is_dir() {
            return Err(DirOpError::NotDirectory);
        }
        
        let file_size = dir_inode.size();
        let blocks_used = ((file_size + self.block_size as u64 - 1) / self.block_size as u64) as u32;
        
        Ok(DirStats {
            total_entries: 0, // Would count entries in real implementation
            total_size: file_size,
            blocks_used,
            free_space: (blocks_used as u64 * self.block_size as u64) - file_size,
        })
    }
    
    /// Rename an entry within a directory
    pub fn rename(&mut self, old_dir: &mut VexfsInodeInfo, old_name: &str, new_dir: &mut VexfsInodeInfo, new_name: &str) -> Result<(), DirOpError> {
        // Check if source exists
        let lookup_result = self.lookup_entry(old_dir, old_name)?
            .ok_or(DirOpError::NotFound)?;
        
        // Check if destination already exists
        if self.lookup_entry(new_dir, new_name)?.is_some() {
            return Err(DirOpError::AlreadyExists);
        }
        
        // Add entry to new directory
        self.add_entry(new_dir, new_name, lookup_result.ino, lookup_result.file_type)?;
        
        // Remove entry from old directory
        self.remove_entry(old_dir, old_name)?;
        
        Ok(())
    }
}