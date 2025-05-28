//! Queue Manager for VexFS IPC
//!
//! This module implements request queuing, prioritization, and flow control
//! for embedding service requests.

use crate::shared::errors::{VexfsError, VexfsResult};
use crate::ipc::{IpcError, IpcResult, IpcConfig};
use crate::ipc::protocol::*;

#[cfg(not(feature = "std"))]
use alloc::{vec::Vec, string::String, collections::BTreeMap, boxed::Box};
#[cfg(feature = "std")]
use std::{vec::Vec, string::String, collections::BTreeMap, boxed::Box};

/// Queue manager for handling request queuing and prioritization
pub struct QueueManager {
    /// Configuration
    config: IpcConfig,
    /// Priority queues
    priority_queues: BTreeMap<u8, RequestQueue>,
    /// Service-specific queues
    service_queues: BTreeMap<String, RequestQueue>,
    /// Global queue for overflow
    global_queue: RequestQueue,
    /// Queue statistics
    stats: QueueStats,
    /// Flow control state
    flow_control: FlowControlState,
}

/// Request queue implementation
#[derive(Debug, Clone)]
pub struct RequestQueue {
    /// Queue name
    pub name: String,
    /// Queued requests
    pub requests: Vec<QueuedRequest>,
    /// Maximum queue size
    pub max_size: usize,
    /// Queue creation time
    pub created_at: u64,
    /// Queue statistics
    pub stats: QueueStats,
}

/// Queued request information
#[derive(Debug, Clone)]
pub struct QueuedRequest {
    /// Request ID
    pub request_id: u64,
    /// Embedding request
    pub request: EmbeddingRequest,
    /// Target service ID
    pub service_id: String,
    /// Queue timestamp
    pub queued_at: u64,
    /// Priority
    pub priority: u8,
    /// Retry count
    pub retry_count: u32,
    /// Request metadata
    pub metadata: BTreeMap<String, String>,
}

/// Queue statistics
#[derive(Debug, Clone, Default)]
pub struct QueueStats {
    pub total_enqueued: u64,
    pub total_dequeued: u64,
    pub total_dropped: u64,
    pub current_size: usize,
    pub peak_size: usize,
    pub average_wait_time_ms: u64,
    pub total_wait_time_ms: u64,
}

/// Flow control state
#[derive(Debug, Clone)]
pub struct FlowControlState {
    /// Flow control enabled
    pub enabled: bool,
    /// Current flow control level (0.0-1.0)
    pub level: f32,
    /// Backpressure threshold
    pub backpressure_threshold: f32,
    /// Drop threshold
    pub drop_threshold: f32,
    /// Current backpressure
    pub current_backpressure: f32,
}

impl Default for FlowControlState {
    fn default() -> Self {
        Self {
            enabled: true,
            level: 0.0,
            backpressure_threshold: 0.7,
            drop_threshold: 0.9,
            current_backpressure: 0.0,
        }
    }
}

impl QueueManager {
    /// Create a new queue manager
    pub fn new(config: IpcConfig) -> Self {
        let mut priority_queues = BTreeMap::new();
        
        // Create priority queues (0-255)
        for priority in [0, 64, 128, 192, 255] {
            let queue = RequestQueue::new(format!("priority_{}", priority), config.max_queue_size);
            priority_queues.insert(priority, queue);
        }

        let global_queue = RequestQueue::new("global".to_string(), config.max_queue_size * 2);

        Self {
            config,
            priority_queues,
            service_queues: BTreeMap::new(),
            global_queue,
            stats: QueueStats::default(),
            flow_control: FlowControlState::default(),
        }
    }

