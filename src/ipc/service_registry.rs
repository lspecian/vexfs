//! Service Registry for VexFS IPC
//!
//! This module implements service discovery and registration functionality
//! for embedding services, providing a centralized registry for service
//! management and capability advertisement.

use crate::shared::errors::{VexfsError, VexfsResult};
use crate::ipc::{IpcError, IpcResult};
use crate::ipc::protocol::*;

#[cfg(not(feature = "std"))]
use alloc::{vec::Vec, string::String, collections::BTreeMap, boxed::Box};
#[cfg(feature = "std")]
use std::{vec::Vec, string::String, collections::BTreeMap, boxed::Box};

/// Service registry for managing embedding services
pub struct ServiceRegistry {
    /// Registered services
    services: BTreeMap<String, RegisteredService>,
    /// Service statistics
    stats: BTreeMap<String, ServiceStats>,
    /// Registry configuration
    config: RegistryConfig,
}

/// Configuration for service registry
#[derive(Debug, Clone)]
pub struct RegistryConfig {
    /// Maximum number of services
    pub max_services: usize,
    /// Service timeout in seconds
    pub service_timeout_sec: u64,
    /// Health check interval in seconds
    pub health_check_interval_sec: u64,
    /// Statistics retention period in seconds
    pub stats_retention_sec: u64,
    /// Enable service versioning
    pub enable_versioning: bool,
    /// Enable capability filtering
    pub enable_capability_filtering: bool,
}

impl Default for RegistryConfig {
    fn default() -> Self {
        Self {
            max_services: 100,
            service_timeout_sec: 300, // 5 minutes
            health_check_interval_sec: 30,
            stats_retention_sec: 3600, // 1 hour
            enable_versioning: true,
            enable_capability_filtering: true,
        }
    }
}

/// Registered service information
#[derive(Debug, Clone)]
pub struct RegisteredService {
    /// Service information
    pub info: ServiceInfo,
    /// Registration timestamp
    pub registered_at: u64,
    /// Last heartbeat timestamp
    pub last_heartbeat: u64,
    /// Current status
    pub status: ServiceState,
    /// Current health
    pub health: ServiceHealth,
    /// Load information
    pub load_info: ServiceLoadInfo,
    /// Service tags for filtering
    pub tags: Vec<String>,
    /// Service priority (0-255, higher is better)
    pub priority: u8,
}

/// Service statistics
#[derive(Debug, Clone)]
pub struct ServiceStats {
    /// Total requests processed
    pub total_requests: u64,
    /// Successful requests
    pub successful_requests: u64,
    /// Failed requests
    pub failed_requests: u64,
    /// Average response time in microseconds
    pub avg_response_time_us: u64,
    /// Current load (0.0-1.0)
    pub current_load: f32,
    /// Peak load in the last hour
    pub peak_load: f32,
    /// Uptime in seconds
    pub uptime_seconds: u64,
    /// Last update timestamp
    pub last_updated: u64,
    /// Request rate (requests per second)
    pub request_rate: f64,
    /// Error rate (0.0-1.0)
    pub error_rate: f64,
}

impl Default for ServiceStats {
    fn default() -> Self {
        Self {
            total_requests: 0,
            successful_requests: 0,
            failed_requests: 0,
            avg_response_time_us: 0,
            current_load: 0.0,
            peak_load: 0.0,
            uptime_seconds: 0,
            last_updated: 0,
            request_rate: 0.0,
            error_rate: 0.0,
        }
    }
}

impl ServiceRegistry {
    /// Create a new service registry
    pub fn new() -> Self {
        Self::with_config(RegistryConfig::default())
    }

    /// Create a new service registry with configuration
    pub fn with_config(config: RegistryConfig) -> Self {
        Self {
            services: BTreeMap::new(),
            stats: BTreeMap::new(),
            config,
        }
    }

    /// Register a new service
    pub fn register_service(&mut self, service_info: ServiceInfo) -> IpcResult<()> {
        // Check if registry is full
        if self.services.len() >= self.config.max_services {
            return Err(IpcError::RegistrationFailed("Registry full".to_string()));
        }

        // Validate service information
        self.validate_service_info(&service_info)?;

        // Check for duplicate service ID
        if self.services.contains_key(&service_info.id) {
            return Err(IpcError::RegistrationFailed("Service already registered".to_string()));
        }

        // Create registered service entry
        let registered_service = RegisteredService {
            info: service_info.clone(),
            registered_at: self.get_current_timestamp(),
            last_heartbeat: self.get_current_timestamp(),
            status: ServiceState::Starting,
            health: ServiceHealth {
                status: HealthStatus::Unknown,
                score: 0,
                last_check: 0,
                details: BTreeMap::new(),
            },
            load_info: ServiceLoadInfo {
                cpu_usage: 0.0,
                memory_usage: 0.0,
                active_requests: 0,
                queue_depth: 0,
                avg_response_time_ms: 0,
            },
            tags: Vec::new(),
            priority: 128, // Default priority
        };

        // Initialize service statistics
        let stats = ServiceStats::default();

        // Store service and statistics
        self.services.insert(service_info.id.clone(), registered_service);
        self.stats.insert(service_info.id, stats);

        Ok(())
    }

