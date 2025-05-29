//! IPC Message Definitions for VexFS Embedding Services
//!
//! This module defines the specific message structures and utilities
//! for IPC communication between VexFS kernel module and userspace
//! embedding services.

use crate::shared::errors::{VexfsError, VexfsResult};
use crate::ipc::{IpcError, IpcResult};
use crate::ipc::protocol::*;

#[cfg(not(feature = "std"))]
use alloc::{vec::Vec, string::{String, ToString}, collections::BTreeMap};
#[cfg(feature = "std")]
#[cfg(feature = "kernel")]
use alloc::{vec::Vec, string::{String, ToString}, collections::BTreeMap};
#[cfg(not(feature = "kernel"))]
use std::{vec::Vec, string::{String, ToString}, collections::BTreeMap};

/// Message builder for creating IPC messages
pub struct MessageBuilder;

impl MessageBuilder {
    /// Create an embedding request message
    pub fn create_embedding_request(
        request_id: u64,
        service_id: String,
        dimensions: u32,
        data: Vec<u8>,
        data_type: VectorDataType,
        model: Option<String>,
        priority: u8,
        timeout_ms: u64,
    ) -> IpcResult<IpcMessage> {
        // Validate parameters
        if dimensions == 0 || dimensions > MAX_EMBEDDING_DIMENSIONS {
            return Err(IpcError::InvalidMessage("Invalid dimensions".to_string()));
        }
        
        if data.is_empty() {
            return Err(IpcError::InvalidMessage("Empty data".to_string()));
        }
        
        if service_id.is_empty() || service_id.len() > MAX_SERVICE_NAME_LEN {
            return Err(IpcError::InvalidMessage("Invalid service ID".to_string()));
        }

        let request = EmbeddingRequest {
            request_id,
            dimensions,
            data,
            data_type,
            model,
            parameters: BTreeMap::new(),
            priority,
            timeout_ms,
        };

        Ok(IpcMessage::EmbeddingRequest {
            request_id,
            service_id,
            request,
            timestamp: Self::get_current_timestamp(),
        })
    }

    /// Create an embedding response message
    pub fn create_embedding_response(
        request_id: u64,
        status: ResponseStatus,
        embedding: Option<Vec<f32>>,
        error: Option<String>,
        processing_time_us: u64,
        model_used: Option<String>,
    ) -> IpcResult<IpcMessage> {
        // Validate response consistency before creating the response
        match status {
            ResponseStatus::Success => {
                if embedding.is_none() {
                    return Err(IpcError::InvalidMessage("Missing embedding in successful response".to_string()));
                }
            }
            ResponseStatus::Error | ResponseStatus::InternalError => {
                if error.is_none() {
                    return Err(IpcError::InvalidMessage("Missing error message in error response".to_string()));
                }
            }
            _ => {} // Other statuses are valid as-is
        }

        let response = EmbeddingResponse {
            request_id,
            status,
            embedding,
            error,
            processing_time_us,
            model_used,
            metadata: BTreeMap::new(),
        };

        Ok(IpcMessage::EmbeddingResponse {
            request_id,
            response,
            timestamp: Self::get_current_timestamp(),
        })
    }

    /// Create a batch embedding request message
    pub fn create_batch_embedding_request(
        request_id: u64,
        service_id: String,
        requests: Vec<EmbeddingRequest>,
    ) -> IpcResult<IpcMessage> {
        if requests.is_empty() {
            return Err(IpcError::InvalidMessage("Empty batch request".to_string()));
        }
        
        if requests.len() > MAX_BATCH_SIZE {
            return Err(IpcError::InvalidMessage("Batch too large".to_string()));
        }

        // Validate all requests in the batch
        for request in &requests {
            Self::validate_embedding_request(request)?;
        }

        Ok(IpcMessage::BatchEmbeddingRequest {
            request_id,
            service_id,
            requests,
            timestamp: Self::get_current_timestamp(),
        })
    }

