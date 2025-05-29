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
 */

//! Memory Management Module for VexFS
//!
//! This module provides comprehensive memory management facilities including:
//! - Safe memory allocation and deallocation tracking
//! - Reference counting with atomic operations
//! - Memory barriers for proper synchronization
//! - Memory leak detection and monitoring
//! - Safe memory access patterns

use core::sync::atomic::{AtomicUsize, AtomicPtr, Ordering, fence};
use core::ptr::{NonNull, null_mut};
use core::mem::{size_of, align_of};
use core::alloc::{Layout, GlobalAlloc};

#[cfg(feature = "kernel")]
use alloc::{boxed::Box, vec::Vec, collections::BTreeMap, string::{String, ToString}, vec, format};
#[cfg(not(feature = "kernel"))]
use std::{boxed::Box, vec::Vec, collections::BTreeMap, string::{String, ToString}};

use crate::shared::errors::{VexfsError, VexfsResult};

/// Memory allocation statistics
#[derive(Debug, Clone)]
pub struct MemoryStats {
    /// Total bytes allocated
    pub total_allocated: usize,
    /// Total bytes freed
    pub total_freed: usize,
    /// Current bytes in use
    pub current_usage: usize,
    /// Peak memory usage
    pub peak_usage: usize,
    /// Number of active allocations
    pub active_allocations: usize,
    /// Number of allocation failures
    pub allocation_failures: usize,
    /// Number of detected leaks
    pub detected_leaks: usize,
}

impl Default for MemoryStats {
    fn default() -> Self {
        Self {
            total_allocated: 0,
            total_freed: 0,
            current_usage: 0,
            peak_usage: 0,
            active_allocations: 0,
            allocation_failures: 0,
            detected_leaks: 0,
        }
    }
}

/// Memory allocation tracking entry
#[derive(Debug, Clone)]
struct AllocationEntry {
    /// Size of the allocation
    size: usize,
    /// Alignment of the allocation
    align: usize,
    /// Allocation timestamp (in kernel ticks or similar)
    timestamp: u64,
    /// Source location (file:line for debugging)
    location: &'static str,
}

/// Global memory statistics with atomic operations
static MEMORY_STATS: MemoryStatsAtomic = MemoryStatsAtomic::new();

/// Atomic memory statistics for thread-safe access
struct MemoryStatsAtomic {
    total_allocated: AtomicUsize,
    total_freed: AtomicUsize,
    current_usage: AtomicUsize,
    peak_usage: AtomicUsize,
    active_allocations: AtomicUsize,
    allocation_failures: AtomicUsize,
    detected_leaks: AtomicUsize,
}

impl MemoryStatsAtomic {
    const fn new() -> Self {
        Self {
            total_allocated: AtomicUsize::new(0),
            total_freed: AtomicUsize::new(0),
            current_usage: AtomicUsize::new(0),
            peak_usage: AtomicUsize::new(0),
            active_allocations: AtomicUsize::new(0),
            allocation_failures: AtomicUsize::new(0),
            detected_leaks: AtomicUsize::new(0),
        }
    }

    fn to_stats(&self) -> MemoryStats {
        MemoryStats {
            total_allocated: self.total_allocated.load(Ordering::Acquire),
            total_freed: self.total_freed.load(Ordering::Acquire),
            current_usage: self.current_usage.load(Ordering::Acquire),
            peak_usage: self.peak_usage.load(Ordering::Acquire),
            active_allocations: self.active_allocations.load(Ordering::Acquire),
            allocation_failures: self.allocation_failures.load(Ordering::Acquire),
            detected_leaks: self.detected_leaks.load(Ordering::Acquire),
        }
    }
}

/// Reference counted pointer with atomic operations
pub struct AtomicRefPtr<T> {
    ptr: AtomicPtr<RefCountedData<T>>,
}

/// Reference counted data wrapper
struct RefCountedData<T> {
    data: T,
    ref_count: AtomicUsize,
    destructor: Option<fn(&T)>,
}

impl<T> AtomicRefPtr<T> {
    /// Create a new atomic reference pointer
    pub fn new(data: T) -> VexfsResult<Self> {
        let ref_data = Box::new(RefCountedData {
            data,
            ref_count: AtomicUsize::new(1),
            destructor: None,
        });

        let ptr = Box::into_raw(ref_data);
        
        Ok(Self {
            ptr: AtomicPtr::new(ptr),
        })
    }

