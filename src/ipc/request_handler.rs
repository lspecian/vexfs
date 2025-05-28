//! Request Handler for VexFS IPC
//!
//! This module implements asynchronous request processing, request queuing,
//! and coordination between the kernel module and userspace services.

use crate::shared::errors::{VexfsError, VexfsResult};
use crate::ipc::{IpcError, IpcResult, IpcConfig};
use crate::ipc::protocol::*;
use crate::fs_core::OperationContext;

#[cfg(not(feature = "std"))]
use alloc::{vec::Vec, string::String, collections::BTreeMap, boxed::Box};
#[cfg(feature = "std")]
use std::{vec::Vec, string::String, collections::BTreeMap, boxed::Box};

/// Request handler for processing embedding requests
pub struct RequestHandler {
    /// Configuration
    config: IpcConfig,
    /// Active requests
    active_requests: BTreeMap<u64, ActiveRequest>,
    /// Request statistics
    stats: RequestStats,
    /// Request timeout tracking
    request_timeouts: BTreeMap<u64, u64>,
    /// Next request ID
    next_request_id: u64,
}

/// Active request information
#[derive(Debug, Clone)]
pub struct ActiveRequest {
    /// Request details
    pub request: EmbeddingRequest,
    /// Target service ID
    pub service_id: String,
    /// Request start time
    pub start_time: u64,
    /// Request timeout
    pub timeout: u64,
    /// Retry count
    pub retry_count: u32,
    /// Request priority
    pub priority: u8,
    /// Operation context
    pub context_info: RequestContextInfo,
}

/// Request context information
#[derive(Debug, Clone)]
pub struct RequestContextInfo {
    /// User ID
    pub user_id: u32,
    /// Process ID
    pub process_id: u32,
    /// Request source
    pub source: RequestSource,
    /// Security context
    pub security_context: String,
}

/// Request source enumeration
#[derive(Debug, Clone, PartialEq)]
pub enum RequestSource {
    /// IOCTL interface
    Ioctl,
    /// Direct API call
    DirectApi,
    /// Batch operation
    Batch,
    /// Internal system request
    Internal,
}

/// Request statistics
#[derive(Debug, Clone, Default)]
pub struct RequestStats {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub timeout_requests: u64,
    pub retry_requests: u64,
    pub average_processing_time_us: u64,
    pub peak_concurrent_requests: usize,
    pub current_active_requests: usize,
}

impl RequestHandler {
    /// Create a new request handler
    pub fn new(config: IpcConfig) -> Self {
        Self {
            config,
            active_requests: BTreeMap::new(),
            stats: RequestStats::default(),
            request_timeouts: BTreeMap::new(),
            next_request_id: 1,
        }
    }

    /// Submit an embedding request
    pub fn submit_request(
        &mut self,
        dimensions: u32,
        data: Vec<u8>,
        data_type: VectorDataType,
        model: Option<String>,
        priority: u8,
        context: &OperationContext,
    ) -> IpcResult<u64> {
        // Validate request
        self.validate_request(dimensions, &data, data_type.clone())?;

        // Check capacity
        if self.active_requests.len() >= self.config.max_concurrent_requests {
            return Err(IpcError::QueueFull);
        }

        // Generate request ID
        let request_id = self.generate_request_id();

        // Create embedding request
        let embedding_request = EmbeddingRequest {
            request_id,
            dimensions,
            data,
            data_type,
            model,
            parameters: BTreeMap::new(),
            priority,
            timeout_ms: self.config.request_timeout_ms,
        };

        // Create context info
        let context_info = RequestContextInfo {
            user_id: context.user.uid,
            process_id: 0, // Would extract from context
            source: RequestSource::Ioctl,
            security_context: format!("uid:{}", context.user.uid),
        };

        // Create active request
        let active_request = ActiveRequest {
            request: embedding_request,
            service_id: String::new(), // Will be set when service is selected
            start_time: self.get_current_timestamp(),
            timeout: self.get_current_timestamp() + self.config.request_timeout_ms / 1000,
            retry_count: 0,
            priority,
            context_info,
        };

        // Store active request
        self.active_requests.insert(request_id, active_request);
        self.request_timeouts.insert(request_id, self.get_current_timestamp() + self.config.request_timeout_ms / 1000);

        // Update statistics
        self.stats.total_requests += 1;
        self.stats.current_active_requests = self.active_requests.len();
        if self.active_requests.len() > self.stats.peak_concurrent_requests {
            self.stats.peak_concurrent_requests = self.active_requests.len();
        }

        Ok(request_id)
    }

