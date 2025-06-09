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

//! Error types for VexFS Shared Domain
//!
//! This module defines a comprehensive error handling system for VexFS,
//! designed to work in both kernel and userspace environments.

use core::fmt;

#[cfg(not(feature = "kernel"))]
use std::string::String;
#[cfg(feature = "kernel")]
use alloc::string::String;

#[cfg(feature = "kernel")]
use alloc::format;
#[cfg(not(feature = "kernel"))]
use std::format;

// =======================
// Core Error Types
// =======================

/// Main error type for VexFS operations
#[derive(Debug, Clone, PartialEq)]
pub enum VexfsError {
    // I/O and Storage Errors
    IoError(IoErrorKind),
    StorageError(StorageErrorKind),
    
    // Filesystem Errors
    InvalidMagic,
    InvalidSuperblock,
    CorruptedData,
    ChecksumMismatch,
    UnsupportedVersion,
    
    // Inode and File Errors
    InodeNotFound(u64),
    InvalidInode(u64),
    FileTooLarge,
    FileNotFound,
    DirectoryNotEmpty,
    NotADirectory(u64),
    IsADirectory,
    
    // Vector-specific Errors
    VectorError(VectorErrorKind),
    
    // Memory and Resource Errors
    OutOfMemory,
    OutOfSpace,
    StackOverflow,
    ResourceBusy,
    Busy,
    TooManyOpenFiles,
    
    // Concurrency and Locking Errors
    LockError,
    LockConflict(String),
    WouldBlock,
    
    // Permission and Access Errors
    PermissionDenied(String),
    ReadOnlyFilesystem,
    InvalidOperation(String),
    
    // Index and Search Errors
    IndexError(IndexErrorKind),
    SearchError(SearchErrorKind),
    
    // Journal and Transaction Errors
    JournalError(JournalErrorKind),
    TransactionError(TransactionErrorKind),
    
    // Configuration and Validation Errors
    InvalidConfiguration,
    InvalidArgument(String),
    InvalidParameter(String),
    OutOfRange(String),
    
    // Path and Data Errors
    InvalidData(String),
    InvalidPath(String),
    PathTooLong,
    NameTooLong,
    
    // Additional File System Errors
    NotFound,
    NotMounted,
    NoSpace,
    EntryNotFound(String),
    DeviceFull,
    
    // Cache and Storage Errors
    CacheError,
    CacheLocked,
    CacheDirty,
    AllocationError,
    CorruptionError,
    ChecksumError,
    
    // Version and Operation Errors
    VersionMismatch,
    UnsupportedOperation,
    NeedsFsck,
    IOError,
    InsufficientPermissions,
    Internal(String),
    
    // Internal Errors
    InternalError(String),
    NotImplemented(String),
    InitializationFailed,
    
    // Generic errors with context
    Other(String),
    
    // FFI compatibility aliases
    /// No space left on device (FFI alias for OutOfSpace)
    NoSpaceLeft,
    
    /// File exists (FFI alias for AlreadyExists)
    FileExists,
    
    /// Not a directory (FFI alias for NotADirectory)
    NotDirectory,
    
    /// Is a directory (FFI alias for IsADirectory)
    IsDirectory,
    
    /// File already exists
    AlreadyExists,
    
    /// Operation timeout
    Timeout(String),
    
    /// Resource limit exceeded
    ResourceLimit(String),
    
    /// Operation cancelled
    OperationCancelled(String),
}

/// I/O error kinds
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IoErrorKind {
    ReadError,
    WriteError,
    SeekError,
    FlushError,
    DeviceError,
    TimeoutError,
    InterruptedError,
    ConnectionLost,
    BufferTooSmall,
    InvalidOffset,
}

/// Storage-related error kinds
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StorageErrorKind {
    BlockNotFound(u64),
    BlockCorrupted(u64),
    AllocationFailed,
    DeallocationFailed,
    FragmentationError,
    BadBlockTable,
    MetadataCorrupted,
}

/// Vector operation error kinds
#[derive(Debug, Clone, PartialEq)]
pub enum VectorErrorKind {
    InvalidDimensions(u16),
    InvalidDimension(u16), // Alias for InvalidDimensions
    DimensionMismatch { expected: u16, found: u16 },
    InvalidComponent(f32),
    VectorTooLarge,
    VectorNotFound,
    MetadataTooLarge,
    SerializationError,
    DeserializationError,
    NormalizationError,
    SearchError,
    IndexError,
    // Additional variants needed by other modules
    InvalidVectorId,
    CorruptedData,
    NoSpace,
    IoError,
    InvalidVersion,
    ChecksumMismatch,
    FileNotFound,
    CompressionError,
    MetadataError,
    AlignmentError,
}

