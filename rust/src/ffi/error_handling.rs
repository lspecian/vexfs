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

//! FFI-Specific Error Handling and Recovery System
//!
//! This module provides specialized error handling for FFI operations including:
//! - Circuit breakers for FFI calls prone to hanging
//! - Timeout mechanisms for long-running operations
//! - Fallback strategies when FFI calls fail
//! - Comprehensive logging for FFI error diagnosis

use core::ffi::c_int;
use core::sync::atomic::{AtomicU64, AtomicU32, Ordering};

#[cfg(feature = "kernel")]
use alloc::{string::{String, ToString}, format};
#[cfg(not(feature = "kernel"))]
use std::string::String;

use crate::shared::errors::{VexfsError, VexfsResult};
use crate::shared::error_handling::{
    EnhancedError, CircuitBreaker, CircuitBreakerConfig, RetryMechanism, RetryConfig,
    ErrorCorrelationId, ErrorSeverity
};

/// FFI operation types for circuit breaker categorization
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FfiOperationType {
    /// Superblock operations
    Superblock,
    /// Inode operations
    Inode,
    /// File operations
    File,
    /// Directory operations
    Directory,
    /// Vector operations
    Vector,
    /// Storage operations
    Storage,
    /// Synchronization operations
    Sync,
}

/// FFI error handler with circuit breakers and recovery mechanisms
pub struct FfiErrorHandler {
    /// Circuit breakers for different operation types
    circuit_breakers: [CircuitBreaker; 7], // One for each FfiOperationType
    /// Retry mechanism for transient failures
    retry_mechanism: RetryMechanism,
    /// Global error statistics
    error_stats: FfiErrorStats,
}

/// FFI error statistics
#[derive(Debug, Default)]
pub struct FfiErrorStats {
    /// Total FFI calls made
    pub total_calls: AtomicU64,
    /// Total FFI failures
    pub total_failures: AtomicU64,
    /// Circuit breaker trips
    pub circuit_breaker_trips: AtomicU64,
    /// Successful retries
    pub successful_retries: AtomicU64,
    /// Fallback activations
    pub fallback_activations: AtomicU64,
    /// Timeout occurrences
    pub timeout_occurrences: AtomicU64,
}

impl FfiErrorHandler {
    /// Create a new FFI error handler with default configuration
    pub fn new() -> Self {
        Self::with_config(
            CircuitBreakerConfig::default(),
            RetryConfig::default(),
        )
    }
    
    /// Create a new FFI error handler with custom configuration
    pub fn with_config(
        circuit_config: CircuitBreakerConfig,
        retry_config: RetryConfig,
    ) -> Self {
        Self {
            circuit_breakers: [
                CircuitBreaker::with_config(circuit_config.clone()),
                CircuitBreaker::with_config(circuit_config.clone()),
                CircuitBreaker::with_config(circuit_config.clone()),
                CircuitBreaker::with_config(circuit_config.clone()),
                CircuitBreaker::with_config(circuit_config.clone()),
                CircuitBreaker::with_config(circuit_config.clone()),
                CircuitBreaker::with_config(circuit_config),
            ],
            retry_mechanism: RetryMechanism::with_config(retry_config),
            error_stats: FfiErrorStats::default(),
        }
    }
    
