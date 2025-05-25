//! File Operations Implementation for VexFS
//! 
//! This module implements the core file operations including read, write, 
//! truncate, and attribute manipulation for VexFS files.



use crate::ondisk::*;
use crate::inode_mgmt::*;

/// File operations error types
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FileOpError {
    InvalidOffset,
    InvalidSize,
    NoSpace,
    IoError,
    PermissionDenied,
    NotFound,
    IsDirectory,
    NotRegularFile,
    ReadOnly,
    FileTooLarge,
}

/// File handle for managing open files
#[derive(Debug)]
pub struct VexfsFileHandle {
    /// Associated inode
    pub inode: VexfsInodeInfo,
    
    /// Current file position
    pub pos: u64,
    
    /// Open flags (O_RDONLY, O_WRONLY, O_RDWR, etc.)
    pub flags: u32,
    
    /// Reference count
    pub ref_count: u32,
    
    /// Dirty flag for write operations
    pub dirty: bool,
}

/// File I/O operations manager
pub struct VexfsFileOps {
    /// Block size for I/O operations
    pub block_size: u32,
    
    /// Maximum file size
    pub max_file_size: u64,
}

/// Buffer for file I/O operations
#[derive(Debug)]
pub struct IoBuffer {
    /// Buffer data
    pub data: *mut u8,
    
    /// Buffer size
    pub size: usize,
    
    /// Current position in buffer
    pub pos: usize,
}

impl VexfsFileHandle {
    /// Create a new file handle
    pub fn new(inode: VexfsInodeInfo, flags: u32) -> Self {
        Self {
            inode,
            pos: 0,
            flags,
            ref_count: 1,
            dirty: false,
        }
    }
    
    /// Get current file position
    pub fn position(&self) -> u64 {
        self.pos
    }
    
    /// Seek to a specific position
    pub fn seek(&mut self, pos: u64, whence: u32) -> Result<u64, FileOpError> {
        let new_pos = match whence {
            SEEK_SET => pos,
            SEEK_CUR => {
                if pos > u64::MAX - self.pos {
                    return Err(FileOpError::InvalidOffset);
                }
                self.pos + pos
            },
            SEEK_END => {
                let file_size = self.inode.size();
                if pos > file_size {
                    return Err(FileOpError::InvalidOffset);
                }
                file_size - pos
            },
            _ => return Err(FileOpError::InvalidOffset),
        };
        
        self.pos = new_pos;
        Ok(new_pos)
    }
    
    /// Check if file is open for reading
    pub fn can_read(&self) -> bool {
        (self.flags & O_ACCMODE) != O_WRONLY
    }
    
    /// Check if file is open for writing
    pub fn can_write(&self) -> bool {
        (self.flags & O_ACCMODE) != O_RDONLY
    }
    
