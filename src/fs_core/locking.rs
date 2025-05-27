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

//! Filesystem Locking Module
//!
//! This module provides fine-grained locking mechanisms for filesystem operations
//! to ensure data consistency and prevent race conditions in concurrent access scenarios.

use crate::shared::{
    errors::VexfsError,
    types::*,
    constants::*,
};
use super::FsResult;

extern crate alloc;
use alloc::collections::BTreeMap;
use core::sync::atomic::{AtomicU64, Ordering};

/// Lock type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockType {
    /// Shared read lock
    Read,
    /// Exclusive write lock
    Write,
    /// Advisory lock (for file locking)
    Advisory,
}

/// Lock scope enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockScope {
    /// Lock on inode metadata
    Inode(InodeNumber),
    /// Lock on directory structure
    Directory(InodeNumber),
    /// Lock on file range
    FileRange(InodeNumber, FileOffset, FileSize),
    /// Global filesystem lock
    Global,
}

/// Lock identifier for tracking
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct LockId(u64);

/// File lock structure for POSIX-style file locking
#[derive(Debug, Clone)]
pub struct FileLock {
    pub lock_type: LockType,
    pub start: FileOffset,
    pub length: FileSize,
    pub owner: u32, // Process ID or similar identifier
    pub exclusive: bool,
}

impl FileLock {
    pub fn new(lock_type: LockType, start: FileOffset, length: FileSize, owner: u32) -> Self {
        Self {
            lock_type,
            start,
            length,
            owner,
            exclusive: lock_type == LockType::Write,
        }
    }

    /// Check if this lock conflicts with another lock
    pub fn conflicts_with(&self, other: &FileLock) -> bool {
        // Different owners can have conflicting locks
        if self.owner == other.owner {
            return false;
        }

        // Check range overlap
        let self_end = self.start + self.length;
        let other_end = other.start + other.length;
        
        let overlaps = !(self_end <= other.start || other_end <= self.start);
        
        // Conflict if ranges overlap and at least one is exclusive
        overlaps && (self.exclusive || other.exclusive)
    }
}

/// Directory lock for structural operations
#[derive(Debug, Clone)]
pub struct DirectoryLock {
    pub inode: InodeNumber,
    pub lock_type: LockType,
    pub owner: u32,
}

impl DirectoryLock {
    pub fn new(inode: InodeNumber, lock_type: LockType, owner: u32) -> Self {
        Self {
            inode,
            lock_type,
            owner,
        }
    }
}

/// Lock manager for coordinating all filesystem locks
pub struct LockManager {
    next_lock_id: AtomicU64,
    inode_locks: BTreeMap<InodeNumber, Vec<LockId>>,
    directory_locks: BTreeMap<InodeNumber, Vec<LockId>>,
    file_locks: BTreeMap<InodeNumber, Vec<FileLock>>,
    active_locks: BTreeMap<LockId, LockScope>,
    lock_owners: BTreeMap<LockId, u32>,
}

impl LockManager {
    /// Create a new lock manager
    pub fn new() -> Self {
        Self {
            next_lock_id: AtomicU64::new(1),
            inode_locks: BTreeMap::new(),
            directory_locks: BTreeMap::new(),
            file_locks: BTreeMap::new(),
            active_locks: BTreeMap::new(),
            lock_owners: BTreeMap::new(),
        }
    }

    /// Generate a new unique lock ID
    fn next_id(&self) -> LockId {
        LockId(self.next_lock_id.fetch_add(1, Ordering::SeqCst))
    }

    /// Acquire an inode lock
    pub fn lock_inode(&mut self, inode: InodeNumber, lock_type: LockType, owner: u32) -> FsResult<LockId> {
        // Check for conflicts with existing locks
        if let Some(existing_locks) = self.inode_locks.get(&inode) {
            for &lock_id in existing_locks {
                if let Some(scope) = self.active_locks.get(&lock_id) {
                    if let LockScope::Inode(locked_inode) = scope {
                        if locked_inode == &inode {
                            // Check if this is a conflicting lock type
                            if lock_type == LockType::Write {
                                return Err(VexfsError::LockConflict("Inode already locked".into()));
                            }
                        }
                    }
                }
            }
        }

        let lock_id = self.next_id();
        let scope = LockScope::Inode(inode);

        // Record the lock
        self.active_locks.insert(lock_id, scope);
        self.lock_owners.insert(lock_id, owner);
        self.inode_locks.entry(inode).or_insert_with(Vec::new).push(lock_id);

        Ok(lock_id)
    }

