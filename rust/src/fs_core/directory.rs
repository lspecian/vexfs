//! Directory Entity and Operations for VexFS
//! 
//! This module implements the Directory entity and all directory-related operations
//! including creation, reading, listing, and management. It handles directory
//! entries and integrates with the storage domain for persistence.

use crate::shared::errors::VexfsError;
use crate::shared::types::{
    InodeNumber, FileType, Result, FileMode
};
use crate::shared::constants::{VEXFS_ROOT_INO, VEXFS_MAX_NAME_LENGTH};
use crate::fs_core::inode::{Inode, InodeManager, get_inode, put_inode, create_inode, delete_inode};
use crate::fs_core::permissions::{
    UserContext, can_access_directory, can_list_directory,
    can_create_in_directory, can_delete_from_directory,
    check_read_permission, check_create_permission, check_delete_permission
};
use crate::fs_core::locking::{
    acquire_read_lock_guard, acquire_write_lock_guard
};
use crate::fs_core::operations::OperationContext;

#[cfg(not(feature = "kernel"))]
use std::sync::Arc;
#[cfg(feature = "kernel")]
use alloc::sync::Arc;

#[cfg(not(feature = "std"))]
use alloc::{string::ToString, vec::Vec, string::String, format};
#[cfg(feature = "std")]
#[cfg(feature = "kernel")]
use alloc::{string::ToString, vec::Vec, string::String, format};
#[cfg(not(feature = "kernel"))]
use std::{vec::Vec, string::String, format};

/// Directory entry structure
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectoryEntry {
    /// Inode number of the entry
    pub inode_number: InodeNumber,
    /// File type
    pub file_type: FileType,
    /// Entry name
    pub name: String,
    /// Entry name length (for on-disk compatibility)
    pub name_len: u8,
}

impl DirectoryEntry {
    /// Create a new directory entry
    pub fn new(inode_number: InodeNumber, file_type: FileType, name: String) -> Result<Self> {
        if name.len() > VEXFS_MAX_NAME_LENGTH {
            return Err(VexfsError::NameTooLong);
        }
        
        if name.is_empty() {
            return Err(VexfsError::InvalidArgument("invalid directory entry".to_string()));
        }
        
        // Check for invalid characters
        if name.contains('\0') || name.contains('/') {
            return Err(VexfsError::InvalidArgument("invalid characters in name".to_string()));
        }
        
        Ok(Self {
            inode_number,
            file_type,
            name_len: name.len() as u8,
            name,
        })
    }
    
    /// Create a "." entry (current directory)
    pub fn current_dir(inode_number: InodeNumber) -> Self {
        Self {
            inode_number,
            file_type: FileType::Directory,
            name: ".".to_string(),
            name_len: 1,
        }
    }
    
    /// Create a ".." entry (parent directory)
    pub fn parent_dir(parent_inode: InodeNumber) -> Self {
        Self {
            inode_number: parent_inode,
            file_type: FileType::Directory,
            name: "..".to_string(),
            name_len: 2,
        }
    }
    
    /// Check if this is the current directory entry
    pub fn is_current(&self) -> bool {
        self.name == "."
    }
    
    /// Check if this is the parent directory entry
    pub fn is_parent(&self) -> bool {
        self.name == ".."
    }
    
    /// Check if this is a special entry (. or ..)
    pub fn is_special(&self) -> bool {
        self.is_current() || self.is_parent()
    }
}

/// Directory entity representing a directory in the filesystem
#[derive(Debug, Clone)]
pub struct Directory {
    /// Directory inode
    pub inode: Inode,
    /// Directory entries (cached)
    pub entries: Vec<DirectoryEntry>,
    /// Whether the directory is dirty (has uncommitted changes)
    pub dirty: bool,
    /// Whether entries are loaded
    pub entries_loaded: bool,
}

impl Directory {
    /// Create a new directory from an inode
    pub fn from_inode(inode: Inode) -> Self {
        Self {
            inode,
            entries: Vec::new(),
            dirty: false,
            entries_loaded: false,
        }
    }
    
    /// Get the directory inode number
    pub fn inode_number(&self) -> InodeNumber {
        self.inode.ino
    }
    
    /// Check if this is a directory
    pub fn is_directory(&self) -> bool {
        self.inode.is_directory()
    }
    
