// Comprehensive Error Handling System for VexFS
// Provides unified error types, recovery strategies, and error reporting

use std::fmt;
use std::error::Error;
use std::io;
use std::sync::PoisonError;
use serde::{Serialize, Deserialize};

/// Main error type for VexFS operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VexfsError {
    // Filesystem errors
    FileNotFound(String),
    FileExists(String),
    NotADirectory(String),
    NotAFile(String),
    DirectoryNotEmpty(String),
    PermissionDenied(String),
    InvalidPath(String),
    
    // I/O errors
    IoError(String),
    DiskFull,
    ReadOnly,
    CorruptedData(String),
    
    // VexGraph errors
    GraphNodeNotFound(u64),
    GraphEdgeNotFound(u64),
    GraphStorageError(String),
    GraphIntegrityError(String),
    InvalidGraphOperation(String),
    
    // Storage errors
    StorageBackendError(String),
    StorageInitError(String),
    StorageFlushError(String),
    SerializationError(String),
    
    // Concurrency errors
    LockPoisoned(String),
    DeadlockDetected,
    ConcurrentModification,
    
    // System errors
    OutOfMemory,
    SystemCallFailed(String),
    KernelError(String),
    
    // Configuration errors
    InvalidConfiguration(String),
    MissingConfiguration(String),
    
    // Network errors (for API)
    NetworkError(String),
    ConnectionRefused,
    Timeout,
    
    // Unknown/Other
    Unknown(String),
    Internal(String),
}

impl fmt::Display for VexfsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VexfsError::FileNotFound(path) => write!(f, "File not found: {}", path),
            VexfsError::FileExists(path) => write!(f, "File already exists: {}", path),
            VexfsError::NotADirectory(path) => write!(f, "Not a directory: {}", path),
            VexfsError::NotAFile(path) => write!(f, "Not a file: {}", path),
            VexfsError::DirectoryNotEmpty(path) => write!(f, "Directory not empty: {}", path),
            VexfsError::PermissionDenied(msg) => write!(f, "Permission denied: {}", msg),
            VexfsError::InvalidPath(path) => write!(f, "Invalid path: {}", path),
            
            VexfsError::IoError(msg) => write!(f, "I/O error: {}", msg),
            VexfsError::DiskFull => write!(f, "Disk full"),
            VexfsError::ReadOnly => write!(f, "Filesystem is read-only"),
            VexfsError::CorruptedData(msg) => write!(f, "Corrupted data: {}", msg),
            
            VexfsError::GraphNodeNotFound(id) => write!(f, "Graph node not found: {}", id),
            VexfsError::GraphEdgeNotFound(id) => write!(f, "Graph edge not found: {}", id),
            VexfsError::GraphStorageError(msg) => write!(f, "Graph storage error: {}", msg),
            VexfsError::GraphIntegrityError(msg) => write!(f, "Graph integrity error: {}", msg),
            VexfsError::InvalidGraphOperation(msg) => write!(f, "Invalid graph operation: {}", msg),
            
            VexfsError::StorageBackendError(msg) => write!(f, "Storage backend error: {}", msg),
            VexfsError::StorageInitError(msg) => write!(f, "Storage initialization error: {}", msg),
            VexfsError::StorageFlushError(msg) => write!(f, "Storage flush error: {}", msg),
            VexfsError::SerializationError(msg) => write!(f, "Serialization error: {}", msg),
            
            VexfsError::LockPoisoned(msg) => write!(f, "Lock poisoned: {}", msg),
            VexfsError::DeadlockDetected => write!(f, "Deadlock detected"),
            VexfsError::ConcurrentModification => write!(f, "Concurrent modification detected"),
            
            VexfsError::OutOfMemory => write!(f, "Out of memory"),
            VexfsError::SystemCallFailed(msg) => write!(f, "System call failed: {}", msg),
            VexfsError::KernelError(msg) => write!(f, "Kernel error: {}", msg),
            
            VexfsError::InvalidConfiguration(msg) => write!(f, "Invalid configuration: {}", msg),
            VexfsError::MissingConfiguration(msg) => write!(f, "Missing configuration: {}", msg),
            
            VexfsError::NetworkError(msg) => write!(f, "Network error: {}", msg),
            VexfsError::ConnectionRefused => write!(f, "Connection refused"),
            VexfsError::Timeout => write!(f, "Operation timed out"),
            
            VexfsError::Unknown(msg) => write!(f, "Unknown error: {}", msg),
            VexfsError::Internal(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

impl Error for VexfsError {}

// Conversion from standard errors

impl From<io::Error> for VexfsError {
    fn from(error: io::Error) -> Self {
        match error.kind() {
            io::ErrorKind::NotFound => VexfsError::FileNotFound(error.to_string()),
            io::ErrorKind::AlreadyExists => VexfsError::FileExists(error.to_string()),
            io::ErrorKind::PermissionDenied => VexfsError::PermissionDenied(error.to_string()),
            io::ErrorKind::ConnectionRefused => VexfsError::ConnectionRefused,
            io::ErrorKind::TimedOut => VexfsError::Timeout,
            _ => VexfsError::IoError(error.to_string()),
        }
    }
}

impl<T> From<PoisonError<T>> for VexfsError {
    fn from(error: PoisonError<T>) -> Self {
        VexfsError::LockPoisoned(error.to_string())
    }
}

impl From<serde_json::Error> for VexfsError {
    fn from(error: serde_json::Error) -> Self {
        VexfsError::SerializationError(error.to_string())
    }
}

/// Convert VexFS errors to FUSE errno codes
impl From<VexfsError> for libc::c_int {
    fn from(error: VexfsError) -> Self {
        match error {
            VexfsError::FileNotFound(_) => libc::ENOENT,
            VexfsError::FileExists(_) => libc::EEXIST,
            VexfsError::NotADirectory(_) => libc::ENOTDIR,
            VexfsError::NotAFile(_) => libc::EISDIR,
            VexfsError::DirectoryNotEmpty(_) => libc::ENOTEMPTY,
            VexfsError::PermissionDenied(_) => libc::EPERM,
            VexfsError::InvalidPath(_) => libc::EINVAL,
            
            VexfsError::IoError(_) => libc::EIO,
            VexfsError::DiskFull => libc::ENOSPC,
            VexfsError::ReadOnly => libc::EROFS,
            VexfsError::CorruptedData(_) => libc::EIO,
            
            VexfsError::OutOfMemory => libc::ENOMEM,
            VexfsError::DeadlockDetected => libc::EDEADLK,
            
            VexfsError::Timeout => libc::ETIMEDOUT,
            VexfsError::ConnectionRefused => libc::ECONNREFUSED,
            
            _ => libc::EIO, // Generic I/O error for unhandled cases
        }
    }
}

/// Result type for VexFS operations
pub type VexfsResult<T> = Result<T, VexfsError>;

/// Error recovery strategies
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryStrategy {
    Retry,
    Fallback,
    Abort,
    Ignore,
    Log,
    Panic,
}

/// Error context for detailed error reporting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorContext {
    pub operation: String,
    pub path: Option<String>,
    pub inode: Option<u64>,
    pub timestamp: std::time::SystemTime,
    pub thread_id: String,
    pub backtrace: Option<String>,
}

