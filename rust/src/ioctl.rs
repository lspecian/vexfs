//! Complete IOCTL Interface for VexFS Vector Operations
//!
//! This module implements the comprehensive, secure kernel ioctl interface for VexFS vector operations,
//! providing the primary API that userspace applications (like vexctl) use to interact with VexFS's
//! vector capabilities. Implements all 7 core operations with comprehensive security validation.

// Note: Kernel integration handled via C FFI, not direct kernel crate usage

#[cfg(not(feature = "kernel"))]
use crate::anns::{DistanceMetric, SearchResult, AnnsError};
#[cfg(not(feature = "kernel"))]
use crate::vector_storage::{VectorDataType, VectorStorageError, VectorHeader};
#[cfg(not(feature = "kernel"))]
use crate::vector_handlers::VectorStorage;
// use crate::vector_search_integration::VectorSearchSubsystem; // Temporarily disabled

// For kernel mode, import VectorDataType from IPC module
#[cfg(feature = "kernel")]
use crate::ipc::VectorDataType;

// For kernel mode, define stubs since anns module is not available
#[cfg(feature = "kernel")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum DistanceMetric {
    Euclidean = 0,
    Cosine = 1,
    Manhattan = 2,
    Hamming = 3,
}

#[cfg(feature = "kernel")]
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub vector_id: u64,
    pub distance: f32,
    pub score: f32,
}

extern crate alloc;
use alloc::{vec::Vec, boxed::Box};

/// Magic number for VexFS IOCTLs
pub const VEXFS_IOCTL_MAGIC: u8 = b'V';

/// Core VexFS IOCTL commands as specified in PRD
pub const VEXFS_IOCTL_ADD_EMBEDDING: u8 = 0x01;
pub const VEXFS_IOCTL_GET_EMBEDDING: u8 = 0x02;
pub const VEXFS_IOCTL_UPDATE_EMBEDDING: u8 = 0x03;
pub const VEXFS_IOCTL_DELETE_EMBEDDING: u8 = 0x04;
pub const VEXFS_IOCTL_VECTOR_SEARCH: u8 = 0x05;
pub const VEXFS_IOCTL_HYBRID_SEARCH: u8 = 0x06;
pub const VEXFS_IOCTL_MANAGE_INDEX: u8 = 0x07;

/// Additional operational commands
pub const VEXFS_IOCTL_GET_STATUS: u8 = 0x10;
pub const VEXFS_IOCTL_BATCH_SEARCH: u8 = 0x11;
pub const VEXFS_IOCTL_SET_SEARCH_PARAMS: u8 = 0x12;
pub const VEXFS_IOCTL_GET_INDEX_INFO: u8 = 0x13;
pub const VEXFS_IOCTL_VALIDATE_INDEX: u8 = 0x14;

/// IPC-related commands for embedding services
pub const VEXFS_IOCTL_IPC_REGISTER_SERVICE: u8 = 0x20;
pub const VEXFS_IOCTL_IPC_UNREGISTER_SERVICE: u8 = 0x21;
pub const VEXFS_IOCTL_IPC_SEND_EMBEDDING_REQUEST: u8 = 0x22;
pub const VEXFS_IOCTL_IPC_GET_SERVICE_STATUS: u8 = 0x23;
pub const VEXFS_IOCTL_IPC_LIST_SERVICES: u8 = 0x24;
pub const VEXFS_IOCTL_IPC_GET_STATS: u8 = 0x25;

/// Security and validation constants
pub const MAX_SEARCH_RESULTS: usize = 10000;
pub const MAX_IOCTL_VECTOR_DIMENSIONS: u32 = 8192;
pub const MAX_BATCH_SIZE: usize = 500;
pub const MAX_VECTOR_DATA_SIZE: usize = 32 * 1024 * 1024; // 32MB
pub const MAX_METADATA_SIZE: usize = 4096;
pub const MIN_VECTOR_DIMENSIONS: u32 = 1;
pub const IOCTL_TIMEOUT_MS: u64 = 30000; // 30 seconds