    /// Submit a batch embedding request
    pub fn submit_batch_request(
        &mut self,
        requests: Vec<(u32, Vec<u8>, VectorDataType, Option<String>)>,
        priority: u8,
        context: &OperationContext,
    ) -> IpcResult<Vec<u64>> {
        if requests.is_empty() {
            return Err(IpcError::InvalidMessage("Empty batch request".to_string()));
        }

        if requests.len() > self.config.max_queue_size {
            return Err(IpcError::InvalidMessage("Batch too large".to_string()));
        }

        let mut request_ids = Vec::with_capacity(requests.len());

        // Submit each request in the batch
        for (dimensions, data, data_type, model) in requests {
            let request_id = self.submit_request(dimensions, data, data_type, model, priority, context)?;
            request_ids.push(request_id);
        }

        Ok(request_ids)
    }

    /// Assign service to a request
    pub fn assign_service(&mut self, request_id: u64, service_id: String) -> IpcResult<()> {
        let active_request = self.active_requests.get_mut(&request_id)
            .ok_or_else(|| IpcError::CorrelationFailed(request_id))?;

        active_request.service_id = service_id;
        Ok(())
    }

    /// Complete a request successfully
    pub fn complete_request(&mut self, request_id: u64, response: EmbeddingResponse) -> IpcResult<()> {
        let active_request = self.active_requests.remove(&request_id)
            .ok_or_else(|| IpcError::CorrelationFailed(request_id))?;

        self.request_timeouts.remove(&request_id);

        // Update statistics
        self.stats.successful_requests += 1;
        self.stats.current_active_requests = self.active_requests.len();

        // Calculate processing time
        let processing_time = self.get_current_timestamp() - active_request.start_time;
        self.update_average_processing_time(processing_time * 1_000_000); // Convert to microseconds

        Ok(())
    }

    /// Fail a request
    pub fn fail_request(&mut self, request_id: u64, error: String) -> IpcResult<()> {
        let active_request = self.active_requests.get_mut(&request_id)
            .ok_or_else(|| IpcError::CorrelationFailed(request_id))?;

        // Check if we should retry
        if active_request.retry_count < self.config.max_retry_attempts {
            active_request.retry_count += 1;
            self.stats.retry_requests += 1;
            
            // Reset service assignment for retry
            active_request.service_id.clear();
            
            return Ok(());
        }

        // Remove failed request
        self.active_requests.remove(&request_id);
        self.request_timeouts.remove(&request_id);

        // Update statistics
        self.stats.failed_requests += 1;
        self.stats.current_active_requests = self.active_requests.len();

        Ok(())
    }

    /// Get active request
    pub fn get_active_request(&self, request_id: u64) -> IpcResult<&ActiveRequest> {
        self.active_requests.get(&request_id)
            .ok_or_else(|| IpcError::CorrelationFailed(request_id))
    }

    /// Get all active requests
    pub fn get_active_requests(&self) -> Vec<&ActiveRequest> {
        self.active_requests.values().collect()
    }

    /// Get requests by priority
    pub fn get_requests_by_priority(&self, min_priority: u8) -> Vec<&ActiveRequest> {
        self.active_requests.values()
            .filter(|request| request.priority >= min_priority)
            .collect()
    }

    /// Get requests for a specific service
    pub fn get_requests_for_service(&self, service_id: &str) -> Vec<&ActiveRequest> {
        self.active_requests.values()
            .filter(|request| request.service_id == service_id)
            .collect()
    }