    /// Get the number of entries
    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }
    
    /// Check if the directory is empty (only has . and .. entries)
    pub fn is_empty(&self) -> bool {
        self.entries.len() <= 2
    }
    
    /// Mark the directory as dirty
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
        self.inode.mark_dirty();
    }
    
    /// Load entries from storage if not already loaded
    pub fn ensure_entries_loaded(&mut self) -> Result<()> {
        if !self.entries_loaded {
            self.load_entries()?;
        }
        Ok(())
    }
    
    /// Load directory entries from storage (placeholder)
    fn load_entries(&mut self) -> Result<()> {
        // TODO: Implement using StorageManager to read directory blocks
        // For now, we'll create minimal entries for a new directory
        self.entries = Vec::new();
        
        // Add . and .. entries if they don't exist
        if self.entries.is_empty() {
            self.entries.push(DirectoryEntry::current_dir(self.inode.ino));
            // For root directory, .. points to itself
            let parent_inode = if self.inode.ino == VEXFS_ROOT_INO {
                VEXFS_ROOT_INO
            } else {
                // TODO: Get actual parent inode from storage
                VEXFS_ROOT_INO // Placeholder
            };
            self.entries.push(DirectoryEntry::parent_dir(parent_inode));
        }
        
        self.entries_loaded = true;
        Ok(())
    }
    
    /// Sync the directory to storage
    pub fn sync(&mut self) -> Result<()> {
        if self.dirty {
            // TODO: Write entries to storage using StorageManager
            // put_inode requires inode_manager parameter - this is a placeholder
            // put_inode(inode_manager, Arc::new(self.inode.clone()))?;
            self.dirty = false;
        }
        Ok(())
    }
    
    /// Find an entry by name
    pub fn find_entry(&mut self, name: &str) -> Result<Option<&DirectoryEntry>> {
        self.ensure_entries_loaded()?;
        Ok(self.entries.iter().find(|entry| entry.name == name))
    }
    
    /// Add a new entry to the directory
    pub fn add_entry(&mut self, entry: DirectoryEntry) -> Result<()> {
        self.ensure_entries_loaded()?;
        
        // Check if entry already exists
        if self.entries.iter().any(|e| e.name == entry.name) {
            return Err(VexfsError::FileExists);
        }
        
        // Add the entry
        self.entries.push(entry);
        self.mark_dirty();
        
        Ok(())
    }
    
    /// Remove an entry from the directory
    pub fn remove_entry(&mut self, name: &str) -> Result<DirectoryEntry> {
        self.ensure_entries_loaded()?;
        
        // Don't allow removal of . or ..
        if name == "." || name == ".." {
            return Err(VexfsError::InvalidArgument("cannot remove . or ..".to_string()));
        }
        
        // Find and remove the entry
        let position = self.entries.iter()
            .position(|entry| entry.name == name)
            .ok_or(VexfsError::NotFound)?;
        
        let removed = self.entries.remove(position);
        self.mark_dirty();
        
        Ok(removed)
    }
    
    /// List all entries in the directory
    pub fn list_entries(&mut self) -> Result<&Vec<DirectoryEntry>> {
        self.ensure_entries_loaded()?;
        Ok(&self.entries)
    }
}

/// Directory Operations
/// 
/// This module provides all directory-related operations including creation,
/// listing, entry management, and directory tree operations.
pub struct DirectoryOperations;