    /// Queue a request
    pub fn queue_request(&mut self, request: EmbeddingRequest, service: ServiceInfo) -> IpcResult<EmbeddingResponse> {
        // Check flow control
        if self.should_drop_request(&request)? {
            self.stats.total_dropped += 1;
            return Err(IpcError::QueueFull);
        }

        // Create queued request
        let queued_request = QueuedRequest {
            request_id: request.request_id,
            request: request.clone(),
            service_id: service.id.clone(),
            queued_at: self.get_current_timestamp(),
            priority: request.priority,
            retry_count: 0,
            metadata: BTreeMap::new(),
        };

        // Select appropriate queue
        let queue_result = if self.config.max_queue_size > 0 {
            // Try service-specific queue first
            if let Some(service_queue) = self.service_queues.get_mut(&service.id) {
                service_queue.enqueue(queued_request.clone())
            } else {
                // Create service queue if it doesn't exist
                let mut service_queue = RequestQueue::new(
                    format!("service_{}", service.id),
                    self.config.max_queue_size / 4, // Smaller per-service queues
                );
                let result = service_queue.enqueue(queued_request.clone());
                self.service_queues.insert(service.id.clone(), service_queue);
                result
            }
        } else {
            Err(IpcError::QueueFull)
        };

        // If service queue is full, try priority queue
        let queue_result = queue_result.or_else(|_| {
            let priority_level = self.map_priority_to_level(request.priority);
            if let Some(priority_queue) = self.priority_queues.get_mut(&priority_level) {
                priority_queue.enqueue(queued_request.clone())
            } else {
                Err(IpcError::QueueFull)
            }
        });

        // If priority queue is full, try global queue
        let queue_result = queue_result.or_else(|_| {
            self.global_queue.enqueue(queued_request)
        });

        match queue_result {
            Ok(()) => {
                self.stats.total_enqueued += 1;
                self.update_flow_control();
                
                // For now, return a placeholder response indicating queued
                // In a real implementation, this would be handled asynchronously
                Ok(EmbeddingResponse {
                    request_id: request.request_id,
                    status: ResponseStatus::Success, // Would be a "Queued" status
                    embedding: None,
                    error: Some("Request queued".to_string()),
                    processing_time_us: 0,
                    model_used: None,
                    metadata: BTreeMap::new(),
                })
            }
            Err(e) => {
                self.stats.total_dropped += 1;
                Err(e)
            }
        }
    }

    /// Dequeue next request
    pub fn dequeue_next_request(&mut self) -> Option<QueuedRequest> {
        // Try priority queues first (highest priority first)
        for priority in [255, 192, 128, 64, 0] {
            if let Some(priority_queue) = self.priority_queues.get_mut(&priority) {
                if let Some(request) = priority_queue.dequeue() {
                    self.stats.total_dequeued += 1;
                    self.update_flow_control();
                    return Some(request);
                }
            }
        }

        // Try service queues (round-robin)
        let service_ids: Vec<String> = self.service_queues.keys().cloned().collect();
        for service_id in service_ids {
            if let Some(service_queue) = self.service_queues.get_mut(&service_id) {
                if let Some(request) = service_queue.dequeue() {
                    self.stats.total_dequeued += 1;
                    self.update_flow_control();
                    return Some(request);
                }
            }
        }

        // Try global queue
        if let Some(request) = self.global_queue.dequeue() {
            self.stats.total_dequeued += 1;
            self.update_flow_control();
            return Some(request);
        }

        None
    }

    /// Dequeue request for specific service
    pub fn dequeue_request_for_service(&mut self, service_id: &str) -> Option<QueuedRequest> {
        // Try service-specific queue first
        if let Some(service_queue) = self.service_queues.get_mut(service_id) {
            if let Some(request) = service_queue.dequeue() {
                self.stats.total_dequeued += 1;
                self.update_flow_control();
                return Some(request);
            }
        }

        // Try priority queues for requests targeting this service
        for priority in [255, 192, 128, 64, 0] {
            if let Some(priority_queue) = self.priority_queues.get_mut(&priority) {
                if let Some(request) = priority_queue.dequeue_for_service(service_id) {
                    self.stats.total_dequeued += 1;
                    self.update_flow_control();
                    return Some(request);
                }
            }
        }

        // Try global queue
        if let Some(request) = self.global_queue.dequeue_for_service(service_id) {
            self.stats.total_dequeued += 1;
            self.update_flow_control();
            return Some(request);
        }

        None
    }