impl ErrorContext {
    pub fn new(operation: impl Into<String>) -> Self {
        Self {
            operation: operation.into(),
            path: None,
            inode: None,
            timestamp: std::time::SystemTime::now(),
            thread_id: format!("{:?}", std::thread::current().id()),
            backtrace: None,
        }
    }
    
    pub fn with_path(mut self, path: impl Into<String>) -> Self {
        self.path = Some(path.into());
        self
    }
    
    pub fn with_inode(mut self, inode: u64) -> Self {
        self.inode = Some(inode);
        self
    }
    
    pub fn with_backtrace(mut self) -> Self {
        self.backtrace = Some(format!("{:?}", std::backtrace::Backtrace::capture()));
        self
    }
}

/// Error handler for managing error recovery
pub struct ErrorHandler {
    strategies: std::collections::HashMap<String, RecoveryStrategy>,
    error_log: std::sync::Arc<std::sync::Mutex<Vec<(VexfsError, ErrorContext)>>>,
    max_retries: usize,
}

impl ErrorHandler {
    pub fn new() -> Self {
        let mut strategies = std::collections::HashMap::new();
        
        // Default strategies for different error types
        strategies.insert("FileNotFound".to_string(), RecoveryStrategy::Log);
        strategies.insert("FileExists".to_string(), RecoveryStrategy::Ignore);
        strategies.insert("IoError".to_string(), RecoveryStrategy::Retry);
        strategies.insert("LockPoisoned".to_string(), RecoveryStrategy::Fallback);
        strategies.insert("OutOfMemory".to_string(), RecoveryStrategy::Abort);
        strategies.insert("DeadlockDetected".to_string(), RecoveryStrategy::Retry);
        
        Self {
            strategies,
            error_log: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
            max_retries: 3,
        }
    }
    