impl DirectoryOperations {
    /// Create a new directory
    ///
    /// Creates a new directory with the specified mode and ownership.
    ///
    /// # Arguments
    ///
    /// * `parent_inode` - Inode number of the parent directory
    /// * `name` - Name of the new directory
    /// * `mode` - Directory permission mode
    /// * `context` - Operation context containing managers and user info
    ///
    /// # Returns
    ///
    /// Returns the created Directory entity or an error.
    pub fn create_directory(
        parent_inode: InodeNumber,
        name: &str,
        mode: u32,
        context: &mut OperationContext
    ) -> Result<Directory> {
        // Validate name
        if name.is_empty() || name.len() > VEXFS_MAX_NAME_LENGTH {
            return Err(VexfsError::InvalidArgument("invalid name length".to_string()));
        }
        
        if name == "." || name == ".." {
            return Err(VexfsError::InvalidArgument("cannot create . or .. directories".to_string()));
        }
        
        // Get parent directory
        let parent_dir_inode = get_inode(context.inode_manager, parent_inode)?;
        
        // Check if parent is actually a directory
        if !parent_dir_inode.is_directory() {
            return Err(VexfsError::NotDirectory);
        }
        
        // Check permission to create in parent directory
        check_create_permission(&parent_dir_inode, &context.user)?;
        
        // Acquire write lock on parent directory
        let _parent_lock = acquire_write_lock_guard(context.lock_manager, parent_inode)?;
        
        // Load parent directory and check if name already exists
        let mut parent_dir = Directory::from_inode(parent_dir_inode.as_ref().clone());
        if parent_dir.find_entry(name)?.is_some() {
            return Err(VexfsError::FileExists);
        }
        
        // Create the new directory inode
        let dir_inode_arc = create_inode(context.inode_manager, FileType::Directory, FileMode::new(mode), context.user.uid, context.user.gid)?;
        let mut dir_inode = (*dir_inode_arc).clone();
        
        // Set link count (. and parent's entry)
        dir_inode.nlink = 2;
        
        let dir_inode_number = dir_inode.ino;
        
        // Create the directory entity
        let mut directory = Directory::from_inode((*dir_inode_arc).clone());
        
        // Add . and .. entries
        directory.add_entry(DirectoryEntry::current_dir(dir_inode_number))?;
        directory.add_entry(DirectoryEntry::parent_dir(parent_inode))?;
        
        // Add entry to parent directory
        let dir_entry = DirectoryEntry::new(
            dir_inode_number,
            FileType::Directory,
            name.to_string()
        )?;
        parent_dir.add_entry(dir_entry)?;
        
        // Update parent directory link count (for the .. entry in new dir)
        let mut updated_parent = parent_dir.inode.clone();
        updated_parent.nlink += 1;
        updated_parent.mtime = crate::shared::utils::current_timestamp();
        updated_parent.mark_dirty();
        
        // Save both directories
        directory.sync()?;
        parent_dir.sync()?;
        put_inode(context.inode_manager, Arc::new(updated_parent))?;
        
        Ok(directory)
    }
    
    /// Read directory entries
    /// 
    /// Lists all entries in a directory.
    /// 
    /// # Arguments
    /// 
    /// * `inode_number` - Inode number of the directory to read
    /// * `user` - User context for permission checking
    /// 
    /// # Returns
    /// 
    /// Returns a vector of directory entries or an error.
    pub fn read_directory(
        context: &mut OperationContext,
        inode_number: InodeNumber,
    ) -> Result<Vec<DirectoryEntry>> {
        // Get the directory inode
        let dir_inode = get_inode(context.inode_manager, inode_number)?;
        
        // Check if it's actually a directory
        if !dir_inode.is_directory() {
            return Err(VexfsError::NotDirectory);
        }
        
        // Check read permission
        if !can_list_directory(&dir_inode, &context.user) {
            return Err(VexfsError::PermissionDenied("Permission denied".to_string()));
        }
        
        // Acquire read lock
        let _lock = acquire_read_lock_guard(context.lock_manager, inode_number)?;
        
        // Load and return entries
        let mut directory = Directory::from_inode(dir_inode.as_ref().clone());
        let entries = directory.list_entries()?;
        
        Ok(entries.clone())
    }
    
    /// Look up an entry in a directory
    /// 
    /// Finds a specific entry by name in a directory.
    /// 
    /// # Arguments
    /// 
    /// * `dir_inode` - Inode number of the directory to search
    /// * `name` - Name of the entry to find
    /// * `user` - User context for permission checking
    /// 
    /// # Returns
    /// 
    /// Returns the directory entry if found, or an error.
    pub fn lookup_entry(
        dir_inode: InodeNumber,
        name: &str,
        context: &mut OperationContext
    ) -> Result<DirectoryEntry> {
        // Get the directory inode
        let inode = get_inode(context.inode_manager,dir_inode)?;
        
        // Check if it's actually a directory
        if !inode.is_directory() {
            return Err(VexfsError::NotDirectory);
        }
        
        // Check access permission (need execute to search directory)
        if !can_access_directory(&inode, &context.user) {
            return Err(VexfsError::PermissionDenied("Permission denied".to_string()));
        }
        
        // Acquire read lock
        let _lock = acquire_read_lock_guard(context.lock_manager, dir_inode)?;
        
        // Load directory and search for entry
        let mut directory = Directory::from_inode((*inode).clone());
        
        match directory.find_entry(name)? {
            Some(entry) => Ok(entry.clone()),
            None => Err(VexfsError::NotFound),
        }
    }
    
