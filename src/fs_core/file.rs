//! File Entity and Operations for VexFS
//! 
//! This module implements the File entity and all file-related operations
//! including creation, reading, writing, truncation, and deletion. It
//! integrates with the storage domain for persistence and block management.

use crate::shared::errors::VexfsError;
use crate::shared::types::{
    InodeNumber, FileSize, FileType, BlockNumber, Result, Timestamp
};
use crate::shared::constants::{VEXFS_BLOCK_SIZE, VEXFS_MAX_FILE_SIZE};
use crate::fs_core::inode::{Inode, get_inode, put_inode, create_inode, delete_inode};
use crate::fs_core::permissions::{UserContext, can_read, can_write, AccessMode};
use crate::fs_core::locking::{acquire_write_lock_guard, acquire_read_lock_guard, LockGuard};
use crate::storage::StorageManager;

#[cfg(not(feature = "std"))]
use alloc::{vec::Vec, string::String};
#[cfg(feature = "std")]
use std::{vec::Vec, string::String};

/// File handle for tracking open files
#[derive(Debug, Clone)]
pub struct FileHandle {
    /// File inode number
    pub inode_number: InodeNumber,
    /// File access mode
    pub access_mode: AccessMode,
    /// Current file position
    pub position: u64,
    /// File descriptor ID
    pub fd: u32,
    /// Flags for the file handle
    pub flags: u32,
}

impl FileHandle {
    /// Create a new file handle
    pub fn new(inode_number: InodeNumber, access_mode: AccessMode, fd: u32, flags: u32) -> Self {
        Self {
            inode_number,
            access_mode,
            position: 0,
            fd,
            flags,
        }
    }
    
    /// Check if the file is open for reading
    pub fn can_read(&self) -> bool {
        self.access_mode.read
    }
    
    /// Check if the file is open for writing
    pub fn can_write(&self) -> bool {
        self.access_mode.write
    }
    
    /// Seek to a new position
    pub fn seek(&mut self, position: u64) -> Result<u64> {
        self.position = position;
        Ok(self.position)
    }
    
    /// Get current position
    pub fn tell(&self) -> u64 {
        self.position
    }
}

/// File entity representing a regular file in the filesystem
#[derive(Debug, Clone)]
pub struct File {
    /// File inode
    pub inode: Inode,
    /// Whether the file is dirty (has uncommitted changes)
    pub dirty: bool,
}

impl File {
    /// Create a new file from an inode
    pub fn from_inode(inode: Inode) -> Self {
        Self {
            inode,
            dirty: false,
        }
    }
    
    /// Get the file size
    pub fn size(&self) -> FileSize {
        self.inode.size
    }
    
    /// Get the file inode number
    pub fn inode_number(&self) -> InodeNumber {
        self.inode.number
    }
    
    /// Check if this is a regular file
    pub fn is_regular_file(&self) -> bool {
        self.inode.is_regular_file()
    }
    
    /// Check if this is a vector file
    pub fn is_vector_file(&self) -> bool {
        self.inode.is_vector_file()
    }
    
    /// Mark the file as dirty
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
        self.inode.mark_dirty();
    }
    
    /// Sync the file to storage
    pub fn sync(&mut self) -> Result<()> {
        if self.dirty {
            put_inode(self.inode.clone())?;
            self.dirty = false;
        }
        Ok(())
    }
}

/// File Operations
/// 
/// This module provides all file-related operations including creation,
/// opening, reading, writing, and deletion of files.
pub struct FileOperations;