    /// Create a new atomic reference pointer with custom destructor
    pub fn new_with_destructor(data: T, destructor: fn(&T)) -> VexfsResult<Self> {
        let ref_data = Box::new(RefCountedData {
            data,
            ref_count: AtomicUsize::new(1),
            destructor: Some(destructor),
        });

        let ptr = Box::into_raw(ref_data);
        
        Ok(Self {
            ptr: AtomicPtr::new(ptr),
        })
    }

    /// Clone the reference (increment reference count)
    pub fn clone_ref(&self) -> Option<Self> {
        let ptr = self.ptr.load(Ordering::Acquire);
        if ptr.is_null() {
            return None;
        }

        unsafe {
            let ref_data = &*ptr;
            let old_count = ref_data.ref_count.fetch_add(1, Ordering::AcqRel);
            
            // Check for overflow
            if old_count == usize::MAX {
                ref_data.ref_count.fetch_sub(1, Ordering::AcqRel);
                return None;
            }

            // Memory barrier to ensure reference count update is visible
            fence(Ordering::AcqRel);
        }

        Some(Self {
            ptr: AtomicPtr::new(ptr),
        })
    }

    /// Get a reference to the data (unsafe - caller must ensure lifetime)
    pub unsafe fn get_ref(&self) -> Option<&T> {
        let ptr = self.ptr.load(Ordering::Acquire);
        if ptr.is_null() {
            return None;
        }

        Some(&(*ptr).data)
    }

    /// Get the current reference count
    pub fn ref_count(&self) -> usize {
        let ptr = self.ptr.load(Ordering::Acquire);
        if ptr.is_null() {
            return 0;
        }

        unsafe {
            (*ptr).ref_count.load(Ordering::Acquire)
        }
    }

    /// Check if this is the only reference
    pub fn is_unique(&self) -> bool {
        self.ref_count() == 1
    }
}

impl<T> Drop for AtomicRefPtr<T> {
    fn drop(&mut self) {
        let ptr = self.ptr.load(Ordering::Acquire);
        if ptr.is_null() {
            return;
        }

        unsafe {
            let ref_data = &*ptr;
            let old_count = ref_data.ref_count.fetch_sub(1, Ordering::AcqRel);
            
            // Memory barrier to ensure reference count update is visible
            fence(Ordering::AcqRel);

            if old_count == 1 {
                // Last reference - clean up
                if let Some(destructor) = ref_data.destructor {
                    destructor(&ref_data.data);
                }

                // Convert back to Box and drop
                let _ = Box::from_raw(ptr);
            }
        }
    }
}

unsafe impl<T: Send> Send for AtomicRefPtr<T> {}
unsafe impl<T: Sync> Sync for AtomicRefPtr<T> {}

/// Memory pool for frequent allocations
pub struct MemoryPool {
    /// Block size for this pool
    block_size: usize,
    /// Alignment requirement
    alignment: usize,
    /// Free blocks stack
    free_blocks: AtomicPtr<PoolBlock>,
    /// Total blocks allocated
    total_blocks: AtomicUsize,
    /// Free blocks count
    free_count: AtomicUsize,
}

/// Pool block header
struct PoolBlock {
    next: *mut PoolBlock,
}

impl MemoryPool {
    /// Create a new memory pool
    pub fn new(block_size: usize, alignment: usize) -> VexfsResult<Self> {
        if block_size < size_of::<PoolBlock>() {
            return Err(VexfsError::InvalidParameter("Block size too small".into()));
        }

        if !alignment.is_power_of_two() {
            return Err(VexfsError::InvalidParameter("Alignment must be power of two".into()));
        }

        Ok(Self {
            block_size,
            alignment,
            free_blocks: AtomicPtr::new(null_mut()),
            total_blocks: AtomicUsize::new(0),
            free_count: AtomicUsize::new(0),
        })
    }

    /// Allocate a block from the pool
    pub fn allocate(&self) -> VexfsResult<NonNull<u8>> {
        // Try to get a free block first
        loop {
            let head = self.free_blocks.load(Ordering::Acquire);
            if head.is_null() {
                break;
            }

            unsafe {
                let next = (*head).next;
                if self.free_blocks.compare_exchange_weak(
                    head,
                    next,
                    Ordering::AcqRel,
                    Ordering::Relaxed,
                ).is_ok() {
                    self.free_count.fetch_sub(1, Ordering::AcqRel);
                    fence(Ordering::AcqRel);
                    return Ok(NonNull::new_unchecked(head as *mut u8));
                }
            }
        }

        // No free blocks, allocate a new one
        self.allocate_new_block()
    }