    /// Rename/move an entry within directories
    /// 
    /// Moves an entry from one directory to another, potentially with a new name.
    /// 
    /// # Arguments
    /// 
    /// * `old_dir_inode` - Source directory inode
    /// * `old_name` - Current name of the entry
    /// * `new_dir_inode` - Destination directory inode
    /// * `new_name` - New name for the entry
    /// * `user` - User context for permission checking
    /// 
    /// # Returns
    /// 
    /// Returns Ok(()) on success or an error.
    pub fn rename_entry(
        old_dir_inode: InodeNumber,
        old_name: &str,
        new_dir_inode: InodeNumber,
        new_name: &str,
        context: &mut OperationContext
    ) -> Result<()> {
        // Validate names
        if old_name == "." || old_name == ".." || new_name == "." || new_name == ".." {
            return Err(VexfsError::InvalidArgument("cannot rename . or .. entries".to_string()));
        }
        
        // Get directory inodes
        let old_dir = get_inode(context.inode_manager,old_dir_inode)?;
        let new_dir = get_inode(context.inode_manager,new_dir_inode)?;
        
        // Check if both are directories
        if !old_dir.is_directory() || !new_dir.is_directory() {
            return Err(VexfsError::NotDirectory);
        }
        
        // Acquire lock on the first directory (simplified approach to avoid multiple borrows)
        // TODO: Implement proper dual-lock mechanism for cross-directory operations
        let _lock1 = acquire_write_lock_guard(context.lock_manager, old_dir_inode)?;
        
        // Load directories
        let mut old_directory = Directory::from_inode((*old_dir).clone());
        let mut new_directory = if old_dir_inode == new_dir_inode {
            old_directory.clone()
        } else {
            Directory::from_inode((*new_dir).clone())
        };
        
        // Find the entry to move
        let entry = old_directory.find_entry(old_name)?
            .ok_or(VexfsError::NotFound)?
            .clone();
        
        // Get the target inode for permission checking
        let target_inode = get_inode(context.inode_manager,entry.inode_number)?;
        
        // Check permissions
        if !can_delete_from_directory(&old_dir, &context.user) {
            return Err(VexfsError::PermissionDenied("Permission denied".to_string()));
        }
        
        if !can_create_in_directory(&new_dir, &context.user) {
            return Err(VexfsError::PermissionDenied("Permission denied".to_string()));
        }
        
        // Check if destination already exists
        if new_directory.find_entry(new_name)?.is_some() {
            return Err(VexfsError::FileExists);
        }
        
        // Remove from old directory
        old_directory.remove_entry(old_name)?;
        
        // Create new entry with new name
        let new_entry = DirectoryEntry::new(
            entry.inode_number,
            entry.file_type,
            new_name.to_string()
        )?;
        
        // Add to new directory
        new_directory.add_entry(new_entry)?;
        
        // Update timestamps
        let now = crate::shared::utils::current_timestamp();
        old_directory.inode.mtime = now;
        old_directory.inode.ctime = now;
        new_directory.inode.mtime = now;
        new_directory.inode.ctime = now;
        
        // Save directories
        old_directory.sync()?;
        if old_dir_inode != new_dir_inode {
            new_directory.sync()?;
        }
        
        Ok(())
    }
    