/// Index operation error kinds
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IndexErrorKind {
    IndexNotFound,
    IndexCorrupted,
    IndexFull,
    IndexBuildFailed,
    InvalidIndexType,
    InvalidParameters,
    NodeNotFound,
    LayerNotFound,
    ConnectionError,
}

/// Search operation error kinds
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SearchErrorKind {
    InvalidQuery,
    InvalidK(usize),
    InvalidEf(usize),
    NoResults,
    SearchTimeout,
    SearchInterrupted,
    InvalidFilter,
    InvalidMetric,
}

/// Journal operation error kinds
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JournalErrorKind {
    JournalNotFound,
    JournalCorrupted,
    JournalFull,
    InvalidRecord,
    RecordNotFound,
    WriteAheadLogError,
    CheckpointError,
    RecoveryError,
}

/// Transaction error kinds
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransactionErrorKind {
    TransactionNotFound,
    TransactionAborted,
    TransactionConflict,
    DeadlockDetected,
    InvalidTransactionState,
    TooManyTransactions,
    TransactionTimeout,
    CommitFailed,
    RollbackFailed,
}

// =======================
// Result Type
// =======================

/// Result type alias for VexFS operations
pub type VexfsResult<T> = Result<T, VexfsError>;

// =======================
// Error Implementation
// =======================

impl fmt::Display for VexfsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VexfsError::IoError(kind) => write!(f, "I/O error: {}", kind),
            VexfsError::StorageError(kind) => write!(f, "Storage error: {}", kind),
            VexfsError::InvalidMagic => write!(f, "Invalid filesystem magic number"),
            VexfsError::InvalidSuperblock => write!(f, "Invalid or corrupted superblock"),
            VexfsError::CorruptedData => write!(f, "Data corruption detected"),
            VexfsError::ChecksumMismatch => write!(f, "Checksum verification failed"),
            VexfsError::UnsupportedVersion => write!(f, "Unsupported filesystem version"),
            VexfsError::InodeNotFound(ino) => write!(f, "Inode {} not found", ino),
            VexfsError::InvalidInode(ino) => write!(f, "Invalid inode {}", ino),
            VexfsError::FileTooLarge => write!(f, "File size exceeds maximum limit"),
            VexfsError::FileNotFound => write!(f, "File not found"),
            VexfsError::DirectoryNotEmpty => write!(f, "Directory not empty"),
            VexfsError::NotADirectory(ino) => write!(f, "Not a directory: inode {}", ino),
            VexfsError::IsADirectory => write!(f, "Is a directory"),
            VexfsError::VectorError(kind) => write!(f, "Vector error: {}", kind),
            VexfsError::OutOfMemory => write!(f, "Out of memory"),
            VexfsError::OutOfSpace => write!(f, "No space left on device"),
            VexfsError::StackOverflow => write!(f, "Stack overflow detected"),
            VexfsError::ResourceBusy => write!(f, "Resource busy"),
            VexfsError::Busy => write!(f, "Resource busy"),
            VexfsError::TooManyOpenFiles => write!(f, "Too many open files"),
            VexfsError::PermissionDenied(msg) => write!(f, "Permission denied: {}", msg),
            VexfsError::ReadOnlyFilesystem => write!(f, "Read-only filesystem"),
            VexfsError::InvalidOperation(msg) => write!(f, "Invalid operation: {}", msg),
            VexfsError::IndexError(kind) => write!(f, "Index error: {}", kind),
            VexfsError::SearchError(kind) => write!(f, "Search error: {}", kind),
            VexfsError::JournalError(kind) => write!(f, "Journal error: {}", kind),
            VexfsError::TransactionError(kind) => write!(f, "Transaction error: {}", kind),
            VexfsError::InvalidConfiguration => write!(f, "Invalid configuration"),
            VexfsError::InvalidArgument(msg) => write!(f, "Invalid argument: {}", msg),
            VexfsError::InvalidParameter(msg) => write!(f, "Invalid parameter: {}", msg),
            VexfsError::OutOfRange(msg) => write!(f, "Value out of range: {}", msg),
            VexfsError::InvalidData(msg) => write!(f, "Invalid data: {}", msg),
            VexfsError::InvalidPath(msg) => write!(f, "Invalid path: {}", msg),
            VexfsError::PathTooLong => write!(f, "Path too long"),
            VexfsError::NameTooLong => write!(f, "Name too long"),
            VexfsError::NotFound => write!(f, "Not found"),
            VexfsError::NotMounted => write!(f, "Not mounted"),
            VexfsError::NoSpace => write!(f, "No space left"),
            VexfsError::EntryNotFound(name) => write!(f, "Entry not found: {}", name),
            VexfsError::DeviceFull => write!(f, "Device full"),
            VexfsError::CacheError => write!(f, "Cache error"),
            VexfsError::CacheLocked => write!(f, "Cache locked"),
            VexfsError::CacheDirty => write!(f, "Cache dirty"),
            VexfsError::AllocationError => write!(f, "Allocation error"),
            VexfsError::CorruptionError => write!(f, "Corruption error"),
            VexfsError::ChecksumError => write!(f, "Checksum error"),
            VexfsError::VersionMismatch => write!(f, "Version mismatch"),
            VexfsError::UnsupportedOperation => write!(f, "Unsupported operation"),
            VexfsError::NeedsFsck => write!(f, "Needs fsck"),
            VexfsError::IOError => write!(f, "I/O error"),
            VexfsError::InsufficientPermissions => write!(f, "Insufficient permissions"),
            VexfsError::Internal(msg) => write!(f, "Internal: {}", msg),
            VexfsError::InternalError(msg) => write!(f, "Internal error: {}", msg),
            VexfsError::NotImplemented(msg) => write!(f, "Not implemented: {}", msg),
            VexfsError::InitializationFailed => write!(f, "Initialization failed"),
            VexfsError::Other(msg) => write!(f, "{}", msg),
            VexfsError::LockError => write!(f, "Lock error"),
            VexfsError::LockConflict(msg) => write!(f, "Lock conflict: {}", msg),
            VexfsError::WouldBlock => write!(f, "Operation would block"),
            VexfsError::NoSpaceLeft => write!(f, "No space left on device"),
            VexfsError::FileExists => write!(f, "File exists"),
            VexfsError::NotDirectory => write!(f, "Not a directory"),
            VexfsError::IsDirectory => write!(f, "Is a directory"),
            VexfsError::AlreadyExists => write!(f, "File already exists"),
            VexfsError::Timeout(msg) => write!(f, "Operation timed out: {}", msg),
            VexfsError::ResourceLimit(msg) => write!(f, "Resource limit exceeded: {}", msg),
            VexfsError::OperationCancelled(msg) => write!(f, "Operation cancelled: {}", msg),
        }
    }
}

