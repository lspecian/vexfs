//! Inter-Process Communication Module for VexFS
//!
//! This module implements the IPC infrastructure for communication between
//! the VexFS kernel module and userspace embedding services. It provides
//! secure, efficient communication channels for vector embedding operations.

use crate::shared::errors::{VexfsError, VexfsResult};
use crate::shared::types::*;
use crate::fs_core::OperationContext;

#[cfg(not(feature = "kernel"))]
use std::sync::Arc;
#[cfg(feature = "kernel")]
use alloc::sync::Arc;

#[cfg(not(feature = "kernel"))]
use std::{vec::Vec, string::{String, ToString}, collections::BTreeMap, boxed::Box, format};
#[cfg(feature = "kernel")]
use alloc::{vec::Vec, string::{String, ToString}, collections::BTreeMap, boxed::Box, format};

// Sub-modules
pub mod protocol;
pub mod transport;
pub mod messages;
pub mod service_registry;
pub mod service_manager;
pub mod load_balancer;
pub mod request_handler;
pub mod response_manager;
pub mod queue_manager;

// Re-exports
pub use protocol::*;
pub use transport::*;
pub use messages::*;
pub use service_registry::*;
pub use service_manager::*;
pub use load_balancer::*;
pub use request_handler::*;
pub use response_manager::*;
pub use queue_manager::*;

/// IPC error types specific to embedding service communication
#[derive(Debug, Clone, PartialEq)]
pub enum IpcError {
    /// Service not found
    ServiceNotFound(String),
    /// Service unavailable
    ServiceUnavailable(String),
    /// Communication timeout
    Timeout(u64),
    /// Protocol version mismatch
    ProtocolMismatch { expected: u32, found: u32 },
    /// Invalid message format
    InvalidMessage(String),
    /// Authentication failed
    AuthenticationFailed(String),
    /// Authorization denied
    AuthorizationDenied(String),
    /// Transport error
    TransportError(String),
    /// Serialization error
    SerializationError(String),
    /// Service overloaded
    ServiceOverloaded(String),
    /// Invalid service capability
    InvalidCapability(String),
    /// Request queue full
    QueueFull,
    /// Response correlation failed
    CorrelationFailed(u64),
    /// Service registration failed
    RegistrationFailed(String),
    /// Load balancing error
    LoadBalancingError(String),
}

impl core::fmt::Display for IpcError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            IpcError::ServiceNotFound(name) => write!(f, "Service not found: {}", name),
            IpcError::ServiceUnavailable(name) => write!(f, "Service unavailable: {}", name),
            IpcError::Timeout(ms) => write!(f, "Operation timed out after {} ms", ms),
            IpcError::ProtocolMismatch { expected, found } => {
                write!(f, "Protocol version mismatch: expected {}, found {}", expected, found)
            }
            IpcError::InvalidMessage(msg) => write!(f, "Invalid message: {}", msg),
            IpcError::AuthenticationFailed(msg) => write!(f, "Authentication failed: {}", msg),
            IpcError::AuthorizationDenied(msg) => write!(f, "Authorization denied: {}", msg),
            IpcError::TransportError(msg) => write!(f, "Transport error: {}", msg),
            IpcError::SerializationError(msg) => write!(f, "Serialization error: {}", msg),
            IpcError::ServiceOverloaded(name) => write!(f, "Service overloaded: {}", name),
            IpcError::InvalidCapability(cap) => write!(f, "Invalid capability: {}", cap),
            IpcError::QueueFull => write!(f, "Request queue is full"),
            IpcError::CorrelationFailed(id) => write!(f, "Response correlation failed for request {}", id),
            IpcError::RegistrationFailed(msg) => write!(f, "Service registration failed: {}", msg),
            IpcError::LoadBalancingError(msg) => write!(f, "Load balancing error: {}", msg),
        }
    }
}

/// Convert IPC errors to VexFS errors
impl From<IpcError> for VexfsError {
    fn from(err: IpcError) -> Self {
        match err {
            IpcError::ServiceNotFound(msg) => VexfsError::NotFound,
            IpcError::ServiceUnavailable(msg) => VexfsError::ResourceBusy,
            IpcError::Timeout(ms) => VexfsError::Timeout(format!("IPC timeout: {} ms", ms)),
            IpcError::ProtocolMismatch { .. } => VexfsError::InvalidOperation("Protocol mismatch".to_string()),
            IpcError::InvalidMessage(msg) => VexfsError::InvalidData(msg),
            IpcError::AuthenticationFailed(msg) => VexfsError::PermissionDenied(msg),
            IpcError::AuthorizationDenied(msg) => VexfsError::PermissionDenied(msg),
            IpcError::TransportError(msg) => VexfsError::IoError(crate::shared::errors::IoErrorKind::ConnectionLost),
            IpcError::SerializationError(msg) => VexfsError::InvalidData(msg),
            IpcError::ServiceOverloaded(msg) => VexfsError::ResourceBusy,
            IpcError::InvalidCapability(msg) => VexfsError::InvalidArgument(msg),
            IpcError::QueueFull => VexfsError::ResourceBusy,
            IpcError::CorrelationFailed(_) => VexfsError::InternalError("Response correlation failed".to_string()),
            IpcError::RegistrationFailed(msg) => VexfsError::InvalidOperation(msg),
            IpcError::LoadBalancingError(msg) => VexfsError::InternalError(msg),
        }
    }
}