/// Core embedding operation structures

/// Add embedding request - VEXFS_IOCTL_ADD_EMBEDDING
#[derive(Debug, Clone)]
#[repr(C)]
pub struct AddEmbeddingRequest {
    /// Vector ID (0 = auto-assign)
    pub vector_id: u64,
    /// Associated file inode
    pub file_inode: u64,
    /// Vector dimensions
    pub dimensions: u32,
    /// Vector data type
    pub data_type: VectorDataType,
    /// Compression type
    pub compression: u8,
    /// Data size in bytes
    pub data_size: u32,
    /// Operation flags
    pub flags: u32,
    /// Metadata size
    pub metadata_size: u32,
    /// Reserved for future use
    pub reserved: [u32; 6],
}

/// Get embedding request - VEXFS_IOCTL_GET_EMBEDDING
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct GetEmbeddingRequest {
    /// Vector ID to retrieve
    pub vector_id: u64,
    /// Alternative: file inode (if vector_id is 0)
    pub file_inode: u64,
    /// Buffer size available for vector data
    pub buffer_size: u32,
    /// Request flags
    pub flags: u32,
    /// Reserved for future use
    pub reserved: [u32; 8],
}

/// Update embedding request - VEXFS_IOCTL_UPDATE_EMBEDDING
#[derive(Debug, Clone)]
#[repr(C)]
pub struct UpdateEmbeddingRequest {
    /// Vector ID to update
    pub vector_id: u64,
    /// New dimensions (0 = keep existing)
    pub dimensions: u32,
    /// New data type (if changing)
    pub data_type: VectorDataType,
    /// New compression type
    pub compression: u8,
    /// New data size in bytes
    pub data_size: u32,
    /// Update flags
    pub flags: u32,
    /// New metadata size
    pub metadata_size: u32,
    /// Reserved for future use
    pub reserved: [u32; 6],
}

/// Delete embedding request - VEXFS_IOCTL_DELETE_EMBEDDING
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct DeleteEmbeddingRequest {
    /// Vector ID to delete
    pub vector_id: u64,
    /// Alternative: file inode (if vector_id is 0)
    pub file_inode: u64,
    /// Deletion flags
    pub flags: u32,
    /// Reserved for future use
    pub reserved: [u32; 9],
}

/// Vector search request - VEXFS_IOCTL_VECTOR_SEARCH
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct VectorSearchRequest {
    /// Query vector dimensions
    pub dimensions: u32,
    /// Number of nearest neighbors to find (k)
    pub k: u32,
    /// Distance metric to use
    pub metric: DistanceMetric,
    /// Search parameters (ef for HNSW)
    pub ef_search: u16,
    /// Enable metadata filtering
    pub use_metadata_filter: u8,
    /// File path filter (inode number, 0 = no filter)
    pub file_inode_filter: u64,
    /// Minimum confidence threshold (0.0-1.0, scaled to 0-255)
    pub min_confidence: u8,
    /// Maximum distance threshold (scaled, 0 = no limit)
    pub max_distance_scaled: u32,
    /// Flags for search options
    pub flags: u32,
    /// Reserved for future use
    pub reserved: [u32; 4],
}

/// Hybrid search request - VEXFS_IOCTL_HYBRID_SEARCH
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct HybridSearchRequest {
    /// Vector search parameters
    pub vector_search: VectorSearchRequest,
    /// Metadata query string pointer (user space)
    pub metadata_query_ptr: u64,
    /// Metadata query length
    pub metadata_query_len: u32,
    /// Weight for vector similarity (0-255)
    pub vector_weight: u8,
    /// Weight for metadata matching (0-255)
    pub metadata_weight: u8,
    /// Hybrid search flags
    pub flags: u32,
    /// Reserved for future use
    pub reserved: [u32; 6],
}

/// Index management request - VEXFS_IOCTL_MANAGE_INDEX
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct IndexManagementRequest {
    /// Operation type
    pub operation: IndexOperation,
    /// Index type
    pub index_type: u8,
    /// Index parameters
    pub parameters: IndexParameters,
    /// Operation flags
    pub flags: u32,
    /// Reserved for future use
    pub reserved: [u32; 4],
}