    /// Handle an error with the appropriate recovery strategy
    pub fn handle_error(&self, error: VexfsError, context: ErrorContext) -> RecoveryStrategy {
        // Log the error
        if let Ok(mut log) = self.error_log.lock() {
            log.push((error.clone(), context.clone()));
            
            // Keep only last 1000 errors
            if log.len() > 1000 {
                log.drain(0..100);
            }
        }
        
        // Determine strategy based on error type
        let error_type = match &error {
            VexfsError::FileNotFound(_) => "FileNotFound",
            VexfsError::FileExists(_) => "FileExists",
            VexfsError::IoError(_) => "IoError",
            VexfsError::LockPoisoned(_) => "LockPoisoned",
            VexfsError::OutOfMemory => "OutOfMemory",
            VexfsError::DeadlockDetected => "DeadlockDetected",
            _ => "Unknown",
        };
        
        self.strategies.get(error_type)
            .copied()
            .unwrap_or(RecoveryStrategy::Log)
    }
    
    /// Execute an operation with retry logic
    pub fn with_retry<T, F>(&self, mut operation: F) -> VexfsResult<T>
    where
        F: FnMut() -> VexfsResult<T>,
    {
        let mut attempts = 0;
        
        loop {
            match operation() {
                Ok(result) => return Ok(result),
                Err(error) => {
                    attempts += 1;
                    
                    if attempts >= self.max_retries {
                        return Err(error);
                    }
                    
                    // Exponential backoff
                    std::thread::sleep(std::time::Duration::from_millis(
                        100 * (1 << attempts)
                    ));
                }
            }
        }
    }
    
    /// Get error statistics
    pub fn get_error_stats(&self) -> ErrorStats {
        let log = self.error_log.lock().unwrap_or_else(|e| e.into_inner());
        
        let mut stats = ErrorStats::default();
        stats.total_errors = log.len();
        
        for (error, _) in log.iter() {
            match error {
                VexfsError::FileNotFound(_) => stats.file_not_found += 1,
                VexfsError::IoError(_) => stats.io_errors += 1,
                VexfsError::LockPoisoned(_) => stats.lock_errors += 1,
                VexfsError::OutOfMemory => stats.memory_errors += 1,
                _ => stats.other_errors += 1,
            }
        }
        
        stats
    }
}

/// Error statistics for monitoring
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ErrorStats {
    pub total_errors: usize,
    pub file_not_found: usize,
    pub io_errors: usize,
    pub lock_errors: usize,
    pub memory_errors: usize,
    pub other_errors: usize,
}

/// Macros for error handling

/// Create an error with context
#[macro_export]
macro_rules! vexfs_error {
    ($error:expr, $context:expr) => {
        {
            let err = $error;
            eprintln!("VexFS Error: {} in {}", err, $context.operation);
            err
        }
    };
}

/// Try an operation and convert to VexFS error
#[macro_export]
macro_rules! vexfs_try {
    ($expr:expr) => {
        match $expr {
            Ok(val) => val,
            Err(e) => return Err(VexfsError::from(e)),
        }
    };
    ($expr:expr, $error:expr) => {
        match $expr {
            Ok(val) => val,
            Err(_) => return Err($error),
        }
    };
}

/// Ensure a condition is true or return an error
#[macro_export]
macro_rules! vexfs_ensure {
    ($cond:expr, $error:expr) => {
        if !$cond {
            return Err($error);
        }
    };
}

/// Safe unwrap with error context
#[macro_export]
macro_rules! safe_unwrap {
    ($expr:expr, $context:expr) => {
        match $expr {
            Some(val) => val,
            None => {
                let ctx = ErrorContext::new($context);
                return Err(VexfsError::Internal(format!("Unwrap failed in {}", $context)));
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_error_conversion() {
        let io_error = io::Error::new(io::ErrorKind::NotFound, "test file");
        let vexfs_error: VexfsError = io_error.into();
        
        match vexfs_error {
            VexfsError::FileNotFound(_) => assert!(true),
            _ => assert!(false, "Wrong error type"),
        }
    }
    
    #[test]
    fn test_error_handler() {
        let handler = ErrorHandler::new();
        
        let error = VexfsError::FileNotFound("/test.txt".to_string());
        let context = ErrorContext::new("test_operation");
        
        let strategy = handler.handle_error(error, context);
        assert_eq!(strategy, RecoveryStrategy::Log);
    }
    
    #[test]
    fn test_retry_logic() {
        let handler = ErrorHandler::new();
        let mut counter = 0;
        
        let result = handler.with_retry(|| {
            counter += 1;
            if counter < 3 {
                Err(VexfsError::IoError("temporary failure".to_string()))
            } else {
                Ok(42)
            }
        });
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
        assert_eq!(counter, 3);
    }
}