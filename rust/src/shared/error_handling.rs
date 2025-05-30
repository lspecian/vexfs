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

//! Comprehensive Error Handling and Recovery System for VexFS
//!
//! This module provides advanced error handling capabilities including:
//! - Circuit breakers for operations prone to hanging
//! - Automatic retry mechanisms for transient failures
//! - Error correlation and tracking
//! - Recovery strategies for different error types
//! - Comprehensive logging and monitoring

use core::fmt;
use core::sync::atomic::{AtomicU64, AtomicU32, AtomicBool, Ordering};

#[cfg(not(feature = "kernel"))]
use std::time::{Duration, Instant};
#[cfg(feature = "kernel")]
use alloc::{
    string::{String, ToString},
    vec::Vec,
    vec,
};
#[cfg(not(feature = "kernel"))]
use std::string::String;

use crate::shared::errors::{VexfsError, VexfsResult};

/// Error correlation ID for tracking related errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ErrorCorrelationId(u64);

impl ErrorCorrelationId {
    /// Generate a new correlation ID
    pub fn new() -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        Self(COUNTER.fetch_add(1, Ordering::Relaxed))
    }
    
    /// Get the correlation ID value
    pub fn value(&self) -> u64 {
        self.0
    }
}

impl fmt::Display for ErrorCorrelationId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CORR-{:016X}", self.0)
    }
}

/// Error severity levels for classification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorSeverity {
    /// Low severity - operation can continue normally
    Low,
    /// Medium severity - operation can continue with degraded functionality
    Medium,
    /// High severity - operation should be retried or failed gracefully
    High,
    /// Critical severity - system integrity may be at risk
    Critical,
}

/// Error context with additional metadata
#[derive(Debug, Clone)]
pub struct ErrorContext {
    /// Correlation ID for tracking related errors
    pub correlation_id: ErrorCorrelationId,
    /// Error severity level
    pub severity: ErrorSeverity,
    /// Operation that caused the error
    pub operation: String,
    /// Additional context information
    pub context: String,
    /// Stack trace or call path (if available)
    pub stack_trace: Option<String>,
    /// Retry count for this error
    pub retry_count: u32,
    /// Whether this error is recoverable
    pub recoverable: bool,
    /// Recovery hints for handling this error
    pub recovery_hints: Vec<RecoveryHint>,
}

/// Recovery hints for different error types
#[derive(Debug, Clone, PartialEq)]
pub enum RecoveryHint {
    /// Retry the operation after a delay
    RetryAfterDelay(u64), // milliseconds
    /// Use fallback mechanism
    UseFallback,
    /// Invalidate cache and retry
    InvalidateCache,
    /// Reduce operation scope
    ReduceScope,
    /// Switch to read-only mode
    ReadOnlyMode,
    /// Restart component
    RestartComponent,
    /// Manual intervention required
    ManualIntervention,
}

/// Enhanced error type with context and recovery information
#[derive(Debug, Clone)]
pub struct EnhancedError {
    /// Original VexFS error
    pub error: VexfsError,
    /// Error context and metadata
    pub context: ErrorContext,
    /// Timestamp when error occurred (kernel ticks or system time)
    pub timestamp: u64,
}

impl EnhancedError {
    /// Create a new enhanced error with context
    pub fn new(error: VexfsError, operation: &str, context: &str) -> Self {
        let correlation_id = ErrorCorrelationId::new();
        let severity = Self::classify_severity(&error);
        let recoverable = error.is_recoverable();
        let recovery_hints = Self::generate_recovery_hints(&error);
        
        Self {
            error,
            context: ErrorContext {
                correlation_id,
                severity,
                operation: operation.to_string(),
                context: context.to_string(),
                stack_trace: None,
                retry_count: 0,
                recoverable,
                recovery_hints,
            },
            timestamp: Self::current_timestamp(),
        }
    }
    
    /// Add retry information to the error
    pub fn with_retry(mut self, retry_count: u32) -> Self {
        self.context.retry_count = retry_count;
        self
    }
    
    /// Add stack trace information
    pub fn with_stack_trace(mut self, stack_trace: String) -> Self {
        self.context.stack_trace = Some(stack_trace);
        self
    }
    