    /// Acquire a directory lock
    pub fn lock_directory(&mut self, inode: InodeNumber, lock_type: LockType, owner: u32) -> FsResult<LockId> {
        // Check for conflicts
        if let Some(existing_locks) = self.directory_locks.get(&inode) {
            if !existing_locks.is_empty() && lock_type == LockType::Write {
                return Err(VexfsError::LockConflict("Directory already locked".into()));
            }
        }

        let lock_id = self.next_id();
        let scope = LockScope::Directory(inode);

        self.active_locks.insert(lock_id, scope);
        self.lock_owners.insert(lock_id, owner);
        self.directory_locks.entry(inode).or_insert_with(Vec::new).push(lock_id);

        Ok(lock_id)
    }

    /// Acquire a file range lock
    pub fn lock_file_range(
        &mut self,
        inode: InodeNumber,
        start: FileOffset,
        length: FileSize,
        lock_type: LockType,
        owner: u32,
    ) -> FsResult<LockId> {
        let new_lock = FileLock::new(lock_type, start, length, owner);

        // Check for conflicts with existing file locks
        if let Some(existing_locks) = self.file_locks.get(&inode) {
            for existing_lock in existing_locks {
                if new_lock.conflicts_with(existing_lock) {
                    return Err(VexfsError::LockConflict("File range already locked".into()));
                }
            }
        }

        let lock_id = self.next_id();
        let scope = LockScope::FileRange(inode, start, length);

        self.active_locks.insert(lock_id, scope);
        self.lock_owners.insert(lock_id, owner);
        self.file_locks.entry(inode).or_insert_with(Vec::new).push(new_lock);

        Ok(lock_id)
    }

    /// Release a lock
    pub fn unlock(&mut self, lock_id: LockId) -> FsResult<()> {
        let scope = self.active_locks.remove(&lock_id)
            .ok_or_else(|| VexfsError::InvalidOperation)?;

        self.lock_owners.remove(&lock_id);

        match scope {
            LockScope::Inode(inode) => {
                if let Some(locks) = self.inode_locks.get_mut(&inode) {
                    locks.retain(|&id| id != lock_id);
                    if locks.is_empty() {
                        self.inode_locks.remove(&inode);
                    }
                }
            }
            LockScope::Directory(inode) => {
                if let Some(locks) = self.directory_locks.get_mut(&inode) {
                    locks.retain(|&id| id != lock_id);
                    if locks.is_empty() {
                        self.directory_locks.remove(&inode);
                    }
                }
            }
            LockScope::FileRange(inode, start, length) => {
                if let Some(locks) = self.file_locks.get_mut(&inode) {
                    locks.retain(|lock| !(lock.start == start && lock.length == length));
                    if locks.is_empty() {
                        self.file_locks.remove(&inode);
                    }
                }
            }
            LockScope::Global => {
                // Global lock handling would go here
            }
        }

        Ok(())
    }

    /// Release all locks owned by a specific owner
    pub fn unlock_all_for_owner(&mut self, owner: u32) -> FsResult<()> {
        let locks_to_release: Vec<LockId> = self.lock_owners
            .iter()
            .filter(|(_, &lock_owner)| lock_owner == owner)
            .map(|(&lock_id, _)| lock_id)
            .collect();

        for lock_id in locks_to_release {
            self.unlock(lock_id)?;
        }

        Ok(())
    }

    /// Check if an inode is locked
    pub fn is_inode_locked(&self, inode: InodeNumber) -> bool {
        self.inode_locks.get(&inode).map_or(false, |locks| !locks.is_empty())
    }

    /// Check if a directory is locked
    pub fn is_directory_locked(&self, inode: InodeNumber) -> bool {
        self.directory_locks.get(&inode).map_or(false, |locks| !locks.is_empty())
    }

    /// Check if a file range is locked
    pub fn is_file_range_locked(&self, inode: InodeNumber, start: FileOffset, length: FileSize) -> bool {
        if let Some(locks) = self.file_locks.get(&inode) {
            let test_lock = FileLock::new(LockType::Write, start, length, 0);
            locks.iter().any(|lock| test_lock.conflicts_with(lock))
        } else {
            false
        }
    }

    /// Get lock statistics
    pub fn lock_stats(&self) -> (usize, usize, usize) {
        (
            self.inode_locks.len(),
            self.directory_locks.len(),
            self.file_locks.len(),
        )
    }