    /// Get queue size
    pub fn get_queue_size(&self) -> usize {
        let mut total_size = self.global_queue.requests.len();
        
        for queue in self.priority_queues.values() {
            total_size += queue.requests.len();
        }
        
        for queue in self.service_queues.values() {
            total_size += queue.requests.len();
        }
        
        total_size
    }

    /// Get queue statistics
    pub fn get_stats(&self) -> &QueueStats {
        &self.stats
    }

    /// Get flow control state
    pub fn get_flow_control_state(&self) -> &FlowControlState {
        &self.flow_control
    }

    /// Clear all queues
    pub fn clear_all_queues(&mut self) -> usize {
        let mut cleared_count = 0;
        
        cleared_count += self.global_queue.clear();
        
        for queue in self.priority_queues.values_mut() {
            cleared_count += queue.clear();
        }
        
        for queue in self.service_queues.values_mut() {
            cleared_count += queue.clear();
        }
        
        self.stats.total_dropped += cleared_count as u64;
        self.update_flow_control();
        
        cleared_count
    }

    /// Clear queue for specific service
    pub fn clear_service_queue(&mut self, service_id: &str) -> usize {
        if let Some(service_queue) = self.service_queues.get_mut(service_id) {
            let cleared = service_queue.clear();
            self.stats.total_dropped += cleared as u64;
            self.update_flow_control();
            cleared
        } else {
            0
        }
    }

    /// Perform maintenance operations
    pub fn perform_maintenance(&mut self) -> IpcResult<QueueMaintenanceResult> {
        let mut result = QueueMaintenanceResult::default();

        // Clean up expired requests
        let current_time = self.get_current_timestamp();
        let timeout_threshold = current_time - (self.config.request_timeout_ms / 1000);

        result.expired_requests += self.global_queue.remove_expired(timeout_threshold);
        
        for queue in self.priority_queues.values_mut() {
            result.expired_requests += queue.remove_expired(timeout_threshold);
        }
        
        for queue in self.service_queues.values_mut() {
            result.expired_requests += queue.remove_expired(timeout_threshold);
        }

        // Remove empty service queues
        let empty_services: Vec<String> = self.service_queues.iter()
            .filter(|(_, queue)| queue.requests.is_empty())
            .map(|(id, _)| id.clone())
            .collect();

        for service_id in empty_services {
            self.service_queues.remove(&service_id);
            result.removed_empty_queues += 1;
        }

        // Update flow control
        self.update_flow_control();

        Ok(result)
    }

    // Private helper methods

    fn should_drop_request(&self, request: &EmbeddingRequest) -> IpcResult<bool> {
        if !self.flow_control.enabled {
            return Ok(false);
        }

        // Check if we're above drop threshold
        if self.flow_control.current_backpressure > self.flow_control.drop_threshold {
            // Drop low priority requests
            if request.priority < 128 {
                return Ok(true);
            }
        }

        // Check total queue size
        let total_size = self.get_queue_size();
        if total_size >= self.config.max_queue_size {
            return Ok(true);
        }

        Ok(false)
    }

    fn map_priority_to_level(&self, priority: u8) -> u8 {
        match priority {
            0..=63 => 0,
            64..=127 => 64,
            128..=191 => 128,
            192..=254 => 192,
            255 => 255,
        }
    }

    fn update_flow_control(&mut self) {
        let total_size = self.get_queue_size();
        let max_size = self.config.max_queue_size;
        
        if max_size > 0 {
            self.flow_control.current_backpressure = total_size as f32 / max_size as f32;
            self.flow_control.level = self.flow_control.current_backpressure;
        } else {
            self.flow_control.current_backpressure = 0.0;
            self.flow_control.level = 0.0;
        }
    }