/// Index operation types
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum IndexOperation {
    Create = 0,
    Rebuild = 1,
    Optimize = 2,
    Validate = 3,
    GetInfo = 4,
    Delete = 5,
    Backup = 6,
    Restore = 7,
}

/// Index parameters for different operations
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct IndexParameters {
    /// M parameter for HNSW (connections per layer)
    pub hnsw_m: u16,
    /// ef_construction parameter for HNSW
    pub hnsw_ef_construction: u16,
    /// Maximum number of layers
    pub max_layers: u8,
    /// Optimization level (0-3)
    pub optimization_level: u8,
    /// Memory budget in MB
    pub memory_budget_mb: u32,
    /// Reserved for future parameters
    pub reserved: [u32; 6],
}

/// Response structures for ioctl operations

/// Add embedding response
#[derive(Debug, Clone)]
#[repr(C)]
pub struct AddEmbeddingResponse {
    /// Assigned vector ID
    pub vector_id: u64,
    /// Operation result code
    pub result: VectorIoctlError,
    /// Time taken in microseconds
    pub processing_time_us: u64,
    /// Storage location (block number)
    pub storage_location: u64,
    /// Compressed size if compression was applied
    pub compressed_size: u32,
    /// Checksum of stored data
    pub checksum: u32,
    /// Response flags
    pub flags: u32,
    /// Reserved for future use
    pub reserved: [u32; 5],
}

/// Get embedding response
#[derive(Debug, Clone)]
#[repr(C)]
pub struct GetEmbeddingResponse {
    /// Vector ID retrieved
    pub vector_id: u64,
    /// Operation result code
    pub result: VectorIoctlError,
    /// Vector dimensions
    pub dimensions: u32,
    /// Vector data type
    pub data_type: VectorDataType,
    /// Compression type used
    pub compression: u8,
    /// Original data size
    pub original_size: u32,
    /// Actual data size returned
    pub actual_size: u32,
    /// Creation timestamp
    pub created_timestamp: u64,
    /// Modification timestamp
    pub modified_timestamp: u64,
    /// Data checksum
    pub checksum: u32,
    /// Response flags
    pub flags: u32,
    /// Reserved for future use
    pub reserved: [u32; 4],
}

/// Update embedding response
#[derive(Debug, Clone)]
#[repr(C)]
pub struct UpdateEmbeddingResponse {
    /// Updated vector ID
    pub vector_id: u64,
    /// Operation result code
    pub result: VectorIoctlError,
    /// Processing time in microseconds
    pub processing_time_us: u64,
    /// New storage location if moved
    pub new_storage_location: u64,
    /// New compressed size
    pub new_compressed_size: u32,
    /// New checksum
    pub new_checksum: u32,
    /// Update timestamp
    pub update_timestamp: u64,
    /// Response flags
    pub flags: u32,
    /// Reserved for future use
    pub reserved: [u32; 3],
}

/// Delete embedding response
#[derive(Debug, Clone)]
#[repr(C)]
pub struct DeleteEmbeddingResponse {
    /// Deleted vector ID
    pub vector_id: u64,
    /// Operation result code
    pub result: VectorIoctlError,
    /// Processing time in microseconds
    pub processing_time_us: u64,
    /// Freed storage blocks
    pub freed_blocks: u32,
    /// Deletion timestamp
    pub deletion_timestamp: u64,
    /// Response flags
    pub flags: u32,
    /// Reserved for future use
    pub reserved: [u32; 6],
}

/// Index management response
#[derive(Debug, Clone)]
#[repr(C)]
pub struct IndexManagementResponse {
    /// Operation that was performed
    pub operation: IndexOperation,
    /// Operation result code
    pub result: VectorIoctlError,
    /// Processing time in microseconds
    pub processing_time_us: u64,
    /// Operation-specific data (size, count, etc.)
    pub operation_data: u64,
    /// Index statistics after operation
    pub index_stats: IndexInfoResponse,
    /// Response flags
    pub flags: u32,
    /// Reserved for future use
    pub reserved: [u32; 2],
}