    /// Deallocate a block back to the pool
    pub fn deallocate(&self, ptr: NonNull<u8>) {
        let block = ptr.as_ptr() as *mut PoolBlock;
        
        unsafe {
            loop {
                let head = self.free_blocks.load(Ordering::Acquire);
                (*block).next = head;
                
                if self.free_blocks.compare_exchange_weak(
                    head,
                    block,
                    Ordering::AcqRel,
                    Ordering::Relaxed,
                ).is_ok() {
                    self.free_count.fetch_add(1, Ordering::AcqRel);
                    fence(Ordering::AcqRel);
                    break;
                }
            }
        }
    }

    /// Get pool statistics
    pub fn stats(&self) -> (usize, usize) {
        (
            self.total_blocks.load(Ordering::Acquire),
            self.free_count.load(Ordering::Acquire),
        )
    }

    fn allocate_new_block(&self) -> VexfsResult<NonNull<u8>> {
        let layout = Layout::from_size_align(self.block_size, self.alignment)
            .map_err(|_| VexfsError::InvalidParameter("Invalid layout".into()))?;

        // Use system allocator
        #[cfg(feature = "kernel")]
        let ptr = unsafe {
            // In kernel mode, use kmalloc equivalent
            // This is a placeholder - real implementation would use kernel allocator
            alloc::alloc::alloc(layout)
        };

        #[cfg(not(feature = "kernel"))]
        let ptr = unsafe {
            std::alloc::System.alloc(layout)
        };

        if ptr.is_null() {
            MEMORY_STATS.allocation_failures.fetch_add(1, Ordering::AcqRel);
            return Err(VexfsError::OutOfMemory);
        }

        self.total_blocks.fetch_add(1, Ordering::AcqRel);
        track_allocation(self.block_size, "MemoryPool::allocate_new_block");

        Ok(unsafe { NonNull::new_unchecked(ptr) })
    }
}

impl Drop for MemoryPool {
    fn drop(&mut self) {
        // Free all remaining blocks
        let mut current = self.free_blocks.load(Ordering::Acquire);
        while !current.is_null() {
            unsafe {
                let next = (*current).next;
                let layout = Layout::from_size_align(self.block_size, self.alignment).unwrap();
                
                #[cfg(feature = "kernel")]
                alloc::alloc::dealloc(current as *mut u8, layout);
                
                #[cfg(not(feature = "kernel"))]
                std::alloc::System.dealloc(current as *mut u8, layout);
                
                track_deallocation(self.block_size);
                current = next;
            }
        }
    }
}

/// Safe memory access wrapper
pub struct SafeMemoryAccess<T> {
    ptr: NonNull<T>,
    size: usize,
    bounds_check: bool,
}

impl<T> SafeMemoryAccess<T> {
    /// Create a new safe memory access wrapper
    pub fn new(ptr: NonNull<T>, size: usize) -> Self {
        Self {
            ptr,
            size,
            bounds_check: true,
        }
    }

    /// Create without bounds checking (for performance-critical paths)
    pub fn new_unchecked(ptr: NonNull<T>, size: usize) -> Self {
        Self {
            ptr,
            size,
            bounds_check: false,
        }
    }

    /// Get a safe reference to the data
    pub fn get(&self) -> VexfsResult<&T> {
        if self.bounds_check && size_of::<T>() > self.size {
            return Err(VexfsError::InvalidParameter("Access out of bounds".into()));
        }

        Ok(unsafe { self.ptr.as_ref() })
    }

    /// Get a safe mutable reference to the data
    pub fn get_mut(&mut self) -> VexfsResult<&mut T> {
        if self.bounds_check && size_of::<T>() > self.size {
            return Err(VexfsError::InvalidParameter("Access out of bounds".into()));
        }

        Ok(unsafe { self.ptr.as_mut() })
    }

    /// Get a slice of the data
    pub fn get_slice(&self, count: usize) -> VexfsResult<&[T]> {
        let total_size = size_of::<T>() * count;
        if self.bounds_check && total_size > self.size {
            return Err(VexfsError::InvalidParameter("Slice access out of bounds".into()));
        }

        Ok(unsafe { core::slice::from_raw_parts(self.ptr.as_ptr(), count) })
    }

    /// Get a mutable slice of the data
    pub fn get_slice_mut(&mut self, count: usize) -> VexfsResult<&mut [T]> {
        let total_size = size_of::<T>() * count;
        if self.bounds_check && total_size > self.size {
            return Err(VexfsError::InvalidParameter("Slice access out of bounds".into()));
        }

        Ok(unsafe { core::slice::from_raw_parts_mut(self.ptr.as_ptr(), count) })
    }