    fn get_current_timestamp(&self) -> u64 {
        // Get current timestamp in seconds
        0 // Placeholder
    }
}

impl RequestQueue {
    /// Create a new request queue
    pub fn new(name: String, max_size: usize) -> Self {
        Self {
            name,
            requests: Vec::new(),
            max_size,
            created_at: 0, // Would use current timestamp
            stats: QueueStats::default(),
        }
    }

    /// Enqueue a request
    pub fn enqueue(&mut self, request: QueuedRequest) -> IpcResult<()> {
        if self.requests.len() >= self.max_size {
            return Err(IpcError::QueueFull);
        }

        self.requests.push(request);
        self.stats.total_enqueued += 1;
        self.stats.current_size = self.requests.len();
        
        if self.requests.len() > self.stats.peak_size {
            self.stats.peak_size = self.requests.len();
        }

        Ok(())
    }

    /// Dequeue next request (FIFO)
    pub fn dequeue(&mut self) -> Option<QueuedRequest> {
        if self.requests.is_empty() {
            return None;
        }

        let request = self.requests.remove(0);
        self.stats.total_dequeued += 1;
        self.stats.current_size = self.requests.len();
        
        // Update wait time statistics
        let wait_time = 0; // Would calculate actual wait time
        self.stats.total_wait_time_ms += wait_time;
        if self.stats.total_dequeued > 0 {
            self.stats.average_wait_time_ms = self.stats.total_wait_time_ms / self.stats.total_dequeued;
        }

        Some(request)
    }

    /// Dequeue request for specific service
    pub fn dequeue_for_service(&mut self, service_id: &str) -> Option<QueuedRequest> {
        for i in 0..self.requests.len() {
            if self.requests[i].service_id == service_id {
                let request = self.requests.remove(i);
                self.stats.total_dequeued += 1;
                self.stats.current_size = self.requests.len();
                return Some(request);
            }
        }
        None
    }

    /// Clear all requests
    pub fn clear(&mut self) -> usize {
        let count = self.requests.len();
        self.requests.clear();
        self.stats.total_dropped += count as u64;
        self.stats.current_size = 0;
        count
    }

    /// Remove expired requests
    pub fn remove_expired(&mut self, timeout_threshold: u64) -> usize {
        let original_len = self.requests.len();
        
        self.requests.retain(|request| request.queued_at > timeout_threshold);
        
        let removed_count = original_len - self.requests.len();
        self.stats.total_dropped += removed_count as u64;
        self.stats.current_size = self.requests.len();
        
        removed_count
    }

    /// Get queue size
    pub fn size(&self) -> usize {
        self.requests.len()
    }

    /// Check if queue is empty
    pub fn is_empty(&self) -> bool {
        self.requests.is_empty()
    }

    /// Check if queue is full
    pub fn is_full(&self) -> bool {
        self.requests.len() >= self.max_size
    }
}

/// Queue maintenance result
#[derive(Debug, Clone, Default)]
pub struct QueueMaintenanceResult {
    pub expired_requests: usize,
    pub removed_empty_queues: usize,
}

/// Queue monitor for tracking queue health and performance
pub struct QueueMonitor {
    /// Monitor configuration
    config: MonitorConfig,
    /// Queue metrics history
    metrics_history: Vec<QueueMetrics>,
    /// Alert thresholds
    alert_thresholds: AlertThresholds,
}

/// Monitor configuration
#[derive(Debug, Clone)]
pub struct MonitorConfig {
    /// Monitoring interval in seconds
    pub interval_sec: u64,
    /// History retention period in seconds
    pub retention_sec: u64,
    /// Enable alerting
    pub enable_alerts: bool,
}

impl Default for MonitorConfig {
    fn default() -> Self {
        Self {
            interval_sec: 10,
            retention_sec: 3600, // 1 hour
            enable_alerts: true,
        }
    }
}