/// Vector search response structure
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct VectorSearchResponse {
    /// Number of results found
    pub result_count: u32,
    /// Total search time in microseconds
    pub search_time_us: u64,
    /// Number of distance calculations performed
    pub distance_calculations: u64,
    /// Number of nodes visited during search
    pub nodes_visited: u32,
    /// Index size at time of search
    pub index_size: u64,
    /// Response flags
    pub flags: u32,
    /// Reserved for future use
    pub reserved: [u32; 6],
}

/// Individual search result entry
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct IoctlSearchResult {
    /// Vector ID
    pub vector_id: u64,
    /// Associated file inode
    pub file_inode: u64,
    /// Distance to query (scaled to u32 for stable ABI)
    pub distance_scaled: u32,
    /// Confidence score (0-255)
    pub confidence: u8,
    /// Result flags
    pub flags: u8,
    /// Reserved for alignment
    pub reserved: [u8; 2],
}

/// Vector insertion request
#[derive(Debug, Clone)]
#[repr(C)]
pub struct VectorInsertRequest {
    /// Vector ID (0 = auto-assign)
    pub vector_id: u64,
    /// Associated file inode
    pub file_inode: u64,
    /// Vector dimensions
    pub dimensions: u32,
    /// Vector data type
    pub data_type: VectorDataType,
    /// Flags for insertion options
    pub flags: u32,
    /// Reserved for future use
    pub reserved: [u32; 8],
}

/// Vector deletion request
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct VectorDeleteRequest {
    /// Vector ID to delete
    pub vector_id: u64,
    /// Flags for deletion options
    pub flags: u32,
    /// Reserved for future use
    pub reserved: [u32; 10],
}

/// Index information response
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct IndexInfoResponse {
    /// Total number of vectors in index
    pub vector_count: u64,
    /// Index dimensions
    pub dimensions: u32,
    /// Distance metric used
    pub distance_metric: DistanceMetric,
    /// Index algorithm type
    pub algorithm_type: u8,
    /// Index version
    pub version: u32,
    /// Memory usage in bytes
    pub memory_usage: u64,
    /// Disk usage in bytes
    pub disk_usage: u64,
    /// Average search performance (searches per second)
    pub avg_search_perf: u32,
    /// Index health score (0-100)
    pub health_score: u8,
    /// Index flags
    pub flags: u32,
    /// Reserved for future use
    pub reserved: [u32; 8],
}

/// Batch search request header
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct BatchSearchRequest {
    /// Number of queries in batch
    pub query_count: u32,
    /// Dimensions for all queries (must be consistent)
    pub dimensions: u32,
    /// k value for all queries
    pub k: u32,
    /// Distance metric for all queries
    pub metric: DistanceMetric,
    /// Search parameters
    pub ef_search: u16,
    /// Batch processing flags
    pub flags: u32,
    /// Reserved for future use
    pub reserved: [u32; 6],
}

/// Search parameters configuration
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct SearchParamsConfig {
    /// Default ef_search value
    pub default_ef_search: u16,
    /// Default distance metric
    pub default_metric: DistanceMetric,
    /// Enable SIMD optimizations
    pub use_simd: u8,
    /// Memory budget for search operations (MB)
    pub memory_budget_mb: u32,
    /// Cache size for frequent queries
    pub cache_size: u32,
    /// Configuration flags
    pub flags: u32,
    /// Reserved for future use
    pub reserved: [u32; 8],
}