impl FileOperations {
    /// Create a new file
    /// 
    /// Creates a new regular file with the specified mode and ownership.
    /// 
    /// # Arguments
    /// 
    /// * `file_type` - Type of file to create (Regular or VectorFile)
    /// * `mode` - File permission mode
    /// * `user` - User context for permission checking
    /// 
    /// # Returns
    /// 
    /// Returns the created File entity or an error.
    pub fn create_file(file_type: FileType, mode: u32, user: &UserContext) -> Result<File> {
        // Validate file type
        match file_type {
            FileType::Regular | FileType::VectorFile => {}
            _ => return Err(VexfsError::InvalidArgument),
        }
        
        // Create the inode
        let mut inode = create_inode(file_type)?;
        
        // Set ownership and permissions
        inode.uid = user.uid;
        inode.gid = user.gid;
        inode.mode = mode & 0o7777;
        
        // Initialize file-specific fields
        inode.size = 0;
        inode.link_count = 1;
        
        let file = File::from_inode(inode);
        
        Ok(file)
    }
    
    /// Open an existing file
    /// 
    /// Opens a file for reading and/or writing based on the access mode.
    /// 
    /// # Arguments
    /// 
    /// * `inode_number` - Inode number of the file to open
    /// * `access_mode` - Requested access mode (read, write, or both)
    /// * `user` - User context for permission checking
    /// 
    /// # Returns
    /// 
    /// Returns a FileHandle for the opened file or an error.
    pub fn open_file(
        inode_number: InodeNumber, 
        access_mode: AccessMode, 
        user: &UserContext
    ) -> Result<FileHandle> {
        // Get the file inode
        let inode = get_inode(inode_number)?;
        
        // Check if it's a file
        if !inode.is_regular_file() && !inode.is_vector_file() {
            return Err(VexfsError::IsDirectory);
        }
        
        // Check permissions
        if access_mode.read && !can_read(&inode, user)? {
            return Err(VexfsError::PermissionDenied);
        }
        
        if access_mode.write && !can_write(&inode, user)? {
            return Err(VexfsError::PermissionDenied);
        }
        
        // Generate a file descriptor (placeholder implementation)
        let fd = Self::generate_fd();
        
        let handle = FileHandle::new(inode_number, access_mode, fd, 0);
        
        Ok(handle)
    }
    
    /// Read data from a file
    /// 
    /// Reads data from the file at the current position.
    /// 
    /// # Arguments
    /// 
    /// * `handle` - File handle for the open file
    /// * `buffer` - Buffer to read data into
    /// * `offset` - Optional offset to read from (if None, uses handle position)
    /// * `user` - User context for permission checking
    /// 
    /// # Returns
    /// 
    /// Returns the number of bytes read or an error.
    pub fn read_file(
        handle: &mut FileHandle,
        buffer: &mut [u8],
        offset: Option<u64>,
        user: &UserContext
    ) -> Result<usize> {
        // Check read permission
        if !handle.can_read() {
            return Err(VexfsError::PermissionDenied);
        }
        
        // Get the file inode
        let inode = get_inode(handle.inode_number)?;
        
        // Double-check read permission
        if !can_read(&inode, user)? {
            return Err(VexfsError::PermissionDenied);
        }
        
        // Acquire read lock
        let _lock = acquire_read_lock_guard(handle.inode_number)?;
        
        // Determine read position
        let read_position = offset.unwrap_or(handle.position);
        
        // Check bounds
        if read_position >= inode.size {
            return Ok(0); // EOF
        }
        
        // Calculate how much we can actually read
        let available = (inode.size - read_position) as usize;
        let to_read = buffer.len().min(available);
        
        if to_read == 0 {
            return Ok(0);
        }
        
        // Read the data (placeholder - would use storage manager)
        let bytes_read = Self::read_file_data(
            &inode, 
            read_position, 
            &mut buffer[..to_read]
        )?;
        
        // Update position if not using explicit offset
        if offset.is_none() {
            handle.position += bytes_read as u64;
        }
        
        Ok(bytes_read)
    }
    