    /// Execute an FFI operation with comprehensive error handling
    pub fn execute_ffi_operation<F, T>(
        &self,
        operation_type: FfiOperationType,
        operation_name: &str,
        operation: F,
    ) -> FfiResult<T>
    where
        F: Fn() -> FfiResult<T>,
    {
        let correlation_id = ErrorCorrelationId::new();
        self.error_stats.total_calls.fetch_add(1, Ordering::Relaxed);
        
        // Check circuit breaker
        let breaker_index = operation_type as usize;
        let circuit_breaker = &self.circuit_breakers[breaker_index];
        
        if !circuit_breaker.is_call_allowed() {
            self.error_stats.circuit_breaker_trips.fetch_add(1, Ordering::Relaxed);
            self.log_error(
                correlation_id,
                operation_name,
                "Circuit breaker open - operation blocked",
                ErrorSeverity::High,
            );
            return Err(FfiError::CircuitBreakerOpen(operation_type));
        }
        
        // Execute with retry mechanism
        let result = self.retry_mechanism.execute(|| {
            match operation() {
                Ok(value) => Ok(value),
                Err(ffi_error) => {
                    // Convert FFI error to VexFS error for retry logic
                    let vexfs_error = ffi_error.to_vexfs_error();
                    Err(vexfs_error)
                }
            }
        });
        
        match result {
            Ok(value) => {
                // Record success in circuit breaker
                circuit_breaker.record_success();
                self.log_success(correlation_id, operation_name);
                Ok(value)
            },
            Err(vexfs_error) => {
                // Record failure in circuit breaker
                circuit_breaker.record_failure();
                self.error_stats.total_failures.fetch_add(1, Ordering::Relaxed);
                
                // Create enhanced error with context
                let enhanced_error = EnhancedError::new(
                    vexfs_error.clone(),
                    operation_name,
                    &format!("FFI operation type: {:?}", operation_type),
                );
                
                self.log_error(
                    correlation_id,
                    operation_name,
                    &enhanced_error.to_string(),
                    enhanced_error.context.severity,
                );
                
                // Convert back to FFI error
                Err(FfiError::from_vexfs_error(vexfs_error))
            }
        }
    }
    
    /// Execute an FFI operation with fallback
    pub fn execute_with_fallback<F, G, T>(
        &self,
        operation_type: FfiOperationType,
        operation_name: &str,
        primary_operation: F,
        fallback_operation: G,
    ) -> FfiResult<T>
    where
        F: Fn() -> FfiResult<T>,
        G: Fn() -> FfiResult<T>,
    {
        match self.execute_ffi_operation(operation_type, operation_name, primary_operation) {
            Ok(result) => Ok(result),
            Err(ffi_error) => {
                // Check if fallback should be used
                if self.should_use_fallback(&ffi_error) {
                    self.error_stats.fallback_activations.fetch_add(1, Ordering::Relaxed);
                    
                    let correlation_id = ErrorCorrelationId::new();
                    self.log_info(
                        correlation_id,
                        operation_name,
                        "Using fallback operation due to primary failure",
                    );
                    
                    fallback_operation()
                } else {
                    Err(ffi_error)
                }
            }
        }
    }
    
    /// Check if fallback should be used for the given error
    fn should_use_fallback(&self, error: &FfiError) -> bool {
        match error {
            FfiError::CircuitBreakerOpen(_) => true,
            FfiError::Timeout(_) => true,
            FfiError::VexfsError(vexfs_error) => {
                matches!(vexfs_error,
                    VexfsError::ResourceBusy |
                    VexfsError::IoError(_) |
                    VexfsError::OutOfMemory
                )
            },
            _ => false,
        }
    }
    
    /// Get circuit breaker statistics for an operation type
    pub fn get_circuit_breaker_stats(&self, operation_type: FfiOperationType) -> crate::shared::error_handling::CircuitBreakerStats {
        let breaker_index = operation_type as usize;
        self.circuit_breakers[breaker_index].stats()
    }
    
    /// Get overall FFI error statistics
    pub fn get_error_stats(&self) -> FfiErrorStatsSnapshot {
        FfiErrorStatsSnapshot {
            total_calls: self.error_stats.total_calls.load(Ordering::Relaxed),
            total_failures: self.error_stats.total_failures.load(Ordering::Relaxed),
            circuit_breaker_trips: self.error_stats.circuit_breaker_trips.load(Ordering::Relaxed),
            successful_retries: self.error_stats.successful_retries.load(Ordering::Relaxed),
            fallback_activations: self.error_stats.fallback_activations.load(Ordering::Relaxed),
            timeout_occurrences: self.error_stats.timeout_occurrences.load(Ordering::Relaxed),
        }
    }
    