    /// Classify error severity based on error type
    fn classify_severity(error: &VexfsError) -> ErrorSeverity {
        match error {
            // Critical errors that affect system integrity
            VexfsError::CorruptedData | 
            VexfsError::ChecksumMismatch |
            VexfsError::InvalidSuperblock => ErrorSeverity::Critical,
            
            // High severity errors that prevent operation completion
            VexfsError::OutOfMemory |
            VexfsError::OutOfSpace |
            VexfsError::IoError(_) |
            VexfsError::StorageError(_) => ErrorSeverity::High,
            
            // Medium severity errors that can be worked around
            VexfsError::ResourceBusy |
            VexfsError::LockConflict(_) |
            VexfsError::TransactionError(_) => ErrorSeverity::Medium,
            
            // Low severity errors that are expected in normal operation
            VexfsError::FileNotFound |
            VexfsError::InodeNotFound(_) |
            VexfsError::NotFound => ErrorSeverity::Low,
            
            // Default to medium for unknown errors
            _ => ErrorSeverity::Medium,
        }
    }
    
    /// Generate recovery hints based on error type
    fn generate_recovery_hints(error: &VexfsError) -> Vec<RecoveryHint> {
        match error {
            // Transient errors - retry with delay
            VexfsError::ResourceBusy |
            VexfsError::LockConflict(_) => vec![
                RecoveryHint::RetryAfterDelay(100),
                RecoveryHint::ReduceScope,
            ],
            
            // I/O errors - retry and fallback
            VexfsError::IoError(_) => vec![
                RecoveryHint::RetryAfterDelay(500),
                RecoveryHint::InvalidateCache,
                RecoveryHint::UseFallback,
            ],
            
            // Memory errors - reduce scope and retry
            VexfsError::OutOfMemory => vec![
                RecoveryHint::ReduceScope,
                RecoveryHint::InvalidateCache,
                RecoveryHint::RetryAfterDelay(1000),
            ],
            
            // Storage errors - fallback and read-only
            VexfsError::OutOfSpace => vec![
                RecoveryHint::ReduceScope,
                RecoveryHint::ReadOnlyMode,
            ],
            
            // Corruption errors - manual intervention
            VexfsError::CorruptedData |
            VexfsError::ChecksumMismatch => vec![
                RecoveryHint::ManualIntervention,
                RecoveryHint::ReadOnlyMode,
            ],
            
            // Transaction errors - retry and restart
            VexfsError::TransactionError(_) => vec![
                RecoveryHint::RetryAfterDelay(200),
                RecoveryHint::RestartComponent,
            ],
            
            // Default recovery strategy
            _ => vec![RecoveryHint::RetryAfterDelay(100)],
        }
    }
    
    /// Get current timestamp (kernel ticks or system time)
    fn current_timestamp() -> u64 {
        #[cfg(feature = "kernel")]
        {
            // In kernel mode, use jiffies or similar
            // For now, use a simple counter
            static COUNTER: AtomicU64 = AtomicU64::new(0);
            COUNTER.fetch_add(1, Ordering::Relaxed)
        }
        
        #[cfg(not(feature = "kernel"))]
        {
            // In userspace, use system time
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64
        }
    }
}

impl fmt::Display for EnhancedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {} in {}: {} (severity: {:?}, retries: {})",
               self.context.correlation_id,
               self.error,
               self.context.operation,
               self.context.context,
               self.context.severity,
               self.context.retry_count)
    }
}

/// Circuit breaker states
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CircuitBreakerState {
    /// Circuit is closed - operations are allowed
    Closed,
    /// Circuit is open - operations are blocked
    Open,
    /// Circuit is half-open - testing if service has recovered
    HalfOpen,
}

/// Circuit breaker for preventing cascading failures
#[derive(Debug)]
pub struct CircuitBreaker {
    /// Current state of the circuit breaker
    state: AtomicU32, // 0=Closed, 1=Open, 2=HalfOpen
    /// Failure count in current window
    failure_count: AtomicU32,
    /// Success count in half-open state
    success_count: AtomicU32,
    /// Last failure timestamp
    last_failure_time: AtomicU64,
    /// Configuration
    config: CircuitBreakerConfig,
}

