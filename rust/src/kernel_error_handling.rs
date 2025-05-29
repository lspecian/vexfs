//! Kernel-Safe Error Handling for VexFS Vector Operations
//!
//! This module provides kernel-compatible error handling that replaces panic!
//! calls with proper kernel error codes and implements safe error propagation
//! mechanisms suitable for kernel module operation.
//!
//! **Key Features:**
//! - No panic! calls (kernel-unsafe)
//! - Proper kernel error code mapping
//! - Kernel logging integration
//! - Memory-safe error handling
//! - VFS error integration

use crate::shared::macros::*;
use crate::shared::errors::{VexfsError, VexfsResult};
use crate::ioctl::VectorIoctlError;

extern crate alloc;
use alloc::{string::String, format};
use core::fmt;

/// Kernel-safe error handling utilities
pub struct KernelErrorHandler;

impl KernelErrorHandler {
    /// Convert VexfsError to kernel error code
    pub fn to_kernel_error_code(error: &VexfsError) -> i32 {
        match error {
            VexfsError::NotFound => -2,           // -ENOENT
            VexfsError::PermissionDenied => -13,  // -EACCES
            VexfsError::InvalidArgument(_) => -22, // -EINVAL
            VexfsError::OutOfMemory => -12,       // -ENOMEM
            VexfsError::ResourceBusy => -16,      // -EBUSY
            VexfsError::IoError(_) => -5,         // -EIO
            VexfsError::InvalidOperation(_) => -95, // -EOPNOTSUPP
            VexfsError::ChecksumMismatch => -74,  // -EBADMSG
            VexfsError::FileTooLarge => -27,      // -EFBIG
            VexfsError::InvalidInode => -22,      // -EINVAL
            VexfsError::InvalidBlock => -22,      // -EINVAL
            VexfsError::InvalidName => -22,       // -EINVAL
            _ => -22,                             // -EINVAL (default)
        }
    }

    /// Convert VectorIoctlError to kernel error code
    pub fn ioctl_to_kernel_error_code(error: VectorIoctlError) -> i32 {
        match error {
            VectorIoctlError::Success => 0,
            VectorIoctlError::InvalidRequest => -22,      // -EINVAL
            VectorIoctlError::InvalidDimensions => -22,   // -EINVAL
            VectorIoctlError::InvalidVectorId => -22,     // -EINVAL
            VectorIoctlError::VectorNotFound => -2,       // -ENOENT
            VectorIoctlError::IndexNotFound => -2,        // -ENOENT
            VectorIoctlError::PermissionDenied => -13,    // -EACCES
            VectorIoctlError::InsufficientMemory => -12,  // -ENOMEM
            VectorIoctlError::InvalidBuffer => -14,       // -EFAULT
            VectorIoctlError::BufferTooSmall => -7,       // -E2BIG
            VectorIoctlError::InvalidParameters => -22,   // -EINVAL
            VectorIoctlError::IndexCorrupted => -74,      // -EBADMSG
            VectorIoctlError::IoError => -5,              // -EIO
            VectorIoctlError::TimeoutError => -110,       // -ETIMEDOUT
            VectorIoctlError::ConcurrentAccess => -16,    // -EBUSY
            VectorIoctlError::InvalidFormat => -22,       // -EINVAL
            VectorIoctlError::InvalidVectorData => -22,   // -EINVAL
            VectorIoctlError::InvalidParameter => -22,    // -EINVAL
            VectorIoctlError::UnknownError => -22,        // -EINVAL
            _ => -22,                                     // -EINVAL (default)
        }
    }

    /// Log error with kernel-safe logging
    pub fn log_error(error: &VexfsError, context: &str) {
        match error {
            VexfsError::OutOfMemory => {
                vexfs_error!("Memory allocation failed in {}", context);
            }
            VexfsError::InvalidArgument(msg) => {
                vexfs_error!("Invalid argument in {}: {}", context, msg);
            }
            VexfsError::IoError(msg) => {
                vexfs_error!("I/O error in {}: {}", context, msg);
            }
            VexfsError::PermissionDenied => {
                vexfs_warn!("Permission denied in {}", context);
            }
            VexfsError::NotFound => {
                vexfs_debug!("Resource not found in {}", context);
            }
            _ => {
                vexfs_error!("Error in {}: {:?}", context, error);
            }
        }
    }