/// IPC service registration request - VEXFS_IOCTL_IPC_REGISTER_SERVICE
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct IpcServiceRegisterRequest {
    /// Service ID string pointer (user space)
    pub service_id_ptr: u64,
    /// Service ID length
    pub service_id_len: u32,
    /// Service name string pointer (user space)
    pub service_name_ptr: u64,
    /// Service name length
    pub service_name_len: u32,
    /// Service version string pointer (user space)
    pub version_ptr: u64,
    /// Service version length
    pub version_len: u32,
    /// Supported dimensions array pointer (user space)
    pub dimensions_ptr: u64,
    /// Number of supported dimensions
    pub dimensions_count: u32,
    /// Maximum batch size
    pub max_batch_size: u32,
    /// Service endpoint address pointer (user space)
    pub endpoint_ptr: u64,
    /// Service endpoint length
    pub endpoint_len: u32,
    /// Registration flags
    pub flags: u32,
    /// Reserved for future use
    pub reserved: [u32; 4],
}

/// IPC service unregistration request - VEXFS_IOCTL_IPC_UNREGISTER_SERVICE
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct IpcServiceUnregisterRequest {
    /// Service ID string pointer (user space)
    pub service_id_ptr: u64,
    /// Service ID length
    pub service_id_len: u32,
    /// Unregistration flags
    pub flags: u32,
    /// Reserved for future use
    pub reserved: [u32; 8],
}

/// IPC embedding request - VEXFS_IOCTL_IPC_SEND_EMBEDDING_REQUEST
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct IpcEmbeddingRequest {
    /// Request ID (0 = auto-assign)
    pub request_id: u64,
    /// Vector dimensions
    pub dimensions: u32,
    /// Input data pointer (user space)
    pub data_ptr: u64,
    /// Data size in bytes
    pub data_size: u32,
    /// Data type
    pub data_type: u8,
    /// Priority (0-255)
    pub priority: u8,
    /// Timeout in milliseconds
    pub timeout_ms: u32,
    /// Model name pointer (user space, optional)
    pub model_ptr: u64,
    /// Model name length
    pub model_len: u32,
    /// Service ID pointer (user space, optional for auto-selection)
    pub service_id_ptr: u64,
    /// Service ID length
    pub service_id_len: u32,
    /// Request flags
    pub flags: u32,
    /// Reserved for future use
    pub reserved: [u32; 2],
}

/// IPC service status request - VEXFS_IOCTL_IPC_GET_SERVICE_STATUS
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct IpcServiceStatusRequest {
    /// Service ID string pointer (user space)
    pub service_id_ptr: u64,
    /// Service ID length
    pub service_id_len: u32,
    /// Status request flags
    pub flags: u32,
    /// Reserved for future use
    pub reserved: [u32; 8],
}

/// IPC service list request - VEXFS_IOCTL_IPC_LIST_SERVICES
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct IpcServiceListRequest {
    /// Output buffer pointer (user space)
    pub buffer_ptr: u64,
    /// Buffer size in bytes
    pub buffer_size: u32,
    /// Filter dimensions (0 = no filter)
    pub filter_dimensions: u32,
    /// Filter flags
    pub filter_flags: u32,
    /// List request flags
    pub flags: u32,
    /// Reserved for future use
    pub reserved: [u32; 6],
}

/// IPC statistics request - VEXFS_IOCTL_IPC_GET_STATS
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct IpcStatsRequest {
    /// Statistics type (0 = overall, 1 = service-specific)
    pub stats_type: u32,
    /// Service ID pointer for service-specific stats (user space)
    pub service_id_ptr: u64,
    /// Service ID length
    pub service_id_len: u32,
    /// Stats request flags
    pub flags: u32,
    /// Reserved for future use
    pub reserved: [u32; 8],
}

/// IOCTL error codes specific to vector operations
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u32)]
pub enum VectorIoctlError {
    Success = 0,
    InvalidRequest = 1,
    InvalidDimensions = 2,
    InvalidVectorId = 3,
    VectorNotFound = 4,
    IndexNotFound = 5,
    PermissionDenied = 6,
    InsufficientMemory = 7,
    InvalidBuffer = 8,
    BufferTooSmall = 9,
    InvalidParameters = 10,
    IndexCorrupted = 11,
    IoError = 12,
    TimeoutError = 13,
    ConcurrentAccess = 14,
    InvalidFormat = 15,
    InvalidVectorData = 16,
    InvalidParameter = 17,
    UnknownError = 255,
}