    /// Copy data safely
    pub fn copy_from(&mut self, src: &[T]) -> VexfsResult<()> 
    where 
        T: Copy 
    {
        let required_size = size_of::<T>() * src.len();
        if self.bounds_check && required_size > self.size {
            return Err(VexfsError::InvalidParameter("Copy would exceed bounds".into()));
        }

        unsafe {
            core::ptr::copy_nonoverlapping(
                src.as_ptr(),
                self.ptr.as_ptr(),
                src.len(),
            );
        }

        Ok(())
    }

    /// Zero the memory
    pub fn zero(&mut self) -> VexfsResult<()> {
        if self.bounds_check && size_of::<T>() > self.size {
            return Err(VexfsError::InvalidParameter("Zero would exceed bounds".into()));
        }

        unsafe {
            core::ptr::write_bytes(self.ptr.as_ptr(), 0, self.size / size_of::<T>());
        }

        Ok(())
    }
}

/// Memory barrier utilities
pub struct MemoryBarriers;

impl MemoryBarriers {
    /// Full memory barrier
    pub fn full() {
        fence(Ordering::SeqCst);
    }

    /// Acquire barrier (load-acquire)
    pub fn acquire() {
        fence(Ordering::Acquire);
    }

    /// Release barrier (store-release)
    pub fn release() {
        fence(Ordering::Release);
    }

    /// Acquire-release barrier
    pub fn acq_rel() {
        fence(Ordering::AcqRel);
    }
}

/// Track memory allocation
pub fn track_allocation(size: usize, location: &'static str) {
    MEMORY_STATS.total_allocated.fetch_add(size, Ordering::AcqRel);
    let current = MEMORY_STATS.current_usage.fetch_add(size, Ordering::AcqRel) + size;
    MEMORY_STATS.active_allocations.fetch_add(1, Ordering::AcqRel);

    // Update peak usage
    let mut peak = MEMORY_STATS.peak_usage.load(Ordering::Acquire);
    while current > peak {
        match MEMORY_STATS.peak_usage.compare_exchange_weak(
            peak,
            current,
            Ordering::AcqRel,
            Ordering::Relaxed,
        ) {
            Ok(_) => break,
            Err(new_peak) => peak = new_peak,
        }
    }

    // Memory barrier to ensure all updates are visible
    fence(Ordering::AcqRel);

    #[cfg(feature = "kernel")]
    {
        // In kernel mode, log allocation for debugging
        // printk!(KERN_DEBUG "VexFS: Allocated %zu bytes at %s\n", size, location);
    }

    #[cfg(not(feature = "kernel"))]
    {
        // In userspace, use println for debugging
        if cfg!(debug_assertions) {
            println!("VexFS: Allocated {} bytes at {}", size, location);
        }
    }
}

/// Track memory deallocation
pub fn track_deallocation(size: usize) {
    MEMORY_STATS.total_freed.fetch_add(size, Ordering::AcqRel);
    MEMORY_STATS.current_usage.fetch_sub(size, Ordering::AcqRel);
    MEMORY_STATS.active_allocations.fetch_sub(1, Ordering::AcqRel);

    // Memory barrier to ensure all updates are visible
    fence(Ordering::AcqRel);
}

/// Get current memory statistics
pub fn get_memory_stats() -> MemoryStats {
    // Memory barrier to ensure we see latest updates
    fence(Ordering::Acquire);
    MEMORY_STATS.to_stats()
}

/// Detect memory leaks
pub fn detect_memory_leaks() -> Vec<String> {
    let stats = get_memory_stats();
    let mut leaks = Vec::new();

    if stats.current_usage > 0 && stats.active_allocations > 0 {
        leaks.push(format!(
            "Memory leak detected: {} bytes in {} allocations",
            stats.current_usage,
            stats.active_allocations
        ));

        MEMORY_STATS.detected_leaks.fetch_add(1, Ordering::AcqRel);
    }

    leaks
}

/// Reset memory statistics (for testing)
pub fn reset_memory_stats() {
    MEMORY_STATS.total_allocated.store(0, Ordering::Release);
    MEMORY_STATS.total_freed.store(0, Ordering::Release);
    MEMORY_STATS.current_usage.store(0, Ordering::Release);
    MEMORY_STATS.peak_usage.store(0, Ordering::Release);
    MEMORY_STATS.active_allocations.store(0, Ordering::Release);
    MEMORY_STATS.allocation_failures.store(0, Ordering::Release);
    MEMORY_STATS.detected_leaks.store(0, Ordering::Release);
    
    fence(Ordering::Release);
}