    /// Write data to a file
    /// 
    /// Writes data to the file at the current position.
    /// 
    /// # Arguments
    /// 
    /// * `handle` - File handle for the open file
    /// * `data` - Data to write
    /// * `offset` - Optional offset to write to (if None, uses handle position)
    /// * `user` - User context for permission checking
    /// 
    /// # Returns
    /// 
    /// Returns the number of bytes written or an error.
    pub fn write_file(
        handle: &mut FileHandle,
        data: &[u8],
        offset: Option<u64>,
        user: &UserContext
    ) -> Result<usize> {
        // Check write permission
        if !handle.can_write() {
            return Err(VexfsError::PermissionDenied);
        }
        
        // Get the file inode
        let mut inode = get_inode(handle.inode_number)?;
        
        // Double-check write permission
        if !can_write(&inode, user)? {
            return Err(VexfsError::PermissionDenied);
        }
        
        // Acquire write lock
        let _lock = acquire_write_lock_guard(handle.inode_number)?;
        
        // Determine write position
        let write_position = offset.unwrap_or(handle.position);
        
        // Check file size limits
        let new_end = write_position + data.len() as u64;
        if new_end > VEXFS_MAX_FILE_SIZE {
            return Err(VexfsError::FileTooLarge);
        }
        
        // Write the data (placeholder - would use storage manager)
        let bytes_written = Self::write_file_data(
            &mut inode,
            write_position,
            data
        )?;
        
        // Update file size if necessary
        if write_position + bytes_written as u64 > inode.size {
            inode.size = write_position + bytes_written as u64;
            inode.mark_dirty();
        }
        
        // Update timestamps
        inode.mtime = crate::shared::utils::current_timestamp();
        inode.mark_dirty();
        
        // Save the updated inode
        put_inode(inode)?;
        
        // Update position if not using explicit offset
        if offset.is_none() {
            handle.position += bytes_written as u64;
        }
        
        Ok(bytes_written)
    }
    
    /// Truncate a file to the specified size
    /// 
    /// Changes the file size, either extending with zeros or shrinking.
    /// 
    /// # Arguments
    /// 
    /// * `inode_number` - Inode number of the file to truncate
    /// * `new_size` - New file size
    /// * `user` - User context for permission checking
    /// 
    /// # Returns
    /// 
    /// Returns Ok(()) on success or an error.
    pub fn truncate_file(
        inode_number: InodeNumber, 
        new_size: u64, 
        user: &UserContext
    ) -> Result<()> {
        // Get the file inode
        let mut inode = get_inode(inode_number)?;
        
        // Check if it's a file
        if !inode.is_regular_file() && !inode.is_vector_file() {
            return Err(VexfsError::IsDirectory);
        }
        
        // Check write permission
        if !can_write(&inode, user)? {
            return Err(VexfsError::PermissionDenied);
        }
        
        // Check size limits
        if new_size > VEXFS_MAX_FILE_SIZE {
            return Err(VexfsError::FileTooLarge);
        }
        
        // Acquire write lock
        let _lock = acquire_write_lock_guard(inode_number)?;
        
        // Perform the truncation (placeholder - would use storage manager)
        Self::truncate_file_data(&mut inode, new_size)?;
        
        // Update inode
        inode.size = new_size;
        inode.mtime = crate::shared::utils::current_timestamp();
        inode.mark_dirty();
        
        // Save the updated inode
        put_inode(inode)?;
        
        Ok(())
    }
    
    /// Delete a file
    /// 
    /// Removes a file from the filesystem, deallocating its blocks.
    /// 
    /// # Arguments
    /// 
    /// * `inode_number` - Inode number of the file to delete
    /// * `user` - User context for permission checking
    /// 
    /// # Returns
    /// 
    /// Returns Ok(()) on success or an error.
    pub fn delete_file(inode_number: InodeNumber, user: &UserContext) -> Result<()> {
        // Get the file inode
        let mut inode = get_inode(inode_number)?;
        
        // Check if it's a file
        if !inode.is_regular_file() && !inode.is_vector_file() {
            return Err(VexfsError::IsDirectory);
        }
        
        // Check write permission (or ownership)
        if !can_write(&inode, user)? && inode.uid != user.uid && !user.is_superuser {
            return Err(VexfsError::PermissionDenied);
        }
        
        // Acquire write lock
        let _lock = acquire_write_lock_guard(inode_number)?;
        
        // Decrease link count
        inode.link_count -= 1;
        
        if inode.link_count == 0 {
            // No more links, delete the file data
            Self::delete_file_data(&inode)?;
            
            // Delete the inode
            delete_inode(inode_number)?;
        } else {
            // Still has links, just update the inode
            inode.ctime = crate::shared::utils::current_timestamp();
            put_inode(inode)?;
        }
        
        Ok(())
    }
    
