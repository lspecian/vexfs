//! Load Balancer for VexFS IPC Services
//!
//! This module implements load balancing algorithms for distributing
//! embedding requests across multiple services based on various criteria
//! such as load, response time, and service capabilities.

use crate::shared::errors::{VexfsError, VexfsResult};
use crate::ipc::{IpcError, IpcResult};
use crate::ipc::protocol::*;
use crate::ipc::service_registry::{ServiceRegistry, RegisteredService};

#[cfg(not(feature = "std"))]
use alloc::{vec::Vec, string::String, collections::BTreeMap, boxed::Box};
#[cfg(feature = "std")]
use std::{vec::Vec, string::String, collections::BTreeMap, boxed::Box};

/// Load balancing algorithms
#[derive(Debug, Clone, PartialEq)]
pub enum LoadBalancingAlgorithm {
    /// Round-robin selection
    RoundRobin,
    /// Least connections
    LeastConnections,
    /// Least response time
    LeastResponseTime,
    /// Weighted round-robin
    WeightedRoundRobin,
    /// Load-based selection
    LoadBased,
    /// Random selection
    Random,
    /// Priority-based selection
    Priority,
    /// Capability-aware selection
    CapabilityAware,
}

/// Load balancer configuration
#[derive(Debug, Clone)]
pub struct LoadBalancerConfig {
    /// Default algorithm to use
    pub default_algorithm: LoadBalancingAlgorithm,
    /// Enable sticky sessions
    pub enable_sticky_sessions: bool,
    /// Session timeout in seconds
    pub session_timeout_sec: u64,
    /// Health check weight (0.0-1.0)
    pub health_weight: f32,
    /// Load weight (0.0-1.0)
    pub load_weight: f32,
    /// Response time weight (0.0-1.0)
    pub response_time_weight: f32,
    /// Maximum retries for failed services
    pub max_retries: u32,
    /// Enable circuit breaker
    pub enable_circuit_breaker: bool,
    /// Circuit breaker failure threshold
    pub circuit_breaker_threshold: u32,
}

impl Default for LoadBalancerConfig {
    fn default() -> Self {
        Self {
            default_algorithm: LoadBalancingAlgorithm::LoadBased,
            enable_sticky_sessions: false,
            session_timeout_sec: 300, // 5 minutes
            health_weight: 0.3,
            load_weight: 0.4,
            response_time_weight: 0.3,
            max_retries: 3,
            enable_circuit_breaker: true,
            circuit_breaker_threshold: 5,
        }
    }
}

/// Load balancer for distributing requests across services
pub struct LoadBalancer {
    /// Configuration
    config: LoadBalancerConfig,
    /// Round-robin counters
    round_robin_counters: BTreeMap<String, usize>,
    /// Sticky sessions
    sticky_sessions: BTreeMap<String, StickySession>,
    /// Circuit breaker states
    circuit_breakers: BTreeMap<String, CircuitBreakerState>,
    /// Service weights for weighted algorithms
    service_weights: BTreeMap<String, f32>,
    /// Load balancer statistics
    stats: LoadBalancerStats,
}

/// Sticky session information
#[derive(Debug, Clone)]
pub struct StickySession {
    /// Service ID
    pub service_id: String,
    /// Session creation time
    pub created_at: u64,
    /// Last access time
    pub last_access: u64,
    /// Request count
    pub request_count: u64,
}

/// Circuit breaker state
#[derive(Debug, Clone)]
pub struct CircuitBreakerState {
    /// Current state
    pub state: CircuitState,
    /// Failure count
    pub failure_count: u32,
    /// Last failure time
    pub last_failure: u64,
    /// Next retry time (for half-open state)
    pub next_retry: u64,
}

/// Circuit breaker states
#[derive(Debug, Clone, PartialEq)]
pub enum CircuitState {
    Closed,   // Normal operation
    Open,     // Failing, reject requests
    HalfOpen, // Testing if service recovered
}