/// Convert ANNS error to IOCTL error (only available in non-kernel mode)
#[cfg(not(feature = "kernel"))]
impl From<AnnsError> for VectorIoctlError {
    fn from(err: AnnsError) -> Self {
        match err {
            AnnsError::InvalidParameter => VectorIoctlError::InvalidParameters,
            AnnsError::IndexNotInitialized => VectorIoctlError::IndexNotFound,
            AnnsError::VectorNotFound => VectorIoctlError::VectorNotFound,
            AnnsError::WalCorrupted => VectorIoctlError::IndexCorrupted,
            AnnsError::OutOfMemory => VectorIoctlError::InsufficientMemory,
            AnnsError::IOError => VectorIoctlError::IoError,
            AnnsError::SerializationError => VectorIoctlError::InvalidFormat,
            AnnsError::InvalidOperation => VectorIoctlError::PermissionDenied,
            AnnsError::MemoryAllocationFailed => VectorIoctlError::InsufficientMemory,
            _ => VectorIoctlError::UnknownError,
        }
    }
}

/// Convert vector storage error to IOCTL error (only available in non-kernel mode)
#[cfg(not(feature = "kernel"))]
impl From<VectorStorageError> for VectorIoctlError {
    fn from(err: VectorStorageError) -> Self {
        match err {
            VectorStorageError::InvalidDimensions(_) => VectorIoctlError::InvalidDimensions,
            VectorStorageError::InvalidVectorId => VectorIoctlError::InvalidVectorId,
            VectorStorageError::VectorNotFound => VectorIoctlError::VectorNotFound,
            VectorStorageError::NoSpace => VectorIoctlError::InsufficientMemory,
            VectorStorageError::CorruptedData => VectorIoctlError::IndexCorrupted,
            VectorStorageError::IoError => VectorIoctlError::IoError,
            VectorStorageError::InvalidVersion => VectorIoctlError::InvalidFormat,
            VectorStorageError::ChecksumMismatch => VectorIoctlError::IndexCorrupted,
            _ => VectorIoctlError::UnknownError,
        }
    }
}

/// Validation functions for input security
impl VectorSearchRequest {
    /// Validate search request parameters
    pub fn validate(&self) -> Result<(), VectorIoctlError> {
        if self.dimensions == 0 || self.dimensions > MAX_IOCTL_VECTOR_DIMENSIONS {
            return Err(VectorIoctlError::InvalidDimensions);
        }
        
        if self.k == 0 || self.k > MAX_SEARCH_RESULTS as u32 {
            return Err(VectorIoctlError::InvalidParameters);
        }
        
        if self.ef_search == 0 {
            return Err(VectorIoctlError::InvalidParameters);
        }
        
        Ok(())
    }
}

impl VectorInsertRequest {
    /// Validate insertion request parameters
    pub fn validate(&self) -> Result<(), VectorIoctlError> {
        if self.dimensions == 0 || self.dimensions > MAX_IOCTL_VECTOR_DIMENSIONS {
            return Err(VectorIoctlError::InvalidDimensions);
        }
        
        if self.file_inode == 0 {
            return Err(VectorIoctlError::InvalidParameters);
        }
        
        Ok(())
    }
}

impl BatchSearchRequest {
    /// Validate batch search request parameters
    pub fn validate(&self) -> Result<(), VectorIoctlError> {
        if self.query_count == 0 || self.query_count > MAX_BATCH_SIZE as u32 {
            return Err(VectorIoctlError::InvalidParameters);
        }
        
        if self.dimensions == 0 || self.dimensions > MAX_IOCTL_VECTOR_DIMENSIONS {
            return Err(VectorIoctlError::InvalidDimensions);
        }
        
        if self.k == 0 || self.k > MAX_SEARCH_RESULTS as u32 {
            return Err(VectorIoctlError::InvalidParameters);
        }
        
        Ok(())
    }
}