/// Circuit breaker configuration
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Failure threshold to open circuit
    pub failure_threshold: u32,
    /// Success threshold to close circuit from half-open
    pub success_threshold: u32,
    /// Timeout before trying half-open (milliseconds)
    pub timeout_ms: u64,
    /// Window size for failure counting (milliseconds)
    pub window_ms: u64,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            success_threshold: 3,
            timeout_ms: 60000, // 1 minute
            window_ms: 10000,  // 10 seconds
        }
    }
}

impl CircuitBreaker {
    /// Create a new circuit breaker with default configuration
    pub fn new() -> Self {
        Self::with_config(CircuitBreakerConfig::default())
    }
    
    /// Create a new circuit breaker with custom configuration
    pub fn with_config(config: CircuitBreakerConfig) -> Self {
        Self {
            state: AtomicU32::new(0), // Closed
            failure_count: AtomicU32::new(0),
            success_count: AtomicU32::new(0),
            last_failure_time: AtomicU64::new(0),
            config,
        }
    }
    
    /// Get current circuit breaker state
    pub fn state(&self) -> CircuitBreakerState {
        match self.state.load(Ordering::Relaxed) {
            0 => CircuitBreakerState::Closed,
            1 => CircuitBreakerState::Open,
            2 => CircuitBreakerState::HalfOpen,
            _ => CircuitBreakerState::Closed, // Default fallback
        }
    }
    
    /// Check if operation is allowed
    pub fn is_call_allowed(&self) -> bool {
        let current_time = EnhancedError::current_timestamp();
        
        match self.state() {
            CircuitBreakerState::Closed => true,
            CircuitBreakerState::Open => {
                // Check if timeout has elapsed
                let last_failure = self.last_failure_time.load(Ordering::Relaxed);
                if current_time.saturating_sub(last_failure) >= self.config.timeout_ms {
                    // Try to transition to half-open
                    if self.state.compare_exchange(1, 2, Ordering::Relaxed, Ordering::Relaxed).is_ok() {
                        self.success_count.store(0, Ordering::Relaxed);
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            },
            CircuitBreakerState::HalfOpen => true,
        }
    }
    
    /// Record a successful operation
    pub fn record_success(&self) {
        match self.state() {
            CircuitBreakerState::Closed => {
                // Reset failure count on success
                self.failure_count.store(0, Ordering::Relaxed);
            },
            CircuitBreakerState::HalfOpen => {
                let success_count = self.success_count.fetch_add(1, Ordering::Relaxed) + 1;
                if success_count >= self.config.success_threshold {
                    // Transition back to closed
                    self.state.store(0, Ordering::Relaxed);
                    self.failure_count.store(0, Ordering::Relaxed);
                    self.success_count.store(0, Ordering::Relaxed);
                }
            },
            CircuitBreakerState::Open => {
                // Should not happen, but reset if it does
                self.state.store(0, Ordering::Relaxed);
                self.failure_count.store(0, Ordering::Relaxed);
            },
        }
    }
    
    /// Record a failed operation
    pub fn record_failure(&self) {
        let current_time = EnhancedError::current_timestamp();
        self.last_failure_time.store(current_time, Ordering::Relaxed);
        
        match self.state() {
            CircuitBreakerState::Closed => {
                let failure_count = self.failure_count.fetch_add(1, Ordering::Relaxed) + 1;
                if failure_count >= self.config.failure_threshold {
                    // Transition to open
                    self.state.store(1, Ordering::Relaxed);
                }
            },
            CircuitBreakerState::HalfOpen => {
                // Transition back to open on any failure
                self.state.store(1, Ordering::Relaxed);
                self.success_count.store(0, Ordering::Relaxed);
            },
            CircuitBreakerState::Open => {
                // Already open, just update timestamp
            },
        }
    }
    
    /// Get circuit breaker statistics
    pub fn stats(&self) -> CircuitBreakerStats {
        CircuitBreakerStats {
            state: self.state(),
            failure_count: self.failure_count.load(Ordering::Relaxed),
            success_count: self.success_count.load(Ordering::Relaxed),
            last_failure_time: self.last_failure_time.load(Ordering::Relaxed),
        }
    }
}

/// Circuit breaker statistics
#[derive(Debug, Clone)]
pub struct CircuitBreakerStats {
    pub state: CircuitBreakerState,
    pub failure_count: u32,
    pub success_count: u32,
    pub last_failure_time: u64,
}

/// Retry configuration for automatic retry mechanisms
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retry attempts
    pub max_attempts: u32,
    /// Base delay between retries (milliseconds)
    pub base_delay_ms: u64,
    /// Maximum delay between retries (milliseconds)
    pub max_delay_ms: u64,
    /// Backoff multiplier for exponential backoff
    pub backoff_multiplier: f32,
    /// Jitter factor to avoid thundering herd (0.0 to 1.0)
    pub jitter_factor: f32,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            base_delay_ms: 100,
            max_delay_ms: 5000,
            backoff_multiplier: 2.0,
            jitter_factor: 0.1,
        }
    }
}