/// Load balancer statistics
#[derive(Debug, Clone, Default)]
pub struct LoadBalancerStats {
    pub total_requests: u64,
    pub successful_selections: u64,
    pub failed_selections: u64,
    pub circuit_breaker_trips: u64,
    pub sticky_session_hits: u64,
    pub algorithm_usage: BTreeMap<String, u64>,
}

impl LoadBalancer {
    /// Create a new load balancer
    pub fn new() -> Self {
        Self::with_config(LoadBalancerConfig::default())
    }

    /// Create a new load balancer with configuration
    pub fn with_config(config: LoadBalancerConfig) -> Self {
        Self {
            config,
            round_robin_counters: BTreeMap::new(),
            sticky_sessions: BTreeMap::new(),
            circuit_breakers: BTreeMap::new(),
            service_weights: BTreeMap::new(),
            stats: LoadBalancerStats::default(),
        }
    }

    /// Select a service for handling a request
    pub fn select_service(&mut self, request: &EmbeddingRequest) -> IpcResult<ServiceInfo> {
        self.stats.total_requests += 1;

        // Check for sticky session first
        let enable_sticky = self.config.enable_sticky_sessions;
        if enable_sticky {
            if let Some(session_service) = self.check_sticky_session(request)? {
                self.stats.sticky_session_hits += 1;
                return Ok(session_service);
            }
        }

        // Since get_available_services returns empty for now, create a placeholder result
        // In a real implementation, this would query the ServiceRegistry
        self.stats.failed_selections += 1;
        return Err(IpcError::ServiceUnavailable("No services available".to_string()));
    }

    /// Add a service to the load balancer
    pub fn add_service(&mut self, service_info: ServiceInfo) -> IpcResult<()> {
        let service_id = service_info.id.clone();
        
        // Initialize round-robin counter
        self.round_robin_counters.insert(service_id.clone(), 0);
        
        // Initialize circuit breaker
        let circuit_breaker = CircuitBreakerState {
            state: CircuitState::Closed,
            failure_count: 0,
            last_failure: 0,
            next_retry: 0,
        };
        self.circuit_breakers.insert(service_id.clone(), circuit_breaker);
        
        // Set default weight
        self.service_weights.insert(service_id, 1.0);
        
        Ok(())
    }

    /// Remove a service from the load balancer
    pub fn remove_service(&mut self, service_id: &str) -> IpcResult<()> {
        // Clean up load balancer state
        self.round_robin_counters.remove(service_id);
        self.circuit_breakers.remove(service_id);
        self.service_weights.remove(service_id);
        
        // Remove sticky sessions for this service
        self.sticky_sessions.retain(|_, session| session.service_id != service_id);
        
        Ok(())
    }

    /// Record service response (for circuit breaker and statistics)
    pub fn record_response(&mut self, service_id: &str, success: bool, _response_time_us: u64) -> IpcResult<()> {
        // Get current timestamp and threshold once to avoid borrowing conflicts
        let current_timestamp = self.get_current_timestamp();
        let threshold = self.config.circuit_breaker_threshold;
        
        // Update circuit breaker state
        if let Some(circuit_breaker) = self.circuit_breakers.get_mut(service_id) {
            if success {
                // Reset failure count on success
                circuit_breaker.failure_count = 0;
                
                // Transition from half-open to closed if needed
                if circuit_breaker.state == CircuitState::HalfOpen {
                    circuit_breaker.state = CircuitState::Closed;
                }
            } else {
                // Increment failure count
                circuit_breaker.failure_count += 1;
                circuit_breaker.last_failure = current_timestamp;
                
                // Trip circuit breaker if threshold exceeded
                if circuit_breaker.failure_count >= threshold {
                    circuit_breaker.state = CircuitState::Open;
                    circuit_breaker.next_retry = current_timestamp + 60; // 60 second timeout
                    self.stats.circuit_breaker_trips += 1;
                }
            }
        }
        
        Ok(())
    }

    /// Set service weight for weighted algorithms
    pub fn set_service_weight(&mut self, service_id: &str, weight: f32) -> IpcResult<()> {
        if weight < 0.0 || weight > 10.0 {
            return Err(IpcError::InvalidMessage("Invalid weight".to_string()));
        }
        
        self.service_weights.insert(service_id.to_string(), weight);
        Ok(())
    }