impl fmt::Display for IoErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IoErrorKind::ReadError => write!(f, "read operation failed"),
            IoErrorKind::WriteError => write!(f, "write operation failed"),
            IoErrorKind::SeekError => write!(f, "seek operation failed"),
            IoErrorKind::FlushError => write!(f, "flush operation failed"),
            IoErrorKind::DeviceError => write!(f, "device error"),
            IoErrorKind::TimeoutError => write!(f, "operation timed out"),
            IoErrorKind::InterruptedError => write!(f, "operation interrupted"),
            IoErrorKind::ConnectionLost => write!(f, "connection lost"),
            IoErrorKind::BufferTooSmall => write!(f, "buffer too small"),
            IoErrorKind::InvalidOffset => write!(f, "invalid offset"),
        }
    }
}

impl fmt::Display for StorageErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StorageErrorKind::BlockNotFound(block) => write!(f, "block {} not found", block),
            StorageErrorKind::BlockCorrupted(block) => write!(f, "block {} corrupted", block),
            StorageErrorKind::AllocationFailed => write!(f, "block allocation failed"),
            StorageErrorKind::DeallocationFailed => write!(f, "block deallocation failed"),
            StorageErrorKind::FragmentationError => write!(f, "storage fragmentation error"),
            StorageErrorKind::BadBlockTable => write!(f, "bad block table"),
            StorageErrorKind::MetadataCorrupted => write!(f, "storage metadata corrupted"),
        }
    }
}

