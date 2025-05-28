/*
 * VexFS Control Tool Error Handling
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

//! Error handling for vexctl

use thiserror::Error;

/// Result type for vexctl operations
pub type Result<T> = std::result::Result<T, VexctlError>;

/// Comprehensive error types for vexctl operations
#[derive(Error, Debug)]
pub enum VexctlError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("IOCTL operation failed: {0}")]
    Ioctl(#[from] nix::Error),

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("VexFS filesystem not found at path: {path}")]
    FilesystemNotFound { path: String },

    #[error("Invalid VexFS filesystem at path: {path}")]
    InvalidFilesystem { path: String },

    #[error("Permission denied: {operation}")]
    PermissionDenied { operation: String },

    #[error("Invalid vector dimensions: expected {expected}, got {actual}")]
    InvalidDimensions { expected: u32, actual: u32 },

    #[error("Vector not found: ID {id}")]
    VectorNotFound { id: u64 },

    #[error("Index not found: {name}")]
    IndexNotFound { name: String },

    #[error("Invalid vector data: {reason}")]
    InvalidVectorData { reason: String },

    #[error("Search operation failed: {reason}")]
    SearchFailed { reason: String },

    #[error("Index operation failed: {operation} - {reason}")]
    IndexOperationFailed { operation: String, reason: String },

    #[error("IPC communication error: {reason}")]
    IpcError { reason: String },

    #[error("Embedding service error: {service} - {reason}")]
    EmbeddingServiceError { service: String, reason: String },

    #[error("Filesystem check failed: {reason}")]
    FsckFailed { reason: String },

    #[error("Configuration error: {reason}")]
    ConfigError { reason: String },

    #[error("Timeout error: operation timed out after {seconds} seconds")]
    Timeout { seconds: u64 },

    #[error("Invalid argument: {argument} - {reason}")]
    InvalidArgument { argument: String, reason: String },

    #[error("Operation not supported: {operation}")]
    NotSupported { operation: String },

    #[error("Internal error: {reason}")]
    Internal { reason: String },

    #[error("Parse error: {input} - {reason}")]
    ParseError { input: String, reason: String },

    #[error("Network error: {reason}")]
    Network { reason: String },

    #[error("Authentication failed: {reason}")]
    AuthenticationFailed { reason: String },

    #[error("Resource exhausted: {resource}")]
    ResourceExhausted { resource: String },

    #[error("Concurrent access conflict: {resource}")]
    ConcurrentAccess { resource: String },

    #[error("Data corruption detected: {location}")]
    DataCorruption { location: String },

    #[error("Version mismatch: expected {expected}, got {actual}")]
    VersionMismatch { expected: String, actual: String },
}

impl VexctlError {
    /// Create a new I/O error
    pub fn io(msg: &str) -> Self {
        VexctlError::Io(std::io::Error::new(std::io::ErrorKind::Other, msg))
    }

    /// Create a new internal error
    pub fn internal(reason: &str) -> Self {
        VexctlError::Internal {
            reason: reason.to_string(),
        }
    }

    /// Create a new invalid argument error
    pub fn invalid_argument(argument: &str, reason: &str) -> Self {
        VexctlError::InvalidArgument {
            argument: argument.to_string(),
            reason: reason.to_string(),
        }
    }

    /// Create a new search failed error
    pub fn search_failed(reason: &str) -> Self {
        VexctlError::SearchFailed {
            reason: reason.to_string(),
        }
    }

    /// Create a new index operation failed error
    pub fn index_operation_failed(operation: &str, reason: &str) -> Self {
        VexctlError::IndexOperationFailed {
            operation: operation.to_string(),
            reason: reason.to_string(),
        }
    }

    /// Create a new IPC error
    pub fn ipc_error(reason: &str) -> Self {
        VexctlError::IpcError {
            reason: reason.to_string(),
        }
    }

    /// Create a new embedding service error
    pub fn embedding_service_error(service: &str, reason: &str) -> Self {
        VexctlError::EmbeddingServiceError {
            service: service.to_string(),
            reason: reason.to_string(),
        }
    }

    /// Create a new filesystem check failed error
    pub fn fsck_failed(reason: &str) -> Self {
        VexctlError::FsckFailed {
            reason: reason.to_string(),
        }
    }

    /// Check if this error is recoverable
    pub fn is_recoverable(&self) -> bool {
        match self {
            VexctlError::Io(_) => true,
            VexctlError::Timeout { .. } => true,
            VexctlError::Network { .. } => true,
            VexctlError::ConcurrentAccess { .. } => true,
            VexctlError::ResourceExhausted { .. } => true,
            _ => false,
        }
    }

    /// Get the error category for logging and metrics
    pub fn category(&self) -> &'static str {
        match self {
            VexctlError::Io(_) => "io",
            VexctlError::Ioctl(_) => "ioctl",
            VexctlError::Json(_) => "serialization",
            VexctlError::FilesystemNotFound { .. } => "filesystem",
            VexctlError::InvalidFilesystem { .. } => "filesystem",
            VexctlError::PermissionDenied { .. } => "security",
            VexctlError::InvalidDimensions { .. } => "vector",
            VexctlError::VectorNotFound { .. } => "vector",
            VexctlError::IndexNotFound { .. } => "index",
            VexctlError::InvalidVectorData { .. } => "vector",
            VexctlError::SearchFailed { .. } => "search",
            VexctlError::IndexOperationFailed { .. } => "index",
            VexctlError::IpcError { .. } => "ipc",
            VexctlError::EmbeddingServiceError { .. } => "embedding",
            VexctlError::FsckFailed { .. } => "fsck",
            VexctlError::ConfigError { .. } => "config",
            VexctlError::Timeout { .. } => "timeout",
            VexctlError::InvalidArgument { .. } => "argument",
            VexctlError::NotSupported { .. } => "support",
            VexctlError::Internal { .. } => "internal",
            VexctlError::ParseError { .. } => "parse",
            VexctlError::Network { .. } => "network",
            VexctlError::AuthenticationFailed { .. } => "auth",
            VexctlError::ResourceExhausted { .. } => "resource",
            VexctlError::ConcurrentAccess { .. } => "concurrency",
            VexctlError::DataCorruption { .. } => "corruption",
            VexctlError::VersionMismatch { .. } => "version",
        }
    }

    /// Get suggested user action for this error
    pub fn suggestion(&self) -> Option<&'static str> {
        match self {
            VexctlError::FilesystemNotFound { .. } => {
                Some("Check that the path points to a mounted VexFS filesystem")
            }
            VexctlError::PermissionDenied { .. } => {
                Some("Try running with elevated privileges or check file permissions")
            }
            VexctlError::InvalidDimensions { .. } => {
                Some("Ensure vector dimensions match the index configuration")
            }
            VexctlError::Timeout { .. } => {
                Some("Try increasing the timeout or check system load")
            }
            VexctlError::ResourceExhausted { .. } => {
                Some("Free up system resources or increase limits")
            }
            VexctlError::DataCorruption { .. } => {
                Some("Run 'vexctl fsck' to check and repair filesystem integrity")
            }
            _ => None,
        }
    }
}

/// Convert IOCTL error codes to VexctlError
impl From<i32> for VexctlError {
    fn from(code: i32) -> Self {
        match code {
            1 => VexctlError::InvalidArgument {
                argument: "request".to_string(),
                reason: "Invalid IOCTL request".to_string(),
            },
            2 => VexctlError::InvalidDimensions {
                expected: 0,
                actual: 0,
            },
            3 => VexctlError::VectorNotFound { id: 0 },
            4 => VexctlError::VectorNotFound { id: 0 },
            5 => VexctlError::IndexNotFound {
                name: "unknown".to_string(),
            },
            6 => VexctlError::PermissionDenied {
                operation: "IOCTL operation".to_string(),
            },
            7 => VexctlError::ResourceExhausted {
                resource: "memory".to_string(),
            },
            8 => VexctlError::InvalidArgument {
                argument: "buffer".to_string(),
                reason: "Invalid buffer".to_string(),
            },
            9 => VexctlError::InvalidArgument {
                argument: "buffer".to_string(),
                reason: "Buffer too small".to_string(),
            },
            10 => VexctlError::InvalidArgument {
                argument: "parameters".to_string(),
                reason: "Invalid parameters".to_string(),
            },
            11 => VexctlError::DataCorruption {
                location: "index".to_string(),
            },
            12 => VexctlError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                "I/O error",
            )),
            13 => VexctlError::Timeout { seconds: 30 },
            14 => VexctlError::ConcurrentAccess {
                resource: "vector index".to_string(),
            },
            15 => VexctlError::InvalidVectorData {
                reason: "Invalid format".to_string(),
            },
            16 => VexctlError::InvalidVectorData {
                reason: "Invalid vector data".to_string(),
            },
            17 => VexctlError::InvalidArgument {
                argument: "parameter".to_string(),
                reason: "Invalid parameter".to_string(),
            },
            _ => VexctlError::Internal {
                reason: format!("Unknown IOCTL error code: {}", code),
            },
        }
    }
}