    /// Get load balancer statistics
    pub fn get_stats(&self) -> &LoadBalancerStats {
        &self.stats
    }

    /// Perform maintenance (cleanup expired sessions, etc.)
    pub fn perform_maintenance(&mut self) -> IpcResult<MaintenanceResult> {
        let mut result = MaintenanceResult::default();
        
        // Clean up expired sticky sessions
        let current_time = self.get_current_timestamp();
        let session_timeout = self.config.session_timeout_sec;
        
        let expired_sessions: Vec<String> = self.sticky_sessions.iter()
            .filter(|(_, session)| current_time - session.last_access > session_timeout)
            .map(|(key, _)| key.clone())
            .collect();
        
        for session_key in &expired_sessions {
            self.sticky_sessions.remove(session_key);
        }
        result.expired_sessions = expired_sessions.len();
        
        // Update circuit breaker states
        for (_service_id, circuit_breaker) in &mut self.circuit_breakers {
            if circuit_breaker.state == CircuitState::Open && current_time >= circuit_breaker.next_retry {
                circuit_breaker.state = CircuitState::HalfOpen;
                result.circuit_breakers_reset += 1;
            }
        }
        
        Ok(result)
    }

    // Private helper methods

    fn check_sticky_session(&mut self, request: &EmbeddingRequest) -> IpcResult<Option<ServiceInfo>> {
        // Generate session key (could be based on client ID, request characteristics, etc.)
        let session_key = self.generate_session_key(request);
        
        // Get current timestamp once to avoid borrowing conflicts
        let current_timestamp = self.get_current_timestamp();
        
        if let Some(session) = self.sticky_sessions.get_mut(&session_key) {
            // Update last access time
            session.last_access = current_timestamp;
            session.request_count += 1;
            
            // Return the service info (would need to look up from registry)
            // For now, return None to indicate no sticky session
            return Ok(None);
        }
        
        Ok(None)
    }

    fn get_available_services(&self, _request: &EmbeddingRequest) -> IpcResult<Vec<&RegisteredService>> {
        // This would query the ServiceRegistry for services that can handle the request
        // For now, return empty vector as placeholder
        Ok(Vec::new())
    }