/// IPC result type
pub type IpcResult<T> = core::result::Result<T, IpcError>;

/// Main IPC manager that coordinates all IPC operations
pub struct IpcManager {
    /// Transport layer for communication
    transport: Box<dyn IpcTransport>,
    /// Service registry for discovery
    service_registry: ServiceRegistry,
    /// Service manager for lifecycle
    service_manager: ServiceManager,
    /// Load balancer for request distribution
    load_balancer: LoadBalancer,
    /// Request handler for processing
    request_handler: RequestHandler,
    /// Response manager for correlation
    response_manager: ResponseManager,
    /// Queue manager for request queuing
    queue_manager: QueueManager,
    /// Configuration
    config: IpcConfig,
}

/// IPC configuration
#[derive(Debug, Clone)]
pub struct IpcConfig {
    /// Maximum number of concurrent requests
    pub max_concurrent_requests: usize,
    /// Request timeout in milliseconds
    pub request_timeout_ms: u64,
    /// Maximum queue size
    pub max_queue_size: usize,
    /// Service discovery interval in seconds
    pub discovery_interval_sec: u64,
    /// Health check interval in seconds
    pub health_check_interval_sec: u64,
    /// Maximum retry attempts
    pub max_retry_attempts: u32,
    /// Retry backoff base in milliseconds
    pub retry_backoff_base_ms: u64,
    /// Enable authentication
    pub enable_authentication: bool,
    /// Enable encryption
    pub enable_encryption: bool,
}

impl Default for IpcConfig {
    fn default() -> Self {
        Self {
            max_concurrent_requests: 100,
            request_timeout_ms: 30000, // 30 seconds
            max_queue_size: 1000,
            discovery_interval_sec: 60,
            health_check_interval_sec: 30,
            max_retry_attempts: 3,
            retry_backoff_base_ms: 1000,
            enable_authentication: true,
            enable_encryption: false, // Disabled by default for performance
        }
    }
}

impl IpcManager {
    /// Create a new IPC manager
    pub fn new(transport: Box<dyn IpcTransport>, config: IpcConfig) -> Self {
        let service_registry = ServiceRegistry::new();
        let service_manager = ServiceManager::new(config.clone());
        let load_balancer = LoadBalancer::new();
        let request_handler = RequestHandler::new(config.clone());
        let response_manager = ResponseManager::new(config.clone());
        let queue_manager = QueueManager::new(config.clone());

        Self {
            transport,
            service_registry,
            service_manager,
            load_balancer,
            request_handler,
            response_manager,
            queue_manager,
            config,
        }
    }

    /// Initialize the IPC manager
    pub fn initialize(&mut self) -> IpcResult<()> {
        // Initialize transport
        self.transport.initialize()?;
        
        // Start service discovery
        self.service_manager.start_discovery()?;
        
        // Start health monitoring
        self.service_manager.start_health_monitoring()?;
        
        Ok(())
    }

    /// Shutdown the IPC manager
    pub fn shutdown(&mut self) -> IpcResult<()> {
        // Stop health monitoring
        self.service_manager.stop_health_monitoring()?;
        
        // Stop service discovery
        self.service_manager.stop_discovery()?;
        
        // Shutdown transport
        self.transport.shutdown()?;
        
        Ok(())
    }

    /// Send an embedding request to a service
    pub fn send_embedding_request(
        &mut self,
        request: EmbeddingRequest,
        context: &OperationContext,
    ) -> IpcResult<EmbeddingResponse> {
        // Validate request
        self.validate_request(&request, context)?;
        
        // Select service
        let service = self.load_balancer.select_service(&request)?;
        
        // Queue request if necessary
        if self.should_queue_request(&service)? {
            return self.queue_manager.queue_request(request, service);
        }
        
        // Send request directly
        self.send_request_to_service(request, service, context)
    }

    /// Send a batch embedding request
    pub fn send_batch_embedding_request(
        &mut self,
        requests: Vec<EmbeddingRequest>,
        context: &OperationContext,
    ) -> IpcResult<Vec<EmbeddingResponse>> {
        // Validate batch size
        if requests.len() > self.config.max_queue_size {
            return Err(IpcError::InvalidMessage("Batch too large".to_string()));
        }
        
        // Process requests in parallel (conceptually - actual implementation would use async)
        let mut responses = Vec::with_capacity(requests.len());
        
        for request in requests {
            let response = self.send_embedding_request(request, context)?;
            responses.push(response);
        }
        
        Ok(responses)
    }