    /// Unregister a service
    pub fn unregister_service(&mut self, service_id: &str) -> IpcResult<()> {
        if !self.services.contains_key(service_id) {
            return Err(IpcError::ServiceNotFound(service_id.to_string()));
        }

        // Remove service and statistics
        self.services.remove(service_id);
        self.stats.remove(service_id);

        Ok(())
    }

    /// Update service heartbeat
    pub fn update_heartbeat(&mut self, service_id: &str, load_info: ServiceLoadInfo) -> IpcResult<()> {
        // Check if service exists first
        if !self.services.contains_key(service_id) {
            return Err(IpcError::ServiceNotFound(service_id.to_string()));
        }

        // Get current timestamp and calculate values before borrowing mutably
        let current_timestamp = self.get_current_timestamp();
        let registered_at = self.services[service_id].registered_at;
        let new_status = self.determine_service_status(&load_info);
        let current_load = load_info.cpu_usage.max(load_info.memory_usage);

        // Update service
        let service = self.services.get_mut(service_id).unwrap(); // Safe because we checked above
        service.last_heartbeat = current_timestamp;
        service.load_info = load_info;
        service.status = new_status;

        // Update statistics
        if let Some(stats) = self.stats.get_mut(service_id) {
            stats.current_load = current_load;
            stats.peak_load = stats.peak_load.max(stats.current_load);
            stats.last_updated = current_timestamp;
            stats.uptime_seconds = current_timestamp - registered_at;
        }

        Ok(())
    }

    /// Update service health
    pub fn update_health(&mut self, service_id: &str, health: ServiceHealth) -> IpcResult<()> {
        let service = self.services.get_mut(service_id)
            .ok_or_else(|| IpcError::ServiceNotFound(service_id.to_string()))?;

        service.health = health;
        Ok(())
    }

    /// Get service by ID
    pub fn get_service(&self, service_id: &str) -> IpcResult<&RegisteredService> {
        self.services.get(service_id)
            .ok_or_else(|| IpcError::ServiceNotFound(service_id.to_string()))
    }

    /// Get service statistics
    pub fn get_service_stats(&self, service_id: &str) -> IpcResult<ServiceStats> {
        self.stats.get(service_id)
            .cloned()
            .ok_or_else(|| IpcError::ServiceNotFound(service_id.to_string()))
    }

    /// Find services by capabilities
    pub fn find_services_by_capabilities(&self, required_capabilities: &ServiceCapabilities) -> Vec<&RegisteredService> {
        self.services.values()
            .filter(|service| self.matches_capabilities(&service.info.capabilities, required_capabilities))
            .filter(|service| self.is_service_available(service))
            .collect()
    }

    /// Find services by dimensions
    pub fn find_services_by_dimensions(&self, dimensions: u32) -> Vec<&RegisteredService> {
        self.services.values()
            .filter(|service| service.info.capabilities.supported_dimensions.contains(&dimensions))
            .filter(|service| self.is_service_available(service))
            .collect()
    }

    /// Find services by model
    pub fn find_services_by_model(&self, model: &str) -> Vec<&RegisteredService> {
        self.services.values()
            .filter(|service| service.info.capabilities.supported_models.iter().any(|m| m == model))
            .filter(|service| self.is_service_available(service))
            .collect()
    }

    /// Get all active services
    pub fn get_active_services(&self) -> Vec<&RegisteredService> {
        self.services.values()
            .filter(|service| self.is_service_available(service))
            .collect()
    }

    /// Get services sorted by load (ascending)
    pub fn get_services_by_load(&self) -> Vec<&RegisteredService> {
        let mut services: Vec<&RegisteredService> = self.services.values()
            .filter(|service| self.is_service_available(service))
            .collect();

        services.sort_by(|a, b| {
            let load_a = a.load_info.cpu_usage.max(a.load_info.memory_usage);
            let load_b = b.load_info.cpu_usage.max(b.load_info.memory_usage);
            load_a.partial_cmp(&load_b).unwrap_or(core::cmp::Ordering::Equal)
        });

        services
    }