    fn filter_healthy_services<'a>(&self, services: Vec<&'a RegisteredService>) -> IpcResult<Vec<&'a RegisteredService>> {
        let healthy_services = services.into_iter()
            .filter(|service| {
                if let Some(circuit_breaker) = self.circuit_breakers.get(&service.info.id) {
                    circuit_breaker.state != CircuitState::Open
                } else {
                    true // No circuit breaker state means healthy
                }
            })
            .collect();
        
        Ok(healthy_services)
    }

    fn select_by_algorithm<'a>(&mut self, services: &[&'a RegisteredService], algorithm: &LoadBalancingAlgorithm) -> IpcResult<&'a RegisteredService> {
        match algorithm {
            LoadBalancingAlgorithm::RoundRobin => self.select_round_robin(services),
            LoadBalancingAlgorithm::LeastConnections => self.select_least_connections(services),
            LoadBalancingAlgorithm::LeastResponseTime => self.select_least_response_time(services),
            LoadBalancingAlgorithm::WeightedRoundRobin => self.select_weighted_round_robin(services),
            LoadBalancingAlgorithm::LoadBased => self.select_load_based(services),
            LoadBalancingAlgorithm::Random => self.select_random(services),
            LoadBalancingAlgorithm::Priority => self.select_priority(services),
            LoadBalancingAlgorithm::CapabilityAware => self.select_capability_aware(services),
        }
    }

    fn select_round_robin<'a>(&mut self, services: &[&'a RegisteredService]) -> IpcResult<&'a RegisteredService> {
        if services.is_empty() {
            return Err(IpcError::ServiceUnavailable("No services available".to_string()));
        }

        // Use a global counter for simplicity
        let counter = self.round_robin_counters.get("global").unwrap_or(&0);
        let index = counter % services.len();
        let next_counter = counter + 1;
        
        self.round_robin_counters.insert("global".to_string(), next_counter);
        
        Ok(services[index])
    }

    fn select_least_connections<'a>(&self, services: &[&'a RegisteredService]) -> IpcResult<&'a RegisteredService> {
        services.iter()
            .min_by_key(|service| service.load_info.active_requests)
            .copied()
            .ok_or_else(|| IpcError::ServiceUnavailable("No services available".to_string()))
    }

    fn select_least_response_time<'a>(&self, services: &[&'a RegisteredService]) -> IpcResult<&'a RegisteredService> {
        services.iter()
            .min_by_key(|service| service.load_info.avg_response_time_ms)
            .copied()
            .ok_or_else(|| IpcError::ServiceUnavailable("No services available".to_string()))
    }

    fn select_weighted_round_robin<'a>(&mut self, services: &[&'a RegisteredService]) -> IpcResult<&'a RegisteredService> {
        // Simplified weighted round-robin implementation
        // In a real implementation, would use proper weighted selection
        
        let total_weight: f32 = services.iter()
            .map(|service| self.service_weights.get(&service.info.id).unwrap_or(&1.0))
            .sum();
        
        if total_weight == 0.0 {
            return self.select_round_robin(services);
        }
        
        // For now, just select the service with highest weight
        services.iter()
            .max_by(|a, b| {
                let weight_a = self.service_weights.get(&a.info.id).unwrap_or(&1.0);
                let weight_b = self.service_weights.get(&b.info.id).unwrap_or(&1.0);
                weight_a.partial_cmp(weight_b).unwrap_or(core::cmp::Ordering::Equal)
            })
            .copied()
            .ok_or_else(|| IpcError::ServiceUnavailable("No services available".to_string()))
    }

    fn select_load_based<'a>(&self, services: &[&'a RegisteredService]) -> IpcResult<&'a RegisteredService> {
        // Calculate composite score based on multiple factors
        services.iter()
            .min_by(|a, b| {
                let score_a = self.calculate_load_score(a);
                let score_b = self.calculate_load_score(b);
                score_a.partial_cmp(&score_b).unwrap_or(core::cmp::Ordering::Equal)
            })
            .copied()
            .ok_or_else(|| IpcError::ServiceUnavailable("No services available".to_string()))
    }

    fn select_random<'a>(&self, services: &[&'a RegisteredService]) -> IpcResult<&'a RegisteredService> {
        if services.is_empty() {
            return Err(IpcError::ServiceUnavailable("No services available".to_string()));
        }

        // Simple pseudo-random selection
        let index = (self.get_current_timestamp() as usize) % services.len();
        Ok(services[index])
    }

    fn select_priority<'a>(&self, services: &[&'a RegisteredService]) -> IpcResult<&'a RegisteredService> {
        services.iter()
            .max_by_key(|service| service.priority)
            .copied()
            .ok_or_else(|| IpcError::ServiceUnavailable("No services available".to_string()))
    }

    fn select_capability_aware<'a>(&self, services: &[&'a RegisteredService]) -> IpcResult<&'a RegisteredService> {
        // Select based on capability match and performance
        // For now, just use load-based selection
        self.select_load_based(services)
    }

    fn calculate_load_score(&self, service: &RegisteredService) -> f32 {
        let health_score = (100 - service.health.score) as f32 / 100.0;
        let load_score = service.load_info.cpu_usage.max(service.load_info.memory_usage);
        let response_time_score = (service.load_info.avg_response_time_ms as f32) / 1000.0;
        
        // Weighted combination of factors
        health_score * self.config.health_weight +
        load_score * self.config.load_weight +
        response_time_score * self.config.response_time_weight
    }

    fn create_sticky_session(&mut self, request: &EmbeddingRequest, service: &ServiceInfo) -> IpcResult<()> {
        let session_key = self.generate_session_key(request);
        let current_time = self.get_current_timestamp();
        
        let session = StickySession {
            service_id: service.id.clone(),
            created_at: current_time,
            last_access: current_time,
            request_count: 1,
        };
        
        self.sticky_sessions.insert(session_key, session);
        Ok(())
    }

    fn generate_session_key(&self, request: &EmbeddingRequest) -> String {
        // Generate session key based on request characteristics
        // For now, use request ID as a simple key
        format!("session_{}", request.request_id)
    }

    fn get_current_timestamp(&self) -> u64 {
        // Get current timestamp in seconds
        0 // Placeholder
    }
}

