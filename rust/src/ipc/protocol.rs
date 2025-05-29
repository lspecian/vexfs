//! IPC Protocol Definitions for VexFS Embedding Services
//!
//! This module defines the message format, protocol versioning, and
//! serialization mechanisms for communication between VexFS kernel
//! module and userspace embedding services.

use crate::shared::errors::{VexfsError, VexfsResult};
use crate::ipc::{IpcError, IpcResult};

#[cfg(not(feature = "kernel"))]
use std::{vec::Vec, string::String, collections::BTreeMap};
#[cfg(feature = "kernel")]
use alloc::{vec::Vec, vec, string::{String, ToString}, collections::BTreeMap};

/// Current IPC protocol version
pub const IPC_PROTOCOL_VERSION: u32 = 1;

/// Magic number for IPC messages
pub const IPC_MAGIC: u32 = 0x56455849; // "VEXI" in ASCII

/// Maximum message size (16MB)
pub const MAX_MESSAGE_SIZE: usize = 16 * 1024 * 1024;

/// Maximum embedding dimensions
pub const MAX_EMBEDDING_DIMENSIONS: u32 = 8192;

/// Maximum batch size for embedding requests
pub const MAX_BATCH_SIZE: usize = 1000;

/// Maximum service name length
pub const MAX_SERVICE_NAME_LEN: usize = 64;

/// Maximum capability string length
pub const MAX_CAPABILITY_LEN: usize = 256;

/// IPC message types
#[derive(Debug, Clone, PartialEq)]
#[repr(u32)]
pub enum MessageType {
    // Service management
    ServiceRegister = 0x0001,
    ServiceUnregister = 0x0002,
    ServiceHeartbeat = 0x0003,
    ServiceDiscovery = 0x0004,
    
    // Embedding operations
    EmbeddingRequest = 0x0100,
    EmbeddingResponse = 0x0101,
    BatchEmbeddingRequest = 0x0102,
    BatchEmbeddingResponse = 0x0103,
    
    // Status and control
    StatusRequest = 0x0200,
    StatusResponse = 0x0201,
    HealthCheck = 0x0202,
    HealthResponse = 0x0203,
    
    // Error handling
    Error = 0x0300,
    Ack = 0x0301,
    Nack = 0x0302,
}

impl From<u32> for MessageType {
    fn from(value: u32) -> Self {
        match value {
            0x0001 => MessageType::ServiceRegister,
            0x0002 => MessageType::ServiceUnregister,
            0x0003 => MessageType::ServiceHeartbeat,
            0x0004 => MessageType::ServiceDiscovery,
            0x0100 => MessageType::EmbeddingRequest,
            0x0101 => MessageType::EmbeddingResponse,
            0x0102 => MessageType::BatchEmbeddingRequest,
            0x0103 => MessageType::BatchEmbeddingResponse,
            0x0200 => MessageType::StatusRequest,
            0x0201 => MessageType::StatusResponse,
            0x0202 => MessageType::HealthCheck,
            0x0203 => MessageType::HealthResponse,
            0x0300 => MessageType::Error,
            0x0301 => MessageType::Ack,
            0x0302 => MessageType::Nack,
            _ => MessageType::Error, // Default to error for unknown types
        }
    }
}

/// IPC message header
#[derive(Debug, Clone, PartialEq)]
#[repr(C)]
pub struct MessageHeader {
    /// Magic number for validation
    pub magic: u32,
    /// Protocol version
    pub version: u32,
    /// Message type
    pub message_type: u32,
    /// Message length (including header)
    pub length: u32,
    /// Request/response correlation ID
    pub correlation_id: u64,
    /// Timestamp (milliseconds since epoch)
    pub timestamp: u64,
    /// Flags for message options
    pub flags: u32,
    /// Checksum for integrity
    pub checksum: u32,
}

impl MessageHeader {
    /// Create a new message header
    pub fn new(message_type: MessageType, length: u32, correlation_id: u64) -> Self {
        Self {
            magic: IPC_MAGIC,
            version: IPC_PROTOCOL_VERSION,
            message_type: message_type as u32,
            length,
            correlation_id,
            timestamp: 0, // Will be set by transport layer
            flags: 0,
            checksum: 0, // Will be calculated during serialization
        }
    }

    /// Validate the header
    pub fn validate(&self) -> IpcResult<()> {
        if self.magic != IPC_MAGIC {
            return Err(IpcError::InvalidMessage("Invalid magic number".to_string()));
        }
        
        if self.version != IPC_PROTOCOL_VERSION {
            return Err(IpcError::ProtocolMismatch {
                expected: IPC_PROTOCOL_VERSION,
                found: self.version,
            });
        }
        
        if self.length > MAX_MESSAGE_SIZE as u32 {
            return Err(IpcError::InvalidMessage("Message too large".to_string()));
        }
        
        Ok(())
    }
}