    /// Close a file handle
    /// 
    /// Closes an open file handle and releases associated resources.
    /// 
    /// # Arguments
    /// 
    /// * `handle` - File handle to close
    /// 
    /// # Returns
    /// 
    /// Returns Ok(()) on success or an error.
    pub fn close_file(handle: FileHandle) -> Result<()> {
        // In a real implementation, we would:
        // 1. Flush any pending writes
        // 2. Release file descriptor
        // 3. Clean up handle resources
        
        // For now, this is a no-op
        Ok(())
    }
    
    /// Generate a unique file descriptor ID (placeholder)
    fn generate_fd() -> u32 {
        // In a real implementation, this would be managed by a FD allocator
        static mut NEXT_FD: u32 = 3; // Start after stdin, stdout, stderr
        unsafe {
            let fd = NEXT_FD;
            NEXT_FD += 1;
            fd
        }
    }
    
    /// Read file data from storage (placeholder)
    fn read_file_data(_inode: &Inode, _offset: u64, _buffer: &mut [u8]) -> Result<usize> {
        // TODO: Implement using StorageManager and block reading
        Err(VexfsError::NotImplemented)
    }
    
    /// Write file data to storage (placeholder)
    fn write_file_data(_inode: &mut Inode, _offset: u64, _data: &[u8]) -> Result<usize> {
        // TODO: Implement using StorageManager and block writing
        Ok(_data.len()) // Placeholder: pretend we wrote everything
    }
    
    /// Truncate file data (placeholder)
    fn truncate_file_data(_inode: &mut Inode, _new_size: u64) -> Result<()> {
        // TODO: Implement using StorageManager
        // - If shrinking: deallocate blocks beyond new size
        // - If growing: allocate new blocks and zero them
        Ok(())
    }
    
    /// Delete file data (placeholder)
    fn delete_file_data(_inode: &Inode) -> Result<()> {
        // TODO: Implement using StorageManager
        // - Deallocate all data blocks
        // - Deallocate indirect blocks
        // - Clean up vector index blocks if vector file
        Ok(())
    }
}

// Public API functions for file operations

/// Create a new file
pub fn create_file(file_type: FileType, mode: u32, user: &UserContext) -> Result<File> {
    FileOperations::create_file(file_type, mode, user)
}

/// Open an existing file
pub fn open_file(
    inode_number: InodeNumber, 
    access_mode: AccessMode, 
    user: &UserContext
) -> Result<FileHandle> {
    FileOperations::open_file(inode_number, access_mode, user)
}

/// Read from a file
pub fn read_file(
    handle: &mut FileHandle,
    buffer: &mut [u8],
    offset: Option<u64>,
    user: &UserContext
) -> Result<usize> {
    FileOperations::read_file(handle, buffer, offset, user)
}

/// Write to a file
pub fn write_file(
    handle: &mut FileHandle,
    data: &[u8],
    offset: Option<u64>,
    user: &UserContext
) -> Result<usize> {
    FileOperations::write_file(handle, data, offset, user)
}

/// Truncate a file
pub fn truncate_file(inode_number: InodeNumber, new_size: u64, user: &UserContext) -> Result<()> {
    FileOperations::truncate_file(inode_number, new_size, user)
}

/// Delete a file
pub fn delete_file(inode_number: InodeNumber, user: &UserContext) -> Result<()> {
    FileOperations::delete_file(inode_number, user)
}

/// Close a file handle
pub fn close_file(handle: FileHandle) -> Result<()> {
    FileOperations::close_file(handle)
}