    /// Log successful operation
    fn log_success(&self, correlation_id: ErrorCorrelationId, operation_name: &str) {
        #[cfg(feature = "kernel")]
        {
            // Use kernel logging
            // printk!(KERN_DEBUG "VexFS FFI: [{}] {} succeeded\n", correlation_id, operation_name);
        }
        
        #[cfg(not(feature = "kernel"))]
        {
            println!("VexFS FFI: [{}] {} succeeded", correlation_id, operation_name);
        }
    }
    
    /// Log error with severity
    fn log_error(
        &self,
        correlation_id: ErrorCorrelationId,
        operation_name: &str,
        error_message: &str,
        severity: ErrorSeverity,
    ) {
        let severity_str = match severity {
            ErrorSeverity::Low => "DEBUG",
            ErrorSeverity::Medium => "INFO",
            ErrorSeverity::High => "WARNING",
            ErrorSeverity::Critical => "ERR",
        };
        
        #[cfg(feature = "kernel")]
        {
            // Use kernel logging with appropriate level
            // printk!(KERN_{} "VexFS FFI: [{}] {} failed: {}\n", severity_str, correlation_id, operation_name, error_message);
        }
        
        #[cfg(not(feature = "kernel"))]
        {
            println!("VexFS FFI {}: [{}] {} failed: {}", severity_str, correlation_id, operation_name, error_message);
        }
    }
    
    /// Log informational message
    fn log_info(&self, correlation_id: ErrorCorrelationId, operation_name: &str, message: &str) {
        #[cfg(feature = "kernel")]
        {
            // printk!(KERN_INFO "VexFS FFI: [{}] {}: {}\n", correlation_id, operation_name, message);
        }
        
        #[cfg(not(feature = "kernel"))]
        {
            println!("VexFS FFI INFO: [{}] {}: {}", correlation_id, operation_name, message);
        }
    }
}

/// FFI error statistics snapshot
#[derive(Debug, Clone)]
pub struct FfiErrorStatsSnapshot {
    pub total_calls: u64,
    pub total_failures: u64,
    pub circuit_breaker_trips: u64,
    pub successful_retries: u64,
    pub fallback_activations: u64,
    pub timeout_occurrences: u64,
}

/// FFI-specific error types
#[derive(Debug, Clone)]
pub enum FfiError {
    /// Circuit breaker is open for the operation type
    CircuitBreakerOpen(FfiOperationType),
    /// Operation timed out
    Timeout(String),
    /// FFI call returned invalid result
    InvalidResult(String),
    /// Underlying VexFS error
    VexfsError(VexfsError),
}

impl FfiError {
    /// Convert VexFS error to FFI error
    pub fn from_vexfs_error(error: VexfsError) -> Self {
        Self::VexfsError(error)
    }
    
    /// Convert FFI error to VexFS error
    pub fn to_vexfs_error(&self) -> VexfsError {
        match self {
            FfiError::CircuitBreakerOpen(_) => VexfsError::ResourceBusy,
            FfiError::Timeout(msg) => VexfsError::Timeout(msg.clone()),
            FfiError::InvalidResult(msg) => VexfsError::InvalidData(msg.clone()),
            FfiError::VexfsError(error) => error.clone(),
        }
    }
    
    /// Convert FFI error to C error code
    pub fn to_c_error_code(&self) -> c_int {
        match self {
            FfiError::CircuitBreakerOpen(_) => super::VEXFS_ERROR_GENERIC,
            FfiError::Timeout(_) => -110, // ETIMEDOUT
            FfiError::InvalidResult(_) => super::VEXFS_ERROR_INVAL,
            FfiError::VexfsError(error) => error.to_kernel_errno(),
        }
    }
}

impl core::fmt::Display for FfiError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            FfiError::CircuitBreakerOpen(op_type) => {
                write!(f, "Circuit breaker open for {:?} operations", op_type)
            },
            FfiError::Timeout(msg) => write!(f, "FFI operation timed out: {}", msg),
            FfiError::InvalidResult(msg) => write!(f, "FFI returned invalid result: {}", msg),
            FfiError::VexfsError(error) => write!(f, "VexFS error in FFI: {}", error),
        }
    }
}

/// Result type for FFI operations
pub type FfiResult<T> = Result<T, FfiError>;

