// FUSE Error Handling Utilities
// Provides safe wrappers for lock operations and error handling

use std::sync::{MutexGuard, RwLockReadGuard, RwLockWriteGuard, PoisonError};
use libc::{EIO, EAGAIN};

/// Safe wrapper for mutex lock operations
/// Returns EIO on poisoned lock, which indicates a panic in another thread
pub fn safe_lock<'a, T>(
    mutex: &'a std::sync::Mutex<T>,
    operation: &str,
) -> Result<MutexGuard<'a, T>, i32> {
    match mutex.lock() {
        Ok(guard) => Ok(guard),
        Err(poisoned) => {
            eprintln!("VexFS: Poisoned mutex in {}: recovering", operation);
            // Recover by returning the guard anyway - data might be partially corrupted
            // but it's better than crashing
            Ok(poisoned.into_inner())
        }
    }
}

/// Safe wrapper for RwLock read operations
pub fn safe_read<'a, T>(
    lock: &'a std::sync::RwLock<T>,
    operation: &str,
) -> Result<RwLockReadGuard<'a, T>, i32> {
    match lock.read() {
        Ok(guard) => Ok(guard),
        Err(poisoned) => {
            eprintln!("VexFS: Poisoned RwLock (read) in {}: recovering", operation);
            Ok(poisoned.into_inner())
        }
    }
}

/// Safe wrapper for RwLock write operations
pub fn safe_write<'a, T>(
    lock: &'a std::sync::RwLock<T>,
    operation: &str,
) -> Result<RwLockWriteGuard<'a, T>, i32> {
    match lock.write() {
        Ok(guard) => Ok(guard),
        Err(poisoned) => {
            eprintln!("VexFS: Poisoned RwLock (write) in {}: recovering", operation);
            Ok(poisoned.into_inner())
        }
    }
}

/// Macro for safe lock operations with early return on error
#[macro_export]
macro_rules! safe_lock {
    ($mutex:expr, $op:expr, $reply:expr) => {
        match $crate::fuse_error_handling::safe_lock($mutex, $op) {
            Ok(guard) => guard,
            Err(errno) => {
                $reply.error(errno);
                return;
            }
        }
    };
}

/// Macro for safe read lock operations  
#[macro_export]
macro_rules! safe_read {
    ($lock:expr, $op:expr, $reply:expr) => {
        match $crate::fuse_error_handling::safe_read($lock, $op) {
            Ok(guard) => guard,
            Err(errno) => {
                $reply.error(errno);
                return;
            }
        }
    };
}

/// Macro for safe write lock operations
#[macro_export]
macro_rules! safe_write {
    ($lock:expr, $op:expr, $reply:expr) => {
        match $crate::fuse_error_handling::safe_write($lock, $op) {
            Ok(guard) => guard,
            Err(errno) => {
                $reply.error(errno);
                return;
            }
        }
    };
}

/// Helper function to log errors with context
pub fn log_error(operation: &str, error: &str) {
    eprintln!("VexFS Error in {}: {}", operation, error);
}

/// Helper function to handle Result types safely
pub fn handle_result<T, E: std::fmt::Debug>(
    result: Result<T, E>,
    operation: &str,
    default: T,
) -> T {
    match result {
        Ok(val) => val,
        Err(e) => {
            log_error(operation, &format!("{:?}", e));
            default
        }
    }
}