    /// Delete a directory
    /// 
    /// Removes an empty directory from the filesystem.
    /// 
    /// # Arguments
    /// 
    /// * `parent_inode` - Parent directory inode
    /// * `name` - Name of the directory to delete
    /// * `user` - User context for permission checking
    /// 
    /// # Returns
    /// 
    /// Returns Ok(()) on success or an error.
    pub fn delete_directory(
        parent_inode: InodeNumber,
        name: &str,
        context: &mut OperationContext
    ) -> Result<()> {
        // Validate name
        if name == "." || name == ".." {
            return Err(VexfsError::InvalidArgument("cannot rename . or .. entries".to_string()));
        }
        
        // Get parent directory
        let parent_dir_inode = get_inode(context.inode_manager,parent_inode)?;
        
        // Check if parent is actually a directory
        if !parent_dir_inode.is_directory() {
            return Err(VexfsError::NotDirectory);
        }
        
        // Acquire write locks on both parent and target (in order to prevent deadlock)
        let entry = {
            let _parent_lock = acquire_write_lock_guard(context.lock_manager,parent_inode)?;
            
            // Load parent directory and find the target
            let mut parent_dir = Directory::from_inode(parent_dir_inode.as_ref().clone());
            let entry = parent_dir.find_entry(name)?
                .ok_or(VexfsError::NotFound)?
                .clone();
            
            // Check if target is a directory
            if entry.file_type != FileType::Directory {
                return Err(VexfsError::NotDirectory);
            }
            
            // Check deletion permission
            if !can_delete_from_directory(&parent_dir_inode, &context.user) {
                return Err(VexfsError::PermissionDenied("Permission denied".to_string()));
            }
            
            entry
        };
        
        // Get the target directory
        let target_dir_inode = get_inode(context.inode_manager,entry.inode_number)?;
        
        // Now acquire locks in proper order to prevent deadlock
        let (first_inode, second_inode) = if parent_inode <= entry.inode_number {
            (parent_inode, entry.inode_number)
        } else {
            (entry.inode_number, parent_inode)
        };
        
        let _lock1 = acquire_write_lock_guard(context.lock_manager, first_inode)?;
        // TODO: Implement proper dual-lock mechanism for parent-target operations
        // For now, using single lock to avoid multiple mutable borrows
        
        // Check if directory is empty
        let mut target_dir = Directory::from_inode((*target_dir_inode).clone());
        if !target_dir.is_empty() {
            return Err(VexfsError::DirectoryNotEmpty);
        }
        
        // Load parent directory again and remove entry
        let mut parent_directory = Directory::from_inode(parent_dir_inode.as_ref().clone());
        parent_directory.remove_entry(name)?;
        
        // Update parent link count (removing the .. link from deleted dir)
        let mut updated_parent = parent_directory.inode.clone();
        updated_parent.nlink -= 1;
        updated_parent.mtime = crate::shared::utils::current_timestamp();
        updated_parent.mark_dirty();
        
        // Delete the target directory inode
        delete_inode(context.inode_manager, entry.inode_number)?;
        
        // Save parent directory
        parent_directory.sync()?;
        put_inode(context.inode_manager, Arc::new(updated_parent))?;
        
        Ok(())
    }
    
    /// Create a hard link
    /// 
    /// Creates a hard link to an existing file.
    /// 
    /// # Arguments
    /// 
    /// * `target_inode` - Inode of the file to link to
    /// * `dir_inode` - Directory to create the link in
    /// * `name` - Name for the new link
    /// * `user` - User context for permission checking
    /// 
    /// # Returns
    /// 
    /// Returns Ok(()) on success or an error.
    pub fn create_hard_link(
        target_inode: InodeNumber,
        dir_inode: InodeNumber,
        name: &str,
        context: &mut OperationContext
    ) -> Result<()> {
        // Get target inode
        let mut target = get_inode(context.inode_manager,target_inode)?;
        
        // Can't create hard links to directories
        if target.is_directory() {
            return Err(VexfsError::IsDirectory);
        }
        
        // Get directory inode
        let dir = get_inode(context.inode_manager,dir_inode)?;
        
        // Check if it's actually a directory
        if !dir.is_directory() {
            return Err(VexfsError::NotDirectory);
        }
        
        // Check permission to create in directory
        if !can_create_in_directory(&dir, &context.user) {
            return Err(VexfsError::PermissionDenied("Permission denied".to_string()));
        }
        
        // Acquire directory lock (simplified approach to avoid multiple borrows)
        // TODO: Implement proper dual-lock mechanism for directory-target operations
        let _dir_lock = acquire_write_lock_guard(context.lock_manager,dir_inode)?;
        
        // Load directory and check if name exists
        let mut directory = Directory::from_inode(dir.as_ref().clone());
        if directory.find_entry(name)?.is_some() {
            return Err(VexfsError::FileExists);
        }
        
        // Create directory entry
        let entry = DirectoryEntry::new(target_inode, target.file_type, name.to_string())?;
        directory.add_entry(entry)?;
        
        // Increment link count
        let mut target_mut = Arc::try_unwrap(target).unwrap_or_else(|arc| (*arc).clone());
        target_mut.nlink += 1;
        target_mut.ctime = crate::shared::utils::current_timestamp();
        target_mut.mark_dirty();
        
        // Save changes
        directory.sync()?;
        put_inode(context.inode_manager, Arc::new(target_mut))?;
        
        Ok(())
    }
    