/// Retry mechanism with exponential backoff and jitter
pub struct RetryMechanism {
    config: RetryConfig,
}

impl RetryMechanism {
    /// Create a new retry mechanism with default configuration
    pub fn new() -> Self {
        Self::with_config(RetryConfig::default())
    }
    
    /// Create a new retry mechanism with custom configuration
    pub fn with_config(config: RetryConfig) -> Self {
        Self { config }
    }
    
    /// Execute an operation with automatic retry
    pub fn execute<F, T>(&self, mut operation: F) -> VexfsResult<T>
    where
        F: FnMut() -> VexfsResult<T>,
    {
        let mut last_error = None;
        
        for attempt in 0..self.config.max_attempts {
            match operation() {
                Ok(result) => return Ok(result),
                Err(error) => {
                    // Check if error is retryable
                    if !error.is_recoverable() {
                        return Err(error);
                    }
                    
                    last_error = Some(error);
                    
                    // Don't delay after the last attempt
                    if attempt < self.config.max_attempts - 1 {
                        let delay = self.calculate_delay(attempt);
                        self.sleep(delay);
                    }
                }
            }
        }
        
        // All retries exhausted
        Err(last_error.unwrap_or(VexfsError::InternalError("Retry exhausted".to_string())))
    }
    
    /// Calculate delay for the given attempt with exponential backoff and jitter
    fn calculate_delay(&self, attempt: u32) -> u64 {
        let base_delay = self.config.base_delay_ms as f32;
        let exponential_delay = base_delay * self.config.backoff_multiplier* self.config.backoff_multiplier;
        let capped_delay = exponential_delay.min(self.config.max_delay_ms as f32);
        
        // Add jitter to avoid thundering herd
        let jitter_range = capped_delay * self.config.jitter_factor;
        let jitter = (attempt as f32 * 17.0) % jitter_range; // Simple pseudo-random jitter
        
        (capped_delay + jitter) as u64
    }
    
    /// Sleep for the specified duration
    fn sleep(&self, duration_ms: u64) {
        #[cfg(not(feature = "kernel"))]
        {
            std::thread::sleep(Duration::from_millis(duration_ms));
        }
        
        #[cfg(feature = "kernel")]
        {
            // In kernel mode, use appropriate delay mechanism
            // For now, just a simple busy wait (not ideal for production)
            let start = EnhancedError::current_timestamp();
            while EnhancedError::current_timestamp().saturating_sub(start) < duration_ms {
                // Yield CPU
                core::hint::spin_loop();
            }
        }
    }
}

/// Error aggregator for collecting and analyzing multiple errors
#[derive(Debug)]
pub struct ErrorAggregator {
    /// Collected errors
    errors: Vec<EnhancedError>,
    /// Maximum number of errors to keep
    max_errors: usize,
}

impl ErrorAggregator {
    /// Create a new error aggregator
    pub fn new(max_errors: usize) -> Self {
        Self {
            errors: Vec::new(),
            max_errors,
        }
    }
    
    /// Add an error to the aggregator
    pub fn add_error(&mut self, error: EnhancedError) {
        self.errors.push(error);
        
        // Keep only the most recent errors
        if self.errors.len() > self.max_errors {
            self.errors.remove(0);
        }
    }
    