    /// Get services sorted by priority (descending)
    pub fn get_services_by_priority(&self) -> Vec<&RegisteredService> {
        let mut services: Vec<&RegisteredService> = self.services.values()
            .filter(|service| self.is_service_available(service))
            .collect();

        services.sort_by(|a, b| b.priority.cmp(&a.priority));
        services
    }

    /// Update service statistics
    pub fn update_service_stats(
        &mut self,
        service_id: &str,
        response_time_us: u64,
        success: bool,
    ) -> IpcResult<()> {
        // Get current timestamp once to avoid borrowing conflicts
        let current_timestamp = self.get_current_timestamp();
        
        let stats = self.stats.get_mut(service_id)
            .ok_or_else(|| IpcError::ServiceNotFound(service_id.to_string()))?;

        // Update request counts
        stats.total_requests += 1;
        if success {
            stats.successful_requests += 1;
        } else {
            stats.failed_requests += 1;
        }

        // Update average response time (exponential moving average)
        if stats.avg_response_time_us == 0 {
            stats.avg_response_time_us = response_time_us;
        } else {
            // Use exponential moving average with alpha = 0.1
            stats.avg_response_time_us = (stats.avg_response_time_us * 9 + response_time_us) / 10;
        }

        // Update error rate
        stats.error_rate = stats.failed_requests as f64 / stats.total_requests as f64;

        // Update timestamp
        stats.last_updated = current_timestamp;

        Ok(())
    }

    /// Clean up expired services
    pub fn cleanup_expired_services(&mut self) -> Vec<String> {
        let current_time = self.get_current_timestamp();
        let timeout = self.config.service_timeout_sec;
        
        let expired_services: Vec<String> = self.services.iter()
            .filter(|(_, service)| {
                current_time - service.last_heartbeat > timeout
            })
            .map(|(id, _)| id.clone())
            .collect();

        // Remove expired services
        for service_id in &expired_services {
            self.services.remove(service_id);
            self.stats.remove(service_id);
        }

        expired_services
    }

    /// Get registry statistics
    pub fn get_registry_stats(&self) -> RegistryStats {
        let active_services = self.get_active_services().len();
        let total_services = self.services.len();
        
        let total_requests: u64 = self.stats.values().map(|s| s.total_requests).sum();
        let total_successful: u64 = self.stats.values().map(|s| s.successful_requests).sum();
        let total_failed: u64 = self.stats.values().map(|s| s.failed_requests).sum();

        let avg_load = if !self.services.is_empty() {
            self.services.values()
                .map(|s| s.load_info.cpu_usage.max(s.load_info.memory_usage))
                .sum::<f32>() / self.services.len() as f32
        } else {
            0.0
        };

        RegistryStats {
            total_services,
            active_services,
            total_requests,
            successful_requests: total_successful,
            failed_requests: total_failed,
            average_load: avg_load,
            registry_uptime: 0, // Would track registry start time
        }
    }

    /// Get active service count
    pub fn get_active_service_count(&self) -> usize {
        self.services.values()
            .filter(|service| self.is_service_available(service))
            .count()
    }

    // Private helper methods

    fn validate_service_info(&self, service_info: &ServiceInfo) -> IpcResult<()> {
        if service_info.id.is_empty() || service_info.id.len() > MAX_SERVICE_NAME_LEN {
            return Err(IpcError::RegistrationFailed("Invalid service ID".to_string()));
        }

        if service_info.name.is_empty() || service_info.name.len() > MAX_SERVICE_NAME_LEN {
            return Err(IpcError::RegistrationFailed("Invalid service name".to_string()));
        }

        if service_info.capabilities.supported_dimensions.is_empty() {
            return Err(IpcError::RegistrationFailed("No supported dimensions".to_string()));
        }

        if service_info.capabilities.max_batch_size == 0 {
            return Err(IpcError::RegistrationFailed("Invalid batch size".to_string()));
        }

        Ok(())
    }