    /// Create a batch embedding response message
    pub fn create_batch_embedding_response(
        request_id: u64,
        responses: Vec<EmbeddingResponse>,
    ) -> IpcResult<IpcMessage> {
        if responses.is_empty() {
            return Err(IpcError::InvalidMessage("Empty batch response".to_string()));
        }

        // Validate all responses in the batch
        for response in &responses {
            Self::validate_embedding_response(response)?;
        }

        Ok(IpcMessage::BatchEmbeddingResponse {
            request_id,
            responses,
            timestamp: Self::get_current_timestamp(),
        })
    }

    /// Create a service registration message
    pub fn create_service_registration(
        service_id: String,
        service_name: String,
        version: String,
        capabilities: ServiceCapabilities,
        endpoint: ServiceEndpoint,
    ) -> IpcResult<IpcMessage> {
        // Validate service information
        if service_id.is_empty() || service_id.len() > MAX_SERVICE_NAME_LEN {
            return Err(IpcError::InvalidMessage("Invalid service ID".to_string()));
        }

        if service_name.is_empty() || service_name.len() > MAX_SERVICE_NAME_LEN {
            return Err(IpcError::InvalidMessage("Invalid service name".to_string()));
        }

        Self::validate_service_capabilities(&capabilities)?;

        let service_info = ServiceInfo {
            id: service_id,
            name: service_name,
            version,
            capabilities,
            endpoint,
            metadata: BTreeMap::new(),
        };

        let header = MessageHeader::new(
            MessageType::ServiceRegister,
            0, // Will be calculated during serialization
            Self::generate_correlation_id(),
        );

        Ok(IpcMessage::ServiceRegister {
            header,
            service_info,
        })
    }

    /// Create a service unregistration message
    pub fn create_service_unregistration(service_id: String) -> IpcResult<IpcMessage> {
        if service_id.is_empty() || service_id.len() > MAX_SERVICE_NAME_LEN {
            return Err(IpcError::InvalidMessage("Invalid service ID".to_string()));
        }

        let header = MessageHeader::new(
            MessageType::ServiceUnregister,
            0, // Will be calculated during serialization
            Self::generate_correlation_id(),
        );

        Ok(IpcMessage::ServiceUnregister {
            header,
            service_id,
        })
    }

    /// Create a service heartbeat message
    pub fn create_service_heartbeat(
        service_id: String,
        load_info: ServiceLoadInfo,
    ) -> IpcResult<IpcMessage> {
        if service_id.is_empty() || service_id.len() > MAX_SERVICE_NAME_LEN {
            return Err(IpcError::InvalidMessage("Invalid service ID".to_string()));
        }

        Self::validate_load_info(&load_info)?;

        let header = MessageHeader::new(
            MessageType::ServiceHeartbeat,
            0, // Will be calculated during serialization
            Self::generate_correlation_id(),
        );

        Ok(IpcMessage::ServiceHeartbeat {
            header,
            service_id,
            load_info,
        })
    }

    /// Create a health check message
    pub fn create_health_check(service_id: String) -> IpcResult<IpcMessage> {
        if service_id.is_empty() || service_id.len() > MAX_SERVICE_NAME_LEN {
            return Err(IpcError::InvalidMessage("Invalid service ID".to_string()));
        }

        let header = MessageHeader::new(
            MessageType::HealthCheck,
            0, // Will be calculated during serialization
            Self::generate_correlation_id(),
        );

        Ok(IpcMessage::HealthCheck {
            header,
            service_id,
        })
    }

    /// Create a health response message
    pub fn create_health_response(health: ServiceHealth) -> IpcResult<IpcMessage> {
        Self::validate_service_health(&health)?;

        let header = MessageHeader::new(
            MessageType::HealthResponse,
            0, // Will be calculated during serialization
            Self::generate_correlation_id(),
        );

        Ok(IpcMessage::HealthResponse {
            header,
            health,
        })
    }