    /// Create a symbolic link
    /// 
    /// Creates a symbolic link to a target path.
    /// 
    /// # Arguments
    /// 
    /// * `target_path` - Path that the symlink points to
    /// * `dir_inode` - Directory to create the link in
    /// * `name` - Name for the new symlink
    /// * `user` - User context for permission checking
    /// 
    /// # Returns
    /// 
    /// Returns Ok(()) on success or an error.
    pub fn create_symbolic_link(
        target_path: &str,
        dir_inode: InodeNumber,
        name: &str,
        context: &mut OperationContext
    ) -> Result<()> {
        // Get directory inode
        let dir = get_inode(context.inode_manager,dir_inode)?;
        
        // Check if it's actually a directory
        if !dir.is_directory() {
            return Err(VexfsError::NotDirectory);
        }
        
        // Check permission to create in directory
        if !can_create_in_directory(&dir, &context.user) {
            return Err(VexfsError::PermissionDenied("Permission denied".to_string()));
        }
        
        // Acquire directory lock
        let _dir_lock = acquire_write_lock_guard(context.lock_manager,dir_inode)?;
        
        // Load directory and check if name exists
        let mut directory = Directory::from_inode(dir.as_ref().clone());
        if directory.find_entry(name)?.is_some() {
            return Err(VexfsError::FileExists);
        }
        
        // Create symlink inode
        let symlink_inode = create_inode(context.inode_manager, FileType::Symlink, FileMode::new(0o777), context.user.uid, context.user.gid)?;
        let mut symlink_inode_mut = (*symlink_inode).clone();
        symlink_inode_mut.size = target_path.len() as u64;
        symlink_inode_mut.nlink = 1;
        
        // TODO: Store the target path in the symlink inode data
        // For now, this is a placeholder
        
        let symlink_inode_number = symlink_inode_mut.ino;
        
        // Create directory entry
        let entry = DirectoryEntry::new(
            symlink_inode_number,
            FileType::Symlink,
            name.to_string()
        )?;
        directory.add_entry(entry)?;
        
        // Save changes
        directory.sync()?;
        put_inode(context.inode_manager, Arc::new(symlink_inode_mut))?;
        
        Ok(())
    }
}

// Public API functions for directory operations

/// Create a new directory
pub fn create_directory(
    parent_inode: InodeNumber,
    name: &str,
    mode: u32,
    user: &UserContext
) -> Result<Directory> {
    // TODO: Create OperationContext from UserContext - needs managers
    // This is a placeholder implementation
    Err(VexfsError::NotImplemented("create_directory wrapper needs OperationContext".to_string()))
}

/// Read directory entries
pub fn read_directory(inode_number: InodeNumber, user: &UserContext) -> Result<Vec<DirectoryEntry>> {
    // TODO: Create OperationContext from UserContext - needs managers
    // This is a placeholder implementation
    Err(VexfsError::NotImplemented("read_directory wrapper needs OperationContext".to_string()))
}

/// Look up an entry in a directory
pub fn lookup_entry(
    dir_inode: InodeNumber,
    name: &str,
    user: &UserContext
) -> Result<DirectoryEntry> {
    // TODO: Create OperationContext from UserContext - needs managers
    // This is a placeholder implementation
    Err(VexfsError::NotImplemented("lookup_entry wrapper needs OperationContext".to_string()))
}

/// Rename/move an entry
pub fn rename_entry(
    old_dir_inode: InodeNumber,
    old_name: &str,
    new_dir_inode: InodeNumber,
    new_name: &str,
    user: &UserContext
) -> Result<()> {
    // TODO: Create OperationContext from UserContext - needs managers
    // This is a placeholder implementation
    Err(VexfsError::NotImplemented("rename_entry wrapper needs OperationContext".to_string()))
}

/// Delete a directory
pub fn delete_directory(parent_inode: InodeNumber, name: &str, user: &UserContext) -> Result<()> {
    // TODO: Create OperationContext from UserContext - needs managers
    // This is a placeholder implementation
    Err(VexfsError::NotImplemented("delete_directory wrapper needs OperationContext".to_string()))
}