/// Main IPC message enum
#[derive(Debug, Clone)]
pub enum IpcMessage {
    /// Service registration
    ServiceRegister {
        header: MessageHeader,
        service_info: ServiceInfo,
    },
    
    /// Service unregistration
    ServiceUnregister {
        header: MessageHeader,
        service_id: String,
    },
    
    /// Service heartbeat
    ServiceHeartbeat {
        header: MessageHeader,
        service_id: String,
        load_info: ServiceLoadInfo,
    },
    
    /// Service discovery request
    ServiceDiscovery {
        header: MessageHeader,
        capabilities_filter: Option<ServiceCapabilities>,
    },
    
    /// Embedding request
    EmbeddingRequest {
        request_id: u64,
        service_id: String,
        request: EmbeddingRequest,
        timestamp: u64,
    },
    
    /// Embedding response
    EmbeddingResponse {
        request_id: u64,
        response: EmbeddingResponse,
        timestamp: u64,
    },
    
    /// Batch embedding request
    BatchEmbeddingRequest {
        request_id: u64,
        service_id: String,
        requests: Vec<EmbeddingRequest>,
        timestamp: u64,
    },
    
    /// Batch embedding response
    BatchEmbeddingResponse {
        request_id: u64,
        responses: Vec<EmbeddingResponse>,
        timestamp: u64,
    },
    
    /// Status request
    StatusRequest {
        header: MessageHeader,
        service_id: Option<String>,
    },
    
    /// Status response
    StatusResponse {
        header: MessageHeader,
        status: ServiceStatus,
    },
    
    /// Health check
    HealthCheck {
        header: MessageHeader,
        service_id: String,
    },
    
    /// Health response
    HealthResponse {
        header: MessageHeader,
        health: ServiceHealth,
    },
    
    /// Error message
    Error {
        header: MessageHeader,
        error: String,
        error_code: u32,
    },
    
    /// Acknowledgment
    Ack {
        header: MessageHeader,
    },
    
    /// Negative acknowledgment
    Nack {
        header: MessageHeader,
        reason: String,
    },
}

/// Service information for registration
#[derive(Debug, Clone, PartialEq)]
pub struct ServiceInfo {
    /// Unique service identifier
    pub id: String,
    /// Human-readable service name
    pub name: String,
    /// Service version
    pub version: String,
    /// Service capabilities
    pub capabilities: ServiceCapabilities,
    /// Service endpoint information
    pub endpoint: ServiceEndpoint,
    /// Service metadata
    pub metadata: BTreeMap<String, String>,
}

/// Service capabilities
#[derive(Debug, Clone, PartialEq)]
pub struct ServiceCapabilities {
    /// Supported vector dimensions
    pub supported_dimensions: Vec<u32>,
    /// Supported data types
    pub supported_data_types: Vec<VectorDataType>,
    /// Maximum batch size
    pub max_batch_size: usize,
    /// Supported embedding models
    pub supported_models: Vec<String>,
    /// Performance characteristics
    pub performance_info: PerformanceInfo,
    /// Feature flags
    pub features: Vec<String>,
}

/// Vector data types supported by services
#[derive(Debug, Clone, PartialEq)]
#[repr(u8)]
pub enum VectorDataType {
    Float32 = 0,
    Float16 = 1,
    Int8 = 2,
    Int16 = 3,
    Binary = 4,
}

/// Service endpoint information
#[derive(Debug, Clone, PartialEq)]
pub struct ServiceEndpoint {
    /// Transport type (netlink, unix_socket, etc.)
    pub transport_type: String,
    /// Address or path
    pub address: String,
    /// Port (if applicable)
    pub port: Option<u16>,
    /// Authentication token
    pub auth_token: Option<String>,
}

/// Performance information
#[derive(Debug, Clone, PartialEq)]
pub struct PerformanceInfo {
    /// Average latency in microseconds
    pub avg_latency_us: u64,
    /// Throughput in embeddings per second
    pub throughput_eps: u64,
    /// Memory usage in bytes
    pub memory_usage_bytes: u64,
    /// CPU usage percentage (0-100)
    pub cpu_usage_percent: u8,
}

/// Service load information
#[derive(Debug, Clone, PartialEq)]
pub struct ServiceLoadInfo {
    /// Current CPU usage (0.0-1.0)
    pub cpu_usage: f32,
    /// Current memory usage (0.0-1.0)
    pub memory_usage: f32,
    /// Active request count
    pub active_requests: u32,
    /// Queue depth
    pub queue_depth: u32,
    /// Average response time in milliseconds
    pub avg_response_time_ms: u64,
}