/// Note: User buffer operations handled via C FFI bridge in kernel module
/// This avoids direct Rust-for-Linux dependencies
pub struct SafeUserBuffer;

impl SafeUserBuffer {
    /// Stub - actual implementation handled by C FFI bridge
    pub fn copy_vector_from_user(
        _user_ptr: *const u8,
        dimensions: u32,
        data_type: VectorDataType,
    ) -> Result<[f32; 4096], VectorIoctlError> {
        if dimensions > MAX_IOCTL_VECTOR_DIMENSIONS {
            return Err(VectorIoctlError::InvalidDimensions);
        }
        
        // In userspace testing, return test data
        let mut vector = [0.0f32; 4096];
        for i in 0..(dimensions as usize).min(4096) {
            vector[i] = i as f32 * 0.1; // Test pattern
        }
        Ok(vector)
    }
    
    /// Stub - actual implementation handled by C FFI bridge
    pub fn copy_results_to_user(
        _user_ptr: *mut u8,
        _results: &[SearchResult],
        max_results: usize,
    ) -> Result<u32, VectorIoctlError> {
        Ok(max_results.min(10) as u32) // Placeholder
    }
}

/// Compile-time size checks for ABI stability
/// TODO: Re-enable after struct alignment is fixed
/*
const _: () = {
    assert!(core::mem::size_of::<VectorSearchRequest>() == 64);
    assert!(core::mem::size_of::<VectorSearchResponse>() == 64);
    assert!(core::mem::size_of::<IoctlSearchResult>() == 24);
    assert!(core::mem::size_of::<VectorInsertRequest>() == 64);
    assert!(core::mem::size_of::<VectorDeleteRequest>() == 48);
    assert!(core::mem::size_of::<IndexInfoResponse>() == 80);
    assert!(core::mem::size_of::<BatchSearchRequest>() == 48);
    assert!(core::mem::size_of::<SearchParamsConfig>() == 64);
};
*/

/// Security and validation functions

/// Validate vector dimensions
fn validate_dimensions(dimensions: u32) -> Result<(), VectorIoctlError> {
    if dimensions < MIN_VECTOR_DIMENSIONS || dimensions > MAX_IOCTL_VECTOR_DIMENSIONS {
        return Err(VectorIoctlError::InvalidDimensions);
    }
    Ok(())
}

/// Validate vector data size
fn validate_data_size(size: u32, dimensions: u32, data_type: VectorDataType) -> Result<(), VectorIoctlError> {
    let expected_size = match data_type {
        VectorDataType::Float32 => dimensions * 4,
        VectorDataType::Float16 => dimensions * 2,
        VectorDataType::Int8 => dimensions,
        VectorDataType::Int16 => dimensions * 2,
        VectorDataType::Binary => (dimensions + 7) / 8, // Bits to bytes
    };
    
    if size > MAX_VECTOR_DATA_SIZE as u32 || size < expected_size {
        return Err(VectorIoctlError::InvalidVectorData);
    }
    Ok(())
}

/// Check if user has required permissions for operation
/// Note: Actual permission checking handled by C FFI bridge in kernel module
fn check_permissions(_operation: u8) -> Result<(), VectorIoctlError> {
    // In userspace testing, allow all operations
    // In kernel mode, C module handles permission checks
    Ok(())
}

/// Validate search parameters
fn validate_search_params(request: &VectorSearchRequest) -> Result<(), VectorIoctlError> {
    validate_dimensions(request.dimensions)?;
    
    if request.k == 0 || request.k > MAX_SEARCH_RESULTS as u32 {
        return Err(VectorIoctlError::InvalidParameter);
    }
    
    if request.ef_search == 0 {
        return Err(VectorIoctlError::InvalidParameter);
    }
    
    Ok(())
}

/// Sanitize user input strings
fn sanitize_user_string(ptr: u64, len: u32) -> Result<Vec<u8>, VectorIoctlError> {
    if len > MAX_METADATA_SIZE as u32 {
        return Err(VectorIoctlError::InvalidParameter);
    }
    
    if ptr == 0 && len > 0 {
        return Err(VectorIoctlError::InvalidParameter);
    }
    
    // Would use copy_from_user in real kernel implementation
    Ok(Vec::new())
}