    /// Register a new embedding service
    pub fn register_service(&mut self, service_info: ServiceInfo) -> IpcResult<()> {
        // Validate service capabilities
        self.validate_service_capabilities(&service_info)?;
        
        // Register with service registry
        self.service_registry.register_service(service_info.clone())?;
        
        // Add to load balancer
        self.load_balancer.add_service(service_info)?;
        
        Ok(())
    }

    /// Unregister an embedding service
    pub fn unregister_service(&mut self, service_id: &str) -> IpcResult<()> {
        // Remove from load balancer
        self.load_balancer.remove_service(service_id)?;
        
        // Unregister from service registry
        self.service_registry.unregister_service(service_id)?;
        
        Ok(())
    }

    /// Get service statistics
    pub fn get_service_stats(&self, service_id: &str) -> IpcResult<ServiceStats> {
        self.service_registry.get_service_stats(service_id)
    }

    /// Get overall IPC statistics
    pub fn get_ipc_stats(&self) -> IpcStats {
        IpcStats {
            total_requests: self.request_handler.get_total_requests(),
            successful_requests: self.request_handler.get_successful_requests(),
            failed_requests: self.request_handler.get_failed_requests(),
            average_response_time_ms: self.response_manager.get_average_response_time(),
            active_services: self.service_registry.get_active_service_count(),
            queued_requests: self.queue_manager.get_queue_size(),
        }
    }

    // Private helper methods

    fn validate_request(&self, request: &EmbeddingRequest, context: &OperationContext) -> IpcResult<()> {
        // Validate request format
        if request.data.is_empty() {
            return Err(IpcError::InvalidMessage("Empty embedding data".to_string()));
        }
        
        if request.dimensions == 0 || request.dimensions > 8192 {
            return Err(IpcError::InvalidMessage("Invalid dimensions".to_string()));
        }
        
        // Check permissions if authentication is enabled
        if self.config.enable_authentication {
            // TODO: Implement authentication check using SecurityManager
        }
        
        Ok(())
    }

    fn validate_service_capabilities(&self, service_info: &ServiceInfo) -> IpcResult<()> {
        // Validate service capabilities
        if service_info.capabilities.supported_dimensions.is_empty() {
            return Err(IpcError::InvalidCapability("No supported dimensions".to_string()));
        }
        
        if service_info.capabilities.max_batch_size == 0 {
            return Err(IpcError::InvalidCapability("Invalid batch size".to_string()));
        }
        
        Ok(())
    }

    fn should_queue_request(&self, service: &ServiceInfo) -> IpcResult<bool> {
        // Check if service is overloaded
        let stats = self.service_registry.get_service_stats(&service.id)?;
        Ok(stats.current_load > 0.8) // Queue if load > 80%
    }

    fn send_request_to_service(
        &mut self,
        request: EmbeddingRequest,
        service: ServiceInfo,
        context: &OperationContext,
    ) -> IpcResult<EmbeddingResponse> {
        // Create IPC message
        let message = IpcMessage::EmbeddingRequest {
            request_id: self.generate_request_id(),
            service_id: service.id.clone(),
            request,
            timestamp: self.get_current_timestamp(),
        };
        
        // Send via transport
        let response_message = self.transport.send_message(message, &service)?;
        
        // Extract response
        match response_message {
            IpcMessage::EmbeddingResponse { response, .. } => Ok(response),
            IpcMessage::Error { error, .. } => Err(IpcError::TransportError(error)),
            _ => Err(IpcError::InvalidMessage("Unexpected response type".to_string())),
        }
    }

    fn generate_request_id(&self) -> u64 {
        // Simple counter-based ID generation
        // In real implementation, would use atomic counter or UUID
        42 // Placeholder
    }

    fn get_current_timestamp(&self) -> u64 {
        // Get current timestamp in milliseconds
        // In kernel mode, would use kernel time functions
        0 // Placeholder
    }
}

/// IPC statistics
#[derive(Debug, Clone)]
pub struct IpcStats {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub average_response_time_ms: u64,
    pub active_services: usize,
    pub queued_requests: usize,
}

/// Trait for IPC transport implementations
pub trait IpcTransport: Send + Sync {
    /// Initialize the transport
    fn initialize(&mut self) -> IpcResult<()>;
    
    /// Shutdown the transport
    fn shutdown(&mut self) -> IpcResult<()>;
    
    /// Send a message to a service
    fn send_message(&mut self, message: IpcMessage, service: &ServiceInfo) -> IpcResult<IpcMessage>;
    
    /// Receive a message (for service implementations)
    fn receive_message(&mut self) -> IpcResult<Option<IpcMessage>>;
    
    /// Check if transport is connected
    fn is_connected(&self) -> bool;
}