    /// Check for timed out requests
    pub fn check_timeouts(&mut self) -> Vec<u64> {
        let current_time = self.get_current_timestamp();
        
        let timed_out_requests: Vec<u64> = self.request_timeouts.iter()
            .filter(|(_, &timeout)| current_time > timeout)
            .map(|(&request_id, _)| request_id)
            .collect();

        // Remove timed out requests
        for &request_id in &timed_out_requests {
            self.active_requests.remove(&request_id);
            self.request_timeouts.remove(&request_id);
            self.stats.timeout_requests += 1;
        }

        self.stats.current_active_requests = self.active_requests.len();

        timed_out_requests
    }

    /// Get request statistics
    pub fn get_stats(&self) -> &RequestStats {
        &self.stats
    }

    /// Get total requests processed
    pub fn get_total_requests(&self) -> u64 {
        self.stats.total_requests
    }

    /// Get successful requests
    pub fn get_successful_requests(&self) -> u64 {
        self.stats.successful_requests
    }

    /// Get failed requests
    pub fn get_failed_requests(&self) -> u64 {
        self.stats.failed_requests
    }

    /// Perform maintenance operations
    pub fn perform_maintenance(&mut self) -> IpcResult<RequestMaintenanceResult> {
        let mut result = RequestMaintenanceResult::default();

        // Check for timeouts
        let timed_out = self.check_timeouts();
        result.timed_out_requests = timed_out.len();

        // Clean up old statistics (if needed)
        // This could involve resetting counters, archiving old data, etc.

        Ok(result)
    }

    // Private helper methods

    fn validate_request(&self, dimensions: u32, data: &[u8], data_type: VectorDataType) -> IpcResult<()> {
        if dimensions == 0 || dimensions > crate::ipc::protocol::MAX_EMBEDDING_DIMENSIONS {
            return Err(IpcError::InvalidMessage("Invalid dimensions".to_string()));
        }

        if data.is_empty() {
            return Err(IpcError::InvalidMessage("Empty data".to_string()));
        }

        // Validate data size based on type and dimensions
        let expected_min_size = match data_type {
            VectorDataType::Float32 => dimensions as usize * 4,
            VectorDataType::Float16 => dimensions as usize * 2,
            VectorDataType::Int8 => dimensions as usize,
            VectorDataType::Int16 => dimensions as usize * 2,
            VectorDataType::Binary => (dimensions as usize + 7) / 8,
        };

        if data.len() < expected_min_size {
            return Err(IpcError::InvalidMessage("Data size too small".to_string()));
        }

        Ok(())
    }

    fn generate_request_id(&mut self) -> u64 {
        let id = self.next_request_id;
        self.next_request_id = self.next_request_id.wrapping_add(1);
        id
    }

    fn update_average_processing_time(&mut self, processing_time_us: u64) {
        if self.stats.average_processing_time_us == 0 {
            self.stats.average_processing_time_us = processing_time_us;
        } else {
            // Exponential moving average
            self.stats.average_processing_time_us = 
                (self.stats.average_processing_time_us * 9 + processing_time_us) / 10;
        }
    }

    fn get_current_timestamp(&self) -> u64 {
        // Get current timestamp in seconds
        0 // Placeholder
    }
}

/// Request maintenance result
#[derive(Debug, Clone, Default)]
pub struct RequestMaintenanceResult {
    pub timed_out_requests: usize,
}

/// Request priority levels
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum RequestPriority {
    Low = 0,
    Normal = 128,
    High = 192,
    Critical = 255,
}

impl From<u8> for RequestPriority {
    fn from(value: u8) -> Self {
        match value {
            0..=63 => RequestPriority::Low,
            64..=191 => RequestPriority::Normal,
            192..=254 => RequestPriority::High,
            255 => RequestPriority::Critical,
        }
    }
}

impl From<RequestPriority> for u8 {
    fn from(priority: RequestPriority) -> Self {
        priority as u8
    }
}

/// Request validator for validating embedding requests
pub struct RequestValidator;