impl fmt::Display for VectorErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VectorErrorKind::InvalidDimensions(dims) => write!(f, "invalid dimensions: {}", dims),
            VectorErrorKind::InvalidDimension(dims) => write!(f, "invalid dimension: {}", dims),
            VectorErrorKind::DimensionMismatch { expected, found } => {
                write!(f, "dimension mismatch: expected {}, found {}", expected, found)
            }
            VectorErrorKind::InvalidComponent(val) => write!(f, "invalid component: {}", val),
            VectorErrorKind::VectorTooLarge => write!(f, "vector too large"),
            VectorErrorKind::VectorNotFound => write!(f, "vector not found"),
            VectorErrorKind::MetadataTooLarge => write!(f, "vector metadata too large"),
            VectorErrorKind::SerializationError => write!(f, "vector serialization failed"),
            VectorErrorKind::DeserializationError => write!(f, "vector deserialization failed"),
            VectorErrorKind::NormalizationError => write!(f, "vector normalization failed"),
            VectorErrorKind::SearchError => write!(f, "vector search error"),
            VectorErrorKind::IndexError => write!(f, "vector index error"),
            VectorErrorKind::InvalidVectorId => write!(f, "invalid vector ID"),
            VectorErrorKind::CorruptedData => write!(f, "vector data corrupted"),
            VectorErrorKind::NoSpace => write!(f, "no space for vector storage"),
            VectorErrorKind::IoError => write!(f, "vector I/O error"),
            VectorErrorKind::InvalidVersion => write!(f, "invalid vector format version"),
            VectorErrorKind::ChecksumMismatch => write!(f, "vector checksum mismatch"),
            VectorErrorKind::FileNotFound => write!(f, "vector file not found"),
            VectorErrorKind::CompressionError => write!(f, "vector compression error"),
            VectorErrorKind::MetadataError => write!(f, "vector metadata error"),
            VectorErrorKind::AlignmentError => write!(f, "vector alignment error"),
        }
    }
}

impl fmt::Display for IndexErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IndexErrorKind::IndexNotFound => write!(f, "index not found"),
            IndexErrorKind::IndexCorrupted => write!(f, "index corrupted"),
            IndexErrorKind::IndexFull => write!(f, "index full"),
            IndexErrorKind::IndexBuildFailed => write!(f, "index build failed"),
            IndexErrorKind::InvalidIndexType => write!(f, "invalid index type"),
            IndexErrorKind::InvalidParameters => write!(f, "invalid index parameters"),
            IndexErrorKind::NodeNotFound => write!(f, "index node not found"),
            IndexErrorKind::LayerNotFound => write!(f, "index layer not found"),
            IndexErrorKind::ConnectionError => write!(f, "index connection error"),
        }
    }
}

impl fmt::Display for SearchErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SearchErrorKind::InvalidQuery => write!(f, "invalid search query"),
            SearchErrorKind::InvalidK(k) => write!(f, "invalid K value: {}", k),
            SearchErrorKind::InvalidEf(ef) => write!(f, "invalid ef value: {}", ef),
            SearchErrorKind::NoResults => write!(f, "no search results"),
            SearchErrorKind::SearchTimeout => write!(f, "search timed out"),
            SearchErrorKind::SearchInterrupted => write!(f, "search interrupted"),
            SearchErrorKind::InvalidFilter => write!(f, "invalid search filter"),
            SearchErrorKind::InvalidMetric => write!(f, "invalid similarity metric"),
        }
    }
}

impl fmt::Display for JournalErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            JournalErrorKind::JournalNotFound => write!(f, "journal not found"),
            JournalErrorKind::JournalCorrupted => write!(f, "journal corrupted"),
            JournalErrorKind::JournalFull => write!(f, "journal full"),
            JournalErrorKind::InvalidRecord => write!(f, "invalid journal record"),
            JournalErrorKind::RecordNotFound => write!(f, "journal record not found"),
            JournalErrorKind::WriteAheadLogError => write!(f, "write-ahead log error"),
            JournalErrorKind::CheckpointError => write!(f, "checkpoint error"),
            JournalErrorKind::RecoveryError => write!(f, "recovery error"),
        }
    }
}

impl fmt::Display for TransactionErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TransactionErrorKind::TransactionNotFound => write!(f, "transaction not found"),
            TransactionErrorKind::TransactionAborted => write!(f, "transaction aborted"),
            TransactionErrorKind::TransactionConflict => write!(f, "transaction conflict"),
            TransactionErrorKind::DeadlockDetected => write!(f, "deadlock detected"),
            TransactionErrorKind::InvalidTransactionState => write!(f, "invalid transaction state"),
            TransactionErrorKind::TooManyTransactions => write!(f, "too many transactions"),
            TransactionErrorKind::TransactionTimeout => write!(f, "transaction timeout"),
            TransactionErrorKind::CommitFailed => write!(f, "commit failed"),
            TransactionErrorKind::RollbackFailed => write!(f, "rollback failed"),
        }
    }
}