/// Safe string handling for kernel context
pub fn safe_string_copy(dest: &mut [u8], src: &[u8]) -> VexfsResult<usize> {
    if dest.is_empty() {
        return Err(VexfsError::InvalidParameter("Destination buffer is empty".into()));
    }

    let copy_len = core::cmp::min(dest.len() - 1, src.len());
    
    // Safe copy with bounds checking
    dest[..copy_len].copy_from_slice(&src[..copy_len]);
    dest[copy_len] = 0; // Null terminate

    Ok(copy_len)
}

/// Validate user-provided memory addresses
pub fn validate_user_memory(ptr: *const u8, size: usize) -> VexfsResult<()> {
    if ptr.is_null() {
        return Err(VexfsError::InvalidParameter("Null pointer".into()));
    }

    if size == 0 {
        return Err(VexfsError::InvalidParameter("Zero size".into()));
    }

    // Check for overflow
    if (ptr as usize).checked_add(size).is_none() {
        return Err(VexfsError::InvalidParameter("Address overflow".into()));
    }

    #[cfg(feature = "kernel")]
    {
        // In kernel mode, validate user memory access
        // This would use access_ok() or similar kernel function
        // For now, just basic validation
        if (ptr as usize) < 0x1000 {
            return Err(VexfsError::InvalidParameter("Invalid user address".into()));
        }
    }

    Ok(())
}

/// Memory allocation wrapper with tracking
pub fn tracked_alloc(size: usize, location: &'static str) -> VexfsResult<NonNull<u8>> {
    let layout = Layout::from_size_align(size, align_of::<usize>())
        .map_err(|_| VexfsError::InvalidParameter("Invalid layout".into()))?;

    #[cfg(feature = "kernel")]
    let ptr = unsafe {
        // In kernel mode, use kmalloc equivalent
        alloc::alloc::alloc(layout)
    };

    #[cfg(not(feature = "kernel"))]
    let ptr = unsafe {
        std::alloc::System.alloc(layout)
    };

    if ptr.is_null() {
        MEMORY_STATS.allocation_failures.fetch_add(1, Ordering::AcqRel);
        return Err(VexfsError::OutOfMemory);
    }

    track_allocation(size, location);
    Ok(unsafe { NonNull::new_unchecked(ptr) })
}

/// Memory deallocation wrapper with tracking
pub fn tracked_dealloc(ptr: NonNull<u8>, size: usize) {
    let layout = Layout::from_size_align(size, align_of::<usize>()).unwrap();

    #[cfg(feature = "kernel")]
    unsafe {
        alloc::alloc::dealloc(ptr.as_ptr(), layout);
    }

    #[cfg(not(feature = "kernel"))]
    unsafe {
        std::alloc::System.dealloc(ptr.as_ptr(), layout);
    }

    track_deallocation(size);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_tracking() {
        reset_memory_stats();
        
        let ptr = tracked_alloc(1024, "test").unwrap();
        let stats = get_memory_stats();
        
        assert_eq!(stats.current_usage, 1024);
        assert_eq!(stats.active_allocations, 1);
        
        tracked_dealloc(ptr, 1024);
        let stats = get_memory_stats();
        
        assert_eq!(stats.current_usage, 0);
        assert_eq!(stats.active_allocations, 0);
    }

    #[test]
    fn test_atomic_ref_ptr() {
        let data = 42u32;
        let ref_ptr = AtomicRefPtr::new(data).unwrap();
        
        assert_eq!(ref_ptr.ref_count(), 1);
        assert!(ref_ptr.is_unique());
        
        let cloned = ref_ptr.clone_ref().unwrap();
        assert_eq!(ref_ptr.ref_count(), 2);
        assert!(!ref_ptr.is_unique());
        
        drop(cloned);
        assert_eq!(ref_ptr.ref_count(), 1);
        assert!(ref_ptr.is_unique());
    }

    #[test]
    fn test_memory_pool() {
        let pool = MemoryPool::new(64, 8).unwrap();
        
        let ptr1 = pool.allocate().unwrap();
        let ptr2 = pool.allocate().unwrap();
        
        pool.deallocate(ptr1);
        pool.deallocate(ptr2);
        
        let (total, free) = pool.stats();
        assert_eq!(total, 2);
        assert_eq!(free, 2);
    }

    #[test]
    fn test_safe_memory_access() {
        let data = vec![1u32, 2, 3, 4, 5];
        let ptr = NonNull::new(data.as_ptr() as *mut u32).unwrap();
        let mut access = SafeMemoryAccess::new(ptr, data.len() * size_of::<u32>());
        
        let slice = access.get_slice(5).unwrap();
        assert_eq!(slice, &[1, 2, 3, 4, 5]);
        
        // Test bounds checking
        assert!(access.get_slice(6).is_err());
    }
}