/// Service status
#[derive(Debug, Clone, PartialEq)]
pub struct ServiceStatus {
    /// Service ID
    pub service_id: String,
    /// Current state
    pub state: ServiceState,
    /// Uptime in seconds
    pub uptime_seconds: u64,
    /// Total requests processed
    pub total_requests: u64,
    /// Failed requests
    pub failed_requests: u64,
    /// Load information
    pub load_info: ServiceLoadInfo,
}

/// Service state enumeration
#[derive(Debug, Clone, PartialEq)]
#[repr(u8)]
pub enum ServiceState {
    Starting = 0,
    Ready = 1,
    Busy = 2,
    Overloaded = 3,
    Stopping = 4,
    Stopped = 5,
    Error = 6,
}

/// Service health information
#[derive(Debug, Clone, PartialEq)]
pub struct ServiceHealth {
    /// Overall health status
    pub status: HealthStatus,
    /// Health score (0-100)
    pub score: u8,
    /// Last health check timestamp
    pub last_check: u64,
    /// Health details
    pub details: BTreeMap<String, String>,
}

/// Health status enumeration
#[derive(Debug, Clone, PartialEq)]
#[repr(u8)]
pub enum HealthStatus {
    Healthy = 0,
    Degraded = 1,
    Unhealthy = 2,
    Unknown = 3,
}

/// Embedding request structure
#[derive(Debug, Clone)]
pub struct EmbeddingRequest {
    /// Request ID for correlation
    pub request_id: u64,
    /// Vector dimensions
    pub dimensions: u32,
    /// Input data for embedding
    pub data: Vec<u8>,
    /// Data type
    pub data_type: VectorDataType,
    /// Model to use (optional)
    pub model: Option<String>,
    /// Additional parameters
    pub parameters: BTreeMap<String, String>,
    /// Priority (0-255, higher is more urgent)
    pub priority: u8,
    /// Timeout in milliseconds
    pub timeout_ms: u64,
}

/// Embedding response structure
#[derive(Debug, Clone)]
pub struct EmbeddingResponse {
    /// Request ID for correlation
    pub request_id: u64,
    /// Result status
    pub status: ResponseStatus,
    /// Generated embedding vector
    pub embedding: Option<Vec<f32>>,
    /// Error message if failed
    pub error: Option<String>,
    /// Processing time in microseconds
    pub processing_time_us: u64,
    /// Model used
    pub model_used: Option<String>,
    /// Additional metadata
    pub metadata: BTreeMap<String, String>,
}

/// Response status enumeration
#[derive(Debug, Clone, PartialEq)]
#[repr(u8)]
pub enum ResponseStatus {
    Success = 0,
    Error = 1,
    Timeout = 2,
    Overloaded = 3,
    InvalidRequest = 4,
    ModelNotFound = 5,
    InternalError = 6,
}

/// Message serialization trait
pub trait MessageSerialization {
    /// Serialize message to bytes
    fn serialize(&self) -> IpcResult<Vec<u8>>;
    
    /// Deserialize message from bytes
    fn deserialize(data: &[u8]) -> IpcResult<Self>
    where
        Self: Sized;
    
    /// Calculate message size
    fn size(&self) -> usize;
}

impl MessageSerialization for IpcMessage {
    fn serialize(&self) -> IpcResult<Vec<u8>> {
        // Simple binary serialization
        // In a real implementation, would use a proper serialization format
        // like Protocol Buffers, MessagePack, or custom binary format
        
        let mut buffer = Vec::new();
        
        match self {
            IpcMessage::EmbeddingRequest { request_id, service_id, request, timestamp } => {
                // Serialize embedding request
                buffer.extend_from_slice(&(MessageType::EmbeddingRequest as u32).to_le_bytes());
                buffer.extend_from_slice(&request_id.to_le_bytes());
                buffer.extend_from_slice(&(service_id.len() as u32).to_le_bytes());
                buffer.extend_from_slice(service_id.as_bytes());
                buffer.extend_from_slice(&request.dimensions.to_le_bytes());
                buffer.extend_from_slice(&(request.data.len() as u32).to_le_bytes());
                buffer.extend_from_slice(&request.data);
                buffer.extend_from_slice(&timestamp.to_le_bytes());
            }
            
            IpcMessage::EmbeddingResponse { request_id, response, timestamp } => {
                // Serialize embedding response
                buffer.extend_from_slice(&(MessageType::EmbeddingResponse as u32).to_le_bytes());
                buffer.extend_from_slice(&request_id.to_le_bytes());
                buffer.extend_from_slice(&(response.status.clone() as u8).to_le_bytes());
                
                if let Some(ref embedding) = response.embedding {
                    buffer.extend_from_slice(&(embedding.len() as u32).to_le_bytes());
                    for &value in embedding {
                        buffer.extend_from_slice(&value.to_le_bytes());
                    }
                } else {
                    buffer.extend_from_slice(&0u32.to_le_bytes());
                }
                
                buffer.extend_from_slice(&timestamp.to_le_bytes());
            }
            
            IpcMessage::Error { header: _, error, error_code } => {
                // Serialize error message
                buffer.extend_from_slice(&(MessageType::Error as u32).to_le_bytes());
                buffer.extend_from_slice(&error_code.to_le_bytes());
                buffer.extend_from_slice(&(error.len() as u32).to_le_bytes());
                buffer.extend_from_slice(error.as_bytes());
            }
            
            _ => {
                return Err(IpcError::SerializationError("Unsupported message type".to_string()));
            }
        }
        
        Ok(buffer)
    }
    