    /// Deadlock detection and prevention
    pub fn detect_deadlock(&self, _requesting_owner: u32, _requested_scope: &LockScope) -> bool {
        // TODO: Implement deadlock detection algorithm
        // For now, return false (no deadlock detected)
        false
    }
}

/// RAII lock guard for automatic lock release
pub struct LockGuard<'a> {
    lock_manager: &'a mut LockManager,
    lock_id: LockId,
}

impl<'a> LockGuard<'a> {
    pub fn new(lock_manager: &'a mut LockManager, lock_id: LockId) -> Self {
        Self {
            lock_manager,
            lock_id,
        }
    }
}

impl<'a> Drop for LockGuard<'a> {
    fn drop(&mut self) {
        let _ = self.lock_manager.unlock(self.lock_id);
    }
}

// Public helper functions that were being imported
pub fn lock_inode_read(manager: &mut LockManager, inode: InodeNumber) -> FsResult<LockId> {
    manager.lock_inode(inode, LockType::Read, 0)
}

pub fn lock_inode_write(manager: &mut LockManager, inode: InodeNumber) -> FsResult<LockId> {
    manager.lock_inode(inode, LockType::Write, 0)
}

pub fn unlock_inode(manager: &mut LockManager, lock_id: LockId) -> FsResult<()> {
    manager.unlock(lock_id)
}

pub fn lock_directory_read(manager: &mut LockManager, inode: InodeNumber) -> FsResult<LockId> {
    manager.lock_directory(inode, LockType::Read, 0)
}

pub fn lock_directory_write(manager: &mut LockManager, inode: InodeNumber) -> FsResult<LockId> {
    manager.lock_directory(inode, LockType::Write, 0)
}

pub fn unlock_directory(manager: &mut LockManager, lock_id: LockId) -> FsResult<()> {
    manager.unlock(lock_id)
}
pub fn acquire_write_lock_guard<'a>(manager: &'a mut LockManager, inode: InodeNumber) -> FsResult<LockGuard<'a>> {
    // TODO: Review owner/context for lock guards, using 0 as placeholder
    let lock_id = manager.lock_inode(inode, LockType::Write, 0)?;
    Ok(LockGuard::new(manager, lock_id))
}

pub fn acquire_read_lock_guard<'a>(manager: &'a mut LockManager, inode: InodeNumber) -> FsResult<LockGuard<'a>> {
    // TODO: Review owner/context for lock guards, using 0 as placeholder
    let lock_id = manager.lock_inode(inode, LockType::Read, 0)?;
    Ok(LockGuard::new(manager, lock_id))
}

pub fn acquire_inode_lock(manager: &mut LockManager, inode: InodeNumber, lock_type: LockType, owner: u32) -> FsResult<LockId> {
    manager.lock_inode(inode, lock_type, owner)
}

pub fn release_inode_lock(manager: &mut LockManager, lock_id: LockId) -> FsResult<()> {
    manager.unlock(lock_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_lock_conflicts() {
        let lock1 = FileLock::new(LockType::Write, 0, 100, 1);
        let lock2 = FileLock::new(LockType::Read, 50, 100, 2);
        let lock3 = FileLock::new(LockType::Read, 200, 100, 3);

        assert!(lock1.conflicts_with(&lock2));
        assert!(!lock1.conflicts_with(&lock3));
    }

    #[test]
    fn test_lock_manager_inode_locking() {
        let mut manager = LockManager::new();
        
        let lock1 = manager.lock_inode(1, LockType::Read, 100).unwrap();
        let lock2 = manager.lock_inode(1, LockType::Read, 101).unwrap();
        
        // Should fail with write lock
        assert!(manager.lock_inode(1, LockType::Write, 102).is_err());
        
        manager.unlock(lock1).unwrap();
        manager.unlock(lock2).unwrap();
        
        // Should succeed after unlocking
        let _lock3 = manager.lock_inode(1, LockType::Write, 102).unwrap();
    }

    #[test]
    fn test_lock_cleanup() {
        let mut manager = LockManager::new();
        
        let _lock1 = manager.lock_inode(1, LockType::Read, 100).unwrap();
        let _lock2 = manager.lock_directory(2, LockType::Write, 100).unwrap();
        
        assert_eq!(manager.lock_stats(), (1, 1, 0));
        
        manager.unlock_all_for_owner(100).unwrap();
        
        assert_eq!(manager.lock_stats(), (0, 0, 0));
    }
}