// =======================
// Error Conversion Traits
// =======================

impl VexfsError {
    /// Add context to an error
    pub fn with_context(self, context: &str) -> Self {
        match self {
            VexfsError::Other(msg) => VexfsError::Other(format!("{}: {}", context, msg)),
            _ => VexfsError::Other(format!("{}: {}", context, self)),
        }
    }

    /// Convert to kernel error code
    pub fn to_kernel_errno(&self) -> i32 {
        match self {
            VexfsError::IoError(_) => -5,          // EIO
            VexfsError::StorageError(_) => -5,     // EIO
            VexfsError::FileNotFound => -2,       // ENOENT
            VexfsError::InodeNotFound(_) => -2,   // ENOENT
            VexfsError::PermissionDenied(_) => -13,  // EACCES
            VexfsError::OutOfMemory => -12,       // ENOMEM
            VexfsError::OutOfSpace => -28,        // ENOSPC
            VexfsError::StackOverflow => -12,     // ENOMEM (closest equivalent)
            VexfsError::InvalidArgument(_) => -22, // EINVAL
            VexfsError::InvalidParameter(_) => -22, // EINVAL
            VexfsError::FileTooLarge => -27,      // EFBIG
            VexfsError::ReadOnlyFilesystem => -30, // EROFS
            VexfsError::DirectoryNotEmpty => -39, // ENOTEMPTY
            VexfsError::NotADirectory(_) => -20,     // ENOTDIR
            VexfsError::IsADirectory => -21,      // EISDIR
            VexfsError::TooManyOpenFiles => -24,  // EMFILE
            VexfsError::ResourceBusy => -16,      // EBUSY
            VexfsError::Busy => -16,              // EBUSY
            VexfsError::InvalidOperation(_) => -95,  // EOPNOTSUPP
            VexfsError::UnsupportedVersion => -95, // EOPNOTSUPP
            // New error variants
            VexfsError::InvalidData(_) => -22,    // EINVAL
            VexfsError::InvalidPath(_) => -22,    // EINVAL
            VexfsError::PathTooLong => -36,       // ENAMETOOLONG
            VexfsError::NameTooLong => -36,       // ENAMETOOLONG
            VexfsError::NotFound => -2,           // ENOENT
            VexfsError::NotMounted => -19,        // ENODEV
            VexfsError::NoSpace => -28,           // ENOSPC
            VexfsError::EntryNotFound(_) => -2,      // ENOENT
            VexfsError::DeviceFull => -28,        // ENOSPC
            VexfsError::CacheError => -5,         // EIO
            VexfsError::CacheLocked => -16,       // EBUSY
            VexfsError::CacheDirty => -16,        // EBUSY
            VexfsError::AllocationError => -12,   // ENOMEM
            VexfsError::CorruptionError => -5,    // EIO
            VexfsError::ChecksumError => -5,      // EIO
            VexfsError::VersionMismatch => -95,   // EOPNOTSUPP
            VexfsError::UnsupportedOperation => -95, // EOPNOTSUPP
            VexfsError::NeedsFsck => -5,          // EIO
            VexfsError::IOError => -5,            // EIO
            VexfsError::InsufficientPermissions => -13, // EACCES
            VexfsError::Internal(_) => -22,       // EINVAL
            VexfsError::Timeout(_) => -110,       // ETIMEDOUT
            VexfsError::ResourceLimit(_) => -12,  // ENOMEM
            VexfsError::OperationCancelled(_) => -125, // ECANCELED
            VexfsError::InitializationFailed => -22, // EINVAL
            _ => -22,                             // EINVAL (generic)
        }
    }

    /// Check if error is recoverable
    pub fn is_recoverable(&self) -> bool {
        match self {
            VexfsError::IoError(IoErrorKind::TimeoutError) => true,
            VexfsError::IoError(IoErrorKind::InterruptedError) => true,
            VexfsError::ResourceBusy => true,
            VexfsError::Busy => true,
            VexfsError::OutOfMemory => true,
            VexfsError::TransactionError(TransactionErrorKind::TransactionConflict) => true,
            VexfsError::TransactionError(TransactionErrorKind::DeadlockDetected) => true,
            _ => false,
        }
    }