    fn deserialize(data: &[u8]) -> IpcResult<Self> {
        if data.len() < 4 {
            return Err(IpcError::SerializationError("Data too short".to_string()));
        }
        
        let message_type = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        let message_type = MessageType::from(message_type);
        
        match message_type {
            MessageType::EmbeddingRequest => {
                // Deserialize embedding request
                if data.len() < 16 {
                    return Err(IpcError::SerializationError("Invalid embedding request".to_string()));
                }
                
                let request_id = u64::from_le_bytes([
                    data[4], data[5], data[6], data[7],
                    data[8], data[9], data[10], data[11]
                ]);
                
                // Simplified deserialization - would need proper implementation
                Ok(IpcMessage::EmbeddingRequest {
                    request_id,
                    service_id: "default".to_string(),
                    request: EmbeddingRequest {
                        request_id,
                        dimensions: 128,
                        data: vec![0; 512],
                        data_type: VectorDataType::Float32,
                        model: None,
                        parameters: BTreeMap::new(),
                        priority: 0,
                        timeout_ms: 30000,
                    },
                    timestamp: 0,
                })
            }
            
            MessageType::EmbeddingResponse => {
                // Deserialize embedding response
                if data.len() < 16 {
                    return Err(IpcError::SerializationError("Invalid embedding response".to_string()));
                }
                
                let request_id = u64::from_le_bytes([
                    data[4], data[5], data[6], data[7],
                    data[8], data[9], data[10], data[11]
                ]);
                
                Ok(IpcMessage::EmbeddingResponse {
                    request_id,
                    response: EmbeddingResponse {
                        request_id,
                        status: ResponseStatus::Success,
                        embedding: Some(vec![0.0; 128]),
                        error: None,
                        processing_time_us: 1000,
                        model_used: None,
                        metadata: BTreeMap::new(),
                    },
                    timestamp: 0,
                })
            }
            
            _ => Err(IpcError::SerializationError("Unsupported message type for deserialization".to_string())),
        }
    }
    
    fn size(&self) -> usize {
        // Calculate approximate message size
        match self {
            IpcMessage::EmbeddingRequest { service_id, request, .. } => {
                32 + service_id.len() + request.data.len()
            }
            IpcMessage::EmbeddingResponse { response, .. } => {
                32 + response.embedding.as_ref().map_or(0, |e| e.len() * 4)
            }
            _ => 64, // Default size for other message types
        }
    }
}

/// Protocol validation utilities
pub struct ProtocolValidator;

impl ProtocolValidator {
    /// Validate message format
    pub fn validate_message(message: &IpcMessage) -> IpcResult<()> {
        match message {
            IpcMessage::EmbeddingRequest { request, .. } => {
                Self::validate_embedding_request(request)
            }
            IpcMessage::EmbeddingResponse { response, .. } => {
                Self::validate_embedding_response(response)
            }
            _ => Ok(()), // Other message types don't need validation yet
        }
    }
    
    /// Validate embedding request
    pub fn validate_embedding_request(request: &EmbeddingRequest) -> IpcResult<()> {
        if request.dimensions == 0 || request.dimensions > MAX_EMBEDDING_DIMENSIONS {
            return Err(IpcError::InvalidMessage("Invalid dimensions".to_string()));
        }
        
        if request.data.is_empty() {
            return Err(IpcError::InvalidMessage("Empty data".to_string()));
        }
        
        if request.timeout_ms == 0 {
            return Err(IpcError::InvalidMessage("Invalid timeout".to_string()));
        }
        
        Ok(())
    }
    
    /// Validate embedding response
    pub fn validate_embedding_response(response: &EmbeddingResponse) -> IpcResult<()> {
        match response.status {
            ResponseStatus::Success => {
                if response.embedding.is_none() {
                    return Err(IpcError::InvalidMessage("Missing embedding in successful response".to_string()));
                }
            }
            ResponseStatus::Error | ResponseStatus::InternalError => {
                if response.error.is_none() {
                    return Err(IpcError::InvalidMessage("Missing error message in error response".to_string()));
                }
            }
            _ => {} // Other statuses are valid as-is
        }
        
        Ok(())
    }
}