    /// Check if file should be appended to
    pub fn is_append(&self) -> bool {
        (self.flags & O_APPEND) != 0
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

impl VexfsFileOps {
    /// Create a new file operations manager
    pub fn new(block_size: u32) -> Self {
        Self {
            block_size,
            max_file_size: VEXFS_MAX_FILE_SIZE,
        }
    }
    
    /// Open a file and create a file handle
    pub fn open(&self, inode: VexfsInodeInfo, flags: u32) -> Result<VexfsFileHandle, FileOpError> {
        // Check if it's a regular file
        if !inode.is_file() {
            return Err(FileOpError::NotRegularFile);
        }
        
        // Check permissions based on flags
        let uid = 0; // In real implementation, get from current task
        let gid = 0; // In real implementation, get from current task
        
        let required_perms = match flags & O_ACCMODE {
            O_RDONLY => 0o4, // Read permission
            O_WRONLY => 0o2, // Write permission
            O_RDWR => 0o6,   // Read and write permission
            _ => return Err(FileOpError::PermissionDenied),
        };
        
        if !inode.check_permission(uid, gid, required_perms) {
            return Err(FileOpError::PermissionDenied);
        }
        
        let mut handle = VexfsFileHandle::new(inode, flags);
        
        // If O_APPEND, seek to end
        if handle.is_append() {
            handle.pos = handle.inode.size();
        }
        
        // If O_TRUNC and writable, truncate file
        if (flags & O_TRUNC) != 0 && handle.can_write() {
            self.truncate(&mut handle, 0)?;
        }
        
        Ok(handle)
    }
    
    /// Read data from a file
    pub fn read(&self, handle: &mut VexfsFileHandle, buffer: &mut IoBuffer, count: usize) -> Result<usize, FileOpError> {
        if !handle.can_read() {
            return Err(FileOpError::PermissionDenied);
        }
        
        let file_size = handle.inode.size();
        if handle.pos >= file_size {
            return Ok(0); // EOF
        }
        
        // Calculate how much we can actually read
        let remaining = (file_size - handle.pos) as usize;
        let to_read = core::cmp::min(count, remaining);
        let to_read = core::cmp::min(to_read, buffer.size - buffer.pos);
        
        if to_read == 0 {
            return Ok(0);
        }
        
        // Read data block by block
        let mut bytes_read = 0;
        let mut current_pos = handle.pos;
        
        while bytes_read < to_read {
            let block_offset = current_pos / self.block_size as u64;
            let offset_in_block = (current_pos % self.block_size as u64) as usize;
            
            // Get the block number for this file offset
            let block_num = handle.inode.get_block(block_offset);
            
            if block_num.is_none() {
                // Sparse file - return zeros
                let bytes_in_block = core::cmp::min(
                    to_read - bytes_read,
                    self.block_size as usize - offset_in_block
                );
                
                // In real implementation, we would zero the buffer
                // For now, just update counters
                bytes_read += bytes_in_block;
                current_pos += bytes_in_block as u64;
                buffer.pos += bytes_in_block;
                continue;
            }
            
            // Read from the actual block
            let bytes_in_block = core::cmp::min(
                to_read - bytes_read,
                self.block_size as usize - offset_in_block
            );
            
            // In a real implementation, we would:
            // 1. Read the block from disk
            // 2. Copy the relevant portion to the user buffer
            // For now, just update counters
            
            bytes_read += bytes_in_block;
            current_pos += bytes_in_block as u64;
            buffer.pos += bytes_in_block;
        }
        
        // Update file position and access time
        handle.pos = current_pos;
        handle.inode.touch_atime(0); // In real implementation, use current time
        
        Ok(bytes_read)
    }
    
    /// Write data to a file
    pub fn write(&self, handle: &mut VexfsFileHandle, buffer: &IoBuffer, count: usize) -> Result<usize, FileOpError> {
        if !handle.can_write() {
            return Err(FileOpError::PermissionDenied);
        }
        
        if count == 0 {
            return Ok(0);
        }
        
        // Check if we would exceed maximum file size
        if handle.pos > self.max_file_size || 
           count > (self.max_file_size - handle.pos) as usize {
            return Err(FileOpError::FileTooLarge);
        }
        
        let available_buffer = core::cmp::min(count, buffer.size - buffer.pos);
        if available_buffer == 0 {
            return Ok(0);
        }
        
        // If append mode, always write at end
        if handle.is_append() {
            handle.pos = handle.inode.size();
        }
        
        // Write data block by block
        let mut bytes_written = 0;
        let mut current_pos = handle.pos;
        
        while bytes_written < available_buffer {
            let block_offset = current_pos / self.block_size as u64;
            let offset_in_block = (current_pos % self.block_size as u64) as usize;
            
            // Get or allocate block for this file offset
            let block_num = handle.inode.get_block(block_offset);
            
            if block_num.is_none() {
                // Need to allocate a new block
                // In real implementation, we would call the space allocator
                let new_block = self.allocate_block()?;
                handle.inode.set_block(block_offset, new_block)?;
            }
            
            // Write to the block
            let bytes_in_block = core::cmp::min(
                available_buffer - bytes_written,
                self.block_size as usize - offset_in_block
            );
            
            // In a real implementation, we would:
            // 1. Read the block from disk (if partial write)
            // 2. Modify the relevant portion
            // 3. Write the block back to disk
            // For now, just update counters
            
            bytes_written += bytes_in_block;
            current_pos += bytes_in_block as u64;
        }
        
        // Update file position and size
        handle.pos = current_pos;
        if current_pos > handle.inode.size() {
            handle.inode.set_size(current_pos);
        }
        
        // Update modification time
        handle.inode.touch_mtime(0); // In real implementation, use current time
        handle.inode.touch_ctime(0);
        handle.dirty = true;
        
        Ok(bytes_written)
    }
    
    /// Truncate a file to a specific size
    pub fn truncate(&self, handle: &mut VexfsFileHandle, size: u64) -> Result<(), FileOpError> {
        if !handle.can_write() {
            return Err(FileOpError::PermissionDenied);
        }
        
        if size > self.max_file_size {
            return Err(FileOpError::FileTooLarge);
        }
        
        let current_size = handle.inode.size();
        
        if size == current_size {
            return Ok(()); // No change needed
        }
        
        if size < current_size {
            // Truncating - need to free blocks beyond the new size
            self.free_blocks_beyond(handle, size)?;
        }
        
        // Update file size
        handle.inode.set_size(size);
        
        // If current position is beyond new size, adjust it
        if handle.pos > size {
            handle.pos = size;
        }
        
        // Update modification time
        handle.inode.touch_mtime(0); // In real implementation, use current time
        handle.inode.touch_ctime(0);
        handle.dirty = true;
        
        Ok(())
    }
    
    /// Sync file data to disk
    pub fn sync(&self, handle: &mut VexfsFileHandle) -> Result<(), FileOpError> {
        if handle.dirty {
            // In a real implementation, we would:
            // 1. Write all dirty data blocks to disk
            // 2. Update the inode on disk
            // 3. Sync the device
            
            handle.dirty = false;
        }
        
        Ok(())
    }
    
    /// Get file attributes
    pub fn getattr(&self, handle: &VexfsFileHandle) -> VexfsFileAttrs {
        VexfsFileAttrs {
            size: handle.inode.size(),
            mode: handle.inode.disk_inode.i_mode,
            uid: handle.inode.get_uid(),
            gid: handle.inode.get_gid(),
            atime: handle.inode.disk_inode.i_atime,
            mtime: handle.inode.disk_inode.i_mtime,
            ctime: handle.inode.disk_inode.i_ctime,
            blocks: handle.inode.calculate_blocks(),
            block_size: self.block_size,
        }
    }
    
    /// Set file attributes
    pub fn setattr(&self, handle: &mut VexfsFileHandle, attrs: &VexfsFileAttrs, valid: u32) -> Result<(), FileOpError> {
        if !handle.can_write() && (valid & (ATTR_SIZE | ATTR_MODE | ATTR_UID | ATTR_GID)) != 0 {
            return Err(FileOpError::PermissionDenied);
        }
        
        if (valid & ATTR_SIZE) != 0 {
            self.truncate(handle, attrs.size)?;
        }
        
        if (valid & ATTR_MODE) != 0 {
            handle.inode.disk_inode.i_mode = (handle.inode.disk_inode.i_mode & S_IFMT) | (attrs.mode & !S_IFMT);
            handle.inode.mark_dirty();
        }
        
        if (valid & ATTR_UID) != 0 {
            handle.inode.set_uid(attrs.uid);
        }
        
        if (valid & ATTR_GID) != 0 {
            handle.inode.set_gid(attrs.gid);
        }
        
        if (valid & ATTR_ATIME) != 0 {
            handle.inode.touch_atime(attrs.atime);
        }
        
        if (valid & ATTR_MTIME) != 0 {
            handle.inode.touch_mtime(attrs.mtime);
        }
        
        // Always update ctime when any attribute changes
        handle.inode.touch_ctime(0); // In real implementation, use current time
        handle.dirty = true;
        
        Ok(())
    }
    
    /// Allocate a new data block (stub implementation)
    fn allocate_block(&self) -> Result<u64, FileOpError> {
        // In a real implementation, this would call the space allocator
        // For now, return a dummy block number
        Ok(1000) // Placeholder
    }
    
    /// Free blocks beyond a certain file offset
    fn free_blocks_beyond(&self, handle: &mut VexfsFileHandle, size: u64) -> Result<(), FileOpError> {
        let blocks_needed = (size + self.block_size as u64 - 1) / self.block_size as u64;
        
        // Free direct blocks beyond the needed amount
        for i in blocks_needed..(VEXFS_N_DIRECT as u64) {
            if let Some(block_num) = handle.inode.get_block(i) {
                if block_num != 0 {
                    // In real implementation, free the block
                    handle.inode.set_block(i, 0)?;
                }
            }
        }
        
        // In a real implementation, we would also handle indirect blocks
        
        Ok(())
    }
}

/// File attribute structure
#[derive(Debug, Clone, Copy)]
pub struct VexfsFileAttrs {
    pub size: u64,
    pub mode: u16,
    pub uid: u32,
    pub gid: u32,
    pub atime: u32,
    pub mtime: u32,
    pub ctime: u32,
    pub blocks: u32,
    pub block_size: u32,
}

// File operation constants
pub const O_ACCMODE: u32 = 0o3;
pub const O_RDONLY: u32 = 0o0;
pub const O_WRONLY: u32 = 0o1;
pub const O_RDWR: u32 = 0o2;
pub const O_CREAT: u32 = 0o100;
pub const O_EXCL: u32 = 0o200;
pub const O_TRUNC: u32 = 0o1000;
pub const O_APPEND: u32 = 0o2000;

// Seek constants
pub const SEEK_SET: u32 = 0;
pub const SEEK_CUR: u32 = 1;
pub const SEEK_END: u32 = 2;

// Attribute validity flags
pub const ATTR_MODE: u32 = 1;
pub const ATTR_UID: u32 = 2;
pub const ATTR_GID: u32 = 4;
pub const ATTR_SIZE: u32 = 8;
pub const ATTR_ATIME: u32 = 16;
pub const ATTR_MTIME: u32 = 32;
pub const ATTR_CTIME: u32 = 64;

impl IoBuffer {
    /// Create a new I/O buffer
    pub fn new(data: *mut u8, size: usize) -> Self {
        Self {
            data,
            size,
            pos: 0,
        }
    }
    
    /// Reset buffer position
    pub fn reset(&mut self) {
        self.pos = 0;
    }
    
    /// Get remaining space in buffer
    pub fn remaining(&self) -> usize {
        self.size - self.pos
    }
    
    /// Check if buffer is full
    pub fn is_full(&self) -> bool {
        self.pos >= self.size
    }
    
    /// Check if buffer is empty
    pub fn is_empty(&self) -> bool {
        self.pos == 0
    }
}

/// File range for partial operations
#[derive(Debug, Clone, Copy)]
pub struct FileRange {
    pub start: u64,
    pub end: u64,
}

impl FileRange {
    /// Create a new file range
    pub fn new(start: u64, end: u64) -> Self {
        Self { start, end }
    }
    
    /// Get the length of the range
    pub fn len(&self) -> u64 {
        if self.end >= self.start {
            self.end - self.start
        } else {
            0
        }
    }
    
    /// Check if range is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    
    /// Check if range contains an offset
    pub fn contains(&self, offset: u64) -> bool {
        offset >= self.start && offset < self.end
    }
    
    /// Get intersection with another range
    pub fn intersect(&self, other: &FileRange) -> Option<FileRange> {
        let start = core::cmp::max(self.start, other.start);
        let end = core::cmp::min(self.end, other.end);
        
        if start < end {
            Some(FileRange::new(start, end))
        } else {
            None
        }
    }
}

/// File lock operations (for future implementation)
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LockType {
    Read,
    Write,
    Unlock,
}

/// File lock structure (for future implementation)
#[derive(Debug)]
pub struct FileLock {
    pub lock_type: LockType,
    pub range: FileRange,
    pub pid: u32,
}

impl VexfsFileOps {
    /// Punch a hole in a file (for future sparse file support)
    pub fn punch_hole(&self, handle: &mut VexfsFileHandle, offset: u64, len: u64) -> Result<(), FileOpError> {
        if !handle.can_write() {
            return Err(FileOpError::PermissionDenied);
        }
        
        let file_size = handle.inode.size();
        if offset >= file_size {
            return Ok(()); // Nothing to punch
        }
        
        let end = core::cmp::min(offset + len, file_size);
        let range = FileRange::new(offset, end);
        
        // In a real implementation, we would:
        // 1. Free blocks in the specified range
        // 2. Mark them as holes (sparse)
        // 3. Update metadata
        
        handle.inode.touch_mtime(0);
        handle.inode.touch_ctime(0);
        handle.dirty = true;
        
        Ok(())
    }
    
    /// Preallocate space for a file (for future implementation)
    pub fn fallocate(&self, handle: &mut VexfsFileHandle, offset: u64, len: u64) -> Result<(), FileOpError> {
        if !handle.can_write() {
            return Err(FileOpError::PermissionDenied);
        }
        
        let new_size = offset + len;
        if new_size > self.max_file_size {
            return Err(FileOpError::FileTooLarge);
        }
        
        // In a real implementation, we would:
        // 1. Allocate blocks for the specified range
        // 2. Don't initialize them (unwritten extents)
        // 3. Update file size if necessary
        
        if new_size > handle.inode.size() {
            handle.inode.set_size(new_size);
        }
        
        handle.inode.touch_ctime(0);
        handle.dirty = true;
        
        Ok(())
    }
}