impl RequestValidator {
    /// Validate embedding request
    pub fn validate_embedding_request(request: &EmbeddingRequest) -> IpcResult<()> {
        // Validate dimensions
        if request.dimensions == 0 || request.dimensions > MAX_EMBEDDING_DIMENSIONS {
            return Err(IpcError::InvalidMessage("Invalid dimensions".to_string()));
        }

        // Validate data
        if request.data.is_empty() {
            return Err(IpcError::InvalidMessage("Empty data".to_string()));
        }

        // Validate timeout
        if request.timeout_ms == 0 {
            return Err(IpcError::InvalidMessage("Invalid timeout".to_string()));
        }

        // Validate priority
        if request.priority > 255 {
            return Err(IpcError::InvalidMessage("Invalid priority".to_string()));
        }

        Ok(())
    }

    /// Validate batch request
    pub fn validate_batch_request(requests: &[EmbeddingRequest]) -> IpcResult<()> {
        if requests.is_empty() {
            return Err(IpcError::InvalidMessage("Empty batch".to_string()));
        }

        if requests.len() > MAX_BATCH_SIZE {
            return Err(IpcError::InvalidMessage("Batch too large".to_string()));
        }

        // Validate each request in the batch
        for request in requests {
            Self::validate_embedding_request(request)?;
        }

        Ok(())
    }

    /// Validate request context
    pub fn validate_request_context(context: &RequestContextInfo) -> IpcResult<()> {
        // Validate user ID (0 is root, which is valid)
        // No specific validation needed for user_id

        // Validate security context
        if context.security_context.is_empty() {
            return Err(IpcError::AuthenticationFailed("Empty security context".to_string()));
        }

        Ok(())
    }
}

/// Request metrics collector
pub struct RequestMetrics {
    /// Request latency histogram
    latency_histogram: BTreeMap<u64, u64>,
    /// Request size histogram
    size_histogram: BTreeMap<usize, u64>,
    /// Error counts by type
    error_counts: BTreeMap<String, u64>,
    /// Success rate over time
    success_rate_history: Vec<(u64, f64)>,
}

impl RequestMetrics {
    /// Create new request metrics collector
    pub fn new() -> Self {
        Self {
            latency_histogram: BTreeMap::new(),
            size_histogram: BTreeMap::new(),
            error_counts: BTreeMap::new(),
            success_rate_history: Vec::new(),
        }
    }

    /// Record request latency
    pub fn record_latency(&mut self, latency_us: u64) {
        // Round to nearest millisecond for histogram
        let latency_ms = (latency_us + 500) / 1000;
        *self.latency_histogram.entry(latency_ms).or_insert(0) += 1;
    }

    /// Record request size
    pub fn record_size(&mut self, size_bytes: usize) {
        // Round to nearest KB for histogram
        let size_kb = (size_bytes + 512) / 1024;
        *self.size_histogram.entry(size_kb).or_insert(0) += 1;
    }

    /// Record error
    pub fn record_error(&mut self, error_type: &str) {
        *self.error_counts.entry(error_type.to_string()).or_insert(0) += 1;
    }

    /// Record success rate
    pub fn record_success_rate(&mut self, timestamp: u64, success_rate: f64) {
        self.success_rate_history.push((timestamp, success_rate));
        
        // Keep only recent history (last 1000 entries)
        if self.success_rate_history.len() > 1000 {
            self.success_rate_history.remove(0);
        }
    }

    /// Get latency percentile
    pub fn get_latency_percentile(&self, percentile: f64) -> Option<u64> {
        if self.latency_histogram.is_empty() {
            return None;
        }

        let total_requests: u64 = self.latency_histogram.values().sum();
        let target_count = (total_requests as f64 * percentile / 100.0) as u64;
        
        let mut cumulative_count = 0;
        for (&latency, &count) in &self.latency_histogram {
            cumulative_count += count;
            if cumulative_count >= target_count {
                return Some(latency);
            }
        }

        None
    }

    /// Get most common error
    pub fn get_most_common_error(&self) -> Option<(&String, u64)> {
        self.error_counts.iter()
            .max_by_key(|(_, &count)| count)
            .map(|(error, &count)| (error, count))
    }

    /// Get current success rate
    pub fn get_current_success_rate(&self) -> Option<f64> {
        self.success_rate_history.last()
            .map(|(_, rate)| *rate)
    }
}