    fn matches_capabilities(&self, service_caps: &ServiceCapabilities, required_caps: &ServiceCapabilities) -> bool {
        // Check if service supports all required dimensions
        for &required_dim in &required_caps.supported_dimensions {
            if !service_caps.supported_dimensions.contains(&required_dim) {
                return false;
            }
        }

        // Check if service supports all required data types
        for required_type in &required_caps.supported_data_types {
            if !service_caps.supported_data_types.contains(required_type) {
                return false;
            }
        }

        // Check batch size capability
        if service_caps.max_batch_size < required_caps.max_batch_size {
            return false;
        }

        // Check model support if specified
        if !required_caps.supported_models.is_empty() {
            let has_common_model = required_caps.supported_models.iter()
                .any(|model| service_caps.supported_models.contains(model));
            if !has_common_model {
                return false;
            }
        }

        true
    }

    fn is_service_available(&self, service: &RegisteredService) -> bool {
        // Check if service is in a usable state
        match service.status {
            ServiceState::Ready | ServiceState::Busy => true,
            ServiceState::Overloaded => false, // Could be configurable
            ServiceState::Starting => false,
            ServiceState::Stopping | ServiceState::Stopped | ServiceState::Error => false,
        }
    }

    fn determine_service_status(&self, load_info: &ServiceLoadInfo) -> ServiceState {
        let max_load = load_info.cpu_usage.max(load_info.memory_usage);
        
        if max_load > 0.9 {
            ServiceState::Overloaded
        } else if max_load > 0.7 {
            ServiceState::Busy
        } else {
            ServiceState::Ready
        }
    }

    fn get_current_timestamp(&self) -> u64 {
        // Get current timestamp in seconds
        // In kernel mode, would use kernel time functions
        // In userspace, would use system time
        0 // Placeholder
    }
}

/// Registry statistics
#[derive(Debug, Clone)]
pub struct RegistryStats {
    pub total_services: usize,
    pub active_services: usize,
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub average_load: f32,
    pub registry_uptime: u64,
}

/// Service discovery filter
#[derive(Debug, Clone)]
pub struct ServiceFilter {
    /// Required dimensions
    pub dimensions: Option<Vec<u32>>,
    /// Required data types
    pub data_types: Option<Vec<VectorDataType>>,
    /// Required models
    pub models: Option<Vec<String>>,
    /// Minimum batch size
    pub min_batch_size: Option<usize>,
    /// Maximum load threshold
    pub max_load: Option<f32>,
    /// Minimum health score
    pub min_health_score: Option<u8>,
    /// Required tags
    pub tags: Option<Vec<String>>,
    /// Minimum priority
    pub min_priority: Option<u8>,
}

impl ServiceFilter {
    /// Create a new empty filter
    pub fn new() -> Self {
        Self {
            dimensions: None,
            data_types: None,
            models: None,
            min_batch_size: None,
            max_load: None,
            min_health_score: None,
            tags: None,
            min_priority: None,
        }
    }

    /// Apply filter to a service
    pub fn matches(&self, service: &RegisteredService) -> bool {
        // Check dimensions
        if let Some(ref required_dims) = self.dimensions {
            let has_required_dims = required_dims.iter()
                .all(|dim| service.info.capabilities.supported_dimensions.contains(dim));
            if !has_required_dims {
                return false;
            }
        }

        // Check data types
        if let Some(ref required_types) = self.data_types {
            let has_required_types = required_types.iter()
                .all(|dtype| service.info.capabilities.supported_data_types.contains(dtype));
            if !has_required_types {
                return false;
            }
        }

        // Check models
        if let Some(ref required_models) = self.models {
            let has_required_models = required_models.iter()
                .any(|model| service.info.capabilities.supported_models.contains(model));
            if !has_required_models {
                return false;
            }
        }

        // Check batch size
        if let Some(min_batch) = self.min_batch_size {
            if service.info.capabilities.max_batch_size < min_batch {
                return false;
            }
        }

        // Check load
        if let Some(max_load) = self.max_load {
            let current_load = service.load_info.cpu_usage.max(service.load_info.memory_usage);
            if current_load > max_load {
                return false;
            }
        }

        // Check health score
        if let Some(min_health) = self.min_health_score {
            if service.health.score < min_health {
                return false;
            }
        }

        // Check tags
        if let Some(ref required_tags) = self.tags {
            let has_required_tags = required_tags.iter()
                .all(|tag| service.tags.contains(tag));
            if !has_required_tags {
                return false;
            }
        }

        // Check priority
        if let Some(min_priority) = self.min_priority {
            if service.priority < min_priority {
                return false;
            }
        }

        true
    }
}

impl ServiceRegistry {
    /// Find services using a filter
    pub fn find_services_with_filter(&self, filter: &ServiceFilter) -> Vec<&RegisteredService> {
        self.services.values()
            .filter(|service| filter.matches(service))
            .filter(|service| self.is_service_available(service))
            .collect()
    }
}