/// Create a hard link
pub fn create_hard_link(
    target_inode: InodeNumber,
    dir_inode: InodeNumber,
    name: &str,
    user: &UserContext
) -> Result<()> {
    // TODO: Create OperationContext from UserContext - needs managers
    // This is a placeholder implementation
    Err(VexfsError::NotImplemented("create_hard_link wrapper needs OperationContext".to_string()))
}

/// Create a symbolic link
pub fn create_symbolic_link(
    target_path: &str,
    dir_inode: InodeNumber,
    name: &str,
    user: &UserContext
) -> Result<()> {
    // TODO: Create OperationContext from UserContext - needs managers
    // This is a placeholder implementation
    Err(VexfsError::NotImplemented("create_symbolic_link wrapper needs OperationContext".to_string()))
}

// Wrapper functions to match the interface expected by operations.rs

/// Add an entry to a directory (wrapper for operations.rs compatibility)
pub fn add_entry(
    inode_manager: &mut InodeManager,
    dir_inode: InodeNumber,
    entry: DirectoryEntry,
    user: &UserContext,
) -> Result<()> {
    // Get the directory inode
    let dir_arc = get_inode(inode_manager, dir_inode)?;
    let dir = &*dir_arc;
    
    // Check if it's actually a directory
    if !dir.is_directory() {
        return Err(VexfsError::NotDirectory);
    }
    
    // Check permission to create in directory
    check_create_permission(dir, user)?;
    
    // Load directory and add entry
    let mut directory = Directory::from_inode(dir.clone());
    directory.add_entry(entry)?;
    // TODO: Implement proper sync with inode_manager
    
    Ok(())
}

/// Remove an entry from a directory (wrapper for operations.rs compatibility)
pub fn remove_entry(
    inode_manager: &mut InodeManager,
    dir_inode: InodeNumber,
    name: &str,
    user: &UserContext,
) -> Result<DirectoryEntry> {
    // Get the directory inode
    let dir_arc = get_inode(inode_manager, dir_inode)?;
    let dir = &*dir_arc;
    
    // Check if it's actually a directory
    if !dir.is_directory() {
        return Err(VexfsError::NotDirectory);
    }
    
    // Check permission to delete from directory
    check_delete_permission(dir, user)?;
    
    // Load directory and remove entry
    let mut directory = Directory::from_inode(dir.clone());
    let removed_entry = directory.remove_entry(name)?;
    // TODO: Implement proper sync with inode_manager
    
    Ok(removed_entry)
}

/// Read directory entries (wrapper for operations.rs compatibility)
pub fn read_entries(
    inode_manager: &mut InodeManager,
    dir_inode: InodeNumber,
    user: &UserContext,
) -> Result<Vec<DirectoryEntry>> {
    // Get the directory inode
    let dir_arc = get_inode(inode_manager, dir_inode)?;
    let dir = &*dir_arc;
    
    // Check if it's actually a directory
    if !dir.is_directory() {
        return Err(VexfsError::NotDirectory);
    }
    
    // Check read permission
    check_read_permission(dir, user)?;
    
    // Load directory and return entries
    let mut directory = Directory::from_inode(dir.clone());
    let entries = directory.list_entries()?;
    
    Ok(entries.clone())
}

/// Update the ".." entry in a directory (placeholder implementation)
pub fn update_dotdot_entry(
    inode_manager: &mut InodeManager,
    dir_inode: InodeNumber,
    new_parent_inode: InodeNumber,
) -> Result<()> {
    // Get the directory inode
    let dir_arc = get_inode(inode_manager, dir_inode)?;
    let dir = &*dir_arc;
    
    // Check if it's actually a directory
    if !dir.is_directory() {
        return Err(VexfsError::NotDirectory);
    }
    
    // Load directory
    let mut directory = Directory::from_inode(dir.clone());
    directory.ensure_entries_loaded()?;
    
    // Find and update the ".." entry
    for entry in &mut directory.entries {
        if entry.name == ".." {
            entry.inode_number = new_parent_inode;
            directory.mark_dirty();
            break;
        }
    }
    
    // TODO: Implement proper sync with inode_manager
    Ok(())
}