    /// Create an error message
    pub fn create_error_message(
        correlation_id: u64,
        error: String,
        error_code: u32,
    ) -> IpcResult<IpcMessage> {
        let mut header = MessageHeader::new(
            MessageType::Error,
            0, // Will be calculated during serialization
            correlation_id,
        );
        header.correlation_id = correlation_id; // Use provided correlation ID

        Ok(IpcMessage::Error {
            header,
            error,
            error_code,
        })
    }

    /// Create an acknowledgment message
    pub fn create_ack_message(correlation_id: u64) -> IpcResult<IpcMessage> {
        let mut header = MessageHeader::new(
            MessageType::Ack,
            0, // Will be calculated during serialization
            correlation_id,
        );
        header.correlation_id = correlation_id; // Use provided correlation ID

        Ok(IpcMessage::Ack { header })
    }

    /// Create a negative acknowledgment message
    pub fn create_nack_message(correlation_id: u64, reason: String) -> IpcResult<IpcMessage> {
        let mut header = MessageHeader::new(
            MessageType::Nack,
            0, // Will be calculated during serialization
            correlation_id,
        );
        header.correlation_id = correlation_id; // Use provided correlation ID

        Ok(IpcMessage::Nack { header, reason })
    }

    // Private helper methods

    fn validate_embedding_request(request: &EmbeddingRequest) -> IpcResult<()> {
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

    fn validate_embedding_response(response: &EmbeddingResponse) -> IpcResult<()> {
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

    fn validate_service_capabilities(capabilities: &ServiceCapabilities) -> IpcResult<()> {
        if capabilities.supported_dimensions.is_empty() {
            return Err(IpcError::InvalidCapability("No supported dimensions".to_string()));
        }
        
        if capabilities.max_batch_size == 0 {
            return Err(IpcError::InvalidCapability("Invalid batch size".to_string()));
        }
        
        // Validate dimension ranges
        for &dim in &capabilities.supported_dimensions {
            if dim == 0 || dim > MAX_EMBEDDING_DIMENSIONS {
                return Err(IpcError::InvalidCapability("Invalid dimension value".to_string()));
            }
        }
        
        Ok(())
    }

    fn validate_load_info(load_info: &ServiceLoadInfo) -> IpcResult<()> {
        if load_info.cpu_usage < 0.0 || load_info.cpu_usage > 1.0 {
            return Err(IpcError::InvalidMessage("Invalid CPU usage".to_string()));
        }
        
        if load_info.memory_usage < 0.0 || load_info.memory_usage > 1.0 {
            return Err(IpcError::InvalidMessage("Invalid memory usage".to_string()));
        }
        
        Ok(())
    }

    fn validate_service_health(health: &ServiceHealth) -> IpcResult<()> {
        if health.score > 100 {
            return Err(IpcError::InvalidMessage("Invalid health score".to_string()));
        }
        
        Ok(())
    }

    fn get_current_timestamp() -> u64 {
        // Get current timestamp in milliseconds
        // In kernel mode, would use kernel time functions
        // In userspace, would use system time
        0 // Placeholder
    }

    fn generate_correlation_id() -> u64 {
        // Generate unique correlation ID
        // In real implementation, would use atomic counter or UUID
        42 // Placeholder
    }
}

/// Message parser for extracting information from IPC messages
pub struct MessageParser;

impl MessageParser {
    /// Extract request ID from any message type
    pub fn extract_request_id(message: &IpcMessage) -> Option<u64> {
        match message {
            IpcMessage::EmbeddingRequest { request_id, .. } => Some(*request_id),
            IpcMessage::EmbeddingResponse { request_id, .. } => Some(*request_id),
            IpcMessage::BatchEmbeddingRequest { request_id, .. } => Some(*request_id),
            IpcMessage::BatchEmbeddingResponse { request_id, .. } => Some(*request_id),
            _ => None,
        }
    }

    /// Extract correlation ID from message header
    pub fn extract_correlation_id(message: &IpcMessage) -> Option<u64> {
        match message {
            IpcMessage::ServiceRegister { header, .. } => Some(header.correlation_id),
            IpcMessage::ServiceUnregister { header, .. } => Some(header.correlation_id),
            IpcMessage::ServiceHeartbeat { header, .. } => Some(header.correlation_id),
            IpcMessage::ServiceDiscovery { header, .. } => Some(header.correlation_id),
            IpcMessage::StatusRequest { header, .. } => Some(header.correlation_id),
            IpcMessage::StatusResponse { header, .. } => Some(header.correlation_id),
            IpcMessage::HealthCheck { header, .. } => Some(header.correlation_id),
            IpcMessage::HealthResponse { header, .. } => Some(header.correlation_id),
            IpcMessage::Error { header, .. } => Some(header.correlation_id),
            IpcMessage::Ack { header, .. } => Some(header.correlation_id),
            IpcMessage::Nack { header, .. } => Some(header.correlation_id),
            _ => None,
        }
    }

    /// Extract service ID from message
    pub fn extract_service_id(message: &IpcMessage) -> Option<&String> {
        match message {
            IpcMessage::ServiceRegister { service_info, .. } => Some(&service_info.id),
            IpcMessage::ServiceUnregister { service_id, .. } => Some(service_id),
            IpcMessage::ServiceHeartbeat { service_id, .. } => Some(service_id),
            IpcMessage::EmbeddingRequest { service_id, .. } => Some(service_id),
            IpcMessage::BatchEmbeddingRequest { service_id, .. } => Some(service_id),
            IpcMessage::HealthCheck { service_id, .. } => Some(service_id),
            _ => None,
        }
    }

    /// Check if message is a request type
    pub fn is_request_message(message: &IpcMessage) -> bool {
        matches!(message,
            IpcMessage::ServiceRegister { .. } |
            IpcMessage::ServiceUnregister { .. } |
            IpcMessage::ServiceDiscovery { .. } |
            IpcMessage::EmbeddingRequest { .. } |
            IpcMessage::BatchEmbeddingRequest { .. } |
            IpcMessage::StatusRequest { .. } |
            IpcMessage::HealthCheck { .. }
        )
    }

    /// Check if message is a response type
    pub fn is_response_message(message: &IpcMessage) -> bool {
        matches!(message,
            IpcMessage::EmbeddingResponse { .. } |
            IpcMessage::BatchEmbeddingResponse { .. } |
            IpcMessage::StatusResponse { .. } |
            IpcMessage::HealthResponse { .. } |
            IpcMessage::Ack { .. } |
            IpcMessage::Nack { .. } |
            IpcMessage::Error { .. }
        )
    }

    /// Get message type as string
    pub fn get_message_type_string(message: &IpcMessage) -> &'static str {
        match message {
            IpcMessage::ServiceRegister { .. } => "ServiceRegister",
            IpcMessage::ServiceUnregister { .. } => "ServiceUnregister",
            IpcMessage::ServiceHeartbeat { .. } => "ServiceHeartbeat",
            IpcMessage::ServiceDiscovery { .. } => "ServiceDiscovery",
            IpcMessage::EmbeddingRequest { .. } => "EmbeddingRequest",
            IpcMessage::EmbeddingResponse { .. } => "EmbeddingResponse",
            IpcMessage::BatchEmbeddingRequest { .. } => "BatchEmbeddingRequest",
            IpcMessage::BatchEmbeddingResponse { .. } => "BatchEmbeddingResponse",
            IpcMessage::StatusRequest { .. } => "StatusRequest",
            IpcMessage::StatusResponse { .. } => "StatusResponse",
            IpcMessage::HealthCheck { .. } => "HealthCheck",
            IpcMessage::HealthResponse { .. } => "HealthResponse",
            IpcMessage::Error { .. } => "Error",
            IpcMessage::Ack { .. } => "Ack",
            IpcMessage::Nack { .. } => "Nack",
        }
    }
}

/// Message validator for comprehensive message validation
pub struct MessageValidator;

impl MessageValidator {
    /// Validate any IPC message
    pub fn validate_message(message: &IpcMessage) -> IpcResult<()> {
        match message {
            IpcMessage::ServiceRegister { service_info, .. } => {
                Self::validate_service_info(service_info)
            }
            IpcMessage::EmbeddingRequest { request, .. } => {
                MessageBuilder::validate_embedding_request(request)
            }
            IpcMessage::EmbeddingResponse { response, .. } => {
                MessageBuilder::validate_embedding_response(response)
            }
            IpcMessage::BatchEmbeddingRequest { requests, .. } => {
                for request in requests {
                    MessageBuilder::validate_embedding_request(request)?;
                }
                Ok(())
            }
            IpcMessage::BatchEmbeddingResponse { responses, .. } => {
                for response in responses {
                    MessageBuilder::validate_embedding_response(response)?;
                }
                Ok(())
            }
            IpcMessage::ServiceHeartbeat { load_info, .. } => {
                MessageBuilder::validate_load_info(load_info)
            }
            IpcMessage::HealthResponse { health, .. } => {
                MessageBuilder::validate_service_health(health)
            }
            _ => Ok(()), // Other message types don't need specific validation
        }
    }