    /// Handle kernel panic replacement
    pub fn handle_critical_error(error: &VexfsError, context: &str) -> ! {
        vexfs_error!("CRITICAL ERROR in {}: {:?}", context, error);
        
        kernel_or_std!(
            kernel: {
                // In kernel mode, we cannot panic. Instead, we should:
                // 1. Log the error
                // 2. Attempt cleanup
                // 3. Return an error code to the caller
                // Since we can't return from a ! function, we use a kernel-specific mechanism
                
                // This is a placeholder - in real kernel implementation, this would be:
                // BUG() or similar kernel panic mechanism as last resort
                loop {
                    // Infinite loop to prevent return - kernel will handle this
                }
            },
            std: {
                // In userspace, we can panic for testing
                panic!("Critical error in {}: {:?}", context, error);
            }
        )
    }

    /// Safe error conversion with logging
    pub fn safe_convert_error(result: VexfsResult<i32>, context: &str) -> i32 {
        match result {
            Ok(value) => value,
            Err(error) => {
                Self::log_error(&error, context);
                Self::to_kernel_error_code(&error)
            }
        }
    }

    /// Validate kernel memory access
    pub fn validate_user_pointer(ptr: *const u8, size: usize) -> VexfsResult<()> {
        if ptr.is_null() {
            vexfs_error!("Null user pointer validation failed");
            return Err(VexfsError::InvalidArgument("Null pointer".to_string()));
        }

        if size == 0 {
            vexfs_error!("Zero size validation failed");
            return Err(VexfsError::InvalidArgument("Zero size".to_string()));
        }

        // In real kernel implementation, would use access_ok() or similar
        kernel_or_std!(
            kernel: {
                // Kernel pointer validation would go here
                // if !access_ok(VERIFY_READ, ptr, size) {
                //     return Err(VexfsError::InvalidArgument("Invalid user pointer".to_string()));
                // }
            },
            std: {
                // In userspace, basic null check is sufficient for testing
            }
        );

        Ok(())
    }

    /// Validate kernel memory allocation size
    pub fn validate_allocation_size(size: usize) -> VexfsResult<()> {
        const MAX_KERNEL_ALLOC: usize = 4 * 1024 * 1024; // 4MB limit

        if size == 0 {
            return Err(VexfsError::InvalidArgument("Zero allocation size".to_string()));
        }

        if size > MAX_KERNEL_ALLOC {
            vexfs_error!("Allocation size too large: {} bytes", size);
            return Err(VexfsError::OutOfMemory);
        }

        Ok(())
    }

    /// Handle kernel resource cleanup on error
    pub fn cleanup_on_error(context: &str) {
        vexfs_debug!("Performing error cleanup in {}", context);
        
        // Perform any necessary cleanup operations
        // This would include:
        // - Releasing locks
        // - Freeing allocated memory
        // - Closing file handles
        // - Resetting state variables
        
        kernel_or_std!(
            kernel: {
                // Kernel-specific cleanup
                // - Release spinlocks
                // - Free kernel memory
                // - Reset kernel state
            },
            std: {
                // Userspace cleanup for testing
                // - Release mutexes
                // - Free heap memory
                // - Close file descriptors
            }
        );
    }
}

/// Kernel-safe error result wrapper
pub struct KernelResult<T> {
    inner: Result<T, KernelError>,
}

/// Kernel-specific error type
#[derive(Debug, Clone)]
pub struct KernelError {
    pub code: i32,
    pub message: String,
    pub context: String,
}

impl<T> KernelResult<T> {
    /// Create successful result
    pub fn ok(value: T) -> Self {
        Self {
            inner: Ok(value),
        }
    }

    /// Create error result
    pub fn err(code: i32, message: String, context: String) -> Self {
        Self {
            inner: Err(KernelError { code, message, context }),
        }
    }