    /// Get all errors
    pub fn errors(&self) -> &[EnhancedError] {
        &self.errors
    }
    
    /// Get errors by severity
    pub fn errors_by_severity(&self, severity: ErrorSeverity) -> Vec<&EnhancedError> {
        self.errors.iter()
            .filter(|e| e.context.severity == severity)
            .collect()
    }
    
    /// Get errors by correlation ID
    pub fn errors_by_correlation(&self, correlation_id: ErrorCorrelationId) -> Vec<&EnhancedError> {
        self.errors.iter()
            .filter(|e| e.context.correlation_id == correlation_id)
            .collect()
    }
    
    /// Clear all errors
    pub fn clear(&mut self) {
        self.errors.clear();
    }
    
    /// Get error statistics
    pub fn stats(&self) -> ErrorStats {
        let mut stats = ErrorStats::default();
        
        for error in &self.errors {
            match error.context.severity {
                ErrorSeverity::Low => stats.low_severity_count += 1,
                ErrorSeverity::Medium => stats.medium_severity_count += 1,
                ErrorSeverity::High => stats.high_severity_count += 1,
                ErrorSeverity::Critical => stats.critical_severity_count += 1,
            }
            
            if error.context.recoverable {
                stats.recoverable_count += 1;
            } else {
                stats.non_recoverable_count += 1;
            }
        }
        
        stats.total_count = self.errors.len();
        stats
    }
}

/// Error statistics
#[derive(Debug, Default)]
pub struct ErrorStats {
    pub total_count: usize,
    pub low_severity_count: usize,
    pub medium_severity_count: usize,
    pub high_severity_count: usize,
    pub critical_severity_count: usize,
    pub recoverable_count: usize,
    pub non_recoverable_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_error_correlation_id() {
        let id1 = ErrorCorrelationId::new();
        let id2 = ErrorCorrelationId::new();
        assert_ne!(id1, id2);
        assert!(id1.value() > 0);
        assert!(id2.value() > id1.value());
    }
    
    #[test]
    fn test_enhanced_error_creation() {
        let error = VexfsError::FileNotFound;
        let enhanced = EnhancedError::new(error.clone(), "test_operation", "test context");
        
        assert_eq!(enhanced.error, error);
        assert_eq!(enhanced.context.operation, "test_operation");
        assert_eq!(enhanced.context.context, "test context");
        assert_eq!(enhanced.context.severity, ErrorSeverity::Low);
        assert!(!enhanced.context.recoverable);
    }
    
    #[test]
    fn test_circuit_breaker_states() {
        let breaker = CircuitBreaker::new();
        assert_eq!(breaker.state(), CircuitBreakerState::Closed);
        assert!(breaker.is_call_allowed());
        
        // Record failures to open circuit
        for _ in 0..5 {
            breaker.record_failure();
        }
        assert_eq!(breaker.state(), CircuitBreakerState::Open);
        assert!(!breaker.is_call_allowed());
    }
    
    #[test]
    fn test_retry_mechanism() {
        let retry = RetryMechanism::new();
        let mut attempt_count = 0;
        
        let result = retry.execute(|| {
            attempt_count += 1;
            if attempt_count < 3 {
                Err(VexfsError::ResourceBusy)
            } else {
                Ok(42)
            }
        });
        
        assert_eq!(result.unwrap(), 42);
        assert_eq!(attempt_count, 3);
    }
    
    #[test]
    fn test_error_aggregator() {
        let mut aggregator = ErrorAggregator::new(10);
        
        let error1 = EnhancedError::new(VexfsError::FileNotFound, "op1", "context1");
        let error2 = EnhancedError::new(VexfsError::OutOfMemory, "op2", "context2");
        
        aggregator.add_error(error1);
        aggregator.add_error(error2);
        
        assert_eq!(aggregator.errors().len(), 2);
        
        let stats = aggregator.stats();
        assert_eq!(stats.total_count, 2);
        assert_eq!(stats.low_severity_count, 1);
        assert_eq!(stats.high_severity_count, 1);
    }
}