    /// Check if error indicates corruption
    pub fn is_corruption(&self) -> bool {
        match self {
            VexfsError::CorruptedData => true,
            VexfsError::ChecksumMismatch => true,
            VexfsError::InvalidSuperblock => true,
            VexfsError::StorageError(StorageErrorKind::BlockCorrupted(_)) => true,
            VexfsError::StorageError(StorageErrorKind::MetadataCorrupted) => true,
            VexfsError::IndexError(IndexErrorKind::IndexCorrupted) => true,
            VexfsError::JournalError(JournalErrorKind::JournalCorrupted) => true,
            _ => false,
        }
    }
}

// Convert from specific error kinds
impl From<IoErrorKind> for VexfsError {
    fn from(kind: IoErrorKind) -> Self {
        VexfsError::IoError(kind)
    }
}

impl From<StorageErrorKind> for VexfsError {
    fn from(kind: StorageErrorKind) -> Self {
        VexfsError::StorageError(kind)
    }
}

impl From<VectorErrorKind> for VexfsError {
    fn from(kind: VectorErrorKind) -> Self {
        VexfsError::VectorError(kind)
    }
}

impl From<IndexErrorKind> for VexfsError {
    fn from(kind: IndexErrorKind) -> Self {
        VexfsError::IndexError(kind)
    }
}

impl From<SearchErrorKind> for VexfsError {
    fn from(kind: SearchErrorKind) -> Self {
        VexfsError::SearchError(kind)
    }
}

impl From<JournalErrorKind> for VexfsError {
    fn from(kind: JournalErrorKind) -> Self {
        VexfsError::JournalError(kind)
    }
}

impl From<TransactionErrorKind> for VexfsError {
    fn from(kind: TransactionErrorKind) -> Self {
        VexfsError::TransactionError(kind)
    }
}

// Convert from standard library errors (userspace only)
#[cfg(not(feature = "kernel"))]
impl From<std::io::Error> for VexfsError {
    fn from(error: std::io::Error) -> Self {
        match error.kind() {
            std::io::ErrorKind::NotFound => VexfsError::FileNotFound,
            std::io::ErrorKind::PermissionDenied => VexfsError::PermissionDenied("permission denied".to_string()),
            std::io::ErrorKind::OutOfMemory => VexfsError::OutOfMemory,
            std::io::ErrorKind::WriteZero => VexfsError::IoError(IoErrorKind::WriteError),
            std::io::ErrorKind::Interrupted => VexfsError::IoError(IoErrorKind::InterruptedError),
            std::io::ErrorKind::InvalidInput => VexfsError::InvalidArgument("invalid input".to_string()),
            std::io::ErrorKind::TimedOut => VexfsError::IoError(IoErrorKind::TimeoutError),
            _ => VexfsError::IoError(IoErrorKind::DeviceError),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let error = VexfsError::InodeNotFound(123);
        assert_eq!(format!("{}", error), "Inode 123 not found");
        
        let io_error = VexfsError::IoError(IoErrorKind::ReadError);
        assert_eq!(format!("{}", io_error), "I/O error: read operation failed");
    }

    #[test]
    fn test_error_conversion() {
        let io_kind = IoErrorKind::WriteError;
        let error: VexfsError = io_kind.into();
        assert!(matches!(error, VexfsError::IoError(IoErrorKind::WriteError)));
    }

    #[test]
    fn test_kernel_errno_conversion() {
        assert_eq!(VexfsError::FileNotFound.to_kernel_errno(), -2);
        assert_eq!(VexfsError::PermissionDenied("test".to_string()).to_kernel_errno(), -13);
        assert_eq!(VexfsError::OutOfMemory.to_kernel_errno(), -12);
    }

    #[test]
    fn test_error_classification() {
        assert!(VexfsError::IoError(IoErrorKind::TimeoutError).is_recoverable());
        assert!(!VexfsError::FileNotFound.is_recoverable());
        
        assert!(VexfsError::CorruptedData.is_corruption());
        assert!(!VexfsError::FileNotFound.is_corruption());
    }

    #[test]
    fn test_error_with_context() {
        let error = VexfsError::FileNotFound.with_context("reading config");
        assert_eq!(format!("{}", error), "reading config: File not found");
    }

    #[test]
    fn test_vector_error_kinds() {
        let error = VectorErrorKind::DimensionMismatch { expected: 128, found: 256 };
        assert_eq!(format!("{}", error), "dimension mismatch: expected 128, found 256");
    }

    #[test]
    fn test_storage_error_kinds() {
        let error = StorageErrorKind::BlockNotFound(42);
        assert_eq!(format!("{}", error), "block 42 not found");
    }
}