/// Maintenance result
#[derive(Debug, Clone, Default)]
pub struct MaintenanceResult {
    pub expired_sessions: usize,
    pub circuit_breakers_reset: usize,
}

/// Load balancing strategy for specific request types
#[derive(Debug, Clone)]
pub struct LoadBalancingStrategy {
    /// Algorithm to use
    pub algorithm: LoadBalancingAlgorithm,
    /// Request type filter
    pub request_filter: RequestFilter,
    /// Priority (higher wins)
    pub priority: u8,
}

/// Request filter for strategy selection
#[derive(Debug, Clone)]
pub struct RequestFilter {
    /// Dimension ranges
    pub dimension_ranges: Option<Vec<(u32, u32)>>,
    /// Data types
    pub data_types: Option<Vec<VectorDataType>>,
    /// Models
    pub models: Option<Vec<String>>,
    /// Batch size ranges
    pub batch_size_ranges: Option<Vec<(usize, usize)>>,
}

impl RequestFilter {
    /// Check if request matches this filter
    pub fn matches(&self, request: &EmbeddingRequest) -> bool {
        // Check dimensions
        if let Some(ref ranges) = self.dimension_ranges {
            let matches_dimension = ranges.iter()
                .any(|(min, max)| request.dimensions >= *min && request.dimensions <= *max);
            if !matches_dimension {
                return false;
            }
        }

        // Check data type
        if let Some(ref types) = self.data_types {
            if !types.contains(&request.data_type) {
                return false;
            }
        }

        // Check model
        if let Some(ref models) = self.models {
            if let Some(ref request_model) = request.model {
                if !models.contains(request_model) {
                    return false;
                }
            } else {
                return false; // Request has no model but filter requires one
            }
        }

        true
    }
}

/// Advanced load balancer with strategy support
pub struct AdvancedLoadBalancer {
    /// Base load balancer
    base_balancer: LoadBalancer,
    /// Load balancing strategies
    strategies: Vec<LoadBalancingStrategy>,
}

impl AdvancedLoadBalancer {
    /// Create a new advanced load balancer
    pub fn new(config: LoadBalancerConfig) -> Self {
        Self {
            base_balancer: LoadBalancer::with_config(config),
            strategies: Vec::new(),
        }
    }

    /// Add a load balancing strategy
    pub fn add_strategy(&mut self, strategy: LoadBalancingStrategy) {
        self.strategies.push(strategy);
        // Sort by priority (highest first)
        self.strategies.sort_by(|a, b| b.priority.cmp(&a.priority));
    }

    /// Select service using strategy-based selection
    pub fn select_service_with_strategy(&mut self, request: &EmbeddingRequest) -> IpcResult<ServiceInfo> {
        // Find matching strategy
        for strategy in &self.strategies {
            if strategy.request_filter.matches(request) {
                // Clone the algorithm to avoid borrowing conflicts
                let algorithm = strategy.algorithm.clone();
                // Use strategy-specific algorithm
                return self.select_with_algorithm(request, &algorithm);
            }
        }

        // Fall back to default selection
        self.base_balancer.select_service(request)
    }

    fn select_with_algorithm(&mut self, request: &EmbeddingRequest, algorithm: &LoadBalancingAlgorithm) -> IpcResult<ServiceInfo> {
        // Temporarily override the algorithm and select
        let original_algorithm = self.base_balancer.config.default_algorithm.clone();
        self.base_balancer.config.default_algorithm = algorithm.clone();
        
        let result = self.base_balancer.select_service(request);
        
        // Restore original algorithm
        self.base_balancer.config.default_algorithm = original_algorithm;
        
        result
    }
}