/// Global FFI error handler instance
static mut GLOBAL_FFI_ERROR_HANDLER: Option<FfiErrorHandler> = None;
static FFI_HANDLER_INITIALIZED: AtomicU32 = AtomicU32::new(0);

/// Initialize the global FFI error handler
pub fn initialize_ffi_error_handler() -> VexfsResult<()> {
    if FFI_HANDLER_INITIALIZED.compare_exchange(0, 1, Ordering::Relaxed, Ordering::Relaxed).is_ok() {
        unsafe {
            GLOBAL_FFI_ERROR_HANDLER = Some(FfiErrorHandler::new());
        }
        Ok(())
    } else {
        Err(VexfsError::AlreadyExists)
    }
}

/// Get the global FFI error handler
pub fn get_ffi_error_handler() -> Option<&'static FfiErrorHandler> {
    if FFI_HANDLER_INITIALIZED.load(Ordering::Relaxed) == 1 {
        unsafe { GLOBAL_FFI_ERROR_HANDLER.as_ref() }
    } else {
        None
    }
}

/// Cleanup the global FFI error handler
pub fn cleanup_ffi_error_handler() {
    if FFI_HANDLER_INITIALIZED.compare_exchange(1, 0, Ordering::Relaxed, Ordering::Relaxed).is_ok() {
        unsafe {
            GLOBAL_FFI_ERROR_HANDLER = None;
        }
    }
}

/// Macro for safe FFI calls with comprehensive error handling
#[macro_export]
macro_rules! safe_ffi_call {
    ($operation_type:expr, $operation_name:expr, $operation:expr) => {{
        use $crate::ffi::error_handling::{get_ffi_error_handler, FfiError};
        
        if let Some(handler) = get_ffi_error_handler() {
            handler.execute_ffi_operation($operation_type, $operation_name, || $operation)
        } else {
            // Fallback if handler not initialized
            match $operation {
                Ok(result) => Ok(result),
                Err(error) => Err(FfiError::from_vexfs_error(error)),
            }
        }
    }};
}

/// Macro for safe FFI calls with fallback
#[macro_export]
macro_rules! safe_ffi_call_with_fallback {
    ($operation_type:expr, $operation_name:expr, $primary:expr, $fallback:expr) => {{
        use $crate::ffi::error_handling::{get_ffi_error_handler, FfiError};
        
        if let Some(handler) = get_ffi_error_handler() {
            handler.execute_with_fallback($operation_type, $operation_name, || $primary, || $fallback)
        } else {
            // Fallback if handler not initialized
            match $primary {
                Ok(result) => Ok(result),
                Err(_) => $fallback,
            }
        }
    }};
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_ffi_error_handler_creation() {
        let handler = FfiErrorHandler::new();
        let stats = handler.get_error_stats();
        assert_eq!(stats.total_calls, 0);
        assert_eq!(stats.total_failures, 0);
    }
    
    #[test]
    fn test_ffi_error_conversion() {
        let vexfs_error = VexfsError::FileNotFound;
        let ffi_error = FfiError::from_vexfs_error(vexfs_error.clone());
        let converted_back = ffi_error.to_vexfs_error();
        assert_eq!(converted_back, vexfs_error);
    }
    
    #[test]
    fn test_circuit_breaker_integration() {
        let handler = FfiErrorHandler::new();
        
        // Simulate successful operation
        let result = handler.execute_ffi_operation(
            FfiOperationType::File,
            "test_operation",
            || Ok(42),
        );
        assert!(result.is_ok());
        
        let stats = handler.get_error_stats();
        assert_eq!(stats.total_calls, 1);
        assert_eq!(stats.total_failures, 0);
    }
    
    #[test]
    fn test_fallback_mechanism() {
        let handler = FfiErrorHandler::new();
        
        let result = handler.execute_with_fallback(
            FfiOperationType::File,
            "test_operation",
            || Err(FfiError::Timeout("test timeout".to_string())),
            || Ok(42),
        );
        
        assert_eq!(result.unwrap(), 42);
        
        let stats = handler.get_error_stats();
        assert_eq!(stats.fallback_activations, 1);
    }
}