    /// Validate service information
    fn validate_service_info(service_info: &ServiceInfo) -> IpcResult<()> {
        if service_info.id.is_empty() || service_info.id.len() > MAX_SERVICE_NAME_LEN {
            return Err(IpcError::InvalidMessage("Invalid service ID".to_string()));
        }

        if service_info.name.is_empty() || service_info.name.len() > MAX_SERVICE_NAME_LEN {
            return Err(IpcError::InvalidMessage("Invalid service name".to_string()));
        }

        MessageBuilder::validate_service_capabilities(&service_info.capabilities)?;

        Ok(())
    }
}

/// Message statistics for monitoring and debugging
#[derive(Debug, Clone, Default)]
pub struct MessageStats {
    pub total_messages: u64,
    pub embedding_requests: u64,
    pub embedding_responses: u64,
    pub batch_requests: u64,
    pub batch_responses: u64,
    pub service_registrations: u64,
    pub service_unregistrations: u64,
    pub heartbeats: u64,
    pub health_checks: u64,
    pub errors: u64,
    pub acks: u64,
    pub nacks: u64,
}

impl MessageStats {
    /// Update statistics for a message
    pub fn update_for_message(&mut self, message: &IpcMessage) {
        self.total_messages += 1;
        
        match message {
            IpcMessage::EmbeddingRequest { .. } => self.embedding_requests += 1,
            IpcMessage::EmbeddingResponse { .. } => self.embedding_responses += 1,
            IpcMessage::BatchEmbeddingRequest { .. } => self.batch_requests += 1,
            IpcMessage::BatchEmbeddingResponse { .. } => self.batch_responses += 1,
            IpcMessage::ServiceRegister { .. } => self.service_registrations += 1,
            IpcMessage::ServiceUnregister { .. } => self.service_unregistrations += 1,
            IpcMessage::ServiceHeartbeat { .. } => self.heartbeats += 1,
            IpcMessage::HealthCheck { .. } | IpcMessage::HealthResponse { .. } => self.health_checks += 1,
            IpcMessage::Error { .. } => self.errors += 1,
            IpcMessage::Ack { .. } => self.acks += 1,
            IpcMessage::Nack { .. } => self.nacks += 1,
            _ => {} // Other message types
        }
    }

    /// Get success rate
    pub fn get_success_rate(&self) -> f64 {
        if self.total_messages == 0 {
            return 0.0;
        }
        
        let successful = self.total_messages - self.errors - self.nacks;
        successful as f64 / self.total_messages as f64
    }
}