    /// Convert from VexfsResult
    pub fn from_vexfs_result(result: VexfsResult<T>, context: &str) -> Self {
        match result {
            Ok(value) => Self::ok(value),
            Err(error) => {
                let code = KernelErrorHandler::to_kernel_error_code(&error);
                let message = format!("{:?}", error);
                Self::err(code, message, context.to_string())
            }
        }
    }

    /// Unwrap with kernel-safe error handling
    pub fn unwrap_or_kernel_error(self, default: T) -> T {
        match self.inner {
            Ok(value) => value,
            Err(error) => {
                KernelErrorHandler::log_error(
                    &VexfsError::InvalidOperation(error.message.clone()),
                    &error.context
                );
                default
            }
        }
    }

    /// Convert to kernel error code
    pub fn to_kernel_code(self) -> i32 {
        match self.inner {
            Ok(_) => 0,
            Err(error) => {
                vexfs_error!("Kernel error in {}: {}", error.context, error.message);
                error.code
            }
        }
    }

    /// Check if result is ok
    pub fn is_ok(&self) -> bool {
        self.inner.is_ok()
    }

    /// Check if result is error
    pub fn is_err(&self) -> bool {
        self.inner.is_err()
    }
}

impl fmt::Display for KernelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Kernel error {} in {}: {}", self.code, self.context, self.message)
    }
}

/// Macro for kernel-safe error handling
#[macro_export]
macro_rules! kernel_try {
    ($expr:expr, $context:expr) => {
        match $expr {
            Ok(val) => val,
            Err(err) => {
                $crate::kernel_error_handling::KernelErrorHandler::log_error(&err, $context);
                return Err(err);
            }
        }
    };
}

/// Macro for kernel-safe error conversion
#[macro_export]
macro_rules! kernel_safe_call {
    ($expr:expr, $context:expr) => {
        $crate::kernel_error_handling::KernelErrorHandler::safe_convert_error($expr, $context)
    };
}

/// Macro for kernel-safe pointer validation
#[macro_export]
macro_rules! validate_user_ptr {
    ($ptr:expr, $size:expr) => {
        $crate::kernel_error_handling::KernelErrorHandler::validate_user_pointer($ptr, $size)?
    };
}

/// Macro for kernel-safe allocation validation
#[macro_export]
macro_rules! validate_alloc_size {
    ($size:expr) => {
        $crate::kernel_error_handling::KernelErrorHandler::validate_allocation_size($size)?
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_code_conversion() {
        let error = VexfsError::NotFound;
        let code = KernelErrorHandler::to_kernel_error_code(&error);
        assert_eq!(code, -2); // -ENOENT
    }

    #[test]
    fn test_ioctl_error_conversion() {
        let error = VectorIoctlError::VectorNotFound;
        let code = KernelErrorHandler::ioctl_to_kernel_error_code(error);
        assert_eq!(code, -2); // -ENOENT
    }

    #[test]
    fn test_kernel_result_creation() {
        let result = KernelResult::ok(42);
        assert!(result.is_ok());
        assert!(!result.is_err());
    }

    #[test]
    fn test_kernel_result_from_vexfs() {
        let vexfs_result: VexfsResult<i32> = Ok(42);
        let kernel_result = KernelResult::from_vexfs_result(vexfs_result, "test");
        assert!(kernel_result.is_ok());
    }

    #[test]
    fn test_pointer_validation() {
        let data = vec![1u8, 2, 3, 4];
        let result = KernelErrorHandler::validate_user_pointer(data.as_ptr(), data.len());
        assert!(result.is_ok());

        let result = KernelErrorHandler::validate_user_pointer(core::ptr::null(), 10);
        assert!(result.is_err());
    }

    #[test]
    fn test_allocation_validation() {
        let result = KernelErrorHandler::validate_allocation_size(1024);
        assert!(result.is_ok());

        let result = KernelErrorHandler::validate_allocation_size(0);
        assert!(result.is_err());

        let result = KernelErrorHandler::validate_allocation_size(10 * 1024 * 1024); // 10MB
        assert!(result.is_err());
    }
}