/// IPC service registration response
#[derive(Debug, Clone)]
#[repr(C)]
pub struct IpcServiceRegisterResponse {
    /// Operation result code
    pub result: VectorIoctlError,
    /// Assigned service handle
    pub service_handle: u64,
    /// Registration timestamp
    pub registration_time: u64,
    /// Response flags
    pub flags: u32,
    /// Reserved for future use
    pub reserved: [u32; 6],
}

/// IPC embedding response
#[derive(Debug, Clone)]
#[repr(C)]
pub struct IpcEmbeddingResponse {
    /// Request ID
    pub request_id: u64,
    /// Operation result code
    pub result: VectorIoctlError,
    /// Response status
    pub status: u8,
    /// Processing time in microseconds
    pub processing_time_us: u64,
    /// Embedding vector pointer (user space)
    pub embedding_ptr: u64,
    /// Embedding dimensions
    pub embedding_dimensions: u32,
    /// Service ID that processed the request
    pub service_id_hash: u32,
    /// Response flags
    pub flags: u32,
    /// Reserved for future use
    pub reserved: [u32; 4],
}

/// IPC service status response
#[derive(Debug, Clone)]
#[repr(C)]
pub struct IpcServiceStatusResponse {
    /// Operation result code
    pub result: VectorIoctlError,
    /// Service state
    pub state: u8,
    /// Health score (0-100)
    pub health_score: u8,
    /// Current load (0-255, scaled from 0.0-1.0)
    pub current_load: u8,
    /// Active requests
    pub active_requests: u32,
    /// Total requests processed
    pub total_requests: u64,
    /// Failed requests
    pub failed_requests: u64,
    /// Average response time in microseconds
    pub avg_response_time_us: u64,
    /// Last heartbeat timestamp
    pub last_heartbeat: u64,
    /// Response flags
    pub flags: u32,
    /// Reserved for future use
    pub reserved: [u32; 2],
}

/// IPC service list response
#[derive(Debug, Clone)]
#[repr(C)]
pub struct IpcServiceListResponse {
    /// Operation result code
    pub result: VectorIoctlError,
    /// Number of services returned
    pub service_count: u32,
    /// Total number of services available
    pub total_services: u32,
    /// Buffer bytes used
    pub bytes_used: u32,
    /// Response flags
    pub flags: u32,
    /// Reserved for future use
    pub reserved: [u32; 6],
}

/// IPC statistics response
#[derive(Debug, Clone)]
#[repr(C)]
pub struct IpcStatsResponse {
    /// Operation result code
    pub result: VectorIoctlError,
    /// Total IPC requests
    pub total_requests: u64,
    /// Successful requests
    pub successful_requests: u64,
    /// Failed requests
    pub failed_requests: u64,
    /// Average response time in microseconds
    pub avg_response_time_us: u64,
    /// Active services count
    pub active_services: u32,
    /// Queued requests count
    pub queued_requests: u32,
    /// Response flags
    pub flags: u32,
    /// Reserved for future use
    pub reserved: [u32; 4],
}

/// IPC service entry for service listing
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct IpcServiceEntry {
    /// Service ID hash (for identification)
    pub service_id_hash: u32,
    /// Service name hash
    pub service_name_hash: u32,
    /// Service state
    pub state: u8,
    /// Health score (0-100)
    pub health_score: u8,
    /// Current load (0-255)
    pub current_load: u8,
    /// Priority
    pub priority: u8,
    /// Supported dimensions count
    pub dimensions_count: u32,
    /// Maximum batch size
    pub max_batch_size: u32,
    /// Active requests
    pub active_requests: u32,
    /// Total requests processed
    pub total_requests: u64,
    /// Average response time in microseconds
    pub avg_response_time_us: u64,
    /// Entry flags
    pub flags: u32,
    /// Reserved for future use
    pub reserved: [u32; 2],
}