/// Queue metrics snapshot
#[derive(Debug, Clone)]
pub struct QueueMetrics {
    /// Timestamp
    pub timestamp: u64,
    /// Total queue size
    pub total_size: usize,
    /// Queue utilization (0.0-1.0)
    pub utilization: f32,
    /// Enqueue rate (requests per second)
    pub enqueue_rate: f64,
    /// Dequeue rate (requests per second)
    pub dequeue_rate: f64,
    /// Average wait time
    pub avg_wait_time_ms: u64,
    /// Flow control level
    pub flow_control_level: f32,
}

/// Alert thresholds
#[derive(Debug, Clone)]
pub struct AlertThresholds {
    /// High utilization threshold
    pub high_utilization: f32,
    /// Critical utilization threshold
    pub critical_utilization: f32,
    /// High wait time threshold (ms)
    pub high_wait_time_ms: u64,
    /// Critical wait time threshold (ms)
    pub critical_wait_time_ms: u64,
}

impl Default for AlertThresholds {
    fn default() -> Self {
        Self {
            high_utilization: 0.7,
            critical_utilization: 0.9,
            high_wait_time_ms: 1000,
            critical_wait_time_ms: 5000,
        }
    }
}

impl QueueMonitor {
    /// Create a new queue monitor
    pub fn new(config: MonitorConfig) -> Self {
        Self {
            config,
            metrics_history: Vec::new(),
            alert_thresholds: AlertThresholds::default(),
        }
    }

    /// Record queue metrics
    pub fn record_metrics(&mut self, queue_manager: &QueueManager) {
        let current_time = self.get_current_timestamp();
        let stats = queue_manager.get_stats();
        let flow_control = queue_manager.get_flow_control_state();
        
        let metrics = QueueMetrics {
            timestamp: current_time,
            total_size: queue_manager.get_queue_size(),
            utilization: flow_control.current_backpressure,
            enqueue_rate: 0.0, // Would calculate from recent history
            dequeue_rate: 0.0, // Would calculate from recent history
            avg_wait_time_ms: stats.average_wait_time_ms,
            flow_control_level: flow_control.level,
        };

        self.metrics_history.push(metrics);

        // Clean up old metrics
        let retention_threshold = current_time - self.config.retention_sec;
        self.metrics_history.retain(|m| m.timestamp > retention_threshold);

        // Check for alerts
        if self.config.enable_alerts {
            self.check_alerts(&self.metrics_history.last().unwrap());
        }
    }

    /// Check for alert conditions
    fn check_alerts(&self, metrics: &QueueMetrics) {
        // Check utilization alerts
        if metrics.utilization >= self.alert_thresholds.critical_utilization {
            // Would trigger critical alert
        } else if metrics.utilization >= self.alert_thresholds.high_utilization {
            // Would trigger warning alert
        }

        // Check wait time alerts
        if metrics.avg_wait_time_ms >= self.alert_thresholds.critical_wait_time_ms {
            // Would trigger critical alert
        } else if metrics.avg_wait_time_ms >= self.alert_thresholds.high_wait_time_ms {
            // Would trigger warning alert
        }
    }

    /// Get recent metrics
    pub fn get_recent_metrics(&self, duration_sec: u64) -> Vec<&QueueMetrics> {
        let current_time = self.get_current_timestamp();
        let threshold = current_time - duration_sec;
        
        self.metrics_history.iter()
            .filter(|m| m.timestamp > threshold)
            .collect()
    }

    /// Get average utilization over period
    pub fn get_average_utilization(&self, duration_sec: u64) -> f32 {
        let recent_metrics = self.get_recent_metrics(duration_sec);
        
        if recent_metrics.is_empty() {
            return 0.0;
        }

        let total_utilization: f32 = recent_metrics.iter()
            .map(|m| m.utilization)
            .sum();
        
        total_utilization / recent_metrics.len() as f32
    }

    fn get_current_timestamp(&self) -> u64 {
        // Get current timestamp in seconds
        0 // Placeholder
    }
}